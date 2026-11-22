use crate::{ExecutionContext, ExecutionSignal, RuntimeCheckpointState, WorkflowExecutor};
use anyhow::{bail, Context, Result};
use chrono::{Duration as ChronoDuration, Utc};
use fp_adb::{classify_device_adapter, AdbDeviceDriver, AdbRunner};
use fp_device::{DeviceActorHandle, DeviceRegistry, FencedDeviceDriver};
use fp_domain::{
    DeviceAdapterKind, DeviceDriver, JobState, StepEvent, StepEventType, VariableSource,
};
use fp_scheduler::{DeviceLease, MultiDeviceScheduler};
use fp_storage::{
    CheckpointRecordInput, GameProjectRecord, ProjectJobInput, ProjectWorkflowRecord,
    StorageManager,
};
use fp_workflow_schema::{WorkflowDocument, WorkflowValidator};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tokio::sync::{oneshot, watch};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunProjectWorkflowRequest {
    pub job_id: Option<String>,
    pub project_id: String,
    pub workflow_id: String,
    pub task_id: Option<String>,
    pub device_id: String,
    pub account_id: Option<String>,
    pub priority: i32,
    pub lease_ttl_secs: u64,
    pub workflow_timeout_ms: Option<u64>,
    #[serde(default)]
    pub initial_variables: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunWorkflowExecutionSummary {
    pub job_id: String,
    pub project_id: String,
    pub workflow_id: String,
    pub workflow_version_id: String,
    pub workflow_sha256: String,
    pub task_id: Option<String>,
    pub device_id: String,
    pub account_id: Option<String>,
    pub fencing_generation: u64,
    pub adapter_kind: DeviceAdapterKind,
    pub state: String,
    pub current_node_id: Option<String>,
    pub checkpoint_id: Option<String>,
    pub resumed_from_node_id: Option<String>,
    pub block_reason: Option<String>,
    pub events_recorded: usize,
    pub variables: HashMap<String, serde_json::Value>,
}

#[derive(Clone)]
pub struct JobControlHandle {
    pub job_id: String,
    pub device_id: String,
    pub signal_tx: watch::Sender<ExecutionSignal>,
}

impl JobControlHandle {
    pub fn pause(&self) -> Result<()> {
        self.signal_tx
            .send(ExecutionSignal::Pause)
            .map_err(|_| anyhow::anyhow!("Job '{}' is no longer active", self.job_id))
    }

    pub fn resume(&self) -> Result<()> {
        self.signal_tx
            .send(ExecutionSignal::Running)
            .map_err(|_| anyhow::anyhow!("Job '{}' is no longer active", self.job_id))
    }

    pub fn suspend_to_checkpoint(&self) -> Result<()> {
        self.signal_tx
            .send(ExecutionSignal::SuspendToCheckpoint)
            .map_err(|_| anyhow::anyhow!("Job '{}' is no longer active", self.job_id))
    }

    pub fn cancel(&self) -> Result<()> {
        self.signal_tx
            .send(ExecutionSignal::Cancel)
            .map_err(|_| anyhow::anyhow!("Job '{}' is no longer active", self.job_id))
    }

    pub fn current_signal(&self) -> ExecutionSignal {
        self.signal_tx.borrow().clone()
    }
}

/// Unified runtime orchestrator tying Project/Workflow/Task -> MultiDeviceScheduler leases & fencing ->
/// DeviceRegistry single-writer actors -> WorkflowExecutor -> SQLite Storage checkpoints & step events.
#[derive(Clone)]
pub struct RuntimeCoordinator {
    pub storage: Arc<StorageManager>,
    pub scheduler: Arc<MultiDeviceScheduler>,
    pub registry: Arc<DeviceRegistry>,
    pub adb_runner: AdbRunner,
    active_jobs: Arc<RwLock<HashMap<String, watch::Sender<ExecutionSignal>>>>,
}

struct PreparedExecution {
    job_id: String,
    project: GameProjectRecord,
    workflow_id: String,
    workflow_version_id: String,
    workflow_sha256: String,
    doc: WorkflowDocument,
    task_id: Option<String>,
    device_id: String,
    account_id: Option<String>,
    fencing_generation: u64,
    adapter_kind: DeviceAdapterKind,
    lease_ttl_secs: u64,
    workflow_timeout_ms: Option<u64>,
    resume_node_id: Option<String>,
    ctx: ExecutionContext,
    driver: Arc<dyn DeviceDriver>,
    signal_tx: watch::Sender<ExecutionSignal>,
    signal_rx: watch::Receiver<ExecutionSignal>,
}

impl RuntimeCoordinator {
    pub fn new(storage: Arc<StorageManager>, adb_binary: PathBuf) -> Self {
        let adb_arg = Some(adb_binary.to_string_lossy().to_string());
        Self {
            storage,
            scheduler: Arc::new(MultiDeviceScheduler::new(4)),
            registry: Arc::new(DeviceRegistry::new()),
            adb_runner: AdbRunner::new(adb_arg),
            active_jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_scheduler(
        storage: Arc<StorageManager>,
        scheduler: Arc<MultiDeviceScheduler>,
        adb_binary: PathBuf,
    ) -> Self {
        let adb_arg = Some(adb_binary.to_string_lossy().to_string());
        Self {
            storage,
            scheduler,
            registry: Arc::new(DeviceRegistry::new()),
            adb_runner: AdbRunner::new(adb_arg),
            active_jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn active_device_leases(&self) -> Vec<DeviceLease> {
        self.scheduler.active_device_leases()
    }

    pub fn get_job_control(&self, job_id: &str) -> Option<JobControlHandle> {
        let map = self.active_jobs.read().unwrap();
        map.get(job_id).map(|tx| JobControlHandle {
            job_id: job_id.to_string(),
            device_id: String::new(),
            signal_tx: tx.clone(),
        })
    }

    pub fn pause_job(&self, job_id: &str) -> Result<()> {
        let tx = self
            .active_jobs
            .read()
            .unwrap()
            .get(job_id)
            .cloned()
            .with_context(|| format!("Job '{}' is not currently running", job_id))?;
        let _ = self
            .storage
            .update_job_progress(job_id, "paused", None, None);
        tx.send(ExecutionSignal::Pause)
            .map_err(|_| anyhow::anyhow!("Job '{}' signal channel closed", job_id))
    }

    pub fn resume_job(&self, job_id: &str) -> Result<()> {
        let tx = self
            .active_jobs
            .read()
            .unwrap()
            .get(job_id)
            .cloned()
            .with_context(|| format!("Job '{}' is not currently running", job_id))?;
        let _ = self
            .storage
            .update_job_progress(job_id, "running", None, None);
        tx.send(ExecutionSignal::Running)
            .map_err(|_| anyhow::anyhow!("Job '{}' signal channel closed", job_id))
    }

    pub fn suspend_job_to_checkpoint(&self, job_id: &str) -> Result<()> {
        let tx = self
            .active_jobs
            .read()
            .unwrap()
            .get(job_id)
            .cloned()
            .with_context(|| format!("Job '{}' is not currently running", job_id))?;
        tx.send(ExecutionSignal::SuspendToCheckpoint)
            .map_err(|_| anyhow::anyhow!("Job '{}' signal channel closed", job_id))
    }

    pub fn cancel_job(&self, job_id: &str) -> Result<()> {
        let tx = self
            .active_jobs
            .read()
            .unwrap()
            .get(job_id)
            .cloned()
            .with_context(|| format!("Job '{}' is not currently running", job_id))?;
        tx.send(ExecutionSignal::Cancel)
            .map_err(|_| anyhow::anyhow!("Job '{}' signal channel closed", job_id))
    }

    fn resolve_project_workflow(
        &self,
        project_id: &str,
        workflow_id: &str,
    ) -> Result<ProjectWorkflowRecord> {
        let workflows = self.storage.list_project_workflows(project_id)?;
        workflows
            .into_iter()
            .find(|w| w.id == workflow_id)
            .with_context(|| {
                format!(
                    "PROJECT_ISOLATION_VIOLATION: Workflow '{}' does not exist inside project '{}'",
                    workflow_id, project_id
                )
            })
    }

    fn validate_task_binding(&self, project_id: &str, workflow_id: &str, task_id: &str) -> Result<()> {
        let tasks = self.storage.list_project_tasks(project_id)?;
        let task = tasks.into_iter().find(|t| t.id == task_id).with_context(|| {
            format!(
                "PROJECT_ISOLATION_VIOLATION: Task '{}' does not exist inside project '{}'",
                task_id, project_id
            )
        })?;
        if task.workflow_id != workflow_id {
            bail!(
                "TASK_WORKFLOW_MISMATCH: Task '{}' maps to workflow '{}', requested '{}'",
                task_id,
                task.workflow_id,
                workflow_id
            );
        }
        Ok(())
    }

    fn prepare_fenced_driver(
        &self,
        device_id: &str,
        fencing_generation: u64,
        custom_driver: Option<Arc<dyn DeviceDriver>>,
    ) -> (DeviceAdapterKind, Arc<dyn DeviceDriver>) {
        if let Some(drv) = custom_driver {
            // Keep any previously registered actor in sync so stale holders fail generation fencing immediately
            if let Some(existing) = self.registry.get(device_id) {
                existing.sync_generation(fencing_generation);
            }
            let actor = DeviceActorHandle::with_adapter(
                device_id.to_string(),
                DeviceAdapterKind::Synthetic,
                drv,
            );
            actor.sync_generation(fencing_generation);
            let fenced = Arc::new(FencedDeviceDriver::new(actor, fencing_generation));
            return (DeviceAdapterKind::Synthetic, fenced);
        }

        let adapter_kind = classify_device_adapter(device_id, None);
        let runner = self.adb_runner.clone();
        let dev_id = device_id.to_string();
        let actor = self.registry.register_or_get(device_id, adapter_kind.clone(), move || {
            Arc::new(AdbDeviceDriver::new(dev_id, runner))
        });
        actor.sync_generation(fencing_generation);
        let fenced = Arc::new(FencedDeviceDriver::new(actor, fencing_generation));
        (adapter_kind, fenced)
    }

    fn prepare_new_execution(
        &self,
        req: RunProjectWorkflowRequest,
        custom_driver: Option<Arc<dyn DeviceDriver>>,
    ) -> Result<PreparedExecution> {
        let project = self
            .storage
            .get_project(&req.project_id)?
            .with_context(|| format!("GameProject '{}' not found", req.project_id))?;

        let wf = self.resolve_project_workflow(&req.project_id, &req.workflow_id)?;
        if let Some(ref tid) = req.task_id {
            self.validate_task_binding(&req.project_id, &req.workflow_id, tid)?;
        }

        if let Some(ref acc_id) = req.account_id {
            let accounts = self.storage.list_project_accounts(&req.project_id)?;
            if !accounts.iter().any(|a| a.id == *acc_id) {
                bail!(
                    "PROJECT_ISOLATION_VIOLATION: Account '{}' does not belong to project '{}'",
                    acc_id,
                    req.project_id
                );
            }
        }

        let doc = WorkflowValidator::parse_and_validate(&wf.draft_json)?;
        let start_node_id = doc
            .graph
            .nodes
            .iter()
            .find(|n| n.node_type == "flow.start")
            .map(|n| n.id.clone());

        let job_id = req
            .job_id
            .unwrap_or_else(|| format!("job-{}", Uuid::new_v4()));
        let ttl_secs = req.lease_ttl_secs.max(4);
        let ttl = ChronoDuration::seconds(ttl_secs as i64);

        let device_lease = self
            .scheduler
            .acquire_device_lease(&req.device_id, &job_id, ttl)?;

        if let Some(ref acc_id) = req.account_id {
            if let Err(err) = self.scheduler.acquire_account_lease(acc_id, &job_id, ttl) {
                let _ = self.scheduler.release_device_lease(&req.device_id, &job_id);
                return Err(err);
            }
        }

        // Persist job row and SQLite device_leases record; rollback leases on DB error
        let setup_res = (|| -> Result<(String, String)> {
            let job_rec = self.storage.create_or_update_project_job(ProjectJobInput {
                project_id: req.project_id.clone(),
                job_id: job_id.clone(),
                device_id: req.device_id.clone(),
                account_id: req.account_id.clone(),
                workflow_id: wf.id.clone(),
                workflow_name: wf.name.clone(),
                graph_json: wf.draft_json.clone(),
                priority: req.priority,
                state: "running".to_string(),
                fencing_generation: device_lease.generation,
                current_node_id: start_node_id.clone(),
            })?;
            let resume_ctx = self
                .storage
                .load_job_resume_context(&job_id)?
                .context("Failed loading workflow version metadata for job")?;

            self.storage.upsert_device_lease_record(
                &req.device_id,
                &job_id,
                device_lease.generation,
                &device_lease.expires_at.to_rfc3339(),
            )?;

            Ok((job_rec.workflow_version_id, resume_ctx.workflow_sha256))
        })();

        let (workflow_version_id, workflow_sha256) = match setup_res {
            Ok(pair) => pair,
            Err(err) => {
                if let Some(ref acc_id) = req.account_id {
                    self.scheduler.release_account_lease(acc_id, &job_id);
                }
                self.scheduler.release_device_lease(&req.device_id, &job_id);
                return Err(err);
            }
        };

        let (adapter_kind, driver) =
            self.prepare_fenced_driver(&req.device_id, device_lease.generation, custom_driver);

        let mut ctx = ExecutionContext::new(job_id.clone(), req.device_id.clone())
            .with_project_dir(&project.project_dir_path)
            .with_source_mode(match adapter_kind {
                DeviceAdapterKind::Synthetic => VariableSource::TestkitMock,
                _ => VariableSource::LiveState,
            });
        ctx.project_id = Some(req.project_id.clone());
        ctx.workflow_version_id = Some(workflow_version_id.clone());
        ctx.workflow_version = Some(format!("{}@v{}", wf.id, wf.latest_version_no.unwrap_or(1)));
        ctx.workflow_sha256 = Some(workflow_sha256.clone());
        ctx.task_id = req.task_id.clone();
        ctx.account_id = req.account_id.clone();
        ctx.fencing_generation = device_lease.generation;
        ctx.variables = req.initial_variables;

        let (signal_tx, signal_rx) = watch::channel(ExecutionSignal::Running);
        self.active_jobs
            .write()
            .unwrap()
            .insert(job_id.clone(), signal_tx.clone());

        Ok(PreparedExecution {
            job_id,
            project,
            workflow_id: wf.id,
            workflow_version_id,
            workflow_sha256,
            doc,
            task_id: req.task_id,
            device_id: req.device_id,
            account_id: req.account_id,
            fencing_generation: device_lease.generation,
            adapter_kind,
            lease_ttl_secs: ttl_secs,
            workflow_timeout_ms: req.workflow_timeout_ms,
            resume_node_id: None,
            ctx,
            driver,
            signal_tx,
            signal_rx,
        })
    }

    async fn execute_prepared(&self, mut prep: PreparedExecution) -> Result<RunWorkflowExecutionSummary> {
        let storage = Arc::clone(&self.storage);
        let scheduler = Arc::clone(&self.scheduler);

        let latest_checkpoint_id: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let last_node_id: Arc<Mutex<Option<String>>> =
            Arc::new(Mutex::new(prep.resume_node_id.clone()));
        let event_count = Arc::new(AtomicUsize::new(0));

        // 1. Spawn lease heartbeat renewal loop
        let (hb_stop_tx, mut hb_stop_rx) = oneshot::channel::<()>();
        let hb_scheduler = Arc::clone(&scheduler);
        let hb_storage = Arc::clone(&storage);
        let hb_device_id = prep.device_id.clone();
        let hb_job_id = prep.job_id.clone();
        let hb_gen = prep.fencing_generation;
        let hb_ttl_secs = prep.lease_ttl_secs;
        let hb_interval_ms = ((hb_ttl_secs * 1000) / 2).clamp(250, 5000);

        let hb_task = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_millis(hb_interval_ms));
            ticker.tick().await;
            loop {
                tokio::select! {
                    _ = &mut hb_stop_rx => break,
                    _ = ticker.tick() => {
                        match hb_scheduler.renew_device_lease(
                            &hb_device_id,
                            &hb_job_id,
                            hb_gen,
                            ChronoDuration::seconds(hb_ttl_secs as i64),
                        ) {
                            Ok(renewed) => {
                                let _ = hb_storage.upsert_device_lease_record(
                                    &hb_device_id,
                                    &hb_job_id,
                                    hb_gen,
                                    &renewed.expires_at.to_rfc3339(),
                                );
                            }
                            Err(_) => break,
                        }
                    }
                }
            }
        });

        // 2. Wire checkpoint sink and step event persistence to SQLite
        let cp_storage = Arc::clone(&storage);
        let cp_job_id = prep.job_id.clone();
        let cp_id_holder = Arc::clone(&latest_checkpoint_id);
        let cp_node_holder = Arc::clone(&last_node_id);
        let checkpoint_sink = Arc::new(move |cp_state: RuntimeCheckpointState| -> Result<()> {
            let state_json = serde_json::to_string(&cp_state)?;
            let node_id = cp_state.next_node_id.clone();
            let rec = cp_storage.save_checkpoint(CheckpointRecordInput {
                id: None,
                job_id: cp_job_id.clone(),
                node_id: node_id.clone(),
                state_json,
                evidence_ref: None,
            })?;
            *cp_id_holder.lock().unwrap() = Some(rec.id);
            *cp_node_holder.lock().unwrap() = Some(node_id);
            Ok(())
        });

        let ev_storage = Arc::clone(&storage);
        let ev_count = Arc::clone(&event_count);
        let ev_node_holder = Arc::clone(&last_node_id);
        let event_callback: Arc<dyn Fn(StepEvent) + Send + Sync> = Arc::new(move |ev: StepEvent| {
            *ev_node_holder.lock().unwrap() = Some(ev.node_id.clone());
            if ev_storage.insert_step_event(&ev).is_ok() {
                ev_count.fetch_add(1, Ordering::SeqCst);
            }
        });

        // 3. Run workflow executor (with optional overall timeout)
        let mut executor = WorkflowExecutor::new(prep.driver, prep.signal_rx);
        let resume_override = prep.resume_node_id.clone();
        let exec_res = if let Some(timeout_ms) = prep.workflow_timeout_ms {
            match tokio::time::timeout(
                Duration::from_millis(timeout_ms.max(1)),
                executor.execute_workflow_from_node(
                    &prep.doc,
                    &mut prep.ctx,
                    resume_override.as_deref(),
                    Some(checkpoint_sink),
                    Some(event_callback),
                ),
            )
            .await
            {
                Ok(inner) => inner,
                Err(_) => {
                    let _ = prep.signal_tx.send(ExecutionSignal::Cancel);
                    Ok(JobState::Blocked)
                }
            }
        } else {
            executor
                .execute_workflow_from_node(
                    &prep.doc,
                    &mut prep.ctx,
                    resume_override.as_deref(),
                    Some(checkpoint_sink),
                    Some(event_callback),
                )
                .await
        };

        // 4. Stop heartbeat and unconditionally clean up scheduler/SQLite leases + active registry
        let _ = hb_stop_tx.send(());
        let _ = hb_task.await;
        self.active_jobs.write().unwrap().remove(&prep.job_id);
        if let Some(ref acc_id) = prep.account_id {
            scheduler.release_account_lease(acc_id, &prep.job_id);
        }
        scheduler.release_device_lease(&prep.device_id, &prep.job_id);
        let _ = storage.delete_device_lease_record(&prep.device_id, &prep.job_id);

        let final_node = last_node_id.lock().unwrap().clone();
        let checkpoint_id = latest_checkpoint_id.lock().unwrap().clone();
        let events_recorded = event_count.load(Ordering::SeqCst);

        match exec_res {
            Ok(state) => {
                let state_str = match state {
                    JobState::Succeeded => "succeeded",
                    JobState::Paused => "paused",
                    JobState::Cancelled => "cancelled",
                    JobState::Blocked => "blocked",
                    JobState::Failed => "failed",
                    JobState::Running => "running",
                    JobState::Ready => "ready",
                    JobState::Pending => "pending",
                    JobState::Waiting => "waiting",
                    JobState::Verifying => "verifying",
                    JobState::Unknown => "unknown",
                }
                .to_string();

                let _ = storage.update_job_progress(
                    &prep.job_id,
                    &state_str,
                    Some(prep.fencing_generation),
                    final_node.as_deref(),
                );

                let block_reason = if state == JobState::Blocked {
                    Some("Workflow stopped in BLOCKED state (timeout or ambiguous unsafe action policy)".to_string())
                } else {
                    None
                };

                Ok(RunWorkflowExecutionSummary {
                    job_id: prep.job_id,
                    project_id: prep.project.id,
                    workflow_id: prep.workflow_id,
                    workflow_version_id: prep.workflow_version_id,
                    workflow_sha256: prep.workflow_sha256,
                    task_id: prep.task_id,
                    device_id: prep.device_id,
                    account_id: prep.account_id,
                    fencing_generation: prep.fencing_generation,
                    adapter_kind: prep.adapter_kind,
                    state: state_str,
                    current_node_id: final_node,
                    checkpoint_id,
                    resumed_from_node_id: prep.resume_node_id,
                    block_reason,
                    events_recorded,
                    variables: prep.ctx.variables,
                })
            }
            Err(err) => {
                let _ = storage.update_job_progress(
                    &prep.job_id,
                    "failed",
                    Some(prep.fencing_generation),
                    final_node.as_deref(),
                );
                Err(err)
            }
        }
    }

    pub async fn start_project_workflow_background(
        &self,
        req: RunProjectWorkflowRequest,
        custom_driver: Option<Arc<dyn DeviceDriver>>,
    ) -> Result<(
        JobControlHandle,
        tokio::task::JoinHandle<Result<RunWorkflowExecutionSummary>>,
    )> {
        let prep = self.prepare_new_execution(req, custom_driver)?;
        let handle = JobControlHandle {
            job_id: prep.job_id.clone(),
            device_id: prep.device_id.clone(),
            signal_tx: prep.signal_tx.clone(),
        };
        let coord = self.clone();
        let join = tokio::spawn(async move { coord.execute_prepared(prep).await });
        Ok((handle, join))
    }

    pub async fn run_project_workflow(
        &self,
        req: RunProjectWorkflowRequest,
        custom_driver: Option<Arc<dyn DeviceDriver>>,
    ) -> Result<RunWorkflowExecutionSummary> {
        let prep = self.prepare_new_execution(req, custom_driver)?;
        self.execute_prepared(prep).await
    }

    /// Resumes an interrupted/paused job from its latest SQLite checkpoint.
    /// Enforces TWO critical safety invariants:
    /// 1. Refuses to blindly replay an ambiguous/unacknowledged non-idempotent action (`pending_unsafe_node_id.is_some()`),
    ///    transitioning the job to `blocked`.
    /// 2. Refuses to resume if workflow SHA-256 or target node is incompatible with the checkpointed graph (`blocked`).
    pub async fn resume_from_checkpoint(
        &self,
        job_id: &str,
        lease_ttl_secs: u64,
        workflow_timeout_ms: Option<u64>,
        custom_driver: Option<Arc<dyn DeviceDriver>>,
    ) -> Result<RunWorkflowExecutionSummary> {
        let resume_ctx = self
            .storage
            .load_job_resume_context(job_id)?
            .with_context(|| format!("Job '{}' resume context not found", job_id))?;

        let project_id = resume_ctx
            .job
            .project_id
            .clone()
            .context("PROJECT_ISOLATION_VIOLATION: Job has no project_id")?;
        let project = self
            .storage
            .get_project(&project_id)?
            .with_context(|| format!("Project '{}' not found", project_id))?;

        let cp = resume_ctx
            .latest_checkpoint
            .with_context(|| format!("CHECKPOINT_NOT_FOUND: No checkpoint persisted for job '{}'", job_id))?;
        let cp_state: RuntimeCheckpointState = serde_json::from_str(&cp.state_json)
            .with_context(|| format!("Corrupt checkpoint JSON in checkpoint '{}'", cp.id))?;

        let mark_blocked = |reason: String| -> Result<RunWorkflowExecutionSummary> {
            self.storage.update_job_progress(
                job_id,
                "blocked",
                Some(resume_ctx.job.fencing_generation.max(0) as u64),
                Some(&cp_state.next_node_id),
            )?;
            let ev = StepEvent {
                id: Uuid::new_v4().to_string(),
                job_id: job_id.to_string(),
                node_id: cp_state.next_node_id.clone(),
                event_type: StepEventType::VerificationFailed,
                attempt_no: 1,
                at: Utc::now(),
                payload_json: serde_json::json!({
                    "blocked": true,
                    "reason": reason,
                    "checkpointId": cp.id,
                })
                .to_string(),
            };
            let _ = self.storage.insert_step_event(&ev);
            Ok(RunWorkflowExecutionSummary {
                job_id: job_id.to_string(),
                project_id: project.id.clone(),
                workflow_id: resume_ctx.workflow_id.clone(),
                workflow_version_id: resume_ctx.job.workflow_version_id.clone(),
                workflow_sha256: resume_ctx.workflow_sha256.clone(),
                task_id: cp_state.task_id.clone(),
                device_id: resume_ctx.job.device_id.clone(),
                account_id: resume_ctx.job.account_id.clone(),
                fencing_generation: resume_ctx.job.fencing_generation.max(0) as u64,
                adapter_kind: classify_device_adapter(&resume_ctx.job.device_id, None),
                state: "blocked".to_string(),
                current_node_id: Some(cp_state.next_node_id.clone()),
                checkpoint_id: Some(cp.id.clone()),
                resumed_from_node_id: Some(cp_state.next_node_id.clone()),
                block_reason: Some(reason),
                events_recorded: 1,
                variables: cp_state.variables.clone(),
            })
        };

        // Guard 1: Unsafe unacknowledged side-effect action in progress
        if let Some(ref unsafe_node) = cp_state.pending_unsafe_node_id {
            return mark_blocked(format!(
                "BLOCKED_UNSAFE_PENDING_ACTION: Checkpoint '{}' recorded in-flight side-effect node '{}' without confirmed DeviceAck; refusing blind replay",
                cp.id, unsafe_node
            ));
        }

        // Guard 2: Workflow SHA-256 compatibility check against current workflow version AND saved draft
        if !cp_state.workflow_sha256.is_empty()
            && cp_state.workflow_sha256 != resume_ctx.workflow_sha256
        {
            return mark_blocked(format!(
                "BLOCKED_WORKFLOW_SHA256_MISMATCH: Checkpoint expects SHA-256 '{}', current version '{}' has '{}'",
                cp_state.workflow_sha256,
                resume_ctx.job.workflow_version_id,
                resume_ctx.workflow_sha256
            ));
        }

        let current_wf = self.resolve_project_workflow(&project_id, &resume_ctx.workflow_id)?;
        if current_wf.draft_json != resume_ctx.workflow_graph_json {
            return mark_blocked(format!(
                "BLOCKED_WORKFLOW_GRAPH_MODIFIED: Project workflow '{}' draft diverged from checkpointed version '{}'",
                resume_ctx.workflow_id, resume_ctx.job.workflow_version_id
            ));
        }

        let doc = WorkflowValidator::parse_and_validate(&resume_ctx.workflow_graph_json)?;
        if !doc.graph.nodes.iter().any(|n| n.id == cp_state.next_node_id) {
            return mark_blocked(format!(
                "BLOCKED_UNKNOWN_RESUME_NODE: Checkpoint target node '{}' missing from workflow '{}'",
                cp_state.next_node_id, resume_ctx.workflow_id
            ));
        }

        // Safe to resume: acquire fresh device/account leases with bumped generation
        let ttl_secs = lease_ttl_secs.max(4);
        let ttl = ChronoDuration::seconds(ttl_secs as i64);
        let device_lease = self
            .scheduler
            .acquire_device_lease(&resume_ctx.job.device_id, job_id, ttl)?;

        if let Some(ref acc_id) = resume_ctx.job.account_id {
            if let Err(err) = self.scheduler.acquire_account_lease(acc_id, job_id, ttl) {
                self.scheduler
                    .release_device_lease(&resume_ctx.job.device_id, job_id);
                return Err(err);
            }
        }

        self.storage.update_job_progress(
            job_id,
            "running",
            Some(device_lease.generation),
            Some(&cp_state.next_node_id),
        )?;
        self.storage.upsert_device_lease_record(
            &resume_ctx.job.device_id,
            job_id,
            device_lease.generation,
            &device_lease.expires_at.to_rfc3339(),
        )?;

        let (adapter_kind, driver) = self.prepare_fenced_driver(
            &resume_ctx.job.device_id,
            device_lease.generation,
            custom_driver,
        );

        let mut ctx = ExecutionContext::new(job_id.to_string(), resume_ctx.job.device_id.clone())
            .with_project_dir(&project.project_dir_path)
            .with_source_mode(match adapter_kind {
                DeviceAdapterKind::Synthetic => VariableSource::TestkitMock,
                _ => VariableSource::LiveState,
            });
        ctx.project_id = Some(project.id.clone());
        ctx.workflow_version_id = Some(resume_ctx.job.workflow_version_id.clone());
        ctx.workflow_version = Some(format!(
            "{}@v{}",
            resume_ctx.workflow_id, resume_ctx.workflow_version_no
        ));
        ctx.workflow_sha256 = Some(resume_ctx.workflow_sha256.clone());
        ctx.task_id = cp_state.task_id.clone();
        ctx.account_id = resume_ctx.job.account_id.clone();
        ctx.fencing_generation = device_lease.generation;
        ctx.variables = cp_state.variables;

        let (signal_tx, signal_rx) = watch::channel(ExecutionSignal::Running);
        self.active_jobs
            .write()
            .unwrap()
            .insert(job_id.to_string(), signal_tx.clone());

        let prep = PreparedExecution {
            job_id: job_id.to_string(),
            project,
            workflow_id: resume_ctx.workflow_id,
            workflow_version_id: resume_ctx.job.workflow_version_id,
            workflow_sha256: resume_ctx.workflow_sha256,
            doc,
            task_id: cp_state.task_id,
            device_id: resume_ctx.job.device_id,
            account_id: resume_ctx.job.account_id,
            fencing_generation: device_lease.generation,
            adapter_kind,
            lease_ttl_secs: ttl_secs,
            workflow_timeout_ms,
            resume_node_id: Some(cp_state.next_node_id),
            ctx,
            driver,
            signal_tx,
            signal_rx,
        };

        self.execute_prepared(prep).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fp_storage::{CreateProjectRequest, VisionTemplateInput};
    use fp_testkit::FakeDeviceDriver;

    fn setup_test_project(
        root: &std::path::Path,
        slug: &str,
        package_id: &str,
        workflow_id: &str,
        workflow_json: &str,
    ) -> (Arc<StorageManager>, GameProjectRecord, PathBuf) {
        let db_path = root.join("test_p3.db");
        let projects_root = root.join("projects");
        let storage = Arc::new(StorageManager::new(&db_path).unwrap());
        let project = storage
            .create_project(
                CreateProjectRequest {
                    slug: slug.to_string(),
                    name: format!("Test {}", slug),
                    game_package_id: package_id.to_string(),
                    version: Some("1.0.0".to_string()),
                    exe_name: None,
                    window_title: None,
                    icon_rel_path: None,
                    target_width_px: Some(1080),
                    target_height_px: Some(1920),
                    bundle_adb: Some(true),
                    enabled_modules: None,
                },
                &projects_root,
            )
            .unwrap();

        storage
            .save_project_workflow(
                &project.id,
                workflow_id,
                &format!("Workflow {}", workflow_id),
                "daily",
                workflow_json,
            )
            .unwrap();

        let tpl_bytes = FakeDeviceDriver::generate_template_png(120, 60);
        storage
            .save_project_template_file(
                VisionTemplateInput {
                    id: Some(format!("tpl-{}", slug)),
                    project_id: project.id.clone(),
                    name: "quest_btn".to_string(),
                    rel_path: "templates/quest_btn.png".to_string(),
                    width_px: 120,
                    height_px: 60,
                    source_crop_x: Some(200),
                    source_crop_y: Some(400),
                    default_confidence: Some(0.9),
                    sha256: "auto".to_string(),
                },
                &tpl_bytes,
            )
            .unwrap();

        (storage, project, db_path)
    }

    fn sample_vision_tap_workflow(workflow_id: &str, package_id: &str, tpl_ref: &str) -> String {
        serde_json::json!({
            "schemaVersion": 1,
            "workflowId": workflow_id,
            "name": "Vision Tap Workflow",
            "status": "published",
            "graph": {
                "nodes": [
                    { "id": "n-start", "type": "flow.start", "position": { "x": 0.0, "y": 0.0 } },
                    { "id": "n-launch", "type": "app.launch", "position": { "x": 100.0, "y": 0.0 }, "config": { "packageId": package_id } },
                    { "id": "n-cap", "type": "vision.capture", "position": { "x": 200.0, "y": 0.0 } },
                    { "id": "n-find", "type": "vision.find_image", "position": { "x": 300.0, "y": 0.0 }, "config": { "templateRef": tpl_ref, "minConfidence": 0.85 } },
                    { "id": "n-cp", "type": "state.checkpoint", "position": { "x": 400.0, "y": 0.0 }, "config": { "name": "before_tap" } },
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
        .to_string()
    }

    #[tokio::test]
    async fn test_coordinator_runs_project_workflow_with_sqlite_events_and_fencing() {
        let tmp = std::env::temp_dir().join(format!("fp_p3_coord_ok_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();

        let wf_json =
            sample_vision_tap_workflow("wf-p3-basic", "com.test.gamea", "templates/quest_btn.png");
        let (storage, proj, _db) =
            setup_test_project(&tmp, "game-a", "com.test.gamea", "wf-p3-basic", &wf_json);

        let coord = RuntimeCoordinator::new(storage.clone(), PathBuf::from("adb"));
        let driver = Arc::new(FakeDeviceDriver::new("emulator-5554", 1080, 1920));

        let summary = coord
            .run_project_workflow(
                RunProjectWorkflowRequest {
                    job_id: Some("job-p3-basic-1".to_string()),
                    project_id: proj.id.clone(),
                    workflow_id: "wf-p3-basic".to_string(),
                    task_id: None,
                    device_id: "emulator-5554".to_string(),
                    account_id: None,
                    priority: 5,
                    lease_ttl_secs: 10,
                    workflow_timeout_ms: Some(5000),
                    initial_variables: HashMap::new(),
                },
                Some(driver.clone()),
            )
            .await
            .unwrap();

        assert_eq!(summary.state, "succeeded");
        assert_eq!(summary.fencing_generation, 1);
        assert!(summary.events_recorded >= 6);
        assert_eq!(driver.tap_count.load(Ordering::SeqCst), 1);

        let job_row = storage.get_job("job-p3-basic-1").unwrap().unwrap();
        assert_eq!(job_row.state, "succeeded");
        assert_eq!(job_row.fencing_generation, 1);

        let cps = storage.list_job_checkpoints("job-p3-basic-1").unwrap();
        assert!(!cps.is_empty());
        assert!(coord.active_device_leases().is_empty());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[tokio::test]
    async fn test_concurrent_two_devices_isolation_and_same_device_lease_conflict() {
        let tmp = std::env::temp_dir().join(format!("fp_p3_coord_2dev_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();

        let wf_slow = serde_json::json!({
            "schemaVersion": 1,
            "workflowId": "wf-slow",
            "name": "Slow Device Isolation",
            "status": "published",
            "graph": {
                "nodes": [
                    { "id": "n-start", "type": "flow.start", "position": { "x": 0.0, "y": 0.0 } },
                    { "id": "n-wait", "type": "control.wait", "position": { "x": 100.0, "y": 0.0 }, "config": { "durationMs": 220 } },
                    { "id": "n-key", "type": "action.key", "position": { "x": 200.0, "y": 0.0 }, "config": { "keycode": 4 } },
                    { "id": "n-end", "type": "flow.end", "position": { "x": 300.0, "y": 0.0 } }
                ],
                "edges": [
                    { "id": "e1", "source": "n-start", "sourcePort": "flow", "target": "n-wait", "targetPort": "flow" },
                    { "id": "e2", "source": "n-wait", "sourcePort": "flow", "target": "n-key", "targetPort": "flow" },
                    { "id": "e3", "source": "n-key", "sourcePort": "flow", "target": "n-end", "targetPort": "flow" }
                ]
            }
        })
        .to_string();

        let (storage, proj, _db) =
            setup_test_project(&tmp, "game-2dev", "com.test.two", "wf-slow", &wf_slow);
        let coord = RuntimeCoordinator::new(storage.clone(), PathBuf::from("adb"));

        let drv1 = Arc::new(FakeDeviceDriver::new("emulator-5554", 1080, 1920));
        let drv2 = Arc::new(FakeDeviceDriver::new("127.0.0.1:62001", 720, 1280));

        let (_h1, j1) = coord
            .start_project_workflow_background(
                RunProjectWorkflowRequest {
                    job_id: Some("job-dev1".to_string()),
                    project_id: proj.id.clone(),
                    workflow_id: "wf-slow".to_string(),
                    task_id: None,
                    device_id: "emulator-5554".to_string(),
                    account_id: None,
                    priority: 1,
                    lease_ttl_secs: 10,
                    workflow_timeout_ms: None,
                    initial_variables: HashMap::new(),
                },
                Some(drv1.clone()),
            )
            .await
            .unwrap();

        // Trying to lease the SAME device while job-dev1 is active must be rejected
        let conflict = coord
            .run_project_workflow(
                RunProjectWorkflowRequest {
                    job_id: Some("job-dev1-dup".to_string()),
                    project_id: proj.id.clone(),
                    workflow_id: "wf-slow".to_string(),
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
            .await;
        assert!(conflict.is_err());

        // Different device executes concurrently with complete command isolation
        let (_h2, j2) = coord
            .start_project_workflow_background(
                RunProjectWorkflowRequest {
                    job_id: Some("job-dev2".to_string()),
                    project_id: proj.id.clone(),
                    workflow_id: "wf-slow".to_string(),
                    task_id: None,
                    device_id: "127.0.0.1:62001".to_string(),
                    account_id: None,
                    priority: 1,
                    lease_ttl_secs: 10,
                    workflow_timeout_ms: None,
                    initial_variables: HashMap::new(),
                },
                Some(drv2.clone()),
            )
            .await
            .unwrap();

        let s1 = j1.await.unwrap().unwrap();
        let s2 = j2.await.unwrap().unwrap();
        assert_eq!(s1.state, "succeeded");
        assert_eq!(s2.state, "succeeded");
        assert_eq!(drv1.key_events.lock().await.as_slice(), &[4]);
        assert_eq!(drv2.key_events.lock().await.as_slice(), &[4]);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[tokio::test]
    async fn test_cross_project_template_isolation_blocked() {
        let tmp = std::env::temp_dir().join(format!("fp_p3_xproj_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();

        let db_path = tmp.join("xproj.db");
        let projects_root = tmp.join("projects");
        let storage = Arc::new(StorageManager::new(&db_path).unwrap());

        let proj_a = storage
            .create_project(
                CreateProjectRequest {
                    slug: "alpha-game".to_string(),
                    name: "Alpha Game".to_string(),
                    game_package_id: "com.alpha".to_string(),
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

        let proj_b = storage
            .create_project(
                CreateProjectRequest {
                    slug: "beta-game".to_string(),
                    name: "Beta Game".to_string(),
                    game_package_id: "com.beta".to_string(),
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

        // Create template inside Project B only
        let b_tpl = PathBuf::from(&proj_b.project_dir_path)
            .join("templates")
            .join("beta_secret.png");
        std::fs::create_dir_all(b_tpl.parent().unwrap()).unwrap();
        std::fs::write(&b_tpl, FakeDeviceDriver::generate_template_png(120, 60)).unwrap();

        // Workflow owned by Project A attempts to escape into Project B's absolute path and relative .. path
        let wf_escape = sample_vision_tap_workflow(
            "wf-alpha-escape",
            "com.alpha",
            &b_tpl.to_string_lossy(),
        );
        storage
            .save_project_workflow(&proj_a.id, "wf-alpha-escape", "Escape", "daily", &wf_escape)
            .unwrap();

        let coord = RuntimeCoordinator::new(storage.clone(), PathBuf::from("adb"));
        let drv = Arc::new(FakeDeviceDriver::new("fake-xproj", 1080, 1920));

        let res = coord
            .run_project_workflow(
                RunProjectWorkflowRequest {
                    job_id: Some("job-escape".to_string()),
                    project_id: proj_a.id.clone(),
                    workflow_id: "wf-alpha-escape".to_string(),
                    task_id: None,
                    device_id: "fake-xproj".to_string(),
                    account_id: None,
                    priority: 1,
                    lease_ttl_secs: 10,
                    workflow_timeout_ms: None,
                    initial_variables: HashMap::new(),
                },
                Some(drv.clone()),
            )
            .await
            .unwrap();

        // Must fail without tapping because Project A cannot read Project B's template file!
        assert_eq!(res.state, "failed");
        assert_eq!(drv.tap_count.load(Ordering::SeqCst), 0);

        // Also test Project B workflow cannot be executed under Project A context
        let err = coord
            .run_project_workflow(
                RunProjectWorkflowRequest {
                    job_id: Some("job-wrong-wf".to_string()),
                    project_id: proj_b.id.clone(),
                    workflow_id: "wf-alpha-escape".to_string(),
                    task_id: None,
                    device_id: "fake-xproj".to_string(),
                    account_id: None,
                    priority: 1,
                    lease_ttl_secs: 10,
                    workflow_timeout_ms: None,
                    initial_variables: HashMap::new(),
                },
                Some(drv),
            )
            .await;
        assert!(err
            .unwrap_err()
            .to_string()
            .contains("PROJECT_ISOLATION_VIOLATION"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[tokio::test]
    async fn test_checkpoint_suspend_resume_and_unsafe_pending_block_and_sha_mismatch() {
        let tmp = std::env::temp_dir().join(format!("fp_p3_cp_res_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();

        let wf_json = serde_json::json!({
            "schemaVersion": 1,
            "workflowId": "wf-cp-test",
            "name": "Checkpoint Resume Test",
            "status": "published",
            "graph": {
                "nodes": [
                    { "id": "n-start", "type": "flow.start", "position": { "x": 0.0, "y": 0.0 } },
                    { "id": "n-wait", "type": "control.wait", "position": { "x": 100.0, "y": 0.0 }, "config": { "durationMs": 220 } },
                    { "id": "n-cp", "type": "state.checkpoint", "position": { "x": 200.0, "y": 0.0 }, "config": { "name": "safe_gate" } },
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

        let (storage, proj, db_path) =
            setup_test_project(&tmp, "game-cp", "com.cp.game", "wf-cp-test", &wf_json);
        let coord = RuntimeCoordinator::new(storage.clone(), PathBuf::from("adb"));
        let drv = Arc::new(FakeDeviceDriver::new("dev-cp", 1080, 1920));

        // 1. Start job, then suspend to checkpoint cleanly before n-key runs
        let (handle, join) = coord
            .start_project_workflow_background(
                RunProjectWorkflowRequest {
                    job_id: Some("job-suspend-1".to_string()),
                    project_id: proj.id.clone(),
                    workflow_id: "wf-cp-test".to_string(),
                    task_id: None,
                    device_id: "dev-cp".to_string(),
                    account_id: None,
                    priority: 1,
                    lease_ttl_secs: 10,
                    workflow_timeout_ms: None,
                    initial_variables: HashMap::new(),
                },
                Some(drv.clone()),
            )
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(60)).await;
        handle.suspend_to_checkpoint().unwrap();

        let paused_summary = join.await.unwrap().unwrap();
        assert_eq!(paused_summary.state, "paused");
        assert!(drv.key_events.lock().await.is_empty());

        // 2. Resume from safe checkpoint -> finishes execution cleanly & increments generation!
        let resumed = coord
            .resume_from_checkpoint("job-suspend-1", 10, Some(5000), Some(drv.clone()))
            .await
            .unwrap();
        assert_eq!(resumed.state, "succeeded");
        assert_eq!(resumed.fencing_generation, 2);
        assert_eq!(drv.key_events.lock().await.as_slice(), &[66]);

        // 3. Test unsafe pending side-effect guard -> BLOCKED without repeating action
        let latest_cp = storage.get_latest_checkpoint("job-suspend-1").unwrap().unwrap();
        let mut unsafe_state: RuntimeCheckpointState =
            serde_json::from_str(&latest_cp.state_json).unwrap();
        unsafe_state.next_node_id = "n-key".to_string();
        unsafe_state.pending_unsafe_node_id = Some("n-key".to_string());
        unsafe_state.resume_policy = "unsafe_pending_block".to_string();
        storage
            .save_checkpoint(CheckpointRecordInput {
                id: None,
                job_id: "job-suspend-1".to_string(),
                node_id: "n-key".to_string(),
                state_json: serde_json::to_string(&unsafe_state).unwrap(),
                evidence_ref: None,
            })
            .unwrap();

        let blocked_unsafe = coord
            .resume_from_checkpoint("job-suspend-1", 10, Some(5000), Some(drv.clone()))
            .await
            .unwrap();
        assert_eq!(blocked_unsafe.state, "blocked");
        assert!(blocked_unsafe
            .block_reason
            .unwrap()
            .contains("BLOCKED_UNSAFE_PENDING_ACTION"));
        // Still only 1 key event (refused blind replay!)
        assert_eq!(drv.key_events.lock().await.len(), 1);

        // 4. Test workflow SHA-256 mismatch guard -> BLOCKED
        let mut sha_cp = unsafe_state;
        sha_cp.pending_unsafe_node_id = None;
        sha_cp.workflow_sha256 = "0000deadbeef".to_string();
        storage
            .save_checkpoint(CheckpointRecordInput {
                id: None,
                job_id: "job-suspend-1".to_string(),
                node_id: "n-cp".to_string(),
                state_json: serde_json::to_string(&sha_cp).unwrap(),
                evidence_ref: None,
            })
            .unwrap();

        let blocked_sha = coord
            .resume_from_checkpoint("job-suspend-1", 10, Some(5000), Some(drv.clone()))
            .await
            .unwrap();
        assert_eq!(blocked_sha.state, "blocked");
        assert!(blocked_sha
            .block_reason
            .unwrap()
            .contains("BLOCKED_WORKFLOW_SHA256_MISMATCH"));

        // Verify SQLite FK/integrity remains spotless
        assert!(db_path.exists());
        let integrity = storage.check_integrity().unwrap();
        assert!(integrity.is_healthy);
        assert_eq!(integrity.foreign_key_violations, 0);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[tokio::test]
    async fn test_pause_resume_cancel_and_timeout_guards() {
        let tmp = std::env::temp_dir().join(format!("fp_p3_ctrl_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();

        let wf_slow = serde_json::json!({
            "schemaVersion": 1,
            "workflowId": "wf-ctrl",
            "name": "Control Signals Test",
            "status": "published",
            "graph": {
                "nodes": [
                    { "id": "n-start", "type": "flow.start", "position": { "x": 0.0, "y": 0.0 } },
                    { "id": "n-w1", "type": "control.wait", "position": { "x": 100.0, "y": 0.0 }, "config": { "durationMs": 80 } },
                    { "id": "n-w2", "type": "control.wait", "position": { "x": 200.0, "y": 0.0 }, "config": { "durationMs": 80 } },
                    { "id": "n-key", "type": "action.key", "position": { "x": 300.0, "y": 0.0 }, "config": { "keycode": 3 } },
                    { "id": "n-end", "type": "flow.end", "position": { "x": 400.0, "y": 0.0 } }
                ],
                "edges": [
                    { "id": "e1", "source": "n-start", "sourcePort": "flow", "target": "n-w1", "targetPort": "flow" },
                    { "id": "e2", "source": "n-w1", "sourcePort": "flow", "target": "n-w2", "targetPort": "flow" },
                    { "id": "e3", "source": "n-w2", "sourcePort": "flow", "target": "n-key", "targetPort": "flow" },
                    { "id": "e4", "source": "n-key", "sourcePort": "flow", "target": "n-end", "targetPort": "flow" }
                ]
            }
        })
        .to_string();

        let (storage, proj, _) =
            setup_test_project(&tmp, "game-ctrl", "com.ctrl", "wf-ctrl", &wf_slow);
        let coord = RuntimeCoordinator::new(storage.clone(), PathBuf::from("adb"));
        let drv = Arc::new(FakeDeviceDriver::new("dev-ctrl", 1080, 1920));

        // Pause -> Resume -> Succeed
        let (handle, join) = coord
            .start_project_workflow_background(
                RunProjectWorkflowRequest {
                    job_id: Some("job-pause-resume".to_string()),
                    project_id: proj.id.clone(),
                    workflow_id: "wf-ctrl".to_string(),
                    task_id: None,
                    device_id: "dev-ctrl".to_string(),
                    account_id: None,
                    priority: 1,
                    lease_ttl_secs: 10,
                    workflow_timeout_ms: Some(5000),
                    initial_variables: HashMap::new(),
                },
                Some(drv.clone()),
            )
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(25)).await;
        handle.pause().unwrap();
        tokio::time::sleep(Duration::from_millis(120)).await;
        assert!(drv.key_events.lock().await.is_empty());
        handle.resume().unwrap();

        let res = join.await.unwrap().unwrap();
        assert_eq!(res.state, "succeeded");
        assert_eq!(drv.key_events.lock().await.as_slice(), &[3]);

        // Cancel stops early & cleans up lease
        let (h_cancel, j_cancel) = coord
            .start_project_workflow_background(
                RunProjectWorkflowRequest {
                    job_id: Some("job-cancel".to_string()),
                    project_id: proj.id.clone(),
                    workflow_id: "wf-ctrl".to_string(),
                    task_id: None,
                    device_id: "dev-ctrl".to_string(),
                    account_id: None,
                    priority: 1,
                    lease_ttl_secs: 10,
                    workflow_timeout_ms: None,
                    initial_variables: HashMap::new(),
                },
                Some(drv.clone()),
            )
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(25)).await;
        h_cancel.cancel().unwrap();
        let cancelled = j_cancel.await.unwrap().unwrap();
        assert_eq!(cancelled.state, "cancelled");
        assert!(coord.active_device_leases().is_empty());

        // Timeout blocks cleanly & cleans up lease
        let timed_out = coord
            .run_project_workflow(
                RunProjectWorkflowRequest {
                    job_id: Some("job-timeout".to_string()),
                    project_id: proj.id.clone(),
                    workflow_id: "wf-ctrl".to_string(),
                    task_id: None,
                    device_id: "dev-ctrl".to_string(),
                    account_id: None,
                    priority: 1,
                    lease_ttl_secs: 10,
                    workflow_timeout_ms: Some(40),
                    initial_variables: HashMap::new(),
                },
                Some(drv),
            )
            .await
            .unwrap();
        assert_eq!(timed_out.state, "blocked");
        assert!(coord.active_device_leases().is_empty());

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
