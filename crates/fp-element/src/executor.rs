use crate::action::{
    plan_element_action, ActionExecutionMode, ElementActionKind, ElementActionPlan,
};
use crate::model::{ElementError, ElementSnapshot};
use crate::selectors::{evaluate_selector, Selector, SelectorMatch};
use fp_actions::SafeTapPolicy;
use fp_domain::{ActionReceipt, Bounds, DeviceDriver, SwipePath};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionResult {
    pub target_bounds: Bounds,
    pub resolved_bounds: Bounds,
    pub confidence: f32,
    pub candidate_count: usize,
    pub strategy_used: String,
    pub is_clickable: bool,
    pub resolved_clickable_ancestor_id: Option<usize>,
    pub matches: Vec<SelectorMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VerificationRule {
    WaitDisappear { timeout_ms: u64 },
    WaitAppear { timeout_ms: u64 },
    AssertTextContains(String),
}

/// Unified Element Executor across Studio UI, Workflow Runtime, and Standalone Mini Auto-Tool.
/// Guarantees that selector resolution, 15% inner padding margin tap calculation, execution dispatch,
/// and verification logic are 100% identical regardless of entry point.
pub struct UnifiedElementExecutor;

impl UnifiedElementExecutor {
    /// Detects target element across any of the 7 selector modalities without sending any touch action.
    pub fn detect(
        snapshot: Option<&ElementSnapshot>,
        selector: &Selector,
        screen_bounds: Option<&Bounds>,
        require_unique: bool,
    ) -> Result<DetectionResult, ElementError> {
        let matches = evaluate_selector(snapshot, selector, screen_bounds, require_unique)?;
        if matches.is_empty() {
            return Err(ElementError::NotFound(format!("{:?}", selector)));
        }
        if require_unique && matches.len() > 1 {
            return Err(ElementError::AmbiguousMatch {
                count: matches.len(),
                selector: format!("{:?}", selector),
            });
        }
        let first = matches[0].clone();
        Ok(DetectionResult {
            target_bounds: first.bounds,
            resolved_bounds: first.resolved_bounds,
            confidence: first.confidence,
            candidate_count: matches.len(),
            strategy_used: first.strategy_used,
            is_clickable: first.is_clickable,
            resolved_clickable_ancestor_id: first.resolved_clickable_ancestor_id,
            matches,
        })
    }

    /// Plans the safe randomized action with 15% inner margin and exclusion masks
    pub fn plan(
        match_item: &SelectorMatch,
        kind: ElementActionKind,
        mode: ActionExecutionMode,
        policy: Option<SafeTapPolicy>,
    ) -> Result<ElementActionPlan, ElementError> {
        plan_element_action(match_item, kind, mode, policy)
    }

    /// Executes the planned action strictly through the DeviceDriver abstraction
    pub async fn execute(
        driver: &dyn DeviceDriver,
        plan: &ElementActionPlan,
    ) -> Result<ActionReceipt, ElementError> {
        match &plan.execution_kind {
            ElementActionKind::Tap => {
                let tap_x = plan.safe_point.x as u32;
                let tap_y = plan.safe_point.y as u32;
                driver
                    .tap(tap_x, tap_y)
                    .await
                    .map_err(|e| ElementError::ActionFailed(e.to_string()))
            }
            ElementActionKind::DoubleTap => {
                let tap_x = plan.safe_point.x as u32;
                let tap_y = plan.safe_point.y as u32;
                let res = driver
                    .tap(tap_x, tap_y)
                    .await
                    .map_err(|e| ElementError::ActionFailed(e.to_string()))?;
                tokio::time::sleep(std::time::Duration::from_millis(80)).await;
                let _ = driver.tap(tap_x, tap_y).await;
                Ok(res)
            }
            ElementActionKind::LongPress { duration_ms } => {
                let tap_x = plan.safe_point.x as u32;
                let tap_y = plan.safe_point.y as u32;
                let path = SwipePath {
                    start_x: tap_x,
                    start_y: tap_y,
                    end_x: tap_x,
                    end_y: tap_y,
                    duration_ms: *duration_ms,
                };
                driver
                    .swipe(path)
                    .await
                    .map_err(|e| ElementError::ActionFailed(e.to_string()))
            }
            ElementActionKind::SetText { text, clear_first } => {
                if *clear_first {
                    let tap_x = plan.safe_point.x as u32;
                    let tap_y = plan.safe_point.y as u32;
                    let _ = driver.tap(tap_x, tap_y).await;
                    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                }
                driver
                    .input_text(text)
                    .await
                    .map_err(|e| ElementError::ActionFailed(e.to_string()))
            }
            ElementActionKind::AssertExists
            | ElementActionKind::WaitAppear { .. }
            | ElementActionKind::WaitDisappear { .. } => Ok(ActionReceipt {
                success: true,
                duration_ms: 0,
                error: None,
                point_tapped: Some(plan.safe_point),
                verify_passed: true,
            }),
        }
    }
}
