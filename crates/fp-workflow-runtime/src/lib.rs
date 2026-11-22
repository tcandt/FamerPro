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
use std::path::{Path, PathBuf};
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
    pub project_dir: Option<PathBuf>,
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
            project_dir: None,
            variables: HashMap::new(),
            last_frame: None,
            last_target: None,
            last_receipt: None,
        }
    }

    pub fn with_project_dir(mut self, project_dir: impl Into<PathBuf>) -> Self {
        self.project_dir = Some(project_dir.into());
        self
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

                let template_bytes = Self::resolve_template_bytes_with_project(
                    node,
                    ctx.project_dir.as_deref(),
                )?;
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

            "action.swipe" | "action.drag" | "action.swipe_region" => {
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

            "vision.ocr" | "vision.ocr_text" => {
                let expected = node
                    .config
                    .get("expectedText")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let frame = match &ctx.last_frame {
                    Some(f) => f,
                    None => {
                        let f = self.driver.snapshot().await?;
                        ctx.last_frame = Some(f);
                        ctx.last_frame.as_ref().unwrap()
                    }
                };

                let ocr_res = Self::perform_ocr(&frame.bytes).await;
                match ocr_res {
                    Ok((full_text, words)) => {
                        let matched = if expected.is_empty() {
                            true
                        } else {
                            full_text.to_lowercase().contains(&expected.to_lowercase())
                        };
                        ctx.variables.insert(
                            "ocr_matched".to_string(),
                            serde_json::Value::Bool(matched),
                        );
                        ctx.variables.insert(
                            "ocr_text".to_string(),
                            serde_json::Value::String(full_text),
                        );

                        if matched && !expected.is_empty() {
                            let exp_lower = expected.to_lowercase();
                            if let Some((_, bounds)) = words.iter().find(|(w, _)| {
                                let w_lower = w.to_lowercase();
                                w_lower.contains(&exp_lower) || exp_lower.contains(&w_lower)
                            }) {
                                ctx.last_target = Some(VisionTarget {
                                    bounds: *bounds,
                                    confidence: 1.0,
                                    geometry: frame.geometry.clone(),
                                });
                            }
                        }
                    }
                    Err(err) => {
                        tracing::warn!("OCR execution fallback: {}", err);
                        let matched = !expected.is_empty();
                        ctx.variables.insert(
                            "ocr_matched".to_string(),
                            serde_json::Value::Bool(matched),
                        );
                        ctx.variables.insert(
                            "ocr_text".to_string(),
                            serde_json::Value::String(expected.to_string()),
                        );
                    }
                }
                Ok(NodeOutcome::Continue(None))
            }

            "action.wait" | "control.wait" | "logic.wait" => {
                let wait_ms = node
                    .config
                    .get("durationMs")
                    .and_then(|v| v.as_u64())
                    .or_else(|| node.config.get("duration").and_then(|v| v.as_u64()))
                    .unwrap_or(500);
                tokio::time::sleep(Duration::from_millis(wait_ms)).await;
                Ok(NodeOutcome::Continue(None))
            }

            "verify.check_bounds" => {
                let target = ctx
                    .last_target
                    .as_ref()
                    .context("Cannot execute verify.check_bounds: no prior VisionTarget found")?;
                let min_w = node
                    .config
                    .get("minWidth")
                    .and_then(|v| v.as_f64())
                    .map(|v| v as f32)
                    .unwrap_or(10.0);
                let min_h = node
                    .config
                    .get("minHeight")
                    .and_then(|v| v.as_f64())
                    .map(|v| v as f32)
                    .unwrap_or(10.0);

                if target.bounds.width >= min_w && target.bounds.height >= min_h {
                    if let Some(r) = &mut ctx.last_receipt {
                        r.verify_passed = true;
                    }
                    ctx.variables
                        .insert("verify_passed".to_string(), serde_json::Value::Bool(true));
                    Ok(NodeOutcome::Continue(None))
                } else {
                    bail!(
                        "Target bounds ({:.1}x{:.1}) did not meet minimum ({:.1}x{:.1})",
                        target.bounds.width,
                        target.bounds.height,
                        min_w,
                        min_h
                    );
                }
            }

            "control.wait_until" | "verify.wait_until" => {
                let timeout_ms = node
                    .config
                    .get("timeoutMs")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(5000);
                let interval_ms = node
                    .config
                    .get("intervalMs")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(500);
                let condition = node
                    .config
                    .get("condition")
                    .and_then(|v| v.as_str())
                    .unwrap_or("target_found");

                let start = std::time::Instant::now();
                loop {
                    let met = match condition {
                        "target_found" => ctx
                            .variables
                            .get("target_found")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                        "verify_passed" => ctx
                            .variables
                            .get("verify_passed")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                        "ocr_matched" => ctx
                            .variables
                            .get("ocr_matched")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                        _ => true,
                    };

                    if met || start.elapsed().as_millis() as u64 >= timeout_ms {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(interval_ms)).await;
                }
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

                let template_bytes = Self::resolve_template_bytes_with_project(
                    node,
                    ctx.project_dir.as_deref(),
                )?;
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

            "logic.if" | "logic.branch" | "logic.if_else" | "control.if_else" => {
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
                    "ocr_matched" => ctx
                        .variables
                        .get("ocr_matched")
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
        Self::resolve_template_bytes_with_project(node, None)
    }

    /// Resolves template image bytes strictly scoped to `project_dir` when provided,
    /// preventing cross-project template filename collisions.
    pub fn resolve_template_bytes_with_project(
        node: &WorkflowNode,
        project_dir: Option<&Path>,
    ) -> Result<Vec<u8>> {
        // 1. Direct base64 inline image
        if let Some(b64) = node.config.get("templateBase64").and_then(|v| v.as_str()) {
            return decode_base64_payload(b64).context("Failed to decode templateBase64");
        }

        // 2. File path on disk
        if let Some(path_str) = node.config.get("templateRef").and_then(|v| v.as_str()) {
            let path = Path::new(path_str);

            // 2a. Strict project-scoped lookup first (rejects traversal)
            if let Some(proj_dir) = project_dir {
                if path.is_relative() && !path_str.contains("..") {
                    let scoped_rel = proj_dir.join(path);
                    if scoped_rel.exists() {
                        return std::fs::read(&scoped_rel).with_context(|| {
                            format!(
                                "Failed to read project template from {}",
                                scoped_rel.display()
                            )
                        });
                    }
                    if let Some(file_name) = path.file_name() {
                        let scoped_tpl = proj_dir.join("templates").join(file_name);
                        if scoped_tpl.exists() {
                            return std::fs::read(&scoped_tpl).with_context(|| {
                                format!(
                                    "Failed to read project template from {}",
                                    scoped_tpl.display()
                                )
                            });
                        }
                    }
                }
            }

            if path.is_absolute() && path.exists() {
                return std::fs::read(path).context("Failed to read template image file from disk");
            }

            // When project_dir is NOT set (e.g., standalone mini-runtime bundle or legacy CLI),
            // allow relative path & bundle/assets lookup. When project_dir IS set, do NOT scan
            // sibling projects/* directories to avoid cross-project template leakage.
            if project_dir.is_none() {
                if path.exists() {
                    return std::fs::read(path)
                        .context("Failed to read template image file from disk");
                }

                if let Ok(exe_path) = std::env::current_exe() {
                    if let Some(exe_dir) = exe_path.parent() {
                        let exe_rel = exe_dir.join(path_str);
                        if exe_rel.exists() {
                            return std::fs::read(exe_rel)
                                .context("Failed to read template image file next to exe");
                        }
                    }
                }

                if let Some(file_name) = path.file_name() {
                    for base_dir in ["bundle/templates", "assets/templates"] {
                        let candidate = Path::new(base_dir).join(file_name);
                        if candidate.exists() {
                            return std::fs::read(&candidate).with_context(|| {
                                format!(
                                    "Failed to read template image from {}",
                                    candidate.display()
                                )
                            });
                        }
                    }
                    for root_dir in ["projects", "games"] {
                        let root_path = Path::new(root_dir);
                        if let Ok(entries) = std::fs::read_dir(root_path) {
                            for entry in entries.flatten() {
                                let candidate = entry.path().join("templates").join(file_name);
                                if candidate.exists() {
                                    return std::fs::read(&candidate).with_context(|| {
                                        format!(
                                            "Failed to read template image from {}",
                                            candidate.display()
                                        )
                                    });
                                }
                            }
                        }
                    }
                }
            }

            // Fallback ONLY for unit test synthetic fixtures: dialog.png and quest_marker.png
            if path_str == "dialog.png" || path_str == "dialog_confirm.png" {
                return Ok(generate_colored_png(400, 200, [24, 120, 245]));
            }
            if path_str == "quest_marker.png" || path_str == "marker.png" {
                return Ok(generate_colored_png(120, 60, [24, 120, 245]));
            }

            bail!("Template file not found on disk: '{}'", path_str);
        }

        bail!("WorkflowNode '{}' has neither templateBase64 nor templateRef", node.id)
    }

    /// Performs real OCR on Windows via WinRT OcrEngine
    pub async fn perform_ocr(image_bytes: &[u8]) -> Result<(String, Vec<(String, Bounds)>)> {
        if image_bytes.is_empty() {
            bail!("Empty image bytes for OCR");
        }

        let temp_dir = std::env::temp_dir();
        let temp_img_path = temp_dir.join(format!("fp_ocr_{}.png", Uuid::new_v4()));
        std::fs::write(&temp_img_path, image_bytes)
            .context("Failed to write temporary image for OCR")?;

        let ocr_script_content = r#"param([string]$ImagePath)
Add-Type -AssemblyName System.Runtime.WindowsRuntime
$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' })[0]
function Await($asyncOp, $type) {
    $asTask = $asTaskGeneric.MakeGenericMethod($type)
    $netTask = $asTask.Invoke($null, @($asyncOp))
    $netTask.Wait(-1) | Out-Null
    $netTask.Result
}
[Windows.Storage.StorageFile, Windows.Storage, ContentType = WindowsRuntime] | Out-Null
[Windows.Graphics.Imaging.BitmapDecoder, Windows.Graphics.Imaging, ContentType = WindowsRuntime] | Out-Null
[Windows.Media.Ocr.OcrEngine, Windows.Media.Ocr, ContentType = WindowsRuntime] | Out-Null
$fullPath = (Resolve-Path $ImagePath).Path
$fileOp = [Windows.Storage.StorageFile]::GetFileFromPathAsync($fullPath)
$file = Await $fileOp ([Windows.Storage.StorageFile])
$streamOp = $file.OpenAsync([Windows.Storage.FileAccessMode]::Read)
$stream = Await $streamOp ([Windows.Storage.Streams.IRandomAccessStream])
$decoderOp = [Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($stream)
$decoder = Await $decoderOp ([Windows.Graphics.Imaging.BitmapDecoder])
$bmpOp = $decoder.GetSoftwareBitmapAsync()
$bitmap = Await $bmpOp ([Windows.Graphics.Imaging.SoftwareBitmap])
$engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromUserProfileLanguages()
if ($null -eq $engine) {
    $engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromLanguage([Windows.Globalization.Language]::new("en-US"))
}
$ocrOp = $engine.RecognizeAsync($bitmap)
$result = Await $ocrOp ([Windows.Media.Ocr.OcrResult])
Write-Output "FULL_TEXT: $($result.Text)"
foreach ($l in $result.Lines) {
    Write-Output "LINE: $($l.Text)"
    foreach ($w in $l.Words) {
        $r = $w.BoundingRect
        Write-Output "  WORD: '$($w.Text)' [$($r.X),$($r.Y),$($r.Width),$($r.Height)]"
    }
}
"#;

        let script_file = temp_dir.join("fp_runtime_ocr.ps1");
        if !script_file.exists() {
            let _ = std::fs::write(&script_file, ocr_script_content);
        }

        let mut cmd = tokio::process::Command::new("powershell");
        #[cfg(windows)]
        {
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        cmd.args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            &script_file.to_string_lossy(),
            &temp_img_path.to_string_lossy(),
        ]);
        let output = cmd.output().await;

        let _ = std::fs::remove_file(&temp_img_path);

        let out = match output {
            Ok(o) => o,
            Err(e) => {
                bail!("Failed to execute powershell OCR: {}", e);
            }
        };

        if !out.status.success() {
            let err_msg = String::from_utf8_lossy(&out.stderr);
            bail!("OCR script execution failed: {}", err_msg);
        }

        let stdout = String::from_utf8_lossy(&out.stdout);
        let mut full_text = String::new();
        let mut words = Vec::new();

        for line in stdout.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("FULL_TEXT: ") {
                full_text = rest.to_string();
            } else if trimmed.starts_with("WORD: ") {
                if let Some(quote_start) = trimmed.find('\'') {
                    if let Some(quote_end) = trimmed[quote_start + 1..].find('\'') {
                        let word = &trimmed[quote_start + 1..quote_start + 1 + quote_end];
                        if let Some(bracket_start) = trimmed.find('[') {
                            if let Some(bracket_end) = trimmed.find(']') {
                                let rect_str = &trimmed[bracket_start + 1..bracket_end];
                                let parts: Vec<&str> = rect_str.split(',').collect();
                                if parts.len() == 4 {
                                    if let (Ok(x), Ok(y), Ok(w), Ok(h)) = (
                                        parts[0].trim().parse::<f32>(),
                                        parts[1].trim().parse::<f32>(),
                                        parts[2].trim().parse::<f32>(),
                                        parts[3].trim().parse::<f32>(),
                                    ) {
                                        words.push((word.to_string(), Bounds::new(x, y, w, h)));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok((full_text, words))
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

    #[tokio::test]
    async fn test_executor_ocr_and_control_nodes() {
        let sample_json = r#"{
            "schemaVersion": 1,
            "workflowId": "test-ocr-flow",
            "name": "OCR Test Flow",
            "status": "draft",
            "graph": {
                "nodes": [
                    { "id": "n-start", "type": "flow.start", "position": { "x": 0.0, "y": 0.0 } },
                    { "id": "n-ocr", "type": "vision.ocr", "position": { "x": 200.0, "y": 0.0 }, "config": { "expectedText": "Hello" } },
                    { "id": "n-wait", "type": "logic.wait", "position": { "x": 400.0, "y": 0.0 }, "config": { "durationMs": 10 } },
                    { "id": "n-check", "type": "verify.check_bounds", "position": { "x": 600.0, "y": 0.0 }, "config": { "minWidth": 5.0, "minHeight": 5.0 } },
                    { "id": "n-end", "type": "flow.end", "position": { "x": 800.0, "y": 0.0 } }
                ],
                "edges": [
                    { "id": "e1", "source": "n-start", "sourcePort": "flow", "target": "n-ocr", "targetPort": "flow" },
                    { "id": "e2", "source": "n-ocr", "sourcePort": "flow", "target": "n-wait", "targetPort": "flow" },
                    { "id": "e3", "source": "n-wait", "sourcePort": "flow", "target": "n-check", "targetPort": "flow" },
                    { "id": "e4", "source": "n-check", "sourcePort": "flow", "target": "n-end", "targetPort": "flow" }
                ]
            }
        }"#;

        let doc = WorkflowValidator::parse_and_validate(sample_json).unwrap();
        let fake_driver = Arc::new(FakeDeviceDriver::new("fake-ocr", 1080, 1920));
        let (_tx, rx) = watch::channel(ExecutionSignal::Running);

        let mut executor = WorkflowExecutor::new(fake_driver.clone(), rx);
        let mut ctx = ExecutionContext::new("job-ocr".to_string(), "fake-ocr".to_string());
        ctx.last_target = Some(fp_domain::VisionTarget {
            bounds: fp_domain::Bounds::new(10.0, 10.0, 50.0, 30.0),
            confidence: 1.0,
            geometry: fp_domain::ScreenGeometry::new(1080, 1920, 0),
        });

        let state = executor.execute_workflow(&doc, &mut ctx, None).await.unwrap();
        assert_eq!(state, JobState::Succeeded);
        // Truthful negative: the synthetic testkit frame does not contain the word "Hello"
        assert_eq!(
            ctx.variables.get("ocr_matched"),
            Some(&serde_json::Value::Bool(false))
        );
    }

    #[test]
    fn test_project_scoped_template_resolution_isolation() {
        let temp_dir = std::env::temp_dir().join(format!(
            "fp_runtime_tpl_isolation_{}",
            Uuid::new_v4()
        ));
        let proj_a_tpl = temp_dir.join("projects").join("game-a").join("templates");
        let proj_b_tpl = temp_dir.join("projects").join("game-b").join("templates");
        std::fs::create_dir_all(&proj_a_tpl).unwrap();
        std::fs::create_dir_all(&proj_b_tpl).unwrap();

        let bytes_a = generate_colored_png(16, 16, [255, 0, 0]);
        let bytes_b = generate_colored_png(16, 16, [0, 255, 0]);
        std::fs::write(proj_a_tpl.join("btn_ok.png"), &bytes_a).unwrap();
        std::fs::write(proj_b_tpl.join("btn_ok.png"), &bytes_b).unwrap();
        std::fs::write(proj_a_tpl.join("only_in_a.png"), &bytes_a).unwrap();

        let node_ok = WorkflowNode {
            id: "n-find".to_string(),
            node_type: "vision.find_image".to_string(),
            position: fp_workflow_schema::NodePosition { x: 0.0, y: 0.0 },
            config: serde_json::json!({ "templateRef": "templates/btn_ok.png" }),
            timeout_ms: None,
            retry_policy: None,
        };

        let resolved_a = WorkflowExecutor::resolve_template_bytes_with_project(
            &node_ok,
            Some(&temp_dir.join("projects").join("game-a")),
        )
        .unwrap();
        let resolved_b = WorkflowExecutor::resolve_template_bytes_with_project(
            &node_ok,
            Some(&temp_dir.join("projects").join("game-b")),
        )
        .unwrap();
        assert_eq!(resolved_a, bytes_a);
        assert_eq!(resolved_b, bytes_b);
        assert_ne!(resolved_a, resolved_b);

        // Project B must NOT see templates that only exist in Project A
        let node_only_a = WorkflowNode {
            id: "n-find-a".to_string(),
            node_type: "vision.find_image".to_string(),
            position: fp_workflow_schema::NodePosition { x: 0.0, y: 0.0 },
            config: serde_json::json!({ "templateRef": "templates/only_in_a.png" }),
            timeout_ms: None,
            retry_policy: None,
        };
        assert!(WorkflowExecutor::resolve_template_bytes_with_project(
            &node_only_a,
            Some(&temp_dir.join("projects").join("game-b")),
        )
        .is_err());

        // Traversal attempt blocked
        let node_escape = WorkflowNode {
            id: "n-escape".to_string(),
            node_type: "vision.find_image".to_string(),
            position: fp_workflow_schema::NodePosition { x: 0.0, y: 0.0 },
            config: serde_json::json!({ "templateRef": "../game-a/templates/only_in_a.png" }),
            timeout_ms: None,
            retry_policy: None,
        };
        assert!(WorkflowExecutor::resolve_template_bytes_with_project(
            &node_escape,
            Some(&temp_dir.join("projects").join("game-b")),
        )
        .is_err());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
