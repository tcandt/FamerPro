use crate::model::ElementError;
use crate::selectors::SelectorMatch;
use fp_actions::{SafeRegionSampler, SafeTapPolicy};
use fp_domain::{Bounds, Point};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActionExecutionMode {
    DynamicBoundsTap,
    ClickableParent,
    SemanticClick,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ElementActionKind {
    Tap,
    DoubleTap,
    LongPress { duration_ms: u64 },
    SetText { text: String, clear_first: bool },
    WaitAppear { timeout_ms: u64 },
    WaitDisappear { timeout_ms: u64 },
    AssertExists,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ElementActionPlan {
    pub mode: ActionExecutionMode,
    pub target_bounds: Bounds,
    pub safe_point: Point,
    pub is_clickable_parent_used: bool,
    pub execution_kind: ElementActionKind,
}

/// Computes the exact safe randomized tap target for an element match
pub fn plan_element_action(
    selector_match: &SelectorMatch,
    kind: ElementActionKind,
    preferred_mode: ActionExecutionMode,
    policy: Option<SafeTapPolicy>,
) -> Result<ElementActionPlan, ElementError> {
    let tap_policy = policy.unwrap_or(SafeTapPolicy {
        margin_ratio: 0.15,
        min_area_px: 36.0,
        avoid_recent_points: 3,
        min_distance_px: 6.0,
        max_tries: 30,
    });

    let (use_bounds, is_parent) = match preferred_mode {
        ActionExecutionMode::ClickableParent => {
            if selector_match.resolved_clickable_ancestor_id.is_some() {
                (selector_match.resolved_bounds, true)
            } else {
                (selector_match.bounds, false)
            }
        }
        _ => (
            selector_match.resolved_bounds,
            selector_match.resolved_clickable_ancestor_id.is_some(),
        ),
    };

    if use_bounds.area() <= 0.0 {
        return Err(ElementError::OffscreenOrOccluded(use_bounds));
    }

    // Apply safe margin
    let safe_box =
        SafeRegionSampler::apply_margin(&use_bounds, tap_policy.margin_ratio).unwrap_or(use_bounds);

    // Randomize within safe box
    let rx = rand::random::<f32>();
    let ry = rand::random::<f32>();

    let tap_x = safe_box.x + rx * safe_box.width;
    let tap_y = safe_box.y + ry * safe_box.height;

    Ok(ElementActionPlan {
        mode: preferred_mode,
        target_bounds: use_bounds,
        safe_point: Point { x: tap_x, y: tap_y },
        is_clickable_parent_used: is_parent,
        execution_kind: kind,
    })
}
