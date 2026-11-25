// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod element_ipc;
mod studio_p2;
mod studio_p3;

use fp_adb::AdbRunner;
use fp_domain::{ConnectionState, DeviceDetail, DeviceDriver, DeviceSummary, StepEvent};
use fp_security::CredentialVault;
use fp_storage::StorageManager;
use fp_testkit::FakeDeviceDriver;
use fp_workflow_runtime::{
    coordinator::RuntimeCoordinator, ExecutionContext, ExecutionSignal, WorkflowExecutor,
};
use fp_workflow_schema::{ValidationReport, WorkflowDocument, WorkflowNode, WorkflowValidator};
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::time::Duration;
use tauri::State;
use tokio::sync::watch;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppPackageInfo {
    pub package_name: String,
    pub display_name: String,
    pub is_game: bool,
    pub is_system: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LdPlayerInstance {
    pub index: u32,
    pub name: String,
    pub is_running: bool,
    pub pid: i32,
    pub top_handle: u64,
    pub width: u32,
    pub height: u32,
    pub dpi: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScanRangeResult {
    pub scanned_count: usize,
    pub connected_endpoints: Vec<String>,
}

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
    pub storage: Arc<StorageManager>,
    pub adb_runner: AdbRunner,
    pub coordinator: RuntimeCoordinator,
    pub vault: CredentialVault,
    pub db_path: String,
    pub projects_root: std::path::PathBuf,
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
        .run_args(&["-s", &device_id, "shell", "pm", "list", "packages", "-3"])
        .await
        .unwrap_or_default();
    let mut installed_packages = Vec::new();
    for line in packages_out.lines() {
        if let Some(pkg) = line.trim().strip_prefix("package:") {
            let clean = pkg.trim();
            if !clean.is_empty() {
                installed_packages.push(clean.to_string());
            }
        }
    }
    if installed_packages.is_empty() {
        // Fallback to all packages if no 3rd-party packages found
        if let Ok(all_pkgs) = runner
            .run_args(&["-s", &device_id, "shell", "pm", "list", "packages"])
            .await
        {
            for line in all_pkgs.lines() {
                if let Some(pkg) = line.trim().strip_prefix("package:") {
                    let clean = pkg.trim();
                    if !clean.is_empty() {
                        installed_packages.push(clean.to_string());
                    }
                }
            }
        }
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

    let (adb_path, storage, scheduler) = {
        let st = state.lock().unwrap();
        (
            st.adb_runner.adb_path.clone(),
            st.storage.clone(),
            st.coordinator.scheduler.clone(),
        )
    };

    let is_synthetic = mock || device_id.is_empty() || device_id.starts_with("fake");
    let driver: Arc<dyn DeviceDriver> = if is_synthetic {
        Arc::new(FakeDeviceDriver::new(&device_id, 1080, 1920))
    } else {
        Arc::new(fp_adb::AdbDeviceDriver::new(
            device_id.clone(),
            AdbRunner::new(Some(adb_path.clone())),
        ))
    };

    let job_id = format!("job-{}", uuid::Uuid::new_v4());
    let mut lease_guard = if !is_synthetic {
        let canon = if fp_adb::infer_local_emulator_canonical_key(&device_id).is_none()
            && !adb_path.trim().is_empty()
        {
            AdbRunner::new(Some(adb_path))
                .probe_canonical_device_identity(&device_id)
                .await
        } else {
            None
        };
        Some(
            scheduler
                .acquire_device_lease_guarded(
                    &device_id,
                    canon.as_deref(),
                    &job_id,
                    chrono::Duration::seconds(180),
                    fp_scheduler::ConflictPolicy::FailImmediately,
                )
                .map_err(|e| e.to_string())?,
        )
    } else {
        None
    };

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
    let exec_res = executor
        .execute_workflow(
            &doc,
            &mut ctx,
            Some(Arc::new(move |evt| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                let _ = storage_clone.insert_step_event(&evt);
            })),
        )
        .await;
    if let Some(guard) = lease_guard.take() {
        guard.release();
    }
    let job_state = match exec_res {
        Ok(s) => s,
        Err(e) => {
            let _ = storage.complete_job(&job_id, "Failed");
            return Err(e.to_string());
        }
    };

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

#[tauri::command]
async fn adb_connect_endpoint(
    endpoint: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<String, String> {
    let adb_path = {
        let st = state.lock().unwrap();
        st.adb_runner.adb_path.clone()
    };
    let runner = AdbRunner::new(Some(adb_path));
    let cleaned = endpoint.trim();
    if cleaned.is_empty() {
        return Err("Địa chỉ IP:Port không được để trống".to_string());
    }
    let res = runner.connect(cleaned).await.map_err(|e| e.to_string())?;
    Ok(res)
}

#[tauri::command]
async fn adb_disconnect_endpoint(
    endpoint: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<String, String> {
    let (adb_path, scheduler) = {
        let st = state.lock().unwrap();
        (
            st.adb_runner.adb_path.clone(),
            st.coordinator.scheduler.clone(),
        )
    };
    let runner = AdbRunner::new(Some(adb_path));
    let cleaned = endpoint.trim().to_string();
    if cleaned.is_empty() {
        return Err("Địa chỉ IP:Port không được để trống".to_string());
    }
    let res = runner
        .disconnect(&cleaned)
        .await
        .map_err(|e| e.to_string())?;
    let _ = scheduler.release_leases_for_disconnected_device(&cleaned);
    Ok(res)
}

#[tauri::command]
async fn adb_scan_range(
    subnet: Option<String>,
    port: Option<u16>,
    emulator_ports: Option<bool>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<ScanRangeResult, String> {
    let adb_path = {
        let st = state.lock().unwrap();
        st.adb_runner.adb_path.clone()
    };
    let runner = AdbRunner::new(Some(adb_path));

    let existing_raw = runner.list_devices_raw().await.unwrap_or_default();
    let attached_emu_loopback_ports: std::collections::HashSet<u16> = existing_raw
        .iter()
        .filter_map(|d| d.adb_serial.strip_prefix("emulator-"))
        .filter_map(|s| s.parse::<u16>().ok())
        .filter(|p| *p >= 5554 && p % 2 == 0)
        .map(|p| p + 1)
        .collect();

    let scan_emulators = emulator_ports.unwrap_or(true);
    let mut targets_to_probe: Vec<(String, u16)> = Vec::new();

    if scan_emulators {
        // Standard Android emulator & LDPlayer ports on localhost: 5555..=5595
        // Skip loopback ports whose `emulator-<port-1>` transport is already attached.
        for p in 5555..=5595 {
            if !attached_emu_loopback_ports.contains(&p) {
                targets_to_probe.push(("127.0.0.1".to_string(), p));
            }
        }
        // Nox ports
        for p in 62001..=62025 {
            targets_to_probe.push(("127.0.0.1".to_string(), p));
        }
        // Memu ports
        for p in (21503..=21523).step_by(10) {
            targets_to_probe.push(("127.0.0.1".to_string(), p));
        }
        // MuMu & WSA ports
        targets_to_probe.push(("127.0.0.1".to_string(), 7555));
        targets_to_probe.push(("127.0.0.1".to_string(), 58526));
    }

    if let Some(sub) = subnet {
        let sub_clean = sub.trim().trim_end_matches('.');
        if !sub_clean.is_empty() {
            let target_port = port.unwrap_or(5555);
            for i in 1..=254 {
                targets_to_probe.push((format!("{}.{}", sub_clean, i), target_port));
            }
        }
    }

    let scanned_count = targets_to_probe.len();
    let mut open_endpoints = Vec::new();

    let mut set = tokio::task::JoinSet::new();
    for (ip, p) in targets_to_probe {
        set.spawn(async move {
            let addr_str = format!("{}:{}", ip, p);
            if let Ok(addr) = addr_str.parse::<std::net::SocketAddr>() {
                if let Ok(Ok(_)) = tokio::time::timeout(
                    Duration::from_millis(220),
                    tokio::net::TcpStream::connect(addr),
                )
                .await
                {
                    return Some(addr_str);
                }
            }
            None
        });
    }

    while let Some(res) = set.join_next().await {
        if let Ok(Some(ep)) = res {
            open_endpoints.push(ep);
        }
    }

    let mut connected_endpoints = Vec::new();
    for ep in open_endpoints {
        if let Ok(msg) = runner.connect(&ep).await {
            if msg.contains("connected to") || msg.contains("already connected") {
                connected_endpoints.push(ep);
            }
        }
    }

    Ok(ScanRangeResult {
        scanned_count,
        connected_endpoints,
    })
}

#[tauri::command]
async fn auto_discover_devices(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<DeviceSummary>, String> {
    let adb_path = {
        let st = state.lock().unwrap();
        st.adb_runner.adb_path.clone()
    };
    let runner = AdbRunner::new(Some(adb_path));

    let existing_raw = runner.list_devices_raw().await.unwrap_or_default();
    let attached_emu_loopback_ports: std::collections::HashSet<u16> = existing_raw
        .iter()
        .filter_map(|d| d.adb_serial.strip_prefix("emulator-"))
        .filter_map(|s| s.parse::<u16>().ok())
        .filter(|p| *p >= 5554 && p % 2 == 0)
        .map(|p| p + 1)
        .collect();

    let mut ports_to_probe: Vec<u16> = (5555..=5595)
        .filter(|p| !attached_emu_loopback_ports.contains(p))
        .collect();
    ports_to_probe.extend_from_slice(&[62001, 62025, 21503, 21513, 21523, 7555, 58526]);

    let mut set = tokio::task::JoinSet::new();
    for p in ports_to_probe {
        set.spawn(async move {
            let addr_str = format!("127.0.0.1:{}", p);
            if let Ok(addr) = addr_str.parse::<std::net::SocketAddr>() {
                if let Ok(Ok(_)) = tokio::time::timeout(
                    Duration::from_millis(150),
                    tokio::net::TcpStream::connect(addr),
                )
                .await
                {
                    return Some(addr_str);
                }
            }
            None
        });
    }

    let mut open_addrs = Vec::new();
    while let Some(res) = set.join_next().await {
        if let Ok(Some(addr_str)) = res {
            open_addrs.push(addr_str);
        }
    }

    // Connect to all open emulator ports concurrently
    let mut conn_set = tokio::task::JoinSet::new();
    for addr_str in open_addrs {
        let r = runner.clone();
        conn_set.spawn(async move {
            let _ = r.connect(&addr_str).await;
        });
    }
    while conn_set.join_next().await.is_some() {}

    let devices = runner.list_devices().await.unwrap_or_default();
    let st = state.lock().unwrap();
    for d in &devices {
        let _ = st.storage.upsert_device(d);
    }

    Ok(devices)
}

fn resolve_app_info(pkg: &str) -> (String, bool) {
    let friendly = match pkg {
        "com.blackhole.jx2mobile" => Some("Võ Hiệp Truyền Kỳ 2 Mobile"),
        "com.vng.pubgmobile" => Some("PUBG Mobile VN"),
        "com.garena.game.kgvn" => Some("Liên Quân Mobile"),
        "com.vng.cuuam" => Some("Cửu Âm Chân Kinh VNG"),
        "com.vng.vltkm" => Some("Võ Lâm Truyền Kỳ Mobile"),
        "com.vng.vltkm2" => Some("Võ Lâm Truyền Kỳ 2 Mobile"),
        "com.playfun.kiemthe" => Some("Kiếm Thế Origin"),
        "com.miHoYo.GenshinImpact" => Some("Genshin Impact"),
        "com.riotgames.league.wildriftvn" => Some("Liên Minh: Tốc Chiến VN"),
        "com.dts.freefireth" => Some("Free Fire"),
        "com.garena.game.codm" => Some("Call of Duty Mobile"),
        "com.google.android.youtube" => Some("YouTube"),
        "com.android.chrome" => Some("Google Chrome"),
        "com.iot.hunonic" => Some("Hunonic Smart"),
        _ => None,
    };

    if let Some(name) = friendly {
        let is_game = !matches!(
            pkg,
            "com.google.android.youtube" | "com.android.chrome" | "com.iot.hunonic"
        );
        (format!("{} ({})", name, pkg), is_game)
    } else {
        let lower = pkg.to_lowercase();
        let is_game = lower.contains("game")
            || lower.contains("mobile")
            || lower.contains("play")
            || lower.contains("blackhole")
            || lower.contains("vng")
            || lower.contains("garena")
            || lower.contains("tencent")
            || lower.contains("jx2")
            || lower.contains("vltk");

        let last_part = pkg.split('.').next_back().unwrap_or(pkg);
        let mut capitalized = String::new();
        for word in last_part.split('_') {
            if !capitalized.is_empty() {
                capitalized.push(' ');
            }
            let mut chars = word.chars();
            if let Some(first) = chars.next() {
                capitalized.extend(first.to_uppercase());
                capitalized.push_str(chars.as_str());
            }
        }
        if capitalized.trim().is_empty() {
            capitalized = pkg.to_string();
        }
        (format!("{} ({})", capitalized, pkg), is_game)
    }
}

#[tauri::command]
async fn list_device_packages(
    device_id: String,
    user_only: Option<bool>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<AppPackageInfo>, String> {
    let adb_path = {
        let st = state.lock().unwrap();
        st.adb_runner.adb_path.clone()
    };
    let runner = AdbRunner::new(Some(adb_path));
    let user_apps_only = user_only.unwrap_or(true);

    let raw_packages = runner
        .list_packages(&device_id, user_apps_only)
        .await
        .unwrap_or_default();

    let mut result = Vec::new();
    for pkg in raw_packages {
        let (display_name, is_game) = resolve_app_info(&pkg);
        result.push(AppPackageInfo {
            package_name: pkg,
            display_name,
            is_game,
            is_system: !user_apps_only,
        });
    }

    result.sort_by(|a, b| {
        b.is_game.cmp(&a.is_game).then_with(|| {
            a.display_name
                .to_lowercase()
                .cmp(&b.display_name.to_lowercase())
        })
    });

    Ok(result)
}

#[tauri::command]
async fn launch_device_app(
    device_id: String,
    package_name: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<String, String> {
    let adb_path = {
        let st = state.lock().unwrap();
        st.adb_runner.adb_path.clone()
    };
    let driver = fp_adb::AdbDeviceDriver::new(device_id, AdbRunner::new(Some(adb_path)));
    driver
        .launch_app(&package_name)
        .await
        .map_err(|e| e.to_string())?;
    Ok(format!("Đã khởi chạy ứng dụng {}", package_name))
}

#[tauri::command]
async fn stop_device_app(
    device_id: String,
    package_name: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<String, String> {
    let adb_path = {
        let st = state.lock().unwrap();
        st.adb_runner.adb_path.clone()
    };
    let driver = fp_adb::AdbDeviceDriver::new(device_id, AdbRunner::new(Some(adb_path)));
    driver
        .stop_app(&package_name)
        .await
        .map_err(|e| e.to_string())?;
    Ok(format!("Đã tắt ứng dụng {}", package_name))
}

#[tauri::command]
async fn is_device_app_running(
    device_id: String,
    package_name: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<bool, String> {
    let adb_path = {
        let st = state.lock().unwrap();
        st.adb_runner.adb_path.clone()
    };
    let runner = AdbRunner::new(Some(adb_path));
    let out = runner
        .run_args(&["-s", &device_id, "shell", "pidof", &package_name])
        .await
        .unwrap_or_default();
    Ok(!out.trim().is_empty())
}

fn find_ldplayer_console() -> Option<std::path::PathBuf> {
    if let Ok(env_val) = std::env::var("LDPLAYER_PATH") {
        let p = std::path::PathBuf::from(env_val);
        if p.exists() {
            if p.is_file() {
                return Some(p);
            }
            let candidate = p.join("ldconsole.exe");
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    let candidates = [
        r"C:\LDPlayer\LDPlayer14\ldconsole.exe",
        r"C:\LDPlayer\LDPlayer9\ldconsole.exe",
        r"C:\LDPlayer\LDPlayer4\ldconsole.exe",
        r"C:\LDPlayer\LDPlayer\ldconsole.exe",
        r"C:\LDPlayer\LDPlayer14\dnconsole.exe",
        r"C:\LDPlayer\LDPlayer9\dnconsole.exe",
        r"C:\LDPlayer\LDPlayer4\dnconsole.exe",
        r"D:\LDPlayer\LDPlayer14\ldconsole.exe",
        r"D:\LDPlayer\LDPlayer9\ldconsole.exe",
        r"D:\LDPlayer\LDPlayer4\ldconsole.exe",
        r"D:\LDPlayer\LDPlayer\ldconsole.exe",
        r"E:\LDPlayer\LDPlayer14\ldconsole.exe",
        r"E:\LDPlayer\LDPlayer9\ldconsole.exe",
        r"E:\LDPlayer\LDPlayer4\ldconsole.exe",
        r"C:\leidian\LDPlayer9\ldconsole.exe",
        r"D:\leidian\LDPlayer9\ldconsole.exe",
        r"E:\leidian\LDPlayer9\ldconsole.exe",
    ];

    for path in &candidates {
        let pb = std::path::PathBuf::from(path);
        if pb.exists() {
            return Some(pb);
        }
    }
    None
}

#[tauri::command]
async fn ldplayer_list_instances() -> Result<Vec<LdPlayerInstance>, String> {
    let ldconsole = match find_ldplayer_console() {
        Some(p) => p,
        None => return Ok(Vec::new()),
    };

    let mut cmd = tokio::process::Command::new(ldconsole);
    #[cfg(windows)]
    {
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    cmd.arg("list2");

    let output = cmd.output().await.map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut list = Vec::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() >= 7 {
            let index = parts[0].trim().parse::<u32>().unwrap_or(0);
            let name = parts[1].trim().to_string();
            let top_handle = parts[2].trim().parse::<u64>().unwrap_or(0);
            let is_in_android = parts[4].trim() == "1";
            let pid = parts[5].trim().parse::<i32>().unwrap_or(-1);
            let width = if parts.len() > 7 {
                parts[7].trim().parse::<u32>().unwrap_or(1920)
            } else {
                1920
            };
            let height = if parts.len() > 8 {
                parts[8].trim().parse::<u32>().unwrap_or(1080)
            } else {
                1080
            };
            let dpi = if parts.len() > 9 {
                parts[9].trim().parse::<u32>().unwrap_or(280)
            } else {
                280
            };

            list.push(LdPlayerInstance {
                index,
                name,
                is_running: is_in_android || pid > 0,
                pid,
                top_handle,
                width,
                height,
                dpi,
            });
        }
    }

    Ok(list)
}

#[tauri::command]
async fn ldplayer_action(action: String, index: Option<u32>) -> Result<String, String> {
    let ldconsole = find_ldplayer_console()
        .ok_or_else(|| "Không tìm thấy đường dẫn LDPlayer (ldconsole.exe)".to_string())?;

    let idx = index.unwrap_or(0);
    let action_lower = action.to_lowercase();

    match action_lower.as_str() {
        "launch" | "activate" => {
            let mut cmd = tokio::process::Command::new(&ldconsole);
            #[cfg(windows)]
            {
                cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
            }
            cmd.args(["launch", "--index", &idx.to_string()]);
            cmd.output().await.map_err(|e| e.to_string())?;
            Ok(format!(
                "Đã yêu cầu mở / kích hoạt LDPlayer instance {}",
                idx
            ))
        }
        "reboot" => {
            let mut cmd = tokio::process::Command::new(&ldconsole);
            #[cfg(windows)]
            {
                cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
            }
            cmd.args(["reboot", "--index", &idx.to_string()]);
            cmd.output().await.map_err(|e| e.to_string())?;
            Ok(format!(
                "Đã yêu cầu khởi động lại LDPlayer instance {}",
                idx
            ))
        }
        "quit" | "kill" => {
            let mut cmd = tokio::process::Command::new(&ldconsole);
            #[cfg(windows)]
            {
                cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
            }
            cmd.args(["quit", "--index", &idx.to_string()]);
            let _ = cmd.output().await;
            Ok(format!("Đã tắt LDPlayer instance {}", idx))
        }
        "kill_all" => {
            let mut cmd = tokio::process::Command::new(&ldconsole);
            #[cfg(windows)]
            {
                cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
            }
            cmd.arg("quitall");
            let _ = cmd.output().await;

            #[cfg(windows)]
            {
                let mut kill_cmd = tokio::process::Command::new("taskkill");
                kill_cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
                kill_cmd.args([
                    "/F",
                    "/IM",
                    "dnplayer.exe",
                    "/IM",
                    "Ld9BoxHeadless.exe",
                    "/IM",
                    "LdVBoxHeadless.exe",
                    "/IM",
                    "LdBoxHeadless.exe",
                    "/IM",
                    "dnmultiplayer.exe",
                ]);
                let _ = kill_cmd.output().await;
            }

            Ok("Đã tắt toàn bộ tiến trình LDPlayer".to_string())
        }
        _ => Err(format!("Hành động không hợp lệ: {}", action)),
    }
}

#[tauri::command]
fn ldplayer_detect_path() -> Result<Option<String>, String> {
    Ok(find_ldplayer_console().map(|p| p.to_string_lossy().to_string()))
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

fn is_dir_writable(dir: &std::path::Path) -> bool {
    if std::fs::create_dir_all(dir).is_err() {
        return false;
    }
    let probe = dir.join(format!(".fp_write_probe_{}", std::process::id()));
    match std::fs::write(&probe, b"ok") {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

fn resolve_portable_paths() -> (std::path::PathBuf, bool) {
    if let Ok(override_dir) = std::env::var("FARMERPRO_DATA_DIR") {
        let trimmed = override_dir.trim();
        if !trimmed.is_empty() {
            let data_dir = std::path::PathBuf::from(trimmed);
            if std::fs::create_dir_all(&data_dir).is_ok() {
                return (data_dir.join("farmerpro.db"), false);
            }
        }
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            // If running from an NSIS installation (uninstall.exe present), ALWAYS isolate user DB/projects
            // inside %LOCALAPPDATA%\FarmerPro\StudioData so upgrades & uninstalls NEVER delete user data!
            let is_nsis_installed = exe_dir.join("uninstall.exe").exists()
                || exe_dir.join("Uninstall.exe").exists()
                || exe_dir.join(".nsis_installed").exists();
            if is_nsis_installed {
                let appdata = std::env::var("LOCALAPPDATA")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|_| exe_dir.to_path_buf());
                let user_data_dir = appdata.join("FarmerPro").join("StudioData").join("data");
                if is_dir_writable(&user_data_dir) {
                    return (user_data_dir.join("farmerpro.db"), false);
                }
            }

            // Otherwise (Portable / Sandbox mode): check next to executable <exe_dir>/data
            let data_dir = exe_dir.join("data");
            if is_dir_writable(&data_dir) {
                return (data_dir.join("farmerpro.db"), true);
            }
        }
    }

    // Standard fallback: %LOCALAPPDATA%\FarmerPro\StudioData\data
    if let Ok(local_app) = std::env::var("LOCALAPPDATA") {
        let fallback_data = std::path::PathBuf::from(local_app)
            .join("FarmerPro")
            .join("StudioData")
            .join("data");
        if is_dir_writable(&fallback_data) {
            return (fallback_data.join("farmerpro.db"), false);
        }
    }

    let local_data = std::path::Path::new("data");
    if is_dir_writable(local_data) {
        return (local_data.join("farmerpro.db"), true);
    }

    (std::path::PathBuf::from("farmerpro.db"), false)
}

fn resolve_projects_root(db_path: &std::path::Path) -> std::path::PathBuf {
    if let Ok(env_dir) = std::env::var("FARMERPRO_PROJECTS_DIR") {
        if !env_dir.trim().is_empty() {
            return std::path::PathBuf::from(env_dir);
        }
    }
    if let Some(parent) = db_path.parent() {
        if parent.file_name().and_then(|n| n.to_str()) == Some("data") {
            if let Some(root) = parent.parent() {
                return root.join("projects");
            }
        }
    }
    std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("projects")
}

fn main() {
    let (db_path_buf, is_portable) = resolve_portable_paths();
    let db_path_str = db_path_buf.to_string_lossy().to_string();
    let projects_root = resolve_projects_root(&db_path_buf);

    let storage = match StorageManager::new(&db_path_buf) {
        Ok(s) => Arc::new(s),
        Err(err) => {
            eprintln!(
                "CRITICAL SQLITE DATABASE INITIALIZATION FAILURE ({}): {:#}",
                db_path_buf.display(),
                err
            );
            std::process::exit(4);
        }
    };
    let _ = storage.reconcile_projects_storage(&projects_root);
    let adb_runner = AdbRunner::default();
    let coordinator = RuntimeCoordinator::new(
        Arc::clone(&storage),
        std::path::PathBuf::from(&adb_runner.adb_path),
    );
    let _ = storage.recover_interrupted_jobs_on_startup(|dev_id| {
        let canon = fp_adb::infer_local_emulator_canonical_key(dev_id);
        match coordinator
            .scheduler
            .inspect_os_lock_holder(dev_id, canon.as_deref())
        {
            Ok(Some(_)) => true,
            Ok(None) => false,
            Err(_) => true,
        }
    });
    let vault = CredentialVault::new();

    let app_state = Arc::new(Mutex::new(AppState {
        storage,
        adb_runner,
        coordinator,
        vault,
        db_path: db_path_str,
        projects_root,
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
            execute_test_node,
            adb_connect_endpoint,
            adb_disconnect_endpoint,
            adb_scan_range,
            auto_discover_devices,
            list_device_packages,
            launch_device_app,
            stop_device_app,
            is_device_app_running,
            ldplayer_list_instances,
            ldplayer_action,
            ldplayer_detect_path,
            studio_p2::studio_list_projects,
            studio_p2::studio_get_project_detail,
            studio_p2::studio_create_project,
            studio_p2::studio_update_project,
            studio_p2::studio_clone_project,
            studio_p2::studio_delete_project,
            studio_p2::studio_list_project_workflows,
            studio_p2::studio_save_project_workflow,
            studio_p2::studio_list_project_templates,
            studio_p2::studio_list_project_tasks,
            studio_p2::studio_upsert_project_task,
            studio_p2::studio_delete_project_task,
            studio_p2::studio_list_project_modules,
            studio_p2::studio_upsert_project_module,
            studio_p2::studio_get_ui_schema,
            studio_p2::studio_save_ui_schema,
            studio_p2::studio_list_project_accounts,
            studio_p2::studio_upsert_project_account,
            studio_p2::studio_vault_set_secret,
            studio_p2::studio_vault_has_secret,
            studio_p2::studio_vault_list_namespace,
            studio_p2::studio_vault_delete_secret,
            studio_p2::studio_get_producer_key_status,
            studio_p2::studio_rotate_producer_key,
            studio_p2::studio_backup_producer_key,
            studio_p2::studio_restore_producer_key,
            studio_p2::studio_export_portable_producer_key,
            studio_p2::studio_import_portable_producer_key,
            studio_p2::studio_backup_database,
            studio_p2::studio_check_db_integrity,
            studio_p2::studio_optimize_db,
            studio_p2::studio_migrate_legacy_jx2,
            studio_p3::studio_run_project_workflow,
            studio_p3::studio_start_project_workflow,
            studio_p3::studio_pause_job,
            studio_p3::studio_resume_job,
            studio_p3::studio_suspend_job_to_checkpoint,
            studio_p3::studio_cancel_job,
            studio_p3::studio_resume_job_from_checkpoint,
            studio_p3::studio_list_project_jobs,
            studio_p3::studio_get_job_resume_detail,
            studio_p3::studio_list_active_device_leases,
            studio_p3::studio_probe_device_health,
            studio_p3::studio_reconnect_device,
            studio_p3::studio_list_device_ports,
            studio_p3::studio_add_device_forward,
            studio_p3::studio_remove_device_forward,
            studio_p3::studio_add_device_reverse,
            studio_p3::studio_remove_device_reverse,
            studio_p3::studio_crop_and_save_template,
            studio_p3::studio_capture_and_crop_template,
            studio_p3::studio_test_project_template_match,
            studio_p3::studio_scale_roi_for_geometry,
            studio_p3::studio_probe_project_proxy,
            studio_p3::studio_builder_preflight,
            studio_p3::studio_builder_export_release,
            studio_p3::studio_ensure_project_icon,
            studio_p3::studio_list_publisher_profiles,
            studio_p3::studio_upsert_publisher_profile,
            studio_p3::studio_delete_publisher_profile,
            studio_p3::studio_create_self_signed_qa_cert,
            studio_p3::studio_signing_preflight,
            studio_p3::studio_discover_signing_toolchain,
            studio_p3::studio_verify_final_release,
            studio_p3::studio_rotate_project_ed25519_key_with_delegation,
            studio_p3::studio_get_project_runtime_vars,
            studio_p3::studio_list_project_releases,
            studio_p3::studio_check_release_compatibility,
            studio_p3::studio_transition_release_lifecycle,
            studio_p3::studio_list_project_release_transitions,
            studio_p3::studio_deploy_minitool_release,
            studio_p3::studio_rollback_minitool_install,
            studio_p3::studio_reconcile_minitool_install,
            studio_p3::studio_inspect_minitool_install,
            element_ipc::element_inspect_snapshot,
            element_ipc::element_find_matches,
            element_ipc::element_generate_selectors,
            element_ipc::element_find_at_point,
            element_ipc::element_test_detection,
            element_ipc::element_execute_action,
            element_ipc::element_save_to_library,
            element_ipc::element_list_library,
            element_ipc::element_delete_from_library,
            element_ipc::device_press_key,
            element_ipc::device_tap,
            element_ipc::device_swipe,
            element_ipc::device_input_text,
            element_ipc::device_stream_frame,
            element_ipc::recorder_record_touch,
            element_ipc::recorder_replay_step,
            element_ipc::recorder_replay_all,
            element_ipc::recorder_export_workflow,
            element_ipc::recorder_save_workflow
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
