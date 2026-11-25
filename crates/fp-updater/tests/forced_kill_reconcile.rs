use chrono::Utc;
use ed25519_dalek::{Signer, SigningKey};
use fp_builder::pe_icon::inject_ed25519_pubkey_into_pe_exe;
use fp_bundle_contract::{
    CompatibilityVersions, GameProjectManifest, MiniLayoutPreset, MiniToolUiSchema,
    ReleaseBundleManifest, CURRENT_RUNTIME_VERSION, CURRENT_SQLITE_SCHEMA_VERSION,
};
use fp_bundle_verify::{canonical_signing_message, sha256_hex, SignedBundleEnvelope};
use fp_updater::{MiniToolUpdater, UpdateStage};
use fp_workflow_schema::{
    NodePosition, WorkflowDocument, WorkflowEdge, WorkflowGraph, WorkflowNode,
};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use uuid::Uuid;

fn build_signed_dummy_release(dir: &Path, slug: &str, exe_name: &str, version: &str) {
    let bundle_dir = dir.join("bundle");
    fs::create_dir_all(&bundle_dir).unwrap();

    let signing_key = SigningKey::from_bytes(&[91u8; 32]);
    let pub_hex: String = signing_key
        .verifying_key()
        .to_bytes()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();

    let self_bin = std::env::current_exe().unwrap();
    let target_exe = dir.join(exe_name);
    fs::copy(&self_bin, &target_exe).unwrap();
    inject_ed25519_pubkey_into_pe_exe(&target_exe, &pub_hex).unwrap();

    let compat = CompatibilityVersions {
        project_schema_version: 1,
        min_runtime_version: CURRENT_RUNTIME_VERSION.to_string(),
        workflow_schema_version: 1,
        ui_schema_version: 1,
        module_api_version: "1.0.0".to_string(),
        sqlite_schema_version: CURRENT_SQLITE_SCHEMA_VERSION,
    };
    let manifest = ReleaseBundleManifest {
        compatibility: compat.clone(),
        project: GameProjectManifest {
            compatibility: compat,
            project_id: format!("proj-{}", slug),
            slug: slug.to_string(),
            name: format!("Tool {}", slug),
            game_package_id: format!("com.test.{}", slug),
            version: version.to_string(),
            exe_name: exe_name.to_string(),
            window_title: format!("Tool {} v{}", slug, version),
            icon_rel_path: "branding/icon.ico".to_string(),
            target_resolution: (1280, 720),
            packaged_modules: vec!["mod.adb_core".to_string()],
            workflow_files: vec!["wf_main.json".to_string()],
            template_files: vec![],
            bundle_adb: false,
        },
        module_states: vec![],
        workflows_count: 1,
        templates_count: 0,
        built_at_utc: Utc::now().to_rfc3339(),
        publisher: None,
        signing: None,
    };
    let ui_schema = MiniToolUiSchema {
        schema_version: 1,
        project_slug: slug.to_string(),
        window_title: format!("Tool {} v{}", slug, version),
        version_badge: format!("v{}", version),
        theme_accent_hex: "#1878F5".to_string(),
        header_bg_hex: "#121E2A".to_string(),
        layout_preset: MiniLayoutPreset::Compact1Col,
        default_window_size: (680, 480),
        sections_order: vec![],
        hidden_components: vec![],
        device_columns: vec![],
        task_categories: vec![],
        tasks: vec![],
        utilities: vec![],
        initial_runtime_vars: HashMap::new(),
    };
    let workflows = vec![WorkflowDocument {
        schema_version: 1,
        workflow_id: "wf-main".to_string(),
        name: "Main".to_string(),
        status: "Published".to_string(),
        graph: WorkflowGraph {
            nodes: vec![
                WorkflowNode {
                    id: "n1".to_string(),
                    node_type: "Trigger".to_string(),
                    position: NodePosition { x: 0.0, y: 0.0 },
                    config: serde_json::json!({}),
                    timeout_ms: None,
                    retry_policy: None,
                },
                WorkflowNode {
                    id: "n2".to_string(),
                    node_type: "End".to_string(),
                    position: NodePosition { x: 100.0, y: 0.0 },
                    config: serde_json::json!({}),
                    timeout_ms: None,
                    retry_policy: None,
                },
            ],
            edges: vec![WorkflowEdge {
                id: "e1".to_string(),
                source: "n1".to_string(),
                source_port: "out".to_string(),
                target: "n2".to_string(),
                target_port: "in".to_string(),
            }],
        },
        runtime_policy: None,
        disclaimer: None,
    }];

    let m_bytes = serde_json::to_vec_pretty(&manifest).unwrap();
    let u_bytes = serde_json::to_vec_pretty(&ui_schema).unwrap();
    let w_bytes = serde_json::to_vec_pretty(&workflows).unwrap();
    fs::write(bundle_dir.join("manifest.fpbundle.json"), &m_bytes).unwrap();
    fs::write(bundle_dir.join("ui-schema.json"), &u_bytes).unwrap();
    fs::write(bundle_dir.join("workflows.fpbundle.json"), &w_bytes).unwrap();
    fs::write(bundle_dir.join("trusted_publisher.pub"), pub_hex.as_bytes()).unwrap();

    let mut checksums = BTreeMap::new();
    checksums.insert("manifest.fpbundle.json".to_string(), sha256_hex(&m_bytes));
    checksums.insert("ui-schema.json".to_string(), sha256_hex(&u_bytes));
    checksums.insert("workflows.fpbundle.json".to_string(), sha256_hex(&w_bytes));

    let manifest_sha = sha256_hex(&m_bytes);
    let tbl_json = serde_json::to_string(&checksums).unwrap();
    let tbl_hash = sha256_hex(tbl_json.as_bytes());
    let msg = canonical_signing_message("key-kill-test", &manifest_sha, &tbl_hash);
    let sig = signing_key.sign(&msg);
    let sig_hex: String = sig
        .to_bytes()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();

    let env = SignedBundleEnvelope {
        algorithm: "Ed25519-RFC8032-SHA256".to_string(),
        key_id: "key-kill-test".to_string(),
        publisher_public_key_hex: pub_hex,
        manifest_sha256: manifest_sha,
        checksums_table_sha256: tbl_hash,
        file_checksums: checksums,
        signature_ed25519_hex: sig_hex,
        signed_at_utc: Utc::now().to_rfc3339(),
    };
    fs::write(
        bundle_dir.join("signature.fpbundle.json"),
        serde_json::to_vec_pretty(&env).unwrap(),
    )
    .unwrap();
}

#[test]
fn test_real_os_forced_kill_at_stage_and_reconcile_recovery() {
    let updater_bin = env!("CARGO_BIN_EXE_fp-updater");
    let tmp = std::env::temp_dir().join(format!("fp_forced_kill_p52_{}", Uuid::new_v4()));
    let rel_100 = tmp.join("rel_100");
    let rel_110 = tmp.join("rel_110");
    build_signed_dummy_release(&rel_100, "kill-tool", "Auto_KillTool.exe", "1.0.0");
    build_signed_dummy_release(&rel_110, "kill-tool", "Auto_KillTool.exe", "1.1.0");

    // Scenarios A, B, C: Kill real child fp-updater.exe via OS TerminateProcess (child.kill())
    for (stage_arg, expected_active_after, expected_journal_stage) in [
        ("AfterBackup", "1.0.0", UpdateStage::RolledBack),
        ("AfterStage", "1.0.0", UpdateStage::RolledBack),
        ("AfterActivate", "1.1.0", UpdateStage::Completed),
    ] {
        let install_root = tmp.join(format!("install_{}", stage_arg));

        // Install v1.0.0 cleanly first
        let init_status = Command::new(updater_bin)
            .args([
                "install-initial",
                "--install-root",
                install_root.to_str().unwrap(),
                "--release-dir",
                rel_100.to_str().unwrap(),
                "--skip-child-verify-exec",
            ])
            .status()
            .unwrap();
        assert!(init_status.success());

        // Start real OS process upgrade paused after target stage
        let mut child = Command::new(updater_bin)
            .args([
                "upgrade",
                "--install-root",
                install_root.to_str().unwrap(),
                "--release-dir",
                rel_110.to_str().unwrap(),
                "--pause-at",
                stage_arg,
                "--pause-ms",
                "15000",
                "--skip-child-verify-exec",
            ])
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        let stdout = child.stdout.take().unwrap();
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        assert!(
            line.contains(&format!("PAUSED_AT:{}", stage_arg)),
            "Unexpected child output: {}",
            line
        );

        // TerminateProcess on Windows via child.kill()
        child.kill().unwrap();
        let _ = child.wait();

        // Simulate post-crash reboot reconciliation
        let rec_out = Command::new(updater_bin)
            .args([
                "reconcile",
                "--install-root",
                install_root.to_str().unwrap(),
                "--skip-child-verify-exec",
            ])
            .output()
            .unwrap();
        assert!(
            rec_out.status.success(),
            "Reconcile failed: {}",
            String::from_utf8_lossy(&rec_out.stderr)
        );

        let inspector = MiniToolUpdater::new(&install_root);
        let active = inspector.read_active_pointer().unwrap().unwrap();
        let journal = inspector.read_journal().unwrap().unwrap();
        assert_eq!(active.active_version, expected_active_after);
        assert_eq!(journal.stage, expected_journal_stage);
        eprintln!(
            "[OS SIGKILL Reconcile {}] stdout={}",
            stage_arg,
            String::from_utf8_lossy(&rec_out.stdout).trim()
        );
    }

    // Concurrent Dual Updater Process Lock Test on same install_root:
    let lock_root = tmp.join("install_concurrent_lock");
    let init_st = Command::new(updater_bin)
        .args([
            "install-initial",
            "--install-root",
            lock_root.to_str().unwrap(),
            "--release-dir",
            rel_100.to_str().unwrap(),
            "--skip-child-verify-exec",
        ])
        .status()
        .unwrap();
    assert!(init_st.success());

    let mut updater1 = Command::new(updater_bin)
        .args([
            "upgrade",
            "--install-root",
            lock_root.to_str().unwrap(),
            "--release-dir",
            rel_110.to_str().unwrap(),
            "--pause-at",
            "AfterBackup",
            "--pause-ms",
            "15000",
            "--skip-child-verify-exec",
        ])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut r1 = BufReader::new(updater1.stdout.take().unwrap());
    let mut l1 = String::new();
    r1.read_line(&mut l1).unwrap();
    assert!(l1.contains("PAUSED_AT:AfterBackup"));

    // Spawn 2nd updater targeting same install_root while 1st holds updater.lock!
    let updater2_out = Command::new(updater_bin)
        .args([
            "upgrade",
            "--install-root",
            lock_root.to_str().unwrap(),
            "--release-dir",
            rel_110.to_str().unwrap(),
            "--skip-child-verify-exec",
        ])
        .output()
        .unwrap();
    assert!(!updater2_out.status.success());
    let u2_err = format!(
        "{} {}",
        String::from_utf8_lossy(&updater2_out.stdout),
        String::from_utf8_lossy(&updater2_out.stderr)
    );
    assert!(
        u2_err.contains("CONCURRENT_UPDATER_LOCKED"),
        "Expected concurrent lock error, got: {}",
        u2_err
    );

    updater1.kill().unwrap();
    let _ = updater1.wait();
    let rec_lock = Command::new(updater_bin)
        .args([
            "reconcile",
            "--install-root",
            lock_root.to_str().unwrap(),
            "--skip-child-verify-exec",
        ])
        .output()
        .unwrap();
    assert!(rec_lock.status.success());
    assert_eq!(
        MiniToolUpdater::new(&lock_root)
            .read_active_pointer()
            .unwrap()
            .unwrap()
            .active_version,
        "1.0.0"
    );

    let _ = fs::remove_dir_all(&tmp);
}
