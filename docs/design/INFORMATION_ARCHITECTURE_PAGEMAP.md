# FarmerPro Information Architecture & Page Map

## 1. Top-Level Views (Shell Navigation)
The left-hand navigation rail groups the application into 3 clear domains:

### A. Operations (Operational Domain)
1. **Overview (`/`)**: High-level command center with real device counts, active jobs, attention queue, and recent execution runs.
2. **Devices (`/devices`)**: Device management table, ADB connection diagnostics, battery/orientation metadata, and on-demand screen capture inspector.
3. **Workflows (`/workflows`)**: Workflow catalog, draft/published status, template importing, cloning, and version history.
4. **Workflow Studio (`/studio`)**: The hero visual graph editor. Contains the node palette, React Flow canvas, node property inspector, and resizable execution dock.
5. **Tasks (`/tasks`)**: Task catalog, selected task sequence, priority scheduling, prerequisite evaluation, and execution plan preview.
6. **Runs (`/runs`)**: Execution history, step-by-step event timeline, screenshot evidence viewer, and diagnostic log export.

### B. Resources
7. **Accounts (`/accounts`)**: Game account profiles, alias mapping, secret credential references (Windows Credential Store), and device assignments.
8. **Network / Proxy (`/network`)**: Per-device network interface configuration, system proxy checks, VPN status, and latency testing.

### C. System
9. **Settings (`/settings`)**: ADB path configuration, SQLite storage metrics, log retention policies, appearance preferences, and system diagnostics.
10. **Design System & Component Gallery (`/gallery`)**: Verification playground displaying all UI primitives, component states, and responsive viewport simulators (1600x900, 1440x900, 1280x720).

## 2. Key Operator Flows
- **Flow 1: Connect & Verify Device**: Connect USB/TCP Android -> Overview shows device -> Open Devices view -> Run ADB inspection -> Capture fresh screen frame -> Ready.
- **Flow 2: Build & Validate Workflow**: Open Studio -> Drag Launch App, Capture, Find Image, Safe Tap, Verify -> Connect typed ports -> Click "Kiểm tra luồng" (Validate) -> Zero errors.
- **Flow 3: Step-by-Step Interactive Test**: Select target device in Studio command bar -> Click "Chạy thử node" (Test Node) -> View fresh capture and detected bounding box -> Execute Safe Random Tap -> Verify receipt.
- **Flow 4: Task Plan Sequencing**: Open Tasks -> Pick "Nhiệm vụ tân thủ" -> Reorder priorities -> Evaluate prerequisites -> Execute plan -> Monitor execution timeline in Runs dock.
- **Flow 5: Account Assignment & Switch**: Link game account to device -> Verify credential reference -> Trigger scheduled task set.
