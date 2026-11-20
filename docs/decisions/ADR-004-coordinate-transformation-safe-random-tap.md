# ADR-004: Safe Random Tap Policy and Coordinate Transformations

## Status
Accepted

## Context
Fixed hard-coded pixel taps (`tap(500, 800)`) fail across different screen sizes, aspect ratios, orientations, and UI scaling factors. Furthermore, clicking the exact same pixel repeatedly is fragile and violates the product's safety guidelines.

## Decision
1. **Fresh Observation Requirement**: Every tap action must be grounded in a freshly captured frame and its associated `VisionTarget` bounding box.
2. **Safe Margin Contraction**: The detected bounding box is shrunk inward by `margin_ratio` (default: 0.18) so that sampled points stay well inside the button boundaries.
3. **Exclusion Masks**: Rectangles/polygons marked as dangerous (e.g. system navigation bar, in-game cancel buttons, notifications) are subtracted from the safe region.
4. **Recent Points Window**: The random sampling algorithm records the last N sampled points (default: 5) and enforces a minimum Euclidean distance between consecutive clicks when area allows.
5. **Coordinate Transform**: Normalization transforms capture coordinates (`capture_px`) to device display coordinates (`display_px`) accounting for orientation (0°, 90°, 180°, 270°) and display insets. If orientation changes during the flow, the engine fails closed with `StaleGeometry` and triggers recapture.
6. **Post-Condition Verification**: Tapping does not infer success. A subsequent verification step captures a fresh frame to confirm state transition before completing the step.

## Consequences
- Guaranteed point containment within valid UI buttons.
- Robust execution regardless of screen resolution differences.
- Zero blind clicks when disconnected or when geometry changes.
