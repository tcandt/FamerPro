use fp_bundle_contract::{MiniTaskDef, TaskCategoryDef};
use fp_storage::{
    fs_atomic::{JOURNAL_DIR_NAME, STAGING_DIR_NAME, TRASH_DIR_NAME},
    CloneProjectRequest, CreateProjectRequest, ProjectAccountInput, StorageManager,
    UpdateProjectRequest, VisionTemplateInput,
};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

fn seed_sandbox_with_victim_and_target(
    db_path: &Path,
    projects_root: &Path,
) -> (String, String, Vec<u8>, String, Vec<u8>) {
    let storage = StorageManager::new(db_path).unwrap();

    // 1. Seed completely unrelated Victim project with workflow, template, task, account
    let victim = storage
        .create_project(
            CreateProjectRequest {
                slug: "victim-sentinel".to_string(),
                name: "Victim Sentinel Game".to_string(),
                game_package_id: "com.victim.sentinel".to_string(),
                version: Some("2.4.0".to_string()),
                exe_name: None,
                window_title: None,
                icon_rel_path: None,
                target_width_px: Some(1280),
                target_height_px: Some(720),
                bundle_adb: Some(true),
                enabled_modules: None,
            },
            projects_root,
        )
        .unwrap();

    let victim_wf_json = r#"{"schemaVersion":1,"workflowId":"wf-victim-01","name":"Victim Guard","status":"published","graph":{"nodes":[{"id":"v-start","type":"flow.start","position":{"x":0,"y":0}},{"id":"v-end","type":"flow.end","position":{"x":120,"y":0}}],"edges":[{"id":"ve1","source":"v-start","sourcePort":"flow","target":"v-end","targetPort":"flow"}]}}"#;
    storage
        .save_project_workflow(
            &victim.id,
            "wf-victim-01",
            "Victim Guard Workflow",
            "Bảo Vệ",
            victim_wf_json,
        )
        .unwrap();

    let victim_tpl_bytes = b"\x89PNG\r\n\x1a\nVICTIM_SENTINEL_BYTES_DO_NOT_TOUCH".to_vec();
    storage
        .save_project_template_file(
            VisionTemplateInput {
                id: Some("tpl-victim-01".to_string()),
                project_id: victim.id.clone(),
                name: "victim_marker".to_string(),
                rel_path: "templates/victim_marker.png".to_string(),
                width_px: 64,
                height_px: 64,
                source_crop_x: Some(10),
                source_crop_y: Some(20),
                default_confidence: Some(0.95),
                sha256: String::new(),
            },
            &victim_tpl_bytes,
        )
        .unwrap();

    storage
        .upsert_project_task(
            &victim.id,
            MiniTaskDef {
                id: "task-victim-01".to_string(),
                title: "Victim Sentinel Task".to_string(),
                category_id: "cat-v".to_string(),
                workflow_id: "wf-victim-01".to_string(),
                default_enabled: true,
                priority: 1,
                estimate_sec: 15,
                settings_schema: vec![],
            },
            &TaskCategoryDef {
                id: "cat-v".to_string(),
                short_tag: "[VIC]".to_string(),
                display_name: "Sentinel".to_string(),
                color_hex: "#22C55E".to_string(),
            },
        )
        .unwrap();

    storage
        .upsert_project_account(ProjectAccountInput {
            id: Some("acc-victim-01".to_string()),
            project_id: victim.id.clone(),
            alias: "Victim_Account".to_string(),
            game_profile_id: Some("Server_Victim".to_string()),
            secret_ref: Some(
                "CRED_MGR://FarmerPro/account/victim-sentinel/acc-victim-01".to_string(),
            ),
            state: "ready".to_string(),
        })
        .unwrap();

    // 2. Seed pre-existing Target project for update/workflow/delete crash tests
    let target = storage
        .create_project(
            CreateProjectRequest {
                slug: "target-crash-proj".to_string(),
                name: "Target Crash Project".to_string(),
                game_package_id: "com.target.crash".to_string(),
                version: Some("1.0.0".to_string()),
                exe_name: None,
                window_title: None,
                icon_rel_path: None,
                target_width_px: Some(1280),
                target_height_px: Some(720),
                bundle_adb: Some(true),
                enabled_modules: None,
            },
            projects_root,
        )
        .unwrap();

    let target_wf_v1 = r#"{"schemaVersion":1,"workflowId":"wf-target-01","name":"Target V1","status":"draft","graph":{"nodes":[{"id":"t-start","type":"flow.start","position":{"x":0,"y":0}},{"id":"t-end","type":"flow.end","position":{"x":120,"y":0}}],"edges":[{"id":"te1","source":"t-start","sourcePort":"flow","target":"t-end","targetPort":"flow"}]}}"#.to_string();
    storage
        .save_project_workflow(
            &target.id,
            "wf-target-01",
            "Target V1",
            "Chung",
            &target_wf_v1,
        )
        .unwrap();

    let victim_manifest =
        std::fs::read(PathBuf::from(&victim.project_dir_path).join("project.json")).unwrap();
    (
        victim.id,
        target.id,
        victim_manifest,
        victim_wf_json.to_string(),
        victim_tpl_bytes,
    )
}

fn assert_victim_intact_and_db_healthy(
    storage: &StorageManager,
    projects_root: &Path,
    victim_id: &str,
    victim_manifest_expected: &[u8],
    victim_wf_expected: &str,
    victim_tpl_expected: &[u8],
) {
    let integ = storage.check_integrity().unwrap();
    assert_eq!(integ.integrity_check.to_lowercase(), "ok");
    assert_eq!(integ.foreign_key_violations, 0);
    assert!(integ.is_healthy);

    let v_detail = storage
        .get_project_detail(victim_id)
        .unwrap()
        .expect("Victim project must still exist after crash recovery");
    assert_eq!(v_detail.project.slug, "victim-sentinel");
    assert_eq!(v_detail.project.version, "2.4.0");
    assert_eq!(v_detail.workflows.len(), 1);
    assert_eq!(v_detail.workflows[0].draft_json, victim_wf_expected);
    assert_eq!(v_detail.templates.len(), 1);
    assert_eq!(v_detail.tasks.len(), 1);
    assert_eq!(v_detail.accounts.len(), 1);

    let victim_dir = projects_root.join("victim-sentinel");
    assert_eq!(
        std::fs::read(victim_dir.join("project.json")).unwrap(),
        victim_manifest_expected
    );
    assert_eq!(
        std::fs::read_to_string(victim_dir.join("workflows").join("wf-victim-01.json")).unwrap(),
        victim_wf_expected
    );
    assert_eq!(
        std::fs::read(victim_dir.join("templates").join("victim_marker.png")).unwrap(),
        victim_tpl_expected
    );
}

fn assert_no_orphan_dirs_or_journals(projects_root: &Path) {
    for sub in [STAGING_DIR_NAME, TRASH_DIR_NAME, JOURNAL_DIR_NAME] {
        let p = projects_root.join(sub);
        if p.exists() {
            assert_eq!(
                std::fs::read_dir(&p).unwrap().count(),
                0,
                "Expected 0 orphan entries in {}",
                p.display()
            );
        }
    }
}

fn run_forced_kill_child(db_path: &Path, projects_root: &Path, stage: &str, target_id: &str) {
    let exe = std::env::current_exe().unwrap();
    let mut child = Command::new(exe)
        .args(["--exact", "forced_crash_worker_entry", "--nocapture"])
        .env("FP_FORCE_CRASH_STAGE", stage)
        .env("FP_CRASH_DB", db_path)
        .env("FP_CRASH_PROJECTS_ROOT", projects_root)
        .env("FP_CRASH_TARGET_ID", target_id)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed spawning crash worker process");

    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    let expected_banner = format!("ARMED:{}", stage);
    let mut armed = false;
    let mut line = String::new();
    while reader.read_line(&mut line).unwrap_or(0) > 0 {
        if line.trim() == expected_banner {
            armed = true;
            break;
        }
        line.clear();
    }
    assert!(
        armed,
        "Child worker never reached crash arm banner '{}'",
        expected_banner
    );

    // REAL forced OS process termination (TerminateProcess on Windows / SIGKILL on Unix)
    child.kill().expect("OS TerminateProcess failed");
    let exit_status = child.wait().expect("Failed waiting on killed child");
    assert!(
        !exit_status.success(),
        "Killed child process should not exit cleanly"
    );
}

#[test]
fn forced_crash_worker_entry() {
    let Ok(stage) = std::env::var("FP_FORCE_CRASH_STAGE") else {
        return; // Skipped when run as regular unit test without env trigger
    };
    let db_path = PathBuf::from(std::env::var("FP_CRASH_DB").unwrap());
    let projects_root = PathBuf::from(std::env::var("FP_CRASH_PROJECTS_ROOT").unwrap());
    let target_id = std::env::var("FP_CRASH_TARGET_ID").unwrap_or_default();

    let storage = StorageManager::new(&db_path).unwrap();
    let stage_target = stage.clone();
    storage.set_crash_stage_hook(Some(move |hit: &str| {
        if hit == stage_target {
            println!("ARMED:{}", hit);
            let _ = std::io::stdout().flush();
            std::thread::sleep(Duration::from_secs(60));
        }
    }));

    match stage.as_str() {
        "create:after_staging_before_rename"
        | "create:after_rename_before_db_commit"
        | "create:after_db_commit_before_cleanup" => {
            let _ = storage.create_project(
                CreateProjectRequest {
                    slug: "crashed-new-proj".to_string(),
                    name: "Crashed New Project".to_string(),
                    game_package_id: "com.crashed.new".to_string(),
                    version: Some("1.0.0".to_string()),
                    exe_name: None,
                    window_title: None,
                    icon_rel_path: None,
                    target_width_px: None,
                    target_height_px: None,
                    bundle_adb: None,
                    enabled_modules: None,
                },
                &projects_root,
            );
        }
        "workflow:after_wf_write_before_manifest"
        | "workflow:after_disk_writes_before_db_commit"
        | "workflow:after_db_commit_before_cleanup" => {
            let new_wf = r#"{"schemaVersion":1,"workflowId":"wf-target-01","name":"Target V2 Crash","status":"published","graph":{"nodes":[{"id":"t2-start","type":"flow.start","position":{"x":0,"y":0}},{"id":"t2-end","type":"flow.end","position":{"x":200,"y":0}}],"edges":[{"id":"te2","source":"t2-start","sourcePort":"flow","target":"t2-end","targetPort":"flow"}]}}"#;
            let _ = storage.save_project_workflow(
                &target_id,
                "wf-target-01",
                "Target V2 Crash",
                "Chung",
                new_wf,
            );
        }
        "update:after_disk_writes_before_db_commit" => {
            let _ = storage.update_project(UpdateProjectRequest {
                id: target_id,
                name: Some("Mutated Uncommitted Name".to_string()),
                version: Some("9.9.9".to_string()),
                ..Default::default()
            });
        }
        "delete:after_move_to_trash_before_commit" | "delete:after_db_commit_before_cleanup" => {
            let _ = storage.delete_project(&target_id, true);
        }
        "clone:after_staging_before_rename"
        | "clone:after_rename_before_db_commit"
        | "clone:after_db_commit_before_cleanup" => {
            let _ = storage.clone_project(
                CloneProjectRequest {
                    source_project_id: target_id,
                    new_slug: "deep-clone-crash".to_string(),
                    new_name: "Deep Clone Crash Target".to_string(),
                    new_game_package_id: Some("com.deep.clone.target".to_string()),
                    new_exe_name: None,
                },
                &projects_root,
            );
        }
        other => panic!("Unknown FP_FORCE_CRASH_STAGE: {}", other),
    }
}

#[test]
fn test_forced_kill_1_after_staging_before_rename() {
    let temp_dir = std::env::temp_dir().join(format!("fp_fk1_{}", uuid::Uuid::new_v4()));
    let db_path = temp_dir.join("sandbox.db");
    let projects_root = temp_dir.join("projects");
    let (victim_id, _, v_mf, v_wf, v_tpl) =
        seed_sandbox_with_victim_and_target(&db_path, &projects_root);

    run_forced_kill_child(
        &db_path,
        &projects_root,
        "create:after_staging_before_rename",
        "",
    );

    // Verify uncommitted staging & journal actually existed before startup reconcile
    assert!(
        std::fs::read_dir(projects_root.join(STAGING_DIR_NAME))
            .unwrap()
            .count()
            >= 1
    );
    assert!(
        std::fs::read_dir(projects_root.join(JOURNAL_DIR_NAME))
            .unwrap()
            .count()
            >= 1
    );

    // Restart & Reconcile
    let (reopened, rep) = StorageManager::open_and_reconcile(&db_path, &projects_root).unwrap();
    assert!(rep.total_resolved >= 1);
    assert!(reopened
        .get_project_by_slug("crashed-new-proj")
        .unwrap()
        .is_none());
    assert!(!projects_root.join("crashed-new-proj").exists());
    assert_no_orphan_dirs_or_journals(&projects_root);
    assert_victim_intact_and_db_healthy(
        &reopened,
        &projects_root,
        &victim_id,
        &v_mf,
        &v_wf,
        &v_tpl,
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_forced_kill_2_after_rename_before_db_commit() {
    let temp_dir = std::env::temp_dir().join(format!("fp_fk2_{}", uuid::Uuid::new_v4()));
    let db_path = temp_dir.join("sandbox.db");
    let projects_root = temp_dir.join("projects");
    let (victim_id, _, v_mf, v_wf, v_tpl) =
        seed_sandbox_with_victim_and_target(&db_path, &projects_root);

    run_forced_kill_child(
        &db_path,
        &projects_root,
        "create:after_rename_before_db_commit",
        "",
    );

    // Right after OS kill: directory was ALREADY renamed into projects/, while SQLite never committed!
    assert!(projects_root.join("crashed-new-proj").exists());

    // Restart & Reconcile MUST detect uncommitted create journal and remove orphan directory!
    let (reopened, rep) = StorageManager::open_and_reconcile(&db_path, &projects_root).unwrap();
    assert_eq!(rep.uncommitted_dirs_reverted, 1);
    assert!(reopened
        .get_project_by_slug("crashed-new-proj")
        .unwrap()
        .is_none());
    assert!(!projects_root.join("crashed-new-proj").exists());
    assert_no_orphan_dirs_or_journals(&projects_root);
    assert_victim_intact_and_db_healthy(
        &reopened,
        &projects_root,
        &victim_id,
        &v_mf,
        &v_wf,
        &v_tpl,
    );

    // Retrying create_project with the SAME slug MUST now succeed instead of being blocked!
    let retried = reopened
        .create_project(
            CreateProjectRequest {
                slug: "crashed-new-proj".to_string(),
                name: "Recovered Retry Create".to_string(),
                game_package_id: "com.retry.ok".to_string(),
                version: None,
                exe_name: None,
                window_title: None,
                icon_rel_path: None,
                target_width_px: None,
                target_height_px: None,
                bundle_adb: None,
                enabled_modules: None,
            },
            &projects_root,
        )
        .unwrap();
    assert_eq!(retried.slug, "crashed-new-proj");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_forced_kill_3_after_db_commit_before_cleanup() {
    let temp_dir = std::env::temp_dir().join(format!("fp_fk3_{}", uuid::Uuid::new_v4()));
    let db_path = temp_dir.join("sandbox.db");
    let projects_root = temp_dir.join("projects");
    let (victim_id, _, v_mf, v_wf, v_tpl) =
        seed_sandbox_with_victim_and_target(&db_path, &projects_root);

    run_forced_kill_child(
        &db_path,
        &projects_root,
        "create:after_db_commit_before_cleanup",
        "",
    );

    // Journal still lingering before restart
    assert!(
        std::fs::read_dir(projects_root.join(JOURNAL_DIR_NAME))
            .unwrap()
            .count()
            >= 1
    );

    let (reopened, rep) = StorageManager::open_and_reconcile(&db_path, &projects_root).unwrap();
    assert_eq!(rep.creates_finalized, 1);
    assert!(reopened
        .get_project_by_slug("crashed-new-proj")
        .unwrap()
        .is_some());
    assert!(projects_root
        .join("crashed-new-proj")
        .join("project.json")
        .exists());
    assert_no_orphan_dirs_or_journals(&projects_root);
    assert_victim_intact_and_db_healthy(
        &reopened,
        &projects_root,
        &victim_id,
        &v_mf,
        &v_wf,
        &v_tpl,
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_forced_kill_4_manifest_and_workflow_updates_pre_and_post_commit() {
    let temp_dir = std::env::temp_dir().join(format!("fp_fk4_{}", uuid::Uuid::new_v4()));
    let db_path = temp_dir.join("sandbox.db");
    let projects_root = temp_dir.join("projects");
    let (victim_id, target_id, v_mf, v_wf, v_tpl) =
        seed_sandbox_with_victim_and_target(&db_path, &projects_root);

    let target_dir = projects_root.join("target-crash-proj");
    let wf_disk_path = target_dir.join("workflows").join("wf-target-01.json");
    let mf_disk_path = target_dir.join("project.json");
    let pre_wf_bytes = std::fs::read(&wf_disk_path).unwrap();
    let pre_mf_bytes = std::fs::read(&mf_disk_path).unwrap();

    // 4a. Kill AFTER overwriting workflow file on disk, BEFORE manifest/DB commit
    run_forced_kill_child(
        &db_path,
        &projects_root,
        "workflow:after_wf_write_before_manifest",
        &target_id,
    );
    assert_ne!(std::fs::read(&wf_disk_path).unwrap(), pre_wf_bytes); // Disk diverged before restart!

    let (reopened_a, rep_a) = StorageManager::open_and_reconcile(&db_path, &projects_root).unwrap();
    assert_eq!(rep_a.mutations_rolled_back, 1);
    assert_eq!(std::fs::read(&wf_disk_path).unwrap(), pre_wf_bytes);
    assert_eq!(std::fs::read(&mf_disk_path).unwrap(), pre_mf_bytes);
    let wf_rec = reopened_a.list_project_workflows(&target_id).unwrap();
    assert!(wf_rec[0].draft_json.contains("Target V1"));
    assert_eq!(wf_rec[0].latest_version_no, Some(1));
    assert_no_orphan_dirs_or_journals(&projects_root);
    assert_victim_intact_and_db_healthy(
        &reopened_a,
        &projects_root,
        &victim_id,
        &v_mf,
        &v_wf,
        &v_tpl,
    );
    drop(reopened_a);

    // 4b. Kill AFTER update_project overwrote project.json & ui-schema.json, BEFORE DB commit
    run_forced_kill_child(
        &db_path,
        &projects_root,
        "update:after_disk_writes_before_db_commit",
        &target_id,
    );
    assert_ne!(std::fs::read(&mf_disk_path).unwrap(), pre_mf_bytes);

    let (reopened_b, rep_b) = StorageManager::open_and_reconcile(&db_path, &projects_root).unwrap();
    assert_eq!(rep_b.mutations_rolled_back, 1);
    assert_eq!(std::fs::read(&mf_disk_path).unwrap(), pre_mf_bytes);
    assert_eq!(
        reopened_b.get_project(&target_id).unwrap().unwrap().version,
        "1.0.0"
    );
    assert_no_orphan_dirs_or_journals(&projects_root);
    assert_victim_intact_and_db_healthy(
        &reopened_b,
        &projects_root,
        &victim_id,
        &v_mf,
        &v_wf,
        &v_tpl,
    );
    drop(reopened_b);

    // 4c. Kill AFTER workflow DB commit, BEFORE journal/backup cleanup
    run_forced_kill_child(
        &db_path,
        &projects_root,
        "workflow:after_db_commit_before_cleanup",
        &target_id,
    );
    let (reopened_c, rep_c) = StorageManager::open_and_reconcile(&db_path, &projects_root).unwrap();
    assert_eq!(rep_c.mutations_finalized, 1);
    let wf_rec_c = reopened_c.list_project_workflows(&target_id).unwrap();
    assert!(wf_rec_c[0].draft_json.contains("Target V2 Crash"));
    assert_eq!(wf_rec_c[0].latest_version_no, Some(2));
    assert!(std::fs::read_to_string(&wf_disk_path)
        .unwrap()
        .contains("Target V2 Crash"));
    assert_no_orphan_dirs_or_journals(&projects_root);
    assert_victim_intact_and_db_healthy(
        &reopened_c,
        &projects_root,
        &victim_id,
        &v_mf,
        &v_wf,
        &v_tpl,
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_forced_kill_5_after_delete_to_trash_before_commit() {
    let temp_dir = std::env::temp_dir().join(format!("fp_fk5_{}", uuid::Uuid::new_v4()));
    let db_path = temp_dir.join("sandbox.db");
    let projects_root = temp_dir.join("projects");
    let (victim_id, target_id, v_mf, v_wf, v_tpl) =
        seed_sandbox_with_victim_and_target(&db_path, &projects_root);

    let target_dir = projects_root.join("target-crash-proj");
    let pre_mf_bytes = std::fs::read(target_dir.join("project.json")).unwrap();
    let pre_wf_bytes =
        std::fs::read(target_dir.join("workflows").join("wf-target-01.json")).unwrap();

    run_forced_kill_child(
        &db_path,
        &projects_root,
        "delete:after_move_to_trash_before_commit",
        &target_id,
    );

    // Before reconcile: target dir was moved to .fp_trash, so live dir is missing!
    assert!(!target_dir.exists());
    assert!(
        std::fs::read_dir(projects_root.join(TRASH_DIR_NAME))
            .unwrap()
            .count()
            >= 1
    );

    // Restart & Reconcile MUST restore target directory completely from .fp_trash!
    let (reopened, rep) = StorageManager::open_and_reconcile(&db_path, &projects_root).unwrap();
    assert_eq!(rep.deletes_restored, 1);
    assert!(reopened.get_project(&target_id).unwrap().is_some());
    assert_eq!(
        std::fs::read(target_dir.join("project.json")).unwrap(),
        pre_mf_bytes
    );
    assert_eq!(
        std::fs::read(target_dir.join("workflows").join("wf-target-01.json")).unwrap(),
        pre_wf_bytes
    );
    assert_no_orphan_dirs_or_journals(&projects_root);
    assert_victim_intact_and_db_healthy(
        &reopened,
        &projects_root,
        &victim_id,
        &v_mf,
        &v_wf,
        &v_tpl,
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_forced_kill_6_after_delete_commit_before_cleanup() {
    let temp_dir = std::env::temp_dir().join(format!("fp_fk6_{}", uuid::Uuid::new_v4()));
    let db_path = temp_dir.join("sandbox.db");
    let projects_root = temp_dir.join("projects");
    let (victim_id, target_id, v_mf, v_wf, v_tpl) =
        seed_sandbox_with_victim_and_target(&db_path, &projects_root);

    run_forced_kill_child(
        &db_path,
        &projects_root,
        "delete:after_db_commit_before_cleanup",
        &target_id,
    );

    // Trash & delete journal left behind on crash right after commit
    assert!(
        std::fs::read_dir(projects_root.join(TRASH_DIR_NAME))
            .unwrap()
            .count()
            >= 1
    );
    assert!(
        std::fs::read_dir(projects_root.join(JOURNAL_DIR_NAME))
            .unwrap()
            .count()
            >= 1
    );

    let (reopened, rep) = StorageManager::open_and_reconcile(&db_path, &projects_root).unwrap();
    assert_eq!(rep.deletes_finalized, 1);
    assert!(reopened.get_project(&target_id).unwrap().is_none());
    assert!(!projects_root.join("target-crash-proj").exists());
    assert_no_orphan_dirs_or_journals(&projects_root);
    assert_victim_intact_and_db_healthy(
        &reopened,
        &projects_root,
        &victim_id,
        &v_mf,
        &v_wf,
        &v_tpl,
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_forced_kill_7_deep_clone_multi_asset_pre_and_post_commit() {
    let temp_dir = std::env::temp_dir().join(format!("fp_fk7_deep_clone_{}", uuid::Uuid::new_v4()));
    let db_path = temp_dir.join("sandbox.db");
    let projects_root = temp_dir.join("projects");
    let (victim_id, target_id, v_mf, v_wf, v_tpl) =
        seed_sandbox_with_victim_and_target(&db_path, &projects_root);

    // Enrich `target_id` with 10 workflows, 8 PNG templates (including nested subdirs), 3 tasks, accounts & branding asset
    {
        let storage = StorageManager::new(&db_path).unwrap();
        for i in 2..=10 {
            let wf_id = format!("wf-deep-{:02}", i);
            let wf_json = format!(
                r#"{{"schemaVersion":1,"workflowId":"{}","name":"Deep Flow {}","status":"published","graph":{{"nodes":[{{"id":"s{}","type":"flow.start","position":{{"x":0,"y":0}}}},{{"id":"e{}","type":"flow.end","position":{{"x":100,"y":0}}}}],"edges":[{{"id":"ed{}","source":"s{}","sourcePort":"flow","target":"e{}","targetPort":"flow"}}]}}}}"#,
                wf_id, i, i, i, i, i, i
            );
            storage
                .save_project_workflow(
                    &target_id,
                    &wf_id,
                    &format!("Deep Flow {}", i),
                    "Deep",
                    &wf_json,
                )
                .unwrap();
        }
        for i in 1..=8 {
            let rel = if i <= 4 {
                format!("templates/btn_{:02}.png", i)
            } else {
                format!("templates/nested/icon_{:02}.png", i)
            };
            storage
                .save_project_template_file(
                    VisionTemplateInput {
                        id: Some(format!("tpl-deep-{:02}", i)),
                        project_id: target_id.clone(),
                        name: format!("deep_tpl_{}", i),
                        rel_path: rel,
                        width_px: 48,
                        height_px: 48,
                        source_crop_x: Some(i * 10),
                        source_crop_y: Some(i * 12),
                        default_confidence: Some(0.9),
                        sha256: String::new(),
                    },
                    format!("FAKEPNG_ASSET_PAYLOAD_{:02}", i).as_bytes(),
                )
                .unwrap();
        }
        for i in 1..=3 {
            storage
                .upsert_project_task(
                    &target_id,
                    MiniTaskDef {
                        id: format!("task-deep-{:02}", i),
                        title: format!("Deep Task {}", i),
                        category_id: "cat-deep".to_string(),
                        workflow_id: if i == 1 {
                            "wf-target-01".to_string()
                        } else {
                            format!("wf-deep-{:02}", i)
                        },
                        default_enabled: i == 1,
                        priority: i as i32,
                        estimate_sec: 30,
                        settings_schema: vec![],
                    },
                    &TaskCategoryDef {
                        id: "cat-deep".to_string(),
                        short_tag: "[DEEP]".to_string(),
                        display_name: "Deep Assets".to_string(),
                        color_hex: "#38BDF8".to_string(),
                    },
                )
                .unwrap();
        }
        std::fs::write(
            projects_root
                .join("target-crash-proj")
                .join("branding")
                .join("icon.ico"),
            b"FAKE-ICO-BRANDING-DEEP-CLONE",
        )
        .unwrap();
    }

    // 7a. Kill mid-staging before rename (multi-asset clone aborted cleanly)
    run_forced_kill_child(
        &db_path,
        &projects_root,
        "clone:after_staging_before_rename",
        &target_id,
    );
    let (reopened_a, _rep_a) =
        StorageManager::open_and_reconcile(&db_path, &projects_root).unwrap();
    assert!(reopened_a
        .get_project_by_slug("deep-clone-crash")
        .unwrap()
        .is_none());
    assert!(!projects_root.join("deep-clone-crash").exists());
    assert_no_orphan_dirs_or_journals(&projects_root);
    assert_victim_intact_and_db_healthy(
        &reopened_a,
        &projects_root,
        &victim_id,
        &v_mf,
        &v_wf,
        &v_tpl,
    );
    drop(reopened_a);

    // 7b. Kill AFTER rename of deep-cloned dir -> projects/deep-clone-crash BEFORE SQLite commit!
    run_forced_kill_child(
        &db_path,
        &projects_root,
        "clone:after_rename_before_db_commit",
        &target_id,
    );
    assert!(projects_root.join("deep-clone-crash").exists());
    let (reopened_b, rep_b) = StorageManager::open_and_reconcile(&db_path, &projects_root).unwrap();
    assert_eq!(rep_b.uncommitted_dirs_reverted, 1);
    assert!(reopened_b
        .get_project_by_slug("deep-clone-crash")
        .unwrap()
        .is_none());
    assert!(!projects_root.join("deep-clone-crash").exists());
    assert_no_orphan_dirs_or_journals(&projects_root);
    assert_victim_intact_and_db_healthy(
        &reopened_b,
        &projects_root,
        &victim_id,
        &v_mf,
        &v_wf,
        &v_tpl,
    );
    assert_eq!(
        reopened_b.list_project_workflows(&target_id).unwrap().len(),
        10
    );
    assert_eq!(
        reopened_b.list_vision_templates(&target_id).unwrap().len(),
        8
    );
    drop(reopened_b);

    // 7c. Kill AFTER SQLite commit BEFORE cleanup -> deep clone with ALL 10 workflows, 8 nested templates, 3 tasks preserved!
    run_forced_kill_child(
        &db_path,
        &projects_root,
        "clone:after_db_commit_before_cleanup",
        &target_id,
    );
    let (reopened_c, rep_c) = StorageManager::open_and_reconcile(&db_path, &projects_root).unwrap();
    assert_eq!(rep_c.creates_finalized, 1);
    let cloned = reopened_c
        .get_project_by_slug("deep-clone-crash")
        .unwrap()
        .expect("Committed deep clone must survive post-commit crash");
    assert_eq!(
        reopened_c.list_project_workflows(&cloned.id).unwrap().len(),
        10
    );
    assert_eq!(
        reopened_c.list_vision_templates(&cloned.id).unwrap().len(),
        8
    );
    assert_eq!(reopened_c.list_project_tasks(&cloned.id).unwrap().len(), 3);
    assert!(PathBuf::from(&cloned.project_dir_path)
        .join("templates")
        .join("nested")
        .join("icon_08.png")
        .exists());
    assert!(PathBuf::from(&cloned.project_dir_path)
        .join("branding")
        .join("icon.ico")
        .exists());
    assert_no_orphan_dirs_or_journals(&projects_root);
    assert_victim_intact_and_db_healthy(
        &reopened_c,
        &projects_root,
        &victim_id,
        &v_mf,
        &v_wf,
        &v_tpl,
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}
