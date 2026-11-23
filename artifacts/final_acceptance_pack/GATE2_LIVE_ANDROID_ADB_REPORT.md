# FARMERPRO — GATE 2: LIVE ANDROID ADB VALIDATION REPORT

**Status:** `PASS (LIVE_ANDROID_ADB_VERIFIED — PLATFORM & ANDROID SETTINGS)` · **Game-Specific Workflows:** `NOT TESTED / PENDING`  
**Verified RC Code Baseline:** `b5a05673b1c0ad10dd4c6ce4463eaabbd3be40fe` (Git Tag: `v0.1.0-p6-rc.2`; hotfix từ `v0.1.0-p6-rc.1` `802f316` sau khi phát hiện trùng transport `emulator-5554`/`127.0.0.1:5555` và lỗi parse `adb reverse --list`)  
**Addendum:** [GATE2_ACCEPTANCE_ADDENDUM.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/GATE2_ACCEPTANCE_ADDENDUM.md)  
**Verification Harness:** [scripts/verify_gate2_live_adb.js](file:///e:/DATA/FarmerPro/scripts/verify_gate2_live_adb.js)  
**Machine-Readable Evidence:** [gate2_live_adb_report.json](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_live_adb_report.json)

---

## 1. Chẩn đoán LDPlayer & Khởi chạy Android SDK AVD (`fp_gate2_avd`)

1. **Nguyên nhân `LDPlayer9` / `LDPlayer14` dừng trên máy chủ:**
   - `C:\LDPlayer\LDPlayer9\vms\leidian0\Logs\VBox.log`: `00:00:00.250446 HM: HMR3Init: Attempting fall back to NEM: VT-x is not available` khi WHv đang kích hoạt (`WHvCapabilityCodeHypervisorPresent is TRUE`), dẫn đến `00:01:34.771087 Heartbeat flatlined` → `PDMR3PowerOff` / `VERR_INTERRUPTED`.
2. **Android SDK AVD (`fp_gate2_avd`, `1280×720` WHPX):**
   - `emulator-check.exe accel` xác nhận `WHPX(10.0.26200) is installed and usable`.
   - `adb devices -l` kết nối ổn định: `emulator-5554 device product:sdk_gphone16k_x86_64 model:sdk_gphone16k_x86_64 device:emu64xa16k transport_id:1`.
   - Kiểm chứng đồng nhất `boot_id` (`5ddf4574-9b3f-450f-9550-b39f4c1a95bf`) và `ro.boot.serialno` (`EMULATOR37X1X11X0`) giữa `emulator-5554` và `127.0.0.1:5555` xác nhận cả hai là cùng 1 thực thể AVD; hotfix `b5a0567` hợp nhất về 1 bản ghi duy nhất `emulator-5554`.

---

## 2. Kết quả Kiểm chứng FarmerPro Studio (`FarmerPro.exe`, `v0.1.0-p6-rc.2`)

- **Canonical Discovery & Device Lab:** Dù `127.0.0.1:5555` được gắn trước khi chạy Studio, `auto_discover_devices` và `list_devices` trả về **`adbDiscoveredCount = 1`** (`emulator-5554`, API `37`, `ready`). Xem [gate2_studio_device_lab.png](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_studio_device_lab.png).
- **Live ADB Forward & Reverse (Create → List → Remove):**
  - `studio_add_device_forward` (`tcp:19555 -> tcp:19556`) & `studio_add_device_reverse` (`tcp:19557 -> tcp:19558`).
  - `studio_list_device_ports` ghi nhận cả `forwards` và `reverses` với đúng `serial = "emulator-5554"`, `local = "tcp:19558"`, `remote = "tcp:19557"`.
  - `studio_remove_device_forward` & `studio_remove_device_reverse` xóa sạch về `forwards: [], reverses: []`.
- **Live Screenshot, Template Match & Workflow Lifecycle:**
  - `capture_device` (`78,728` B PNG), template crop (`260×56`), match confidence **`0.93104136`** (`bounds: {x: 54, y: 78, w: 221, h: 48}`, `suggestedTapPoint: {x: 128.64, y: 99.69}`).
  - **Job 1 (`job-19f46ff3-b257-4122-8716-991742e37c26`):** `Pause` (`paused`) → `Resume` → OCR (`"11:26 Q Search Settings..."`, `ocr_matched = true`, `confidence = 0.94`, `source = LIVE_ADB_OCR`, `verified = true`) → `random_safe_tap` → `swipe` → `checkpoint` → `state.set_var` → `succeeded` (`24` events). Xem [gate2_studio_runtime_vars.png](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_studio_runtime_vars.png).
  - **Job 2 (`job-51284b9e-7825-4b73-ae1c-aaa2f422cde5`):** `suspend_job_to_checkpoint` (`paused`, `cp-854fc17a-62fb-4bbe-8d2e-902f21108ff1`) → `resume_job_from_checkpoint` (`resumedFromNodeId = "c3"`, `fencingGeneration = 3`) → `succeeded` (`13` events, `checkpoint_resumed = "SETTINGS_HOME_RESTORED"`).
  - **Job 3:** `studio_cancel_job` → `cancelled`.

---

## 3. Kết quả Mini Tool Độc lập (`Auto_Gate2Live.exe`, Đã Đóng & Gỡ Studio)

Sau khi chạy `uninstall.exe /S` gỡ Studio khỏi sandbox, giải nén `Gate2_LiveMini_v1.0.0_win-x64.zip` (`11,944,570` B, SHA-256 `ddb899b491df8697cc7fe817987e9a13ae2d1f2819da77568c46290afbae9424`):
- **Device Pill:** `#adb-mode-pill` hiển thị chính xác **`LIVE ADB: 1 ONLINE`** cả trước và sau chạy. Xem [gate2_mini_before.png](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_mini_before.png) và [gate2_mini_after.png](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_mini_after.png).
- **Chạy qua `CHẠY NHIỆM VỤ (ADB THẬT)`:** `anyTestkitBadge = false`, không có `FakeDeviceDriver`:
  - `ocr_matched = true` (`LIVE_ADB_OCR`, `verified = true`, `confidence = 0.94`)
  - `ocr_screen_label = "11:27 Q Search Settings G Google Services & preferences Network & internet Mobile, Wi-Fi, hotspot Connected devices Bluetooth, pairing Apps Assistant, recent apps, default apps"` (`LIVE_ADB_OCR`, `verified = true`)
  - `mini_live_result = "SEARCH_SETTINGS_OPENED"` (`LIVE_STATE`, `verified = true`)
  - `last_data_source = "LIVE_ADB(emulator-5554)"`, `last_task_state = "Succeeded"`, `task_completed_count = 1`
  - SQLite độc lập (`runtime_live-gate2-android.db`): job `mini-job-f773ccb7-e695-4c4d-928a-498a317dd58f` trên `emulator-5554` (`Succeeded`), `22` `step_events`.
- **Ảnh Trước / Sau Trên Thiết bị Android (`emulator-5554`):**
  - Studio: [gate2_device_before_studio.png](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_device_before_studio.png) (`59,095` B, Settings Home) → [gate2_device_after_studio.png](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_device_after_studio.png) (`50,040` B, Search settings + Gboard).
  - Mini Tool: [gate2_device_before_mini.png](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_device_before_mini.png) (`58,886` B, Settings Home) → [gate2_device_after_mini.png](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_device_after_mini.png) (`25,119` B, Search settings opened).
