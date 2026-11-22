use anyhow::Result;
use fp_actions::{SafeRegionSampler, SafeTapPolicy};
use fp_adb::{classify_device_adapter, validate_adb_token, AdbRunner};
use fp_builder::MiniToolPackager;
use fp_bundle_contract::{MiniTaskDef, TaskCategoryDef};
use fp_bundle_verify::sha256_hex;
use fp_domain::{Bounds, ConnectionState, DeviceDriver, ScreenGeometry};
use fp_proxy::{probe_proxy, ProxyConfig, ProxyProtocol};
use fp_storage::{
    CheckpointRecordInput, CreateProjectRequest, ProjectAccountInput, ProjectReleaseRecord,
    StorageManager, VisionTemplateInput,
};
use fp_testkit::FakeDeviceDriver;
use fp_vision::{
    annotate_match_and_tap_png, crop_roi_png, scale_bounds_for_geometry, RoiRect, VisionEngine,
};
use fp_workflow_runtime::coordinator::{RunProjectWorkflowRequest, RuntimeCoordinator};
use fp_workflow_runtime::RuntimeCheckpointState;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use uuid::Uuid;

fn build_test_ico_32x32() -> Vec<u8> {
    let mut ico = Vec::new();
    ico.extend_from_slice(&0u16.to_le_bytes());
    ico.extend_from_slice(&1u16.to_le_bytes());
    ico.extend_from_slice(&1u16.to_le_bytes());

    let mut bmp = Vec::new();
    bmp.extend_from_slice(&40u32.to_le_bytes());
    bmp.extend_from_slice(&32i32.to_le_bytes());
    bmp.extend_from_slice(&(64i32).to_le_bytes());
    bmp.extend_from_slice(&1u16.to_le_bytes());
    bmp.extend_from_slice(&32u16.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(&(32 * 32 * 4u32).to_le_bytes());
    bmp.extend_from_slice(&0i32.to_le_bytes());
    bmp.extend_from_slice(&0i32.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    for _ in 0..(32 * 32) {
        bmp.extend_from_slice(&[245, 120, 24, 255]);
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

pub async fn run_p3_verification(repo_root: &Path, out_dir: &Path) -> Result<serde_json::Value> {
    std::fs::create_dir_all(out_dir)?;
    let sandbox = out_dir.join("sandbox_p3");
    if sandbox.exists() {
        let _ = std::fs::remove_dir_all(&sandbox);
    }
    std::fs::create_dir_all(&sandbox)?;

    let db_path = sandbox.join("farmerpro_p3_sandbox.db");
    let projects_root = sandbox.join("projects");
    let storage = Arc::new(StorageManager::new(&db_path)?);
    let adb_path = repo_root.join("bin").join("adb.exe");
    let adb_arg = if adb_path.exists() {
        Some(adb_path.to_string_lossy().to_string())
    } else {
        None
    };
    let adb_runner = AdbRunner::new(adb_arg);
    let coord = RuntimeCoordinator::new(
        Arc::clone(&storage),
        PathBuf::from(&adb_runner.adb_path),
    );

    // ------------------------------------------------------------------
    // 1. Two Isolated Generic GameProjects + Tasks/Modules/Accounts
    // ------------------------------------------------------------------
    let proj_a = storage.create_project(
        CreateProjectRequest {
            slug: "starlight-ops".to_string(),
            name: "Starlight Tactical Ops".to_string(),
            game_package_id: "com.p3.starlight".to_string(),
            version: Some("1.4.0".to_string()),
            exe_name: Some("Auto_Starlight_Ops.exe".to_string()),
            window_title: Some("Starlight Tactical Auto v1.4".to_string()),
            icon_rel_path: None,
            target_width_px: Some(1080),
            target_height_px: Some(1920),
            bundle_adb: Some(true),
            enabled_modules: Some(vec![
                "mod.adb_core".to_string(),
                "mod.vision_ocr".to_string(),
                "mod.proxy_network".to_string(),
                "mod.adb_port_forward".to_string(),
            ]),
        },
        &projects_root,
    )?;

    let proj_b = storage.create_project(
        CreateProjectRequest {
            slug: "voxel-kingdom".to_string(),
            name: "Voxel Kingdom Builder".to_string(),
            game_package_id: "com.p3.voxel".to_string(),
            version: Some("1.0.0".to_string()),
            exe_name: Some("Auto_Voxel_Kingdom.exe".to_string()),
            window_title: Some("Voxel Kingdom Auto".to_string()),
            icon_rel_path: None,
            target_width_px: Some(1080),
            target_height_px: Some(1920),
            bundle_adb: Some(true),
            enabled_modules: None,
        },
        &projects_root,
    )?;

    // ------------------------------------------------------------------
    // 2. Vision ROI Capture, Crop, Project-Scoped Save, Match, Scaling & Annotation PNGs
    // ------------------------------------------------------------------
    let before_driver = Arc::new(FakeDeviceDriver::new("emulator-5554", 1080, 1920));
    *before_driver.current_screen_state.lock().await = "quest_marker_visible".to_string();
    let before_frame = before_driver.snapshot().await?;
    let before_png_path = out_dir.join("vision_before_capture.png");
    std::fs::write(&before_png_path, &before_frame.bytes)?;

    let (cropped_png, cw, ch) = crop_roi_png(
        &before_frame.bytes,
        RoiRect {
            x: 200,
            y: 400,
            width: 120,
            height: 60,
        },
    )?;
    let cropped_png_path = out_dir.join("vision_cropped_roi.png");
    std::fs::write(&cropped_png_path, &cropped_png)?;

    let tpl_rec = storage.save_project_template_file(
        VisionTemplateInput {
            id: None,
            project_id: proj_a.id.clone(),
            name: "Deploy Mission Button".to_string(),
            rel_path: "templates/deploy_btn.png".to_string(),
            width_px: cw,
            height_px: ch,
            source_crop_x: Some(200),
            source_crop_y: Some(400),
            default_confidence: Some(0.9),
            sha256: sha256_hex(&cropped_png),
        },
        &cropped_png,
    )?;

    let vision = VisionEngine::new();
    let target = vision.find_template_multiscale(&before_frame, &cropped_png, Some(0.85), &[1.0])?;
    let mut sampler = SafeRegionSampler::new(SafeTapPolicy::default());
    let tap_pt = sampler.sample_safe_point(
        &target.bounds,
        &Bounds::new(0.0, 0.0, 1080.0, 1920.0),
        &[],
    )?;
    let after_annotated =
        annotate_match_and_tap_png(&before_frame.bytes, target.bounds, Some(tap_pt))?;
    let after_png_path = out_dir.join("vision_after_annotated.png");
    std::fs::write(&after_png_path, &after_annotated)?;

    let scaled_720p = scale_bounds_for_geometry(
        target.bounds,
        &ScreenGeometry::new(1080, 1920, 0),
        &ScreenGeometry::new(720, 1280, 0),
    )?;

    // Save workflow + task + account in Project A
    let wf_vision_json = serde_json::json!({
        "schemaVersion": 1,
        "workflowId": "wf-starlight-deploy",
        "name": "Deploy Squad & Verify",
        "status": "published",
        "graph": {
            "nodes": [
                { "id": "n-start", "type": "flow.start", "position": { "x": 0.0, "y": 0.0 } },
                { "id": "n-launch", "type": "app.launch", "position": { "x": 100.0, "y": 0.0 }, "config": { "packageId": "com.p3.starlight" } },
                { "id": "n-cap", "type": "vision.capture", "position": { "x": 200.0, "y": 0.0 } },
                { "id": "n-find", "type": "vision.find_image", "position": { "x": 300.0, "y": 0.0 }, "config": { "templateRef": "templates/deploy_btn.png", "minConfidence": 0.85 } },
                { "id": "n-cp", "type": "state.checkpoint", "position": { "x": 400.0, "y": 0.0 }, "config": { "name": "detected_deploy" } },
                { "id": "n-tap", "type": "action.random_safe_tap", "position": { "x": 500.0, "y": 0.0 }, "config": { "marginRatio": 0.15 } },
                { "id": "n-end", "type": "flow.end", "position": { "x": 600.0, "y": 0.0 } }
            ],
            "edges": [
                { "id": "e1", "source": "n-start", "sourcePort": "flow", "target": "n-launch", "targetPort": "flow" },
                { "id": "e2", "source": "n-launch", "sourcePort": "flow", "target": "n-cap", "targetPort": "flow" },
                { "id": "e3", "source": "n-cap", "sourcePort": "flow", "target": "n-find", "targetPort": "flow" },
                { "id": "e4", "source": "n-find", "sourcePort": "flow", "target": "n-cp", "targetPort": "flow" },
                { "id": "e5", "source": "n-cp", "sourcePort": "flow", "target": "n-tap", "targetPort": "flow" },
                { "id": "e6", "source": "n-tap", "sourcePort": "flow", "target": "n-end", "targetPort": "flow" }
            ]
        }
    })
    .to_string();

    storage.save_project_workflow(
        &proj_a.id,
        "wf-starlight-deploy",
        "Deploy Squad & Verify",
        "Combat",
        &wf_vision_json,
    )?;

    storage.upsert_project_task(
        &proj_a.id,
        MiniTaskDef {
            id: "task-starlight-deploy".to_string(),
            title: "Auto Deploy Squad".to_string(),
            category_id: "combat".to_string(),
            workflow_id: "wf-starlight-deploy".to_string(),
            default_enabled: true,
            priority: 1,
            estimate_sec: 15,
            settings_schema: vec![],
        },
        &TaskCategoryDef {
            id: "combat".to_string(),
            short_tag: "CB".to_string(),
            display_name: "Combat".to_string(),
            color_hex: "#EF4444".to_string(),
        },
    )?;

    let acc_a = storage.upsert_project_account(ProjectAccountInput {
        id: None,
        project_id: proj_a.id.clone(),
        alias: "commander-01".to_string(),
        game_profile_id: Some("uid-1001".to_string()),
        secret_ref: None,
        state: "ready".to_string(),
    })?;

    let vision_run = coord
        .run_project_workflow(
            RunProjectWorkflowRequest {
                job_id: Some("job-p3-vision-1".to_string()),
                project_id: proj_a.id.clone(),
                workflow_id: "wf-starlight-deploy".to_string(),
                task_id: Some("task-starlight-deploy".to_string()),
                device_id: "emulator-5554".to_string(),
                account_id: Some(acc_a.id.clone()),
                priority: 10,
                lease_ttl_secs: 15,
                workflow_timeout_ms: Some(5000),
                initial_variables: HashMap::new(),
            },
            Some(Arc::new(FakeDeviceDriver::new("emulator-5554", 1080, 1920))),
        )
        .await?;

    // ------------------------------------------------------------------
    // 3. Cross-Project Template Escape Test
    // ------------------------------------------------------------------
    let escape_wf = serde_json::json!({
        "schemaVersion": 1,
        "workflowId": "wf-voxel-escape",
        "name": "Cross Project Escape Probe",
        "status": "published",
        "graph": {
            "nodes": [
                { "id": "n-start", "type": "flow.start", "position": { "x": 0.0, "y": 0.0 } },
                { "id": "n-find", "type": "vision.find_image", "position": { "x": 100.0, "y": 0.0 }, "config": { "templateRef": format!("{}/templates/deploy_btn.png", proj_a.project_dir_path) } },
                { "id": "n-end", "type": "flow.end", "position": { "x": 200.0, "y": 0.0 } }
            ],
            "edges": [
                { "id": "e1", "source": "n-start", "sourcePort": "flow", "target": "n-find", "targetPort": "flow" },
                { "id": "e2", "source": "n-find", "sourcePort": "flow", "target": "n-end", "targetPort": "flow" }
            ]
        }
    })
    .to_string();
    storage.save_project_workflow(&proj_b.id, "wf-voxel-escape", "Escape Probe", "Audit", &escape_wf)?;
    let cross_drv = Arc::new(FakeDeviceDriver::new("fake-escape", 1080, 1920));
    *cross_drv.current_screen_state.lock().await = "quest_marker_visible".to_string();
    let cross_run = coord
        .run_project_workflow(
            RunProjectWorkflowRequest {
                job_id: Some("job-p3-escape".to_string()),
                project_id: proj_b.id.clone(),
                workflow_id: "wf-voxel-escape".to_string(),
                task_id: None,
                device_id: "fake-escape".to_string(),
                account_id: None,
                priority: 1,
                lease_ttl_secs: 10,
                workflow_timeout_ms: Some(5000),
                initial_variables: HashMap::new(),
            },
            Some(cross_drv.clone()),
        )
        .await?;

    // ------------------------------------------------------------------
    // 4. Multi-Device Concurrent + Pause/Suspend/Checkpoint Resume + Unsafe Block
    // ------------------------------------------------------------------
    let wf_slow = serde_json::json!({
        "schemaVersion": 1,
        "workflowId": "wf-slow-cp",
        "name": "Slow Checkpoint Flow",
        "status": "published",
        "graph": {
            "nodes": [
                { "id": "n-start", "type": "flow.start", "position": { "x": 0.0, "y": 0.0 } },
                { "id": "n-wait", "type": "control.wait", "position": { "x": 100.0, "y": 0.0 }, "config": { "durationMs": 220 } },
                { "id": "n-cp", "type": "state.checkpoint", "position": { "x": 200.0, "y": 0.0 }, "config": { "name": "gate" } },
                { "id": "n-key", "type": "action.key", "position": { "x": 300.0, "y": 0.0 }, "config": { "keycode": 66 } },
                { "id": "n-end", "type": "flow.end", "position": { "x": 400.0, "y": 0.0 } }
            ],
            "edges": [
                { "id": "e1", "source": "n-start", "sourcePort": "flow", "target": "n-wait", "targetPort": "flow" },
                { "id": "e2", "source": "n-wait", "sourcePort": "flow", "target": "n-cp", "targetPort": "flow" },
                { "id": "e3", "source": "n-cp", "sourcePort": "flow", "target": "n-key", "targetPort": "flow" },
                { "id": "e4", "source": "n-key", "sourcePort": "flow", "target": "n-end", "targetPort": "flow" }
            ]
        }
    })
    .to_string();
    storage.save_project_workflow(&proj_a.id, "wf-slow-cp", "Slow Checkpoint", "Sync", &wf_slow)?;

    let drv_d1 = Arc::new(FakeDeviceDriver::new("emulator-5554", 1080, 1920));
    let drv_d2 = Arc::new(FakeDeviceDriver::new("127.0.0.1:62001", 720, 1280));
    let (_h1, j1) = coord
        .start_project_workflow_background(
            RunProjectWorkflowRequest {
                job_id: Some("job-2dev-a".to_string()),
                project_id: proj_a.id.clone(),
                workflow_id: "wf-slow-cp".to_string(),
                task_id: None,
                device_id: "emulator-5554".to_string(),
                account_id: None,
                priority: 1,
                lease_ttl_secs: 10,
                workflow_timeout_ms: None,
                initial_variables: HashMap::new(),
            },
            Some(drv_d1.clone()),
        )
        .await?;
    let lease_conflict_blocked = coord
        .run_project_workflow(
            RunProjectWorkflowRequest {
                job_id: Some("job-2dev-conflict".to_string()),
                project_id: proj_a.id.clone(),
                workflow_id: "wf-slow-cp".to_string(),
                task_id: None,
                device_id: "emulator-5554".to_string(),
                account_id: None,
                priority: 1,
                lease_ttl_secs: 10,
                workflow_timeout_ms: None,
                initial_variables: HashMap::new(),
            },
            None,
        )
        .await
        .is_err();
    let (_h2, j2) = coord
        .start_project_workflow_background(
            RunProjectWorkflowRequest {
                job_id: Some("job-2dev-b".to_string()),
                project_id: proj_a.id.clone(),
                workflow_id: "wf-slow-cp".to_string(),
                task_id: None,
                device_id: "127.0.0.1:62001".to_string(),
                account_id: None,
                priority: 1,
                lease_ttl_secs: 10,
                workflow_timeout_ms: None,
                initial_variables: HashMap::new(),
            },
            Some(drv_d2.clone()),
        )
        .await?;
    let s1 = j1.await??;
    let s2 = j2.await??;

    // Suspend & resume from checkpoint + unsafe guard
    let cp_drv = Arc::new(FakeDeviceDriver::new("emulator-5556", 1080, 1920));
    let (cp_handle, cp_join) = coord
        .start_project_workflow_background(
            RunProjectWorkflowRequest {
                job_id: Some("job-p3-resume".to_string()),
                project_id: proj_a.id.clone(),
                workflow_id: "wf-slow-cp".to_string(),
                task_id: None,
                device_id: "emulator-5556".to_string(),
                account_id: None,
                priority: 1,
                lease_ttl_secs: 10,
                workflow_timeout_ms: None,
                initial_variables: HashMap::new(),
            },
            Some(cp_drv.clone()),
        )
        .await?;
    tokio::time::sleep(Duration::from_millis(50)).await;
    cp_handle.suspend_to_checkpoint()?;
    let cp_paused = cp_join.await??;
    let cp_resumed = coord
        .resume_from_checkpoint("job-p3-resume", 10, Some(5000), Some(cp_drv.clone()))
        .await?;

    let latest_cp = storage.get_latest_checkpoint("job-p3-resume")?.unwrap();
    let mut unsafe_cp: RuntimeCheckpointState = serde_json::from_str(&latest_cp.state_json)?;
    unsafe_cp.next_node_id = "n-key".to_string();
    unsafe_cp.pending_unsafe_node_id = Some("n-key".to_string());
    unsafe_cp.resume_policy = "unsafe_pending_block".to_string();
    storage.save_checkpoint(CheckpointRecordInput {
        id: None,
        job_id: "job-p3-resume".to_string(),
        node_id: "n-key".to_string(),
        state_json: serde_json::to_string(&unsafe_cp)?,
        evidence_ref: None,
    })?;
    let cp_unsafe_blocked = coord
        .resume_from_checkpoint("job-p3-resume", 10, Some(5000), Some(cp_drv.clone()))
        .await?;

    // ------------------------------------------------------------------
    // 5. Real Loopback Proxy Probes (SOCKS5 + HTTP CONNECT + Unreachable Fail)
    // ------------------------------------------------------------------
    let socks_listener = TcpListener::bind("127.0.0.1:0").await?;
    let socks_port = socks_listener.local_addr()?.port();
    let socks_srv = tokio::spawn(async move {
        if let Ok((mut s, _)) = socks_listener.accept().await {
            let mut g = [0u8; 3];
            let _ = s.read_exact(&mut g).await;
            let _ = s.write_all(&[0x05, 0x00]).await;
            let mut r = [0u8; 10];
            let _ = s.read_exact(&mut r).await;
            let _ = s
                .write_all(&[0x05, 0x00, 0x00, 0x01, 127, 0, 0, 1, 0, 80])
                .await;
            let _ = s.shutdown().await;
        }
    });
    let proxy_socks5_ok = probe_proxy(&ProxyConfig {
        host: "127.0.0.1".to_string(),
        port: socks_port,
        protocol: ProxyProtocol::Socks5,
        username: None,
        password: None,
        timeout_ms: 1500,
        target_probe_host: Some("127.0.0.1".to_string()),
        target_probe_port: Some(80),
    })
    .await;
    let _ = socks_srv.await;

    let http_listener = TcpListener::bind("127.0.0.1:0").await?;
    let http_port = http_listener.local_addr()?.port();
    let http_srv = tokio::spawn(async move {
        if let Ok((mut s, _)) = http_listener.accept().await {
            let mut buf = [0u8; 512];
            let _ = s.read(&mut buf).await;
            let _ = s
                .write_all(b"HTTP/1.1 407 Proxy Authentication Required\r\n\r\n")
                .await;
            let _ = s.shutdown().await;
        }
    });
    let proxy_http_407 = probe_proxy(&ProxyConfig {
        host: "127.0.0.1".to_string(),
        port: http_port,
        protocol: ProxyProtocol::HttpConnect,
        username: Some("bad".to_string()),
        password: Some("creds".to_string()),
        timeout_ms: 1500,
        target_probe_host: Some("1.1.1.1".to_string()),
        target_probe_port: Some(443),
    })
    .await;
    let _ = http_srv.await;

    // ------------------------------------------------------------------
    // 6. Real ADB CLI Probe (Truthful: Blocked if no attached device)
    // ------------------------------------------------------------------
    let discovered = adb_runner.list_devices().await;
    let live_hardware_section = match discovered {
        Ok(devs) if devs.iter().any(|d| d.connection_state == ConnectionState::Ready) => {
            let ready_dev = devs
                .iter()
                .find(|d| d.connection_state == ConnectionState::Ready)
                .unwrap();
            let health = adb_runner.probe_health(&ready_dev.adb_serial).await;
            let fwd_ok = adb_runner
                .forward_add(&ready_dev.adb_serial, "tcp:19991", "tcp:19991")
                .await
                .is_ok();
            let fwd_list = adb_runner
                .forward_list(Some(&ready_dev.adb_serial))
                .await
                .unwrap_or_default();
            let _ = adb_runner
                .forward_remove(&ready_dev.adb_serial, "tcp:19991")
                .await;
            let rev_ok = adb_runner
                .reverse_add(&ready_dev.adb_serial, "tcp:19992", "tcp:19992")
                .await
                .is_ok();
            let rev_list = adb_runner
                .reverse_list(&ready_dev.adb_serial)
                .await
                .unwrap_or_default();
            let _ = adb_runner
                .reverse_remove(&ready_dev.adb_serial, "tcp:19992")
                .await;

            serde_json::json!({
                "status": "PASS",
                "serial": ready_dev.adb_serial,
                "adapterKind": classify_device_adapter(&ready_dev.adb_serial, ready_dev.model.as_deref()),
                "health": health,
                "forwardRoundtripOk": fwd_ok && !fwd_list.is_empty(),
                "reverseRoundtripOk": rev_ok && !rev_list.is_empty(),
            })
        }
        Ok(devs) => {
            let offline_probe = adb_runner.probe_health("127.0.0.1:5555").await;
            let reconnect_err = adb_runner
                .reconnect("127.0.0.1:5555")
                .await
                .err()
                .map(|e| e.to_string());
            serde_json::json!({
                "status": "BLOCKED",
                "reason": "No ready Android device/emulator attached to ADB (LDPlayer dnconsole.exe requires UAC elevation on this host)",
                "discoveredCount": devs.len(),
                "adbBinaryPath": adb_runner.adb_path,
                "offlineProbe": offline_probe,
                "reconnectError": reconnect_err,
                "tokenSanitizationRejectsInjection": validate_adb_token("serial", "127.0.0.1:5555;calc").is_err(),
            })
        }
        Err(err) => {
            serde_json::json!({
                "status": "BLOCKED",
                "reason": format!("ADB discovery failed: {}", err),
                "adbBinaryPath": adb_runner.adb_path,
            })
        }
    };

    // ------------------------------------------------------------------
    // 7. Generic Builder Preflight & Export (Zero JX2 Hardcoding)
    // ------------------------------------------------------------------
    let ico_path = PathBuf::from(&proj_a.project_dir_path).join("branding/icon.ico");
    std::fs::create_dir_all(ico_path.parent().unwrap())?;
    std::fs::write(&ico_path, build_test_ico_32x32())?;

    let runtime_exe = {
        let dbg = repo_root.join("target/debug/fp-mini-runtime.exe");
        if dbg.exists() {
            dbg
        } else {
            std::env::current_exe()?
        }
    };
    let bin_dir = repo_root.join("bin");
    let preflight = MiniToolPackager::preflight_validate_project(
        Path::new(&proj_a.project_dir_path),
        &runtime_exe,
        Some(&bin_dir),
        None,
    )?;

    let release_dir = sandbox.join("release_starlight");
    let build_report = MiniToolPackager::build_and_export_release(
        Path::new(&proj_a.project_dir_path),
        &runtime_exe,
        Some(&bin_dir),
        &release_dir,
        None,
    )?;

    let release_id = format!("rel-{}", Uuid::new_v4());
    storage.record_project_release(ProjectReleaseRecord {
        id: release_id.clone(),
        project_id: proj_a.id.clone(),
        version: proj_a.version.clone(),
        output_dir_path: release_dir.to_string_lossy().to_string(),
        exe_path: build_report.tool_exe_path.clone(),
        exe_size_bytes: build_report.tool_exe_size_bytes,
        exe_sha256: build_report.tool_exe_sha256.clone(),
        key_id: build_report.key_id.clone(),
        publisher_public_key_hex: build_report.publisher_public_key_hex.clone(),
        pe_rcdata_pubkey_id: build_report.pe_embedded_pubkey_resource_id as i64,
        signature_ed25519_hex: build_report.signature_ed25519_hex.clone(),
        build_report_json: serde_json::to_string(&build_report)?,
        built_at: chrono::Utc::now().to_rfc3339(),
    })?;

    let integrity = storage.check_integrity()?;
    let detail_a = storage.get_project_detail(&proj_a.id)?.unwrap();

    let report = serde_json::json!({
        "command": "verify-p3",
        "executedAtUtc": chrono::Utc::now().to_rfc3339(),
        "productionDbUntouched": true,
        "projects": {
            "projectA": { "id": proj_a.id, "slug": proj_a.slug, "releases": detail_a.releases.len() },
            "projectB": { "id": proj_b.id, "slug": proj_b.slug },
        },
        "visionAndRoi": {
            "beforeCapturePng": before_png_path.to_string_lossy(),
            "croppedRoiPng": cropped_png_path.to_string_lossy(),
            "afterAnnotatedPng": after_png_path.to_string_lossy(),
            "templateRecord": tpl_rec,
            "matchedConfidence": target.confidence,
            "matchedBounds1080p": target.bounds,
            "scaledBounds720p": scaled_720p,
            "safeTapPoint": tap_pt,
        },
        "workflowExecution": {
            "visionRun": vision_run,
            "crossProjectEscapeState": cross_run.state,
            "crossProjectEscapeTaps": cross_drv.tap_count.load(Ordering::SeqCst),
        },
        "multiDeviceScheduler": {
            "device1Summary": s1,
            "device2Summary": s2,
            "sameDeviceLeaseConflictBlocked": lease_conflict_blocked,
            "activeLeasesRemaining": coord.active_device_leases().len(),
        },
        "checkpointRecovery": {
            "suspendedState": cp_paused.state,
            "resumedState": cp_resumed.state,
            "resumedGeneration": cp_resumed.fencing_generation,
            "unsafePendingBlockState": cp_unsafe_blocked.state,
            "unsafePendingBlockReason": cp_unsafe_blocked.block_reason,
        },
        "proxyProbes": {
            "socks5Ok": proxy_socks5_ok,
            "httpConnect407": proxy_http_407,
        },
        "liveHardwareAdb": live_hardware_section,
        "builderGenericExport": {
            "preflightReady": preflight.ready,
            "exeName": build_report.tool_exe_name,
            "exeSha256": build_report.tool_exe_sha256,
            "keyId": build_report.key_id,
            "peEmbeddedPubkeyVerified": build_report.pe_embedded_pubkey_verified,
            "recordedReleaseId": release_id,
        },
        "sqliteIntegrity": integrity,
    });

    std::fs::write(
        out_dir.join("p3_verification_report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;

    Ok(report)
}
