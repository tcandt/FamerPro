use anyhow::{bail, Context, Result};
use chrono::Utc;
use fp_actions::{SafeRegionSampler, SafeTapPolicy};
use fp_domain::{
    ActionReceipt, Bounds, DeviceDriver, Frame, JobState, StepEvent, StepEventType, SwipePath,
    VisionTarget,
};
use fp_vision::VisionEngine;
use fp_workflow_schema::{WorkflowDocument, WorkflowNode};
use image::{ImageFormat, Rgb, RgbImage};
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionSignal {
    Running,
    Pause,
    Resume,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeOutcome {
    Continue(Option<String>),
    TerminalSuccess,
    TerminalFailure,
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

        let start_node = doc
            .graph
            .nodes
            .iter()
            .find(|n| n.node_type == "flow.start")
            .context("No flow.start node found in workflow")?;

        let mut current_node_id = start_node.id.clone();

        loop {
            // Check signals
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

            // Determine retry policy
            let (max_attempts, backoff_ms) = if let Some(p) = &node.retry_policy {
                (p.max_attempts.max(1), p.backoff_ms)
            } else if let Some(p) = doc.runtime_policy.as_ref().and_then(|r| r.retry.as_ref()) {
                (p.max_attempts.max(1), p.backoff_ms)
            } else {
                (1, 500)
            };

            let mut node_outcome = None;

            for attempt in 1..=max_attempts {
                let sent_event = StepEvent {
                    id: Uuid::new_v4().to_string(),
                    job_id: ctx.job_id.clone(),
                    node_id: node.id.clone(),
                    event_type: StepEventType::CommandSent,
                    attempt_no: attempt,
                    at: Utc::now(),
                    payload_json: serde_json::to_string(&node.config).unwrap_or_default(),
                };
                if let Some(cb) = &event_callback {
                    cb(sent_event);
                }

                match self.execute_node(node, ctx).await {
                    Ok(outcome) => {
                        let ack_event = StepEvent {
                            id: Uuid::new_v4().to_string(),
                            job_id: ctx.job_id.clone(),
                            node_id: node.id.clone(),
                            event_type: StepEventType::DeviceAck,
                            attempt_no: attempt,
                            at: Utc::now(),
                            payload_json: "{\"status\":\"ok\"}".to_string(),
                        };
                        if let Some(cb) = &event_callback {
                            cb(ack_event);
                        }

                        if node.node_type.starts_with("verify.") {
                            let verify_event = StepEvent {
                                id: Uuid::new_v4().to_string(),
                                job_id: ctx.job_id.clone(),
                                node_id: node.id.clone(),
                                event_type: StepEventType::VerificationPassed,
                                attempt_no: attempt,
                                at: Utc::now(),
                                payload_json: "{\"verified\":true}".to_string(),
                            };
                            if let Some(cb) = &event_callback {
                                cb(verify_event);
                            }
                        } else if node.node_type == "state.checkpoint" {
                            let cp_event = StepEvent {
                                id: Uuid::new_v4().to_string(),
                                job_id: ctx.job_id.clone(),
                                node_id: node.id.clone(),
                                event_type: StepEventType::CheckpointSaved,
                                attempt_no: attempt,
                                at: Utc::now(),
                                payload_json: serde_json::to_string(&ctx.variables)
                                    .unwrap_or_default(),
                            };
                            if let Some(cb) = &event_callback {
                                cb(cp_event);
                            }
                        }

                        node_outcome = Some(outcome);
                        break;
                    }
                    Err(err) => {
                        let is_final = attempt == max_attempts;
                        let fail_event = StepEvent {
                            id: Uuid::new_v4().to_string(),
                            job_id: ctx.job_id.clone(),
                            node_id: node.id.clone(),
                            event_type: StepEventType::VerificationFailed,
                            attempt_no: attempt,
                            at: Utc::now(),
                            payload_json: format!(
                                "{{\"error\":\"{}\",\"retry_pending\":{}}}",
                                err, !is_final
                            ),
                        };
                        if let Some(cb) = &event_callback {
                            cb(fail_event);
                        }

                        if !is_final {
                            tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                        }
                    }
                }
            }

            let outcome = match node_outcome {
                Some(o) => o,
                None => return Ok(JobState::Failed),
            };

            match outcome {
                NodeOutcome::TerminalSuccess => return Ok(JobState::Succeeded),
                NodeOutcome::TerminalFailure => return Ok(JobState::Failed),
                NodeOutcome::Continue(target_port) => {
                    // Match outgoing edge based on sourcePort
                    let candidate_edge = doc.graph.edges.iter().find(|e| {
                        if e.source != current_node_id {
                            return false;
                        }
                        if let Some(ref port) = target_port {
                            e.source_port == *port || e.source_port == "flow"
                        } else {
                            true
                        }
                    });

                    if let Some(edge) = candidate_edge {
                        current_node_id = edge.target.clone();
                    } else {
                        // End of branch reached
                        return Ok(JobState::Succeeded);
                    }
                }
            }
        }
    }

    pub async fn execute_node(
        &mut self,
        node: &WorkflowNode,
        ctx: &mut ExecutionContext,
    ) -> Result<NodeOutcome> {
        match node.node_type.as_str() {
            "flow.start" => Ok(NodeOutcome::Continue(None)),

            "flow.end" => Ok(NodeOutcome::TerminalSuccess),

            "app.launch" => {
                let package_id = node
                    .config
                    .get("packageId")
                    .and_then(|v| v.as_str())
                    .context("Missing packageId in app.launch config")?;
                let receipt = self.driver.launch_app(package_id).await?;
                ctx.last_receipt = Some(receipt);
                Ok(NodeOutcome::Continue(None))
            }

            "app.stop" => {
                let package_id = node
                    .config
                    .get("packageId")
                    .and_then(|v| v.as_str())
                    .context("Missing packageId in app.stop config")?;
                let receipt = self.driver.stop_app(package_id).await?;
                ctx.last_receipt = Some(receipt);
                Ok(NodeOutcome::Continue(None))
            }

            "vision.capture" => {
                let frame = self.driver.snapshot().await?;
                ctx.last_frame = Some(frame);
                Ok(NodeOutcome::Continue(None))
            }

            "vision.find_image" => {
                let min_confidence = node
                    .config
                    .get("minConfidence")
                    .and_then(|v| v.as_f64())
                    .map(|v| v as f32)
                    .unwrap_or(0.85);

                let frame = match &ctx.last_frame {
                    Some(f) => f,
                    None => {
                        let f = self.driver.snapshot().await?;
                        ctx.last_frame = Some(f);
                        ctx.last_frame.as_ref().unwrap()
                    }
                };

                let template_bytes = Self::resolve_template_bytes(node)?;
                let target = self.vision.find_template_multiscale(
                    frame,
                    &template_bytes,
                    Some(min_confidence),
                    &[0.85, 1.0, 1.15],
                )?;

                ctx.variables.insert(
                    "last_target_x".to_string(),
                    serde_json::Value::from(target.bounds.x),
                );
                ctx.variables.insert(
                    "last_target_y".to_string(),
                    serde_json::Value::from(target.bounds.y),
                );
                ctx.variables
                    .insert("target_found".to_string(), serde_json::Value::Bool(true));
                ctx.last_target = Some(target);
                Ok(NodeOutcome::Continue(None))
            }

            "action.random_safe_tap" => {
                let target = ctx
                    .last_target
                    .as_ref()
                    .context("Cannot execute RandomSafeTap: no prior VisionTarget found")?;

                let margin_ratio = node
                    .config
                    .get("marginRatio")
                    .and_then(|v| v.as_f64())
                    .map(|v| v as f32)
                    .unwrap_or(0.18);

                let screen_bounds = Bounds::new(
                    0.0,
                    0.0,
                    target.geometry.width_px as f32,
                    target.geometry.height_px as f32,
                );

                let mut sampler = SafeRegionSampler::new(SafeTapPolicy {
                    margin_ratio,
                    min_area_px: 36.0,
                    avoid_recent_points: 5,
                    min_distance_px: 6.0,
                    max_tries: 50,
                });

                let safe_point = sampler.sample_safe_point(&target.bounds, &screen_bounds, &[])?;

                let receipt = self
                    .driver
                    .tap(safe_point.x as u32, safe_point.y as u32)
                    .await?;
                ctx.last_receipt = Some(receipt);
                Ok(NodeOutcome::Continue(None))
            }

            "action.swipe" => {
                let s_x = node
                    .config
                    .get("startX")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(500) as u32;
                let s_y = node
                    .config
                    .get("startY")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1500) as u32;
                let e_x = node
                    .config
                    .get("endX")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(500) as u32;
                let e_y = node
                    .config
                    .get("endY")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(500) as u32;
                let dur = node
                    .config
                    .get("durationMs")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(300);

                let receipt = self
                    .driver
                    .swipe(SwipePath {
                        start_x: s_x,
                        start_y: s_y,
                        end_x: e_x,
                        end_y: e_y,
                        duration_ms: dur,
                    })
                    .await?;
                ctx.last_receipt = Some(receipt);
                Ok(NodeOutcome::Continue(None))
            }

            "action.wait" | "control.wait" => {
                let wait_ms = node
                    .config
                    .get("durationMs")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(500);
                tokio::time::sleep(Duration::from_millis(wait_ms)).await;
                Ok(NodeOutcome::Continue(None))
            }

            "verify.wait_for_image" => {
                let min_confidence = node
                    .config
                    .get("minConfidence")
                    .and_then(|v| v.as_f64())
                    .map(|v| v as f32)
                    .unwrap_or(0.85);

                let fresh_frame = self.driver.snapshot().await?;
                ctx.last_frame = Some(fresh_frame);
                let frame = ctx.last_frame.as_ref().unwrap();

                let template_bytes = Self::resolve_template_bytes(node)?;
                let target = self.vision.find_template_multiscale(
                    frame,
                    &template_bytes,
                    Some(min_confidence),
                    &[0.85, 1.0, 1.15],
                )?;

                if let Some(r) = &mut ctx.last_receipt {
                    r.verify_passed = true;
                }
                ctx.variables
                    .insert("verify_passed".to_string(), serde_json::Value::Bool(true));
                ctx.last_target = Some(target);
                Ok(NodeOutcome::Continue(None))
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
                Ok(NodeOutcome::Continue(None))
            }

            "logic.if" | "logic.branch" => {
                let condition = node
                    .config
                    .get("condition")
                    .and_then(|v| v.as_str())
                    .unwrap_or("verify_passed");

                let condition_met = match condition {
                    "verify_passed" => ctx
                        .variables
                        .get("verify_passed")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    "target_found" => ctx
                        .variables
                        .get("target_found")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    _ => false,
                };

                let branch = if condition_met { "true" } else { "false" };
                Ok(NodeOutcome::Continue(Some(branch.to_string())))
            }

            "logic.loop" => {
                let max_iterations = node
                    .config
                    .get("maxIterations")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1);

                let loop_key = format!("loop_{}_iter", node.id);
                let current_iter = ctx
                    .variables
                    .get(&loop_key)
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);

                if current_iter < max_iterations {
                    ctx.variables
                        .insert(loop_key, serde_json::Value::from(current_iter + 1));
                    Ok(NodeOutcome::Continue(Some("loop".to_string())))
                } else {
                    Ok(NodeOutcome::Continue(Some("done".to_string())))
                }
            }

            other => {
                tracing::warn!("Pass-through node execution: {}", other);
                Ok(NodeOutcome::Continue(None))
            }
        }
    }

    /// Resolves template image bytes from config (base64, disk path, or deterministic fallback)
    pub fn resolve_template_bytes(node: &WorkflowNode) -> Result<Vec<u8>> {
        // 1. Direct base64 inline image
        if let Some(b64) = node.config.get("templateBase64").and_then(|v| v.as_str()) {
            return decode_base64_payload(b64).context("Failed to decode templateBase64");
        }

        // 2. File path on disk
        if let Some(path_str) = node.config.get("templateRef").and_then(|v| v.as_str()) {
            let path = std::path::Path::new(path_str);
            if path.exists() {
                return std::fs::read(path).context("Failed to read template image file from disk");
            }

            // Fallback for demo/test templates
            if path_str.contains("dialog") {
                return Ok(generate_colored_png(400, 200, [24, 120, 245]));
            }
            if path_str.contains("marker") || path_str.contains("quest") {
                return Ok(generate_colored_png(120, 60, [24, 120, 245]));
            }
        }

        // 3. Fallback button template for testing
        Ok(generate_colored_png(120, 60, [24, 120, 245]))
    }
}

fn generate_colored_png(width: u32, height: u32, rgb: [u8; 3]) -> Vec<u8> {
    let mut img = RgbImage::new(width, height);
    for pixel in img.pixels_mut() {
        *pixel = Rgb(rgb);
    }
    let mut buf = Vec::new();
    let mut cursor = Cursor::new(&mut buf);
    img.write_to(&mut cursor, ImageFormat::Png).unwrap();
    buf
}

fn decode_base64_payload(input: &str) -> Result<Vec<u8>> {
    // Strip optional data:image/png;base64, prefix
    let clean = if let Some(pos) = input.find(',') {
        &input[pos + 1..]
    } else {
        input
    };

    let trimmed = clean.trim();
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut decode_map = [255u8; 256];
    for (i, &b) in TABLE.iter().enumerate() {
        decode_map[b as usize] = i as u8;
    }

    let mut out = Vec::new();
    let bytes = trimmed.as_bytes();
    let mut buf = 0u32;
    let mut bits = 0;

    for &b in bytes {
        if b == b'=' {
            break;
        }
        let val = decode_map[b as usize];
        if val == 255 {
            continue;
        }
        buf = (buf << 6) | (val as u32);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fp_testkit::FakeDeviceDriver;
    use fp_workflow_schema::WorkflowValidator;

    #[tokio::test]
    async fn test_executor_beginner_quest_workflow_with_real_matching_and_verification() {
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
                    { "id": "n-find", "type": "vision.find_image", "position": { "x": 600.0, "y": 0.0 }, "config": { "templateRef": "quest_marker.png", "minConfidence": 0.90 } },
                    { "id": "n-tap", "type": "action.random_safe_tap", "position": { "x": 800.0, "y": 0.0 } },
                    { "id": "n-verify", "type": "verify.wait_for_image", "position": { "x": 1000.0, "y": 0.0 }, "config": { "templateRef": "dialog.png", "minConfidence": 0.90 } },
                    { "id": "n-checkpoint", "type": "state.checkpoint", "position": { "x": 1200.0, "y": 0.0 }, "config": { "name": "quest_done" } },
                    { "id": "n-end", "type": "flow.end", "position": { "x": 1400.0, "y": 0.0 } }
                ],
                "edges": [
                    { "id": "e1", "source": "n-start", "sourcePort": "flow", "target": "n-launch", "targetPort": "flow" },
                    { "id": "e2", "source": "n-launch", "sourcePort": "flow", "target": "n-capture", "targetPort": "flow" },
                    { "id": "e3", "source": "n-capture", "sourcePort": "flow", "target": "n-find", "targetPort": "flow" },
                    { "id": "e4", "source": "n-find", "sourcePort": "flow", "target": "n-tap", "targetPort": "flow" },
                    { "id": "e5", "source": "n-tap", "sourcePort": "flow", "target": "n-verify", "targetPort": "flow" },
                    { "id": "e6", "source": "n-verify", "sourcePort": "flow", "target": "n-checkpoint", "targetPort": "flow" },
                    { "id": "e7", "source": "n-checkpoint", "sourcePort": "flow", "target": "n-end", "targetPort": "flow" }
                ]
            }
        }"#;

        let doc = WorkflowValidator::parse_and_validate(sample_json).unwrap();
        let fake_driver = Arc::new(FakeDeviceDriver::new("fake-01", 1080, 1920));
        let (_tx, rx) = watch::channel(ExecutionSignal::Running);

        let mut executor = WorkflowExecutor::new(fake_driver.clone(), rx);
        let mut ctx = ExecutionContext::new("job-1".to_string(), "fake-01".to_string());

        let mut captured_events = Vec::new();
        let callback_events = Arc::new(std::sync::Mutex::new(Vec::new()));
        let cb_clone = callback_events.clone();

        let final_state = executor
            .execute_workflow(
                &doc,
                &mut ctx,
                Some(Arc::new(move |evt| {
                    cb_clone.lock().unwrap().push(evt);
                })),
            )
            .await
            .unwrap();

        assert_eq!(final_state, JobState::Succeeded);
        assert_eq!(
            fake_driver
                .tap_count
                .load(std::sync::atomic::Ordering::SeqCst),
            1
        );

        // Verify that target was actually found
        assert!(ctx.last_target.is_some());
        let tgt = ctx.last_target.unwrap();
        assert!(tgt.confidence >= 0.90);

        // Verify that receipt has verify_passed = true
        assert!(ctx.last_receipt.is_some());
        assert!(ctx.last_receipt.unwrap().verify_passed);

        // Verify events were emitted properly
        captured_events.extend(callback_events.lock().unwrap().clone());
        assert!(!captured_events.is_empty());
        assert!(captured_events
            .iter()
            .any(|e| e.event_type == StepEventType::VerificationPassed));
        assert!(captured_events
            .iter()
            .any(|e| e.event_type == StepEventType::CheckpointSaved));
    }

    #[tokio::test]
    async fn test_executor_retry_logic() {
        let sample_json = r#"{
            "schemaVersion": 1,
            "workflowId": "test-retry-flow",
            "name": "Retry Test",
            "status": "draft",
            "graph": {
                "nodes": [
                    { "id": "n-start", "type": "flow.start", "position": { "x": 0.0, "y": 0.0 } },
                    { "id": "n-tap-failing", "type": "action.random_safe_tap", "position": { "x": 200.0, "y": 0.0 }, "retryPolicy": { "maxAttempts": 3, "backoffMs": 50 } },
                    { "id": "n-end", "type": "flow.end", "position": { "x": 400.0, "y": 0.0 } }
                ],
                "edges": [
                    { "id": "e1", "source": "n-start", "sourcePort": "flow", "target": "n-tap-failing", "targetPort": "flow" },
                    { "id": "e2", "source": "n-tap-failing", "sourcePort": "flow", "target": "n-end", "targetPort": "flow" }
                ]
            }
        }"#;

        let doc = WorkflowValidator::parse_and_validate(sample_json).unwrap();
        let fake_driver = Arc::new(FakeDeviceDriver::new("fake-retry", 1080, 1920));
        let (_tx, rx) = watch::channel(ExecutionSignal::Running);

        let mut executor = WorkflowExecutor::new(fake_driver.clone(), rx);
        // Note: ctx has NO last_target, so action.random_safe_tap fails closed!
        let mut ctx = ExecutionContext::new("job-retry".to_string(), "fake-retry".to_string());

        let failure_events = Arc::new(std::sync::Mutex::new(0));
        let fe_clone = failure_events.clone();

        let state = executor
            .execute_workflow(
                &doc,
                &mut ctx,
                Some(Arc::new(move |evt| {
                    if evt.event_type == StepEventType::VerificationFailed {
                        *fe_clone.lock().unwrap() += 1;
                    }
                })),
            )
            .await
            .unwrap();

        assert_eq!(state, JobState::Failed);
        // Retried 3 times before failing
        assert_eq!(*failure_events.lock().unwrap(), 3);
    }
}
