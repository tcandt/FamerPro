use chrono::Utc;
use fp_domain::{Bounds, DeviceDriver, Point, SwipePath};
use fp_workflow_schema::{
    NodePosition, WorkflowDocument, WorkflowEdge, WorkflowGraph, WorkflowNode,
};
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::{
    generate_selectors_for_node, ActionExecutionMode, ElementActionKind, ElementSnapshot, Selector,
    UnifiedElementExecutor,
};

/// Represents a single recorded step in a workflow recording session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecordedWorkflowStep {
    pub id: String,
    pub step_number: usize,
    pub action_type: String, // "tap", "swipe", "input_text", "key_event", "wait", "assert_exists"
    pub label: String,
    pub selector: Option<Selector>,
    pub resolved_bounds: Option<Bounds>,
    pub point: Option<Point>,
    pub swipe_path: Option<SwipePath>,
    pub key_code: Option<u32>,
    pub input_text: Option<String>,
    pub wait_ms: Option<u64>,
    pub assertion_rule: Option<String>,
    pub before_screenshot: Option<String>,
    pub after_screenshot: Option<String>,
    pub duration_ms: u64,
    pub status: String, // "recorded", "replayed", "verified", "failed"
    pub enabled: bool,
    pub uniqueness_score: u32,
    pub strategy_used: String,
    pub notes: Option<String>,
}

/// Execution result for an individual step during replay.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StepExecutionResult {
    pub step_id: String,
    pub success: bool,
    pub duration_ms: u64,
    pub error_message: Option<String>,
    pub verified: bool,
    pub target_point: Option<Point>,
}

/// Summary report after replaying a sequence of recorded steps.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowReplayReport {
    pub total_steps: usize,
    pub executed_steps: usize,
    pub passed_steps: usize,
    pub failed_steps: usize,
    pub total_duration_ms: u64,
    pub results: Vec<StepExecutionResult>,
}

/// Automatically creates a RecordedWorkflowStep from a physical screen touch coordinate.
/// Follows the strict selector priority:
/// 1. Resource ID (if unique, score 100)
/// 2. Exact Text (if unique, score 100)
/// 3. Strict XPath
/// 4. Fallback: Normalized Relative Point
pub fn build_step_from_touch(
    snapshot: &ElementSnapshot,
    x: f32,
    y: f32,
    step_number: usize,
    action_type: &str,
    text_input: Option<String>,
) -> RecordedWorkflowStep {
    let step_id = format!("step-{}", uuid_simple(step_number));

    // 1. Locate innermost node under the touch coordinates
    if let Some(innermost_node) = snapshot.find_node_at_point(x, y) {
        let mut resolved_bounds = innermost_node.bounds;

        // If the node itself is not clickable, look up clickable ancestor for bounds
        if !innermost_node.clickable {
            let mut curr = innermost_node;
            while let Some(parent_id) = curr.parent_id {
                if let Some(parent) = snapshot.nodes.iter().find(|n| n.id == parent_id) {
                    if parent.clickable {
                        resolved_bounds = parent.bounds;
                        break;
                    }
                    curr = parent;
                } else {
                    break;
                }
            }
        }

        // Generate selectors for the innermost targeted node (where text / id live)
        let candidates = generate_selectors_for_node(snapshot, innermost_node.id);

        // Pick the best candidate (uniqueness score == 100 preferred)
        let best_candidate = candidates
            .iter()
            .find(|c| c.uniqueness_score == 100)
            .or_else(|| candidates.first());

        if let Some(cand) = best_candidate {
            let label = if !innermost_node.text.is_empty() {
                format!("Click \"{}\" [{}]", innermost_node.text, cand.strategy_name)
            } else if !innermost_node.resource_id.is_empty() {
                let id_short = innermost_node
                    .resource_id
                    .split('/')
                    .last()
                    .unwrap_or(&innermost_node.resource_id);
                format!("Click #{} [{}]", id_short, cand.strategy_name)
            } else {
                format!(
                    "Click {} [{}]",
                    innermost_node
                        .class_name
                        .split('.')
                        .last()
                        .unwrap_or("Element"),
                    cand.strategy_name
                )
            };

            return RecordedWorkflowStep {
                id: step_id,
                step_number,
                action_type: action_type.to_string(),
                label,
                selector: Some(Selector::Native(cand.selector.clone())),
                resolved_bounds: Some(resolved_bounds),
                point: Some(Point { x, y }),
                swipe_path: None,
                key_code: None,
                input_text: text_input,
                wait_ms: Some(500),
                assertion_rule: None,
                before_screenshot: None,
                after_screenshot: None,
                duration_ms: 0,
                status: "recorded".to_string(),
                enabled: true,
                uniqueness_score: cand.uniqueness_score,
                strategy_used: cand.strategy_name.clone(),
                notes: Some(format!(
                    "Class: {}, Bounds: [{},{},{},{}]",
                    innermost_node.class_name,
                    resolved_bounds.x,
                    resolved_bounds.y,
                    resolved_bounds.width,
                    resolved_bounds.height
                )),
            };
        }
    }

    // Fallback: Normalized Relative Point selector
    let norm_x = if snapshot.screen_width > 0 {
        x / snapshot.screen_width as f32
    } else {
        0.0
    };
    let norm_y = if snapshot.screen_height > 0 {
        y / snapshot.screen_height as f32
    } else {
        0.0
    };

    RecordedWorkflowStep {
        id: step_id,
        step_number,
        action_type: action_type.to_string(),
        label: format!("Tap ({:.0}, {:.0}) [Relative Point]", x, y),
        selector: Some(Selector::Point(crate::PointSelector {
            x: norm_x,
            y: norm_y,
            is_relative: true,
        })),
        resolved_bounds: Some(Bounds::new(x - 10.0, y - 10.0, 20.0, 20.0)),
        point: Some(Point { x, y }),
        swipe_path: None,
        key_code: None,
        input_text: text_input,
        wait_ms: Some(500),
        assertion_rule: None,
        before_screenshot: None,
        after_screenshot: None,
        duration_ms: 0,
        status: "recorded".to_string(),
        enabled: true,
        uniqueness_score: 50,
        strategy_used: "Relative Point".to_string(),
        notes: Some(format!("Coordinates: ({:.1}, {:.1})", x, y)),
    }
}

/// Converts a recorded list of steps into a valid FarmerPro `WorkflowDocument`
/// compatible with `fp-workflow-schema`, Workflow Studio, and SQLite storage.
pub fn recorded_steps_to_workflow_document(
    workflow_id: &str,
    name: &str,
    steps: &[RecordedWorkflowStep],
) -> WorkflowDocument {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    // 1. Start node
    let start_node_id = "node-start".to_string();
    nodes.push(WorkflowNode {
        id: start_node_id.clone(),
        node_type: "flow.start".to_string(),
        position: NodePosition { x: 100.0, y: 150.0 },
        config: serde_json::json!({
            "label": "Start Recording Session"
        }),
        timeout_ms: Some(5000),
        retry_policy: None,
    });

    let mut prev_node_id = start_node_id;
    let mut current_x = 350.0;
    let mut edge_counter = 1;

    for (idx, step) in steps.iter().filter(|s| s.enabled).enumerate() {
        let node_id = format!("node-action-{}", idx + 1);

        let (node_type, config) = match step.action_type.as_str() {
            "tap" => (
                "action.elementTap".to_string(),
                serde_json::json!({
                    "label": step.label,
                    "selector": step.selector,
                    "targetBounds": step.resolved_bounds,
                    "strategy": step.strategy_used,
                    "uniquenessScore": step.uniqueness_score,
                    "fallbackPoint": step.point,
                    "waitAfterMs": step.wait_ms.unwrap_or(500),
                }),
            ),
            "swipe" => (
                "action.swipe".to_string(),
                serde_json::json!({
                    "label": step.label,
                    "path": step.swipe_path,
                    "waitAfterMs": step.wait_ms.unwrap_or(500),
                }),
            ),
            "input_text" => (
                "action.inputText".to_string(),
                serde_json::json!({
                    "label": step.label,
                    "selector": step.selector,
                    "text": step.input_text.as_deref().unwrap_or(""),
                    "waitAfterMs": step.wait_ms.unwrap_or(500),
                }),
            ),
            "key_event" => (
                "action.keyEvent".to_string(),
                serde_json::json!({
                    "label": step.label,
                    "keyCode": step.key_code,
                    "waitAfterMs": step.wait_ms.unwrap_or(500),
                }),
            ),
            "wait" => (
                "flow.delay".to_string(),
                serde_json::json!({
                    "label": step.label,
                    "delayMs": step.wait_ms.unwrap_or(1000),
                }),
            ),
            _ => (
                "action.elementTap".to_string(),
                serde_json::json!({
                    "label": step.label,
                    "selector": step.selector,
                    "point": step.point,
                }),
            ),
        };

        nodes.push(WorkflowNode {
            id: node_id.clone(),
            node_type,
            position: NodePosition {
                x: current_x,
                y: 150.0 + (idx % 2) as f64 * 30.0,
            },
            config,
            timeout_ms: Some(10000),
            retry_policy: None,
        });

        edges.push(WorkflowEdge {
            id: format!("edge-{}", edge_counter),
            source: prev_node_id,
            source_port: "out".to_string(),
            target: node_id.clone(),
            target_port: "in".to_string(),
        });
        edge_counter += 1;
        prev_node_id = node_id;
        current_x += 250.0;
    }

    // End node
    let end_node_id = "node-end".to_string();
    nodes.push(WorkflowNode {
        id: end_node_id.clone(),
        node_type: "flow.end".to_string(),
        position: NodePosition {
            x: current_x,
            y: 150.0,
        },
        config: serde_json::json!({
            "label": "Workflow Complete"
        }),
        timeout_ms: None,
        retry_policy: None,
    });

    edges.push(WorkflowEdge {
        id: format!("edge-{}", edge_counter),
        source: prev_node_id,
        source_port: "out".to_string(),
        target: end_node_id,
        target_port: "in".to_string(),
    });

    WorkflowDocument {
        schema_version: 1,
        workflow_id: workflow_id.to_string(),
        name: name.to_string(),
        status: "recorded".to_string(),
        graph: WorkflowGraph { nodes, edges },
        runtime_policy: None,
        disclaimer: Some("Recorded via FarmerPro Smart Workflow Recorder".to_string()),
    }
}

/// Executes a single recorded step live on a real physical device.
pub async fn execute_recorded_step<D: DeviceDriver>(
    driver: &D,
    step: &RecordedWorkflowStep,
    snapshot: Option<&ElementSnapshot>,
) -> StepExecutionResult {
    let start = Instant::now();

    match step.action_type.as_str() {
        "tap" => {
            // If selector exists, attempt detection first
            if let Some(sel) = &step.selector {
                match UnifiedElementExecutor::detect(snapshot, sel, None, false) {
                    Ok(detection) => {
                        let target_match = match detection.matches.first() {
                            Some(m) => m,
                            None => {
                                return StepExecutionResult {
                                    step_id: step.id.clone(),
                                    success: false,
                                    duration_ms: start.elapsed().as_millis() as u64,
                                    error_message: Some("No match found in detection".to_string()),
                                    verified: false,
                                    target_point: None,
                                };
                            }
                        };
                        let plan = match UnifiedElementExecutor::plan(
                            target_match,
                            ElementActionKind::Tap,
                            ActionExecutionMode::DynamicBoundsTap,
                            None,
                        ) {
                            Ok(p) => p,
                            Err(e) => {
                                return StepExecutionResult {
                                    step_id: step.id.clone(),
                                    success: false,
                                    duration_ms: start.elapsed().as_millis() as u64,
                                    error_message: Some(format!("Planning error: {}", e)),
                                    verified: false,
                                    target_point: None,
                                };
                            }
                        };

                        match UnifiedElementExecutor::execute(driver, &plan).await {
                            Ok(receipt) => StepExecutionResult {
                                step_id: step.id.clone(),
                                success: receipt.success,
                                duration_ms: start.elapsed().as_millis() as u64,
                                error_message: receipt.error,
                                verified: receipt.success,
                                target_point: receipt.point_tapped,
                            },
                            Err(e) => StepExecutionResult {
                                step_id: step.id.clone(),
                                success: false,
                                duration_ms: start.elapsed().as_millis() as u64,
                                error_message: Some(format!("Execution failed: {}", e)),
                                verified: false,
                                target_point: None,
                            },
                        }
                    }
                    Err(e) => {
                        // Detection failed, fallback to recorded coordinates if available
                        if let Some(pt) = &step.point {
                            match driver.tap(pt.x as u32, pt.y as u32).await {
                                Ok(receipt) => StepExecutionResult {
                                    step_id: step.id.clone(),
                                    success: receipt.success,
                                    duration_ms: start.elapsed().as_millis() as u64,
                                    error_message: None,
                                    verified: true,
                                    target_point: Some(pt.clone()),
                                },
                                Err(err) => StepExecutionResult {
                                    step_id: step.id.clone(),
                                    success: false,
                                    duration_ms: start.elapsed().as_millis() as u64,
                                    error_message: Some(format!(
                                        "Detection error '{}' & fallback tap failed: {}",
                                        e, err
                                    )),
                                    verified: false,
                                    target_point: None,
                                },
                            }
                        } else {
                            StepExecutionResult {
                                step_id: step.id.clone(),
                                success: false,
                                duration_ms: start.elapsed().as_millis() as u64,
                                error_message: Some(format!("Detection failed: {}", e)),
                                verified: false,
                                target_point: None,
                            }
                        }
                    }
                }
            } else if let Some(pt) = &step.point {
                // Direct point tap
                match driver.tap(pt.x as u32, pt.y as u32).await {
                    Ok(receipt) => StepExecutionResult {
                        step_id: step.id.clone(),
                        success: receipt.success,
                        duration_ms: start.elapsed().as_millis() as u64,
                        error_message: None,
                        verified: true,
                        target_point: Some(pt.clone()),
                    },
                    Err(e) => StepExecutionResult {
                        step_id: step.id.clone(),
                        success: false,
                        duration_ms: start.elapsed().as_millis() as u64,
                        error_message: Some(e.to_string()),
                        verified: false,
                        target_point: None,
                    },
                }
            } else {
                StepExecutionResult {
                    step_id: step.id.clone(),
                    success: false,
                    duration_ms: start.elapsed().as_millis() as u64,
                    error_message: Some("No selector or point provided for tap".to_string()),
                    verified: false,
                    target_point: None,
                }
            }
        }
        "swipe" => {
            if let Some(path) = &step.swipe_path {
                match driver.swipe(path.clone()).await {
                    Ok(receipt) => StepExecutionResult {
                        step_id: step.id.clone(),
                        success: receipt.success,
                        duration_ms: start.elapsed().as_millis() as u64,
                        error_message: None,
                        verified: true,
                        target_point: None,
                    },
                    Err(e) => StepExecutionResult {
                        step_id: step.id.clone(),
                        success: false,
                        duration_ms: start.elapsed().as_millis() as u64,
                        error_message: Some(e.to_string()),
                        verified: false,
                        target_point: None,
                    },
                }
            } else {
                StepExecutionResult {
                    step_id: step.id.clone(),
                    success: false,
                    duration_ms: start.elapsed().as_millis() as u64,
                    error_message: Some("Swipe path missing".to_string()),
                    verified: false,
                    target_point: None,
                }
            }
        }
        "key_event" => {
            if let Some(code) = step.key_code {
                match driver.key_event(code).await {
                    Ok(receipt) => StepExecutionResult {
                        step_id: step.id.clone(),
                        success: receipt.success,
                        duration_ms: start.elapsed().as_millis() as u64,
                        error_message: None,
                        verified: true,
                        target_point: None,
                    },
                    Err(e) => StepExecutionResult {
                        step_id: step.id.clone(),
                        success: false,
                        duration_ms: start.elapsed().as_millis() as u64,
                        error_message: Some(e.to_string()),
                        verified: false,
                        target_point: None,
                    },
                }
            } else {
                StepExecutionResult {
                    step_id: step.id.clone(),
                    success: false,
                    duration_ms: start.elapsed().as_millis() as u64,
                    error_message: Some("Key code missing".to_string()),
                    verified: false,
                    target_point: None,
                }
            }
        }
        "input_text" => {
            if let Some(text) = &step.input_text {
                match driver.input_text(text).await {
                    Ok(receipt) => StepExecutionResult {
                        step_id: step.id.clone(),
                        success: receipt.success,
                        duration_ms: start.elapsed().as_millis() as u64,
                        error_message: None,
                        verified: true,
                        target_point: None,
                    },
                    Err(e) => StepExecutionResult {
                        step_id: step.id.clone(),
                        success: false,
                        duration_ms: start.elapsed().as_millis() as u64,
                        error_message: Some(e.to_string()),
                        verified: false,
                        target_point: None,
                    },
                }
            } else {
                StepExecutionResult {
                    step_id: step.id.clone(),
                    success: false,
                    duration_ms: start.elapsed().as_millis() as u64,
                    error_message: Some("Input text missing".to_string()),
                    verified: false,
                    target_point: None,
                }
            }
        }
        "wait" => {
            let wait_dur = step.wait_ms.unwrap_or(500);
            tokio::time::sleep(std::time::Duration::from_millis(wait_dur)).await;
            StepExecutionResult {
                step_id: step.id.clone(),
                success: true,
                duration_ms: wait_dur,
                error_message: None,
                verified: true,
                target_point: None,
            }
        }
        other => StepExecutionResult {
            step_id: step.id.clone(),
            success: false,
            duration_ms: start.elapsed().as_millis() as u64,
            error_message: Some(format!("Unknown action type: {}", other)),
            verified: false,
            target_point: None,
        },
    }
}

fn uuid_simple(n: usize) -> String {
    format!(
        "{:04x}",
        (Utc::now().timestamp_subsec_millis() as usize + n * 31) % 0xFFFF
    )
}
