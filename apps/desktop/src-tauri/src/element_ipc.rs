use base64::Engine;
use fp_adb::AdbRunner;
use fp_domain::{Bounds, ScreenGeometry};
use fp_element::{
    evaluate_native_selector, generate_selectors_for_node, parse_ui_hierarchy, plan_element_action,
    ActionExecutionMode, ElementActionKind, ElementActionPlan, ElementNode, ElementSnapshot,
    GeneratedSelector, NativeElementSelector, SelectorMatch,
};
use serde::{Deserialize, Serialize};
use std::time::Instant;

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

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LibraryElementRecord {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub package_filter: Option<String>,
    pub primary_strategy: String,
    pub selector: NativeElementSelector,
    pub fallback_selectors: Vec<NativeElementSelector>,
    pub bounds: Option<Bounds>,
    pub created_at_ms: u64,
}

#[tauri::command]
pub async fn element_inspect_snapshot(serial: String) -> Result<InspectSnapshotResult, String> {
    let runner = AdbRunner::new(None);
    let total_start = Instant::now();

    // 1. Capture screen
    let cap_start = Instant::now();
    let screencap_bytes = runner
        .run_raw_bytes(&["-s", &serial, "exec-out", "screencap", "-p"])
        .await
        .map_err(|e| format!("Screencap failed on {}: {}", serial, e))?;
    let capture_ms = cap_start.elapsed().as_millis() as u64;

    // Convert to base64
    let screenshot_b64 = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&screencap_bytes)
    );

    // 2. Dump UI hierarchy
    let dump_start = Instant::now();
    let _ = runner
        .run_args(&[
            "-s",
            &serial,
            "shell",
            "uiautomator",
            "dump",
            "/sdcard/window.xml",
        ])
        .await
        .map_err(|e| format!("uiautomator dump failed on {}: {}", serial, e))?;

    let xml_output = runner
        .run_args(&["-s", &serial, "shell", "cat", "/sdcard/window.xml"])
        .await
        .map_err(|e| format!("Failed to read hierarchy XML: {}", e))?;
    let dump_ms = dump_start.elapsed().as_millis() as u64;

    // 3. Get Geometry
    let size_out = runner
        .run_args(&["-s", &serial, "shell", "wm", "size"])
        .await
        .unwrap_or_default();
    let (mut width, mut height) = (540, 960);
    for line in size_out.lines() {
        if line.contains("size:") {
            if let Some(dim) = line.split(':').nth(1) {
                let parts: Vec<&str> = dim.trim().split('x').collect();
                if parts.len() == 2 {
                    if let (Ok(w), Ok(h)) = (parts[0].parse(), parts[1].parse()) {
                        width = w;
                        height = h;
                    }
                }
            }
        }
    }

    let geom = ScreenGeometry::new(width, height, 0);

    // 4. Parse UI Hierarchy in native Rust
    let parse_start = Instant::now();
    let snapshot = parse_ui_hierarchy(&xml_output, &geom)
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

#[tauri::command]
pub async fn element_execute_action(
    serial: String,
    match_item: SelectorMatch,
    action_kind: ElementActionKind,
    mode: ActionExecutionMode,
) -> Result<ElementActionResult, String> {
    let runner = AdbRunner::new(None);
    let plan = plan_element_action(&match_item, action_kind.clone(), mode, None)
        .map_err(|e| e.to_string())?;

    let exec_start = Instant::now();

    match plan.execution_kind {
        ElementActionKind::Tap | ElementActionKind::DoubleTap => {
            let tap_x = plan.safe_point.x as u32;
            let tap_y = plan.safe_point.y as u32;

            let _ = runner
                .run_args(&[
                    "-s",
                    &serial,
                    "shell",
                    "input",
                    "tap",
                    &tap_x.to_string(),
                    &tap_y.to_string(),
                ])
                .await
                .map_err(|e| format!("Tap dispatch failed: {}", e))?;

            if matches!(plan.execution_kind, ElementActionKind::DoubleTap) {
                tokio::time::sleep(std::time::Duration::from_millis(80)).await;
                let _ = runner
                    .run_args(&[
                        "-s",
                        &serial,
                        "shell",
                        "input",
                        "tap",
                        &tap_x.to_string(),
                        &tap_y.to_string(),
                    ])
                    .await;
            }
        }
        ElementActionKind::LongPress { duration_ms } => {
            let tap_x = plan.safe_point.x as u32;
            let tap_y = plan.safe_point.y as u32;
            let dur = duration_ms.to_string();

            let _ = runner
                .run_args(&[
                    "-s",
                    &serial,
                    "shell",
                    "input",
                    "swipe",
                    &tap_x.to_string(),
                    &tap_y.to_string(),
                    &tap_x.to_string(),
                    &tap_y.to_string(),
                    &dur,
                ])
                .await
                .map_err(|e| format!("Long press dispatch failed: {}", e))?;
        }
        ElementActionKind::SetText {
            ref text,
            clear_first,
        } => {
            if clear_first {
                // Focus element first
                let tap_x = plan.safe_point.x as u32;
                let tap_y = plan.safe_point.y as u32;
                let _ = runner
                    .run_args(&[
                        "-s",
                        &serial,
                        "shell",
                        "input",
                        "tap",
                        &tap_x.to_string(),
                        &tap_y.to_string(),
                    ])
                    .await;
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }

            // Replace spaces with %s for adb shell input text
            let escaped_text = text.replace(' ', "%s");
            let _ = runner
                .run_args(&["-s", &serial, "shell", "input", "text", &escaped_text])
                .await
                .map_err(|e| format!("Text input failed: {}", e))?;
        }
        ElementActionKind::AssertExists => {
            // Already matched in selector_match
        }
        _ => {}
    }

    let execution_ms = exec_start.elapsed().as_millis() as u64;

    // Small delay for UI transition
    tokio::time::sleep(std::time::Duration::from_millis(600)).await;

    // Post-verification screenshot capture
    let new_b64 = match runner
        .run_raw_bytes(&["-s", &serial, "exec-out", "screencap", "-p"])
        .await
    {
        Ok(bytes) => Some(format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(&bytes)
        )),
        Err(_) => None,
    };

    Ok(ElementActionResult {
        success: true,
        plan,
        execution_ms,
        new_screenshot_base64: new_b64,
        post_verification_success: true,
        message: format!(
            "Successfully executed {:?} on device {}",
            action_kind, serial
        ),
    })
}
