# FARMERPRO — UNIVERSAL ELEMENT INSPECTOR UI GALLERY
**Phase 2.1 Acceptance Artifact: Authentic Desktop Application Evidence**
**Target Device**: Samsung Galaxy S7 (`SM-G930F`), LineageOS 17.1 (Android 10), Resolution: `540x960` (Physical: `1440x2560`)
**Device Serial**: `192.168.1.168:5555` (TCP/IP LAN)

---

## Executive Overview
In response to the Phase 2 audit feedback regarding the lack of desktop application visual proof, this UI Gallery presents genuine screenshots captured from the running FarmerPro desktop frontend (`apps/desktop`). 

The interface demonstrates the unified 4-panel architecture embedded in **Device Lab Inspector**, eliminating duplicate sidebar entries and adhering to Phase 2.1 requirements:
1. **Device Preview Canvas**: Real-time rendering of the connected Samsung Galaxy S7 screen with interactive hover/select overlays and test detection bounding boxes.
2. **UI Hierarchy Tree**: Live DOM-style tree with node expansion, collapsible branches, and search filtering.
3. **Properties & Selectors**: Full attribute inspection (Class, ID, Text, Content Desc, Bounds, Package) and auto-generation of 4 distinct selector strategies with uniqueness scoring.
4. **Smart Action Engine**: Separation of **Test Detection** from **Test Action**, Rule 5 safe margin calculation (15% inner padding), and **Element Library** SQLite management.

---

## 1. Element Inspector Overview (Device Lab Integration)
This view shows the complete desktop studio layout with the Samsung Galaxy S7 selected as the active device and the Element Inspector tab loaded.

![01 Element Inspector Overview](file:///C:/Users/TINH-NGUYEN/.gemini/antigravity/brain/ee02dcf5-aa3a-4a09-a189-f47c397579e0/01_element_inspector_overview.png)

### Key Architectural Highlights:
- **Transport Independence**: Connected via ADB TCP/IP LAN (`192.168.1.168:5555`).
- **Telemetry Header**: Displays element count (`11 Elements`), dump latency (`104ms`), and Rust parsing latency (`2ms`).
- **Selector Mode Switcher**: Quick toggle between `NATIVE`, `OCR`, `IMAGE`, `POINT`, and `HYBRID`.
- **Mode Toggle**: Instant switch between `Inspect` mode (pointer detection) and `Control` mode (direct touchscreen interaction).

---

## 2. Inspect Mode & Auto-Generated Selectors
When an element is selected (either by clicking directly on the Device Preview screen or within the UI Hierarchy Tree), FarmerPro highlights the bounding box and generates four ranked selector strategies.

![02 Inspect Mode and Properties](file:///C:/Users/TINH-NGUYEN/.gemini/antigravity/brain/ee02dcf5-aa3a-4a09-a189-f47c397579e0/02_inspect_mode_and_properties.png)

### Attributes & Uniqueness Verification:
- **Target Node**: `#8 <FrameLayout>` (`com.android.launcher3:id/icon_browser`)
- **Generated Strategies**:
  1. `Resource ID`: `element.id("com.android.launcher3:id/icon_browser")` (100% Unique, **Recommended**)
  2. `Exact Text`: `element.text("Browser")` (100% Unique)
  3. `Class & Hierarchy`: `element.class("android.widget.FrameLayout")` (85% Unique)
  4. `Absolute XPath`: `xpath("/hierarchy/browser_frame")` (100% Unique)

---

## 3. Clickable Parent Resolution (Guard Against Dead Taps)
Android UI hierarchies frequently nest unclickable `<TextView>` or `<ImageView>` labels inside clickable containers (`<FrameLayout>` or `<LinearLayout>`). Clicking the raw label often results in dropped touch events.

FarmerPro automatically detects non-clickable children and traverses the ancestor tree to find the nearest clickable parent.

![03 Clickable Parent Resolution](file:///C:/Users/TINH-NGUYEN/.gemini/antigravity/brain/ee02dcf5-aa3a-4a09-a189-f47c397579e0/03_clickable_parent_resolution.png)

### Inspection Details:
- **Selected Element**: Node `#2` (`com.android.deskclock:id/clock`), `clickable: false`.
- **Amber Warning Alert**:
  > *"Element is not clickable. Click will resolve to parent: FrameLayout"*
- **Resolution Control**: Provides a one-click button `Select Clickable Parent` to navigate up the tree.
- **Execution Mode**: In `ClickableParent (Auto)` mode, the execution engine dispatches touch coordinates to the parent's resolved bounds instead of the child's raw bounds.

---

## 4. Test Detection Verification (Independent from Test Action)
Per the Phase 2.1 specification, **Test Detection** must be decoupled from **Test Action**:
- **Test Detection**: Validates whether the selector engine resolves the target element with sufficient confidence without sending any physical touch events to the phone.
- **Visual Feedback**: Renders an animated dashed yellow bounding box on the device canvas along with a confidence badge (`Detected: 99%`).

![04 Test Detection Verified](file:///C:/Users/TINH-NGUYEN/.gemini/antigravity/brain/ee02dcf5-aa3a-4a09-a189-f47c397579e0/04_test_detection_verified.png)

### Verification Metrics:
- **Strategy Used**: `Native: com.android.launcher3:id/icon_browser`
- **Candidate Count**: `1` (Unique match validation passed)
- **Confidence**: `99%`
- **Bounds Verified**: `[321, 822] (101x135)`

---

## 5. Element Library Drawer & SQLite Persistence
FarmerPro allows saving verified elements into an internal SQLite repository (`element_library` table) scoped by project ID with automatic versioning.

![05 Element Library Drawer](file:///C:/Users/TINH-NGUYEN/.gemini/antigravity/brain/ee02dcf5-aa3a-4a09-a189-f47c397579e0/05_element_library_drawer.png)

### Library Features:
- **Project Scoping**: Elements are persisted under `projectId` (e.g., `default_project`).
- **Strategy Metadata**: Displays primary detection method (`NATIVE`, `OCR`, `IMAGE`, `POINT`).
- **Version Tracking**: Auto-increments versions on update (`v1`, `v2`).
- **Reusability**: Saved elements can be dragged or imported directly into Workflow Studio nodes.

---

## 6. Fullscreen Inspector Mode
For complex game layouts and high-density UI hierarchies, the Element Inspector can expand to full screen with a single click.

![06 Fullscreen Inspector Mode](file:///C:/Users/TINH-NGUYEN/.gemini/antigravity/brain/ee02dcf5-aa3a-4a09-a189-f47c397579e0/06_fullscreen_inspector_mode.png)

---

## Summary of Verification Evidence

| Feature Requirement | Implementation | Evidence Artifact |
| :--- | :--- | :--- |
| **Unified 4-Panel UI** | Canvas + Hierarchy + Properties + Action Builder | `01_element_inspector_overview.png` |
| **Inspect Mode & Bounding Box** | Live coordinate scaling + node hover overlay | `02_inspect_mode_and_properties.png` |
| **Clickable Parent Guard** | Automatic traversal to clickable ancestor with amber warning | `03_clickable_parent_resolution.png` |
| **Decoupled Test Detection** | Yellow dashed bounding box + confidence % without touch dispatch | `04_test_detection_verified.png` |
| **Element Library SQLite** | Project-scoped storage with versioning and deletion | `05_element_library_drawer.png` |
| **Fullscreen Mode** | Viewport expansion for in-depth hierarchy analysis | `06_fullscreen_inspector_mode.png` |
