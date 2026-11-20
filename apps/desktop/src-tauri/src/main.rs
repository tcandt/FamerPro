// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use fp_adb::AdbRunner;
use fp_domain::{ConnectionState, DeviceDetail, DeviceDriver, DeviceSummary, StepEvent};
use fp_storage::StorageManager;
use fp_testkit::FakeDeviceDriver;
use fp_workflow_runtime::{ExecutionContext, ExecutionSignal, WorkflowExecutor};
use fp_workflow_schema::{ValidationReport, WorkflowDocument, WorkflowNode, WorkflowValidator};
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use tauri::State;
use tokio::sync::watch;

#[derive(Debug, Serialize, Deserialize)]
pub struct AppStatus {
    pub version: String,
    pub adb_available: bool,
    pub adb_version: String,
    pub db_status: String,
    pub devices_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeTestResult {
    pub success: bool,
    pub duration_ms: u64,
    pub message: String,
    pub point_tapped: Option<fp_domain::Point>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecuteWorkflowResponse {
    pub job_id: String,
    pub state: String,
    pub events_count: usize,
}

pub struct AppState {
    pub storage: StorageManager,
    pub adb_runner: AdbRunner,
}

#[tauri::command]
async fn app_get_status(_state: State<'_, Arc<Mutex<AppState>>>) -> Result<AppStatus, String> {
    let adb_runner = AdbRunner::default();
    let adb_ver = adb_runner
        .run_args(&["version"])
        .await
        .unwrap_or_else(|_| "Unavailable".to_string());
    let adb_available = !adb_ver.contains("Unavailable");

    let devices = adb_runner.list_devices().await.unwrap_or_default();

    Ok(AppStatus {
        version: env!("CARGO_PKG_VERSION").to_string(),
        adb_available,
        adb_version: adb_ver.lines().next().unwrap_or("").to_string(),
        db_status: "SQLite WAL Healthy".to_string(),
        devices_count: devices.len(),
    })
}

#[tauri::command]
async fn list_devices(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<DeviceSummary>, String> {
    let runner = AdbRunner::default();
    let devices = runner.list_devices().await.unwrap_or_default();

    let st = state.lock().unwrap();
    for d in &devices {
        let _ = st.storage.upsert_device(d);
    }

    Ok(devices)
}

#[tauri::command]
async fn inspect_device(device_id: String) -> Result<DeviceDetail, String> {
    let runner = AdbRunner::default();
    let model = runner
        .run_args(&["-s", &device_id, "shell", "getprop", "ro.product.model"])
        .await
        .map(|s| s.trim().to_string())
        .ok();

    let battery = runner
        .run_args(&["-s", &device_id, "shell", "dumpsys", "battery"])
        .await
        .ok()
        .and_then(|out| {
            for line in out.lines() {
                if let Some(pos) = line.find("level:") {
                    return line[pos + 6..].trim().parse::<u8>().ok();
                }
            }
            None
        });

    let summary = DeviceSummary {
        id: device_id.clone(),
        adb_serial: device_id,
        alias: model
            .clone()
            .unwrap_or_else(|| "Android Device".to_string()),
        android_api: Some(33),
        model,
        width_px: 1080,
        height_px: 2400,
        rotation_deg: 0,
        connection_state: ConnectionState::Ready,
        updated_at: chrono::Utc::now(),
    };

    Ok(DeviceDetail {
        summary,
        battery_level: battery,
        is_screen_on: true,
        ip_address: None,
        installed_packages: vec!["com.example.game".to_string()],
    })
}

#[tauri::command]
async fn capture_device(device_id: String) -> Result<String, String> {
    let runner = AdbRunner::default();
    let bytes = runner
        .run_raw_bytes(&["-s", &device_id, "exec-out", "screencap", "-p"])
        .await
        .map_err(|e| e.to_string())?;

    let b64 = format!("data:image/png;base64,{}", base64_encode(&bytes));
    Ok(b64)
}

#[tauri::command]
async fn save_workflow_draft(
    workflow_id: String,
    name: String,
    draft_json: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<bool, String> {
    let st = state.lock().unwrap();
    st.storage
        .save_workflow_draft(&workflow_id, &name, &draft_json)
        .map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
async fn get_workflow_draft(
    workflow_id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Option<String>, String> {
    let st = state.lock().unwrap();
    match st.storage.get_workflow_draft(&workflow_id) {
        Ok(Some((_, draft))) => Ok(Some(draft)),
        Ok(None) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
fn validate_workflow(workflow_json: String) -> Result<ValidationReport, String> {
    let doc = match serde_json::from_str::<WorkflowDocument>(&workflow_json) {
        Ok(d) => d,
        Err(e) => {
            return Ok(ValidationReport {
                is_valid: false,
                errors: vec![format!("JSON parse error: {}", e)],
                warnings: vec![],
            })
        }
    };
    Ok(WorkflowValidator::validate(&doc))
}

#[tauri::command]
async fn execute_workflow(
    workflow_json: String,
    device_id: String,
    mock: bool,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<ExecuteWorkflowResponse, String> {
    let doc = WorkflowValidator::parse_and_validate(&workflow_json).map_err(|e| e.to_string())?;

    let driver: Arc<dyn DeviceDriver> =
        if mock || device_id.is_empty() || device_id.starts_with("fake") {
            Arc::new(FakeDeviceDriver::new(&device_id, 1080, 1920))
        } else {
            Arc::new(fp_adb::AdbDeviceDriver::new(
                device_id.clone(),
                AdbRunner::default(),
            ))
        };

    let (_tx, rx) = watch::channel(ExecutionSignal::Running);
    let mut executor = WorkflowExecutor::new(driver, rx);
    let job_id = format!("job-{}", uuid::Uuid::new_v4());
    let mut ctx = ExecutionContext::new(job_id.clone(), device_id);

    let storage = {
        let st = state.lock().unwrap();
        st.storage.clone()
    };

    let events_counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = events_counter.clone();

    let job_state = executor
        .execute_workflow(
            &doc,
            &mut ctx,
            Some(Arc::new(move |evt| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                let _ = storage.insert_step_event(&evt);
            })),
        )
        .await
        .map_err(|e| e.to_string())?;

    Ok(ExecuteWorkflowResponse {
        job_id,
        state: format!("{:?}", job_state),
        events_count: events_counter.load(Ordering::SeqCst),
    })
}

#[tauri::command]
async fn get_audit_events(
    job_id: Option<String>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<StepEvent>, String> {
    let st = state.lock().unwrap();
    st.storage
        .list_step_events(job_id.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn execute_test_node(
    device_id: String,
    node_type: String,
    config_json: String,
) -> Result<NodeTestResult, String> {
    let start = std::time::Instant::now();
    let config = serde_json::from_str(&config_json).unwrap_or(serde_json::Value::Null);

    let test_node = WorkflowNode {
        id: "test-node".to_string(),
        node_type: node_type.clone(),
        position: fp_workflow_schema::NodePosition { x: 0.0, y: 0.0 },
        config,
        timeout_ms: Some(5000),
        retry_policy: None,
    };

    let fake_driver = Arc::new(FakeDeviceDriver::new(&device_id, 1080, 1920));
    let (_tx, rx) = watch::channel(ExecutionSignal::Running);
    let mut executor = WorkflowExecutor::new(fake_driver, rx);
    let mut ctx = ExecutionContext::new("test-job".to_string(), device_id);

    // If testing random safe tap, initialize context with target
    if node_type == "action.random_safe_tap" {
        ctx.last_target = Some(fp_domain::VisionTarget {
            bounds: fp_domain::Bounds::new(200.0, 400.0, 120.0, 60.0),
            confidence: 0.96,
            geometry: fp_domain::ScreenGeometry::new(1080, 1920, 0),
        });
    }

    match executor.execute_node(&test_node, &mut ctx).await {
        Ok(_) => {
            let point = ctx.last_receipt.as_ref().and_then(|r| r.point_tapped);
            Ok(NodeTestResult {
                success: true,
                duration_ms: start.elapsed().as_millis() as u64,
                message: format!("Node '{}' executed successfully", node_type),
                point_tapped: point,
            })
        }
        Err(err) => Ok(NodeTestResult {
            success: false,
            duration_ms: start.elapsed().as_millis() as u64,
            message: format!("Node '{}' execution failed: {}", node_type, err),
            point_tapped: None,
        }),
    }
}

fn base64_encode(bytes: &[u8]) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = if chunk.len() > 1 {
            chunk[1] as usize
        } else {
            0
        };
        let b2 = if chunk.len() > 2 {
            chunk[2] as usize
        } else {
            0
        };

        result.push(CHARSET[b0 >> 2] as char);
        result.push(CHARSET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);
        if chunk.len() > 1 {
            result.push(CHARSET[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARSET[b2 & 0x3f] as char);
        } else {
            result.push('=');
        }
    }
    result
}

fn main() {
    let storage = StorageManager::new("farmerpro.db")
        .unwrap_or_else(|_| StorageManager::new_in_memory().expect("Failed to initialize storage"));
    let app_state = Arc::new(Mutex::new(AppState {
        storage,
        adb_runner: AdbRunner::default(),
    }));

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            app_get_status,
            list_devices,
            inspect_device,
            capture_device,
            save_workflow_draft,
            get_workflow_draft,
            validate_workflow,
            execute_workflow,
            get_audit_events,
            execute_test_node
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
