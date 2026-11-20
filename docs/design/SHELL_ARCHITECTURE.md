# FarmerPro Shell Architecture & Layout Responsive Rules

## Layout Hierarchy
```text
+-------------------------------------------------------------------------------+
| AppTitlebar (38px) - Window dragging region, app title, minimize/maximize/close |
+-------------------------------------------------------------------------------+
| AppCommandBar (52px) - Breadcrumb, Workflow Name, Target Device, Undo/Redo, Run|
+----+--------------------------------------------------------------------------+
| N  | Main Workspace Content Area                                              |
| A  | +----------------------------------------------------------------------+ |
| V  | | Active View (Overview / Devices / Tasks / Runs / Settings)            | |
|    | | OR                                                                   | |
| R  | | Workflow Studio 3-Panel Hero Canvas:                                  | |
| A  | | +--------------+------------------------------------+--------------+ | |
| I  | | | Palette      | Canvas                             | Inspector    | | |
| L  | | | (240px)      | (flex 1, min 55% width)            | (312px)      | | |
|    | | +--------------+------------------------------------+--------------+ | |
| 64 | | Execution Dock (resizable 160px - 420px, default 220px, collapsible) | |
| /  | +----------------------------------------------------------------------+ |
| 220|                                                                          |
+----+--------------------------------------------------------------------------+
| StatusBar (24px) - ADB daemon status, connected count, active lease, local mode |
+-------------------------------------------------------------------------------+
```

## Responsive Viewport Rules

| Viewport Width | Navigation Rail | Palette Panel | Canvas Area | Inspector Panel | Execution Dock |
|---|---|---|---|---|---|
| **1600×900** (Primary Target) | Expanded (220px) | Fixed (240px) | Flex 1 (~828px, 60% of workspace) | Fixed (312px) | Resizable (220px default) |
| **1440×900** (Standard Laptop) | Collapsed (64px) | Fixed (232px) | Flex 1 (~832px) | Fixed (304px) | Resizable (210px) |
| **1280×720** (Compact Screen) | Collapsed (64px) | Collapsible (216px) | Flex 1 (~712px) | Drawer / Collapsible (288px) | Bottom 180px (collapsible) |
| **1120×700** (Minimum Window) | Collapsed (64px) | Drawer Popover | Flex 1 (~768px) | Sheet Overlay | Tabbed modal 160px |

## Design Non-Negotiables
1. **No Clipping**: At 1280×720 and 1440×900, the inspector and node palette can be toggled or collapsed into side drawers, ensuring the canvas always retains at least 50% screen width.
2. **Contextual Inspector**: When no node is selected on the canvas, the inspector renders a clear empty state with canvas properties and keyboard shortcuts rather than empty disabled inputs.
3. **Collapsible Execution Dock**: The bottom log and verification dock can be collapsed with a single click or keyboard shortcut (`Ctrl+J`) to maximize canvas working area.
