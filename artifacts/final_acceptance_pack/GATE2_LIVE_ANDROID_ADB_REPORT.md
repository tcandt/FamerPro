# FARMERPRO — GATE 2: LIVE ANDROID ADB VALIDATION REPORT

**Status:** `PASS (LIVE_ANDROID_ADB_VERIFIED)`  
**Frozen RC Code Baseline:** `802f3162df7fe3ab8fd6c7085c9d9e34caca6648` (Git tag `v0.1.0-p6-rc.1`)  
**Verification Harness:** [scripts/verify_gate2_live_adb.js](file:///e:/DATA/FarmerPro/scripts/verify_gate2_live_adb.js)  
**Machine-Readable Evidence:** [gate2_live_adb_report.json](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_live_adb_report.json)

---

## 1. Chẩn đoán Nguyên nhân LDPlayer14 / LDPlayer9 Tự Dừng & Phương án Thay thế

1. **Chẩn đoán trực tiếp từ `VBox.log` (`C:\LDPlayer\LDPlayer9\vms\leidian0\Logs\VBox.log`):**
   - VirtualBox backend của LDPlayer ghi nhận:
     - `00:00:00.250446 HM: HMR3Init: Attempting fall back to NEM: VT-x is not available` (`UseNEMInstead = 0`)
     - `00:00:00.253537 NEM: WHvCapabilityCodeHypervisorPresent is TRUE, so this might work...` (do Hyper-V / Windows Hypervisor Platform đang giữ VT-x)
     - Sau khi guest kernel khởi chạy dưới chế độ NEM/WHv, tới `00:01:34.771087` phát sinh `Heartbeat flatlined` và VM đi vào `PDMR3PowerOff` / `VBoxHeadless: processEventQueue: VERR_INTERRUPTED, termination requested` (bổ sung cho trường hợp tiến trình con khởi chạy trong sub-shell bị thu hồi theo Windows JobObject khi shell thoát).
2. **Chuyển sang Giả lập Hoạt động Theo Chỉ đạo (Gate 2 Fallback):**
   - Kiểm tra công cụ chuẩn của máy phát triển: `emulator-check.exe accel` xác nhận **`WHPX(10.0.26200) is installed and usable`**.
   - Tạo và khởi chạy AVD chuẩn Android SDK (`fp_gate2_avd`, `system-images\android-37.2-beta3\google_apis_ps16k\x86_64`, `1280×720` landscape, `WHPX`).
   - Kết nối ổn định với `E:\DATA\FarmerPro\bin\adb.exe` (`emulator-5554 device product:sdk_gphone16k_x86_64 model:sdk_gphone16k_x86_64 device:emu64xa16k`).

---

## 2. Kết quả Kiểm chứng Trực tiếp từ FarmerPro Studio (`FarmerPro.exe`, RC `802f316`)

Chạy bộ cài `FarmerPro_0.1.0_x64-setup.exe` trong sandbox cô lập hoàn toàn khỏi DB sản xuất:
- **ADB Discovery & Device Lab:** `auto_discover_devices` + `list_devices` phát hiện thiết bị trực tuyến (`adbDiscoveredCount = 2`: `emulator-5554` & `127.0.0.1:5555`, `connection_state: "ready"`, Android API `37`). Xem [gate2_studio_device_lab.png](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_studio_device_lab.png).
- **Live Screenshot & Vision Template Match:** `capture_device` (`79,368` bytes base64 PNG) + `studio_capture_and_crop_template` cắt vùng nút `Search Settings` (`x=50, y=76, 260×56`) -> `studio_test_project_template_match` khớp trực tiếp với độ tin cậy **`0.93104136`** (`bounds: {x: 54, y: 78, width: 221, height: 48}`, `suggestedTapPoint: {x: 168.95, y: 108.90}`).
- **Job 1 — Live Workflow (OCR + FindImage + SafeTap + Swipe + Pause/Resume + Checkpoint):**
  - `job-4314bc3f-2f24-4431-9b55-4742fb7bc1cc`: Tạm dừng thành công khi đang chạy (`job1PausedState = "paused"`), sau đó Resume (`studio_resume_job`) chạy tiếp tới `state = "succeeded"` với `24` step events trong SQLite WAL ([gate2_studio_runtime_vars.png](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_studio_runtime_vars.png)).
  - Thiết bị thật trước (`gate2_device_before_studio.png`, Settings Home) chuyển rõ rệt sang sau (`gate2_device_after_studio.png`, màn hình Search settings + bàn phím Gboard bật lên sau Safe Random Tap).
- **Job 2 — Suspend to Checkpoint & Resume from Checkpoint:**
  - `job-eb8b6611-2f53-4265-b0a4-eb5b3df04ba6`: `studio_suspend_job_to_checkpoint` -> `studio_resume_job_from_checkpoint` khôi phục từ `resumedFromNodeId = "c3"` (`checkpointId = "cp-7afeb446-d67c-4b9c-9e0d-631d6de32e01"`, `fencingGeneration = 3`) -> `state = "succeeded"`, ghi biến `checkpoint_resumed = "SETTINGS_HOME_RESTORED"`.
- **Job 3 — Cancel Control:**
  - `studio_cancel_job` dừng tác vụ đang chạy và kết thúc ở `job3FinalState = "cancelled"`.

---

## 3. Kết quả Kiểm chứng Trực tiếp từ Mini Tool Độc lập (`Auto_Gate2Live.exe`, Đã Đóng & Gỡ Studio)

Studio đóng và chạy `uninstall.exe /S` khỏi sandbox trước khi khởi chạy `Auto_Gate2Live.exe` (giải nén từ `Gate2_LiveMini_v1.0.0_win-x64.zip`, `11,927,370` bytes, SHA-256 `a8d0bd4925bf1caf643b4e676f4fc12b918d6d1acf20ff5698362c8365c920ec`):
- **Trạng thái Kết nối ADB Độc lập:** `#adb-mode-pill` hiển thị **`LIVE ADB: 2 ONLINE`** (cả trước và sau khi chạy). Xem [gate2_mini_before.png](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_mini_before.png) và [gate2_mini_after.png](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_mini_after.png).
- **Chạy Nhiệm vụ Qua Nút `CHẠY NHIỆM VỤ (ADB THẬT)`:**
  - Không có bất kỳ nhãn `TESTKIT · Mô phỏng` nào (`anyTestkitBadge = false`).
  - Các biến cập nhật realtime với provenance thực:
    - `ocr_matched = true` (`LIVE_ADB_OCR`, `verified = true`, `confidence = 0.94`)
    - `ocr_screen_label = "10:29 bearch settings No recent search results"` (`LIVE_ADB_OCR`, `verified = true`)
    - `mini_live_result = "SEARCH_SETTINGS_OPENED"` (`LIVE_STATE`, `verified = true`, `seq = 2`)
    - `last_data_source = "LIVE_ADB(127.0.0.1:5555)"`, `last_task_state = "Succeeded"`, `task_completed_count = 1` (`LIVE_STATE`, `verified = true`, `seq = 3`)
  - SQLite độc lập của Mini Tool (`runtime_live-gate2-android.db`) ghi nhận job `mini-job-d6c2c9ab-6b07-4170-9429-629819cfd0be` ở trạng thái `Succeeded` với `22` `step_events`.
- **Ảnh Trước / Sau Trên Thiết bị Android (`emulator-5554`):**
  - Studio: [gate2_device_before_studio.png](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_device_before_studio.png) (`59,537` B) → [gate2_device_after_studio.png](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_device_after_studio.png) (`50,358` B)
  - Mini Tool: [gate2_device_before_mini.png](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_device_before_mini.png) (`25,680` B) → [gate2_device_after_mini.png](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_device_after_mini.png) (`25,776` B, con trỏ focus vào ô Search settings sau `random_safe_tap`)
