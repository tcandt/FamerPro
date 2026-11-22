use crate::AppState;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use fp_actions::{SafeRegionSampler, SafeTapPolicy};
use fp_adb::{validate_adb_token, AdbDeviceDriver, AdbRunner};
use fp_builder::{BuildPreflightReport, BuildReport, MiniToolPackager};
use fp_bundle_verify::{sha256_hex, validate_safe_relative_path};
use fp_domain::{
    Bounds, DeviceDriver, DeviceHealthReport, DevicePortMapping, Frame, Point, ScreenGeometry,
    StepEvent,
};
use fp_proxy::{probe_proxy, ProxyConfig, ProxyProbeReport};
use fp_scheduler::DeviceLease;
use fp_storage::{
    JobResumeContext, ProjectJobRecord, ProjectReleaseRecord, StorageManager, VisionTemplateInput,
    VisionTemplateRecord,
};
use fp_vision::{
    annotate_match_and_tap_png, crop_roi_png, scale_bounds_for_geometry, RoiRect, VisionEngine,
};
use fp_workflow_runtime::coordinator::{
    RunProjectWorkflowRequest, RunWorkflowExecutionSummary, RuntimeCoordinator,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::State;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundJobStarted {
    pub job_id: String,
    pub device_id: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobResumeDetail {
    pub resume_context: JobResumeContext,
    pub step_events: Vec<StepEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdbPortMappingsOverview {
    pub serial: String,
    pub forwards: Vec<DevicePortMapping>,
    pub reverses: Vec<DevicePortMapping>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateMatchPreviewReport {
    pub project_id: String,
    pub template_id: String,
    pub template_name: String,
    pub matched: bool,
    pub confidence: Option<f32>,
    pub bounds: Option<Bounds>,
    pub suggested_tap_point: Option<Point>,
    pub annotated_png_base64: Option<String>,
    pub saved_annotated_path: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuilderReleaseExportResult {
    pub release: ProjectReleaseRecord,
    pub build_report: BuildReport,
}

// =========================================================================
// Pure testable handler functions (Runtime, Vision/ROI, Proxy, Builder)
// =========================================================================

pub(crate) fn handle_list_project_jobs(
    storage: &StorageManager,
    project_id: &str,
) -> Result<Vec<ProjectJobRecord>, String> {
    storage
        .list_project_jobs(project_id)
        .map_err(|e| e.to_string())
}

pub(crate) fn handle_get_job_resume_detail(
    storage: &StorageManager,
    job_id: &str,
) -> Result<Option<JobResumeDetail>, String> {
    let ctx = storage
        .load_job_resume_context(job_id)
        .map_err(|e| e.to_string())?;
    let Some(resume_context) = ctx else {
        return Ok(None);
    };
    let step_events = storage
        .list_step_events(Some(job_id))
        .map_err(|e| e.to_string())?;
    Ok(Some(JobResumeDetail {
        resume_context,
        step_events,
    }))
}

pub(crate) fn handle_crop_and_save_project_template(
    storage: &StorageManager,
    project_id: &str,
    name: &str,
    rel_path: &str,
    frame_png_bytes: &[u8],
    roi: RoiRect,
    default_confidence: Option<f64>,
) -> Result<VisionTemplateRecord, String> {
    let (cropped_png, w, h) = crop_roi_png(frame_png_bytes, roi).map_err(|e| e.to_string())?;
    let clean_rel = if rel_path.trim().is_empty() {
        format!(
            "templates/{}.png",
            name.trim().to_lowercase().replace(' ', "_")
        )
    } else {
        rel_path.to_string()
    };
    let sha = sha256_hex(&cropped_png);
    storage
        .save_project_template_file(
            VisionTemplateInput {
                id: None,
                project_id: project_id.to_string(),
                name: name.to_string(),
                rel_path: clean_rel,
                width_px: w,
                height_px: h,
                source_crop_x: Some(roi.x as i32),
                source_crop_y: Some(roi.y as i32),
                default_confidence,
                sha256: sha,
            },
            &cropped_png,
        )
        .map_err(|e| e.to_string())
}

pub(crate) fn handle_test_project_template_match(
    storage: &StorageManager,
    project_id: &str,
    template_id: &str,
    frame_png_bytes: &[u8],
    frame_width: u32,
    frame_height: u32,
    min_confidence: Option<f32>,
    save_annotated_rel_path: Option<&str>,
) -> Result<TemplateMatchPreviewReport, String> {
    let project = storage
        .get_project(project_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("GameProject '{}' not found", project_id))?;

    let templates = storage
        .list_vision_templates(project_id)
        .map_err(|e| e.to_string())?;
    let tpl = templates
        .into_iter()
        .find(|t| t.id == template_id)
        .ok_or_else(|| {
            format!(
                "PROJECT_ISOLATION_VIOLATION: Template '{}' not found inside project '{}'",
                template_id, project_id
            )
        })?;

    let proj_dir = Path::new(&project.project_dir_path);
    let tpl_path =
        validate_safe_relative_path(proj_dir, &tpl.rel_path).map_err(|e| e.to_string())?;
    let tpl_bytes = std::fs::read(&tpl_path).map_err(|e| e.to_string())?;

    let frame = Frame {
        bytes: frame_png_bytes.to_vec(),
        geometry: ScreenGeometry::new(frame_width, frame_height, 0),
        timestamp_ms: chrono::Utc::now().timestamp_millis() as u64,
    };
    let engine = VisionEngine::new();
    let conf = min_confidence.unwrap_or(tpl.default_confidence as f32);

    match engine.find_template_multiscale(&frame, &tpl_bytes, Some(conf), &[0.85, 1.0, 1.15]) {
        Ok(target) => {
            let mut sampler = SafeRegionSampler::new(SafeTapPolicy::default());
            let screen_bounds = Bounds::new(0.0, 0.0, frame_width as f32, frame_height as f32);
            let pt = sampler
                .sample_safe_point(&target.bounds, &screen_bounds, &[])
                .ok();
            let annotated = annotate_match_and_tap_png(frame_png_bytes, target.bounds, pt)
                .map_err(|e| e.to_string())?;

            let saved_annotated_path = if let Some(rel) = save_annotated_rel_path {
                let dst = validate_safe_relative_path(proj_dir, rel).map_err(|e| e.to_string())?;
                if let Some(parent) = dst.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                std::fs::write(&dst, &annotated).map_err(|e| e.to_string())?;
                Some(dst.to_string_lossy().to_string())
            } else {
                None
            };

            Ok(TemplateMatchPreviewReport {
                project_id: project_id.to_string(),
                template_id: tpl.id,
                template_name: tpl.name,
                matched: true,
                confidence: Some(target.confidence),
                bounds: Some(target.bounds),
                suggested_tap_point: pt,
                annotated_png_base64: Some(B64.encode(&annotated)),
                saved_annotated_path,
                error: None,
            })
        }
        Err(err) => Ok(TemplateMatchPreviewReport {
            project_id: project_id.to_string(),
            template_id: tpl.id,
            template_name: tpl.name,
            matched: false,
            confidence: None,
            bounds: None,
            suggested_tap_point: None,
            annotated_png_base64: None,
            saved_annotated_path: None,
            error: Some(err.to_string()),
        }),
    }
}

pub(crate) async fn handle_probe_project_proxy(
    storage: &StorageManager,
    project_id: &str,
    config: &ProxyConfig,
) -> Result<ProxyProbeReport, String> {
    let _proj = storage
        .get_project(project_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("GameProject '{}' not found", project_id))?;

    let modules = storage
        .list_project_modules(project_id)
        .map_err(|e| e.to_string())?;
    let proxy_mod = modules.iter().find(|m| m.module_id == "mod.proxy_network");
    let proxy_allowed = match proxy_mod {
        Some(m) => m.enabled_by_default || m.packaged_in_release,
        None => true,
    };
    if !proxy_allowed {
        return Err(format!(
            "MODULE_DISABLED: Project '{}' does not have 'mod.proxy_network' enabled",
            project_id
        ));
    }

    Ok(probe_proxy(config).await)
}

pub(crate) fn handle_builder_preflight(
    storage: &StorageManager,
    project_id: &str,
    runtime_exe: &Path,
    workspace_bin_dir: Option<&Path>,
    keystore_dir: Option<&Path>,
) -> Result<BuildPreflightReport, String> {
    let proj = storage
        .get_project(project_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("GameProject '{}' not found", project_id))?;
    MiniToolPackager::preflight_validate_project(
        Path::new(&proj.project_dir_path),
        runtime_exe,
        workspace_bin_dir,
        keystore_dir,
    )
    .map_err(|e| e.to_string())
}

pub(crate) fn handle_builder_export_release(
    storage: &StorageManager,
    project_id: &str,
    runtime_exe: &Path,
    workspace_bin_dir: Option<&Path>,
    output_release_dir: &Path,
    keystore_dir: Option<&Path>,
) -> Result<BuilderReleaseExportResult, String> {
    let proj = storage
        .get_project(project_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("GameProject '{}' not found", project_id))?;

    let preflight = MiniToolPackager::preflight_validate_project(
        Path::new(&proj.project_dir_path),
        runtime_exe,
        workspace_bin_dir,
        keystore_dir,
    )
    .map_err(|e| e.to_string())?;
    if !preflight.ready {
        return Err(format!(
            "BUILDER_PREFLIGHT_FAILED: {}",
            preflight.issues.join("; ")
        ));
    }

    let report = MiniToolPackager::build_and_export_release(
        Path::new(&proj.project_dir_path),
        runtime_exe,
        workspace_bin_dir,
        output_release_dir,
        keystore_dir,
    )
    .map_err(|e| e.to_string())?;

    let release = ProjectReleaseRecord {
        id: format!("rel-{}", Uuid::new_v4()),
        project_id: proj.id.clone(),
        version: proj.version.clone(),
        output_dir_path: output_release_dir.to_string_lossy().to_string(),
        exe_path: report.tool_exe_path.clone(),
        exe_size_bytes: report.tool_exe_size_bytes,
        exe_sha256: report.tool_exe_sha256.clone(),
        key_id: report.key_id.clone(),
        publisher_public_key_hex: report.publisher_public_key_hex.clone(),
        pe_rcdata_pubkey_id: report.pe_embedded_pubkey_resource_id as i64,
        signature_ed25519_hex: report.signature_ed25519_hex.clone(),
        build_report_json: serde_json::to_string(&report).map_err(|e| e.to_string())?,
        built_at: chrono::Utc::now().to_rfc3339(),
    };

    storage
        .record_project_release(release.clone())
        .map_err(|e| e.to_string())?;

    Ok(BuilderReleaseExportResult {
        release,
        build_report: report,
    })
}

fn resolve_runtime_and_bin_paths() -> (PathBuf, Option<PathBuf>) {
    let mut exe_candidates: Vec<PathBuf> = Vec::new();
    if let Ok(cur_exe) = std::env::current_exe() {
        if let Some(dir) = cur_exe.parent() {
            exe_candidates.push(dir.join("fp-mini-runtime.exe"));
        }
    }
    exe_candidates.push(PathBuf::from("target/release/fp-mini-runtime.exe"));
    exe_candidates.push(PathBuf::from("target/debug/fp-mini-runtime.exe"));
    exe_candidates.push(PathBuf::from("dist-portable/FarmerPro/fp-mini-runtime.exe"));

    let runtime_exe = exe_candidates
        .into_iter()
        .find(|p| p.exists())
        .unwrap_or_else(|| PathBuf::from("target/debug/fp-mini-runtime.exe"));

    let mut bin_candidates: Vec<PathBuf> = vec![PathBuf::from("bin")];
    if let Ok(cur_exe) = std::env::current_exe() {
        if let Some(dir) = cur_exe.parent() {
            bin_candidates.push(dir.join("bin"));
        }
    }
    bin_candidates.push(PathBuf::from("dist-portable/FarmerPro/bin"));
    let workspace_bin_dir = bin_candidates
        .into_iter()
        .find(|p| p.join("adb.exe").exists());

    (runtime_exe, workspace_bin_dir)
}

// =========================================================================
// Tauri P3 Commands — Runtime Coordinator & Job Control
// =========================================================================

#[tauri::command]
pub async fn studio_run_project_workflow(
    req: RunProjectWorkflowRequest,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<RunWorkflowExecutionSummary, String> {
    let coord = {
        let st = state.lock().unwrap();
        st.coordinator.clone()
    };
    coord
        .run_project_workflow(req, None)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn studio_start_project_workflow(
    req: RunProjectWorkflowRequest,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<BackgroundJobStarted, String> {
    let coord = {
        let st = state.lock().unwrap();
        st.coordinator.clone()
    };
    let (handle, _join) = coord
        .start_project_workflow_background(req, None)
        .await
        .map_err(|e| e.to_string())?;
    Ok(BackgroundJobStarted {
        job_id: handle.job_id,
        device_id: handle.device_id,
        state: "running".to_string(),
    })
}

#[tauri::command]
pub fn studio_pause_job(
    job_id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let coord = state.lock().unwrap().coordinator.clone();
    coord.pause_job(&job_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_resume_job(
    job_id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let coord = state.lock().unwrap().coordinator.clone();
    coord.resume_job(&job_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_suspend_job_to_checkpoint(
    job_id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let coord = state.lock().unwrap().coordinator.clone();
    coord
        .suspend_job_to_checkpoint(&job_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_cancel_job(
    job_id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let coord = state.lock().unwrap().coordinator.clone();
    coord.cancel_job(&job_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn studio_resume_job_from_checkpoint(
    job_id: String,
    lease_ttl_secs: Option<u64>,
    workflow_timeout_ms: Option<u64>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<RunWorkflowExecutionSummary, String> {
    let coord = {
        let st = state.lock().unwrap();
        st.coordinator.clone()
    };
    coord
        .resume_from_checkpoint(&job_id, lease_ttl_secs.unwrap_or(30), workflow_timeout_ms, None)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_list_project_jobs(
    project_id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<ProjectJobRecord>, String> {
    let st = state.lock().unwrap();
    handle_list_project_jobs(&st.storage, &project_id)
}

#[tauri::command]
pub fn studio_get_job_resume_detail(
    job_id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Option<JobResumeDetail>, String> {
    let st = state.lock().unwrap();
    handle_get_job_resume_detail(&st.storage, &job_id)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveDeviceLeaseView {
    pub device_id: String,
    pub job_id: String,
    pub generation: u64,
    pub expires_at: String,
}

#[tauri::command]
pub fn studio_list_active_device_leases(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<ActiveDeviceLeaseView>, String> {
    let st = state.lock().unwrap();
    let leases: Vec<DeviceLease> = st.coordinator.active_device_leases();
    Ok(leases
        .into_iter()
        .map(|l| ActiveDeviceLeaseView {
            device_id: l.device_id,
            job_id: l.job_id,
            generation: l.generation,
            expires_at: l.expires_at.to_rfc3339(),
        })
        .collect())
}

// =========================================================================
// Tauri P3 Commands — Device Lab Health, Reconnect & ADB Port Mappings
// =========================================================================

#[tauri::command]
pub async fn studio_probe_device_health(
    serial: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<DeviceHealthReport, String> {
    let runner = state.lock().unwrap().adb_runner.clone();
    Ok(runner.probe_health(&serial).await)
}

#[tauri::command]
pub async fn studio_reconnect_device(
    endpoint: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<DeviceHealthReport, String> {
    let runner = state.lock().unwrap().adb_runner.clone();
    runner.reconnect(&endpoint).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn studio_list_device_ports(
    serial: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<AdbPortMappingsOverview, String> {
    let runner = state.lock().unwrap().adb_runner.clone();
    let forwards = runner
        .forward_list(Some(&serial))
        .await
        .map_err(|e| e.to_string())?;
    let reverses = runner.reverse_list(&serial).await.map_err(|e| e.to_string())?;
    Ok(AdbPortMappingsOverview {
        serial,
        forwards,
        reverses,
        note: "ADB forward/reverse manages host<->device local sockets only and does not act as an outbound game SOCKS5/HTTP proxy."
            .to_string(),
    })
}

#[tauri::command]
pub async fn studio_add_device_forward(
    serial: String,
    local_spec: String,
    remote_spec: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<DevicePortMapping, String> {
    let runner = state.lock().unwrap().adb_runner.clone();
    runner
        .forward_add(&serial, &local_spec, &remote_spec)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn studio_remove_device_forward(
    serial: String,
    local_spec: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let runner = state.lock().unwrap().adb_runner.clone();
    runner
        .forward_remove(&serial, &local_spec)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn studio_add_device_reverse(
    serial: String,
    remote_spec: String,
    local_spec: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<DevicePortMapping, String> {
    let runner = state.lock().unwrap().adb_runner.clone();
    runner
        .reverse_add(&serial, &remote_spec, &local_spec)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn studio_remove_device_reverse(
    serial: String,
    remote_spec: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let runner = state.lock().unwrap().adb_runner.clone();
    runner
        .reverse_remove(&serial, &remote_spec)
        .await
        .map_err(|e| e.to_string())
}

// =========================================================================
// Tauri P3 Commands — Vision ROI Capture, Template Match & Scaling
// =========================================================================

#[tauri::command]
pub fn studio_crop_and_save_template(
    project_id: String,
    name: String,
    rel_path: String,
    frame_png_base64: String,
    roi: RoiRect,
    default_confidence: Option<f64>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<VisionTemplateRecord, String> {
    let png_bytes = B64
        .decode(frame_png_base64.trim())
        .map_err(|e| format!("Invalid base64 PNG input: {}", e))?;
    let st = state.lock().unwrap();
    handle_crop_and_save_project_template(
        &st.storage,
        &project_id,
        &name,
        &rel_path,
        &png_bytes,
        roi,
        default_confidence,
    )
}

#[tauri::command]
pub async fn studio_capture_and_crop_template(
    project_id: String,
    serial: String,
    name: String,
    rel_path: String,
    roi: RoiRect,
    default_confidence: Option<f64>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<VisionTemplateRecord, String> {
    let clean_serial = validate_adb_token("serial", &serial).map_err(|e| e.to_string())?;
    let (runner, storage) = {
        let st = state.lock().unwrap();
        (st.adb_runner.clone(), Arc::clone(&st.storage))
    };
    let driver = AdbDeviceDriver::new(clean_serial, runner);
    let frame = driver.snapshot().await.map_err(|e| e.to_string())?;
    handle_crop_and_save_project_template(
        &storage,
        &project_id,
        &name,
        &rel_path,
        &frame.bytes,
        roi,
        default_confidence,
    )
}

#[tauri::command]
pub fn studio_test_project_template_match(
    project_id: String,
    template_id: String,
    frame_png_base64: String,
    frame_width: u32,
    frame_height: u32,
    min_confidence: Option<f32>,
    save_annotated_rel_path: Option<String>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<TemplateMatchPreviewReport, String> {
    let png_bytes = B64
        .decode(frame_png_base64.trim())
        .map_err(|e| format!("Invalid base64 PNG input: {}", e))?;
    let st = state.lock().unwrap();
    handle_test_project_template_match(
        &st.storage,
        &project_id,
        &template_id,
        &png_bytes,
        frame_width,
        frame_height,
        min_confidence,
        save_annotated_rel_path.as_deref(),
    )
}

#[tauri::command]
pub fn studio_scale_roi_for_geometry(
    bounds: Bounds,
    from_width: u32,
    from_height: u32,
    to_width: u32,
    to_height: u32,
) -> Result<Bounds, String> {
    let from = ScreenGeometry::new(from_width, from_height, 0);
    let to = ScreenGeometry::new(to_width, to_height, 0);
    scale_bounds_for_geometry(bounds, &from, &to).map_err(|e| e.to_string())
}

// =========================================================================
// Tauri P3 Commands — Proxy Probe & Mini Tool Builder IPC
// =========================================================================

#[tauri::command]
pub async fn studio_probe_project_proxy(
    project_id: String,
    config: ProxyConfig,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<ProxyProbeReport, String> {
    let storage = Arc::clone(&state.lock().unwrap().storage);
    handle_probe_project_proxy(&storage, &project_id, &config).await
}

#[tauri::command]
pub fn studio_builder_preflight(
    project_id: String,
    runtime_exe_override: Option<String>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<BuildPreflightReport, String> {
    let (default_runtime, bin_dir) = resolve_runtime_and_bin_paths();
    let runtime_exe = runtime_exe_override
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or(default_runtime);
    let st = state.lock().unwrap();
    handle_builder_preflight(&st.storage, &project_id, &runtime_exe, bin_dir.as_deref(), None)
}

#[tauri::command]
pub fn studio_builder_export_release(
    project_id: String,
    output_dir: Option<String>,
    runtime_exe_override: Option<String>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<BuilderReleaseExportResult, String> {
    let (default_runtime, bin_dir) = resolve_runtime_and_bin_paths();
    let runtime_exe = runtime_exe_override
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or(default_runtime);

    let st = state.lock().unwrap();
    let proj = st
        .storage
        .get_project(&project_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("GameProject '{}' not found", project_id))?;

    let release_dir = output_dir
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            st.projects_root
                .join(&proj.slug)
                .join("releases")
                .join(format!("v{}", proj.version))
        });

    handle_builder_export_release(
        &st.storage,
        &project_id,
        &runtime_exe,
        bin_dir.as_deref(),
        &release_dir,
        None,
    )
}

#[tauri::command]
pub fn studio_list_project_releases(
    project_id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<ProjectReleaseRecord>, String> {
    let st = state.lock().unwrap();
    st.storage
        .list_project_releases(&project_id)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use fp_proxy::ProxyProtocol;
    use fp_security::CredentialVault;
    use fp_storage::CreateProjectRequest;
    use fp_testkit::FakeDeviceDriver;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    fn build_test_ico_32x32() -> Vec<u8> {
        let mut ico = Vec::new();
        // ICONDIR: reserved(0), type(1=ICO), count(1)
        ico.extend_from_slice(&0u16.to_le_bytes());
        ico.extend_from_slice(&1u16.to_le_bytes());
        ico.extend_from_slice(&1u16.to_le_bytes());

        // BMP payload inside ICO: 40-byte BITMAPINFOHEADER + 32*32*4 RGBA + 32*4 AND mask
        let mut bmp = Vec::new();
        bmp.extend_from_slice(&40u32.to_le_bytes()); // biSize
        bmp.extend_from_slice(&32i32.to_le_bytes()); // biWidth
        bmp.extend_from_slice(&(64i32).to_le_bytes()); // biHeight (doubled for XOR+AND)
        bmp.extend_from_slice(&1u16.to_le_bytes()); // biPlanes
        bmp.extend_from_slice(&32u16.to_le_bytes()); // biBitCount
        bmp.extend_from_slice(&0u32.to_le_bytes()); // biCompression
        bmp.extend_from_slice(&(32 * 32 * 4u32).to_le_bytes()); // biSizeImage
        bmp.extend_from_slice(&0i32.to_le_bytes());
        bmp.extend_from_slice(&0i32.to_le_bytes());
        bmp.extend_from_slice(&0u32.to_le_bytes());
        bmp.extend_from_slice(&0u32.to_le_bytes());
        for _ in 0..(32 * 32) {
            bmp.extend_from_slice(&[245, 120, 24, 255]); // BGRA
        }
        bmp.resize(bmp.len() + 32 * 4, 0u8); // AND mask

        // ICONDIRENTRY (16 bytes)
        ico.push(32); // bWidth
        ico.push(32); // bHeight
        ico.push(0); // bColorCount
        ico.push(0); // bReserved
        ico.extend_from_slice(&1u16.to_le_bytes()); // wPlanes
        ico.extend_from_slice(&32u16.to_le_bytes()); // wBitCount
        ico.extend_from_slice(&(bmp.len() as u32).to_le_bytes()); // dwBytesInRes
        ico.extend_from_slice(&22u32.to_le_bytes()); // dwImageOffset
        ico.extend_from_slice(&bmp);
        ico
    }

    #[tokio::test]
    async fn test_studio_p3_vision_roi_isolation_proxy_and_builder_ipc() {
        let tmp = std::env::temp_dir().join(format!("fp_tauri_p3_ipc_{}", Uuid::new_v4()));
        let projects_root = tmp.join("projects");
        let storage = Arc::new(StorageManager::new(tmp.join("studio_p3.db")).unwrap());
        let coord = RuntimeCoordinator::new(Arc::clone(&storage), PathBuf::from("adb"));
        let _state = AppState {
            storage: Arc::clone(&storage),
            adb_runner: AdbRunner::default(),
            coordinator: coord.clone(),
            vault: CredentialVault::new(),
            db_path: ":memory:".to_string(),
            projects_root: projects_root.clone(),
            is_portable: false,
        };

        // 1. Create two isolated generic GameProjects
        let proj_a = storage
            .create_project(
                CreateProjectRequest {
                    slug: "sci-fi-tactics".to_string(),
                    name: "Sci-Fi Tactics".to_string(),
                    game_package_id: "com.studio.scifi".to_string(),
                    version: Some("1.2.0".to_string()),
                    exe_name: Some("Auto_SciFi_Tactics.exe".to_string()),
                    window_title: Some("Sci-Fi Tactics Auto".to_string()),
                    icon_rel_path: None,
                    target_width_px: Some(1080),
                    target_height_px: Some(1920),
                    bundle_adb: Some(true),
                    enabled_modules: Some(vec![
                        "mod.adb_core".to_string(),
                        "mod.vision_ocr".to_string(),
                    ]),
                },
                &projects_root,
            )
            .unwrap();

        let proj_b = storage
            .create_project(
                CreateProjectRequest {
                    slug: "casual-farm".to_string(),
                    name: "Casual Farm".to_string(),
                    game_package_id: "com.studio.farm".to_string(),
                    version: Some("1.0.0".to_string()),
                    exe_name: None,
                    window_title: None,
                    icon_rel_path: None,
                    target_width_px: Some(1080),
                    target_height_px: Some(1920),
                    bundle_adb: Some(true),
                    enabled_modules: Some(vec![
                        "mod.adb_core".to_string(),
                        "mod.vision_ocr".to_string(),
                        "mod.proxy_network".to_string(),
                    ]),
                },
                &projects_root,
            )
            .unwrap();

        // 2. Capture synthetic frame -> Crop ROI -> Save template in Project A -> Match & Annotate PNG
        let frame_png = FakeDeviceDriver::generate_synthetic_screen_png(
            1080,
            1920,
            Some(Bounds::new(200.0, 400.0, 120.0, 60.0)),
        );
        let tpl_rec = handle_crop_and_save_project_template(
            &storage,
            &proj_a.id,
            "Deploy Squad",
            "templates/deploy_btn.png",
            &frame_png,
            RoiRect {
                x: 200,
                y: 400,
                width: 120,
                height: 60,
            },
            Some(0.9),
        )
        .unwrap();
        assert_eq!(tpl_rec.width_px, 120);
        assert_eq!(tpl_rec.height_px, 60);

        let match_res = handle_test_project_template_match(
            &storage,
            &proj_a.id,
            &tpl_rec.id,
            &frame_png,
            1080,
            1920,
            Some(0.85),
            Some("evidence/annotated_match.png"),
        )
        .unwrap();
        assert!(match_res.matched);
        assert!(match_res.saved_annotated_path.is_some());
        assert!(PathBuf::from(match_res.saved_annotated_path.unwrap()).exists());

        // Cross-project template match blocked! Project B cannot use Project A's template ID!
        let cross_err = handle_test_project_template_match(
            &storage,
            &proj_b.id,
            &tpl_rec.id,
            &frame_png,
            1080,
            1920,
            None,
            None,
        )
        .unwrap_err();
        assert!(cross_err.contains("PROJECT_ISOLATION_VIOLATION"));

        // 3. Proxy probe IPC enforces `mod.proxy_network` per project & performs real SOCKS5 probe
        let disabled_proxy = handle_probe_project_proxy(
            &storage,
            &proj_a.id,
            &ProxyConfig {
                host: "127.0.0.1".to_string(),
                port: 1,
                protocol: ProxyProtocol::TcpOnly,
                username: None,
                password: None,
                timeout_ms: 400,
                target_probe_host: None,
                target_probe_port: None,
            },
        )
        .await;
        assert!(disabled_proxy.unwrap_err().contains("MODULE_DISABLED"));

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let srv = tokio::spawn(async move {
            if let Ok((mut sock, _)) = listener.accept().await {
                let mut greeting = [0u8; 3];
                let _ = sock.read_exact(&mut greeting).await;
                let _ = sock.write_all(&[0x05, 0x00]).await;
                let mut req = [0u8; 10];
                let _ = sock.read_exact(&mut req).await;
                let _ = sock
                    .write_all(&[0x05, 0x00, 0x00, 0x01, 127, 0, 0, 1, 0, 80])
                    .await;
                let _ = sock.shutdown().await;
            }
        });

        let ok_proxy = handle_probe_project_proxy(
            &storage,
            &proj_b.id,
            &ProxyConfig {
                host: "127.0.0.1".to_string(),
                port,
                protocol: ProxyProtocol::Socks5,
                username: None,
                password: None,
                timeout_ms: 1500,
                target_probe_host: Some("127.0.0.1".to_string()),
                target_probe_port: Some(80),
            },
        )
        .await
        .unwrap();
        let _ = srv.await;
        assert!(ok_proxy.handshake_ok);
        assert_eq!(ok_proxy.target_connect_ok, Some(true));

        // 4. Generic Mini Tool Builder preflight & full export recording into `project_releases`
        let wf_json = serde_json::json!({
            "schemaVersion": 1,
            "workflowId": "wf-scifi-deploy",
            "name": "Deploy Squad",
            "status": "published",
            "graph": {
                "nodes": [
                    { "id": "n-start", "type": "flow.start", "position": { "x": 0.0, "y": 0.0 } },
                    { "id": "n-cap", "type": "vision.capture", "position": { "x": 100.0, "y": 0.0 } },
                    { "id": "n-find", "type": "vision.find_image", "position": { "x": 200.0, "y": 0.0 }, "config": { "templateRef": "templates/deploy_btn.png", "minConfidence": 0.85 } },
                    { "id": "n-tap", "type": "action.random_safe_tap", "position": { "x": 300.0, "y": 0.0 } },
                    { "id": "n-end", "type": "flow.end", "position": { "x": 400.0, "y": 0.0 } }
                ],
                "edges": [
                    { "id": "e1", "source": "n-start", "sourcePort": "flow", "target": "n-cap", "targetPort": "flow" },
                    { "id": "e2", "source": "n-cap", "sourcePort": "flow", "target": "n-find", "targetPort": "flow" },
                    { "id": "e3", "source": "n-find", "sourcePort": "flow", "target": "n-tap", "targetPort": "flow" },
                    { "id": "e4", "source": "n-tap", "sourcePort": "flow", "target": "n-end", "targetPort": "flow" }
                ]
            }
        })
        .to_string();
        storage
            .save_project_workflow(&proj_a.id, "wf-scifi-deploy", "Deploy Squad", "Daily", &wf_json)
            .unwrap();

        let runtime_exe = std::env::current_exe().unwrap();
        let bin_dir = tmp.join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        std::fs::write(bin_dir.join("adb.exe"), b"MZ-adb-stub").unwrap();
        let ks_dir = tmp.join("keystore");

        // Before adding branding/icon.ico, preflight reports ready=false
        let pre_bad = handle_builder_preflight(
            &storage,
            &proj_a.id,
            &runtime_exe,
            Some(&bin_dir),
            Some(&ks_dir),
        )
        .unwrap();
        assert!(!pre_bad.ready);

        // Add valid ICO at branding/icon.ico -> preflight passes & export creates signed release + DB row!
        let ico_path = PathBuf::from(&proj_a.project_dir_path).join("branding/icon.ico");
        std::fs::create_dir_all(ico_path.parent().unwrap()).unwrap();
        std::fs::write(&ico_path, build_test_ico_32x32()).unwrap();

        let pre_ok = handle_builder_preflight(
            &storage,
            &proj_a.id,
            &runtime_exe,
            Some(&bin_dir),
            Some(&ks_dir),
        )
        .unwrap();
        assert!(pre_ok.ready, "Preflight issues: {:?}", pre_ok.issues);

        let release_out = tmp.join("release_out");
        let export_res = handle_builder_export_release(
            &storage,
            &proj_a.id,
            &runtime_exe,
            Some(&bin_dir),
            &release_out,
            Some(&ks_dir),
        )
        .unwrap();
        assert_eq!(export_res.release.project_id, proj_a.id);
        assert!(PathBuf::from(&export_res.release.exe_path).exists());

        let db_releases = storage.list_project_releases(&proj_a.id).unwrap();
        assert_eq!(db_releases.len(), 1);
        assert_eq!(db_releases[0].exe_sha256, export_res.release.exe_sha256);

        // 5. Run workflow through coordinator + verify job list & job resume detail IPC handlers
        let drv = Arc::new(FakeDeviceDriver::new("emulator-5554", 1080, 1920));
        *drv.current_screen_state.lock().await = "quest_marker_visible".to_string();
        let summary = coord
            .run_project_workflow(
                RunProjectWorkflowRequest {
                    job_id: Some("job-studio-p3-1".to_string()),
                    project_id: proj_a.id.clone(),
                    workflow_id: "wf-scifi-deploy".to_string(),
                    task_id: None,
                    device_id: "emulator-5554".to_string(),
                    account_id: None,
                    priority: 5,
                    lease_ttl_secs: 10,
                    workflow_timeout_ms: Some(5000),
                    initial_variables: std::collections::HashMap::new(),
                },
                Some(drv),
            )
            .await
            .unwrap();
        assert_eq!(summary.project_id, proj_a.id);
        assert_eq!(summary.state, "succeeded");

        let jobs = handle_list_project_jobs(&storage, &proj_a.id).unwrap();
        assert_eq!(jobs.len(), 1);
        let detail = handle_get_job_resume_detail(&storage, "job-studio-p3-1")
            .unwrap()
            .unwrap();
        assert_eq!(detail.resume_context.job.id, "job-studio-p3-1");
        assert!(!detail.step_events.is_empty());

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
