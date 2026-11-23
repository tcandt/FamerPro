// Prevents additional console window on Windows in release GUI mode
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::{bail, Context, Result};
use fp_adb::AdbRunner;
use fp_bundle_contract::{
    audit_portable_environment, check_sqlite_downgrade_compatibility,
    resolve_writable_runtime_db_path, AuthenticodeTrustStatus, MiniToolUiSchema, ModuleCapability,
    ModuleRegistry, ReleaseBundleManifest, RuntimeCompatibilityProfile, SigningMode,
    CURRENT_RUNTIME_VERSION, CURRENT_SQLITE_SCHEMA_VERSION,
};
use fp_bundle_verify::{
    read_pe_embedded_pubkey_from_current_process, sha256_hex, verify_authenticode_pe,
    verify_bundle_compatibility_gate, SignedBundleEnvelope, PE_ED25519_PUBKEY_RESOURCE_ID,
};
use fp_domain::{
    ConnectionState, DeviceDriver, DeviceSummary, RuntimeVarDeltaEvent, RuntimeVariableEnvelope,
    StepEvent, VariableSource,
};
use fp_storage::{ExternalWalReader, RuntimeProvenanceSnapshot, StorageManager};
use fp_testkit::FakeDeviceDriver;
use fp_workflow_runtime::{ExecutionContext, ExecutionSignal, WorkflowExecutor};
use fp_workflow_schema::WorkflowDocument;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex,
};
use tauri::{Emitter, Manager, State, WindowEvent};
use tokio::sync::watch;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MiniBootstrapPayload {
    pub manifest: ReleaseBundleManifest,
    pub ui_schema: MiniToolUiSchema,
    pub runtime_vars: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub live_envelopes: BTreeMap<String, RuntimeVariableEnvelope>,
    #[serde(default)]
    pub testkit_envelopes: BTreeMap<String, RuntimeVariableEnvelope>,
    #[serde(default)]
    pub stream_seqs: HashMap<String, u64>,
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
    pub session_id: String,
    pub job_id: String,
    pub device_id: String,
    pub driver_used: String,
    pub is_simulated_demo: bool,
    pub state: String,
    pub events_count: usize,
    pub step_events: Vec<StepEvent>,
    pub point_tapped: Option<fp_domain::Point>,
    pub updated_runtime_vars: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub envelopes: Vec<RuntimeVariableEnvelope>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MiniTaskStartedAck {
    pub task_id: String,
    pub is_simulated_demo: bool,
    pub started_at_utc: String,
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
    pub live_envelopes: BTreeMap<String, RuntimeVariableEnvelope>,
    pub testkit_envelopes: BTreeMap<String, RuntimeVariableEnvelope>,
    pub stream_seqs: HashMap<String, u64>,
    pub icon_base64: Option<String>,
    pub is_demo_mode: bool,
}

fn resolve_runtime_db_path(exe_dir: &Path, slug: &str) -> PathBuf {
    if let Ok(explicit_data) = std::env::var("FP_MINI_DATA_DIR") {
        let dir = PathBuf::from(explicit_data);
        if std::fs::create_dir_all(&dir).is_ok() {
            return dir.join(format!("runtime_{}.db", slug));
        }
    }
    if let Some(versions_dir) = exe_dir.parent() {
        if versions_dir
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.eq_ignore_ascii_case("versions"))
            .unwrap_or(false)
        {
            if let Some(install_root) = versions_dir.parent() {
                let managed_data = install_root.join("data");
                if std::fs::create_dir_all(&managed_data).is_ok() {
                    return managed_data.join(format!("runtime_{}.db", slug));
                }
            }
        }
    }
    resolve_writable_runtime_db_path(exe_dir, slug).0
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
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("bundle")
}

fn load_and_verify_bundle(bundle_dir: &Path, is_demo_mode: bool) -> Result<MiniRuntimeState> {
    let exe_dir = bundle_dir
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        });

    // 1. Read Pinned Root-of-Trust Ed25519 Public Key directly from this executable's PE RT_RCDATA #7777 resource!
    //    An attacker CANNOT bypass verification by editing `bundle/trusted_publisher.pub` on disk.
    let pe_trusted_pubkey_hex = read_pe_embedded_pubkey_from_current_process()?;

    // 1b. Verify PE Authenticode Integrity of the running executable (closes P5.2 Startup Integrity Backlog)
    if let Ok(cur_exe) = std::env::current_exe() {
        let auth_rep = verify_authenticode_pe(&cur_exe);
        match auth_rep.trust_status {
            AuthenticodeTrustStatus::TamperedOrInvalid => {
                bail!(
                    "STARTUP_AUTHENTICODE_TAMPERED: Executable Authenticode signature invalid (0x{:08X}: {})",
                    auth_rep.win_trust_error_code,
                    auth_rep.detail
                );
            }
            AuthenticodeTrustStatus::ExpiredOrTimestampError => {
                bail!(
                    "STARTUP_AUTHENTICODE_EXPIRED: Certificate/timestamp validation failed (0x{:08X}: {})",
                    auth_rep.win_trust_error_code,
                    auth_rep.detail
                );
            }
            _ => {}
        }
    }

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
    if let Some(ref signing_meta) = manifest.signing {
        if signing_meta.signing_mode == SigningMode::PublicSigned {
            if let Ok(cur_exe) = std::env::current_exe() {
                let auth_rep = verify_authenticode_pe(&cur_exe);
                if !auth_rep.trusted_public_release {
                    bail!(
                        "STARTUP_PUBLIC_TRUST_REQUIRED: PublicSigned release failed TrustedPublicChain check ({:?})",
                        auth_rep.trust_status
                    );
                }
            }
        }
    }
    let ui_schema: MiniToolUiSchema =
        serde_json::from_slice(&std::fs::read(bundle_dir.join("ui-schema.json"))?)?;
    let workflows: Vec<WorkflowDocument> =
        serde_json::from_slice(&std::fs::read(bundle_dir.join("workflows.fpbundle.json"))?)?;

    // 2b. Fail-closed 5-Dimension Compatibility Gate (Runtime, Workflow, UI, Module API, SQLite)
    let wf_versions: Vec<u32> = workflows.iter().map(|w| w.schema_version).collect();
    verify_bundle_compatibility_gate(
        &manifest,
        &ui_schema,
        &wf_versions,
        &RuntimeCompatibilityProfile::default(),
    )?;

    // 3. Initialize 5-State Module Registry
    let module_registry = ModuleRegistry::resolve_for_release(
        &manifest.project.packaged_modules,
        &manifest.project.packaged_modules,
    );

    // 4. Resolve Writable SQLite Database Path (supports versioned external data/ & ACL fallback)
    let db_path = resolve_runtime_db_path(&exe_dir, &manifest.project.slug);
    let storage = StorageManager::new(&db_path)
        .unwrap_or_else(|_| StorageManager::new_in_memory().expect("In-memory DB fallback"));
    let db_schema_ver = storage.highest_applied_migration_version().unwrap_or(0);
    check_sqlite_downgrade_compatibility(db_schema_ver, CURRENT_SQLITE_SCHEMA_VERSION)?;

    // 5. Load optional branding PNG as base64 for window header
    let icon_png_path = bundle_dir.join("branding").join("icon.png");
    let icon_base64 = if icon_png_path.exists() {
        std::fs::read(&icon_png_path)
            .ok()
            .map(|b| format!("data:image/png;base64,{}", base64_encode(&b)))
    } else {
        None
    };

    let scope_key = format!("mini:{}", manifest.project.slug);
    let prov = storage
        .load_runtime_snapshot_with_recovery(&scope_key, None)
        .unwrap_or_else(|_| RuntimeProvenanceSnapshot {
            base_scope_key: scope_key.clone(),
            live_envelopes: BTreeMap::new(),
            testkit_envelopes: BTreeMap::new(),
            stream_seqs: HashMap::new(),
            missed_events: Vec::new(),
            loaded_at_utc: chrono::Utc::now().to_rfc3339(),
        });
    let mut runtime_vars = HashMap::new();
    for (k, env) in &prov.testkit_envelopes {
        runtime_vars.insert(k.clone(), env.value.clone());
    }
    for (k, env) in &prov.live_envelopes {
        runtime_vars.insert(k.clone(), env.value.clone());
    }

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
        live_envelopes: prov.live_envelopes,
        testkit_envelopes: prov.testkit_envelopes,
        stream_seqs: prov.stream_seqs,
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

fn apply_delta_event_to_state(st: &mut MiniRuntimeState, delta_ev: &RuntimeVarDeltaEvent) {
    st.stream_seqs
        .insert(delta_ev.stream_id.clone(), delta_ev.seq);
    for env in &delta_ev.deltas {
        if env.source == VariableSource::TestkitMock {
            let should_insert = st
                .testkit_envelopes
                .get(&env.name)
                .map(|cur| env.seq >= cur.seq)
                .unwrap_or(true);
            if should_insert {
                st.testkit_envelopes.insert(env.name.clone(), env.clone());
                if !st.live_envelopes.contains_key(&env.name) {
                    st.runtime_vars.insert(env.name.clone(), env.value.clone());
                }
            }
        } else {
            let should_insert = st
                .live_envelopes
                .get(&env.name)
                .map(|cur| env.seq >= cur.seq)
                .unwrap_or(true);
            if should_insert {
                st.live_envelopes.insert(env.name.clone(), env.clone());
                st.runtime_vars.insert(env.name.clone(), env.value.clone());
            }
        }
    }
}

fn sanitize_workflow_json_for_job_store(wf: &WorkflowDocument) -> String {
    let mut scrubbed = wf.clone();
    for node in &mut scrubbed.graph.nodes {
        if let Some(var_key) = node
            .config
            .get("varKey")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
        {
            if let Some(raw_v) = node.config.get("value").cloned() {
                let (clean_v, _) = fp_security::sanitize_runtime_variable(&var_key, &raw_v);
                node.config["value"] = clean_v;
            }
        }
        let (clean_cfg, _) = fp_security::sanitize_runtime_variable(&node.id, &node.config);
        node.config = clean_cfg;
    }
    serde_json::to_string(&scrubbed).unwrap_or_default()
}

async fn execute_task_internal(
    st_arc: Arc<Mutex<MiniRuntimeState>>,
    task_id: &str,
    adb_serial_override: Option<String>,
    allow_demo_mock: bool,
    cancel_after_ms: Option<u64>,
    app_handle: Option<tauri::AppHandle>,
) -> Result<MiniTaskRunReceipt> {
    let (workflow_doc, adb_path, storage, slug, project_id, project_version, seeded_vars) = {
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

        // Partition seed cleanly: NEVER feed TESTKIT variables into LIVE execution!
        let mut seed = HashMap::new();
        if allow_demo_mock {
            for (k, env) in &st.testkit_envelopes {
                seed.insert(k.clone(), env.value.clone());
            }
        } else {
            for (k, env) in &st.live_envelopes {
                seed.insert(k.clone(), env.value.clone());
            }
        }

        (
            wf,
            st.adb_runner.adb_path.clone(),
            st.storage.clone(),
            st.manifest.project.slug.clone(),
            st.manifest.project.project_id.clone(),
            st.manifest.project.version.clone(),
            seed,
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
    let wf_json = sanitize_workflow_json_for_job_store(&workflow_doc);
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

    let source_mode = if is_sim {
        VariableSource::TestkitMock
    } else {
        VariableSource::LiveState
    };
    let workflow_version = format!("{}@v{}", workflow_doc.workflow_id, project_version);
    let mut executor = WorkflowExecutor::new(driver, rx);
    let mut ctx = ExecutionContext::new(job_id.clone(), device_id.clone())
        .with_source_mode(source_mode);
    ctx.project_id = Some(project_id.clone());
    ctx.workflow_version = Some(workflow_version.clone());
    ctx.task_id = Some(task_id.to_string());
    ctx.variables = seeded_vars.clone();
    let session_id = ctx.session_id.clone();

    let events_counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = events_counter.clone();
    let captured_events = Arc::new(Mutex::new(Vec::new()));
    let cap_clone = captured_events.clone();
    let storage_clone = storage.clone();
    let st_arc_cb = st_arc.clone();
    let app_cb = app_handle.clone();
    let base_scope_key = format!("mini:{}", slug);
    let scope_cb = base_scope_key.clone();
    let proj_cb = project_id.clone();
    let sess_cb = session_id.clone();
    let job_cb = job_id.clone();
    let wf_ver_cb = workflow_version.clone();
    let task_cb = task_id.to_string();

    let job_state = executor
        .execute_workflow(
            &workflow_doc,
            &mut ctx,
            Some(Arc::new(move |mut evt| {
                // Ensure StepEvent payload stored in SQLite is scrubbed before insert
                if let Ok(mut pj) = serde_json::from_str::<serde_json::Value>(&evt.payload_json) {
                    if let Some(vars_obj) = pj.get("variables").cloned() {
                        let (clean_vars, _) =
                            fp_security::sanitize_runtime_variable("variables", &vars_obj);
                        pj["variables"] = clean_vars;
                    }
                    if let Some(deltas) = pj
                        .get("deltaEnvelopes")
                        .and_then(|d| serde_json::from_value::<Vec<RuntimeVariableEnvelope>>(d.clone()).ok())
                    {
                        if !deltas.is_empty() {
                            if let Ok(delta_ev) = storage_clone.record_runtime_var_deltas(
                                &scope_cb,
                                &proj_cb,
                                &sess_cb,
                                &job_cb,
                                &wf_ver_cb,
                                deltas,
                            ) {
                                {
                                    let mut st = st_arc_cb.lock().unwrap();
                                    apply_delta_event_to_state(&mut st, &delta_ev);
                                }
                                if let Some(ref app) = app_cb {
                                    let _ = app.emit("mini://runtime-var-delta", &delta_ev);
                                }
                            }
                        }
                    }
                    evt.payload_json = serde_json::to_string(&pj).unwrap_or(evt.payload_json);
                }
                counter_clone.fetch_add(1, Ordering::SeqCst);
                let _ = storage_clone.insert_step_event(&evt);
                if let Some(ref app) = app_cb {
                    let _ = app.emit(
                        "mini://task-progress",
                        serde_json::json!({
                            "taskId": task_cb,
                            "jobId": evt.job_id,
                            "nodeId": evt.node_id,
                            "eventType": evt.event_type,
                            "atUtc": evt.at.to_rfc3339(),
                        }),
                    );
                }
                cap_clone.lock().unwrap().push(evt);
            })),
        )
        .await?;

    let _ = storage.complete_job(&job_id, &format!("{:?}", job_state));
    let point_tapped = ctx.last_receipt.as_ref().and_then(|r| r.point_tapped);

    let mut final_envelopes: Vec<RuntimeVariableEnvelope> = Vec::new();
    let now_utc = chrono::Utc::now().to_rfc3339();
    if job_state == fp_domain::JobState::Succeeded {
        let prev_done = seeded_vars
            .get("task_completed_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        final_envelopes.push(RuntimeVariableEnvelope {
            project_id: project_id.clone(),
            session_id: session_id.clone(),
            job_id: job_id.clone(),
            name: "task_completed_count".to_string(),
            value_type: "number".to_string(),
            value: serde_json::Value::from(prev_done + 1),
            source: source_mode,
            verified: !is_sim,
            confidence: None,
            updated_at_utc: now_utc.clone(),
            workflow_version: workflow_version.clone(),
            seq: 0,
            redacted: false,
        });
    }
    for (k, v) in [
        ("last_task_id", serde_json::Value::String(task_id.to_string())),
        ("last_task_state", serde_json::Value::String(format!("{:?}", job_state))),
        ("last_data_source", serde_json::Value::String(driver_label.clone())),
        ("last_updated_utc", serde_json::Value::String(now_utc.clone())),
    ] {
        final_envelopes.push(RuntimeVariableEnvelope {
            project_id: project_id.clone(),
            session_id: session_id.clone(),
            job_id: job_id.clone(),
            name: k.to_string(),
            value_type: "string".to_string(),
            value: v,
            source: source_mode,
            verified: !is_sim,
            confidence: None,
            updated_at_utc: now_utc.clone(),
            workflow_version: workflow_version.clone(),
            seq: 0,
            redacted: false,
        });
    }

    if !final_envelopes.is_empty() {
        if let Ok(final_delta_ev) = storage.record_runtime_var_deltas(
            &base_scope_key,
            &project_id,
            &session_id,
            &job_id,
            &workflow_version,
            final_envelopes,
        ) {
            {
                let mut st = st_arc.lock().unwrap();
                apply_delta_event_to_state(&mut st, &final_delta_ev);
            }
            if let Some(ref app) = app_handle {
                let _ = app.emit("mini://runtime-var-delta", &final_delta_ev);
            }
        }
    }

    let (updated_runtime_vars, collected_envelopes) = {
        let st = st_arc.lock().unwrap();
        let envs: Vec<RuntimeVariableEnvelope> = if is_sim {
            st.testkit_envelopes.values().cloned().collect()
        } else {
            st.live_envelopes.values().cloned().collect()
        };
        (st.runtime_vars.clone(), envs)
    };

    let step_events = captured_events.lock().unwrap().clone();

    Ok(MiniTaskRunReceipt {
        task_id: task_id.to_string(),
        workflow_id: workflow_doc.workflow_id,
        session_id,
        job_id,
        device_id,
        driver_used: driver_label,
        is_simulated_demo: is_sim,
        state: format!("{:?}", job_state),
        events_count: events_counter.load(Ordering::SeqCst),
        step_events,
        point_tapped,
        updated_runtime_vars,
        envelopes: collected_envelopes,
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
        sqlite_db_path,
        sig_alg,
        key_id,
        pubkey,
        icon_b64,
        adb_path,
        is_demo,
        storage,
        slug,
    ) = {
        let st = state.lock().unwrap();
        (
            st.manifest.clone(),
            st.ui_schema.clone(),
            st.sqlite_db_path.clone(),
            st.signature_algorithm.clone(),
            st.key_id.clone(),
            st.publisher_public_key_hex.clone(),
            st.icon_base64.clone(),
            st.adb_runner.adb_path.clone(),
            st.is_demo_mode,
            st.storage.clone(),
            st.manifest.project.slug.clone(),
        )
    };

    let scope_key = format!("mini:{}", slug);
    let prov = storage
        .load_runtime_snapshot_with_recovery(&scope_key, None)
        .map_err(|e| e.to_string())?;
    let mut flat_vars = HashMap::new();
    for (k, env) in &prov.testkit_envelopes {
        flat_vars.insert(k.clone(), env.value.clone());
    }
    for (k, env) in &prov.live_envelopes {
        flat_vars.insert(k.clone(), env.value.clone());
    }
    {
        let mut st = state.lock().unwrap();
        st.live_envelopes = prov.live_envelopes.clone();
        st.testkit_envelopes = prov.testkit_envelopes.clone();
        st.stream_seqs = prov.stream_seqs.clone();
        st.runtime_vars = flat_vars.clone();
    }

    let runner = AdbRunner::new(Some(adb_path));
    let devices = runner.list_devices().await.unwrap_or_default();

    Ok(MiniBootstrapPayload {
        manifest,
        ui_schema,
        runtime_vars: flat_vars,
        live_envelopes: prov.live_envelopes,
        testkit_envelopes: prov.testkit_envelopes,
        stream_seqs: prov.stream_seqs,
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
async fn mini_get_runtime_snapshot(
    since_seq_by_stream: Option<HashMap<String, u64>>,
    state: State<'_, Arc<Mutex<MiniRuntimeState>>>,
) -> Result<RuntimeProvenanceSnapshot, String> {
    let (storage, scope_key) = {
        let st = state.lock().unwrap();
        (st.storage.clone(), format!("mini:{}", st.manifest.project.slug))
    };
    storage
        .load_runtime_snapshot_with_recovery(&scope_key, since_seq_by_stream.as_ref())
        .map_err(|e| e.to_string())
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
    app: tauri::AppHandle,
    state: State<'_, Arc<Mutex<MiniRuntimeState>>>,
) -> Result<MiniTaskRunReceipt, String> {
    let st_arc = state.inner().clone();
    let allow_demo = use_demo_mock.unwrap_or(false);
    execute_task_internal(st_arc, &task_id, None, allow_demo, None, Some(app))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn mini_start_task(
    task_id: String,
    use_demo_mock: Option<bool>,
    app: tauri::AppHandle,
    state: State<'_, Arc<Mutex<MiniRuntimeState>>>,
) -> Result<MiniTaskStartedAck, String> {
    let allow_demo = use_demo_mock.unwrap_or(false);
    if !allow_demo {
        let adb_path = {
            let st = state.lock().unwrap();
            st.adb_runner.adb_path.clone()
        };
        let runner = AdbRunner::new(Some(adb_path));
        let devices = runner.list_devices().await.unwrap_or_default();
        if !devices.iter().any(|d| d.connection_state == ConnectionState::Ready) {
            return Err("NO_LIVE_ADB_DEVICE_ONLINE: 0 ready Android devices found via ADB. Fail-closed: connect a device or explicitly use TestKit Simulation.".to_string());
        }
    }

    let st_arc = state.inner().clone();
    let task_clone = task_id.clone();
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(err) =
            execute_task_internal(st_arc, &task_clone, None, allow_demo, None, Some(app_clone.clone()))
                .await
        {
            let _ = app_clone.emit(
                "mini://task-progress",
                serde_json::json!({
                    "taskId": task_clone,
                    "state": "Failed",
                    "error": err.to_string(),
                    "atUtc": chrono::Utc::now().to_rfc3339(),
                }),
            );
        }
    });

    Ok(MiniTaskStartedAck {
        task_id,
        is_simulated_demo: allow_demo,
        started_at_utc: chrono::Utc::now().to_rfc3339(),
    })
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
                "version": state.manifest.project.version,
                "runtimeVersion": CURRENT_RUNTIME_VERSION,
                "sqliteSchemaVersion": state.storage.highest_applied_migration_version().unwrap_or(0),
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
                None,
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

            // Case 5: Attacker modifies bundle files, signs them with a real rogue Ed25519 keypair,
            //         and replaces BOTH trusted_publisher.pub AND signature.fpbundle.json on disk.
            //         Prove that:
            //         (a) The forged bundle IS 100% cryptographically valid against disk trusted_publisher.pub, AND
            //         (b) load_and_verify_bundle STILL rejects it with ED25519_PUBKEY_UNTRUSTED because
            //             Root of Trust is pinned in PE RT_RCDATA #7777 inside the .exe!
            let (r5, b5) = clone_bundle("rogue_keypair_full_replace")?;
            let rogue_pubkey = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
            let rogue_manifest = b"{\"project\":\"jx2-mobile\"}";
            let rogue_ui = b"{\"windowTitle\":\"HACKED BY ROGUE KEYPAIR\"}";
            std::fs::write(b5.join("manifest.fpbundle.json"), rogue_manifest)?;
            std::fs::write(b5.join("ui-schema.json"), rogue_ui)?;
            std::fs::write(b5.join("trusted_publisher.pub"), rogue_pubkey.as_bytes())?;

            let mut rogue_checksums = std::collections::BTreeMap::new();
            rogue_checksums.insert(
                "manifest.fpbundle.json".to_string(),
                sha256_hex(rogue_manifest),
            );
            rogue_checksums.insert("ui-schema.json".to_string(), sha256_hex(rogue_ui));
            let rogue_table_json = serde_json::to_string(&rogue_checksums)?;

            let env5 = SignedBundleEnvelope {
                algorithm: "Ed25519-RFC8032-SHA256".to_string(),
                key_id: "fp-ed25519-rogue-attacker".to_string(),
                publisher_public_key_hex: rogue_pubkey.to_string(),
                manifest_sha256: sha256_hex(rogue_manifest),
                checksums_table_sha256: sha256_hex(rogue_table_json.as_bytes()),
                file_checksums: rogue_checksums,
                signature_ed25519_hex: "e407316b446d120cc627328f731a68d371db3e23a73fcaccb1767bedfb588953676b5331dddf799b0c95f72cc587df25e1f7b5f5cb0d18221ffdf24e08ed9c02".to_string(),
                signed_at_utc: chrono::Utc::now().to_rfc3339(),
            };
            std::fs::write(
                b5.join("signature.fpbundle.json"),
                serde_json::to_vec_pretty(&env5)?,
            )?;

            // Confirm the rogue signature is 100% cryptographically valid under the replaced disk public key
            let valid_against_disk_pubkey = env5
                .verify_ed25519_with_public_key(&b5, rogue_pubkey)
                .is_ok();

            let err5 = load_and_verify_bundle(&b5, false)
                .err()
                .map(|e| format!("{:#}", e))
                .unwrap_or_default();
            let _ = std::fs::remove_dir_all(&r5);

            let case5_passed =
                valid_against_disk_pubkey && err5.contains("ED25519_PUBKEY_UNTRUSTED");

            let report = serde_json::json!({
                "command": "test-tamper-suite",
                "algorithm": "Ed25519-RFC8032-SHA256",
                "rootOfTrustSource": format!("PE_RT_RCDATA_{}", PE_ED25519_PUBKEY_RESOURCE_ID),
                "allPassed": err1.contains("ED25519_SIGNATURE_INVALID")
                    && err2.contains("PE_ROOT_OF_TRUST_MISMATCH")
                    && err3.contains("ED25519_SIGNATURE_INVALID")
                    && !err4.is_empty()
                    && case5_passed,
                "cases": [
                    { "case": "1_modify_bundle_and_recompute_sha256", "rejected": err1.contains("ED25519_SIGNATURE_INVALID"), "error": err1 },
                    { "case": "2_replace_disk_trusted_publisher_pub", "rejected": err2.contains("PE_ROOT_OF_TRUST_MISMATCH"), "error": err2 },
                    { "case": "3_replace_signature_bytes", "rejected": err3.contains("ED25519_SIGNATURE_INVALID"), "error": err3 },
                    { "case": "4_missing_manifest_or_file", "rejected": !err4.is_empty(), "error": err4 },
                    {
                        "case": "5_rogue_keypair_replace_both_disk_pubkey_and_signature_blocked_by_pe_rcdata_7777",
                        "rogueSignatureValidAgainstDiskPubkey": valid_against_disk_pubkey,
                        "rejected": case5_passed,
                        "error": err5
                    }
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
        if args[1] == "--check-deps" {
            let exe_dir = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.to_path_buf()))
                .unwrap_or_else(|| PathBuf::from("."));
            let bundle_dir = resolve_bundle_dir();
            let slug = std::fs::read(bundle_dir.join("manifest.fpbundle.json"))
                .ok()
                .and_then(|b| serde_json::from_slice::<ReleaseBundleManifest>(&b).ok())
                .map(|m| m.project.slug)
                .unwrap_or_else(|| "mini-tool".to_string());
            let audit = audit_portable_environment(&exe_dir, &slug);
            println!("{}", serde_json::to_string_pretty(&audit).unwrap_or_default());
            std::process::exit(0);
        }
        match run_cli_mode(&args) {
            Ok(true) => std::process::exit(0),
            Ok(false) => {}
            Err(e) => {
                eprintln!("MINI_RUNTIME_ERROR: {:#}", e);
                std::process::exit(1);
            }
        }
    }

    let (wv2_ok, wv2_msg) = fp_bundle_contract::detect_webview2_runtime();
    if !wv2_ok {
        eprintln!("WEBVIEW2_RUNTIME_MISSING: {}", wv2_msg);
        std::process::exit(3);
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
    let watch_db_path = PathBuf::from(&state.sqlite_db_path);
    let pid_lock_path = watch_db_path
        .parent()
        .map(|d| d.join(format!(".runtime_{}.pid", state.manifest.project.slug)));
    if let Some(ref lp) = pid_lock_path {
        let _ = std::fs::write(lp, std::process::id().to_string());
    }
    let pid_lock_win = pid_lock_path.clone();
    let watch_scope = format!("mini:{}", state.manifest.project.slug);
    let initial_watch_seqs = state.stream_seqs.clone();
    let shared_state = Arc::new(Mutex::new(state));
    let shared_for_watcher = Arc::clone(&shared_state);
    let watcher_stop = Arc::new(AtomicBool::new(false));
    let watcher_stop_bg = Arc::clone(&watcher_stop);
    let watcher_stop_win = Arc::clone(&watcher_stop);

    tauri::Builder::default()
        .manage(shared_state)
        .on_window_event(move |_window, event| {
            if matches!(event, WindowEvent::Destroyed) {
                watcher_stop_win.store(true, Ordering::Relaxed);
                if let Some(ref lp) = pid_lock_win {
                    let _ = std::fs::remove_file(lp);
                }
            }
        })
        .setup(move |app| {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.set_title(&win_title);
                let _ = win.set_size(tauri::Size::Logical(tauri::LogicalSize {
                    width: win_w as f64,
                    height: win_h as f64,
                }));
            }

            // Cross-process SQLite WAL delta watcher using ONE persistent ExternalWalReader connection
            // so `PRAGMA data_version` tracks external writer commits cleanly with bounded retry on SQLITE_BUSY.
            let app_handle_bg = app.handle().clone();
            std::thread::spawn(move || {
                let mut seen_seqs = initial_watch_seqs;
                let mut wal_reader = ExternalWalReader::open(&watch_db_path).ok();
                while !watcher_stop_bg.load(Ordering::Relaxed) {
                    std::thread::sleep(std::time::Duration::from_millis(180));
                    if watcher_stop_bg.load(Ordering::Relaxed) {
                        break;
                    }
                    // Also sync in-memory dispatched seqs so we avoid needless re-emits
                    {
                        if let Ok(st) = shared_for_watcher.lock() {
                            for (sid, s) in &st.stream_seqs {
                                let cur = seen_seqs.entry(sid.clone()).or_insert(0);
                                if *s > *cur {
                                    *cur = *s;
                                }
                            }
                        }
                    }
                    if wal_reader.is_none() {
                        wal_reader = ExternalWalReader::open(&watch_db_path).ok();
                    }
                    if let Some(reader) = wal_reader.as_mut() {
                        if let Ok((changed, events)) = reader.poll_deltas(&watch_scope, &seen_seqs)
                        {
                            if changed && !events.is_empty() {
                                for ev in events {
                                    let cur = seen_seqs.entry(ev.stream_id.clone()).or_insert(0);
                                    if ev.seq > *cur {
                                        *cur = ev.seq;
                                        if let Ok(mut st) = shared_for_watcher.lock() {
                                            apply_delta_event_to_state(&mut st, &ev);
                                        }
                                        let _ = app_handle_bg.emit("mini://runtime-var-delta", &ev);
                                    }
                                }
                            }
                        }
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            mini_get_bootstrap,
            mini_get_runtime_snapshot,
            mini_list_devices,
            mini_run_task,
            mini_start_task
        ])
        .run(tauri::generate_context!())
        .expect("error while running fp-mini-runtime application");
}
