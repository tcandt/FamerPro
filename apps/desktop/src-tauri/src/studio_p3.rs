use crate::AppState;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use fp_actions::{SafeRegionSampler, SafeTapPolicy};
use fp_adb::{validate_adb_token, AdbDeviceDriver};
use fp_builder::{
    authenticode::{
        create_self_signed_qa_cert, discover_signing_toolchain, find_signtool_exe,
        QaSelfSignedCertInfo, SigningToolchainStatus,
    },
    security::{ProducerKeyManager, ProducerKeyMetadata},
    verify_final_distributed_release, BuildPreflightReport, BuildReport, MiniToolPackager,
    ReleaseBuildOptions,
};
use fp_security::ProducerKeystore;
use fp_bundle_contract::{
    evaluate_bundle_compatibility, AuthenticodeTrustStatus, CompatibilityReport,
    CompatibilityVersions, FinalReleaseVerificationReport, KeyRotationDelegation, PeVersionInfoSpec,
    PublisherIdentitySpec, RuntimeCompatibilityProfile, SemVer, SigningMode,
    TimestampVerificationPolicy, CURRENT_RUNTIME_VERSION, CURRENT_SQLITE_SCHEMA_VERSION,
    SUPPORTED_MODULE_API_VERSION, SUPPORTED_UI_SCHEMA_VERSION, SUPPORTED_WORKFLOW_SCHEMA_VERSION,
};
use fp_bundle_verify::{sha256_hex, validate_safe_relative_path};
use fp_domain::{
    Bounds, DeviceDriver, DeviceHealthReport, DevicePortMapping, Frame, Point, ScreenGeometry,
    StepEvent,
};
use fp_proxy::{probe_proxy, ProxyConfig, ProxyProbeReport};
use fp_scheduler::DeviceLease;
use fp_storage::{
    JobResumeContext, ProjectJobRecord, ProjectReleaseRecord, PublisherProfileInput,
    PublisherProfileRecord, ReleaseTransitionRecord, RuntimeVariablesSnapshot, StorageManager,
    VisionTemplateInput, VisionTemplateRecord,
};
use fp_updater::{
    ActiveVersionPointer, MiniToolUpdater, ReconcileOutcome, UpdateExecutionOptions, UpdateJournal,
    UpdateOutcome,
};
use fp_workflow_schema::WorkflowDocument;
use fp_vision::{
    annotate_match_and_tap_png, crop_roi_png, scale_bounds_for_geometry, RoiRect, VisionEngine,
};
use fp_workflow_runtime::coordinator::{RunProjectWorkflowRequest, RunWorkflowExecutionSummary};
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BuilderExportOptions {
    pub signing_mode: Option<SigningMode>,
    pub publisher_profile_id: Option<String>,
    pub authenticode_cert_ref: Option<String>,
    pub timestamp_url: Option<String>,
    pub release_channel: Option<String>,
    pub version_info: Option<PeVersionInfoSpec>,
    pub key_rotation_delegation: Option<KeyRotationDelegation>,
    pub include_pending_delegation: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SigningPreflightReport {
    pub ready: bool,
    pub signtool_available: bool,
    pub signtool_path: Option<String>,
    pub signing_mode: SigningMode,
    pub publisher_profile_present: bool,
    pub cert_ref_present: bool,
    pub keystore_key_id: Option<String>,
    pub issues: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyRotationWithDelegationResult {
    pub new_meta: ProducerKeyMetadata,
    pub delegation: KeyRotationDelegation,
    pub delegation_saved_path: String,
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

fn authenticode_status_str(s: AuthenticodeTrustStatus) -> &'static str {
    match s {
        AuthenticodeTrustStatus::Unsigned => "unsigned",
        AuthenticodeTrustStatus::SelfSignedUntrustedRoot => "self_signed_untrusted_root",
        AuthenticodeTrustStatus::TrustedPublicChain => "trusted_public_chain",
        AuthenticodeTrustStatus::TamperedOrInvalid => "tampered_or_invalid",
        AuthenticodeTrustStatus::ExpiredOrTimestampError => "expired_or_timestamp_error",
    }
}

pub(crate) fn handle_builder_export_release(
    storage: &StorageManager,
    project_id: &str,
    runtime_exe: &Path,
    workspace_bin_dir: Option<&Path>,
    output_release_dir: &Path,
    keystore_dir: Option<&Path>,
) -> Result<BuilderReleaseExportResult, String> {
    handle_builder_export_release_with_options(
        storage,
        project_id,
        runtime_exe,
        workspace_bin_dir,
        output_release_dir,
        keystore_dir,
        None,
    )
}

pub(crate) fn handle_builder_export_release_with_options(
    storage: &StorageManager,
    project_id: &str,
    runtime_exe: &Path,
    workspace_bin_dir: Option<&Path>,
    output_release_dir: &Path,
    keystore_dir: Option<&Path>,
    options: Option<BuilderExportOptions>,
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

    let opts = options.unwrap_or_default();
    let effective_profile_id = opts
        .publisher_profile_id
        .clone()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| proj.publisher_profile_id.clone());

    let profile_rec = if let Some(ref pid) = effective_profile_id {
        storage.get_publisher_profile(pid).map_err(|e| e.to_string())?
    } else {
        None
    };

    let signing_mode = opts
        .signing_mode
        .or_else(|| SigningMode::from_str_checked(&proj.signing_mode).ok())
        .unwrap_or(SigningMode::UnsignedInternal);

    let effective_cert_ref = opts
        .authenticode_cert_ref
        .clone()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            profile_rec
                .as_ref()
                .and_then(|p| p.authenticode_cert_ref.clone())
        });

    let effective_channel = opts
        .release_channel
        .clone()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| profile_rec.as_ref().map(|p| p.release_channel.clone()))
        .unwrap_or_else(|| match signing_mode {
            SigningMode::UnsignedInternal => "internal".to_string(),
            SigningMode::SelfSignedQa => "qa".to_string(),
            SigningMode::PublicSigned => "production".to_string(),
        });

    let publisher_spec: Option<PublisherIdentitySpec> =
        profile_rec.as_ref().map(|p| PublisherIdentitySpec {
            publisher_id: p.publisher_id.clone(),
            display_name: p.display_name.clone(),
            organization: p.organization.clone(),
            website: p.website.clone(),
            support_contact: p.support_contact.clone(),
            copyright: p.copyright.clone(),
            ed25519_key_id: p.ed25519_key_id.clone(),
            ed25519_pubkey_hex: p.ed25519_pubkey_hex.clone(),
            authenticode_cert_ref: effective_cert_ref.clone(),
            release_channel: effective_channel.clone(),
            signing_policy: signing_mode,
        });

    let version_info: Option<PeVersionInfoSpec> = opts
        .version_info
        .clone()
        .or_else(|| {
            proj.version_info_json
                .as_deref()
                .and_then(|raw| serde_json::from_str::<PeVersionInfoSpec>(raw).ok())
        })
        .or_else(|| {
            let org = profile_rec
                .as_ref()
                .map(|p| p.organization.clone())
                .unwrap_or_else(|| "FarmerPro Studio".to_string());
            let copy = profile_rec
                .as_ref()
                .map(|p| p.copyright.clone())
                .unwrap_or_else(|| format!("Copyright (C) 2026 {}", org));
            let q = PeVersionInfoSpec::parse_quad_version(&proj.version);
            Some(PeVersionInfoSpec {
                company_name: org,
                file_description: format!("{} Automation Runner", proj.name),
                file_version: format!("{}.{}.{}.{}", q[0], q[1], q[2], q[3]),
                internal_name: proj.slug.clone(),
                legal_copyright: copy,
                original_filename: proj.exe_name.clone(),
                product_name: proj.name.clone(),
                product_version: proj.version.clone(),
            })
        });

    let key_rotation_delegation = opts.key_rotation_delegation.clone().or_else(|| {
        if opts.include_pending_delegation.unwrap_or(false) {
            let del_path = Path::new(&proj.project_dir_path).join("key-rotation-delegation.json");
            std::fs::read(&del_path)
                .ok()
                .and_then(|b| serde_json::from_slice::<KeyRotationDelegation>(&b).ok())
        } else {
            None
        }
    });

    let release_opts = ReleaseBuildOptions {
        signing_mode,
        authenticode_cert_ref: effective_cert_ref,
        timestamp_url: opts.timestamp_url.clone(),
        publisher_profile: publisher_spec,
        version_info,
        key_rotation_delegation,
    };

    let report = MiniToolPackager::build_and_export_release_with_options(
        Path::new(&proj.project_dir_path),
        runtime_exe,
        workspace_bin_dir,
        output_release_dir,
        keystore_dir,
        &release_opts,
    )
    .map_err(|e| e.to_string())?;

    let git_commit = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let (auth_status_str, auth_thumb, auth_subj) = match report.authenticode_verification.as_ref() {
        Some(v) => (
            authenticode_status_str(v.trust_status).to_string(),
            v.signer_thumbprint.clone(),
            v.signer_subject.clone(),
        ),
        None => ("unsigned".to_string(), None, None),
    };

    let compat = CompatibilityVersions::default();
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
        lifecycle_state: "Draft".to_string(),
        release_channel: effective_channel,
        changelog_md: String::new(),
        min_runtime_version: compat.min_runtime_version,
        workflow_schema_version: compat.workflow_schema_version,
        ui_schema_version: compat.ui_schema_version,
        module_api_version: compat.module_api_version,
        sqlite_schema_version: compat.sqlite_schema_version,
        git_commit,
        signature_status: report.authenticode_status.clone(),
        promoted_at: None,
        rolled_back_from_id: None,
        publisher_profile_id: profile_rec.map(|p| p.id),
        signing_mode: signing_mode.as_str().to_string(),
        authenticode_trust_status: auth_status_str,
        authenticode_thumbprint: auth_thumb,
        authenticode_subject: auth_subj,
        trusted_public_release: report.trusted_public_release,
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
    let mut runtime_candidates: Vec<PathBuf> = Vec::new();
    if let Ok(cur_exe) = std::env::current_exe() {
        if let Some(dir) = cur_exe.parent() {
            runtime_candidates.push(dir.join("fp-mini-runtime.exe"));
            runtime_candidates.push(dir.join("resources").join("fp-mini-runtime.exe"));
        }
    }
    runtime_candidates.push(PathBuf::from("target/release/fp-mini-runtime.exe"));
    runtime_candidates.push(PathBuf::from("target/debug/fp-mini-runtime.exe"));
    runtime_candidates.push(PathBuf::from("dist-portable/FarmerPro/fp-mini-runtime.exe"));

    let runtime_exe = runtime_candidates
        .into_iter()
        .find(|p| p.exists())
        .unwrap_or_else(|| PathBuf::from("target/debug/fp-mini-runtime.exe"));

    let mut bin_candidates: Vec<PathBuf> = vec![PathBuf::from("bin")];
    if let Ok(cur_exe) = std::env::current_exe() {
        if let Some(dir) = cur_exe.parent() {
            bin_candidates.push(dir.join("bin"));
            bin_candidates.push(dir.join("resources").join("bin"));
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
    let (coord, storage) = {
        let st = state.lock().unwrap();
        (st.coordinator.clone(), Arc::clone(&st.storage))
    };
    let summary = coord
        .run_project_workflow(req, None)
        .await
        .map_err(|e| e.to_string())?;
    let mut vars = summary.variables.clone();
    let now_utc = chrono::Utc::now().to_rfc3339();
    let is_synth = summary.adapter_kind == fp_domain::DeviceAdapterKind::Synthetic;
    let src = if is_synth {
        fp_domain::VariableSource::TestkitMock
    } else {
        fp_domain::VariableSource::LiveState
    };
    let mut envelopes: Vec<fp_domain::RuntimeVariableEnvelope> = summary
        .variables
        .iter()
        .map(|(k, v)| fp_domain::RuntimeVariableEnvelope {
            project_id: summary.project_id.clone(),
            session_id: format!("studio-{}", summary.job_id),
            job_id: summary.job_id.clone(),
            name: k.clone(),
            value_type: fp_domain::infer_json_value_type(v),
            value: v.clone(),
            source: src,
            verified: !is_synth && !v.is_null(),
            confidence: None,
            updated_at_utc: now_utc.clone(),
            workflow_version: format!("{}@latest", summary.workflow_id),
            seq: 0,
            redacted: false,
        })
        .collect();
    for (k, v) in [
        ("last_task_state", serde_json::Value::String(summary.state.clone())),
        ("last_data_source", serde_json::Value::String(format!("{:?}", summary.adapter_kind))),
        ("last_updated_utc", serde_json::Value::String(now_utc.clone())),
    ] {
        vars.insert(k.to_string(), v.clone());
        envelopes.push(fp_domain::RuntimeVariableEnvelope {
            project_id: summary.project_id.clone(),
            session_id: format!("studio-{}", summary.job_id),
            job_id: summary.job_id.clone(),
            name: k.to_string(),
            value_type: fp_domain::infer_json_value_type(&v),
            value: v,
            source: src,
            verified: !is_synth,
            confidence: None,
            updated_at_utc: now_utc.clone(),
            workflow_version: format!("{}@latest", summary.workflow_id),
            seq: 0,
            redacted: false,
        });
    }
    let _ = storage.record_runtime_var_deltas(
        &summary.project_id,
        &summary.project_id,
        &format!("studio-{}", summary.job_id),
        &summary.job_id,
        &format!("{}@latest", summary.workflow_id),
        envelopes,
    );
    Ok(RunWorkflowExecutionSummary {
        variables: vars,
        ..summary
    })
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
    let (coord, storage) = {
        let st = state.lock().unwrap();
        (st.coordinator.clone(), Arc::clone(&st.storage))
    };
    let summary = coord
        .resume_from_checkpoint(&job_id, lease_ttl_secs.unwrap_or(30), workflow_timeout_ms, None)
        .await
        .map_err(|e| e.to_string())?;
    let _ = storage.save_runtime_variables(&summary.project_id, &summary.variables);
    Ok(summary)
}

#[tauri::command]
pub fn studio_get_project_runtime_vars(
    project_id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<RuntimeVariablesSnapshot, String> {
    let st = state.lock().unwrap();
    st.storage
        .load_runtime_variables(&project_id)
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
    options: Option<BuilderExportOptions>,
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

    handle_builder_export_release_with_options(
        &st.storage,
        &project_id,
        &runtime_exe,
        bin_dir.as_deref(),
        &release_dir,
        None,
        options,
    )
}

#[tauri::command]
pub fn studio_list_publisher_profiles(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<PublisherProfileRecord>, String> {
    let st = state.lock().unwrap();
    st.storage
        .list_publisher_profiles()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_upsert_publisher_profile(
    input: PublisherProfileInput,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<PublisherProfileRecord, String> {
    let st = state.lock().unwrap();
    st.storage
        .upsert_publisher_profile(input)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_delete_publisher_profile(
    id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<bool, String> {
    let st = state.lock().unwrap();
    st.storage
        .delete_publisher_profile(&id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_create_self_signed_qa_cert(
    subject_cn: Option<String>,
    _valid_days: Option<u32>,
) -> Result<QaSelfSignedCertInfo, String> {
    let raw_cn = subject_cn
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "FarmerPro QA Publisher".to_string());
    let cn = raw_cn
        .trim()
        .strip_prefix("CN=")
        .or_else(|| raw_cn.trim().strip_prefix("cn="))
        .unwrap_or(raw_cn.trim())
        .to_string();
    create_self_signed_qa_cert(&cn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_signing_preflight(
    project_id: String,
    signing_mode: Option<String>,
    cert_ref: Option<String>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<SigningPreflightReport, String> {
    let st = state.lock().unwrap();
    let proj = st
        .storage
        .get_project(&project_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("GameProject '{}' not found", project_id))?;

    let mode = signing_mode
        .as_deref()
        .or(Some(proj.signing_mode.as_str()))
        .and_then(|s| SigningMode::from_str_checked(s).ok())
        .unwrap_or(SigningMode::UnsignedInternal);

    let profile = if let Some(ref pid) = proj.publisher_profile_id {
        st.storage
            .get_publisher_profile(pid)
            .map_err(|e| e.to_string())?
    } else {
        None
    };

    let effective_cert = cert_ref
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            profile
                .as_ref()
                .and_then(|p| p.authenticode_cert_ref.clone())
        });

    let toolchain = discover_signing_toolchain();
    let (signtool_available, signtool_path) = (
        toolchain.signtool_found,
        toolchain
            .discovered_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string()),
    );

    let ks_dir = ProducerKeystore::default_external_keystore_dir();
    let keystore_key_id = ProducerKeystore::load_or_create_signing_key(&ks_dir)
        .ok()
        .map(|(_, m)| m.key_id);

    let mut issues = Vec::new();
    let mut warnings = Vec::new();
    match mode {
        SigningMode::UnsignedInternal => {
            warnings.push(
                "Unsigned Internal build: PE VERSIONINFO & Ed25519 bundle signature applied; works with ZERO Windows SDK installed."
                    .to_string(),
            );
        }
        SigningMode::SelfSignedQa => {
            if !signtool_available {
                issues.push(toolchain.guidance.clone());
            }
            if effective_cert.is_none() {
                warnings.push(
                    "No Authenticode thumbprint/cert selected; Builder can create a local Windows CNG QA cert right here."
                        .to_string(),
                );
            } else {
                warnings.push(
                    "Self-Signed QA builds verify cryptographic integrity & Authenticode signature but DO NOT claim TrustedPublicRelease."
                        .to_string(),
                );
            }
        }
        SigningMode::PublicSigned => {
            if !signtool_available {
                issues.push(toolchain.guidance.clone());
            }
            if effective_cert.is_none() {
                issues.push(
                    "PublicSigned mode requires a commercial Code Signing certificate reference (thumbprint or PFX) — never falls back to Self-Signed QA."
                        .to_string(),
                );
            }
        }
    }

    Ok(SigningPreflightReport {
        ready: issues.is_empty(),
        signtool_available,
        signtool_path,
        signing_mode: mode,
        publisher_profile_present: profile.is_some(),
        cert_ref_present: effective_cert.is_some(),
        keystore_key_id,
        issues,
        warnings,
    })
}

#[tauri::command]
pub fn studio_discover_signing_toolchain() -> SigningToolchainStatus {
    discover_signing_toolchain()
}

#[tauri::command]
pub fn studio_verify_final_release(
    release_dir: String,
    require_public_chain: Option<bool>,
) -> Result<FinalReleaseVerificationReport, String> {
    verify_final_distributed_release(
        Path::new(&release_dir),
        require_public_chain.unwrap_or(false),
        TimestampVerificationPolicy::AllowOfflineUnsignedTimestamp,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_rotate_project_ed25519_key_with_delegation(
    project_id: String,
    publisher_id: Option<String>,
    valid_days: Option<i64>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<KeyRotationWithDelegationResult, String> {
    let st = state.lock().unwrap();
    let proj = st
        .storage
        .get_project(&project_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("GameProject '{}' not found", project_id))?;

    let eff_pub_id = publisher_id
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            proj.publisher_profile_id
                .as_deref()
                .and_then(|pid| st.storage.get_publisher_profile(pid).ok().flatten())
                .map(|p| p.publisher_id)
        })
        .unwrap_or_else(|| format!("pub-{}", proj.slug));

    let ks_dir = ProducerKeystore::default_external_keystore_dir();
    let (old_key, _old_meta) =
        ProducerKeystore::load_or_create_signing_key(&ks_dir).map_err(|e| e.to_string())?;
    let (_new_key, new_meta) = ProducerKeystore::rotate_producer_key(
        &ks_dir,
        &format!("p5_3_delegated_rotation:{}", proj.slug),
    )
    .map_err(|e| e.to_string())?;

    let delegation = ProducerKeyManager::sign_key_rotation_delegation(
        &old_key,
        &new_meta.public_key_hex,
        &eff_pub_id,
        &proj.slug,
        valid_days.unwrap_or(90),
    );

    let del_path = PathBuf::from(&proj.project_dir_path).join("key-rotation-delegation.json");
    let bytes = serde_json::to_vec_pretty(&delegation).map_err(|e| e.to_string())?;
    std::fs::write(&del_path, bytes).map_err(|e| e.to_string())?;

    Ok(KeyRotationWithDelegationResult {
        new_meta,
        delegation,
        delegation_saved_path: del_path.to_string_lossy().to_string(),
    })
}

fn parse_hex_bgra(accent_hex: Option<&str>) -> [u8; 4] {
    let raw = accent_hex.unwrap_or("#1878F5").trim().trim_start_matches('#');
    if raw.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&raw[0..2], 16),
            u8::from_str_radix(&raw[2..4], 16),
            u8::from_str_radix(&raw[4..6], 16),
        ) {
            return [b, g, r, 255];
        }
    }
    [245, 120, 24, 255]
}

fn build_branded_ico_32x32(accent_hex: Option<&str>) -> Vec<u8> {
    let accent = parse_hex_bgra(accent_hex);
    let border = [42, 30, 18, 255]; // dark #121E2A BGRA
    let inner = [235, 185, 95, 255];
    let mut ico = Vec::new();
    ico.extend_from_slice(&0u16.to_le_bytes());
    ico.extend_from_slice(&1u16.to_le_bytes());
    ico.extend_from_slice(&1u16.to_le_bytes());

    let mut bmp = Vec::new();
    bmp.extend_from_slice(&40u32.to_le_bytes());
    bmp.extend_from_slice(&32i32.to_le_bytes());
    bmp.extend_from_slice(&64i32.to_le_bytes());
    bmp.extend_from_slice(&1u16.to_le_bytes());
    bmp.extend_from_slice(&32u16.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(&(32 * 32 * 4u32).to_le_bytes());
    bmp.extend_from_slice(&0i32.to_le_bytes());
    bmp.extend_from_slice(&0i32.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    for y in 0..32 {
        for x in 0..32 {
            let px = if !(3..=28).contains(&x) || !(3..=28).contains(&y) {
                border
            } else if (11..=20).contains(&x) && (11..=20).contains(&y) {
                inner
            } else {
                accent
            };
            bmp.extend_from_slice(&px);
        }
    }
    bmp.resize(bmp.len() + 32 * 4, 0u8);

    ico.push(32);
    ico.push(32);
    ico.push(0);
    ico.push(0);
    ico.extend_from_slice(&1u16.to_le_bytes());
    ico.extend_from_slice(&32u16.to_le_bytes());
    ico.extend_from_slice(&(bmp.len() as u32).to_le_bytes());
    ico.extend_from_slice(&22u32.to_le_bytes());
    ico.extend_from_slice(&bmp);
    ico
}

#[tauri::command]
pub fn studio_ensure_project_icon(
    project_id: String,
    icon_base64: Option<String>,
    accent_hex: Option<String>,
    overwrite: Option<bool>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<String, String> {
    let st = state.lock().unwrap();
    let proj = st
        .storage
        .get_project(&project_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("GameProject '{}' not found", project_id))?;

    let proj_dir = PathBuf::from(&proj.project_dir_path);
    let icon_path =
        validate_safe_relative_path(&proj_dir, &proj.icon_rel_path).map_err(|e| e.to_string())?;
    if let Some(parent) = icon_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    if let Some(b64) = icon_base64.filter(|s| !s.trim().is_empty()) {
        let bytes = B64
            .decode(b64.trim())
            .map_err(|e| format!("Invalid base64 ICO payload: {}", e))?;
        if bytes.len() < 22 || bytes[0..4] != [0, 0, 1, 0] {
            return Err("Uploaded file is not a valid Windows .ico container".to_string());
        }
        std::fs::write(&icon_path, bytes).map_err(|e| e.to_string())?;
    } else if !icon_path.exists() || overwrite.unwrap_or(false) {
        let bytes = build_branded_ico_32x32(accent_hex.as_deref());
        std::fs::write(&icon_path, bytes).map_err(|e| e.to_string())?;
    }

    Ok(icon_path.to_string_lossy().to_string())
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedInstallInspection {
    pub install_root: String,
    pub active_pointer: Option<ActiveVersionPointer>,
    pub journal: Option<UpdateJournal>,
}

pub(crate) fn handle_check_release_compatibility(
    storage: &StorageManager,
    project_id: &str,
) -> Result<CompatibilityReport, String> {
    let detail = storage
        .get_project_detail(project_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Project '{}' not found", project_id))?;

    SemVer::parse(&detail.project.version).map_err(|e| e.to_string())?;

    let mut wf_versions = Vec::new();
    for wf in &detail.workflows {
        if let Ok(doc) = serde_json::from_str::<WorkflowDocument>(&wf.draft_json) {
            wf_versions.push(doc.schema_version);
        } else {
            wf_versions.push(SUPPORTED_WORKFLOW_SCHEMA_VERSION);
        }
    }
    let ui_schema_ver = detail
        .ui_schema
        .as_ref()
        .map(|s| s.schema_version)
        .unwrap_or(SUPPORTED_UI_SCHEMA_VERSION);

    let compat = CompatibilityVersions {
        project_schema_version: 1,
        workflow_schema_version: SUPPORTED_WORKFLOW_SCHEMA_VERSION,
        ui_schema_version: ui_schema_ver,
        module_api_version: SUPPORTED_MODULE_API_VERSION.to_string(),
        min_runtime_version: CURRENT_RUNTIME_VERSION.to_string(),
        sqlite_schema_version: CURRENT_SQLITE_SCHEMA_VERSION,
    };

    Ok(evaluate_bundle_compatibility(
        &compat,
        ui_schema_ver,
        &wf_versions,
        &RuntimeCompatibilityProfile::default(),
    ))
}

#[tauri::command]
pub fn studio_check_release_compatibility(
    project_id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<CompatibilityReport, String> {
    let st = state.lock().unwrap();
    handle_check_release_compatibility(&st.storage, &project_id)
}

#[tauri::command]
pub fn studio_transition_release_lifecycle(
    project_id: String,
    release_id: String,
    target_state: String,
    note: Option<String>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<ProjectReleaseRecord, String> {
    let st = state.lock().unwrap();
    st.storage
        .transition_project_release_lifecycle(
            &project_id,
            &release_id,
            &target_state,
            note.as_deref().unwrap_or("Studio lifecycle update"),
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_list_project_release_transitions(
    project_id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<ReleaseTransitionRecord>, String> {
    let st = state.lock().unwrap();
    st.storage
        .list_project_release_transitions(&project_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_deploy_minitool_release(
    install_root: String,
    release_dir: String,
) -> Result<UpdateOutcome, String> {
    let updater = MiniToolUpdater::new(&install_root);
    let opts = UpdateExecutionOptions::default();
    if updater.active_pointer_path().exists() {
        updater
            .upgrade_from_release(Path::new(&release_dir), &opts)
            .map_err(|e| e.to_string())
    } else {
        updater
            .install_initial(Path::new(&release_dir), &opts)
            .map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn studio_rollback_minitool_install(
    install_root: String,
    restore_backup: Option<bool>,
) -> Result<UpdateOutcome, String> {
    let updater = MiniToolUpdater::new(&install_root);
    updater
        .rollback_to_previous(restore_backup.unwrap_or(false), false)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_reconcile_minitool_install(
    install_root: String,
) -> Result<ReconcileOutcome, String> {
    let updater = MiniToolUpdater::new(&install_root);
    updater
        .reconcile_incomplete_journal(false)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_inspect_minitool_install(
    install_root: String,
) -> Result<ManagedInstallInspection, String> {
    let updater = MiniToolUpdater::new(&install_root);
    let active_pointer = updater.read_active_pointer().map_err(|e| e.to_string())?;
    let journal = updater.read_journal().map_err(|e| e.to_string())?;
    Ok(ManagedInstallInspection {
        install_root,
        active_pointer,
        journal,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use fp_adb::AdbRunner;
    use fp_proxy::ProxyProtocol;
    use fp_security::CredentialVault;
    use fp_storage::CreateProjectRequest;
    use fp_testkit::FakeDeviceDriver;
    use fp_workflow_runtime::coordinator::RuntimeCoordinator;
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
