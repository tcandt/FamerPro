# FARMERPRO RC.3 — BÁO CÁO HAI BÀI KIỂM CHỨNG BỔ SUNG (CROSS-PROCESS LEASE & SQLITE V5 UPGRADE)

**Verified Code Baseline (Untouched):** `629cce236e173f91642684858e44638948773b9e` (`v0.1.0-p6-rc.3`)  
**Verification Script:** [scripts/verify_rc3_closeout_supplement.js](file:///e:/DATA/FarmerPro/scripts/verify_rc3_closeout_supplement.js)  
**Raw Evidence JSON:** [rc3_supplement_verification.json](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/rc3_supplement_evidence/rc3_supplement_verification.json)

---

## I. Test A — Cross-Process Device Lease (`CROSS_PROCESS_LEASE = NOT VERIFIED`)

**Phương pháp thực nghiệm trên binary đóng băng RC.3 (`629cce2`):**
Cài `FarmerPro_0.1.0_x64-setup.exe` trong sandbox, mở đồng thời `FarmerPro.exe` (Studio) và `Auto_CrossMini.exe` (Mini Tool độc lập) cùng trỏ `emulator-5554` (`fp_gate2_avd`).

1. **Same-Process Exclusion (Cùng tiến trình Studio):**
   - Studio chạy `Job A` (`job-65c0c282-0193-4ab0-8d96-e46cce0b103e`, wait `4200ms`) trên `emulator-5554` (`state = "running"`, `activeLeasesCount = 1`).
   - Thử khởi tạo job thứ hai trên `emulator-5554` ngay trong cùng tiến trình Studio -> **BỊ CHẶN**: `Device 'emulator-5554' (canonical 'local-emu:5554') is currently leased by job 'job-65c0c282-...'`.
2. **Cross-Process Exclusion (Hai EXE riêng biệt):**
   - Trong lúc `Job A` của `FarmerPro.exe` vẫn đang `running`, kích hoạt `Job B` (`mini-job-0847c536-9215-4eac-a802-7a4520b42538`) từ `Auto_CrossMini.exe` vào `2026-11-23T05:39:08.063Z` trên cùng `emulator-5554`.
   - **Kết quả quan sát thực tế:** `Job B` **KHÔNG bị chặn** (`blocked = false`, `state = "Succeeded"`, `driverUsed = "LIVE_ADB(emulator-5554)"`), trong khi `Job A` của Studio ngay sau đó vẫn đang `running` (`leasesCount = 1`) và chỉ kết thúc `succeeded` về sau (`leasesAfterJobADoneCount = 0`).
   - **Nguyên nhân kỹ thuật:** `MultiDeviceScheduler` giữ `device_leases: Arc<Mutex<HashMap<String, DeviceLease>>>` thuần trong RAM nội tiến trình, và Studio (`AppData/Local/FarmerPro/StudioData/data/farmerpro.db`) dùng SQLite DB tách biệt với Mini Tool (`<release>/data/runtime_<slug>.db`), không có OS mutex hoặc advisory lockfile liên tiến trình.
3. **Forced-Kill Recovery:**
   - Tạo `Job C` (`job-88bd5793-8675-419f-aed9-6eff5a954d00`) trong Studio rồi `taskkill /PID 8188 /F`. Mở lại `FarmerPro.exe` mới: `leasesOnRestart = []`, chạy `Job D` trên `emulator-5554` đạt `succeeded`.
4. **Kết luận & Đề xuất Khắc phục Tối thiểu:**
   - **Trạng thái:** **`CROSS_PROCESS_LEASE = NOT VERIFIED (IN_PROCESS_ONLY)`**.
   - **Hai thiết bị khác nhau:** Trên host chỉ có 1 AVD đang chạy; khả năng chạy đồng thời 2 thiết bị khác nhau hiện được bảo chứng bởi unit/coordinator tests trong workspace (`test_concurrent_two_devices_isolation_and_same_device_lease_conflict`, `test_canonical_device_identity_lease_exclusion_across_transports`).
   - **Cách khắc phục tối thiểu:** Bổ sung OS-level advisory lockfile theo canonical device key tại `%LOCALAPPDATA%\FarmerPro\locks\dev_<canon_sha256>.lock` (ghi `{pid, exe, job_id, lease_expires_at}`) dùng Win32 `LockFileEx` trong `fp-scheduler`, tự nhả khi drop và thu hồi an toàn khi `lease_expires_at` hết hạn hoặc PID giữ lock đã chết sau forced-kill.

---

## II. Test B — SQLite v5 Upgrade Compatibility (`FAIL` Trên RC.3 `629cce2`)

**Phương pháp thực nghiệm trên binary đóng băng RC.3 (`629cce2`):**
Tạo bản sao SQLite v5 chuẩn theo đúng DDL của **RC.2 (`b5a0567`)** (có `schema_migrations = [1, 2, 3, 4, 5]`, có dữ liệu `publisher_profiles`, `projects`, `workflows`, `workflow_versions`, `task_definitions`, `project_releases`, `accounts`, `runtime_var_streams`, `runtime_variable_envelopes`, và **chưa có** 4 cột provenance RC.3). Mở bản sao này **2 lần** bằng `FarmerPro.exe` RC.3 (`629cce2`) với `FARMERPRO_DATA_DIR` trỏ vào sandbox.

1. **Kết quả Bảo toàn Dữ liệu & Kiểm tra Toàn vẹn:**
   - Mọi bảng hiện có đều đọc lại được đầy đủ sau nâng cấp (`projects=1`, `workflows=1`, `workflow_versions=1`, `task_definitions=1`, `project_releases=1`, `accounts=1` với `secret_ref='dpapi_ref_rc2_token'`, `publisher_profiles=1`, `legacy_ocr_val` trong `runtime_variable_envelopes` còn nguyên giá trị `"Settings"`, `seq=7`).
   - `PRAGMA integrity_check` = `["ok"]`, `PRAGMA foreign_key_check` = `[]` (`0` lỗi).
   - Thử mô phỏng lỗi migration trên bản sao riêng và khôi phục từ backup `rc2_studio_backup.db` khớp chính xác SHA-256 (`731379fb247f512824816fad70cef42060d2fda46562c35efe7b5a2c37d9fc31`).
2. **Lỗi Thực tế Bắt được Khi Mở DB RC.2 bằng RC.3:**
   - Sau 2 lần mở bằng `FarmerPro.exe` RC.3 (`629cce2`), `PRAGMA table_info(runtime_variable_envelopes)` **KHÔNG có** 4 cột mới (`addedProvenanceColsPresent = false`).
   - Lệnh Tauri IPC `studio_get_project_runtime_vars` trả về lỗi thật:
     `no such column: confidence_source in SELECT project_id, session_id, job_id, var_name, value_type, value_json, source, verified, confidence, confidence_source, measured_confidence, rule_matched, device_id, updated_at_utc, workflow_version, seq, redacted FROM runtime_variable_envelopes WHERE scope_key = ?1 ORDER BY var_name ASC;`
   - **Nguyên nhân gốc tại [crates/fp-storage/src/migrations.rs](file:///e:/DATA/FarmerPro/crates/fp-storage/src/migrations.rs#L551-L559):** `apply_migrations` có fast-path `if applied.contains(&1) && ... && applied.contains(&5) { return list_applied_versions(conn); }` ở đầu hàm, **trả về sớm trước khi chạy** 4 lệnh `ALTER TABLE runtime_variable_envelopes ADD COLUMN ...` ở dòng `837-848`. Do RC.3 không tăng phiên bản migration lên `v6`, mọi DB đã tồn tại từ RC.2/RC.1/P6 đều bị bỏ qua phần thêm cột!
3. **Kết luận & Đề xuất Sửa Tối thiểu:**
   - **Trạng thái trên `629cce2` (`v0.1.0-p6-rc.3`):** **`SQLITE_V5_UPGRADE_COMPATIBILITY = FAIL`**.
   - **Đề xuất vá tối thiểu (nếu Auditor phê duyệt mở hotfix RC.4 trước Gate 1):** Chuyển 4 `ALTER TABLE runtime_variable_envelopes` vào `MIGRATION_V6` nằm trong `IMMEDIATE` transaction (tăng `CURRENT_SQLITE_SCHEMA_VERSION = 6` và cập nhật fast-path kiểm tra `applied.contains(&6)`), kèm unit test khởi tạo DB schema v5 (không có 4 cột) rồi mở bằng `StorageManager::new` v6 để xác nhận nâng cấp idempotent + không lỗi truy vấn.
