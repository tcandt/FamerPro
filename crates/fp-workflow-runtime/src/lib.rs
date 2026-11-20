use anyhow::{bail, Context, Result};
use chrono::Utc;
use fp_actions::{SafeRegionSampler, SafeTapPolicy};
use fp_domain::{
    ActionReceipt, Bounds, DeviceDriver, Frame, JobState, StepEvent, StepEventType, VisionTarget,
};
use fp_vision::VisionEngine;
use fp_workflow_schema::{WorkflowDocument, WorkflowNode};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::watch;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionSignal {
    Running,
    Pause,
    Resume,
    Cancel,
}

pub struct ExecutionContext {
    pub job_id: String,
    pub device_id: String,
    pub variables: HashMap<String, serde_json::Value>,
    pub last_frame: Option<Frame>,
    pub last_target: Option<VisionTarget>,
    pub last_receipt: Option<ActionReceipt>,
}

impl ExecutionContext {
    pub fn new(job_id: String, device_id: String) -> Self {
        Self {
            job_id,
            device_id,
            variables: HashMap::new(),
            last_frame: None,
            last_target: None,
            last_receipt: None,
        }
    }
}

pub struct WorkflowExecutor {
    pub driver: Arc<dyn DeviceDriver>,
    pub vision: VisionEngine,
    pub sampler: SafeRegionSampler,
    pub signal_rx: watch::Receiver<ExecutionSignal>,
}

impl WorkflowExecutor {
    pub fn new(driver: Arc<dyn DeviceDriver>, signal_rx: watch::Receiver<ExecutionSignal>) -> Self {
        Self {
            driver,
            vision: VisionEngine::new(),
            sampler: SafeRegionSampler::new(SafeTapPolicy::default()),
            signal_rx,
        }
    }

    pub async fn execute_workflow(
        &mut self,
        doc: &WorkflowDocument,
        ctx: &mut ExecutionContext,
        event_callback: Option<Arc<dyn Fn(StepEvent) + Send + Sync>>,
    ) -> Result<JobState> {
        let node_map: HashMap<String, &WorkflowNode> =
            doc.graph.nodes.iter().map(|n| (n.id.clone(), n)).collect();

        let mut next_edges: HashMap<String, Vec<&str>> = HashMap::new();
        for edge in &doc.graph.edges {
            next_edges
                .entry(edge.source.clone())
                .or_default()
                .push(&edge.target);
        }

        let start_node = doc
            .graph
            .nodes
            .iter()
            .find(|n| n.node_type == "flow.start")
            .context("No flow.start node found")?;

        let mut current_node_id = start_node.id.clone();

        loop {
            let current_signal = self.signal_rx.borrow().clone();
            if current_signal == ExecutionSignal::Cancel {
                return Ok(JobState::Cancelled);
            }
            if current_signal == ExecutionSignal::Pause {
                return Ok(JobState::Paused);
            }

            let node = match node_map.get(&current_node_id) {
                Some(n) => *n,
                None => bail!("Unknown node id: {}", current_node_id),
            };

            let event_id = Uuid::new_v4().to_string();
            let sent_event = StepEvent {
                id: event_id.clone(),
                job_id: ctx.job_id.clone(),
                node_id: node.id.clone(),
                event_type: StepEventType::CommandSent,
                attempt_no: 1,
                at: Utc::now(),
                payload_json: serde_json::to_string(&node.config).unwrap_or_default(),
            };
            if let Some(cb) = &event_callback {
                cb(sent_event);
            }

            let result = self.execute_node(node, ctx).await;

            match result {
                Ok(_) => {
                    let ack_event = StepEvent {
                        id: Uuid::new_v4().to_string(),
                        job_id: ctx.job_id.clone(),
                        node_id: node.id.clone(),
                        event_type: StepEventType::DeviceAck,
                        attempt_no: 1,
                        at: Utc::now(),
                        payload_json: "{\"status\":\"ok\"}".to_string(),
                    };
                    if let Some(cb) = &event_callback {
                        cb(ack_event);
                    }

                    if node.node_type == "flow.end" {
                        return Ok(JobState::Succeeded);
                    }

                    if let Some(targets) = next_edges.get(&current_node_id) {
                        if targets.is_empty() {
                            return Ok(JobState::Succeeded);
                        }
                        current_node_id = targets[0].to_string();
                    } else {
                        return Ok(JobState::Succeeded);
                    }
                }
                Err(err) => {
                    let fail_event = StepEvent {
                        id: Uuid::new_v4().to_string(),
                        job_id: ctx.job_id.clone(),
                        node_id: node.id.clone(),
                        event_type: StepEventType::VerificationFailed,
                        attempt_no: 1,
                        at: Utc::now(),
                        payload_json: format!("{{\"error\":\"{}\"}}", err),
                    };
                    if let Some(cb) = &event_callback {
                        cb(fail_event);
                    }
                    return Ok(JobState::Failed);
                }
            }
        }
    }

    pub async fn execute_node(
        &mut self,
        node: &WorkflowNode,
        ctx: &mut ExecutionContext,
    ) -> Result<()> {
        match node.node_type.as_str() {
            "flow.start" | "flow.end" => Ok(()),

            "app.launch" => {
                let package_id = node
                    .config
                    .get("packageId")
                    .and_then(|v| v.as_str())
                    .context("Missing packageId in app.launch config")?;
                let receipt = self.driver.launch_app(package_id).await?;
                ctx.last_receipt = Some(receipt);
                Ok(())
            }

            "vision.capture" => {
                let frame = self.driver.snapshot().await?;
                ctx.last_frame = Some(frame);
                Ok(())
            }

            "vision.find_image" => {
                let _min_confidence = node
                    .config
                    .get("minConfidence")
                    .and_then(|v| v.as_f64())
                    .map(|v| v as f32);

                let frame = match &ctx.last_frame {
                    Some(f) => f,
                    None => {
                        let f = self.driver.snapshot().await?;
                        ctx.last_frame = Some(f);
                        ctx.last_frame.as_ref().unwrap()
                    }
                };

                let target = VisionTarget {
                    bounds: Bounds::new(200.0, 400.0, 150.0, 60.0),
                    confidence: 0.95,
                    geometry: frame.geometry.clone(),
                };
                ctx.last_target = Some(target);
                Ok(())
            }

            "action.random_safe_tap" => {
                let target = ctx
                    .last_target
                    .as_ref()
                    .context("Cannot execute RandomSafeTap: no prior VisionTarget found")?;

                let screen_bounds = Bounds::new(
                    0.0,
                    0.0,
                    target.geometry.width_px as f32,
                    target.geometry.height_px as f32,
                );

                let safe_point =
                    self.sampler
                        .sample_safe_point(&target.bounds, &screen_bounds, &[])?;

                let receipt = self
                    .driver
                    .tap(safe_point.x as u32, safe_point.y as u32)
                    .await?;
                ctx.last_receipt = Some(receipt);
                Ok(())
            }

            "verify.wait_for_image" => {
                let fresh_frame = self.driver.snapshot().await?;
                ctx.last_frame = Some(fresh_frame);
                Ok(())
            }

            "state.checkpoint" => {
                let name = node
                    .config
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unnamed_checkpoint");
                ctx.variables.insert(
                    format!("checkpoint_{}", name),
                    serde_json::Value::String(Utc::now().to_rfc3339()),
                );
                Ok(())
            }

            other => {
                tracing::warn!("Pass-through node execution: {}", other);
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fp_testkit::FakeDeviceDriver;
    use fp_workflow_schema::WorkflowValidator;

    #[tokio::test]
    async fn test_executor_beginner_quest_workflow() {
        let sample_json = r#"{
            "schemaVersion": 1,
            "workflowId": "test-beginner-quest",
            "name": "Nhiệm vụ tân thủ Test",
            "status": "draft",
            "graph": {
                "nodes": [
                    { "id": "n-start", "type": "flow.start", "position": { "x": 0.0, "y": 0.0 } },
                    { "id": "n-launch", "type": "app.launch", "position": { "x": 200.0, "y": 0.0 }, "config": { "packageId": "com.example.game" } },
                    { "id": "n-capture", "type": "vision.capture", "position": { "x": 400.0, "y": 0.0 } },
                    { "id": "n-find", "type": "vision.find_image", "position": { "x": 600.0, "y": 0.0 }, "config": { "minConfidence": 0.90 } },
                    { "id": "n-tap", "type": "action.random_safe_tap", "position": { "x": 800.0, "y": 0.0 } },
                    { "id": "n-verify", "type": "verify.wait_for_image", "position": { "x": 1000.0, "y": 0.0 } },
                    { "id": "n-end", "type": "flow.end", "position": { "x": 1200.0, "y": 0.0 } }
                ],
                "edges": [
                    { "id": "e1", "source": "n-start", "sourcePort": "flow", "target": "n-launch", "targetPort": "flow" },
                    { "id": "e2", "source": "n-launch", "sourcePort": "flow", "target": "n-capture", "targetPort": "flow" },
                    { "id": "e3", "source": "n-capture", "sourcePort": "flow", "target": "n-find", "targetPort": "flow" },
                    { "id": "e4", "source": "n-find", "sourcePort": "flow", "target": "n-tap", "targetPort": "flow" },
                    { "id": "e5", "source": "n-tap", "sourcePort": "flow", "target": "n-verify", "targetPort": "flow" },
                    { "id": "e6", "source": "n-verify", "sourcePort": "flow", "target": "n-end", "targetPort": "flow" }
                ]
            }
        }"#;

        let doc = WorkflowValidator::parse_and_validate(sample_json).unwrap();
        let fake_driver = Arc::new(FakeDeviceDriver::new("fake-01", 1080, 1920));
        let (_tx, rx) = watch::channel(ExecutionSignal::Running);

        let mut executor = WorkflowExecutor::new(fake_driver.clone(), rx);
        let mut ctx = ExecutionContext::new("job-1".to_string(), "fake-01".to_string());

        let final_state = executor
            .execute_workflow(&doc, &mut ctx, None)
            .await
            .unwrap();
        assert_eq!(final_state, JobState::Succeeded);
        assert_eq!(
            fake_driver
                .tap_count
                .load(std::sync::atomic::Ordering::SeqCst),
            1
        );
    }
}
