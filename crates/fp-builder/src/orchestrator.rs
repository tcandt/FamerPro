use anyhow::Result;
use chrono::Duration;
use fp_device::DeviceActorHandle;
use fp_domain::{DeviceDriver, JobState, StepEvent};
use fp_scheduler::MultiDeviceScheduler;
use fp_testkit::FakeDeviceDriver;
use fp_workflow_runtime::{ExecutionContext, ExecutionSignal, WorkflowExecutor};
use fp_workflow_schema::WorkflowValidator;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::sync::watch;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MultiDeviceProofReport {
    pub passed: bool,
    pub device_a_id: String,
    pub device_b_id: String,
    pub device_a_account: String,
    pub device_b_account: String,
    pub device_a_lease_gen: u64,
    pub device_b_lease_gen: u64,
    pub lease_conflict_blocked: bool,
    pub stale_generation_fenced: bool,
    pub device_a_final_state: String,
    pub device_b_final_state: String,
    pub device_a_events_count: usize,
    pub device_b_events_count: usize,
    pub cross_device_contamination: bool,
}

/// Bridges `fp-device::DeviceActorHandle`, `fp-scheduler::MultiDeviceScheduler`, and
/// `fp-workflow-runtime::WorkflowExecutor` to prove concurrent multi-device isolation,
/// lease exclusivity, generation fencing, and single-device cancellation without affecting peers.
pub async fn run_concurrent_two_device_verification() -> Result<MultiDeviceProofReport> {
    let scheduler = Arc::new(MultiDeviceScheduler::new(4));

    // 1. Acquire device & account leases for Device A and Device B
    let lease_dev_a = scheduler.acquire_device_lease("emulator-5554", "job-dev-A", Duration::seconds(120))?;
    let _lease_acc_a = scheduler.acquire_account_lease("acc-jx2-main", "job-dev-A", Duration::seconds(120))?;

    let lease_dev_b = scheduler.acquire_device_lease("emulator-5556", "job-dev-B", Duration::seconds(120))?;
    let _lease_acc_b = scheduler.acquire_account_lease("acc-slg-lord", "job-dev-B", Duration::seconds(120))?;

    // 2. Verify that a conflicting job trying to hijack emulator-5554 is blocked by lease exclusion
    let conflict_attempt = scheduler.acquire_device_lease("emulator-5554", "job-hijack", Duration::seconds(60));
    let lease_conflict_blocked = conflict_attempt.is_err();

    // 3. Create DeviceActorHandles and verify generation fencing blocks stale commands
    let driver_a = Arc::new(FakeDeviceDriver::new("emulator-5554", 1080, 1920));
    let mut actor_a = DeviceActorHandle::new("emulator-5554".to_string(), driver_a.clone());
    let new_gen_a = actor_a.bump_generation(); // gen is now 2
    let stale_tap_res = actor_a.tap(100, 100, 1).await; // stale gen 1 must fail
    let stale_generation_fenced = stale_tap_res.is_err();
    let valid_tap_res = actor_a.tap(100, 100, new_gen_a).await;
    assert!(valid_tap_res.is_ok());

    let driver_b = Arc::new(FakeDeviceDriver::new("emulator-5556", 1080, 1920));

    // 4. Define 2 different workflows:
    // Device A runs a long multi-step workflow that we cancel mid-flight (simulating device stop/disconnect)
    let wf_a_json = r#"{
        "schemaVersion": 1,
        "workflowId": "wf-dev-a-long",
        "name": "Device A Mission",
        "status": "published",
        "graph": {
            "nodes": [
                { "id": "n-start", "type": "flow.start", "position": { "x": 0.0, "y": 0.0 } },
                { "id": "n-launch", "type": "app.launch", "position": { "x": 100.0, "y": 0.0 }, "config": { "packageId": "com.blackhole.jx2mobile" } },
                { "id": "n-wait-1", "type": "control.wait", "position": { "x": 200.0, "y": 0.0 }, "config": { "durationMs": 250 } },
                { "id": "n-wait-2", "type": "control.wait", "position": { "x": 300.0, "y": 0.0 }, "config": { "durationMs": 250 } },
                { "id": "n-end", "type": "flow.end", "position": { "x": 400.0, "y": 0.0 } }
            ],
            "edges": [
                { "id": "e1", "source": "n-start", "sourcePort": "flow", "target": "n-launch", "targetPort": "flow" },
                { "id": "e2", "source": "n-launch", "sourcePort": "flow", "target": "n-wait-1", "targetPort": "flow" },
                { "id": "e3", "source": "n-wait-1", "sourcePort": "flow", "target": "n-wait-2", "targetPort": "flow" },
                { "id": "e4", "source": "n-wait-2", "sourcePort": "flow", "target": "n-end", "targetPort": "flow" }
            ]
        }
    }"#;

    // Device B runs a vision + safe tap + checkpoint workflow to full completion
    let wf_b_json = r#"{
        "schemaVersion": 1,
        "workflowId": "wf-dev-b-complete",
        "name": "Device B SLG Harvest",
        "status": "published",
        "graph": {
            "nodes": [
                { "id": "n-start", "type": "flow.start", "position": { "x": 0.0, "y": 0.0 } },
                { "id": "n-launch", "type": "app.launch", "position": { "x": 100.0, "y": 0.0 }, "config": { "packageId": "com.kingdom.rise.slg" } },
                { "id": "n-cap", "type": "vision.capture", "position": { "x": 200.0, "y": 0.0 } },
                { "id": "n-find", "type": "vision.find_image", "position": { "x": 300.0, "y": 0.0 }, "config": { "templateRef": "quest_marker.png", "minConfidence": 0.85 } },
                { "id": "n-tap", "type": "action.random_safe_tap", "position": { "x": 400.0, "y": 0.0 } },
                { "id": "n-cp", "type": "state.checkpoint", "position": { "x": 500.0, "y": 0.0 }, "config": { "name": "slg_harvest_done" } },
                { "id": "n-end", "type": "flow.end", "position": { "x": 600.0, "y": 0.0 } }
            ],
            "edges": [
                { "id": "e1", "source": "n-start", "sourcePort": "flow", "target": "n-launch", "targetPort": "flow" },
                { "id": "e2", "source": "n-launch", "sourcePort": "flow", "target": "n-cap", "targetPort": "flow" },
                { "id": "e3", "source": "n-cap", "sourcePort": "flow", "target": "n-find", "targetPort": "flow" },
                { "id": "e4", "source": "n-find", "sourcePort": "flow", "target": "n-tap", "targetPort": "flow" },
                { "id": "e5", "source": "n-tap", "sourcePort": "flow", "target": "n-cp", "targetPort": "flow" },
                { "id": "e6", "source": "n-cp", "sourcePort": "flow", "target": "n-end", "targetPort": "flow" }
            ]
        }
    }"#;

    let doc_a = WorkflowValidator::parse_and_validate(wf_a_json)?;
    let doc_b = WorkflowValidator::parse_and_validate(wf_b_json)?;

    let (tx_a, rx_a) = watch::channel(ExecutionSignal::Running);
    let (_tx_b, rx_b) = watch::channel(ExecutionSignal::Running);

    let events_a: Arc<Mutex<Vec<StepEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let events_b: Arc<Mutex<Vec<StepEvent>>> = Arc::new(Mutex::new(Vec::new()));

    let ev_a_clone = events_a.clone();
    let ev_b_clone = events_b.clone();

    let driver_a_exec: Arc<dyn DeviceDriver> = driver_a.clone();
    let driver_b_exec: Arc<dyn DeviceDriver> = driver_b.clone();

    // Cancel Device A after 120ms while Device B continues concurrently
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(120)).await;
        let _ = tx_a.send(ExecutionSignal::Cancel);
    });

    let task_a = tokio::spawn(async move {
        let mut exec = WorkflowExecutor::new(driver_a_exec, rx_a);
        let mut ctx = ExecutionContext::new("job-dev-A".to_string(), "emulator-5554".to_string());
        exec.execute_workflow(
            &doc_a,
            &mut ctx,
            Some(Arc::new(move |e| {
                ev_a_clone.lock().unwrap().push(e);
            })),
        )
        .await
    });

    let task_b = tokio::spawn(async move {
        let mut exec = WorkflowExecutor::new(driver_b_exec, rx_b);
        let mut ctx = ExecutionContext::new("job-dev-B".to_string(), "emulator-5556".to_string());
        exec.execute_workflow(
            &doc_b,
            &mut ctx,
            Some(Arc::new(move |e| {
                ev_b_clone.lock().unwrap().push(e);
            })),
        )
        .await
    });

    let state_a = task_a.await??;
    let state_b = task_b.await??;

    // Release leases cleanly
    scheduler.release_device_lease("emulator-5554", "job-dev-A");
    scheduler.release_account_lease("acc-jx2-main", "job-dev-A");
    scheduler.release_device_lease("emulator-5556", "job-dev-B");
    scheduler.release_account_lease("acc-slg-lord", "job-dev-B");

    let list_a = events_a.lock().unwrap();
    let list_b = events_b.lock().unwrap();

    let cross_contamination = list_a.iter().any(|e| e.job_id != "job-dev-A")
        || list_b.iter().any(|e| e.job_id != "job-dev-B");

    let passed = lease_conflict_blocked
        && stale_generation_fenced
        && state_a == JobState::Cancelled
        && state_b == JobState::Succeeded
        && !cross_contamination;

    Ok(MultiDeviceProofReport {
        passed,
        device_a_id: "emulator-5554".to_string(),
        device_b_id: "emulator-5556".to_string(),
        device_a_account: "acc-jx2-main".to_string(),
        device_b_account: "acc-slg-lord".to_string(),
        device_a_lease_gen: lease_dev_a.generation,
        device_b_lease_gen: lease_dev_b.generation,
        lease_conflict_blocked,
        stale_generation_fenced,
        device_a_final_state: format!("{:?}", state_a),
        device_b_final_state: format!("{:?}", state_b),
        device_a_events_count: list_a.len(),
        device_b_events_count: list_b.len(),
        cross_device_contamination: cross_contamination,
    })
}
