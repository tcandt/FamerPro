# FARMERPRO — GATE 2 ACCEPTANCE ADDENDUM (LIVE ANDROID ADB, OCR PROVENANCE & REGRESSION FREEZE)

**Active Branch:** `feat/p6-final-validation`  
**Final Frozen RC Code Baseline:** `629cce236e173f91642684858e44638948773b9e` (Git Tag: `v0.1.0-p6-rc.3`)  
**Previous Checkpoints:** `802f316` (`v0.1.0-p6-rc.1`, pre-Gate-2 `BLOCKED (0 ADB)`) → `a149e8e` (initial Gate 2 evidence, 0 code diff) → `b5a0567` (`v0.1.0-p6-rc.2`, canonical dedup & `adb reverse` fix) → `3b05d17` (RC.2 docs)  
**Overall Development Decision:** **P6 Closeout + Gate 2 Platform/Settings Live ADB = PASS**  
**Overall Production Decision:** **NOT APPROVED (`PRODUCTION_ACCEPTANCE_PASS = BLOCKED` at Gate 1 & Gate 3)**

---

## I. Chốt Baseline RC.3 & Chuỗi Truy xuất (`RC.1 → RC.2 → RC.3`)

1. **`v0.1.0-p6-rc.1` (`802f316`):** Baseline P6 Closeout cũ trước Gate 2 (PDF `708,938` B, SHA-256 `87d46378b1d1d915182d0ef4bd8808e66f434fc99bd548997333cd43eb8e67c6`, ghi nhận `BLOCKED (0 ADB)` khi `LDPlayer` dừng dưới WHv `VBox.log: HM -> NEM fallback / Heartbeat flatlined`). Không còn là baseline phát hành mới.
2. **`a149e8e`:** Thêm báo cáo & bằng chứng Gate 2 ban đầu trên Google Android SDK AVD (`fp_gate2_avd`), `0` thay đổi mã nguồn (`crates/`, `apps/`, `Cargo.toml`, `Cargo.lock` trống).
3. **`v0.1.0-p6-rc.2` (`b5a0567`) + `3b05d17`:** Vá 2 lỗi thực tế phát hiện từ Gate 2 (trùng 2 transport `emulator-5554`/`127.0.0.1:5555` trên cùng AVD và parser `adb reverse --list` đảo cột/`host-22`).
4. **`v0.1.0-p6-rc.3` (`629cce2` — Final Frozen Code Baseline):** Hoàn thiện chỉ đạo Gate 2 Closeout:
   - Tách bạch **OCR Provenance** (`ConfidenceSource`, `measuredConfidence`, `ruleMatched`, `deviceId`) và bỏ hiển thị `LIVE ✓ 94%` giả tạo trên UI Mini Tool (thay bằng **`LIVE · Đã khớp quy tắc`** / **`LIVE · Chưa xác minh`**).
   - Mở rộng bộ **regression tests Canonical Device Identity (7 kịch bản)**, đồng bộ scheduler canonical lease vào `apps/mini-runtime`, và bổ sung **regression tests `adb forward`/`reverse`** (parser `reverse`, `host-22` isolation, empty output, session cleanup).
   - Toàn bộ `101 / 101` tests (`38` suites), release binaries, NSIS Setup, Portable ZIPs, `verify_p6.js`, `verify_gate2_live_adb.js`, PDF báo cáo và SHA-256 manifests đã được tạo lại và đối chiếu khớp với đúng `629cce2`.

---

## II. Hoàn thiện OCR Provenance & UI Mini Runtime (`LIVE · Đã khớp quy tắc`)

- **Schema & Engine ([crates/fp-domain/src/lib.rs](file:///e:/DATA/FarmerPro/crates/fp-domain/src/lib.rs), [crates/fp-workflow-runtime/src/lib.rs](file:///e:/DATA/FarmerPro/crates/fp-workflow-runtime/src/lib.rs), [crates/fp-storage/src/lib.rs](file:///e:/DATA/FarmerPro/crates/fp-storage/src/lib.rs)):**
  - Bổ sung `ConfidenceSource` (`MEASURED` | `HEURISTIC` | `SIMULATED` | `UNCALIBRATED`) và các trường `confidenceSource`, `measuredConfidence`, `ruleMatched`, `deviceId` trên `RuntimeVariableEnvelope` và SQLite `runtime_variable_envelopes` (tương thích ngược schema v5).
  - Với `Windows.Media.Ocr` (không có native per-word float confidence), engine gán `confidenceSource = "HEURISTIC"`, `measuredConfidence = null`, `confidence = null` (chỉ dùng điểm nội bộ so ngưỡng `minConfidence`), tách riêng `ocr_text` (raw OCR), `ocr_expected_text` (`"Search"`), `ruleMatched` (`true`/`false`), `verified` (`true`/`false`), `deviceId` (`"emulator-5554"`), `sessionId` và `updatedAtUtc`.
- **Giao diện Mini Runtime ([apps/mini-runtime/ui/index.html](file:///e:/DATA/FarmerPro/apps/mini-runtime/ui/index.html)):**
  - `provenanceBadgeHtml(env)` **chỉ** hiển thị phần trăm `%` khi `env.confidenceSource === 'MEASURED' && typeof env.measuredConfidence === 'number'`. Khi chưa có metric đo hiệu chuẩn, badge hiển thị **`LIVE · Đã khớp quy tắc`** (`verified=true`) hoặc **`LIVE · Chưa xác minh`** (`verified=false`).
  - Xác minh thực tế trên `Auto_Gate2Live.exe` (`629cce2`): `anyUnmeasuredPercentBadge = false` (không còn `94%` hay `LIVE ✓`), `ruleMatchBadgePresent = true`, ảnh chụp [gate2_mini_after.png](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_mini_after.png).
  - Raw OCR thu thật trên `emulator-5554`: `"12:14 Q Search Settings G Google Services & preferences Network & internet Mobile, Wi-Fi, hotspot Connected devices Bluetooth, pairing Apps Assistant, recent apps, default apps"` (`LIVE_ADB_OCR`, `confidenceSource = "HEURISTIC"`, `measuredConfidence = null`, `ruleMatched = true`, `verified = true`).

---

## III. Canonical Device Identity & Regression Tests (7 Kịch bản)

1. **Hợp nhất Discovery, Scheduler & Mini Runtime:**
   - [crates/fp-adb/src/lib.rs](file:///e:/DATA/FarmerPro/crates/fp-adb/src/lib.rs), [crates/fp-scheduler/src/lib.rs](file:///e:/DATA/FarmerPro/crates/fp-scheduler/src/lib.rs), [crates/fp-workflow-runtime/src/coordinator.rs](file:///e:/DATA/FarmerPro/crates/fp-workflow-runtime/src/coordinator.rs), [apps/desktop/src-tauri/src/main.rs](file:///e:/DATA/FarmerPro/apps/desktop/src-tauri/src/main.rs), và [apps/mini-runtime/src/main.rs](file:///e:/DATA/FarmerPro/apps/mini-runtime/src/main.rs) dùng chung chính sách canonical identity + `MultiDeviceScheduler::acquire_device_lease_canonical`.
   - `MultiDeviceScheduler` bổ sung `release_leases_for_disconnected_device` và `reconcile_device_reboot` (khi `boot_id` đổi sau reboot, tự động giải phóng lease cũ của phiên trước khi reboot và cập nhật canonical identity mới; `release_device_lease` có fallback theo `(device_id, job_id)`).
2. **Đủ 7 ca hồi quy được phủ trong `cargo test --workspace` (`101 / 101 PASS`):**
   - (1) Hai serial trỏ cùng 1 Android (`emulator-5554` + `127.0.0.1:5555`); (2) Hai thiết bị thật khác nhau (cùng pattern hardware serial nhưng khác `boot_id`); (3) Một thiết bị nối đồng thời USB + TCP (`USB-PIXEL-9` + `192.168.1.44:5555`); (4) Thiết bị reboot đổi `boot_id` giải phóng lease cũ; (5) Thiết bị disconnect khi đang giữ lease; (6) Hai job tranh chấp 1 thiết bị vật lý qua 2 transport khác nhau bị từ chối; (7) Unknown identity không tự hợp nhất nhầm (`192.168.1.80:5555` và `192.168.1.81:5555` cùng model nhưng thiếu `boot_id` vẫn giữ 2 thiết bị riêng).
- **Xác minh Runtime thật trên `fp_gate2_avd`:** Cố tình `adb connect 127.0.0.1:5555` (`identicalInstanceConfirmed = true` theo `ro.boot.serialno=EMULATOR37X1X11X0` và `boot_id`) → Studio `adbDiscoveredCount = 1` (`emulator-5554`), Mini Tool **`LIVE ADB: 1 ONLINE`**.

---

## IV. Chốt ADB Forward / Reverse trên `emulator-5554`

- **Live Test Create → List → Remove:** `studio_add_device_forward` (`tcp:19555 -> tcp:19556`), `studio_add_device_reverse` (`tcp:19557 -> tcp:19558`, `serial: emulator-5554`), `studio_list_device_ports` → xóa sạch về `forwards: [], reverses: []`.
- **Regression Tests & Cleanup (`crates/fp-adb/src/lib.rs`):** Kiểm thử parser `reverse --list` (`host-22 tcp:19557 tcp:19558`), thay `host-22` thành target `serial` trong `reverse_list` (không bao giờ coi `host-22` là Android device serial), output rỗng `""`, và bổ sung `forward_remove_all_for_serial` / `reverse_remove_all` cho cleanup cuối phiên.

---

## V. Phạm vi Kiểm chứng Gate 2 (Tách bạch)

| Tầng | Trạng thái | Ghi chú |
|---|---|---|
| **Platform Live ADB Integration** | **PASS** | Canonical dedup (`LIVE ADB: 1 ONLINE`), Lease exclusion, Forward/Reverse, Capture/Template/OCR Provenance, Checkpoint/Pause/Resume/Cancel, Mini Tool độc lập (`22` events, `anyTestkitBadge=false`). |
| **Live Workflow — Android Settings (`com.android.settings`)** | **PASS** | Ảnh trước/sau Studio ([before](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_device_before_studio.png) `59,064` B → [after](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_device_after_studio.png) `61,447` B) và Mini Tool ([before](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_device_before_mini.png) `58,919` B → [after](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_device_after_mini.png) `49,826` B). |
| **Live Game-Specific Workflow (JX2 Mobile / Game thật)** | **NOT TESTED** | Nghiệm thu riêng theo từng hồ sơ GameProject; không gộp với Gate 2 nền tảng. |

---

## VI. Checksums Phát hành Chính thức (`629cce2` / Tag `v0.1.0-p6-rc.3`)

- **NSIS Setup (`FarmerPro_0.1.0_x64-setup.exe`):** `15,268,397` B · `ab36da44ccdd50f33994bf9fc88f2b81635e5747bdd2dcc154bcc6bc4b57d7f1`
- **Portable Alpha ZIP (`P6_Alpha_v1.0.0_win-x64.zip`):** `11,971,510` B · `e94279949710eca5baf0bd0ced6bd58fd92227fd2b8af6df60313bc8021b8339`
- **Portable Beta ZIP (`P6_Beta_v1.0.0_win-x64.zip`):** `11,973,272` B · `99512f43ddc10ab8cdcc55171929e230042a58b5a831f4b30e82c1bbacb24771`
- **Gate 2 Portable Mini ZIP (`Gate2_LiveMini_v1.0.0_win-x64.zip`):** `11,987,054` B · `2de448a5e263cbbc84b5aa16acfa4a13ce0e78f0c989d8150f3d5cfcfb1aaa73`
- **PDF Report ([FARMERPRO_P6_ACCEPTANCE_REPORT.pdf](file:///e:/DATA/FarmerPro/FARMERPRO_P6_ACCEPTANCE_REPORT.pdf)):** `712,963` B · `ed6e66a930df0b730c56724a46d70555337bf5fff9457c68c9503b48c16c8836` (`QA passed = true`, `64` manifest files)

---

## VII. Hai Gate Hạ tầng Ngoài Còn Lại (`BLOCKED`)

- **Gate 1 — Clean Windows VM:** **`BLOCKED`** (`WindowsSandbox.exe = False`, Hyper-V cần quyền Administrator).
- **Gate 3 — Commercial Authenticode PublicSigned:** **`BLOCKED`** (`0` Commercial CA code-signing certs trong `Cert:\CurrentUser\My`).
- **`PRODUCTION_ACCEPTANCE_PASS`:** **`BLOCKED` / `NOT APPROVED`**.
