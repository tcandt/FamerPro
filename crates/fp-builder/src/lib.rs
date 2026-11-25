pub mod authenticode;
pub mod orchestrator;
pub mod pe_icon;
pub mod pe_version;
pub mod security;

pub use fp_bundle_contract as contracts;
pub use fp_bundle_verify as verify;

use anyhow::{bail, Context, Result};
use authenticode::{
    sign_pe_authenticode_and_verify, verify_publisher_matches_cert_subject, AuthenticodeSignRequest,
};
use fp_bundle_contract::*;
use fp_bundle_verify::{
    read_pe_embedded_pubkey_from_exe, read_pe_version_info_from_exe, sha256_hex,
    validate_safe_relative_path, verify_authenticode_pe, SignedBundleEnvelope,
    PE_ED25519_PUBKEY_RESOURCE_ID,
};
use fp_workflow_schema::{WorkflowDocument, WorkflowValidator};
use pe_icon::{
    inject_ed25519_pubkey_into_pe_exe, inject_ico_into_pe_exe, verify_pe_embedded_pubkey_resource,
    verify_pe_icon_resource,
};
use pe_version::inject_pe_version_info;
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseBuildOptions {
    #[serde(default)]
    pub signing_mode: SigningMode,
    pub authenticode_cert_ref: Option<String>,
    pub timestamp_url: Option<String>,
    pub publisher_profile: Option<PublisherIdentitySpec>,
    pub version_info: Option<PeVersionInfoSpec>,
    pub key_rotation_delegation: Option<KeyRotationDelegation>,
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
    #[serde(default)]
    pub signing_mode: SigningMode,
    #[serde(default)]
    pub trusted_public_release: bool,
    #[serde(default)]
    pub version_info: Option<PeVersionInfoSpec>,
    #[serde(default)]
    pub authenticode_verification: Option<AuthenticodeVerificationReport>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildPreflightReport {
    pub ready: bool,
    pub project_id: String,
    pub project_slug: String,
    pub project_name: String,
    pub exe_name: String,
    pub workflow_count: usize,
    pub template_count: usize,
    pub runtime_exe_present: bool,
    pub icon_present: bool,
    pub adb_ready: bool,
    pub keystore_key_id: Option<String>,
    pub issues: Vec<String>,
    pub checks: Vec<BuildStepMetric>,
    pub modules: Vec<ModuleStateReport>,
}

pub struct MiniToolPackager;

impl MiniToolPackager {
    /// Default P5.2/P5.1-compatible release export wrapper (defaults to `UnsignedInternal` + derived PE VERSIONINFO).
    pub fn build_and_export_release(
        project_dir: &Path,
        runtime_template_exe: &Path,
        workspace_bin_dir: Option<&Path>,
        output_release_dir: &Path,
        custom_keystore_dir: Option<&Path>,
    ) -> Result<BuildReport> {
        Self::build_and_export_release_with_options(
            project_dir,
            runtime_template_exe,
            workspace_bin_dir,
            output_release_dir,
            custom_keystore_dir,
            &ReleaseBuildOptions::default(),
        )
    }

    /// Full P5.3 8-step Release Pipeline:
    /// Build -> Inject Icon + PE VERSIONINFO + Ed25519 RT_RCDATA #7777 -> Finalize & Sign Bundle ->
    /// Sign Authenticode (signtool.exe) -> Verify Authenticode (WinVerifyTrust) & SHA-256 -> Emit BuildReport.
    pub fn build_and_export_release_with_options(
        project_dir: &Path,
        runtime_template_exe: &Path,
        workspace_bin_dir: Option<&Path>,
        output_release_dir: &Path,
        custom_keystore_dir: Option<&Path>,
        options: &ReleaseBuildOptions,
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
        let (signing_key, key_meta) =
            ProducerKeyManager::load_or_create_signing_key(&keystore_dir)?;

        // Build derived PE VERSIONINFO cleanly if caller didn't override every field
        let mut version_spec = options
            .version_info
            .clone()
            .unwrap_or_else(|| PeVersionInfoSpec {
                company_name: options
                    .publisher_profile
                    .as_ref()
                    .map(|p| p.organization.clone())
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| "FarmerPro Internal Studio".to_string()),
                file_description: format!("{} Automated Mini Tool", project.name),
                file_version: project.version.clone(),
                internal_name: project.slug.clone(),
                legal_copyright: options
                    .publisher_profile
                    .as_ref()
                    .map(|p| p.copyright.clone())
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| format!("Copyright (c) 2026 {}", project.name)),
                original_filename: project.exe_name.clone(),
                product_name: project.name.clone(),
                product_version: project.version.clone(),
            });
        if version_spec.original_filename.trim().is_empty() {
            version_spec.original_filename = project.exe_name.clone();
        }
        if version_spec.internal_name.trim().is_empty() {
            version_spec.internal_name = project.slug.clone();
        }
        if version_spec.file_version.trim().is_empty() {
            version_spec.file_version = project.version.clone();
        }
        if version_spec.product_version.trim().is_empty() {
            version_spec.product_version = project.version.clone();
        }

        // STEP 5: Inject Custom PE Icon + Win32 VERSIONINFO + Ed25519 Public Key in PE RT_RCDATA #7777 BEFORE Signing
        let t5 = Instant::now();
        let layers_injected = inject_ico_into_pe_exe(&target_exe_path, &icon_path)?;
        inject_pe_version_info(&target_exe_path, &version_spec)?;
        inject_ed25519_pubkey_into_pe_exe(&target_exe_path, &key_meta.public_key_hex)?;
        // Verify PE Icon, VERSIONINFO, and RT_RCDATA #7777 BEFORE bundle finalization / Authenticode signing
        let (grp_bytes, ico1_bytes) = verify_pe_icon_resource(&target_exe_path)?;
        let verified_vi = read_pe_version_info_from_exe(&target_exe_path)?;
        let verified_pe_pubkey =
            verify_pe_embedded_pubkey_resource(&target_exe_path, &key_meta.public_key_hex)?;
        steps.push(BuildStepMetric {
            step_no: 5,
            name: "PE Icon + Win32 VERSIONINFO + Ed25519 Root-of-Trust Key Injection".to_string(),
            duration_ms: t5.elapsed().as_millis() as u64,
            status: "PASS".to_string(),
            detail: format!(
                "Injected {} icon layers ({}B/{}B), VERSIONINFO (Product='{}', Company='{}'), PE RT_RCDATA #{}={}",
                layers_injected,
                grp_bytes,
                ico1_bytes,
                verified_vi.product_name,
                verified_vi.company_name,
                PE_ED25519_PUBKEY_RESOURCE_ID,
                &verified_pe_pubkey[..16],
            ),
        });

        // STEP 6: Package Bundle Assets & Optional ADB Dependencies
        let t6 = Instant::now();
        let bundle_dir = output_release_dir.join("bundle");
        let bundle_tpl_dir = bundle_dir.join("templates");
        std::fs::create_dir_all(&bundle_tpl_dir)?;

        let effective_publisher = options.publisher_profile.clone().or_else(|| {
            Some(PublisherIdentitySpec {
                publisher_id: "internal-default".to_string(),
                display_name: verified_vi.company_name.clone(),
                organization: verified_vi.company_name.clone(),
                website: None,
                support_contact: None,
                copyright: verified_vi.legal_copyright.clone(),
                ed25519_key_id: key_meta.key_id.clone(),
                ed25519_pubkey_hex: key_meta.public_key_hex.clone(),
                authenticode_cert_ref: options.authenticode_cert_ref.clone(),
                release_channel: match options.signing_mode {
                    SigningMode::PublicSigned => "stable",
                    SigningMode::SelfSignedQa => "qa",
                    SigningMode::UnsignedInternal => "internal",
                }
                .to_string(),
                signing_policy: options.signing_mode,
            })
        });

        let signing_meta = ReleaseSigningMetadata {
            signing_mode: options.signing_mode,
            publisher_id: effective_publisher
                .as_ref()
                .map(|p| p.publisher_id.clone())
                .unwrap_or_else(|| "internal-default".to_string()),
            publisher_organization: verified_vi.company_name.clone(),
            release_channel: effective_publisher
                .as_ref()
                .map(|p| p.release_channel.clone())
                .unwrap_or_else(|| "internal".to_string()),
            authenticode_trust_status: match options.signing_mode {
                SigningMode::PublicSigned => AuthenticodeTrustStatus::TrustedPublicChain,
                SigningMode::SelfSignedQa => AuthenticodeTrustStatus::SelfSignedUntrustedRoot,
                SigningMode::UnsignedInternal => AuthenticodeTrustStatus::Unsigned,
            },
            trusted_public_release: options.signing_mode == SigningMode::PublicSigned,
            signer_thumbprint: options.authenticode_cert_ref.clone(),
            signer_subject: None,
            version_info: verified_vi.clone(),
            key_rotation: options.key_rotation_delegation.clone(),
        };

        let mut export_compat = project.compatibility.clone();
        if export_compat.sqlite_schema_version < CURRENT_SQLITE_SCHEMA_VERSION {
            export_compat.sqlite_schema_version = CURRENT_SQLITE_SCHEMA_VERSION;
        }
        let release_manifest = ReleaseBundleManifest {
            compatibility: export_compat,
            project: project.clone(),
            module_states: module_states.clone(),
            workflows_count: validated_workflows.len(),
            templates_count: project.template_files.len(),
            built_at_utc: chrono::Utc::now().to_rfc3339(),
            publisher: effective_publisher.clone(),
            signing: Some(signing_meta.clone()),
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

        let mut notice_items: Vec<String> = vec![
            "FarmerPro Mini Runtime — MIT OR Apache-2.0".to_string(),
            "Microsoft Edge WebView2 Loader / Evergreen Runtime — Redistributable Notice (requires Evergreen Runtime or Fixed Version on offline Windows targets)".to_string(),
        ];

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
                    notice_items.push(
                        "Android Debug Bridge (adb.exe, AdbWinApi.dll, AdbWinUsbApi.dll) — Apache License 2.0 (Android SDK Platform-Tools)"
                            .to_string(),
                    );
                }
            }
        }

        // Bundle standalone fp-updater.exe alongside portable Mini Tool if present next to runtime template
        let updater_candidates = [
            runtime_template_exe.with_file_name("fp-updater.exe"),
            runtime_template_exe
                .parent()
                .map(|p| p.join("resources").join("fp-updater.exe"))
                .unwrap_or_default(),
            PathBuf::from("target/release/fp-updater.exe"),
            PathBuf::from("target/debug/fp-updater.exe"),
        ];
        if let Some(up_src) = updater_candidates.into_iter().find(|p| p.is_file()) {
            let up_dir = output_release_dir.join("updater");
            std::fs::create_dir_all(&up_dir)?;
            std::fs::copy(&up_src, up_dir.join("fp-updater.exe"))?;
        }

        std::fs::write(
            output_release_dir.join("THIRD_PARTY_NOTICES.txt"),
            format!(
                "FARMERPRO PORTABLE RELEASE — THIRD PARTY & LICENSE NOTICES\n\
                 ==========================================================\n\
                 {}\n\n\
                 NOTE: Windows SDK signtool.exe is NOT redistributed in FarmerPro installers or Mini Tool packages.\n",
                notice_items.join("\n")
            ),
        )?;

        steps.push(BuildStepMetric {
            step_no: 6,
            name: "Package Bundle Assets, Notices & Standalone Updater/ADB".to_string(),
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

        // STEP 8: Sign PE Executable Authenticode LAST (NEVER modify PE resources after signing!),
        // verify Win32 WinVerifyTrust + PE RT_RCDATA #7777 Ed25519 RoT + compute FINAL EXE SHA-256,
        // emit release-manifest.json & execute Final Post-Sign Distributed Verification!
        let t8 = Instant::now();
        let effective_cert_ref = options.authenticode_cert_ref.clone().or_else(|| {
            effective_publisher
                .as_ref()
                .and_then(|p| p.authenticode_cert_ref.clone())
        });
        let auth_rep = sign_pe_authenticode_and_verify(
            &target_exe_path,
            &AuthenticodeSignRequest {
                signing_mode: options.signing_mode,
                cert_ref: effective_cert_ref,
                description: Some(verified_vi.file_description.clone()),
                timestamp_url: options.timestamp_url.clone(),
            },
        )?;
        let pe_root_pubkey = read_pe_embedded_pubkey_from_exe(&target_exe_path)?;
        envelope.verify_ed25519_with_public_key(&bundle_dir, &pe_root_pubkey)?;
        let dep_audit = audit_portable_environment(output_release_dir, &project.slug);
        let probe_data_dir = output_release_dir.join("data");
        if probe_data_dir.is_dir() {
            let _ = std::fs::remove_dir(&probe_data_dir);
        }
        let exe_bytes = std::fs::read(&target_exe_path)?;
        let exe_sha256 = sha256_hex(&exe_bytes);

        let mut final_signing_meta = signing_meta;
        final_signing_meta.authenticode_trust_status = auth_rep.trust_status;
        final_signing_meta.trusted_public_release = auth_rep.trusted_public_release;
        final_signing_meta.signer_thumbprint = auth_rep.signer_thumbprint.clone();
        final_signing_meta.signer_subject = auth_rep.signer_subject.clone();

        let distributed_files = collect_distributed_files(output_release_dir)?;
        let pkg_manifest = ReleasePackageManifest {
            package_schema_version: 1,
            package_type: "mini_tool_portable_win_x64".to_string(),
            project_slug: project.slug.clone(),
            product_name: verified_vi.product_name.clone(),
            version: project.version.clone(),
            exe_name: project.exe_name.clone(),
            final_exe_sha256: exe_sha256.clone(),
            built_at_utc: chrono::Utc::now().to_rfc3339(),
            requirements: PortableRuntimeRequirements {
                requires_rust: false,
                requires_cargo: false,
                requires_node: false,
                requires_python: false,
                requires_windows_sdk: false,
                webview2_strategy: dep_audit.webview2_version_or_note.clone(),
                offline_ready: dep_audit.offline_execution_ready,
                requires_internet_if_webview2_missing: dep_audit.requires_internet_if_webview2_missing,
                data_isolation_policy: "External data/ next to install root OR %LOCALAPPDATA%\\FarmerPro\\MiniTools\\<slug> fallback; never inside versions/v*".to_string(),
            },
            publisher: effective_publisher.clone(),
            signing: Some(final_signing_meta),
            distributed_files,
            third_party_notices: notice_items,
        };
        std::fs::write(
            output_release_dir.join("release-manifest.json"),
            serde_json::to_vec_pretty(&pkg_manifest)?,
        )?;
        let final_verify = verify_final_distributed_release(
            output_release_dir,
            options.signing_mode == SigningMode::PublicSigned,
            TimestampVerificationPolicy::AllowOfflineUnsignedTimestamp,
        )?;
        if !final_verify.verified {
            bail!(
                "FINAL_DISTRIBUTED_RELEASE_VERIFY_FAILED: {}",
                final_verify.issues.join("; ")
            );
        }

        let auth_status_label = match auth_rep.trust_status {
            AuthenticodeTrustStatus::TrustedPublicChain => format!(
                "Authenticode TrustedPublicChain (Subject='{}', Thumbprint={})",
                auth_rep.signer_subject.as_deref().unwrap_or("?"),
                auth_rep.signer_thumbprint.as_deref().unwrap_or("?")
            ),
            AuthenticodeTrustStatus::SelfSignedUntrustedRoot => format!(
                "Authenticode Self-Signed QA Build (NOT TrustedPublicRelease; Subject='{}', Thumbprint={})",
                auth_rep.signer_subject.as_deref().unwrap_or("?"),
                auth_rep.signer_thumbprint.as_deref().unwrap_or("?")
            ),
            AuthenticodeTrustStatus::Unsigned => {
                "Unsigned Internal Build (No Authenticode; Ed25519 PE RT_RCDATA #7777 Verified)".to_string()
            }
            other => format!("Authenticode {:?}: {}", other, auth_rep.detail),
        };

        steps.push(BuildStepMetric {
            step_no: 8,
            name: "Authenticode Sign/Verify (WinVerifyTrust) & Final Release Manifest Check"
                .to_string(),
            duration_ms: t8.elapsed().as_millis() as u64,
            status: "PASS".to_string(),
            detail: format!(
                "Mode={:?}, Trust={:?}, FilesChecked={}, PubCertMatch={}, exe SHA-256={}",
                options.signing_mode,
                auth_rep.trust_status,
                final_verify.distributed_files_checked,
                final_verify.publisher_identity_matches_cert,
                &exe_sha256[..16],
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
            authenticode_status: auth_status_label,
            signing_mode: options.signing_mode,
            trusted_public_release: auth_rep.trusted_public_release,
            version_info: Some(verified_vi),
            authenticode_verification: Some(auth_rep),
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

    /// Runs non-destructive preflight validation for ANY exported `GameProject` workspace before packaging.
    /// Zero game-specific hardcoding: verifies project manifest, UI schema, workflows, assets, runtime binary,
    /// ADB staging prerequisites, module states, and producer keystore readiness.
    pub fn preflight_validate_project(
        project_dir: &Path,
        runtime_template_exe: &Path,
        workspace_bin_dir: Option<&Path>,
        custom_keystore_dir: Option<&Path>,
    ) -> Result<BuildPreflightReport> {
        let mut issues = Vec::new();
        let mut checks = Vec::new();

        // 1. Project manifest
        let t1 = Instant::now();
        let proj_bytes = std::fs::read(project_dir.join("project.json"))
            .context("Missing project.json in project directory")?;
        let project: GameProjectManifest = serde_json::from_slice(&proj_bytes)?;
        checks.push(BuildStepMetric {
            step_no: 1,
            name: "Preflight Project Manifest".to_string(),
            duration_ms: t1.elapsed().as_millis() as u64,
            status: "PASS".to_string(),
            detail: format!("{} ({}) v{}", project.name, project.slug, project.version),
        });

        // 2. UI Schema
        let t2 = Instant::now();
        let ui_bytes = std::fs::read(project_dir.join("ui-schema.json"))
            .context("Missing ui-schema.json in project directory")?;
        let ui_schema: MiniToolUiSchema = serde_json::from_slice(&ui_bytes)?;
        let ui_ok = ui_schema.project_slug == project.slug;
        if !ui_ok {
            issues.push(format!(
                "UI schema projectSlug '{}' does not match '{}'",
                ui_schema.project_slug, project.slug
            ));
        }
        checks.push(BuildStepMetric {
            step_no: 2,
            name: "Preflight UI Schema".to_string(),
            duration_ms: t2.elapsed().as_millis() as u64,
            status: if ui_ok { "PASS" } else { "FAIL" }.to_string(),
            detail: format!(
                "schema v{}, tasks={}",
                ui_schema.schema_version,
                ui_schema.tasks.len()
            ),
        });

        // 3. Workflows
        let t3 = Instant::now();
        let mut wf_ok = !project.workflow_files.is_empty();
        if project.workflow_files.is_empty() {
            issues.push("Project declares 0 workflow files".to_string());
        }
        for wf_rel in &project.workflow_files {
            match validate_safe_relative_path(project_dir, wf_rel)
                .and_then(|p| Ok(std::fs::read_to_string(p)?))
                .and_then(|s| WorkflowValidator::parse_and_validate(&s))
            {
                Ok(_) => {}
                Err(err) => {
                    wf_ok = false;
                    issues.push(format!("Workflow '{}' invalid: {}", wf_rel, err));
                }
            }
        }
        checks.push(BuildStepMetric {
            step_no: 3,
            name: "Preflight Workflows".to_string(),
            duration_ms: t3.elapsed().as_millis() as u64,
            status: if wf_ok { "PASS" } else { "FAIL" }.to_string(),
            detail: format!("{} workflows checked", project.workflow_files.len()),
        });

        // 4. Assets (Templates & Icon)
        let t4 = Instant::now();
        let mut tpl_ok = true;
        for tpl_rel in &project.template_files {
            match validate_safe_relative_path(project_dir, tpl_rel) {
                Ok(p) if p.exists() => {}
                Ok(p) => {
                    tpl_ok = false;
                    issues.push(format!("Missing template asset: {}", p.display()));
                }
                Err(err) => {
                    tpl_ok = false;
                    issues.push(format!("Invalid template path '{}': {}", tpl_rel, err));
                }
            }
        }
        let icon_present = match validate_safe_relative_path(project_dir, &project.icon_rel_path) {
            Ok(p) => p.exists(),
            Err(err) => {
                issues.push(format!(
                    "Invalid icon path '{}': {}",
                    project.icon_rel_path, err
                ));
                false
            }
        };
        if !icon_present {
            issues.push(format!("Missing project icon '{}'", project.icon_rel_path));
        }
        checks.push(BuildStepMetric {
            step_no: 4,
            name: "Preflight Templates & Branding Icon".to_string(),
            duration_ms: t4.elapsed().as_millis() as u64,
            status: if tpl_ok && icon_present {
                "PASS"
            } else {
                "FAIL"
            }
            .to_string(),
            detail: format!(
                "{} templates, icon_present={}",
                project.template_files.len(),
                icon_present
            ),
        });

        // 5. Runtime Template EXE & ADB Staging & Keystore
        let t5 = Instant::now();
        let runtime_exe_present = runtime_template_exe.exists() && runtime_template_exe.is_file();
        if !runtime_exe_present {
            issues.push(format!(
                "Runtime template binary not found at {}",
                runtime_template_exe.display()
            ));
        }

        let adb_ready = if !project.bundle_adb {
            true
        } else {
            workspace_bin_dir
                .map(|dir| dir.join("adb.exe").exists())
                .unwrap_or(false)
        };
        if !adb_ready {
            issues.push(
                "bundle_adb=true but bin/adb.exe is missing from workspace_bin_dir".to_string(),
            );
        }

        let ks_dir = custom_keystore_dir
            .map(|p| p.to_path_buf())
            .unwrap_or_else(ProducerKeyManager::default_external_keystore_dir);
        let keystore_key_id = match ProducerKeyManager::load_or_create_signing_key(&ks_dir) {
            Ok((_sk, meta)) => Some(meta.key_id),
            Err(err) => {
                issues.push(format!("Producer keystore unavailable: {}", err));
                None
            }
        };

        checks.push(BuildStepMetric {
            step_no: 5,
            name: "Preflight Runtime Binary, ADB & Signing Keystore".to_string(),
            duration_ms: t5.elapsed().as_millis() as u64,
            status: if runtime_exe_present && adb_ready && keystore_key_id.is_some() {
                "PASS"
            } else {
                "FAIL"
            }
            .to_string(),
            detail: format!(
                "runtime_exe={}, adb_ready={}, key_id={}",
                runtime_exe_present,
                adb_ready,
                keystore_key_id.as_deref().unwrap_or("none")
            ),
        });

        let registry = ModuleRegistry::resolve_for_release(
            &project.packaged_modules,
            &project.packaged_modules,
        );
        let ready = issues.is_empty();

        Ok(BuildPreflightReport {
            ready,
            project_id: project.project_id,
            project_slug: project.slug,
            project_name: project.name,
            exe_name: project.exe_name,
            workflow_count: project.workflow_files.len(),
            template_count: project.template_files.len(),
            runtime_exe_present,
            icon_present,
            adb_ready,
            keystore_key_id,
            issues,
            checks,
            modules: registry.reports().to_vec(),
        })
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

fn collect_distributed_files(release_dir: &Path) -> Result<Vec<DistributedFileEntry>> {
    fn walk(root: &Path, cur: &Path, out: &mut Vec<DistributedFileEntry>) -> Result<()> {
        for ent in std::fs::read_dir(cur)? {
            let ent = ent?;
            let p = ent.path();
            let name = ent.file_name().to_string_lossy().to_string();
            if p.is_dir() {
                if name == "data" || name == ".updater" {
                    continue;
                }
                walk(root, &p, out)?;
            } else if p.is_file() {
                let rel = p
                    .strip_prefix(root)
                    .unwrap_or(&p)
                    .to_string_lossy()
                    .replace('\\', "/");
                if rel == "release-manifest.json" || rel == "build-report.json" {
                    continue;
                }
                let bytes = std::fs::read(&p)?;
                out.push(DistributedFileEntry {
                    rel_path: rel,
                    size_bytes: bytes.len() as u64,
                    sha256: sha256_hex(&bytes),
                });
            }
        }
        Ok(())
    }
    let mut entries = Vec::new();
    walk(release_dir, release_dir, &mut entries)?;
    entries.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    Ok(entries)
}

/// Verifies a final exported portable Mini Tool directory AFTER all PE resource injections & Authenticode signing:
/// checks Authenticode PE signature, Ed25519 bundle signature, RT_RCDATA #7777 pubkey pinning,
/// PE VERSIONINFO readability, Publisher-vs-Certificate identity correspondence, and distributed file SHA-256 hashes.
pub fn verify_final_distributed_release(
    release_dir: &Path,
    require_public_chain: bool,
    timestamp_policy: TimestampVerificationPolicy,
) -> Result<FinalReleaseVerificationReport> {
    let mut issues = Vec::new();
    let pkg_bytes = std::fs::read(release_dir.join("release-manifest.json"))
        .context("FINAL_VERIFY_ERROR: missing release-manifest.json")?;
    let pkg: ReleasePackageManifest = serde_json::from_slice(&pkg_bytes)?;

    let exe_path = release_dir.join(&pkg.exe_name);
    if !exe_path.is_file() {
        bail!(
            "FINAL_VERIFY_ERROR: missing target exe '{}'",
            exe_path.display()
        );
    }
    let final_exe_sha = sha256_hex(&std::fs::read(&exe_path)?);
    if !final_exe_sha.eq_ignore_ascii_case(&pkg.final_exe_sha256) {
        issues.push(format!(
            "EXE_SHA256_MISMATCH: disk '{}' != manifest '{}'",
            final_exe_sha, pkg.final_exe_sha256
        ));
    }

    let auth_rep = verify_authenticode_pe(&exe_path);
    match auth_rep.trust_status {
        AuthenticodeTrustStatus::TamperedOrInvalid => {
            issues.push(format!(
                "AUTHENTICODE_TAMPERED_OR_INVALID: 0x{:08X} {}",
                auth_rep.win_trust_error_code, auth_rep.detail
            ));
        }
        AuthenticodeTrustStatus::ExpiredOrTimestampError => {
            issues.push(format!(
                "AUTHENTICODE_EXPIRED_OR_TIMESTAMP: 0x{:08X} {}",
                auth_rep.win_trust_error_code, auth_rep.detail
            ));
        }
        _ => {}
    }
    if require_public_chain && !auth_rep.trusted_public_release {
        issues.push(format!(
            "PUBLIC_CHAIN_REQUIRED: Authenticode status is {:?}, expected TrustedPublicChain",
            auth_rep.trust_status
        ));
    }
    if timestamp_policy == TimestampVerificationPolicy::RequireTimestampWhenPublicSigned
        && require_public_chain
        && auth_rep
            .timestamp_status
            .to_ascii_lowercase()
            .contains("missing")
    {
        issues.push(
            "AUTHENTICODE_TIMESTAMP_REQUIRED: missing counter-signature timestamp".to_string(),
        );
    }

    let bundle_dir = release_dir.join("bundle");
    let pe_pub = read_pe_embedded_pubkey_from_exe(&exe_path)?;
    let sig_bytes = std::fs::read(bundle_dir.join("signature.fpbundle.json"))?;
    let env: SignedBundleEnvelope = serde_json::from_slice(&sig_bytes)?;
    let ed_ok = env
        .verify_ed25519_with_public_key(&bundle_dir, &pe_pub)
        .is_ok();
    if !ed_ok {
        issues.push("ED25519_BUNDLE_VERIFY_FAILED".to_string());
    }
    let disk_pub =
        std::fs::read_to_string(bundle_dir.join("trusted_publisher.pub")).unwrap_or_default();
    let pe_match = disk_pub.trim().eq_ignore_ascii_case(&pe_pub);
    if !pe_match {
        issues.push("PE_RCDATA_PUBKEY_MISMATCH".to_string());
    }

    let vi_ok = read_pe_version_info_from_exe(&exe_path).is_ok();
    if !vi_ok {
        issues.push("PE_VERSIONINFO_READ_FAILED".to_string());
    }

    let (pub_cert_match, pub_cert_detail) = verify_publisher_matches_cert_subject(
        pkg.publisher.as_ref(),
        auth_rep.signer_subject.as_deref(),
    );

    let mut file_mismatches = Vec::new();
    for entry in &pkg.distributed_files {
        let p = release_dir.join(&entry.rel_path);
        match std::fs::read(&p) {
            Ok(bytes) => {
                let actual = sha256_hex(&bytes);
                if actual != entry.sha256 {
                    file_mismatches.push(format!(
                        "{}: expected {}, got {}",
                        entry.rel_path, entry.sha256, actual
                    ));
                }
            }
            Err(e) => {
                file_mismatches.push(format!("{}: missing or unreadable ({})", entry.rel_path, e));
            }
        }
    }
    if !file_mismatches.is_empty() {
        issues.push(format!(
            "{} distributed file hash mismatch(es)",
            file_mismatches.len()
        ));
    }

    Ok(FinalReleaseVerificationReport {
        verified: issues.is_empty(),
        release_dir: release_dir.to_string_lossy().to_string(),
        exe_name: pkg.exe_name,
        final_exe_sha256: final_exe_sha,
        authenticode_checked: true,
        authenticode_trust_status: auth_rep.trust_status,
        authenticode_trusted_public_chain: auth_rep.trusted_public_release,
        ed25519_bundle_verified: ed_ok,
        pe_root_pubkey_matches: pe_match,
        pe_version_info_valid: vi_ok,
        publisher_identity_matches_cert: pub_cert_match,
        publisher_identity_detail: pub_cert_detail,
        distributed_files_checked: pkg.distributed_files.len(),
        distributed_file_mismatches: file_mismatches,
        issues,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ed25519_signing_key_rotation_and_tamper_scenarios() {
        let temp_dir =
            std::env::temp_dir().join(format!("fp_ed25519_test_{}", uuid::Uuid::new_v4()));
        let keystore_dir = temp_dir.join("keystore");
        let bundle_dir = temp_dir.join("bundle");
        std::fs::create_dir_all(&bundle_dir).unwrap();

        let (sk1, meta1) = ProducerKeyManager::load_or_create_signing_key(&keystore_dir).unwrap();

        let manifest_bytes = b"{\"project\":\"jx2\"}";
        let ui_bytes = b"{\"title\":\"Original Title\"}";
        std::fs::write(bundle_dir.join("manifest.fpbundle.json"), manifest_bytes).unwrap();
        std::fs::write(bundle_dir.join("ui-schema.json"), ui_bytes).unwrap();

        let mut checksums = BTreeMap::new();
        checksums.insert(
            "manifest.fpbundle.json".to_string(),
            sha256_hex(manifest_bytes),
        );
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
            .assert_capability_granted(
                "mod.account_vault",
                &ModuleCapability::CredentialVaultAccess
            )
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
        let disk_pubkey =
            std::fs::read_to_string(bundle_dir.join("trusted_publisher.pub")).unwrap();
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
        let env1 =
            ProducerKeyManager::sign_bundle_ed25519(&sk1, &meta1.key_id, sha256_hex(m1), cs1);

        // 2. Producer rotates key -> Key2 becomes active, Key1 is archived
        let (sk2, meta2) =
            ProducerKeyManager::rotate_producer_key(&keystore_dir, "scheduled_rotation").unwrap();
        assert_ne!(meta1.key_id, meta2.key_id);
        inject_ed25519_pubkey_into_pe_exe(&new_minitool_exe, &meta2.public_key_hex).unwrap();

        let m2 = b"{\"version\":\"2.0.0\"}";
        std::fs::write(new_bundle_dir.join("manifest.fpbundle.json"), m2).unwrap();
        let mut cs2 = BTreeMap::new();
        cs2.insert("manifest.fpbundle.json".to_string(), sha256_hex(m2));
        let env2 =
            ProducerKeyManager::sign_bundle_ed25519(&sk2, &meta2.key_id, sha256_hex(m2), cs2);

        // 3. Old Mini Tool v1 STILL verifies its old bundle using its embedded Key1 RT_RCDATA #7777!
        let old_pe_pub = read_pe_embedded_pubkey_from_exe(&old_minitool_exe).unwrap();
        assert_eq!(old_pe_pub, meta1.public_key_hex);
        assert!(env1
            .verify_ed25519_with_public_key(&old_bundle_dir, &old_pe_pub)
            .is_ok());

        // 4. New Mini Tool v2 verifies new bundle using Key2, and rejects Key1 bundle (cross-version mismatch)
        let new_pe_pub = read_pe_embedded_pubkey_from_exe(&new_minitool_exe).unwrap();
        assert_eq!(new_pe_pub, meta2.public_key_hex);
        assert!(env2
            .verify_ed25519_with_public_key(&new_bundle_dir, &new_pe_pub)
            .is_ok());
        assert!(env1
            .verify_ed25519_with_public_key(&old_bundle_dir, &new_pe_pub)
            .is_err());

        // 5. Export V3 Argon2id+AES-256-GCM recovery bundle before rotation OR import restored Key1 after rotation:
        // Simulate restoring archived Key1 via V3 portable recovery from a second producer workstation
        let ks_workstation2 = temp_dir.join("keystore_ws2");
        // First create Key1 backup on ks_ws2 using archive or export before rotate on ks_orig; here export from a keystore holding Key2,
        // wait: export Key2 V3 before rotating again, rotate to Key3, then restore Key2 via V3 recovery and verify v2 Mini Tool accepts re-signed bundle!
        let recovery_v3_path = temp_dir.join("key2_recovery_v3.json");
        let bundle_v3 = ProducerKeyManager::export_portable_recovery_bundle(
            &keystore_dir,
            &recovery_v3_path,
            "Builder-V3-Restore-Check-2026!",
        )
        .unwrap();
        assert_eq!(bundle_v3.protection_scheme, "ARGON2ID_AES_256_GCM_V3");

        let (_sk3, meta3) =
            ProducerKeyManager::rotate_producer_key(&keystore_dir, "rotate_before_v3_restore")
                .unwrap();
        assert_ne!(meta3.key_id, meta2.key_id);

        let restored_meta2 = ProducerKeyManager::import_portable_recovery_bundle(
            &recovery_v3_path,
            &ks_workstation2,
            "Builder-V3-Restore-Check-2026!",
        )
        .unwrap();
        assert_eq!(restored_meta2.key_id, meta2.key_id);
        let (sk2_restored, _) =
            ProducerKeyManager::load_or_create_signing_key(&ks_workstation2).unwrap();
        let env2_resigned = ProducerKeyManager::sign_bundle_ed25519(
            &sk2_restored,
            &restored_meta2.key_id,
            sha256_hex(m2),
            {
                let mut m = BTreeMap::new();
                m.insert("manifest.fpbundle.json".to_string(), sha256_hex(m2));
                m
            },
        );
        // v2 Mini Tool continues verifying bundle signed by restored Key2, while v1 Mini Tool still verifies old Key1 bundle!
        assert!(env2_resigned
            .verify_ed25519_with_public_key(&new_bundle_dir, &new_pe_pub)
            .is_ok());
        assert!(env1
            .verify_ed25519_with_public_key(&old_bundle_dir, &old_pe_pub)
            .is_ok());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_preflight_validate_generic_project_detects_missing_and_valid_assets() {
        let tmp = std::env::temp_dir().join(format!("fp_preflight_{}", uuid::Uuid::new_v4()));
        let proj_dir = tmp.join("generic-rpg");
        std::fs::create_dir_all(proj_dir.join("workflows")).unwrap();
        std::fs::create_dir_all(proj_dir.join("templates")).unwrap();
        std::fs::create_dir_all(proj_dir.join("assets")).unwrap();

        let runtime_exe = std::env::current_exe().unwrap();
        let bin_dir = tmp.join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        std::fs::write(bin_dir.join("adb.exe"), b"MZ-adb").unwrap();
        let ks_dir = tmp.join("keystore");

        let manifest = GameProjectManifest {
            compatibility: CompatibilityVersions::default(),
            project_id: "proj-generic-rpg".to_string(),
            slug: "generic-rpg".to_string(),
            name: "Generic RPG Auto".to_string(),
            game_package_id: "com.example.genericrpg".to_string(),
            version: "2.1.0".to_string(),
            exe_name: "Auto_Generic_RPG.exe".to_string(),
            window_title: "Generic RPG Auto".to_string(),
            icon_rel_path: "assets/app.ico".to_string(),
            target_resolution: (1280, 720),
            workflow_files: vec!["workflows/daily.fpwf.json".to_string()],
            template_files: vec!["templates/collect.png".to_string()],
            packaged_modules: vec!["mod.adb_core".to_string(), "mod.vision_ocr".to_string()],
            bundle_adb: true,
        };
        std::fs::write(
            proj_dir.join("project.json"),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let ui = MiniToolUiSchema {
            schema_version: 1,
            project_slug: manifest.slug.clone(),
            window_title: manifest.window_title.clone(),
            version_badge: "v2.1.0".to_string(),
            theme_accent_hex: "#22C55E".to_string(),
            header_bg_hex: "#141C28".to_string(),
            layout_preset: MiniLayoutPreset::Compact2Col,
            default_window_size: (740, 660),
            sections_order: vec!["device_table".to_string()],
            hidden_components: vec![],
            device_columns: vec![],
            task_categories: vec![TaskCategoryDef {
                id: "daily".to_string(),
                short_tag: "DL".to_string(),
                display_name: "Daily".to_string(),
                color_hex: "#22C55E".to_string(),
            }],
            tasks: vec![],
            utilities: vec![],
            initial_runtime_vars: HashMap::new(),
        };
        std::fs::write(
            proj_dir.join("ui-schema.json"),
            serde_json::to_vec_pretty(&ui).unwrap(),
        )
        .unwrap();

        let wf = r#"{"schemaVersion":1,"workflowId":"wf-daily","name":"Daily","status":"published","graph":{"nodes":[{"id":"n1","type":"flow.start","position":{"x":0,"y":0}},{"id":"n2","type":"flow.end","position":{"x":100,"y":0}}],"edges":[{"id":"e1","source":"n1","sourcePort":"flow","target":"n2","targetPort":"flow"}]}}"#;
        std::fs::write(proj_dir.join("workflows/daily.fpwf.json"), wf).unwrap();

        // Missing icon + template -> ready == false
        let bad = MiniToolPackager::preflight_validate_project(
            &proj_dir,
            &runtime_exe,
            Some(&bin_dir),
            Some(&ks_dir),
        )
        .unwrap();
        assert!(!bad.ready);
        assert!(!bad.issues.is_empty());

        // Populate icon & template -> ready == true
        std::fs::write(proj_dir.join("templates/collect.png"), b"png-data").unwrap();
        std::fs::write(proj_dir.join("assets/app.ico"), b"ico-data").unwrap();

        let ok = MiniToolPackager::preflight_validate_project(
            &proj_dir,
            &runtime_exe,
            Some(&bin_dir),
            Some(&ks_dir),
        )
        .unwrap();
        assert!(ok.ready, "Issues: {:?}", ok.issues);
        assert_eq!(ok.exe_name, "Auto_Generic_RPG.exe");
        assert!(ok.keystore_key_id.is_some());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_p5_3_pe_version_info_readback_authenticode_self_signed_qa_and_public_signed_gate() {
        let tmp = std::env::temp_dir().join(format!("fp_p53_builder_{}", uuid::Uuid::new_v4()));
        let proj_dir = tmp.join("p53-brand");
        std::fs::create_dir_all(proj_dir.join("workflows")).unwrap();
        std::fs::create_dir_all(proj_dir.join("templates")).unwrap();
        std::fs::create_dir_all(proj_dir.join("branding")).unwrap();

        let runtime_exe = std::env::current_exe().unwrap();
        let ks_dir = tmp.join("keystore");
        let manifest = GameProjectManifest {
            compatibility: CompatibilityVersions::default(),
            project_id: "proj-p53-brand".to_string(),
            slug: "p53-brand".to_string(),
            name: "P53 Branded Auto".to_string(),
            game_package_id: "com.example.p53".to_string(),
            version: "2.3.4".to_string(),
            exe_name: "Auto_P53_Brand.exe".to_string(),
            window_title: "P53 Branded Auto".to_string(),
            icon_rel_path: "branding/icon.ico".to_string(),
            target_resolution: (1280, 720),
            workflow_files: vec!["workflows/main.fpwf.json".to_string()],
            template_files: vec![],
            packaged_modules: vec!["mod.adb_core".to_string()],
            bundle_adb: false,
        };
        std::fs::write(
            proj_dir.join("project.json"),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        let ui = MiniToolUiSchema {
            schema_version: 1,
            project_slug: manifest.slug.clone(),
            window_title: manifest.window_title.clone(),
            version_badge: "v2.3.4".to_string(),
            theme_accent_hex: "#1878F5".to_string(),
            header_bg_hex: "#121E2A".to_string(),
            layout_preset: MiniLayoutPreset::Compact1Col,
            default_window_size: (680, 480),
            sections_order: vec!["header_bar".to_string()],
            hidden_components: vec![],
            device_columns: vec![],
            task_categories: vec![],
            tasks: vec![],
            utilities: vec![],
            initial_runtime_vars: HashMap::new(),
        };
        std::fs::write(
            proj_dir.join("ui-schema.json"),
            serde_json::to_vec_pretty(&ui).unwrap(),
        )
        .unwrap();
        let wf = r#"{"schemaVersion":1,"workflowId":"wf-main","name":"Main","status":"published","graph":{"nodes":[{"id":"n1","type":"flow.start","position":{"x":0,"y":0}},{"id":"n2","type":"flow.end","position":{"x":100,"y":0}}],"edges":[{"id":"e1","source":"n1","sourcePort":"flow","target":"n2","targetPort":"flow"}]}}"#;
        std::fs::write(proj_dir.join("workflows/main.fpwf.json"), wf).unwrap();

        // Build minimal 32x32 ICO
        let mut ico = vec![0, 0, 1, 0, 1, 0, 32, 32, 0, 0, 1, 0, 32, 0];
        let bmp_size = (40 + 32 * 32 * 4 + 32 * 4) as u32;
        ico.extend_from_slice(&bmp_size.to_le_bytes());
        ico.extend_from_slice(&22u32.to_le_bytes());
        ico.extend_from_slice(&40u32.to_le_bytes());
        ico.extend_from_slice(&32i32.to_le_bytes());
        ico.extend_from_slice(&64i32.to_le_bytes());
        ico.extend_from_slice(&1u16.to_le_bytes());
        ico.extend_from_slice(&32u16.to_le_bytes());
        ico.resize(ico.len() + 24 + 32 * 32 * 4 + 32 * 4, 0u8);
        std::fs::write(proj_dir.join("branding/icon.ico"), &ico).unwrap();

        let ver_spec = PeVersionInfoSpec {
            company_name: "FarmerPro Studio QA".to_string(),
            file_description: "P53 Branded Automation Runner".to_string(),
            file_version: "2.3.4.0".to_string(),
            internal_name: "p53-brand".to_string(),
            legal_copyright: "Copyright (C) 2026 FarmerPro Studio QA".to_string(),
            original_filename: "Auto_P53_Brand.exe".to_string(),
            product_name: "P53 Branded Auto".to_string(),
            product_version: "2.3.4".to_string(),
        };

        // 1. Unsigned Internal build -> VERSIONINFO injected and readable via Win32 GetFileVersionInfoW!
        let out_unsigned = tmp.join("rel_unsigned");
        let rep_unsigned = MiniToolPackager::build_and_export_release_with_options(
            &proj_dir,
            &runtime_exe,
            None,
            &out_unsigned,
            Some(&ks_dir),
            &ReleaseBuildOptions {
                signing_mode: SigningMode::UnsignedInternal,
                version_info: Some(ver_spec.clone()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(rep_unsigned.signing_mode, SigningMode::UnsignedInternal);
        assert!(!rep_unsigned.trusted_public_release);
        let readback =
            fp_bundle_verify::read_pe_version_info_from_exe(Path::new(&rep_unsigned.tool_exe_path))
                .unwrap();
        assert_eq!(readback.company_name, ver_spec.company_name);
        assert_eq!(readback.product_name, ver_spec.product_name);
        assert_eq!(readback.file_description, ver_spec.file_description);
        assert_eq!(readback.legal_copyright, ver_spec.legal_copyright);
        assert_eq!(readback.file_version, ver_spec.file_version);

        // 2. SelfSignedQa build with Windows CNG cert + PublicSigned fail-closed gate
        if authenticode::find_signtool_exe().is_some() {
            let qa_cert =
                authenticode::create_self_signed_qa_cert("FarmerPro P53 Builder Test").unwrap();
            let out_qa = tmp.join("rel_qa");
            let rep_qa = MiniToolPackager::build_and_export_release_with_options(
                &proj_dir,
                &runtime_exe,
                None,
                &out_qa,
                Some(&ks_dir),
                &ReleaseBuildOptions {
                    signing_mode: SigningMode::SelfSignedQa,
                    authenticode_cert_ref: Some(qa_cert.thumbprint.clone()),
                    version_info: Some(ver_spec.clone()),
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(rep_qa.signing_mode, SigningMode::SelfSignedQa);
            assert!(!rep_qa.trusted_public_release);
            assert!(rep_qa.pe_embedded_pubkey_verified);
            let qa_ver_readback =
                fp_bundle_verify::read_pe_version_info_from_exe(Path::new(&rep_qa.tool_exe_path))
                    .unwrap();
            assert_eq!(qa_ver_readback.company_name, ver_spec.company_name);

            // PublicSigned MUST FAIL CLOSED on self-signed cert (cannot claim TrustedPublicRelease)
            let out_pub_fail = tmp.join("rel_pub_fail");
            let pub_err = MiniToolPackager::build_and_export_release_with_options(
                &proj_dir,
                &runtime_exe,
                None,
                &out_pub_fail,
                Some(&ks_dir),
                &ReleaseBuildOptions {
                    signing_mode: SigningMode::PublicSigned,
                    authenticode_cert_ref: Some(qa_cert.thumbprint.clone()),
                    version_info: Some(ver_spec),
                    ..Default::default()
                },
            );
            assert!(
                pub_err.is_err(),
                "PublicSigned must reject untrusted self-signed cert"
            );

            let _ = std::process::Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-Command",
                    &format!(
                        "Remove-Item 'Cert:\\CurrentUser\\My\\{}' -ErrorAction SilentlyContinue",
                        qa_cert.thumbprint
                    ),
                ])
                .output();
        }

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_p5_4_final_release_manifest_tamper_sdk_discovery_and_publisher_cert_mismatch() {
        use authenticode::discover_signing_toolchain;
        // 1. Verify Signing Toolchain Discovery fails safely when FP_SIGNTOOL_PATH points nowhere (no crash, UnsignedInternal available)
        let prev_env = std::env::var("FP_SIGNTOOL_PATH").ok();
        std::env::set_var("FP_SIGNTOOL_PATH", r"Z:\nonexistent\signtool.exe");
        let tc_status = discover_signing_toolchain();
        assert!(!tc_status.sdk_bundled_in_app);
        assert!(tc_status.unsigned_internal_supported);
        if let Some(p) = prev_env {
            std::env::set_var("FP_SIGNTOOL_PATH", p);
        } else {
            std::env::remove_var("FP_SIGNTOOL_PATH");
        }

        // 2. Verify WebView2 missing detection path cleanly
        let (wv_ok, wv_msg) = fp_bundle_contract::detect_webview2_runtime_at(
            &[Path::new(r"Z:\nonexistent\EdgeWebView\Application")],
            None,
        );
        assert!(!wv_ok);
        assert!(wv_msg.contains("WEBVIEW2_MISSING"));

        // 3. Build full portable release -> check release-manifest.json, THIRD_PARTY_NOTICES.txt, final verify, & tamper rejection!
        let tmp = std::env::temp_dir().join(format!("fp_p54_builder_{}", uuid::Uuid::new_v4()));
        let proj_dir = tmp.join("p54-portable");
        std::fs::create_dir_all(proj_dir.join("workflows")).unwrap();
        std::fs::create_dir_all(proj_dir.join("templates")).unwrap();
        std::fs::create_dir_all(proj_dir.join("branding")).unwrap();

        let runtime_exe = std::env::current_exe().unwrap();
        let bin_dir = tmp.join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        std::fs::write(bin_dir.join("adb.exe"), b"MZ-adb-test").unwrap();
        std::fs::write(bin_dir.join("AdbWinApi.dll"), b"MZ-dll1").unwrap();
        std::fs::write(bin_dir.join("AdbWinUsbApi.dll"), b"MZ-dll2").unwrap();
        let ks_dir = tmp.join("keystore");

        let manifest = GameProjectManifest {
            compatibility: CompatibilityVersions::default(),
            project_id: "proj-p54".to_string(),
            slug: "p54-portable".to_string(),
            name: "P54 Portable Auto".to_string(),
            game_package_id: "com.example.p54".to_string(),
            version: "1.4.0".to_string(),
            exe_name: "Auto_P54.exe".to_string(),
            window_title: "P54 Portable Auto".to_string(),
            icon_rel_path: "branding/icon.ico".to_string(),
            target_resolution: (1280, 720),
            workflow_files: vec!["workflows/main.fpwf.json".to_string()],
            template_files: vec![],
            packaged_modules: vec!["mod.adb_core".to_string()],
            bundle_adb: true,
        };
        std::fs::write(
            proj_dir.join("project.json"),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        let ui = MiniToolUiSchema {
            schema_version: 1,
            project_slug: manifest.slug.clone(),
            window_title: manifest.window_title.clone(),
            version_badge: "v1.4.0".to_string(),
            theme_accent_hex: "#10B981".to_string(),
            header_bg_hex: "#121E2A".to_string(),
            layout_preset: MiniLayoutPreset::Compact1Col,
            default_window_size: (680, 480),
            sections_order: vec!["header_bar".to_string()],
            hidden_components: vec![],
            device_columns: vec![],
            task_categories: vec![],
            tasks: vec![],
            utilities: vec![],
            initial_runtime_vars: HashMap::new(),
        };
        std::fs::write(
            proj_dir.join("ui-schema.json"),
            serde_json::to_vec_pretty(&ui).unwrap(),
        )
        .unwrap();
        let wf = r#"{"schemaVersion":1,"workflowId":"wf-main","name":"Main","status":"published","graph":{"nodes":[{"id":"n1","type":"flow.start","position":{"x":0,"y":0}},{"id":"n2","type":"flow.end","position":{"x":100,"y":0}}],"edges":[{"id":"e1","source":"n1","sourcePort":"flow","target":"n2","targetPort":"flow"}]}}"#;
        std::fs::write(proj_dir.join("workflows/main.fpwf.json"), wf).unwrap();

        let mut ico = vec![0, 0, 1, 0, 1, 0, 32, 32, 0, 0, 1, 0, 32, 0];
        let bmp_size = (40 + 32 * 32 * 4 + 32 * 4) as u32;
        ico.extend_from_slice(&bmp_size.to_le_bytes());
        ico.extend_from_slice(&22u32.to_le_bytes());
        ico.extend_from_slice(&40u32.to_le_bytes());
        ico.extend_from_slice(&32i32.to_le_bytes());
        ico.extend_from_slice(&64i32.to_le_bytes());
        ico.extend_from_slice(&1u16.to_le_bytes());
        ico.extend_from_slice(&32u16.to_le_bytes());
        ico.resize(ico.len() + 24 + 32 * 32 * 4 + 32 * 4, 0u8);
        std::fs::write(proj_dir.join("branding/icon.ico"), &ico).unwrap();

        let out_dir = tmp.join("portable_release");
        let rep = MiniToolPackager::build_and_export_release_with_options(
            &proj_dir,
            &runtime_exe,
            Some(&bin_dir),
            &out_dir,
            Some(&ks_dir),
            &ReleaseBuildOptions::default(),
        )
        .unwrap();
        assert!(out_dir.join("release-manifest.json").is_file());
        assert!(out_dir.join("THIRD_PARTY_NOTICES.txt").is_file());
        assert!(rep.dependency_audit.third_party_notices_present);

        let verify_ok = verify_final_distributed_release(
            &out_dir,
            false,
            TimestampVerificationPolicy::AllowOfflineUnsignedTimestamp,
        )
        .unwrap();
        assert!(verify_ok.verified, "issues: {:?}", verify_ok.issues);
        assert!(verify_ok.distributed_files_checked >= 5);
        // Unsigned build MUST explicitly state publisher identity is NOT proven by cert/VERSIONINFO alone!
        assert!(!verify_ok.publisher_identity_matches_cert);

        // Publisher-vs-Certificate subject check
        let fake_pub = PublisherIdentitySpec {
            publisher_id: "pub-alpha".to_string(),
            display_name: "Alpha Corp".to_string(),
            organization: "Alpha Automation Co., Ltd.".to_string(),
            website: None,
            support_contact: None,
            copyright: "Copyright (C) 2026 Alpha".to_string(),
            ed25519_key_id: "k1".to_string(),
            ed25519_pubkey_hex: "00".repeat(32),
            authenticode_cert_ref: None,
            release_channel: "qa".to_string(),
            signing_policy: SigningMode::SelfSignedQa,
        };
        let (mismatch, _) = verify_publisher_matches_cert_subject(
            Some(&fake_pub),
            Some("CN=Different Rogue Publisher"),
        );
        assert!(!mismatch);
        let (matched, _) = verify_publisher_matches_cert_subject(
            Some(&fake_pub),
            Some("CN=Alpha Automation Co., Ltd."),
        );
        assert!(matched);

        // Tamper 1 distributed file after packaging -> verify_final_distributed_release MUST FAIL!
        std::fs::write(out_dir.join("bin/adb.exe"), b"MZ-tampered-adb").unwrap();
        let verify_tampered = verify_final_distributed_release(
            &out_dir,
            false,
            TimestampVerificationPolicy::AllowOfflineUnsignedTimestamp,
        )
        .unwrap();
        assert!(!verify_tampered.verified);
        assert!(!verify_tampered.distributed_file_mismatches.is_empty());

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
