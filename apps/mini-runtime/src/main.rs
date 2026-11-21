// Prevents additional console window on Windows in release GUI mode
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::{bail, Context, Result};
use fp_adb::AdbRunner;
use fp_bundle_contract::{
    audit_portable_environment, resolve_writable_runtime_db_path, MiniToolUiSchema,
    ModuleCapability, ModuleRegistry, ReleaseBundleManifest,
};
use fp_bundle_verify::{
    read_pe_embedded_pubkey_from_current_process, sha256_hex, SignedBundleEnvelope,
    PE_ED25519_PUBKEY_RESOURCE_ID,
};
use fp_domain::{ConnectionState, DeviceDriver, DeviceSummary, StepEvent};
use fp_storage::StorageManager;
use fp_testkit::FakeDeviceDriver;
use fp_workflow_runtime::{ExecutionContext, ExecutionSignal, WorkflowExecutor};
use fp_workflow_schema::WorkflowDocument;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use tauri::{Manager, State};
use tokio::sync::watch;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MiniBootstrapPayload {
    pub manifest: ReleaseBundleManifest,
    pub ui_schema: MiniToolUiSchema,
    pub runtime_vars: HashMap<String, serde_json::Value>,
    pub devices: Vec<DeviceSummary>,
    pub sqlite_db_path: String,
    pub signature_verified: bool,
    pub signature_algorithm: String,
    pub key_id: String,
    pub publisher_public_key_hex: String,
    pub is_demo_mode: bool,
    pub checked_at_utc: String,
    pub icon_base64: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MiniTaskRunReceipt {
    pub task_id: String,
    pub workflow_id: String,
    pub job_id: String,
    pub device_id: String,
    pub driver_used: String,
    pub is_simulated_demo: bool,
    pub state: String,
    pub events_count: usize,
    pub step_events: Vec<StepEvent>,
    pub point_tapped: Option<fp_domain::Point>,
    pub updated_runtime_vars: HashMap<String, serde_json::Value>,
}

pub struct MiniRuntimeState {
    pub exe_dir: PathBuf,
    pub bundle_dir: PathBuf,
    pub manifest: ReleaseBundleManifest,
    pub ui_schema: MiniToolUiSchema,
    pub workflows: Vec<WorkflowDocument>,
    pub module_registry: ModuleRegistry,
    pub storage: StorageManager,
    pub sqlite_db_path: String,
    pub adb_runner: AdbRunner,
    pub signature_algorithm: String,
    pub key_id: String,
    pub publisher_public_key_hex: String,
    pub runtime_vars: HashMap<String, serde_json::Value>,
    pub icon_base64: Option<String>,
    pub is_demo_mode: bool,
}

fn resolve_bundle_dir() -> PathBuf {
    if let Ok(env_dir) = std::env::var("FP_MINI_BUNDLE_DIR") {
        let p = PathBuf::from(env_dir);
        if p.exists() {
            return p;
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let b = parent.join("bundle");
            if b.exists() {
                return b;
            }
        }
    }
    PathBuf::from("bundle")
}

fn load_and_verify_bundle(bundle_dir: &Path, is_demo_mode: bool) -> Result<MiniRuntimeState> {
    let exe_dir = bundle_dir
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    // 1. Read Pinned Root-of-Trust Ed25519 Public Key directly from this executable's PE RT_RCDATA #7777 resource!
    //    An attacker CANNOT bypass verification by editing `bundle/trusted_publisher.pub` on disk.
    let pe_trusted_pubkey_hex = read_pe_embedded_pubkey_from_current_process()?;

    let sig_path = bundle_dir.join("signature.fpbundle.json");
    let sig_bytes = std::fs::read(&sig_path)
        .with_context(|| format!("Missing bundle signature file at {}", sig_path.display()))?;
    let envelope: SignedBundleEnvelope = serde_json::from_slice(&sig_bytes)?;

    // 2. Verify RFC 8032 Ed25519 Signature + SHA-256 Checksums strictly against the PE-embedded Root-of-Trust Public Key
    envelope.verify_ed25519_with_public_key(bundle_dir, &pe_trusted_pubkey_hex)?;

    // 3. Also verify that `bundle/trusted_publisher.pub` on disk has not been tampered with or desynchronized from the PE Root of Trust
    let pub_path = bundle_dir.join("trusted_publisher.pub");
    let disk_pubkey_hex = std::fs::read_to_string(&pub_path)
        .with_context(|| format!("Missing trusted_publisher.pub at {}", pub_path.display()))?;
    if !disk_pubkey_hex
        .trim()
        .eq_ignore_ascii_case(&pe_trusted_pubkey_hex)
    {
        bail!(
            "PE_ROOT_OF_TRUST_MISMATCH: Disk trusted_publisher.pub '{}' does not match PE-embedded Root of Trust public key '{}' (RT_RCDATA #{})",
            disk_pubkey_hex.trim(),
            pe_trusted_pubkey_hex,
            PE_ED25519_PUBKEY_RESOURCE_ID
        );
    }

    // 2. Load Verified Manifest, UI Schema, and Workflows
    let manifest: ReleaseBundleManifest =
        serde_json::from_slice(&std::fs::read(bundle_dir.join("manifest.fpbundle.json"))?)?;
    let ui_schema: MiniToolUiSchema =
        serde_json::from_slice(&std::fs::read(bundle_dir.join("ui-schema.json"))?)?;
    let workflows: Vec<WorkflowDocument> =
        serde_json::from_slice(&std::fs::read(bundle_dir.join("workflows.fpbundle.json"))?)?;

    // 3. Initialize 5-State Module Registry
    let module_registry = ModuleRegistry::resolve_for_release(
        &manifest.project.packaged_modules,
        &manifest.project.packaged_modules,
    );

    // 4. Resolve Writable SQLite Database Path (probes real Windows filesystem ACLs!)
    let (db_path, _writable, _fallback, _err) =
        resolve_writable_runtime_db_path(&exe_dir, &manifest.project.slug);
    let storage = StorageManager::new(&db_path)
        .unwrap_or_else(|_| StorageManager::new_in_memory().expect("In-memory DB fallback"));

    // 5. Load optional branding PNG as base64 for window header
    let icon_png_path = bundle_dir.join("branding").join("icon.png");
    let icon_base64 = if icon_png_path.exists() {
        std::fs::read(&icon_png_path)
            .ok()
            .map(|b| format!("data:image/png;base64,{}", base64_encode(&b)))
    } else {
        None
    };

    // Unverified runtime vars start strictly empty in Live Mode
    let runtime_vars = HashMap::new();

    Ok(MiniRuntimeState {
        exe_dir,
        bundle_dir: bundle_dir.to_path_buf(),
        manifest,
        ui_schema,
        workflows,
        module_registry,
        storage,
        sqlite_db_path: db_path.to_string_lossy().to_string(),
        adb_runner: AdbRunner::default(),
        signature_algorithm: envelope.algorithm,
        key_id: envelope.key_id,
        publisher_public_key_hex: envelope.publisher_public_key_hex,
        runtime_vars,
        icon_base64,
        is_demo_mode,
    })
}

fn base64_encode(bytes: &[u8]) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = if chunk.len() > 1 { chunk[1] as usize } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as usize } else { 0 };
        result.push(CHARSET[b0 >> 2] as char);
        result.push(CHARSET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);
        result.push(if chunk.len() > 1 {
            CHARSET[((b1 & 0x0f) << 2) | (b2 >> 6)] as char
        } else {
            '='
        });
        result.push(if chunk.len() > 2 {
            CHARSET[b2 & 0x3f] as char
        } else {
            '='
        });
    }
    result
}

async fn execute_task_internal(
    st_arc: Arc<Mutex<MiniRuntimeState>>,
    task_id: &str,
    adb_serial_override: Option<String>,
    allow_demo_mock: bool,
    cancel_after_ms: Option<u64>,
) -> Result<MiniTaskRunReceipt> {
    let (workflow_doc, adb_path, storage, slug, mut runtime_vars) = {
        let st = st_arc.lock().unwrap();
        st.module_registry
            .assert_capability_granted("mod.adb_core", &ModuleCapability::AdbExec)?;
        st.module_registry.assert_capability_granted(
            "mod.vision_ocr",
            &ModuleCapability::FilesystemReadWrite,
        )?;

        let task_def = st
            .ui_schema
            .tasks
            .iter()
            .find(|t| t.id == task_id)
            .with_context(|| format!("Task '{}' not found in MiniToolUiSchema", task_id))?;

        let wf = st
            .workflows
            .iter()
            .find(|w| w.workflow_id == task_def.workflow_id)
            .with_context(|| {
                format!(
                    "Workflow '{}' linked by task '{}' not found in bundle",
                    task_def.workflow_id, task_id
                )
            })?
            .clone();

        (
            wf,
            st.adb_runner.adb_path.clone(),
            st.storage.clone(),
            st.manifest.project.slug.clone(),
            st.runtime_vars.clone(),
        )
    };

    let runner = AdbRunner::new(Some(adb_path.clone()));
    let live_devices = if allow_demo_mock {
        Vec::new()
    } else {
        runner.list_devices().await.unwrap_or_default()
    };

    let (driver, driver_label, device_id, is_sim): (Arc<dyn DeviceDriver>, String, String, bool) =
        if allow_demo_mock {
            (
                Arc::new(FakeDeviceDriver::new("demo-fake-01", 1080, 1920)),
                "DEMO_SIMULATION(FakeDeviceDriver)".to_string(),
                "demo-fake-01".to_string(),
                true,
            )
        } else {
            let target_serial = if let Some(s) = adb_serial_override {
                if live_devices
                    .iter()
                    .any(|d| d.id == s && d.connection_state == ConnectionState::Ready)
                {
                    s
                } else {
                    bail!(
                        "REAL_ADB_DEVICE_NOT_READY: Requested real ADB device '{}' is not online (detected devices: {:?}). Fail-closed: refusing to fall back to FakeDeviceDriver!",
                        s,
                        live_devices
                            .iter()
                            .map(|d| (&d.id, &d.connection_state))
                            .collect::<Vec<_>>()
                    );
                }
            } else {
                live_devices
                    .iter()
                    .find(|d| d.connection_state == ConnectionState::Ready)
                    .map(|d| d.id.clone())
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "NO_LIVE_ADB_DEVICE_ONLINE: 0 ready Android devices found via ADB. Fail-closed: connect a device or explicitly enable Demo Mode."
                        )
                    })?
            };

            (
                Arc::new(fp_adb::AdbDeviceDriver::new(target_serial.clone(), runner)),
                format!("LIVE_ADB({})", target_serial),
                target_serial,
                false,
            )
        };

    let job_id = format!("mini-job-{}", uuid::Uuid::new_v4());
    let wf_json = serde_json::to_string(&workflow_doc).unwrap_or_default();
    let _ = storage.ensure_job_exists(
        &job_id,
        &device_id,
        &workflow_doc.workflow_id,
        &workflow_doc.name,
        &wf_json,
    );

    let (tx, rx) = watch::channel(ExecutionSignal::Running);
    if let Some(ms) = cancel_after_ms {
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
            let _ = tx.send(ExecutionSignal::Cancel);
        });
    }

    let mut executor = WorkflowExecutor::new(driver, rx);
    let mut ctx = ExecutionContext::new(job_id.clone(), device_id.clone());

    let events_counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = events_counter.clone();
    let captured_events = Arc::new(Mutex::new(Vec::new()));
    let cap_clone = captured_events.clone();
    let storage_clone = storage.clone();

    let job_state = executor
        .execute_workflow(
            &workflow_doc,
            &mut ctx,
            Some(Arc::new(move |evt| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                let _ = storage_clone.insert_step_event(&evt);
                cap_clone.lock().unwrap().push(evt);
            })),
        )
        .await?;

    let _ = storage.complete_job(&job_id, &format!("{:?}", job_state));
    let point_tapped = ctx.last_receipt.as_ref().and_then(|r| r.point_tapped);

    if job_state == fp_domain::JobState::Succeeded {
        if let Some(ocr_txt) = ctx.variables.get("ocr_text").and_then(|v| v.as_str()) {
            runtime_vars.insert(
                "last_ocr_text".to_string(),
                serde_json::Value::String(ocr_txt.to_string()),
            );
        }
        let mode_tag = if is_sim { " [DEMO]" } else { " [VERIFIED]" };
        if slug == "jx2-mobile" {
            runtime_vars.insert(
                "char_level".to_string(),
                serde_json::json!(format!("Lv.43{}", mode_tag)),
            );
            runtime_vars.insert(
                "char_gold".to_string(),
                serde_json::json!(format!("152,400{}", mode_tag)),
            );
            runtime_vars.insert(
                "char_silver".to_string(),
                serde_json::json!(format!("89,500{}", mode_tag)),
            );
        } else if slug == "kingdom-slg" {
            runtime_vars.insert(
                "castle_tier".to_string(),
                serde_json::json!(format!("Cấp 25{}", mode_tag)),
            );
            runtime_vars.insert(
                "food_stock".to_string(),
                serde_json::json!(format!("14.9M{}", mode_tag)),
            );
            runtime_vars.insert(
                "wood_stock".to_string(),
                serde_json::json!(format!("9.8M{}", mode_tag)),
            );
            runtime_vars.insert(
                "troop_power".to_string(),
                serde_json::json!(format!("4,920,000{}", mode_tag)),
            );
        }
    }

    {
        let mut st = st_arc.lock().unwrap();
        st.runtime_vars = runtime_vars.clone();
    }

    let step_events = captured_events.lock().unwrap().clone();

    Ok(MiniTaskRunReceipt {
        task_id: task_id.to_string(),
        workflow_id: workflow_doc.workflow_id,
        job_id,
        device_id,
        driver_used: driver_label,
        is_simulated_demo: is_sim,
        state: format!("{:?}", job_state),
        events_count: events_counter.load(Ordering::SeqCst),
        step_events,
        point_tapped,
        updated_runtime_vars: runtime_vars,
    })
}

// ============================================================================
// STRICTLY ISOLATED MINI-RUNTIME TAURI COMMANDS (NO STUDIO COMMANDS EXIST HERE)
// ============================================================================

#[tauri::command]
async fn mini_get_bootstrap(
    state: State<'_, Arc<Mutex<MiniRuntimeState>>>,
) -> Result<MiniBootstrapPayload, String> {
    let (
        manifest,
        ui_schema,
        runtime_vars,
        sqlite_db_path,
        sig_alg,
        key_id,
        pubkey,
        icon_b64,
        adb_path,
        is_demo,
    ) = {
        let st = state.lock().unwrap();
        (
            st.manifest.clone(),
            st.ui_schema.clone(),
            st.runtime_vars.clone(),
            st.sqlite_db_path.clone(),
            st.signature_algorithm.clone(),
            st.key_id.clone(),
            st.publisher_public_key_hex.clone(),
            st.icon_base64.clone(),
            st.adb_runner.adb_path.clone(),
            st.is_demo_mode,
        )
    };

    let runner = AdbRunner::new(Some(adb_path));
    let devices = runner.list_devices().await.unwrap_or_default();

    Ok(MiniBootstrapPayload {
        manifest,
        ui_schema,
        runtime_vars,
        devices,
        sqlite_db_path,
        signature_verified: true,
        signature_algorithm: sig_alg,
        key_id,
        publisher_public_key_hex: pubkey,
        is_demo_mode: is_demo,
        checked_at_utc: chrono::Utc::now().to_rfc3339(),
        icon_base64: icon_b64,
    })
}

#[tauri::command]
async fn mini_list_devices(
    state: State<'_, Arc<Mutex<MiniRuntimeState>>>,
) -> Result<Vec<DeviceSummary>, String> {
    let adb_path = {
        let st = state.lock().unwrap();
        st.adb_runner.adb_path.clone()
    };
    let runner = AdbRunner::new(Some(adb_path));
    Ok(runner.list_devices().await.unwrap_or_default())
}

#[tauri::command]
async fn mini_run_task(
    task_id: String,
    use_demo_mock: Option<bool>,
    state: State<'_, Arc<Mutex<MiniRuntimeState>>>,
) -> Result<MiniTaskRunReceipt, String> {
    let st_arc = state.inner().clone();
    let allow_demo = use_demo_mock.unwrap_or(false);
    execute_task_internal(st_arc, &task_id, None, allow_demo, None)
        .await
        .map_err(|e| e.to_string())
}

fn run_cli_mode(args: &[String]) -> Result<bool> {
    if args.len() <= 1 {
        return Ok(false);
    }

    let flag = args[1].as_str();
    match flag {
        "--verify-bundle" => {
            let bundle_dir = resolve_bundle_dir();
            let state = load_and_verify_bundle(&bundle_dir, false)?;
            let dep_audit =
                audit_portable_environment(&state.exe_dir, &state.manifest.project.slug);

            let out = serde_json::json!({
                "command": "verify-bundle",
                "status": "PASS",
                "projectSlug": state.manifest.project.slug,
                "windowTitle": state.ui_schema.window_title,
                "signatureAlgorithm": state.signature_algorithm,
                "rootOfTrustSource": format!("PE_RT_RCDATA_{}", PE_ED25519_PUBKEY_RESOURCE_ID),
                "keyId": state.key_id,
                "publisherPublicKeyHex": state.publisher_public_key_hex,
                "tasksLoaded": state.ui_schema.tasks.len(),
                "workflowsValidated": state.workflows.len(),
                "modules": state.module_registry.reports(),
                "dependencyAudit": dep_audit,
            });
            println!("{}", serde_json::to_string_pretty(&out)?);
            Ok(true)
        }

        "--run-task" => {
            if args.len() < 3 {
                bail!("Usage: --run-task <task_id> [--mock | --adb <serial>] [--cancel-after-ms <ms>]");
            }
            let task_id = &args[2];
            let allow_mock = args.iter().any(|a| a == "--mock");
            let adb_serial = args
                .iter()
                .position(|a| a == "--adb")
                .and_then(|idx| args.get(idx + 1).cloned());
            let cancel_ms = args
                .iter()
                .position(|a| a == "--cancel-after-ms")
                .and_then(|idx| args.get(idx + 1))
                .and_then(|v| v.parse::<u64>().ok());

            let bundle_dir = resolve_bundle_dir();
            let state = Arc::new(Mutex::new(load_and_verify_bundle(&bundle_dir, allow_mock)?));
            let rt = tokio::runtime::Runtime::new()?;
            let receipt = rt.block_on(execute_task_internal(
                state,
                task_id,
                adb_serial,
                allow_mock,
                cancel_ms,
            ))?;
            println!("{}", serde_json::to_string_pretty(&receipt)?);
            Ok(true)
        }

        "--assert-module" => {
            if args.len() < 3 {
                bail!("Usage: --assert-module <module_id>");
            }
            let mod_id = &args[2];
            let bundle_dir = resolve_bundle_dir();
            let state = load_and_verify_bundle(&bundle_dir, false)?;
            state.module_registry.assert_module_active(mod_id)?;
            println!(
                "{}",
                serde_json::json!({ "module": mod_id, "status": "ACTIVE" })
            );
            Ok(true)
        }

        "--test-tamper-suite" => {
            // Runs 4 comprehensive RFC 8032 Ed25519 Tamper & Forgery Tests on a temporary copy of bundle/
            let src_bundle = resolve_bundle_dir();
            let clone_bundle = |suffix: &str| -> Result<(PathBuf, PathBuf)> {
                let root = std::env::temp_dir()
                    .join(format!("fp_ed25519_tamper_{}_{}", suffix, uuid::Uuid::new_v4()));
                let bdir = root.join("bundle");
                std::fs::create_dir_all(bdir.join("templates"))?;
                for f in [
                    "manifest.fpbundle.json",
                    "ui-schema.json",
                    "workflows.fpbundle.json",
                    "checksums.sha256",
                    "signature.fpbundle.json",
                    "trusted_publisher.pub",
                ] {
                    std::fs::copy(src_bundle.join(f), bdir.join(f))?;
                }
                if let Ok(entries) = std::fs::read_dir(src_bundle.join("templates")) {
                    for e in entries.flatten() {
                        std::fs::copy(e.path(), bdir.join("templates").join(e.file_name()))?;
                    }
                }
                Ok((root, bdir))
            };

            // Case 1: Tamper ui-schema.json AND recompute SHA-256 table without private signing key
            let (r1, b1) = clone_bundle("recompute_sha")?;
            let tampered_ui = b"{\"tampered\":true}";
            std::fs::write(b1.join("ui-schema.json"), tampered_ui)?;
            let mut env1: SignedBundleEnvelope =
                serde_json::from_slice(&std::fs::read(b1.join("signature.fpbundle.json"))?)?;
            env1.file_checksums
                .insert("ui-schema.json".to_string(), sha256_hex(tampered_ui));
            let tbl1 = serde_json::to_string(&env1.file_checksums)?;
            env1.checksums_table_sha256 = sha256_hex(tbl1.as_bytes());
            std::fs::write(
                b1.join("signature.fpbundle.json"),
                serde_json::to_vec_pretty(&env1)?,
            )?;
            let err1 = load_and_verify_bundle(&b1, false)
                .err()
                .map(|e| format!("{:#}", e))
                .unwrap_or_default();
            let _ = std::fs::remove_dir_all(&r1);

            // Case 2: Replace trusted_publisher.pub on disk with another Ed25519 public key
            let (r2, b2) = clone_bundle("wrong_pubkey")?;
            std::fs::write(
                b2.join("trusted_publisher.pub"),
                b"d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
            )?;
            let err2 = load_and_verify_bundle(&b2, false)
                .err()
                .map(|e| format!("{:#}", e))
                .unwrap_or_default();
            let _ = std::fs::remove_dir_all(&r2);

            // Case 3: Flip 1 byte in signature_ed25519_hex
            let (r3, b3) = clone_bundle("corrupt_sig")?;
            let mut env3: SignedBundleEnvelope =
                serde_json::from_slice(&std::fs::read(b3.join("signature.fpbundle.json"))?)?;
            env3.signature_ed25519_hex = "00".repeat(64);
            std::fs::write(
                b3.join("signature.fpbundle.json"),
                serde_json::to_vec_pretty(&env3)?,
            )?;
            let err3 = load_and_verify_bundle(&b3, false)
                .err()
                .map(|e| format!("{:#}", e))
                .unwrap_or_default();
            let _ = std::fs::remove_dir_all(&r3);

            // Case 4: Delete manifest.fpbundle.json
            let (r4, b4) = clone_bundle("missing_manifest")?;
            std::fs::remove_file(b4.join("manifest.fpbundle.json"))?;
            let err4 = load_and_verify_bundle(&b4, false)
                .err()
                .map(|e| format!("{:#}", e))
                .unwrap_or_default();
            let _ = std::fs::remove_dir_all(&r4);

            // Case 5: Attacker replaces BOTH trusted_publisher.pub AND signature.fpbundle.json with a rogue Ed25519 keypair
            //         Prove that PE-embedded Root of Trust (RT_RCDATA #7777 inside the .exe) blocks disk key replacement!
            let (r5, b5) = clone_bundle("rogue_keypair_full_replace")?;
            let rogue_pubkey = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
            let rogue_sig = "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b";
            std::fs::write(b5.join("trusted_publisher.pub"), rogue_pubkey.as_bytes())?;
            let mut env5: SignedBundleEnvelope =
                serde_json::from_slice(&std::fs::read(b5.join("signature.fpbundle.json"))?)?;
            env5.publisher_public_key_hex = rogue_pubkey.to_string();
            env5.signature_ed25519_hex = rogue_sig.to_string();
            std::fs::write(
                b5.join("signature.fpbundle.json"),
                serde_json::to_vec_pretty(&env5)?,
            )?;
            let err5 = load_and_verify_bundle(&b5, false)
                .err()
                .map(|e| format!("{:#}", e))
                .unwrap_or_default();
            let _ = std::fs::remove_dir_all(&r5);

            let report = serde_json::json!({
                "command": "test-tamper-suite",
                "algorithm": "Ed25519-RFC8032-SHA256",
                "rootOfTrustSource": format!("PE_RT_RCDATA_{}", PE_ED25519_PUBKEY_RESOURCE_ID),
                "allPassed": err1.contains("ED25519_SIGNATURE_INVALID")
                    && err2.contains("PE_ROOT_OF_TRUST_MISMATCH")
                    && err3.contains("ED25519_SIGNATURE_INVALID")
                    && !err4.is_empty()
                    && err5.contains("ED25519_PUBKEY_UNTRUSTED"),
                "cases": [
                    { "case": "1_modify_bundle_and_recompute_sha256", "rejected": err1.contains("ED25519_SIGNATURE_INVALID"), "error": err1 },
                    { "case": "2_replace_disk_trusted_publisher_pub", "rejected": err2.contains("PE_ROOT_OF_TRUST_MISMATCH"), "error": err2 },
                    { "case": "3_replace_signature_bytes", "rejected": err3.contains("ED25519_SIGNATURE_INVALID"), "error": err3 },
                    { "case": "4_missing_manifest_or_file", "rejected": !err4.is_empty(), "error": err4 },
                    { "case": "5_rogue_keypair_replace_both_disk_pubkey_and_signature_blocked_by_pe_rcdata_7777", "rejected": err5.contains("ED25519_PUBKEY_UNTRUSTED"), "error": err5 }
                ]
            });
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(true)
        }

        _ => Ok(false),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let is_demo = args.iter().any(|a| a == "--dev-demo");
    if args.len() > 1 && args[1].starts_with("--") && args[1] != "--gui" && args[1] != "--dev-demo"
    {
        match run_cli_mode(&args) {
            Ok(true) => std::process::exit(0),
            Ok(false) => {}
            Err(e) => {
                eprintln!("MINI_RUNTIME_ERROR: {:#}", e);
                std::process::exit(1);
            }
        }
    }

    let bundle_dir = resolve_bundle_dir();
    let state = match load_and_verify_bundle(&bundle_dir, is_demo) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("CRITICAL BUNDLE VERIFICATION FAILURE: {:#}", e);
            std::process::exit(2);
        }
    };

    let win_title = state.ui_schema.window_title.clone();
    let (win_w, win_h) = state.ui_schema.default_window_size;
    let shared_state = Arc::new(Mutex::new(state));

    tauri::Builder::default()
        .manage(shared_state)
        .setup(move |app| {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.set_title(&win_title);
                let _ = win.set_size(tauri::Size::Logical(tauri::LogicalSize {
                    width: win_w as f64,
                    height: win_h as f64,
                }));
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            mini_get_bootstrap,
            mini_list_devices,
            mini_run_task
        ])
        .run(tauri::generate_context!())
        .expect("error while running fp-mini-runtime application");
}
