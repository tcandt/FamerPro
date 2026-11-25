use base64::Engine;
use fp_adb::{AdbDeviceDriver, AdbRunner};
use fp_domain::{ActionReceipt, DeviceDriver, SwipePath};
use fp_element::{
    build_step_from_touch, evaluate_native_selector, execute_recorded_step,
    generate_selectors_for_node, parse_ui_hierarchy, recorded_steps_to_workflow_document,
    ActionExecutionMode, DetectionResult, ElementActionKind, ElementActionPlan, ElementNode,
    ElementSnapshot, GeneratedSelector, NativeElementSelector, RecordedWorkflowStep, Selector,
    SelectorMatch, StepExecutionResult, UnifiedElementExecutor, WorkflowReplayReport,
};
use fp_storage::LibraryElementRecord;
use fp_workflow_schema::WorkflowDocument;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::State;

use crate::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct InspectSnapshotResult {
    pub snapshot: ElementSnapshot,
    pub screenshot_base64: String,
    pub capture_duration_ms: u64,
    pub dump_duration_ms: u64,
    pub parse_duration_ms: u64,
    pub total_duration_ms: u64,
    pub device_serial: String,
    pub package_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ElementActionResult {
    pub success: bool,
    pub plan: ElementActionPlan,
    pub execution_ms: u64,
    pub new_screenshot_base64: Option<String>,
    pub post_verification_success: bool,
    pub message: String,
}

#[tauri::command]
pub async fn element_inspect_snapshot(serial: String) -> Result<InspectSnapshotResult, String> {
    let runner = AdbRunner::new(None);
    let driver = AdbDeviceDriver::new(serial.clone(), runner);
    let total_start = Instant::now();

    // 1. Capture screen via DeviceDriver
    let cap_start = Instant::now();
    let frame = driver
        .snapshot()
        .await
        .map_err(|e| format!("Screencap failed on {}: {}", serial, e))?;
    let capture_ms = cap_start.elapsed().as_millis() as u64;

    // Convert to base64
    let screenshot_b64 = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&frame.bytes)
    );

    // 2. Dump UI hierarchy via DeviceDriver
    let dump_start = Instant::now();
    let xml_output = driver
        .get_ui_tree()
        .await
        .map_err(|e| format!("uiautomator dump failed on {}: {}", serial, e))?;
    let dump_ms = dump_start.elapsed().as_millis() as u64;

    // 3. Parse UI Hierarchy in native Rust
    let parse_start = Instant::now();
    let snapshot = parse_ui_hierarchy(&xml_output, &frame.geometry)
        .map_err(|e| format!("Failed to parse UI hierarchy: {}", e))?;
    let parse_ms = parse_start.elapsed().as_millis() as u64;

    let total_ms = total_start.elapsed().as_millis() as u64;
    let pkg = snapshot.package_name.clone();

    Ok(InspectSnapshotResult {
        snapshot,
        screenshot_base64: screenshot_b64,
        capture_duration_ms: capture_ms,
        dump_duration_ms: dump_ms,
        parse_duration_ms: parse_ms,
        total_duration_ms: total_ms,
        device_serial: serial,
        package_name: pkg,
    })
}

#[tauri::command]
pub fn element_find_matches(
    snapshot: ElementSnapshot,
    selector: NativeElementSelector,
    require_unique: bool,
) -> Result<Vec<SelectorMatch>, String> {
    evaluate_native_selector(&snapshot, &selector, require_unique).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn element_generate_selectors(
    snapshot: ElementSnapshot,
    node_id: usize,
) -> Vec<GeneratedSelector> {
    generate_selectors_for_node(&snapshot, node_id)
}

#[tauri::command]
pub fn element_find_at_point(snapshot: ElementSnapshot, x: f32, y: f32) -> Option<ElementNode> {
    snapshot.find_node_at_point(x, y).cloned()
}

/// Tests detection across any selector strategy without executing any action on the device.
#[tauri::command]
pub fn element_test_detection(
    snapshot: Option<ElementSnapshot>,
    selector: Selector,
    require_unique: bool,
) -> Result<DetectionResult, String> {
    UnifiedElementExecutor::detect(snapshot.as_ref(), &selector, None, require_unique)
        .map_err(|e| e.to_string())
}

/// Executes planned action using the Unified Execution Engine across the DeviceDriver abstraction.
#[tauri::command]
pub async fn element_execute_action(
    serial: String,
    match_item: SelectorMatch,
    action_kind: ElementActionKind,
    mode: ActionExecutionMode,
) -> Result<ElementActionResult, String> {
    let runner = AdbRunner::new(None);
    let driver = AdbDeviceDriver::new(serial.clone(), runner);

    // 1. Plan action using Unified Element Engine
    let plan = UnifiedElementExecutor::plan(&match_item, action_kind.clone(), mode, None)
        .map_err(|e| e.to_string())?;

    let exec_start = Instant::now();

    // 2. Dispatch via unified DeviceDriver
    let receipt = UnifiedElementExecutor::execute(&driver, &plan)
        .await
        .map_err(|e| format!("Action execution failed: {}", e))?;

    let execution_ms = exec_start.elapsed().as_millis() as u64;

    // 3. Post-execution transition verification
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let new_b64 = match driver.snapshot().await {
        Ok(frame) => Some(format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(&frame.bytes)
        )),
        Err(_) => None,
    };

    Ok(ElementActionResult {
        success: receipt.success,
        plan,
        execution_ms,
        new_screenshot_base64: new_b64,
        post_verification_success: receipt.verify_passed,
        message: format!(
            "Successfully executed {:?} on device {} at ({:.1}, {:.1})",
            action_kind,
            serial,
            receipt.point_tapped.as_ref().map(|p| p.x).unwrap_or(0.0),
            receipt.point_tapped.as_ref().map(|p| p.y).unwrap_or(0.0)
        ),
    })
}

// ========================================================================
// Phase 2.1: Element Library Persistence IPC Commands
// ========================================================================

#[tauri::command]
pub fn element_save_to_library(
    state: State<'_, Arc<Mutex<AppState>>>,
    record: LibraryElementRecord,
) -> Result<(), String> {
    let st = state.lock().unwrap();
    st.storage.save_element(&record).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn element_list_library(
    state: State<'_, Arc<Mutex<AppState>>>,
    project_id: String,
) -> Result<Vec<LibraryElementRecord>, String> {
    let st = state.lock().unwrap();
    st.storage
        .list_elements(&project_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn element_delete_from_library(
    state: State<'_, Arc<Mutex<AppState>>>,
    id: String,
) -> Result<(), String> {
    let st = state.lock().unwrap();
    st.storage.delete_element(&id).map_err(|e| e.to_string())
}

// ========================================================================
// Phase 2.2: Device Studio Control & Streaming Commands
// ========================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct StreamFrameResponse {
    pub screenshot_base64: String,
    pub width: u32,
    pub height: u32,
    pub rotation_deg: u32,
    pub timestamp_ms: u64,
    pub latency_ms: u64,
}

#[tauri::command]
pub async fn device_press_key(serial: String, keycode: u32) -> Result<ActionReceipt, String> {
    let runner = AdbRunner::new(None);
    let driver = AdbDeviceDriver::new(serial, runner);
    driver.key_event(keycode).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn device_tap(serial: String, x: u32, y: u32) -> Result<ActionReceipt, String> {
    let runner = AdbRunner::new(None);
    let driver = AdbDeviceDriver::new(serial, runner);
    driver.tap(x, y).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn device_swipe(
    serial: String,
    start_x: f32,
    start_y: f32,
    end_x: f32,
    end_y: f32,
    duration_ms: u32,
) -> Result<ActionReceipt, String> {
    let runner = AdbRunner::new(None);
    let driver = AdbDeviceDriver::new(serial, runner);
    let path = SwipePath {
        start_x: start_x as u32,
        start_y: start_y as u32,
        end_x: end_x as u32,
        end_y: end_y as u32,
        duration_ms: duration_ms as u64,
    };
    driver.swipe(path).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn device_input_text(serial: String, text: String) -> Result<ActionReceipt, String> {
    let runner = AdbRunner::new(None);
    let driver = AdbDeviceDriver::new(serial, runner);
    driver.input_text(&text).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn device_stream_frame(serial: String) -> Result<StreamFrameResponse, String> {
    let runner = AdbRunner::new(None);
    let driver = AdbDeviceDriver::new(serial.clone(), runner);

    let start = Instant::now();
    let frame = driver
        .snapshot()
        .await
        .map_err(|e| format!("Snapshot failed on {}: {}", serial, e))?;
    let latency_ms = start.elapsed().as_millis() as u64;

    let b64 = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&frame.bytes)
    );

    Ok(StreamFrameResponse {
        screenshot_base64: b64,
        width: frame.geometry.width_px,
        height: frame.geometry.height_px,
        rotation_deg: frame.geometry.rotation_deg as u32,
        timestamp_ms: frame.timestamp_ms,
        latency_ms,
    })
}

// ========================================================================
// Phase 2.2: Smart Workflow Recorder Commands
// ========================================================================

#[tauri::command]
pub async fn recorder_record_touch(
    serial: String,
    x: f32,
    y: f32,
    step_number: usize,
    action_type: String,
    text_input: Option<String>,
) -> Result<RecordedWorkflowStep, String> {
    let runner = AdbRunner::new(None);
    let driver = AdbDeviceDriver::new(serial.clone(), runner);

    // 1. Capture before-state
    let before_frame = driver
        .snapshot()
        .await
        .map_err(|e| format!("Pre-touch snapshot failed: {}", e))?;
    let before_b64 = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&before_frame.bytes)
    );

    // 2. Dump UI tree and parse snapshot
    let xml = driver
        .get_ui_tree()
        .await
        .map_err(|e| format!("Pre-touch UI dump failed: {}", e))?;
    let snapshot = parse_ui_hierarchy(&xml, &before_frame.geometry)
        .map_err(|e| format!("Failed to parse UI hierarchy: {}", e))?;

    // 3. Build step from touch coordinates
    let mut step = build_step_from_touch(&snapshot, x, y, step_number, &action_type, text_input);
    step.before_screenshot = Some(before_b64);

    // 4. Dispatch real action on device
    let exec_start = Instant::now();
    let step_res = execute_recorded_step(&driver, &step, Some(&snapshot)).await;
    step.duration_ms = exec_start.elapsed().as_millis() as u64;
    step.status = if step_res.success {
        "recorded".to_string()
    } else {
        "failed".to_string()
    };

    // 5. Short pause then capture after-state
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    if let Ok(after_frame) = driver.snapshot().await {
        step.after_screenshot = Some(format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(&after_frame.bytes)
        ));
    }

    Ok(step)
}

#[tauri::command]
pub async fn recorder_replay_step(
    serial: String,
    step: RecordedWorkflowStep,
) -> Result<StepExecutionResult, String> {
    let runner = AdbRunner::new(None);
    let driver = AdbDeviceDriver::new(serial.clone(), runner);

    // Try to get fresh snapshot for live detection
    let snapshot = if step.selector.is_some() {
        if let Ok(frame) = driver.snapshot().await {
            if let Ok(xml) = driver.get_ui_tree().await {
                parse_ui_hierarchy(&xml, &frame.geometry).ok()
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let result = execute_recorded_step(&driver, &step, snapshot.as_ref()).await;
    Ok(result)
}

#[tauri::command]
pub async fn recorder_replay_all(
    serial: String,
    steps: Vec<RecordedWorkflowStep>,
) -> Result<WorkflowReplayReport, String> {
    let runner = AdbRunner::new(None);
    let driver = AdbDeviceDriver::new(serial.clone(), runner);

    let start_all = Instant::now();
    let total_steps = steps.len();
    let mut results = Vec::new();
    let mut passed_steps = 0;
    let mut failed_steps = 0;
    let mut executed_steps = 0;

    for step in steps.into_iter().filter(|s| s.enabled) {
        executed_steps += 1;

        // Fresh snapshot for step if selector present
        let snapshot = if step.selector.is_some() {
            if let Ok(frame) = driver.snapshot().await {
                if let Ok(xml) = driver.get_ui_tree().await {
                    parse_ui_hierarchy(&xml, &frame.geometry).ok()
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let res = execute_recorded_step(&driver, &step, snapshot.as_ref()).await;
        if res.success {
            passed_steps += 1;
        } else {
            failed_steps += 1;
        }
        results.push(res);

        // Pause configured delay or default 400ms between steps
        let wait_dur = step.wait_ms.unwrap_or(400);
        tokio::time::sleep(std::time::Duration::from_millis(wait_dur)).await;
    }

    Ok(WorkflowReplayReport {
        total_steps,
        executed_steps,
        passed_steps,
        failed_steps,
        total_duration_ms: start_all.elapsed().as_millis() as u64,
        results,
    })
}

#[tauri::command]
pub fn recorder_export_workflow(
    workflow_id: String,
    name: String,
    steps: Vec<RecordedWorkflowStep>,
) -> Result<WorkflowDocument, String> {
    Ok(recorded_steps_to_workflow_document(
        &workflow_id,
        &name,
        &steps,
    ))
}

#[tauri::command]
pub fn recorder_save_workflow(
    state: State<'_, Arc<Mutex<AppState>>>,
    workflow_id: String,
    name: String,
    doc_json: String,
) -> Result<bool, String> {
    let st = state.lock().unwrap();
    st.storage
        .save_workflow_draft(&workflow_id, &name, &doc_json)
        .map_err(|e| e.to_string())?;
    Ok(true)
}
