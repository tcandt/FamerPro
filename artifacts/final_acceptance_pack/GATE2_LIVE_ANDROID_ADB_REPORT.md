# FARMERPRO — GATE 2: LIVE ANDROID ADB VALIDATION REPORT

**Status:** `PASS (LIVE_ANDROID_ADB_VERIFIED — PLATFORM & ANDROID SETTINGS)` · **Game-Specific Workflows:** `NOT TESTED`  
**Verified RC Code Baseline:** `629cce236e173f91642684858e44638948773b9e` (Git Tag: `v0.1.0-p6-rc.3`)  
**Addendum:** [GATE2_ACCEPTANCE_ADDENDUM.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/GATE2_ACCEPTANCE_ADDENDUM.md)  
**Verification Harness:** [scripts/verify_gate2_live_adb.js](file:///e:/DATA/FarmerPro/scripts/verify_gate2_live_adb.js)  
**Machine-Readable Evidence:** [gate2_live_adb_report.json](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_live_adb_report.json)

---

## 1. Chẩn đoán LDPlayer & Môi trường Android SDK AVD (`fp_gate2_avd`)

1. `C:\LDPlayer\LDPlayer9\vms\leidian0\Logs\VBox.log`: `HM: HMR3Init: Attempting fall back to NEM: VT-x is not available` khi WHv kích hoạt (`WHvCapabilityCodeHypervisorPresent is TRUE`), dẫn đến `00:01:34.771087 Heartbeat flatlined` → `PDMR3PowerOff` / `VERR_INTERRUPTED`.
2. Android SDK AVD (`fp_gate2_avd`, `1280×720`, `WHPX 10.0.26200 usable`) kết nối ổn định qua `emulator-5554`. Khi gắn thêm `adb connect 127.0.0.1:5555`, cả hai đều trả về `ro.boot.serialno = EMULATOR37X1X11X0` và cùng kernel `boot_id` (`identicalInstanceConfirmed = true`), được hợp nhất về 1 bản ghi duy nhất `emulator-5554`.

---

## 2. Kết quả FarmerPro Studio (`FarmerPro.exe`, `v0.1.0-p6-rc.3`)

- **Discovery:** Dù `127.0.0.1:5555` đang gắn, `auto_discover_devices` & `list_devices` trả về **`adbDiscoveredCount = 1`** (`emulator-5554`, API `37`, `ready`). Xem [gate2_studio_device_lab.png](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_studio_device_lab.png).
- **ADB Forward & Reverse (Create → List → Remove):** Tạo `tcp:19555 -> tcp:19556` (forward) và `tcp:19557 -> tcp:19558` (reverse, `serial = emulator-5554`), kiểm tra trong `studio_list_device_ports`, xóa sạch về `forwards: [], reverses: []`.
- **Live Capture, Template Match & Jobs 1 / 2 / 3:**
  - Template match: `matched = true`, `confidence = 0.93104136`, `bounds = {x: 54, y: 78, w: 221, h: 48}`, `suggestedTapPoint = {x: 218.72, y: 87.45}`.
  - **Job 1 (`job-33aed864-1311-4648-a654-b1965ed73f9d`):** `Pause` (`paused`) → `Resume` → OCR + SafeTap + Swipe + Checkpoint → `succeeded` (`24` events). Xem [gate2_studio_runtime_vars.png](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_studio_runtime_vars.png).
  - **Job 2 (`job-cee835ee-876e-4829-b3b7-4844c9d2d5a7`):** `suspend_job_to_checkpoint` (`paused`, `cp-9e0e6dd9-ce8a-45ea-b8d7-d6d906990dc9`) → `resume_job_from_checkpoint` (`resumedFromNodeId = "c3"`, `fencingGeneration = 3`) → `succeeded` (`13` events, `checkpoint_resumed = "SETTINGS_HOME_RESTORED"`).
  - **Job 3:** `studio_cancel_job` → `cancelled`.

---

## 3. Kết quả Mini Tool Độc lập (`Auto_Gate2Live.exe`, Đã Đóng & Gỡ Studio)

Sau khi chạy `uninstall.exe /S` gỡ Studio khỏi sandbox, giải nén `Gate2_LiveMini_v1.0.0_win-x64.zip` (`11,987,054` B, SHA-256 `2de448a5e263cbbc84b5aa16acfa4a13ce0e78f0c989d8150f3d5cfcfb1aaa73`):
- **Device Pill & OCR Provenance UI:** `#adb-mode-pill` hiển thị **`LIVE ADB: 1 ONLINE`**; bảng biến hiển thị **`LIVE · Đã khớp quy tắc`** (`anyTestkitBadge = false`, `anyUnmeasuredPercentBadge = false` — không gán `94%` giả tạo). Xem [gate2_mini_before.png](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_mini_before.png) và [gate2_mini_after.png](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_mini_after.png).
- **Envelopes & SQLite Độc lập (`runtime_live-gate2-android.db`):**
  - `ocr_matched = true`, `ocr_screen_label` / `ocr_text = "12:14 Q Search Settings G Google Services & preferences Network & internet Mobile, Wi-Fi, hotspot Connected devices Bluetooth, pairing Apps Assistant, recent apps, default apps"` (`source = LIVE_ADB_OCR`, `verified = true`, `confidence = null`, `confidenceSource = "HEURISTIC"`, `measuredConfidence = null`, `ruleMatched = true`, `deviceId = "emulator-5554"`).
  - `mini_live_result = "SEARCH_SETTINGS_OPENED"`, `last_data_source = "LIVE_ADB(emulator-5554)"`, `last_task_state = "Succeeded"`, `task_completed_count = 1`.
  - Job `mini-job-4c529151-6789-4147-b222-e369abdd6a10` (`emulator-5554`, `Succeeded`), `22` `step_events`.
- **Ảnh Trước / Sau Trên Thiết bị (`emulator-5554`):**
  - Studio: [gate2_device_before_studio.png](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_device_before_studio.png) (`59,064` B) → [gate2_device_after_studio.png](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_device_after_studio.png) (`61,447` B).
  - Mini Tool: [gate2_device_before_mini.png](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_device_before_mini.png) (`58,919` B) → [gate2_device_after_mini.png](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_device_after_mini.png) (`49,826` B).
