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
    pub on_failure: Option<String>,
}

/// Execution result for an individual step during replay.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StepExecutionResult {
    pub step_id: String,
    pub success: bool,
    pub duration_ms: u64,
    pub error_message: Option<String>,
    pub status: String, // "DETECTED", "DISPATCHED", "ACTION_COMPLETED", "VERIFIED", "FAILED", "UNVERIFIED"
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

    // 1. Locate innermost leaf node under touch using structured hit-testing
    if let Some(target) = snapshot.hit_test_element(x, y) {
        let innermost_node = target.target_node;
        let action_target = target.action_target;
        let resolved_bounds = target.resolved_bounds;

        // Generate selectors for the targeted node (where text / id live)
        let mut candidates = generate_selectors_for_node(snapshot, innermost_node.id);

        // If target node itself is not unique or not clickable, also include action_target candidates
        if action_target.id != innermost_node.id {
            let parent_candidates = generate_selectors_for_node(snapshot, action_target.id);
            candidates.extend(parent_candidates);
        }

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
                on_failure: Some("stop".to_string()),
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
        on_failure: Some("stop".to_string()),
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
            "tap" => {
                let (res_id, text, xpath, class_name) = match &step.selector {
                    Some(Selector::Native(n)) => (
                        n.resource_id.clone(),
                        n.text.clone(),
                        n.xpath.clone(),
                        n.class_name.clone(),
                    ),
                    _ => (None, None, None, None),
                };
                (
                    "element.click".to_string(),
                    serde_json::json!({
                        "label": step.label,
                        "selector": step.selector,
                        "resourceId": res_id,
                        "text": text,
                        "xpath": xpath,
                        "className": class_name,
                        "targetBounds": step.resolved_bounds,
                        "strategy": step.strategy_used,
                        "uniquenessScore": step.uniqueness_score,
                        "requireUnique": true,
                        "pointX": step.point.as_ref().map(|p| p.x),
                        "pointY": step.point.as_ref().map(|p| p.y),
                        "waitAfterMs": step.wait_ms.unwrap_or(500),
                    }),
                )
            }
            "swipe" => (
                "action.swipe".to_string(),
                serde_json::json!({
                    "label": step.label,
                    "path": step.swipe_path,
                    "startX": step.swipe_path.as_ref().map(|p| p.start_x),
                    "startY": step.swipe_path.as_ref().map(|p| p.start_y),
                    "endX": step.swipe_path.as_ref().map(|p| p.end_x),
                    "endY": step.swipe_path.as_ref().map(|p| p.end_y),
                    "durationMs": step.swipe_path.as_ref().map(|p| p.duration_ms).unwrap_or(300),
                    "waitAfterMs": step.wait_ms.unwrap_or(500),
                }),
            ),
            "input_text" => {
                let (res_id, xpath) = match &step.selector {
                    Some(Selector::Native(n)) => (n.resource_id.clone(), n.xpath.clone()),
                    _ => (None, None),
                };
                (
                    "element.set_text".to_string(),
                    serde_json::json!({
                        "label": step.label,
                        "selector": step.selector,
                        "resourceId": res_id,
                        "xpath": xpath,
                        "inputText": step.input_text.as_deref().unwrap_or(""),
                        "requireUnique": true,
                        "waitAfterMs": step.wait_ms.unwrap_or(500),
                    }),
                )
            }
            "assert" | "assert_exists" => {
                let (res_id, text, xpath) = match &step.selector {
                    Some(Selector::Native(n)) => (n.resource_id.clone(), n.text.clone(), n.xpath.clone()),
                    _ => (None, None, None),
                };
                (
                    "element.assert".to_string(),
                    serde_json::json!({
                        "label": step.label,
                        "selector": step.selector,
                        "resourceId": res_id,
                        "text": text,
                        "xpath": xpath,
                        "assertionRule": step.assertion_rule,
                        "waitAfterMs": step.wait_ms.unwrap_or(500),
                    }),
                )
            }
            "key_event" => (
                "action.key".to_string(),
                serde_json::json!({
                    "label": step.label,
                    "keyCode": step.key_code,
                    "keycode": step.key_code,
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
                "element.click".to_string(),
                serde_json::json!({
                    "label": step.label,
                    "selector": step.selector,
                    "pointX": step.point.as_ref().map(|p| p.x),
                    "pointY": step.point.as_ref().map(|p| p.y),
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
            // If selector exists, attempt detection first with require_unique = true (FAIL-CLOSED)
            if let Some(sel) = &step.selector {
                match UnifiedElementExecutor::detect(snapshot, sel, None, true) {
                    Ok(detection) => {
                        let target_match = match detection.matches.first() {
                            Some(m) => m,
                            None => {
                                return StepExecutionResult {
                                    step_id: step.id.clone(),
                                    success: false,
                                    duration_ms: start.elapsed().as_millis() as u64,
                                    error_message: Some(format!(
                                        "ELEMENT_NOT_FOUND: Selector '{:?}' not found in snapshot",
                                        sel
                                    )),
                                    status: "FAILED".to_string(),
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
                                    error_message: Some(format!("PLANNING_FAILED: {}", e)),
                                    status: "FAILED".to_string(),
                                    verified: false,
                                    target_point: None,
                                };
                            }
                        };

                        match UnifiedElementExecutor::execute(driver, &plan).await {
                            Ok(receipt) => {
                                let (status, verified) = if receipt.success {
                                    if receipt.verify_passed {
                                        ("VERIFIED".to_string(), true)
                                    } else {
                                        ("ACTION_COMPLETED".to_string(), false)
                                    }
                                } else {
                                    ("FAILED".to_string(), false)
                                };
                                StepExecutionResult {
                                    step_id: step.id.clone(),
                                    success: receipt.success,
                                    duration_ms: start.elapsed().as_millis() as u64,
                                    error_message: receipt.error,
                                    status,
                                    verified,
                                    target_point: receipt.point_tapped,
                                }
                            }
                            Err(e) => StepExecutionResult {
                                step_id: step.id.clone(),
                                success: false,
                                duration_ms: start.elapsed().as_millis() as u64,
                                error_message: Some(format!("EXECUTION_FAILED: {}", e)),
                                status: "FAILED".to_string(),
                                verified: false,
                                target_point: None,
                            },
                        }
                    }
                    Err(e) => {
                        // FAIL-CLOSED: Stop immediately on ambiguous or missing selector!
                        let err_msg = match &e {
                            crate::ElementError::AmbiguousMatch { selector, count } => {
                                format!(
                                    "AMBIGUOUS_MATCH: Selector '{}' matched {} elements. Replay aborted for safety.",
                                    selector, count
                                )
                            }
                            crate::ElementError::NotFound(s) => {
                                format!("ELEMENT_NOT_FOUND: Selector '{}' was not found in snapshot.", s)
                            }
                            other => format!("DETECTION_FAILED: {}", other),
                        };
                        StepExecutionResult {
                            step_id: step.id.clone(),
                            success: false,
                            duration_ms: start.elapsed().as_millis() as u64,
                            error_message: Some(err_msg),
                            status: "FAILED".to_string(),
                            verified: false,
                            target_point: None,
                        }
                    }
                }
            } else if let Some(pt) = &step.point {
                // Direct point tap (only when explicitly recorded without a selector)
                match driver.tap(pt.x as u32, pt.y as u32).await {
                    Ok(receipt) => StepExecutionResult {
                        step_id: step.id.clone(),
                        success: receipt.success,
                        duration_ms: start.elapsed().as_millis() as u64,
                        error_message: receipt.error,
                        status: if receipt.success {
                            "ACTION_COMPLETED".to_string()
                        } else {
                            "FAILED".to_string()
                        },
                        verified: false, // Point taps without assertions are unverified
                        target_point: Some(pt.clone()),
                    },
                    Err(e) => StepExecutionResult {
                        step_id: step.id.clone(),
                        success: false,
                        duration_ms: start.elapsed().as_millis() as u64,
                        error_message: Some(format!("TAP_FAILED: {}", e)),
                        status: "FAILED".to_string(),
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
                    status: "FAILED".to_string(),
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
                        error_message: receipt.error,
                        status: if receipt.success {
                            "ACTION_COMPLETED".to_string()
                        } else {
                            "FAILED".to_string()
                        },
                        verified: false,
                        target_point: None,
                    },
                    Err(e) => StepExecutionResult {
                        step_id: step.id.clone(),
                        success: false,
                        duration_ms: start.elapsed().as_millis() as u64,
                        error_message: Some(e.to_string()),
                        status: "FAILED".to_string(),
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
                    status: "FAILED".to_string(),
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
                        error_message: receipt.error,
                        status: if receipt.success {
                            "ACTION_COMPLETED".to_string()
                        } else {
                            "FAILED".to_string()
                        },
                        verified: false,
                        target_point: None,
                    },
                    Err(e) => StepExecutionResult {
                        step_id: step.id.clone(),
                        success: false,
                        duration_ms: start.elapsed().as_millis() as u64,
                        error_message: Some(e.to_string()),
                        status: "FAILED".to_string(),
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
                    status: "FAILED".to_string(),
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
                        error_message: receipt.error,
                        status: if receipt.success {
                            "ACTION_COMPLETED".to_string()
                        } else {
                            "FAILED".to_string()
                        },
                        verified: false,
                        target_point: None,
                    },
                    Err(e) => StepExecutionResult {
                        step_id: step.id.clone(),
                        success: false,
                        duration_ms: start.elapsed().as_millis() as u64,
                        error_message: Some(e.to_string()),
                        status: "FAILED".to_string(),
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
                    status: "FAILED".to_string(),
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
                status: "ACTION_COMPLETED".to_string(),
                verified: true,
                target_point: None,
            }
        }
        "assert_exists" | "assert" => {
            if let Some(sel) = &step.selector {
                match UnifiedElementExecutor::detect(snapshot, sel, None, false) {
                    Ok(detection) if detection.candidate_count > 0 => StepExecutionResult {
                        step_id: step.id.clone(),
                        success: true,
                        duration_ms: start.elapsed().as_millis() as u64,
                        error_message: None,
                        status: "VERIFIED".to_string(),
                        verified: true,
                        target_point: None,
                    },
                    _ => StepExecutionResult {
                        step_id: step.id.clone(),
                        success: false,
                        duration_ms: start.elapsed().as_millis() as u64,
                        error_message: Some(format!(
                            "ASSERTION_FAILED: Selector '{:?}' not found in snapshot",
                            sel
                        )),
                        status: "FAILED".to_string(),
                        verified: false,
                        target_point: None,
                    },
                }
            } else {
                StepExecutionResult {
                    step_id: step.id.clone(),
                    success: false,
                    duration_ms: start.elapsed().as_millis() as u64,
                    error_message: Some("Selector missing for assertion step".to_string()),
                    status: "FAILED".to_string(),
                    verified: false,
                    target_point: None,
                }
            }
        }
        other => StepExecutionResult {
            step_id: step.id.clone(),
            success: false,
            duration_ms: start.elapsed().as_millis() as u64,
            error_message: Some(format!("Unknown action type: {}", other)),
            status: "FAILED".to_string(),
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
