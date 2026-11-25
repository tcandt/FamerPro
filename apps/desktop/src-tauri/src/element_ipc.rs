use base64::Engine;
use fp_adb::{AdbDeviceDriver, AdbRunner};
use fp_domain::DeviceDriver;
use fp_element::{
    evaluate_native_selector, generate_selectors_for_node, parse_ui_hierarchy, ActionExecutionMode,
    DetectionResult, ElementActionKind, ElementActionPlan, ElementNode, ElementSnapshot,
    GeneratedSelector, NativeElementSelector, Selector, SelectorMatch, UnifiedElementExecutor,
};
use fp_storage::LibraryElementRecord;
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
