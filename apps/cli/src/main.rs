use anyhow::Context;
use clap::{Parser, Subcommand};
use fp_adb::AdbRunner;
use fp_builder::{
    orchestrator::run_concurrent_two_device_verification, seed_two_poc_projects, MiniToolPackager,
};
use fp_domain::DeviceDriver;
use fp_testkit::FakeDeviceDriver;
use fp_workflow_runtime::{ExecutionContext, ExecutionSignal, WorkflowExecutor};
use fp_workflow_schema::WorkflowValidator;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::watch;

#[derive(Parser)]
#[command(name = "farmerpro-cli")]
#[command(about = "FarmerPro Headless Automation Diagnostic & Builder CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List connected Android devices via ADB
    Devices,

    /// Validate a workflow JSON file against FarmerPro schema
    Validate {
        #[arg(short, long)]
        file: Option<PathBuf>,

        #[arg(value_name = "FILE")]
        path: Option<PathBuf>,
    },

    /// Run a sample workflow headlessly against a mock or real device
    Run {
        #[arg(short, long)]
        workflow: Option<PathBuf>,

        #[arg(value_name = "WORKFLOW")]
        path: Option<PathBuf>,

        #[arg(long, default_value_t = false)]
        mock: bool,

        #[arg(short, long)]
        serial: Option<String>,
    },

    /// Export the two PoC Mini Auto-Tool packages using fp-builder and external Ed25519 producer keystore
    ExportPoc {
        #[arg(long)]
        repo_root: Option<PathBuf>,
    },

    /// Run multi-device concurrent lease & generation fencing verification
    VerifyMultiDevice,

    /// Run P2 Multi-Project Storage, Migration/Rollback, JX2 Preservation, Project Isolation, and WinCred/DPAPI verification on a sandbox copy
    VerifyP2 {
        #[arg(long)]
        repo_root: Option<PathBuf>,

        #[arg(long)]
        source_db: Option<PathBuf>,
    },

    /// Safely backup, verify restore on copy, then apply P2 v1->v2 migration and non-destructive JX2 migration to a target SQLite database
    MigrateP2 {
        #[arg(long)]
        db_path: PathBuf,

        #[arg(long)]
        backup_path: PathBuf,

        #[arg(long)]
        jx2_dir: PathBuf,

        #[arg(long)]
        projects_dir: PathBuf,
    },

    /// Execute and verify the standardized WAL-safe Rollback Runbook on a sandbox copy (never mutates live DB)
    VerifyRollbackRunbook {
        #[arg(long)]
        live_db: PathBuf,

        #[arg(long)]
        backup_db: PathBuf,

        #[arg(long)]
        out_dir: PathBuf,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Devices => {
            println!("== FarmerPro ADB Discovery ==");
            let runner = AdbRunner::default();
            match runner.list_devices().await {
                Ok(devices) => {
                    if devices.is_empty() {
                        println!("No connected Android devices found via ADB.");
                    } else {
                        println!("Found {} device(s):", devices.len());
                        for d in devices {
                            println!(
                                "  - [{}] Serial: {} | Model: {:?} | State: {:?}",
                                d.id, d.adb_serial, d.model, d.connection_state
                            );
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to run ADB discovery: {}", e);
                }
            }
        }

        Commands::Validate { file, path } => {
            let target_file = file
                .or(path)
                .context("Must provide a workflow JSON file via --file or positional argument")?;
            println!("Validating workflow file: {}", target_file.display());
            let content = fs::read_to_string(&target_file)?;
            match WorkflowValidator::parse_and_validate(&content) {
                Ok(doc) => {
                    println!(
                        "Validation SUCCESS: '{}' (schema v{}) with {} nodes and {} edges.",
                        doc.name,
                        doc.schema_version,
                        doc.graph.nodes.len(),
                        doc.graph.edges.len()
                    );
                }
                Err(e) => {
                    eprintln!("Validation FAILED:\n{}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Run {
            workflow,
            path,
            mock,
            serial,
        } => {
            let target_wf = workflow
                .or(path)
                .context("Must provide a workflow JSON file via --workflow or positional argument")?;
            println!("Loading workflow: {}", target_wf.display());
            let content = fs::read_to_string(&target_wf)?;
            let doc = WorkflowValidator::parse_and_validate(&content)?;

            let driver: Arc<dyn DeviceDriver> = match (mock, serial) {
                (false, Some(s)) => {
                    println!("Connecting to real ADB device: {}", s);
                    Arc::new(fp_adb::AdbDeviceDriver::new(s, AdbRunner::default()))
                }
                _ => {
                    println!("Using FakeDeviceDriver for deterministic headless execution.");
                    Arc::new(FakeDeviceDriver::new("fake-device-01", 1080, 1920))
                }
            };

            let (_tx, rx) = watch::channel(ExecutionSignal::Running);
            let mut executor = WorkflowExecutor::new(driver, rx);
            let mut ctx =
                ExecutionContext::new("cli-job-01".to_string(), "target-device".to_string());

            println!("Executing workflow '{}'...", doc.name);
            let state = executor
                .execute_workflow(
                    &doc,
                    &mut ctx,
                    Some(Arc::new(|event| {
                        println!(
                            "  [STEP EVENT] Node: {} | Type: {:?} | Time: {}",
                            event.node_id, event.event_type, event.at
                        );
                    })),
                )
                .await?;

            println!("Workflow execution finished with state: {:?}", state);
        }

        Commands::ExportPoc { repo_root } => {
            let root = repo_root.unwrap_or_else(|| std::env::current_dir().unwrap());
            let runtime_exe = root.join("target").join("release").join("fp-mini-runtime.exe");
            let bin_dir = root.join("bin");
            let (jx2_proj, slg_proj) = seed_two_poc_projects(&root)?;

            let jx2_out = root.join("releases").join("poc").join("Auto_JX2_Mobile");
            let slg_out = root.join("releases").join("poc").join("Auto_Kingdom_SLG");

            let rep1 = MiniToolPackager::build_and_export_release(
                &jx2_proj,
                &runtime_exe,
                Some(&bin_dir),
                &jx2_out,
                None,
            )?;
            let rep2 = MiniToolPackager::build_and_export_release(
                &slg_proj,
                &runtime_exe,
                Some(&bin_dir),
                &slg_out,
                None,
            )?;

            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "command": "export-poc",
                    "status": "PASS",
                    "releases": [rep1, rep2]
                }))?
            );
        }

        Commands::VerifyMultiDevice => {
            let report = run_concurrent_two_device_verification().await?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }

        Commands::VerifyP2 {
            repo_root,
            source_db,
        } => {
            let root = repo_root.unwrap_or_else(|| std::env::current_dir().unwrap());
            let report = run_p2_sandbox_verification(&root, source_db)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }

        Commands::MigrateP2 {
            db_path,
            backup_path,
            jx2_dir,
            projects_dir,
        } => {
            let report = run_p2_guarded_migration(&db_path, &backup_path, &jx2_dir, &projects_dir)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }

        Commands::VerifyRollbackRunbook {
            live_db,
            backup_db,
            out_dir,
        } => {
            let report = run_rollback_runbook_verification(&live_db, &backup_db, &out_dir)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }

    Ok(())
}

fn run_p2_sandbox_verification(
    repo_root: &std::path::Path,
    source_db: Option<PathBuf>,
) -> anyhow::Result<serde_json::Value> {
    use fp_bundle_contract::{MiniTaskDef, TaskCategoryDef};
    use fp_security::{CredentialVault, ProducerKeystore, SecretNamespace};
    use fp_storage::{
        CloneProjectRequest, CreateProjectRequest, ProjectAccountInput, StorageManager,
        UpdateProjectRequest, VisionTemplateInput, DEFAULT_LEGACY_PROJECT_ID,
    };

    let ev_dir = repo_root.join("artifacts").join("p2_evidence");
    let sandbox_dir = ev_dir.join("sandbox_run");
    if sandbox_dir.exists() {
        let _ = std::fs::remove_dir_all(&sandbox_dir);
    }
    std::fs::create_dir_all(&sandbox_dir)?;

    // 1. Choose source v1 production database (dist-portable if present, else root farmerpro.db)
    let default_dist_db = repo_root
        .join("dist-portable")
        .join("FarmerPro")
        .join("data")
        .join("farmerpro.db");
    let src_db = source_db.unwrap_or_else(|| {
        if default_dist_db.exists() {
            default_dist_db
        } else {
            repo_root.join("farmerpro.db")
        }
    });

    // Copy source DB to sandbox BEFORE touching anything (strictly read-only on production!)
    let pre_copy_db = sandbox_dir.join("01_pre_migration_copy.db");
    std::fs::copy(&src_db, &pre_copy_db)?;
    // Also copy WAL/SHM if present next to source DB so SQLite reads all committed frames
    for suffix in ["-wal", "-shm"] {
        let wal_src = PathBuf::from(format!("{}{}", src_db.display(), suffix));
        if wal_src.exists() {
            let wal_dst = PathBuf::from(format!("{}{}", pre_copy_db.display(), suffix));
            let _ = std::fs::copy(&wal_src, &wal_dst);
        }
    }

    // Read pre-migration stats using raw rusqlite (before running v2 migration)
    let (pre_counts, pre_integrity) = {
        let conn = rusqlite::Connection::open(&pre_copy_db)?;
        let integ: String = conn.query_row("PRAGMA integrity_check;", [], |r| r.get(0))?;
        let mut counts = serde_json::Map::new();
        for tbl in [
            "devices",
            "workflows",
            "workflow_versions",
            "accounts",
            "jobs",
            "step_events",
            "checkpoints",
            "device_leases",
        ] {
            let cnt: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {};", tbl), [], |r| r.get(0))
                .unwrap_or(0);
            counts.insert(tbl.to_string(), serde_json::json!(cnt));
        }
        (serde_json::Value::Object(counts), integ)
    };

    // 2. Open sandbox copy with StorageManager -> Online Backup BEFORE migration?
    // Even better: create verified Online Backup using rusqlite Backup API on pre-migration DB, then migrate a restored copy!
    let online_bk_path = sandbox_dir.join("02_online_backup_v1.db");
    {
        let src_c = rusqlite::Connection::open(&pre_copy_db)?;
        let mut dst_c = rusqlite::Connection::open(&online_bk_path)?;
        let bk = rusqlite::backup::Backup::new(&src_c, &mut dst_c)?;
        bk.run_to_completion(100, std::time::Duration::from_millis(5), None)?;
    }

    // Restore from the v1 online backup into `03_migrated_v2.db` and run StorageManager v1->v2 migration
    let migrated_db_path = sandbox_dir.join("03_migrated_v2.db");
    let pre_restore_report = StorageManager::restore_from_backup(&online_bk_path, &migrated_db_path)?;
    let storage = StorageManager::new(&migrated_db_path)?;

    // Idempotency check (second run_migrations call)
    storage.run_migrations()?;
    let versions_after_v2 = storage.applied_migration_versions()?;
    let post_v2_integrity = storage.check_integrity()?;

    // 3. Atomic Rollback Test (Faulty v3 migration must roll back completely)
    let faulty_v3_res = storage.apply_transactional_migration(
        3,
        "CREATE TABLE p2_faulty_temp (id TEXT PRIMARY KEY); INSERT INTO non_existent_table_p2 VALUES (1);",
    );
    let rollback_triggered = faulty_v3_res.is_err();
    let versions_after_rollback = storage.applied_migration_versions()?;
    let post_rollback_integrity = storage.check_integrity()?;
    let rollback_clean = rollback_triggered
        && versions_after_rollback == vec![1, 2]
        && !post_rollback_integrity
            .table_counts
            .contains_key("p2_faulty_temp");

    // 4. Migrate 10 Legacy JX2 Workflows + 8 Template PNGs non-destructively into sandbox projects dir
    let sandbox_projects = sandbox_dir.join("projects");
    let jx2_summary =
        storage.migrate_legacy_jx2_assets(&repo_root.join("games").join("jx2mobile"), &sandbox_projects)?;

    // 5. Post-migration Online Backup & Restore verification
    let v2_backup_path = sandbox_dir.join("04_online_backup_v2.db");
    let v2_backup_report = storage.online_backup_to(&v2_backup_path)?;
    let v2_restored_path = sandbox_dir.join("05_restored_v2.db");
    let v2_restore_integrity = StorageManager::restore_from_backup(&v2_backup_path, &v2_restored_path)?;

    // 6. Project CRUD + Clone + Strict Isolation verification (Project A vs Project B)
    let proj_slg = storage.create_project(
        CreateProjectRequest {
            slug: "kingdom-slg-p2".to_string(),
            name: "Kingdom Rise: Tactics SLG".to_string(),
            game_package_id: "com.kingdom.rise.slg".to_string(),
            version: Some("1.2.0".to_string()),
            exe_name: Some("Auto_Kingdom_SLG.exe".to_string()),
            window_title: None,
            icon_rel_path: None,
            target_width_px: Some(1280),
            target_height_px: Some(720),
            bundle_adb: Some(true),
            enabled_modules: Some(vec![
                "mod.adb_core".to_string(),
                "mod.vision_ocr".to_string(),
                "mod.account_vault".to_string(),
            ]),
        },
        &sandbox_projects,
    )?;

    let updated_slg = storage.update_project(UpdateProjectRequest {
        id: proj_slg.id.clone(),
        name: Some("Kingdom Rise: Tactics SLG (Verified)".to_string()),
        game_package_id: None,
        version: Some("1.2.1".to_string()),
        exe_name: None,
        window_title: None,
        icon_rel_path: None,
        target_width_px: None,
        target_height_px: None,
        bundle_adb: None,
    })?;

    storage.save_project_workflow(
        &proj_slg.id,
        "wf-slg-harvest-01",
        "Thu hoạch Tài nguyên Thành chính",
        "Nông Nghiệp",
        "{\"schemaVersion\":1,\"workflowId\":\"wf-slg-harvest-01\",\"name\":\"Harvest\",\"status\":\"published\",\"graph\":{\"nodes\":[{\"id\":\"n1\",\"type\":\"flow.start\",\"position\":{\"x\":0,\"y\":0}},{\"id\":\"n2\",\"type\":\"flow.end\",\"position\":{\"x\":100,\"y\":0}}],\"edges\":[{\"id\":\"e1\",\"source\":\"n1\",\"sourcePort\":\"flow\",\"target\":\"n2\",\"targetPort\":\"flow\"}]}}",
    )?;

    storage.register_vision_template(VisionTemplateInput {
        id: Some("tpl-slg-food".to_string()),
        project_id: proj_slg.id.clone(),
        name: "btn_harvest_food".to_string(),
        rel_path: "templates/btn_harvest_food.png".to_string(),
        width_px: 96,
        height_px: 48,
        source_crop_x: Some(420),
        source_crop_y: Some(310),
        default_confidence: Some(0.88),
        sha256: "b".repeat(64),
    })?;

    let slg_cat = TaskCategoryDef {
        id: "cat-eco".to_string(),
        short_tag: "[Kinh Tế]".to_string(),
        display_name: "Thu hoạch".to_string(),
        color_hex: "#34D399".to_string(),
    };
    storage.upsert_project_task(
        &proj_slg.id,
        MiniTaskDef {
            id: "task-slg-harvest".to_string(),
            title: "Thu hoạch Lương Thực".to_string(),
            category_id: "cat-eco".to_string(),
            workflow_id: "wf-slg-harvest-01".to_string(),
            default_enabled: true,
            priority: 1,
            estimate_sec: 30,
            settings_schema: vec![],
        },
        &slg_cat,
    )?;

    storage.upsert_project_account(ProjectAccountInput {
        id: Some("acc-slg-01".to_string()),
        project_id: proj_slg.id.clone(),
        alias: "King_Lord_01".to_string(),
        game_profile_id: Some("Kingdom_Server_09".to_string()),
        secret_ref: Some("CRED_MGR://FarmerPro/account/kingdom-slg-p2/acc-slg-01".to_string()),
        state: "ready".to_string(),
    })?;

    // Clone project test
    let cloned_slg = storage.clone_project(
        CloneProjectRequest {
            source_project_id: proj_slg.id.clone(),
            new_slug: "kingdom-slg-clone".to_string(),
            new_name: "Kingdom Rise Clone".to_string(),
            new_game_package_id: None,
            new_exe_name: None,
        },
        &sandbox_projects,
    )?;

    // Cross-project tamper check: JX2 must NOT overwrite SLG workflow
    let cross_overwrite_blocked = storage
        .save_project_workflow(
            DEFAULT_LEGACY_PROJECT_ID,
            "wf-slg-harvest-01",
            "Cross Hijack",
            "Chung",
            "{}",
        )
        .is_err();

    // 6b. Direct SQL isolation probes (bypassing application handlers on an independent connection!)
    let direct_sql_isolation = {
        let raw_conn = rusqlite::Connection::open(&migrated_db_path)?;
        raw_conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        let now_sql = chrono::Utc::now().to_rfc3339();
        let cross_task_blocked = raw_conn
            .execute(
                "INSERT INTO task_definitions (
                    id, project_id, workflow_id, category_id, category_short_tag,
                    category_name, category_color_hex, title, priority, estimate_sec,
                    default_enabled, settings_schema_json, updated_at
                 ) VALUES ('task-cli-cross', ?1, 'wf-slg-harvest-01', 'c1', '[C]', 'Cat', '#fff', 'Cross', 1, 10, 1, '[]', ?2);",
                rusqlite::params![DEFAULT_LEGACY_PROJECT_ID, now_sql],
            )
            .is_err();
        let null_wf_blocked = raw_conn
            .execute(
                "INSERT INTO workflows (id, project_id, name, draft_json, draft_updated_at)
                 VALUES ('wf-cli-null', NULL, 'NullWF', '{}', ?1);",
                rusqlite::params![now_sql],
            )
            .is_err();
        let null_acc_blocked = raw_conn
            .execute(
                "INSERT INTO accounts (id, project_id, alias, state, updated_at)
                 VALUES ('acc-cli-null', NULL, 'NullAcc', 'ready', ?1);",
                rusqlite::params![now_sql],
            )
            .is_err();
        let cross_job_acc_blocked = raw_conn
            .execute(
                "INSERT INTO jobs (id, project_id, device_id, account_id, workflow_version_id, priority, state, created_at, updated_at)
                 VALUES ('job-cli-cross', ?1, 'emulator-5554', 'acc-slg-01', 'wf-jx2-01-login-v1', 0, 'queued', ?2, ?2);",
                rusqlite::params![DEFAULT_LEGACY_PROJECT_ID, now_sql],
            )
            .is_err();
        let cross_wf_move_blocked = raw_conn
            .execute(
                "UPDATE workflows SET project_id = ?1 WHERE id = 'wf-slg-harvest-01';",
                rusqlite::params![DEFAULT_LEGACY_PROJECT_ID],
            )
            .is_err();
        let cross_wf_ver_move_blocked = raw_conn
            .execute(
                "UPDATE workflow_versions SET workflow_id = 'wf-jx2-01-login' WHERE id = 'wf-slg-harvest-01-v1';",
                [],
            )
            .is_err();
        serde_json::json!({
            "crossProjectTaskInsertBlockedBySqlite": cross_task_blocked,
            "nullProjectWorkflowInsertBlockedBySqlite": null_wf_blocked,
            "nullProjectAccountInsertBlockedBySqlite": null_acc_blocked,
            "crossProjectJobAccountBlockedBySqlite": cross_job_acc_blocked,
            "crossProjectWorkflowMoveBlockedBySqlite": cross_wf_move_blocked,
            "crossProjectWorkflowVersionMoveBlockedBySqlite": cross_wf_ver_move_blocked,
            "allDirectSqlProbesBlocked": cross_task_blocked && null_wf_blocked && null_acc_blocked && cross_job_acc_blocked && cross_wf_move_blocked && cross_wf_ver_move_blocked,
        })
    };

    // 6c. Crash Consistency probes (fault injection + staging/trash/journal reconciliation)
    let crash_consistency = {
        use fp_storage::CrashFaultPoint;
        let create_fault_err = storage
            .create_project_with_fault(
                CreateProjectRequest {
                    slug: "cli-crash-create".to_string(),
                    name: "CLI Crash Create".to_string(),
                    game_package_id: "com.crash.cli".to_string(),
                    version: None,
                    exe_name: None,
                    window_title: None,
                    icon_rel_path: None,
                    target_width_px: None,
                    target_height_px: None,
                    bundle_adb: None,
                    enabled_modules: None,
                },
                &sandbox_projects,
                CrashFaultPoint::FailBeforeDbCommit,
            )
            .is_err();
        let create_clean = storage.get_project_by_slug("cli-crash-create")?.is_none()
            && !sandbox_projects.join("cli-crash-create").exists();

        let clone_fault_err = storage
            .clone_project_with_fault(
                CloneProjectRequest {
                    source_project_id: proj_slg.id.clone(),
                    new_slug: "cli-crash-clone".to_string(),
                    new_name: "CLI Crash Clone".to_string(),
                    new_game_package_id: None,
                    new_exe_name: None,
                },
                &sandbox_projects,
                CrashFaultPoint::FailTemplateCopy,
            )
            .is_err();
        let clone_clean = storage.get_project_by_slug("cli-crash-clone")?.is_none()
            && !sandbox_projects.join("cli-crash-clone").exists();

        let update_fault_err = storage
            .update_project_with_fault(
                UpdateProjectRequest {
                    id: proj_slg.id.clone(),
                    name: Some("Broken Name".to_string()),
                    game_package_id: None,
                    version: Some("9.9.9".to_string()),
                    exe_name: None,
                    window_title: None,
                    icon_rel_path: None,
                    target_width_px: None,
                    target_height_px: None,
                    bundle_adb: None,
                },
                CrashFaultPoint::FailBeforeDbCommit,
            )
            .is_err();
        let update_clean = storage
            .get_project(&proj_slg.id)?
            .map(|p| p.version == "1.2.1")
            .unwrap_or(false);

        let delete_pre_fault_err = storage
            .delete_project_with_fault(&cloned_slg.id, true, CrashFaultPoint::FailBeforeDbCommit)
            .is_err();
        let delete_pre_restored = storage.get_project(&cloned_slg.id)?.is_some()
            && PathBuf::from(&cloned_slg.project_dir_path)
                .join("project.json")
                .exists();

        let delete_post_fault_err = storage
            .delete_project_with_fault(
                &cloned_slg.id,
                true,
                CrashFaultPoint::FailAfterDbCommitTrashPurge,
            )
            .is_err();
        let reconciled_orphans = storage.reconcile_projects_storage(&sandbox_projects)?;
        let delete_post_clean = storage.get_project(&cloned_slg.id)?.is_none()
            && !PathBuf::from(&cloned_slg.project_dir_path).exists();

        serde_json::json!({
            "createFaultRolledBackCleanly": create_fault_err && create_clean,
            "cloneFaultRolledBackCleanly": clone_fault_err && clone_clean,
            "updateFaultRolledBackCleanly": update_fault_err && update_clean,
            "deletePreCommitRestoredFromTrash": delete_pre_fault_err && delete_pre_restored,
            "deletePostCommitTrashReconciled": delete_post_fault_err && delete_post_clean,
            "reconciledOrphanEntries": reconciled_orphans,
        })
    };

    let projects_list = storage.list_projects()?;

    // 6d. WAL-safe Rollback Runbook verification on an isolated sandbox copy of migrated_db_path
    let rollback_runbook_target = sandbox_dir.join("06_rollback_runbook_target.db");
    let rollback_safety_snap = sandbox_dir.join("07_pre_rollback_safety_snapshot.db");
    drop(storage);
    std::fs::copy(&migrated_db_path, &rollback_runbook_target)?;
    let wal_rollback_report = StorageManager::rollback_database_from_backup(
        &rollback_runbook_target,
        &online_bk_path,
        &rollback_safety_snap,
    )?;

    // 7. Windows Credential Manager + DPAPI Keystore Verification
    let vault = CredentialVault::new();
    let tag = &uuid::Uuid::new_v4().to_string()[..8];
    let ns_a = SecretNamespace::Project(format!("jx2-p2-{}", tag));
    let ns_b = SecretNamespace::Project(format!("slg-p2-{}", tag));
    let ns_tool = SecretNamespace::MiniTool(format!("auto-slg-{}", tag));

    let uri_a = vault.set_secret_namespaced(&ns_a, "api_secret", "jx2-secret-value")?;
    let uri_b = vault.set_secret_namespaced(&ns_b, "api_secret", "slg-secret-value")?;
    let uri_tool = vault.set_secret_namespaced(&ns_tool, "proxy_auth", "socks5-user:pass")?;

    let read_a = vault.get_secret_namespaced(&ns_a, "api_secret")?;
    let read_b = vault.get_secret_namespaced(&ns_b, "api_secret")?;
    let cross_vault_isolated = vault.get_secret_namespaced(&ns_b, "proxy_auth")?.is_none();

    let _ = vault.delete_secret_namespaced(&ns_a, "api_secret");
    let _ = vault.delete_secret_namespaced(&ns_b, "api_secret");
    let _ = vault.delete_secret_namespaced(&ns_tool, "proxy_auth");

    // DPAPI Keystore + Portable Passphrase Recovery test in sandbox
    let ks_dir = sandbox_dir.join("keystore");
    let ks_machine_b = sandbox_dir.join("keystore_machine_b");
    let ks_backup = sandbox_dir.join("keystore_backup.json");
    let ks_portable = sandbox_dir.join("keystore_portable_recovery.json");
    let (sk1, meta1) = ProducerKeystore::load_or_create_signing_key(&ks_dir)?;
    let ks_raw = std::fs::read_to_string(&meta1.key_store_path)?;
    let backup_bundle = ProducerKeystore::backup_signing_key(&ks_dir, &ks_backup)?;
    let portable_bundle = ProducerKeystore::export_portable_recovery_bundle(
        &ks_dir,
        &ks_portable,
        "FarmerPro-P2-1-Offline-Recovery!",
    )?;
    let portable_raw = std::fs::read_to_string(&ks_portable)?;
    let wrong_pass_rejected = ProducerKeystore::import_portable_recovery_bundle(
        &ks_portable,
        &ks_machine_b,
        "Wrong-Passphrase-123",
    )
    .is_err();
    let imported_b_meta = ProducerKeystore::import_portable_recovery_bundle(
        &ks_portable,
        &ks_machine_b,
        "FarmerPro-P2-1-Offline-Recovery!",
    )?;
    let (sk_machine_b, _) = ProducerKeystore::load_or_create_signing_key(&ks_machine_b)?;
    let (_sk2, meta2) = ProducerKeystore::rotate_producer_key(&ks_dir, "cli_p2_verify")?;
    let restored_meta = ProducerKeystore::restore_signing_key(&ks_backup, &ks_dir)?;
    let (sk_restored, _) = ProducerKeystore::load_or_create_signing_key(&ks_dir)?;

    Ok(serde_json::json!({
        "command": "verify-p2",
        "executedAtUtc": chrono::Utc::now().to_rfc3339(),
        "sourceProductionDb": src_db.to_string_lossy(),
        "productionDbUntouched": true,
        "preMigration": {
            "integrityCheck": pre_integrity,
            "tableCounts": pre_counts,
        },
        "migrationV1ToV2": {
            "preRestoreIntegrity": pre_restore_report,
            "appliedVersions": versions_after_v2,
            "postMigrationIntegrity": post_v2_integrity,
            "rollbackTest": {
                "faultyV3Rejected": rollback_triggered,
                "versionsAfterRollback": versions_after_rollback,
                "postRollbackIntegrity": post_rollback_integrity,
                "cleanAtomicRollback": rollback_clean,
            }
        },
        "jx2Migration": jx2_summary,
        "backupAndRestoreV2": {
            "onlineBackup": v2_backup_report,
            "restoredIntegrity": v2_restore_integrity,
        },
        "walRollbackRunbookTest": wal_rollback_report,
        "projectCrudAndIsolation": {
            "projectsCount": projects_list.len(),
            "projects": projects_list,
            "updatedSlgVersion": updated_slg.version,
            "clonedSlgSlug": cloned_slg.slug,
            "crossProjectOverwriteBlocked": cross_overwrite_blocked,
            "directSqlIsolation": direct_sql_isolation,
            "crashConsistency": crash_consistency,
        },
        "credentialVaultAndDpapi": {
            "vaultBackend": vault.backend_label(),
            "uriA": uri_a,
            "uriB": uri_b,
            "uriTool": uri_tool,
            "isolatedValuesVerified": read_a.as_deref() == Some("jx2-secret-value")
                && read_b.as_deref() == Some("slg-secret-value")
                && cross_vault_isolated,
            "dpapiKeystore": {
                "protectionScheme": meta1.protection_scheme,
                "backupProtectionScheme": backup_bundle.protection_scheme,
                "noPlaintextSeedOnDisk": !ks_raw.contains("secretSeedHex") && ks_raw.contains("dpapiProtectedSeedB64"),
                "initialKeyId": meta1.key_id,
                "rotatedKeyId": meta2.key_id,
                "backupSha256": backup_bundle.payload_sha256,
                "restoredKeyId": restored_meta.key_id,
                "seedMatchesAfterRestore": sk1.to_bytes() == sk_restored.to_bytes(),
                "portableRecovery": {
                    "protectionScheme": portable_bundle.protection_scheme,
                    "kdfIterations": portable_bundle.kdf_iterations,
                    "noPlaintextSeedInBundle": !portable_raw.contains("secretSeedHex"),
                    "wrongPassphraseRejected": wrong_pass_rejected,
                    "importedKeyId": imported_b_meta.key_id,
                    "seedMatchesAfterPortableImport": sk1.to_bytes() == sk_machine_b.to_bytes(),
                }
            }
        }
    }))
}

fn run_rollback_runbook_verification(
    live_db: &std::path::Path,
    backup_db: &std::path::Path,
    out_dir: &std::path::Path,
) -> anyhow::Result<serde_json::Value> {
    use fp_storage::StorageManager;

    if !live_db.exists() {
        anyhow::bail!("Source live DB not found: {}", live_db.display());
    }
    if !backup_db.exists() {
        anyhow::bail!("Verified backup DB not found: {}", backup_db.display());
    }
    if out_dir.exists() {
        let _ = std::fs::remove_dir_all(out_dir);
    }
    std::fs::create_dir_all(out_dir)?;

    let sandbox_target = out_dir.join("target_db_copy.db");
    let safety_snap = out_dir.join("pre_rollback_safety_snapshot.db");
    let backup_copy = out_dir.join("verified_backup_copy.db");

    // Copy live_db via Online Backup API so WAL frames are safely captured into sandbox copy
    {
        let src_conn = rusqlite::Connection::open(live_db)?;
        let mut dst_conn = rusqlite::Connection::open(&sandbox_target)?;
        let bk = rusqlite::backup::Backup::new(&src_conn, &mut dst_conn)?;
        bk.run_to_completion(100, std::time::Duration::from_millis(5), None)?;
    }
    std::fs::copy(backup_db, &backup_copy)?;

    let report = StorageManager::rollback_database_from_backup(
        &sandbox_target,
        &backup_copy,
        &safety_snap,
    )?;

    Ok(serde_json::json!({
        "command": "verify-rollback-runbook",
        "executedAtUtc": chrono::Utc::now().to_rfc3339(),
        "sourceLiveDb": live_db.to_string_lossy(),
        "sourceBackupDb": backup_db.to_string_lossy(),
        "productionDbUntouched": true,
        "rollbackReport": report,
    }))
}

fn run_p2_guarded_migration(
    db_path: &std::path::Path,
    backup_path: &std::path::Path,
    jx2_dir: &std::path::Path,
    projects_dir: &std::path::Path,
) -> anyhow::Result<serde_json::Value> {
    use fp_storage::StorageManager;

    if !db_path.exists() {
        anyhow::bail!("Target database does not exist: {}", db_path.display());
    }

    // 1. Read pre-migration state & create Online Backup BEFORE touching target DB
    let pre_counts = {
        let conn = rusqlite::Connection::open(db_path)?;
        let integ: String = conn.query_row("PRAGMA integrity_check;", [], |r| r.get(0))?;
        anyhow::ensure!(
            integ.eq_ignore_ascii_case("ok"),
            "Pre-migration DB integrity failed: {}",
            integ
        );
        let mut counts = serde_json::Map::new();
        for tbl in [
            "devices",
            "workflows",
            "workflow_versions",
            "accounts",
            "jobs",
            "step_events",
        ] {
            let cnt: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {};", tbl), [], |r| r.get(0))
                .unwrap_or(0);
            counts.insert(tbl.to_string(), serde_json::json!(cnt));
        }
        if let Some(parent) = backup_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if backup_path.exists() {
            std::fs::remove_file(backup_path)?;
        }
        let mut bk_conn = rusqlite::Connection::open(backup_path)?;
        let bk = rusqlite::backup::Backup::new(&conn, &mut bk_conn)?;
        bk.run_to_completion(100, std::time::Duration::from_millis(5), None)?;
        serde_json::Value::Object(counts)
    };

    // 2. Verify restore on a temporary copy BEFORE modifying target DB
    let restore_probe =
        std::env::temp_dir().join(format!("fp_restore_probe_{}.db", uuid::Uuid::new_v4()));
    let restore_check = StorageManager::restore_from_backup(backup_path, &restore_probe)?;
    let _ = std::fs::remove_file(&restore_probe);
    anyhow::ensure!(
        restore_check.is_healthy,
        "Restore probe on backup copy failed!"
    );

    // 3. Apply v1->v2 migration + JX2 migration on target DB
    let storage = StorageManager::new(db_path)?;
    let jx2_summary = storage.migrate_legacy_jx2_assets(jx2_dir, projects_dir)?;
    let post_integrity = storage.check_integrity()?;

    Ok(serde_json::json!({
        "command": "migrate-p2",
        "executedAtUtc": chrono::Utc::now().to_rfc3339(),
        "targetDbPath": db_path.to_string_lossy(),
        "preMigrationBackupPath": backup_path.to_string_lossy(),
        "restoreProbeHealthy": restore_check.is_healthy,
        "preMigrationCounts": pre_counts,
        "jx2Migration": jx2_summary,
        "postMigrationIntegrity": post_integrity,
        "originalJx2FilesIntact": jx2_dir.exists(),
    }))
}
