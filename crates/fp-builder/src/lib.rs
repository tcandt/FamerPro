pub mod orchestrator;
pub mod pe_icon;
pub mod security;

pub use fp_bundle_contract as contracts;
pub use fp_bundle_verify as verify;

use anyhow::{bail, Context, Result};
use fp_bundle_contract::*;
use fp_bundle_verify::{
    read_pe_embedded_pubkey_from_exe, sha256_hex, validate_safe_relative_path,
    PE_ED25519_PUBKEY_RESOURCE_ID,
};
use fp_workflow_schema::{WorkflowDocument, WorkflowValidator};
use pe_icon::{
    inject_ed25519_pubkey_into_pe_exe, inject_ico_into_pe_exe, verify_pe_embedded_pubkey_resource,
    verify_pe_icon_resource,
};
use security::ProducerKeyManager;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildStepMetric {
    pub step_no: u32,
    pub name: String,
    pub duration_ms: u64,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildReport {
    pub project_id: String,
    pub project_slug: String,
    pub tool_exe_name: String,
    pub tool_exe_path: String,
    pub tool_exe_size_bytes: u64,
    pub tool_exe_sha256: String,
    pub pe_icon_layers_injected: usize,
    pub pe_icon_group_resource_bytes: u32,
    pub pe_icon_first_layer_bytes: u32,
    pub pe_embedded_pubkey_resource_id: usize,
    pub pe_embedded_pubkey_verified: bool,
    pub authenticode_status: String,
    pub key_id: String,
    pub publisher_public_key_hex: String,
    pub external_keystore_path: String,
    pub signature_algorithm: String,
    pub signature_ed25519_hex: String,
    pub total_duration_ms: u64,
    pub steps: Vec<BuildStepMetric>,
    pub modules: Vec<ModuleStateReport>,
    pub dependency_audit: PortableDependencyAudit,
}

pub struct MiniToolPackager;

impl MiniToolPackager {
    /// Full 8-step Release Pipeline using RFC 8032 Ed25519 (`ed25519-dalek`) and External Producer Keystore:
    /// 1. Validate Project, UI Schema & Workflows
    /// 2. Verify Assets & Path Traversal Security
    /// 3. Resolve 5-State Module Matrix
    /// 4. Copy Dedicated `fp-mini-runtime.exe` Binary
    /// 5. Inject Custom PE Icon (.ico) + Pin Ed25519 Public Key in PE `RT_RCDATA #7777` & Verify BEFORE Signing
    /// 6. Package Assets (`bundle/ui-schema.json`, `bundle/workflows.fpbundle.json`, `bundle/templates/*`, `bin/adb*`)
    /// 7. Compute SHA-256 Checksums & Sign Envelope with External Producer Ed25519 Key
    /// 8. Verify Signed Bundle via `fp-bundle-verify` (using PE-Embedded Root of Trust Public Key) & Emit `build-report.json`
    pub fn build_and_export_release(
        project_dir: &Path,
        runtime_template_exe: &Path,
        workspace_bin_dir: Option<&Path>,
        output_release_dir: &Path,
        custom_keystore_dir: Option<&Path>,
    ) -> Result<BuildReport> {
        let total_start = Instant::now();
        let mut steps = Vec::new();

        // STEP 1: Load & Validate Project, UI Schema, and Workflows
        let t1 = Instant::now();
        let proj_bytes = std::fs::read(project_dir.join("project.json"))
            .context("Missing project.json in project directory")?;
        let project: GameProjectManifest = serde_json::from_slice(&proj_bytes)?;

        let ui_bytes = std::fs::read(project_dir.join("ui-schema.json"))
            .context("Missing ui-schema.json in project directory")?;
        let ui_schema: MiniToolUiSchema = serde_json::from_slice(&ui_bytes)?;

        if ui_schema.project_slug != project.slug {
            bail!(
                "UI schema projectSlug '{}' does not match project slug '{}'",
                ui_schema.project_slug,
                project.slug
            );
        }

        let mut validated_workflows: Vec<WorkflowDocument> = Vec::new();
        for wf_rel in &project.workflow_files {
            let wf_path = validate_safe_relative_path(project_dir, wf_rel)?;
            let wf_str = std::fs::read_to_string(&wf_path)
                .with_context(|| format!("Failed to read workflow file: {}", wf_path.display()))?;
            let doc = WorkflowValidator::parse_and_validate(&wf_str)?;
            validated_workflows.push(doc);
        }
        steps.push(BuildStepMetric {
            step_no: 1,
            name: "Validate Project, UI Schema & Workflows".to_string(),
            duration_ms: t1.elapsed().as_millis() as u64,
            status: "PASS".to_string(),
            detail: format!(
                "Validated {} workflows and UI schema (v{}) for '{}'",
                validated_workflows.len(),
                ui_schema.schema_version,
                project.name
            ),
        });

        // STEP 2: Verify Assets & Path Safety
        let t2 = Instant::now();
        for tpl_rel in &project.template_files {
            let tpl_path = validate_safe_relative_path(project_dir, tpl_rel)?;
            if !tpl_path.exists() {
                bail!("Missing declared template asset: {}", tpl_path.display());
            }
        }
        let icon_path = validate_safe_relative_path(project_dir, &project.icon_rel_path)?;
        if !icon_path.exists() {
            bail!("Missing declared project icon: {}", icon_path.display());
        }
        steps.push(BuildStepMetric {
            step_no: 2,
            name: "Asset Integrity & Path Traversal Guard".to_string(),
            duration_ms: t2.elapsed().as_millis() as u64,
            status: "PASS".to_string(),
            detail: format!(
                "Verified {} template PNGs and icon '{}'",
                project.template_files.len(),
                project.icon_rel_path
            ),
        });

        // STEP 3: Resolve 5-State Modules
        let t3 = Instant::now();
        let registry = ModuleRegistry::resolve_for_release(
            &project.packaged_modules,
            &project.packaged_modules,
        );
        let module_states = registry.reports().to_vec();
        steps.push(BuildStepMetric {
            step_no: 3,
            name: "Resolve 5-State Module Matrix".to_string(),
            duration_ms: t3.elapsed().as_millis() as u64,
            status: "PASS".to_string(),
            detail: format!("Packaged modules: {:?}", project.packaged_modules),
        });

        // STEP 4: Provision Mini Runtime Binary
        let t4 = Instant::now();
        if !runtime_template_exe.exists() {
            bail!(
                "Mini Runtime template binary not found at {}",
                runtime_template_exe.display()
            );
        }
        if output_release_dir.exists() {
            let _ = std::fs::remove_dir_all(output_release_dir);
        }
        std::fs::create_dir_all(output_release_dir)?;
        let target_exe_path = output_release_dir.join(&project.exe_name);
        std::fs::copy(runtime_template_exe, &target_exe_path)?;
        steps.push(BuildStepMetric {
            step_no: 4,
            name: "Copy Dedicated Mini Runtime Binary".to_string(),
            duration_ms: t4.elapsed().as_millis() as u64,
            status: "PASS".to_string(),
            detail: format!(
                "Copied {} -> {}",
                runtime_template_exe.display(),
                target_exe_path.display()
            ),
        });

        // Load External Producer Ed25519 SigningKey prior to Step 5 so its 32-byte Public Key
        // is pinned directly inside the PE executable's RT_RCDATA #7777 resource!
        let keystore_dir = custom_keystore_dir
            .map(|p| p.to_path_buf())
            .unwrap_or_else(ProducerKeyManager::default_external_keystore_dir);
        let (signing_key, key_meta) = ProducerKeyManager::load_or_create_signing_key(&keystore_dir)?;

        // STEP 5: Inject Custom PE Icon Resource AND Pin Ed25519 Public Key in PE RT_RCDATA #7777 BEFORE Signing
        let t5 = Instant::now();
        let layers_injected = inject_ico_into_pe_exe(&target_exe_path, &icon_path)?;
        inject_ed25519_pubkey_into_pe_exe(&target_exe_path, &key_meta.public_key_hex)?;
        // Verify BOTH PE Icon resources and PE RT_RCDATA #7777 Public Key after all PE resource updates complete
        let (grp_bytes, ico1_bytes) = verify_pe_icon_resource(&target_exe_path)?;
        let verified_pe_pubkey =
            verify_pe_embedded_pubkey_resource(&target_exe_path, &key_meta.public_key_hex)?;
        steps.push(BuildStepMetric {
            step_no: 5,
            name: "PE Icon + Ed25519 Root-of-Trust Public Key Injection (RT_RCDATA #7777)".to_string(),
            duration_ms: t5.elapsed().as_millis() as u64,
            status: "PASS".to_string(),
            detail: format!(
                "Injected {} icon layers (RT_GROUP_ICON={}B, RT_ICON#1={}B) and pinned Ed25519 PubKey ({}) in PE RT_RCDATA #{}",
                layers_injected,
                grp_bytes,
                ico1_bytes,
                &verified_pe_pubkey[..16],
                PE_ED25519_PUBKEY_RESOURCE_ID
            ),
        });

        // STEP 6: Package Bundle Assets & Optional ADB Dependencies
        let t6 = Instant::now();
        let bundle_dir = output_release_dir.join("bundle");
        let bundle_tpl_dir = bundle_dir.join("templates");
        std::fs::create_dir_all(&bundle_tpl_dir)?;

        let release_manifest = ReleaseBundleManifest {
            compatibility: project.compatibility.clone(),
            project: project.clone(),
            module_states: module_states.clone(),
            workflows_count: validated_workflows.len(),
            templates_count: project.template_files.len(),
            built_at_utc: chrono::Utc::now().to_rfc3339(),
        };

        let manifest_json = serde_json::to_vec_pretty(&release_manifest)?;
        std::fs::write(bundle_dir.join("manifest.fpbundle.json"), &manifest_json)?;
        std::fs::write(bundle_dir.join("ui-schema.json"), &ui_bytes)?;

        let workflows_json = serde_json::to_vec_pretty(&validated_workflows)?;
        std::fs::write(bundle_dir.join("workflows.fpbundle.json"), &workflows_json)?;

        for tpl_rel in &project.template_files {
            let src_tpl = validate_safe_relative_path(project_dir, tpl_rel)?;
            let file_name = src_tpl.file_name().context("Invalid template filename")?;
            std::fs::copy(&src_tpl, bundle_tpl_dir.join(file_name))?;
        }

        let bundle_brand_dir = bundle_dir.join("branding");
        std::fs::create_dir_all(&bundle_brand_dir)?;
        std::fs::copy(&icon_path, bundle_brand_dir.join("icon.ico"))?;
        let png_candidate = icon_path.with_file_name("128x128.png");
        if png_candidate.exists() {
            std::fs::copy(&png_candidate, bundle_brand_dir.join("icon.png"))?;
        }

        if project.bundle_adb {
            if let Some(bin_src) = workspace_bin_dir {
                if bin_src.exists() {
                    let bin_dst = output_release_dir.join("bin");
                    std::fs::create_dir_all(&bin_dst)?;
                    for f in ["adb.exe", "AdbWinApi.dll", "AdbWinUsbApi.dll"] {
                        let s = bin_src.join(f);
                        if s.exists() {
                            std::fs::copy(&s, bin_dst.join(f))?;
                        }
                    }
                }
            }
        }
        steps.push(BuildStepMetric {
            step_no: 6,
            name: "Package Bundle Assets & Android ADB Dependencies".to_string(),
            duration_ms: t6.elapsed().as_millis() as u64,
            status: "PASS".to_string(),
            detail: format!(
                "Packaged manifest, ui-schema, {} workflows, {} templates, adb={}",
                validated_workflows.len(),
                project.template_files.len(),
                project.bundle_adb
            ),
        });

        // STEP 7: Compute SHA-256 Checksums & Sign Bundle with External Producer Ed25519 Key
        let t7 = Instant::now();
        let mut file_checksums = BTreeMap::new();
        file_checksums.insert(
            "manifest.fpbundle.json".to_string(),
            sha256_hex(&manifest_json),
        );
        file_checksums.insert("ui-schema.json".to_string(), sha256_hex(&ui_bytes));
        file_checksums.insert(
            "workflows.fpbundle.json".to_string(),
            sha256_hex(&workflows_json),
        );

        for tpl_rel in &project.template_files {
            let file_name = Path::new(tpl_rel)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string();
            let rel_in_bundle = format!("templates/{}", file_name);
            let bytes = std::fs::read(bundle_dir.join(&rel_in_bundle))?;
            file_checksums.insert(rel_in_bundle, sha256_hex(&bytes));
        }

        let mut sha_lines = String::new();
        for (k, v) in &file_checksums {
            sha_lines.push_str(&format!("{}  {}\n", v, k));
        }
        std::fs::write(bundle_dir.join("checksums.sha256"), sha_lines.as_bytes())?;

        let manifest_sha = sha256_hex(&manifest_json);
        let envelope = ProducerKeyManager::sign_bundle_ed25519(
            &signing_key,
            &key_meta.key_id,
            manifest_sha,
            file_checksums,
        );
        let sig_json = serde_json::to_vec_pretty(&envelope)?;
        std::fs::write(bundle_dir.join("signature.fpbundle.json"), &sig_json)?;
        std::fs::write(
            bundle_dir.join("trusted_publisher.pub"),
            key_meta.public_key_hex.as_bytes(),
        )?;

        steps.push(BuildStepMetric {
            step_no: 7,
            name: "Compute SHA-256 Integrity Table & RFC 8032 Ed25519 Signature".to_string(),
            duration_ms: t7.elapsed().as_millis() as u64,
            status: "PASS".to_string(),
            detail: format!(
                "Signed {} files with {} (KeyID: {}, PubKey: {})",
                envelope.file_checksums.len(),
                envelope.algorithm,
                key_meta.key_id,
                key_meta.public_key_hex
            ),
        });

        // STEP 8: Verify Signed Bundle using `fp-bundle-verify` (Anchored to PE-Embedded Public Key!) & Audit Dependencies
        let t8 = Instant::now();
        let pe_root_pubkey = read_pe_embedded_pubkey_from_exe(&target_exe_path)?;
        envelope.verify_ed25519_with_public_key(&bundle_dir, &pe_root_pubkey)?;
        let dep_audit = audit_portable_environment(output_release_dir, &project.slug);
        let exe_bytes = std::fs::read(&target_exe_path)?;
        let exe_sha256 = sha256_hex(&exe_bytes);

        steps.push(BuildStepMetric {
            step_no: 8,
            name: "Verify RFC 8032 Ed25519 Signature via PE-Embedded Root of Trust & Audit Dependencies".to_string(),
            duration_ms: t8.elapsed().as_millis() as u64,
            status: "PASS".to_string(),
            detail: format!(
                "Ed25519 verified against PE RT_RCDATA #{} key; exe SHA-256={}; WebView2={}",
                PE_ED25519_PUBKEY_RESOURCE_ID,
                &exe_sha256[..16],
                dep_audit.webview2_evergreen_installed
            ),
        });

        let report = BuildReport {
            project_id: project.project_id.clone(),
            project_slug: project.slug.clone(),
            tool_exe_name: project.exe_name.clone(),
            tool_exe_path: target_exe_path.to_string_lossy().to_string(),
            tool_exe_size_bytes: exe_bytes.len() as u64,
            tool_exe_sha256: exe_sha256,
            pe_icon_layers_injected: layers_injected,
            pe_icon_group_resource_bytes: grp_bytes,
            pe_icon_first_layer_bytes: ico1_bytes,
            pe_embedded_pubkey_resource_id: PE_ED25519_PUBKEY_RESOURCE_ID,
            pe_embedded_pubkey_verified: true,
            authenticode_status: "Unsigned Portable PE Binary (PE Icon + RT_RCDATA #7777 Ed25519 Root-of-Trust Public Key injected at Step 5; ready for optional signtool.exe Authenticode code-signing as final post-resource step)".to_string(),
            key_id: key_meta.key_id,
            publisher_public_key_hex: key_meta.public_key_hex,
            external_keystore_path: key_meta.key_store_path,
            signature_algorithm: envelope.algorithm,
            signature_ed25519_hex: envelope.signature_ed25519_hex,
            total_duration_ms: total_start.elapsed().as_millis() as u64,
            steps,
            modules: module_states,
            dependency_audit: dep_audit,
        };

        std::fs::write(
            output_release_dir.join("build-report.json"),
            serde_json::to_vec_pretty(&report)?,
        )?;

        Ok(report)
    }
}

/// Seeds the two completely distinct PoC Game Projects in `projects/poc/jx2-mobile`
/// and `projects/poc/kingdom-slg`.
/// NOTE (Requirement 2): `initial_runtime_vars` is strictly EMPTY so that unverified runtime columns
/// never display fake pre-seeded numbers as if they were live data!
pub fn seed_two_poc_projects(repo_root: &Path) -> Result<(PathBuf, PathBuf)> {
    let jx2_dir = repo_root.join("projects").join("poc").join("jx2-mobile");
    std::fs::create_dir_all(jx2_dir.join("workflows"))?;
    std::fs::create_dir_all(jx2_dir.join("templates"))?;

    let jx2_wf1 = r#"{
        "schemaVersion": 1,
        "workflowId": "wf-jx2-main-quest",
        "name": "[NV] Chính Tuyến Tân Thủ & Đối Thoại Dã Tẩu",
        "status": "published",
        "graph": {
            "nodes": [
                { "id": "n-start", "type": "flow.start", "position": { "x": 50.0, "y": 50.0 } },
                { "id": "n-launch", "type": "app.launch", "position": { "x": 250.0, "y": 50.0 }, "config": { "packageId": "com.blackhole.jx2mobile" } },
                { "id": "n-wait", "type": "control.wait", "position": { "x": 450.0, "y": 50.0 }, "config": { "durationMs": 150 } },
                { "id": "n-cap", "type": "vision.capture", "position": { "x": 650.0, "y": 50.0 } },
                { "id": "n-find", "type": "vision.find_image", "position": { "x": 850.0, "y": 50.0 }, "config": { "templateRef": "bundle/templates/poc_quest_marker.png", "minConfidence": 0.85 } },
                { "id": "n-tap", "type": "action.random_safe_tap", "position": { "x": 850.0, "y": 220.0 }, "config": { "marginRatio": 0.18 } },
                { "id": "n-cp", "type": "state.checkpoint", "position": { "x": 550.0, "y": 220.0 }, "config": { "name": "jx2_main_quest_step1" } },
                { "id": "n-end", "type": "flow.end", "position": { "x": 250.0, "y": 220.0 } }
            ],
            "edges": [
                { "id": "e1", "source": "n-start", "sourcePort": "flow", "target": "n-launch", "targetPort": "flow" },
                { "id": "e2", "source": "n-launch", "sourcePort": "flow", "target": "n-wait", "targetPort": "flow" },
                { "id": "e3", "source": "n-wait", "sourcePort": "flow", "target": "n-cap", "targetPort": "flow" },
                { "id": "e4", "source": "n-cap", "sourcePort": "flow", "target": "n-find", "targetPort": "flow" },
                { "id": "e5", "source": "n-find", "sourcePort": "flow", "target": "n-tap", "targetPort": "flow" },
                { "id": "e6", "source": "n-tap", "sourcePort": "flow", "target": "n-cp", "targetPort": "flow" },
                { "id": "e7", "source": "n-cp", "sourcePort": "flow", "target": "n-end", "targetPort": "flow" }
            ]
        }
    }"#;

    let jx2_wf2 = r#"{
        "schemaVersion": 1,
        "workflowId": "wf-jx2-live-adb-probe",
        "name": "[ADB Live] Chụp Màn Hình, Khớp Mẫu SSD, Safe Tap Mở System Apps & OCR",
        "status": "published",
        "graph": {
            "nodes": [
                { "id": "n-start", "type": "flow.start", "position": { "x": 50.0, "y": 50.0 } },
                { "id": "n-cap", "type": "vision.capture", "position": { "x": 200.0, "y": 50.0 } },
                { "id": "n-find-sys", "type": "vision.find_image", "position": { "x": 380.0, "y": 50.0 }, "config": { "templateRef": "bundle/templates/live_ldplayer_system_apps.png", "minConfidence": 0.90 } },
                { "id": "n-tap-sys", "type": "action.random_safe_tap", "position": { "x": 560.0, "y": 50.0 }, "config": { "marginRatio": 0.18 } },
                { "id": "n-wait-open", "type": "control.wait", "position": { "x": 740.0, "y": 50.0 }, "config": { "durationMs": 1200 } },
                { "id": "n-cap-after", "type": "vision.capture", "position": { "x": 740.0, "y": 200.0 } },
                { "id": "n-ocr", "type": "vision.ocr", "position": { "x": 560.0, "y": 200.0 }, "config": { "expectedText": "" } },
                { "id": "n-cp", "type": "state.checkpoint", "position": { "x": 380.0, "y": 200.0 }, "config": { "name": "live_adb_verified" } },
                { "id": "n-end", "type": "flow.end", "position": { "x": 200.0, "y": 200.0 } }
            ],
            "edges": [
                { "id": "e1", "source": "n-start", "sourcePort": "flow", "target": "n-cap", "targetPort": "flow" },
                { "id": "e2", "source": "n-cap", "sourcePort": "flow", "target": "n-find-sys", "targetPort": "flow" },
                { "id": "e3", "source": "n-find-sys", "sourcePort": "flow", "target": "n-tap-sys", "targetPort": "flow" },
                { "id": "e4", "source": "n-tap-sys", "sourcePort": "flow", "target": "n-wait-open", "targetPort": "flow" },
                { "id": "e5", "source": "n-wait-open", "sourcePort": "flow", "target": "n-cap-after", "targetPort": "flow" },
                { "id": "e6", "source": "n-cap-after", "sourcePort": "flow", "target": "n-ocr", "targetPort": "flow" },
                { "id": "e7", "source": "n-ocr", "sourcePort": "flow", "target": "n-cp", "targetPort": "flow" },
                { "id": "e8", "source": "n-cp", "sourcePort": "flow", "target": "n-end", "targetPort": "flow" }
            ]
        }
    }"#;
    std::fs::write(jx2_dir.join("workflows/wf_jx2_main_quest.json"), jx2_wf1)?;
    std::fs::write(jx2_dir.join("workflows/wf_jx2_live_adb.json"), jx2_wf2)?;

    let jx2_manifest = GameProjectManifest {
        compatibility: CompatibilityVersions::default(),
        project_id: "proj-jx2-mobile-001".to_string(),
        slug: "jx2-mobile".to_string(),
        name: "Võ Hiệp Truyền Kỳ 2 Mobile".to_string(),
        game_package_id: "com.blackhole.jx2mobile".to_string(),
        version: "1.0.0".to_string(),
        exe_name: "Auto_JX2_Mobile.exe".to_string(),
        window_title: "Auto Võ Hiệp 2 Mobile v1.0.0".to_string(),
        icon_rel_path: "branding/icon.ico".to_string(),
        target_resolution: (1920, 1080),
        packaged_modules: vec![
            "mod.adb_core".to_string(),
            "mod.vision_ocr".to_string(),
            "mod.account_vault".to_string(),
            "mod.proxy_network".to_string(),
            "mod.emulator_optimizer".to_string(),
        ],
        workflow_files: vec![
            "workflows/wf_jx2_main_quest.json".to_string(),
            "workflows/wf_jx2_live_adb.json".to_string(),
        ],
        template_files: vec![
            "templates/poc_quest_marker.png".to_string(),
            "templates/live_ldplayer_system_apps.png".to_string(),
            "templates/btn_dang_nhap.png".to_string(),
            "templates/btn_choi_ngay.png".to_string(),
        ],
        bundle_adb: true,
    };
    std::fs::write(
        jx2_dir.join("project.json"),
        serde_json::to_vec_pretty(&jx2_manifest)?,
    )?;

    let jx2_ui = MiniToolUiSchema {
        schema_version: 1,
        project_slug: "jx2-mobile".to_string(),
        window_title: "Auto Võ Hiệp 2 Mobile v1.0.0".to_string(),
        version_badge: "JX2-MMORPG v1.0".to_string(),
        theme_accent_hex: "#F59E0B".to_string(),
        header_bg_hex: "#18131A".to_string(),
        layout_preset: MiniLayoutPreset::Compact2Col,
        default_window_size: (740, 660),
        sections_order: vec![
            "device_table".to_string(),
            "task_and_log_split".to_string(),
            "footer_controls".to_string(),
        ],
        hidden_components: vec![],
        device_columns: vec![
            RuntimeColumnBinding {
                id: "col-level".to_string(),
                label: "Cấp".to_string(),
                runtime_var_key: "char_level".to_string(),
                fallback_display: "Chưa OCR (-)".to_string(),
                min_window_width: 360,
                badge_color: Some("#FBBF24".to_string()),
            },
            RuntimeColumnBinding {
                id: "col-gold".to_string(),
                label: "Vàng".to_string(),
                runtime_var_key: "char_gold".to_string(),
                fallback_display: "Chưa OCR (-)".to_string(),
                min_window_width: 480,
                badge_color: Some("#F59E0B".to_string()),
            },
            RuntimeColumnBinding {
                id: "col-silver".to_string(),
                label: "Bạc".to_string(),
                runtime_var_key: "char_silver".to_string(),
                fallback_display: "Chưa OCR (-)".to_string(),
                min_window_width: 560,
                badge_color: Some("#94A3B8".to_string()),
            },
        ],
        task_categories: vec![
            TaskCategoryDef {
                id: "cat-nv".to_string(),
                short_tag: "[NV]".to_string(),
                display_name: "Nhiệm vụ Ngày".to_string(),
                color_hex: "#38BDF8".to_string(),
            },
            TaskCategoryDef {
                id: "cat-pb".to_string(),
                short_tag: "[PB]".to_string(),
                display_name: "Phó bản Bang".to_string(),
                color_hex: "#A78BFA".to_string(),
            },
            TaskCategoryDef {
                id: "cat-hd".to_string(),
                short_tag: "[HĐ]".to_string(),
                display_name: "Hoạt động Giờ".to_string(),
                color_hex: "#F59E0B".to_string(),
            },
        ],
        tasks: vec![
            MiniTaskDef {
                id: "task-jx2-main".to_string(),
                title: "Chính Tuyến & Đối thoại Dã Tẩu".to_string(),
                category_id: "cat-nv".to_string(),
                workflow_id: "wf-jx2-main-quest".to_string(),
                default_enabled: true,
                priority: 1,
                estimate_sec: 45,
                settings_schema: vec![
                    TaskSettingField {
                        key: "auto_skip_dialog".to_string(),
                        label: "Tự động bỏ qua hội thoại NPC".to_string(),
                        field_type: "boolean".to_string(),
                        default_value: serde_json::json!(true),
                        options: None,
                    },
                    TaskSettingField {
                        key: "max_quest_loops".to_string(),
                        label: "Số chuỗi Dã Tẩu tối đa".to_string(),
                        field_type: "number".to_string(),
                        default_value: serde_json::json!(10),
                        options: None,
                    },
                ],
            },
            MiniTaskDef {
                id: "task-jx2-treasure".to_string(),
                title: "Tìm Kho Báu & Đào Tàng Bảo Đồ".to_string(),
                category_id: "cat-nv".to_string(),
                workflow_id: "wf-jx2-main-quest".to_string(),
                default_enabled: true,
                priority: 2,
                estimate_sec: 60,
                settings_schema: vec![TaskSettingField {
                    key: "use_golden_shovel".to_string(),
                    label: "Dùng La Bàn Hoàng Kim".to_string(),
                    field_type: "boolean".to_string(),
                    default_value: serde_json::json!(false),
                    options: None,
                }],
            },
            MiniTaskDef {
                id: "task-jx2-dungeon".to_string(),
                title: "Sơn Tặc Mật Quật & Bí Cảnh Tổ Đội".to_string(),
                category_id: "cat-pb".to_string(),
                workflow_id: "wf-jx2-main-quest".to_string(),
                default_enabled: true,
                priority: 3,
                estimate_sec: 120,
                settings_schema: vec![TaskSettingField {
                    key: "difficulty".to_string(),
                    label: "Độ khó Phó bản".to_string(),
                    field_type: "select".to_string(),
                    default_value: serde_json::json!("Anh Hùng"),
                    options: Some(vec![
                        "Thường".to_string(),
                        "Anh Hùng".to_string(),
                        "Địa Ngục".to_string(),
                    ]),
                }],
            },
            MiniTaskDef {
                id: "task-jx2-campfire".to_string(),
                title: "Lửa Trại Bang & Chiến Trường Tống Kim".to_string(),
                category_id: "cat-hd".to_string(),
                workflow_id: "wf-jx2-main-quest".to_string(),
                default_enabled: false,
                priority: 4,
                estimate_sec: 180,
                settings_schema: vec![],
            },
            MiniTaskDef {
                id: "task-jx2-live-adb".to_string(),
                title: "Kiểm tra Thiết bị ADB Thật (Chụp màn hình + OCR + Vuốt)".to_string(),
                category_id: "cat-nv".to_string(),
                workflow_id: "wf-jx2-live-adb-probe".to_string(),
                default_enabled: true,
                priority: 5,
                estimate_sec: 15,
                settings_schema: vec![],
            },
        ],
        utilities: vec![
            UtilityToggleDef {
                id: "util-use-items".to_string(),
                label: "Tự dùng Rương Hoàng Kim & Sách Tu Vi".to_string(),
                default_enabled: true,
                required_module: None,
                settings_schema: vec![],
            },
            UtilityToggleDef {
                id: "util-proxy-route".to_string(),
                label: "Định tuyến SOCKS5 Proxy theo Tài khoản".to_string(),
                default_enabled: false,
                required_module: Some("mod.proxy_network".to_string()),
                settings_schema: vec![],
            },
        ],
        // Strictly empty initial runtime vars — values appear ONLY after live verification/execution
        initial_runtime_vars: HashMap::new(),
    };
    std::fs::write(
        jx2_dir.join("ui-schema.json"),
        serde_json::to_vec_pretty(&jx2_ui)?,
    )?;

    // GAME 2: Kingdom Rise: Tactics SLG
    let slg_dir = repo_root.join("projects").join("poc").join("kingdom-slg");
    std::fs::create_dir_all(slg_dir.join("workflows"))?;
    std::fs::create_dir_all(slg_dir.join("templates"))?;

    let slg_wf1 = r#"{
        "schemaVersion": 1,
        "workflowId": "wf-slg-harvest-and-train",
        "name": "[Nông Nghiệp] Thu Hoạch Tài Nguyên & Huấn Luyện Kỵ Binh",
        "status": "published",
        "graph": {
            "nodes": [
                { "id": "n-start", "type": "flow.start", "position": { "x": 50.0, "y": 50.0 } },
                { "id": "n-launch", "type": "app.launch", "position": { "x": 250.0, "y": 50.0 }, "config": { "packageId": "com.kingdom.rise.slg" } },
                { "id": "n-cap", "type": "vision.capture", "position": { "x": 450.0, "y": 50.0 } },
                { "id": "n-find-food", "type": "vision.find_image", "position": { "x": 650.0, "y": 50.0 }, "config": { "templateRef": "bundle/templates/btn_harvest_food.png", "minConfidence": 0.85 } },
                { "id": "n-tap-food", "type": "action.random_safe_tap", "position": { "x": 650.0, "y": 220.0 }, "config": { "marginRatio": 0.20 } },
                { "id": "n-cp", "type": "state.checkpoint", "position": { "x": 400.0, "y": 220.0 }, "config": { "name": "slg_city_harvested" } },
                { "id": "n-end", "type": "flow.end", "position": { "x": 150.0, "y": 220.0 } }
            ],
            "edges": [
                { "id": "e1", "source": "n-start", "sourcePort": "flow", "target": "n-launch", "targetPort": "flow" },
                { "id": "e2", "source": "n-launch", "sourcePort": "flow", "target": "n-cap", "targetPort": "flow" },
                { "id": "e3", "source": "n-cap", "sourcePort": "flow", "target": "n-find-food", "targetPort": "flow" },
                { "id": "e4", "source": "n-find-food", "sourcePort": "flow", "target": "n-tap-food", "targetPort": "flow" },
                { "id": "e5", "source": "n-tap-food", "sourcePort": "flow", "target": "n-cp", "targetPort": "flow" },
                { "id": "e6", "source": "n-cp", "sourcePort": "flow", "target": "n-end", "targetPort": "flow" }
            ]
        }
    }"#;
    std::fs::write(slg_dir.join("workflows/wf_slg_harvest.json"), slg_wf1)?;

    let slg_manifest = GameProjectManifest {
        compatibility: CompatibilityVersions::default(),
        project_id: "proj-kingdom-slg-002".to_string(),
        slug: "kingdom-slg".to_string(),
        name: "Kingdom Rise: Tactics SLG".to_string(),
        game_package_id: "com.kingdom.rise.slg".to_string(),
        version: "1.2.0".to_string(),
        exe_name: "Auto_Kingdom_SLG.exe".to_string(),
        window_title: "Auto Kingdom Rise: Tactics SLG v1.2.0".to_string(),
        icon_rel_path: "branding/icon.ico".to_string(),
        target_resolution: (1280, 720),
        packaged_modules: vec![
            "mod.adb_core".to_string(),
            "mod.vision_ocr".to_string(),
            "mod.account_vault".to_string(),
            "mod.emulator_optimizer".to_string(),
        ],
        workflow_files: vec!["workflows/wf_slg_harvest.json".to_string()],
        template_files: vec![
            "templates/btn_harvest_food.png".to_string(),
            "templates/btn_train_cavalry.png".to_string(),
            "templates/btn_alliance_help.png".to_string(),
        ],
        bundle_adb: true,
    };
    std::fs::write(
        slg_dir.join("project.json"),
        serde_json::to_vec_pretty(&slg_manifest)?,
    )?;

    let slg_ui = MiniToolUiSchema {
        schema_version: 1,
        project_slug: "kingdom-slg".to_string(),
        window_title: "Auto Kingdom Rise: Tactics SLG v1.2.0".to_string(),
        version_badge: "4X-SLG v1.2".to_string(),
        theme_accent_hex: "#38BDF8".to_string(),
        header_bg_hex: "#111827".to_string(),
        layout_preset: MiniLayoutPreset::Compact2Col,
        default_window_size: (740, 660),
        sections_order: vec![
            "device_table".to_string(),
            "task_and_log_split".to_string(),
            "footer_controls".to_string(),
        ],
        hidden_components: vec!["proxy_settings_panel".to_string()],
        device_columns: vec![
            RuntimeColumnBinding {
                id: "col-castle".to_string(),
                label: "Thành Chính".to_string(),
                runtime_var_key: "castle_tier".to_string(),
                fallback_display: "Chưa OCR (-)".to_string(),
                min_window_width: 360,
                badge_color: Some("#A78BFA".to_string()),
            },
            RuntimeColumnBinding {
                id: "col-food".to_string(),
                label: "Lương Thực".to_string(),
                runtime_var_key: "food_stock".to_string(),
                fallback_display: "Chưa OCR (-)".to_string(),
                min_window_width: 460,
                badge_color: Some("#34D399".to_string()),
            },
            RuntimeColumnBinding {
                id: "col-wood".to_string(),
                label: "Gỗ".to_string(),
                runtime_var_key: "wood_stock".to_string(),
                fallback_display: "Chưa OCR (-)".to_string(),
                min_window_width: 540,
                badge_color: Some("#FBBF24".to_string()),
            },
            RuntimeColumnBinding {
                id: "col-power".to_string(),
                label: "Quân Lực".to_string(),
                runtime_var_key: "troop_power".to_string(),
                fallback_display: "Chưa OCR (-)".to_string(),
                min_window_width: 620,
                badge_color: Some("#38BDF8".to_string()),
            },
        ],
        task_categories: vec![
            TaskCategoryDef {
                id: "cat-eco".to_string(),
                short_tag: "[Nông Nghiệp]".to_string(),
                display_name: "Kinh tế & Thu hoạch".to_string(),
                color_hex: "#34D399".to_string(),
            },
            TaskCategoryDef {
                id: "cat-mil".to_string(),
                short_tag: "[Quân Sự]".to_string(),
                display_name: "Huấn luyện & Trinh sát".to_string(),
                color_hex: "#F87171".to_string(),
            },
            TaskCategoryDef {
                id: "cat-ally".to_string(),
                short_tag: "[Liên Minh]".to_string(),
                display_name: "Pháo đài & Công thành".to_string(),
                color_hex: "#38BDF8".to_string(),
            },
        ],
        tasks: vec![
            MiniTaskDef {
                id: "task-slg-harvest".to_string(),
                title: "Thu hoạch Lương Thực, Gỗ & Đá trong Thành".to_string(),
                category_id: "cat-eco".to_string(),
                workflow_id: "wf-slg-harvest-and-train".to_string(),
                default_enabled: true,
                priority: 1,
                estimate_sec: 30,
                settings_schema: vec![TaskSettingField {
                    key: "use_production_boost".to_string(),
                    label: "Dùng Thẻ Tăng Sản Lượng 25%".to_string(),
                    field_type: "boolean".to_string(),
                    default_value: serde_json::json!(true),
                    options: None,
                }],
            },
            MiniTaskDef {
                id: "task-slg-cavalry".to_string(),
                title: "Huấn luyện Kỵ Binh Hạng Nặng (Tier 4)".to_string(),
                category_id: "cat-mil".to_string(),
                workflow_id: "wf-slg-harvest-and-train".to_string(),
                default_enabled: true,
                priority: 2,
                estimate_sec: 50,
                settings_schema: vec![TaskSettingField {
                    key: "troop_tier".to_string(),
                    label: "Cấp Binh Chủng".to_string(),
                    field_type: "select".to_string(),
                    default_value: serde_json::json!("Tier 4 - Thiết Kỵ"),
                    options: Some(vec![
                        "Tier 3 - Kỵ Binh".to_string(),
                        "Tier 4 - Thiết Kỵ".to_string(),
                        "Tier 5 - Hoàng Gia".to_string(),
                    ]),
                }],
            },
            MiniTaskDef {
                id: "task-slg-barbarians".to_string(),
                title: "Phái Đạo Quân Tiêu Diệt Man Tộc Cấp 15".to_string(),
                category_id: "cat-mil".to_string(),
                workflow_id: "wf-slg-harvest-and-train".to_string(),
                default_enabled: true,
                priority: 3,
                estimate_sec: 90,
                settings_schema: vec![],
            },
            MiniTaskDef {
                id: "task-slg-alliance".to_string(),
                title: "Bấm Hỗ Trợ Xây Dựng & Tiếp Viện Pháo Đài Liên Minh".to_string(),
                category_id: "cat-ally".to_string(),
                workflow_id: "wf-slg-harvest-and-train".to_string(),
                default_enabled: true,
                priority: 4,
                estimate_sec: 25,
                settings_schema: vec![],
            },
        ],
        utilities: vec![
            UtilityToggleDef {
                id: "util-peace-shield".to_string(),
                label: "Tự bật Khiên Hòa Bình 8h khi bị Trinh Sát".to_string(),
                default_enabled: true,
                required_module: None,
                settings_schema: vec![],
            },
            UtilityToggleDef {
                id: "util-hospital-heal".to_string(),
                label: "Tự động Cứu Chữa Thương Binh tại Quân Y Viện".to_string(),
                default_enabled: true,
                required_module: None,
                settings_schema: vec![],
            },
        ],
        // Strictly empty initial runtime vars
        initial_runtime_vars: HashMap::new(),
    };
    std::fs::write(
        slg_dir.join("ui-schema.json"),
        serde_json::to_vec_pretty(&slg_ui)?,
    )?;

    Ok((jx2_dir, slg_dir))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ed25519_signing_key_rotation_and_tamper_scenarios() {
        let temp_dir = std::env::temp_dir().join(format!("fp_ed25519_test_{}", uuid::Uuid::new_v4()));
        let keystore_dir = temp_dir.join("keystore");
        let bundle_dir = temp_dir.join("bundle");
        std::fs::create_dir_all(&bundle_dir).unwrap();

        let (sk1, meta1) = ProducerKeyManager::load_or_create_signing_key(&keystore_dir).unwrap();

        let manifest_bytes = b"{\"project\":\"jx2\"}";
        let ui_bytes = b"{\"title\":\"Original Title\"}";
        std::fs::write(bundle_dir.join("manifest.fpbundle.json"), manifest_bytes).unwrap();
        std::fs::write(bundle_dir.join("ui-schema.json"), ui_bytes).unwrap();

        let mut checksums = BTreeMap::new();
        checksums.insert("manifest.fpbundle.json".to_string(), sha256_hex(manifest_bytes));
        checksums.insert("ui-schema.json".to_string(), sha256_hex(ui_bytes));

        let env = ProducerKeyManager::sign_bundle_ed25519(
            &sk1,
            &meta1.key_id,
            sha256_hex(manifest_bytes),
            checksums,
        );

        // 1. Valid Ed25519 verification passes
        assert!(env
            .verify_ed25519_with_public_key(&bundle_dir, &meta1.public_key_hex)
            .is_ok());

        // 2. Scenario A: Modify bundle file AND recompute SHA-256 table -> rejected with ED25519_SIGNATURE_INVALID
        let tampered_ui = b"{\"title\":\"Tampered\"}";
        std::fs::write(bundle_dir.join("ui-schema.json"), tampered_ui).unwrap();
        let mut forged_env = env.clone();
        forged_env
            .file_checksums
            .insert("ui-schema.json".to_string(), sha256_hex(tampered_ui));
        let new_table = serde_json::to_string(&forged_env.file_checksums).unwrap();
        forged_env.checksums_table_sha256 = sha256_hex(new_table.as_bytes());
        let res_a = forged_env.verify_ed25519_with_public_key(&bundle_dir, &meta1.public_key_hex);
        assert!(res_a.is_err());
        assert!(res_a
            .unwrap_err()
            .to_string()
            .contains("ED25519_SIGNATURE_INVALID"));

        // Restore original ui-schema.json
        std::fs::write(bundle_dir.join("ui-schema.json"), ui_bytes).unwrap();

        // 3. Scenario B: Rotate Producer Key -> old signature verified against NEW rotated public key is rejected with ED25519_PUBKEY_UNTRUSTED
        let (_sk2, meta2) =
            ProducerKeyManager::rotate_producer_key(&keystore_dir, "rotate_test").unwrap();
        assert_ne!(meta1.public_key_hex, meta2.public_key_hex);
        let res_b = env.verify_ed25519_with_public_key(&bundle_dir, &meta2.public_key_hex);
        assert!(res_b.is_err());
        assert!(res_b
            .unwrap_err()
            .to_string()
            .contains("ED25519_PUBKEY_UNTRUSTED"));

        // 4. Scenario C: Corrupt signature bytes -> rejected with ED25519_SIGNATURE_INVALID
        let mut corrupt_sig_env = env.clone();
        corrupt_sig_env.signature_ed25519_hex = "00".repeat(64);
        let res_c =
            corrupt_sig_env.verify_ed25519_with_public_key(&bundle_dir, &meta1.public_key_hex);
        assert!(res_c.is_err());
        assert!(res_c
            .unwrap_err()
            .to_string()
            .contains("ED25519_SIGNATURE_INVALID"));

        // 5. Scenario D: Missing manifest.fpbundle.json -> rejected with INTEGRITY_MISSING_MANIFEST
        std::fs::remove_file(bundle_dir.join("manifest.fpbundle.json")).unwrap();
        let res_d = env.verify_ed25519_with_public_key(&bundle_dir, &meta1.public_key_hex);
        assert!(res_d.is_err());
        assert!(res_d
            .unwrap_err()
            .to_string()
            .contains("INTEGRITY_MISSING_MANIFEST"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_five_state_module_and_capability_enforcement() {
        let packaged = vec![
            "mod.account_vault".to_string(),
            "mod.emulator_optimizer".to_string(),
        ];
        let registry = ModuleRegistry::resolve_for_release(&packaged, &packaged);

        assert!(registry
            .assert_capability_granted("mod.account_vault", &ModuleCapability::CredentialVaultAccess)
            .is_ok());
        assert!(registry
            .assert_capability_granted("mod.account_vault", &ModuleCapability::OutboundProxyConnect)
            .is_err());
        assert!(registry.assert_module_active("mod.proxy_network").is_err());
    }

    #[tokio::test]
    async fn test_multi_device_concurrent_isolation_and_fencing() {
        let report = orchestrator::run_concurrent_two_device_verification()
            .await
            .unwrap();
        assert!(report.passed);
    }

    #[test]
    fn test_pe_embedded_public_key_root_of_trust_blocks_disk_pubkey_replacement() {
        let temp_dir =
            std::env::temp_dir().join(format!("fp_pe_root_trust_test_{}", uuid::Uuid::new_v4()));
        let keystore_dir = temp_dir.join("producer_keystore");
        let rogue_keystore_dir = temp_dir.join("rogue_keystore");
        let bundle_dir = temp_dir.join("bundle");
        std::fs::create_dir_all(&bundle_dir).unwrap();

        // 1. Copy the current test runner PE .exe into temp_dir and verify it initially fails closed with PE_ROOT_OF_TRUST_MISSING
        let current_pe = std::env::current_exe().unwrap();
        let target_exe = temp_dir.join("MiniTool_PE_RootOfTrust_Test.exe");
        std::fs::copy(&current_pe, &target_exe).unwrap();

        let missing_res = read_pe_embedded_pubkey_from_exe(&target_exe);
        assert!(missing_res.is_err());
        assert!(missing_res
            .unwrap_err()
            .to_string()
            .contains("PE_ROOT_OF_TRUST_MISSING"));

        // Reject malformed public key lengths
        assert!(inject_ed25519_pubkey_into_pe_exe(&target_exe, "short_hex").is_err());

        let (producer_sk, producer_meta) =
            ProducerKeyManager::load_or_create_signing_key(&keystore_dir).unwrap();
        inject_ed25519_pubkey_into_pe_exe(&target_exe, &producer_meta.public_key_hex).unwrap();

        // 2. Read back the pinned public key from the PE binary's RT_RCDATA #7777 resource
        let pinned_pe_pubkey = read_pe_embedded_pubkey_from_exe(&target_exe).unwrap();
        assert_eq!(pinned_pe_pubkey, producer_meta.public_key_hex);

        // 3. Producer signs the bundle
        let manifest_bytes = b"{\"project\":\"jx2-mobile\"}";
        let ui_bytes = b"{\"windowTitle\":\"Official Auto JX2\"}";
        std::fs::write(bundle_dir.join("manifest.fpbundle.json"), manifest_bytes).unwrap();
        std::fs::write(bundle_dir.join("ui-schema.json"), ui_bytes).unwrap();

        let mut checksums = BTreeMap::new();
        checksums.insert(
            "manifest.fpbundle.json".to_string(),
            sha256_hex(manifest_bytes),
        );
        checksums.insert("ui-schema.json".to_string(), sha256_hex(ui_bytes));

        let official_env = ProducerKeyManager::sign_bundle_ed25519(
            &producer_sk,
            &producer_meta.key_id,
            sha256_hex(manifest_bytes),
            checksums,
        );
        assert!(official_env
            .verify_ed25519_with_public_key(&bundle_dir, &pinned_pe_pubkey)
            .is_ok());

        // 4. ATTACK SCENARIO: Attacker modifies ui-schema.json, generates their OWN rogue Ed25519 keypair,
        //    re-signs the bundle, AND overwrites trusted_publisher.pub on disk!
        let (rogue_sk, rogue_meta) =
            ProducerKeyManager::load_or_create_signing_key(&rogue_keystore_dir).unwrap();
        assert_ne!(producer_meta.public_key_hex, rogue_meta.public_key_hex);

        let hacked_ui = b"{\"windowTitle\":\"HACKED BY ROGUE KEYPAIR\"}";
        std::fs::write(bundle_dir.join("ui-schema.json"), hacked_ui).unwrap();
        std::fs::write(
            bundle_dir.join("trusted_publisher.pub"),
            rogue_meta.public_key_hex.as_bytes(),
        )
        .unwrap();

        let mut rogue_checksums = BTreeMap::new();
        rogue_checksums.insert(
            "manifest.fpbundle.json".to_string(),
            sha256_hex(manifest_bytes),
        );
        rogue_checksums.insert("ui-schema.json".to_string(), sha256_hex(hacked_ui));

        let rogue_env = ProducerKeyManager::sign_bundle_ed25519(
            &rogue_sk,
            &rogue_meta.key_id,
            sha256_hex(manifest_bytes),
            rogue_checksums.clone(),
        );

        // Also verify deterministic RFC 8032 rogue vector used by mini-runtime Case 5 tamper suite
        let det_seed = fp_bundle_verify::hex_decode(
            "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
        )
        .unwrap();
        let det_sk = ed25519_dalek::SigningKey::from_bytes(det_seed.as_slice().try_into().unwrap());
        let det_env = ProducerKeyManager::sign_bundle_ed25519(
            &det_sk,
            "fp-ed25519-rogue-attacker",
            sha256_hex(manifest_bytes),
            rogue_checksums,
        );
        assert_eq!(
            det_env.signature_ed25519_hex,
            "e407316b446d120cc627328f731a68d371db3e23a73fcaccb1767bedfb588953676b5331dddf799b0c95f72cc587df25e1f7b5f5cb0d18221ffdf24e08ed9c02"
        );
        assert!(det_env
            .verify_ed25519_with_public_key(
                &bundle_dir,
                "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
            )
            .is_ok());

        // Verify that the rogue envelope IS 100% cryptographically valid against the disk trusted_publisher.pub
        let disk_pubkey = std::fs::read_to_string(bundle_dir.join("trusted_publisher.pub")).unwrap();
        assert!(rogue_env
            .verify_ed25519_with_public_key(&bundle_dir, &disk_pubkey)
            .is_ok());

        // Even though rogue_env is cryptographically valid under rogue_meta.public_key_hex (on disk),
        // verifying against the PE-embedded Root of Trust public key inside target_exe FAILS CLOSED!
        let attack_res = rogue_env.verify_ed25519_with_public_key(&bundle_dir, &pinned_pe_pubkey);
        assert!(attack_res.is_err());
        assert!(attack_res
            .unwrap_err()
            .to_string()
            .contains("ED25519_PUBKEY_UNTRUSTED"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_old_minitool_continues_verifying_old_bundle_after_producer_key_rotation() {
        let temp_dir = std::env::temp_dir().join(format!(
            "fp_minitool_rotation_compat_{}",
            uuid::Uuid::new_v4()
        ));
        let keystore_dir = temp_dir.join("keystore");
        let old_bundle_dir = temp_dir.join("old_release_bundle");
        let new_bundle_dir = temp_dir.join("new_release_bundle");
        std::fs::create_dir_all(&old_bundle_dir).unwrap();
        std::fs::create_dir_all(&new_bundle_dir).unwrap();

        let current_pe = std::env::current_exe().unwrap();
        let old_minitool_exe = temp_dir.join("Auto_Game_v1.exe");
        let new_minitool_exe = temp_dir.join("Auto_Game_v2.exe");
        std::fs::copy(&current_pe, &old_minitool_exe).unwrap();
        std::fs::copy(&current_pe, &new_minitool_exe).unwrap();

        // 1. Key1 signs v1 Mini Tool and embeds Key1 PubKey in RT_RCDATA #7777
        let (sk1, meta1) = ProducerKeyManager::load_or_create_signing_key(&keystore_dir).unwrap();
        inject_ed25519_pubkey_into_pe_exe(&old_minitool_exe, &meta1.public_key_hex).unwrap();

        let m1 = b"{\"version\":\"1.0.0\"}";
        std::fs::write(old_bundle_dir.join("manifest.fpbundle.json"), m1).unwrap();
        let mut cs1 = BTreeMap::new();
        cs1.insert("manifest.fpbundle.json".to_string(), sha256_hex(m1));
        let env1 = ProducerKeyManager::sign_bundle_ed25519(&sk1, &meta1.key_id, sha256_hex(m1), cs1);

        // 2. Producer rotates key -> Key2 becomes active, Key1 is archived
        let (sk2, meta2) = ProducerKeyManager::rotate_producer_key(&keystore_dir, "scheduled_rotation").unwrap();
        assert_ne!(meta1.key_id, meta2.key_id);
        inject_ed25519_pubkey_into_pe_exe(&new_minitool_exe, &meta2.public_key_hex).unwrap();

        let m2 = b"{\"version\":\"2.0.0\"}";
        std::fs::write(new_bundle_dir.join("manifest.fpbundle.json"), m2).unwrap();
        let mut cs2 = BTreeMap::new();
        cs2.insert("manifest.fpbundle.json".to_string(), sha256_hex(m2));
        let env2 = ProducerKeyManager::sign_bundle_ed25519(&sk2, &meta2.key_id, sha256_hex(m2), cs2);

        // 3. Old Mini Tool v1 STILL verifies its old bundle using its embedded Key1 RT_RCDATA #7777!
        let old_pe_pub = read_pe_embedded_pubkey_from_exe(&old_minitool_exe).unwrap();
        assert_eq!(old_pe_pub, meta1.public_key_hex);
        assert!(env1.verify_ed25519_with_public_key(&old_bundle_dir, &old_pe_pub).is_ok());

        // 4. New Mini Tool v2 verifies new bundle using Key2, and rejects Key1 bundle (cross-version mismatch)
        let new_pe_pub = read_pe_embedded_pubkey_from_exe(&new_minitool_exe).unwrap();
        assert_eq!(new_pe_pub, meta2.public_key_hex);
        assert!(env2.verify_ed25519_with_public_key(&new_bundle_dir, &new_pe_pub).is_ok());
        assert!(env1.verify_ed25519_with_public_key(&old_bundle_dir, &new_pe_pub).is_err());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
