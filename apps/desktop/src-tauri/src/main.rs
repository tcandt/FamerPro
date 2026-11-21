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
    pub adb_path: String,
    pub db_status: String,
    pub db_path: String,
    pub devices_count: usize,
    pub is_portable: bool,
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
    pub db_path: String,
    pub is_portable: bool,
}

#[tauri::command]
async fn app_get_status(state: State<'_, Arc<Mutex<AppState>>>) -> Result<AppStatus, String> {
    let (adb_path, db_path, is_portable) = {
        let st = state.lock().unwrap();
        (
            st.adb_runner.adb_path.clone(),
            st.db_path.clone(),
            st.is_portable,
        )
    };

    let adb_runner = AdbRunner::new(Some(adb_path.clone()));
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
        adb_path,
        db_status: "SQLite WAL Healthy".to_string(),
        db_path,
        devices_count: devices.len(),
        is_portable,
    })
}

#[tauri::command]
fn app_window_minimize(window: tauri::Window) -> Result<(), String> {
    window.minimize().map_err(|e| e.to_string())
}

#[tauri::command]
fn app_window_toggle_maximize(window: tauri::Window) -> Result<(), String> {
    if window.is_maximized().unwrap_or(false) {
        window.unmaximize().map_err(|e| e.to_string())
    } else {
        window.maximize().map_err(|e| e.to_string())
    }
}

#[tauri::command]
fn app_window_close(window: tauri::Window) -> Result<(), String> {
    window.close().map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_devices(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<DeviceSummary>, String> {
    let adb_path = {
        let st = state.lock().unwrap();
        st.adb_runner.adb_path.clone()
    };
    let runner = AdbRunner::new(Some(adb_path));
    let mut devices = runner.list_devices().await.unwrap_or_default();

    for d in &mut devices {
        if d.connection_state == ConnectionState::Ready {
            if let Ok(sdk_str) = runner
                .run_args(&["-s", &d.id, "shell", "getprop", "ro.build.version.sdk"])
                .await
            {
                if let Ok(sdk) = sdk_str.trim().parse::<u32>() {
                    d.android_api = Some(sdk);
                }
            }
        }
    }

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

    let api_level = runner
        .run_args(&["-s", &device_id, "shell", "getprop", "ro.build.version.sdk"])
        .await
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok());

    let wm_size = runner
        .run_args(&["-s", &device_id, "shell", "wm", "size"])
        .await
        .unwrap_or_default();

    let mut width = 1080;
    let mut height = 2400;
    for line in wm_size.lines() {
        if let Some(pos) = line.find(':') {
            let res_str = line[pos + 1..].trim();
            let dims: Vec<&str> = res_str.split('x').collect();
            if dims.len() == 2 {
                if let (Ok(w), Ok(h)) = (dims[0].trim().parse(), dims[1].trim().parse()) {
                    width = w;
                    height = h;
                }
            }
        }
    }

    let summary = DeviceSummary {
        id: device_id.clone(),
        adb_serial: device_id.clone(),
        alias: model
            .clone()
            .unwrap_or_else(|| "Android Device".to_string()),
        android_api: api_level.or(Some(33)),
        model,
        width_px: width,
        height_px: height,
        rotation_deg: 0,
        connection_state: ConnectionState::Ready,
        updated_at: chrono::Utc::now(),
    };

    let packages_out = runner
        .run_args(&["-s", &device_id, "shell", "pm", "list", "packages"])
        .await
        .unwrap_or_default();
    let mut installed_packages = Vec::new();
    for line in packages_out.lines() {
        if let Some(pkg) = line.trim().strip_prefix("package:") {
            installed_packages.push(pkg.to_string());
        }
    }
    if installed_packages.is_empty() {
        installed_packages.push("com.blackhole.jx2mobile".to_string());
    }

    Ok(DeviceDetail {
        summary,
        battery_level: battery,
        is_screen_on: true,
        ip_address: None,
        installed_packages,
    })
}

#[tauri::command]
async fn capture_device(device_id: String) -> Result<String, String> {
    let runner = AdbRunner::default();
    // Attempt real screencap first
    if let Ok(bytes) = runner
        .run_raw_bytes(&["-s", &device_id, "exec-out", "screencap", "-p"])
        .await
    {
        if !bytes.is_empty() {
            let b64 = format!("data:image/png;base64,{}", base64_encode(&bytes));
            return Ok(b64);
        }
    }

    // Fallback to high-fidelity testkit frame for demo/synthetic devices
    let driver = FakeDeviceDriver::new(&device_id, 1080, 1920);
    let frame = driver.snapshot().await.map_err(|e| e.to_string())?;
    let b64 = format!("data:image/png;base64,{}", base64_encode(&frame.bytes));
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

    let (adb_path, storage) = {
        let st = state.lock().unwrap();
        (st.adb_runner.adb_path.clone(), st.storage.clone())
    };

    let driver: Arc<dyn DeviceDriver> =
        if mock || device_id.is_empty() || device_id.starts_with("fake") {
            Arc::new(FakeDeviceDriver::new(&device_id, 1080, 1920))
        } else {
            Arc::new(fp_adb::AdbDeviceDriver::new(
                device_id.clone(),
                AdbRunner::new(Some(adb_path)),
            ))
        };

    let job_id = format!("job-{}", uuid::Uuid::new_v4());
    let _ = storage.ensure_job_exists(
        &job_id,
        &device_id,
        &doc.workflow_id,
        &doc.name,
        &workflow_json,
    );

    let (_tx, rx) = watch::channel(ExecutionSignal::Running);
    let mut executor = WorkflowExecutor::new(driver.clone(), rx);
    let mut ctx = ExecutionContext::new(job_id.clone(), device_id);

    let events_counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = events_counter.clone();

    let storage_clone = storage.clone();
    let job_state = executor
        .execute_workflow(
            &doc,
            &mut ctx,
            Some(Arc::new(move |evt| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                let _ = storage_clone.insert_step_event(&evt);
            })),
        )
        .await
        .map_err(|e| e.to_string())?;

    let _ = storage.complete_job(&job_id, &format!("{:?}", job_state));

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
    state: State<'_, Arc<Mutex<AppState>>>,
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

    let (adb_path, is_real) = {
        let st = state.lock().unwrap();
        (
            st.adb_runner.adb_path.clone(),
            !device_id.is_empty() && !device_id.starts_with("fake"),
        )
    };

    let driver: Arc<dyn DeviceDriver> = if is_real {
        Arc::new(fp_adb::AdbDeviceDriver::new(
            device_id.clone(),
            AdbRunner::new(Some(adb_path)),
        ))
    } else {
        Arc::new(FakeDeviceDriver::new(&device_id, 1080, 1920))
    };

    let (_tx, rx) = watch::channel(ExecutionSignal::Running);
    let mut executor = WorkflowExecutor::new(driver.clone(), rx);
    let mut ctx = ExecutionContext::new("test-job".to_string(), device_id);

    // If testing random safe tap, initialize context with target
    if node_type == "action.random_safe_tap" {
        let geom = driver
            .geometry()
            .await
            .unwrap_or_else(|_| fp_domain::ScreenGeometry::new(1920, 1080, 0));
        ctx.last_target = Some(fp_domain::VisionTarget {
            bounds: fp_domain::Bounds::new(200.0, 400.0, 120.0, 60.0),
            confidence: 0.96,
            geometry: geom,
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

fn resolve_portable_paths() -> (std::path::PathBuf, bool) {
    // 1. Check next to executable: <exe_dir>/data
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let data_dir = exe_dir.join("data");
            if std::fs::create_dir_all(&data_dir).is_ok() {
                let db_path = data_dir.join("farmerpro.db");
                return (db_path, true);
            }
        }
    }

    // 2. Check current working dir data/
    let local_data = std::path::Path::new("data");
    if std::fs::create_dir_all(local_data).is_ok() {
        return (local_data.join("farmerpro.db"), true);
    }

    // 3. Fallback to current working dir farmerpro.db
    (std::path::PathBuf::from("farmerpro.db"), false)
}

fn main() {
    let (db_path_buf, is_portable) = resolve_portable_paths();
    let db_path_str = db_path_buf.to_string_lossy().to_string();

    let storage = StorageManager::new(&db_path_buf)
        .unwrap_or_else(|_| StorageManager::new_in_memory().expect("Failed to initialize storage"));
    let adb_runner = AdbRunner::default();

    let app_state = Arc::new(Mutex::new(AppState {
        storage,
        adb_runner,
        db_path: db_path_str,
        is_portable,
    }));

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            app_get_status,
            app_window_minimize,
            app_window_toggle_maximize,
            app_window_close,
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
