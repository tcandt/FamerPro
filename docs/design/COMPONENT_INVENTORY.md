# FarmerPro Component Inventory & Contract Specifications

## 1. Primitive UI Elements
- `Button`: Primary (`#1878F5`), Secondary (`#192230`), Outline, Ghost, Destructive (`#F27883`).
- `Badge`: Success (`#3BC38D`), Warning (`#E7B15B`), Danger (`#F27883`), Neutral (`#7BA6D6`), Vision (`#B291F7`).
- `Input` & `Textarea`: Dark slate surface (`#192230`), border subtle (`#2A3647`), focus ring (`#4B9CFF`).
- `Select` & `DropdownMenu`: Accessible keyboard navigation, high contrast focus states.
- `Switch` & `Checkbox`: Clean toggle mechanics with accessible ARIA labels.
- `Tabs`: Segmented controls for Inspector sections and Execution Dock tabs.
- `Dialog` & `Sheet`: Modal overlays with backdrop blur and Esc key dismiss.
- `Tooltip`: Instant helpful hover tooltips for icon-only rail controls.

## 2. Compound Components
- `AppShell`: Window frame, titlebar, command bar, navigation rail, and status footer.
- `WorkflowCanvas`: React Flow container with background grid, minimap, zoom controls, and custom edge renderers.
- `CustomNode`: Standard card container for graph nodes:
  - Header: Icon, Node Category, Node Label, Status Chip.
  - Body: Summary configuration badges.
  - Ports: Input handle (left), Output handle (right), True/False branch handles (bottom/right).
- `NodePalette`: Category accordions (Flow, Device, App, Vision, Action, Logic, Verify, State) with searchable nodes.
- `NodeInspector`: Contextual form panels with dynamic schemas for each node type:
  - Tab 1: Configuration (parameters, target image reference, package name, timeout).
  - Tab 2: Inputs / Outputs (typed port bindings).
  - Tab 3: Error & Retry Policy (max attempts, backoff, fail-closed policy).
  - Tab 4: Notes & Documentation.
- `ExecutionDock`: Tabbed bottom drawer:
  - Tab: Kết quả (Results / receipts)
  - Tab: Timeline (Step-by-step Gantt-like progress)
  - Tab: Logs (Structured filterable log table with secret redaction)
  - Tab: Ảnh xác minh (Verification frame captures with highlighted bounding boxes)
- `DeviceCard` & `DeviceRow`: Device status indicator, battery, model, resolution, connection type, and actions.
- `TaskItem` & `TaskSequence`: Reorderable task sequence with drag handles, priority chips, and readiness indicators.
