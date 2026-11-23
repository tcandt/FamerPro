# FARMERPRO — GATE 2 ACCEPTANCE ADDENDUM (LIVE ANDROID ADB & CANONICAL IDENTITY HOTFIX)

**Active Branch:** `feat/p6-final-validation`  
**Verified RC Code Baseline (Updated After Gate 2 Defect Fix):** `b5a05673b1c0ad10dd4c6ce4463eaabbd3be40fe` (Git Tag: `v0.1.0-p6-rc.2`)  
**Previous Frozen RC Baseline:** `802f3162df7fe3ab8fd6c7085c9d9e34caca6648` (Git Tag: `v0.1.0-p6-rc.1`)  
**Overall Development Decision:** **P6 Closeout + Gate 2 Platform/Settings Live ADB = PASS**  
**Overall Production Decision:** **NOT APPROVED (`PRODUCTION_ACCEPTANCE_PASS = BLOCKED` at Gate 1 & Gate 3)**

---

## I. Đồng bộ PDF P6 Pre-Gate-2 & Addendum Gate 2

1. **Vì sao bản PDF P6 ban đầu (`87d46378...`) ghi `BLOCKED (0 ADB)`:**
   - Báo cáo `FARMERPRO_P6_ACCEPTANCE_REPORT.pdf` tại baseline `802f3162df7fe3ab8fd6c7085c9d9e34caca6648` (`v0.1.0-p6-rc.1`, `708,938` bytes, SHA-256 `87d46378b1d1d915182d0ef4bd8808e66f434fc99bd548997333cd43eb8e67c6`) phản ánh đúng trạng thái trước Gate 2 khi `LDPlayer14` / `LDPlayer9` VM tự tắt dưới Windows Hypervisor Platform (`VBox.log`: `HM: HMR3Init: Attempting fall back to NEM: VT-x is not available` → `Heartbeat flatlined` → `PDMR3PowerOff` / `VERR_INTERRUPTED`), dẫn tới `0` thiết bị ADB kết nối tại thời điểm nghiệm thu P6.
   - Commit `a149e8e67d099233841b9a40cd161b3d49b57670` chỉ bổ sung tài liệu và bằng chứng Gate 2 ban đầu trên Google Android SDK AVD (`fp_gate2_avd`), **0 thay đổi mã nguồn** (`git diff --stat 802f316..a149e8e -- crates apps Cargo.toml Cargo.lock` = trống).
2. **Tại sao phát sinh Hotfix RC.2 (`b5a0567` / `v0.1.0-p6-rc.2`) & Đồng bộ Tài liệu:**
   - Trong quá trình kiểm chứng mục II và IV của Gate 2 trên thiết bị thật `fp_gate2_avd`, log và test thực tế đã chứng minh **2 lỗi thật ở tầng platform**:
     1. Cùng một AVD duy nhất `fp_gate2_avd` xuất hiện đồng thời dưới 2 transport `emulator-5554` và `127.0.0.1:5555`, làm Discovery/UI đếm thành **`LIVE ADB: 2 ONLINE`** và `MultiDeviceScheduler` không khóa xung đột lease giữa hai transport của cùng một thiết bị.
     2. `adb reverse --list` trên thiết bị thật trả về thứ tự cột `<host_transport> <remote_device> <local_host>` (`host-22 tcp:19557 tcp:19558`), khác với `adb forward --list` (`<serial> <local_host> <remote_device>`), làm `AdbRunner::parse_port_mappings` đảo ngược `local`/`remote` và giữ token nội bộ `host-22` thay vì `serial` thật khi UI gọi `studio_remove_device_reverse`.
   - Sau khi vá tối thiểu 2 lỗi trên tại commit `b5a0567` (`v0.1.0-p6-rc.2`), toàn bộ bộ test Rust (`101/101` PASS), release binaries (`fp-updater.exe`, `fp-mini-runtime.exe`, NSIS `FarmerPro_0.1.0_x64-setup.exe`), E2E P6 (`scripts/verify_p6.js`), Gate 2 Live ADB (`scripts/verify_gate2_live_adb.js`), PDF báo cáo ([FARMERPRO_P6_ACCEPTANCE_REPORT.pdf](file:///e:/DATA/FarmerPro/FARMERPRO_P6_ACCEPTANCE_REPORT.pdf), `712,030` bytes, SHA-256 `79821bec5b38028c3a86dc2cadb193a1d99358c061d912d0ddf6fa884e986651`) và `EVIDENCE_MANIFEST_SHA256.json` (`64` files) đã được tạo lại đồng bộ từ chính `b5a0567`.

---

## II. Bằng chứng Danh tính Thiết bị Live ADB (`LIVE ADB: 1 ONLINE` — Không Trùng 2 Transport)

1. **Bằng chứng thô trên `fp_gate2_avd` (`gate2_live_adb_report.json` → `canonicalDeviceIdentityProof`):**
   - Trước khi gắn loopback: `adb devices -l` chỉ có `emulator-5554 device product:sdk_gphone16k_x86_64 model:sdk_gphone16k_x86_64 device:emu64xa16k transport_id:1`.
   - Khi cố tình kết nối `adb connect 127.0.0.1:5555` trước khi khởi chạy Studio, `adb devices -l` thô liệt kê **2 dòng**:
     - `127.0.0.1:5555 device ... device:emu64xa16k transport_id:2`
     - `emulator-5554 device ... device:emu64xa16k transport_id:1`
   - Kiểm tra trực tiếp kernel & build props trên cả hai serial xác nhận **cùng một thực thể AVD đang chạy**:
     - `ro.boot.serialno` = `EMULATOR37X1X11X0`
     - `emulator-5554` `/proc/sys/kernel/random/boot_id` = `5ddf4574-9b3f-450f-9550-b39f4c1a95bf`
     - `127.0.0.1:5555` `/proc/sys/kernel/random/boot_id` = `5ddf4574-9b3f-450f-9550-b39f4c1a95bf` (`identicalInstanceConfirmed = true`)
2. **Cách khắc phục đã áp dụng & kiểm thử:**
   - [crates/fp-adb/src/lib.rs](file:///e:/DATA/FarmerPro/crates/fp-adb/src/lib.rs): Bổ sung `infer_local_emulator_canonical_key`, `serial_transport_priority`, `coalesce_device_summaries`, `list_devices_raw`, `probe_canonical_device_identity` (`ro.boot.serialno` + `/proc/sys/kernel/random/boot_id`) và hợp nhất kết quả trong `AdbRunner::list_devices` (ưu tiên `Ready` và transport chuẩn: USB < `emulator-<n>` < LAN TCP < loopback TCP).
   - [apps/desktop/src-tauri/src/main.rs](file:///e:/DATA/FarmerPro/apps/desktop/src-tauri/src/main.rs): `adb_scan_range` và `auto_discover_devices` loại trừ cổng loopback `even + 1` khi `emulator-<even>` đã gắn.
   - [crates/fp-scheduler/src/lib.rs](file:///e:/DATA/FarmerPro/crates/fp-scheduler/src/lib.rs) & [crates/fp-workflow-runtime/src/coordinator.rs](file:///e:/DATA/FarmerPro/crates/fp-workflow-runtime/src/coordinator.rs): Bổ sung `canonical_device_lease_key`, `acquire_device_lease_canonical`, và kiểm tra lease theo canonical identity trên `MultiDeviceScheduler` + `RuntimeCoordinator` (chống 2 job chiếm đồng thời 1 thiết bị qua 2 serial khác nhau).
   - **Unit test hồi quy mới (PASS):** `test_coalesce_duplicate_emulator_and_multitransport_identities` (`fp-adb`), `test_canonical_device_identity_lease_exclusion_across_transports` (`fp-scheduler`), nâng tổng số test workspace lên **`101 / 101 PASS`** ([cargo_test_workspace_p6.log](file:///e:/DATA/FarmerPro/artifacts/p6_evidence/cargo_test_workspace_p6.log)).
   - **Xác minh Runtime thật sau hotfix:** Ngay cả khi `127.0.0.1:5555` đã được gắn sẵn cạnh `emulator-5554`, `auto_discover_devices` và `list_devices` trả về **`adbDiscoveredCount = 1` (`listedDevices = ["emulator-5554"]`)**, Mini Runtime `#adb-mode-pill` hiển thị chính xác **`LIVE ADB: 1 ONLINE`** và chỉ render 1 dòng thiết bị (`LIVE_ADB(emulator-5554)`). Xem [gate2_studio_device_lab.png](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_studio_device_lab.png) và [gate2_mini_before.png](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_mini_before.png).

---

## III. Bằng chứng Trực tiếp Vision/OCR, Safe Tap, Biến Runtime, Checkpoint & Provenance

1. **Template Matching & Safe Random Tap (Studio):**
   - `capture_device` chụp ảnh PNG thật từ `emulator-5554` (`78,728` bytes), `studio_capture_and_crop_template` cắt vùng nút `Search Settings` (`260×56` tại `x=50, y=76`, SHA-256 `1574e3529a704ab21a76797bca61cdf94697cadf89a0e89bb75618671acad19b`).
   - `studio_test_project_template_match` khớp với **`matched = true`**, **`confidence = 0.93104136`**, `bounds = {x: 54, y: 78, width: 221, height: 48}`, `suggestedTapPoint = {x: 128.63554, y: 99.694275}`.
2. **OCR Thật & Quy tắc `verified = true`:**
   - Thực thi qua `Windows.Media.Ocr.OcrEngine` ([crates/fp-workflow-runtime/src/lib.rs](file:///e:/DATA/FarmerPro/crates/fp-workflow-runtime/src/lib.rs#L780-L830)):
     - Công thức kiểm chứng chính xác: `verified_ok = ctx.runtime_source_mode.is_live() && matched && type_valid && ocr_conf >= min_conf`.
     - Ghi nhận trung thực kỹ thuật: UWP `Windows.Media.Ocr` trả về text/từ theo bounding box mà không cung cấp float confidence gốc từng từ, nên engine gán điểm xác định `ocr_conf = 0.94` khi text không rỗng và khớp `expectedText` (`"Search"`), `0.72` khi không khớp `expectedText`, và `0.40` khi rỗng.
   - **Văn bản OCR thu được trực tiếp từ màn hình `emulator-5554` (`minConfidence = 0.70`, `expectedText = "Search"`):**
     - **Studio Job 1 (`n3`):** `"11:26 Q Search Settings G Google Services & preferences Network & internet Mobile, Wi-Fi, hotspot Connected devices Bluetooth, pairing Apps Assistant, recent apps, default apps"` → `ocr_matched = true`, `confidence = 0.94`, `source = "LIVE_ADB_OCR"`, `verified = true`.
     - **Mini Tool (`Auto_Gate2Live.exe`, Studio đã tắt & gỡ):** `"11:27 Q Search Settings G Google Services & preferences Network & internet Mobile, Wi-Fi, hotspot Connected devices Bluetooth, pairing Apps Assistant, recent apps, default apps"` → `ocr_matched = true`, `confidence = 0.94`, `source = "LIVE_ADB_OCR"`, `verified = true`.
3. **Vòng đời Job, Điều khiển Coordinator, SQLite & Ảnh Trước/Sau:**
   - **Studio Job 1 (`job-19f46ff3-b257-4122-8716-991742e37c26`):** `Pause` giữa luồng (`job1PausedState = "paused"`) → `Resume` → `vision.capture` → `vision.ocr` → `vision.find_image` → `action.random_safe_tap` (`marginRatio=0.15`) → `action.swipe` (`640,500 -> 640,260`) → `state.checkpoint` → `state.set_var("studio_live_status", "STUDIO_LIVE_OK")` → `state = "succeeded"`, **`24` `step_events`** ([gate2_studio_runtime_vars.png](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_studio_runtime_vars.png)). Thiết bị chuyển từ Settings Home ([gate2_device_before_studio.png](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_device_before_studio.png), `59,095` B) sang Search settings + bàn phím ([gate2_device_after_studio.png](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_device_after_studio.png), `50,040` B).
   - **Studio Job 2 (`job-51284b9e-7825-4b73-ae1c-aaa2f422cde5`):** `studio_suspend_job_to_checkpoint` → `jobState = "paused"`, `checkpointId = "cp-854fc17a-62fb-4bbe-8d2e-902f21108ff1"` (`hasCheckpoint = true`) → `studio_resume_job_from_checkpoint` khôi phục từ `resumedFromNodeId = "c3"` (`fencingGeneration = 3`) → `state = "succeeded"`, `eventsRecorded = 13`, `checkpoint_resumed = "SETTINGS_HOME_RESTORED"`.
   - **Studio Job 3:** `studio_cancel_job` → `job3FinalState = "cancelled"`.
   - **Mini Tool Độc lập (`Auto_Gate2Live.exe` sau `uninstall.exe /S`):** Thực thi qua nút `CHẠY NHIỆM VỤ (ADB THẬT)`, `anyTestkitBadge = false`, không có `FakeDeviceDriver`, SQLite `runtime_live-gate2-android.db` ghi job `mini-job-f773ccb7-e695-4c4d-928a-498a317dd58f` trên `emulator-5554` (`Succeeded`), **`22` `step_events`**, `9` biến trong `runtime_variable_envelopes` với `verified = 1`, `source ∈ {LIVE_ADB_OCR, LIVE_STATE}`. Thiết bị chuyển từ Settings Home ([gate2_device_before_mini.png](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_device_before_mini.png), `58,886` B) sang Search settings ([gate2_device_after_mini.png](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_device_after_mini.png), `25,119` B), UI Mini Tool cập nhật từ [gate2_mini_before.png](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_mini_before.png) sang [gate2_mini_after.png](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_mini_after.png).

---

## IV. Kiểm thử Thật `adb forward` & `adb reverse` trên `emulator-5554`

Thực hiện đầy đủ **Create → List → Remove** qua Tauri IPC của Studio trên `emulator-5554`:
- `studio_add_device_forward`: tạo `tcp:19555 -> tcp:19556` (`serial: emulator-5554`).
- `studio_add_device_reverse`: tạo `tcp:19557 -> tcp:19558` (`serial: emulator-5554`).
- `studio_list_device_ports` sau tạo (`portsAfterCreate`): xác nhận có mặt đủ `forwards: [{local: "tcp:19555", remote: "tcp:19556"}]` và `reverses: [{serial: "emulator-5554", local: "tcp:19558", remote: "tcp:19557"}]`.
- `studio_remove_device_forward` & `studio_remove_device_reverse` dùng chính `serial` & `local`/`remote` trả về từ danh sách (như UI thực tế gọi): xóa sạch về `portsAfterRemove = {forwards: [], reverses: []}`.
- **Lỗi đã phát hiện và vá trong Gate 2:** `adb reverse --list` trả về `host-22 tcp:19557 tcp:19558` (đảo cột remote/local và thay `serial` bằng `host-XX`). Đã sửa trong [crates/fp-adb/src/lib.rs](file:///e:/DATA/FarmerPro/crates/fp-adb/src/lib.rs) kèm unit test `test_parse_forward_reverse_and_validate_specs`.

---

## V. Ranh giới Phạm vi Kiểm chứng Gate 2 (Tách bạch 3 Tầng)

| Tầng Phạm vi | Trạng thái | Ghi chú Trung thực |
|---|---|---|
| **1. Live Android ADB Platform Integration** | **PASS** | Discovery canonical deduplication (`LIVE ADB: 1 ONLINE`), `capture_device`, template crop/match, `Pause/Resume`, `Suspend/Resume Checkpoint`, `Cancel`, `adb forward`/`reverse`, và `Mini Tool` độc lập chạy `LIVE_ADB(emulator-5554)` không có Studio/TestKit. |
| **2. Live Workflow Execution (Android System UI — `com.android.settings`)** | **PASS** | Đã kiểm chứng trên Android SDK AVD `fp_gate2_avd` (API 37, WHPX) với nút Search Settings, OCR, Safe Random Tap, Swipe, Key `BACK` và khôi phục Home. |
| **3. Live Game-Specific Workflow Execution (JX2 Mobile / SLG / Game thật)** | **NOT TESTED / PENDING** | Chưa cài hoặc chạy quy trình trên APK game cụ thể nào; không tuyên bố đã nghiệm thu automation cho game cụ thể. |

---

## VI. Đối chiếu Release Candidate Provenance (`RC.1` → `a149e8e` → `RC.2`)

1. `802f3162df7fe3ab8fd6c7085c9d9e34caca6648` (`v0.1.0-p6-rc.1`): Code baseline P6 Closeout ban đầu (`99/99` tests, pre-Gate-2 PDF `708,938` B, SHA-256 `87d46378...`).
2. `a149e8e67d099233841b9a40cd161b3d49b57670`: Chỉ thêm Gate 2 report & bằng chứng trên `fp_gate2_avd`, **`0` thay đổi code** trong `crates/`, `apps/`, `Cargo.toml`, `Cargo.lock`.
3. `b5a05673b1c0ad10dd4c6ce4463eaabbd3be40fe` (`v0.1.0-p6-rc.2`): Hotfix tối thiểu từ 2 lỗi chứng minh được ở Gate 2 (`apps/desktop/src-tauri/src/main.rs`, `crates/fp-adb/src/lib.rs`, `crates/fp-scheduler/src/lib.rs`, `crates/fp-workflow-runtime/src/coordinator.rs`), **`101/101` Cargo workspace tests**, rebuild toàn bộ release & NSIS, chạy lại `verify_p6.js`, `verify_gate2_live_adb.js` và xuất lại báo cáo PDF:
   - **NSIS Setup (`FarmerPro_0.1.0_x64-setup.exe`):** `15,223,807` bytes · SHA-256 `4ffc3fc3d49d9b6f4b3ab82bf035a82dfb6e4411319bbec416c3412567d4d0fa`
   - **Portable Alpha ZIP (`P6_Alpha_v1.0.0_win-x64.zip`):** `11,929,011` bytes · SHA-256 `7b46389bed95c08678a24455d9b1e1c6215c5a61385508b8bddac35b4e9bce05`
   - **Portable Beta ZIP (`P6_Beta_v1.0.0_win-x64.zip`):** `11,930,781` bytes · SHA-256 `cf77b69ad5fcd5bf893f73ae0d4732a01957d31243b555af55ce065c1e6b1db2`
   - **Gate 2 Portable Mini ZIP (`Gate2_LiveMini_v1.0.0_win-x64.zip`):** `11,944,570` bytes · SHA-256 `ddb899b491df8697cc7fe817987e9a13ae2d1f2819da77568c46290afbae9424`
   - **PDF Report ([FARMERPRO_P6_ACCEPTANCE_REPORT.pdf](file:///e:/DATA/FarmerPro/FARMERPRO_P6_ACCEPTANCE_REPORT.pdf)):** `712,030` bytes · SHA-256 `79821bec5b38028c3a86dc2cadb193a1d99358c061d912d0ddf6fa884e986651` (`QA passed: true`, `64` manifest files).

---

## VII. Hai Gate Hạ tầng Ngoài Còn Lại (`BLOCKED`)

- **Gate 1 — Clean Windows VM:** **`BLOCKED`** (`WindowsSandbox.exe = False`, Hyper-V yêu cầu quyền Administrator; chưa test môi trường sạch không dev SDK / WebView2 missing).
- **Gate 3 — Commercial Authenticode PublicSigned:** **`BLOCKED`** (`0` Commercial CA code-signing certs trong `Cert:\CurrentUser\My`).
- **Quyết định Phát hành Thương mại (`PRODUCTION_ACCEPTANCE_PASS`):** **`BLOCKED`**.
