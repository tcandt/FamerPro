# FARMERPRO P2.1 — ROLLBACK RUNBOOK & PRODUCER KEY (DPAPI) SECURITY POLICY

## 1. Phân biệt 4 loại Database trong Workspace
| # | Đường dẫn | Vai trò | Schema | Số bản ghi thực tế |
|---|---|---|---|---|
| 1 | `E:\DATA\FarmerPro\farmerpro.db` | Root dev DB (mặc định khi chạy từ repo root không có `data/`) | **v1** (pre-tracking, `schema_migrations = []`) | `94,208` bytes; 9 bảng v1, **0 bản ghi** |
| 2 | `E:\DATA\FarmerPro\dist-portable\FarmerPro\data\farmerpro.db` | Runtime DB của bản đóng gói Portable (`<exe_dir>/data/farmerpro.db`) | **v2** (`schema_migrations = [1, 2]`) | `282,624` bytes; 15 bảng (`projects=1`, `workflows=11`, `workflow_versions=11`, `devices=2`, `jobs=2`, `step_events=31`, `vision_templates=8`, `task_definitions=3`, `project_modules=6`, `mini_tool_schemas=1`) |
| 3 | `artifacts/p2_evidence/sandbox_run/*.db` & `rollback_runbook_sandbox/*.db` | Sandbox test DB tạm do `fp-cli verify-p2` / `verify-rollback-runbook` tạo | **v1→v2→v1** | Bản sao cách ly; **không bao giờ chạm DB thật** |
| 4 | `backups/p2_verified_backup/dist_portable_pre_v2.db` | Verified pre-v2 Online Backup trước migrate | **v1** (`schema_migrations = []`) | `118,784` bytes; `devices=2`, `workflows=2`, `workflow_versions=2`, `jobs=2`, `step_events=31` |

> **Lưu ý vai trò `dist-portable` vs user runtime DB thực tế**: Khi người dùng cuối chạy `FarmerPro.exe` từ gói portable ở máy khác, hàm `resolve_portable_paths()` tạo `<exe_dir>/data/farmerpro.db` tại thư mục chạy thực tế của người dùng. Bản `dist-portable/FarmerPro/data/farmerpro.db` trong repo là bản portable dev/staging đã chạy `migrate-p2` và có backup pre-v2 đi kèm.

---

## 2. Quy trình Rollback Git P2 / P2.1 (Multi-Commit)
> **Cảnh báo**: KHÔNG chạy `git revert 9bd0e45` đơn lẻ vì nhánh `feat/p2-multi-project-storage` có nhiều commits liên tiếp (`9bd0e45`, `e7a0c9b`, `e77fe7c`, `72f78d6`, `7c856e7`, ...).

- **Phương án A (Khuyến nghị — Quay về baseline P1.2 đã nghiệm thu):**
  ```powershell
  git checkout master   # tại commit fab1a1a (hoặc tạo nhánh recovery: git checkout -b rollback/p1.2 fab1a1a)
  ```
- **Phương án B (Revert toàn bộ chuỗi commits P2..HEAD trên nhánh hiện tại):**
  ```powershell
  git revert fab1a1a..HEAD --no-commit
  git commit -m "revert(p2): rollback P2 & P2.1 back to fab1a1a baseline"
  ```

---

## 3. Rollback Runbook Chuẩn hóa cho SQLite WAL Mode (Đã kiểm chứng thực tế)
Tuyệt đối **KHÔNG** copy đè file `.db` hoặc xóa thủ công `-wal`/`-shm` khi tiến trình khác đang mở kết nối ghi.

1. **Dừng toàn bộ tiến trình/kết nối ghi** (`FarmerPro.exe`, `fp-cli.exe`, `fp-mini-runtime.exe`):
   ```powershell
   Get-Process FarmerPro, fp-cli, fp-mini-runtime -ErrorAction SilentlyContinue | Stop-Process -Force
   ```
2. **Checkpoint WAL trước rollback** (`PRAGMA wal_checkpoint(TRUNCATE);`) và **tạo Safety Snapshot** hiện trạng (`pre_rollback_safety_snapshot.db`) qua SQLite Online Backup API (`rusqlite::backup::Backup`).
3. **Kiểm tra toàn vẹn bản backup đích** (`backups/p2_verified_backup/dist_portable_pre_v2.db`): `PRAGMA integrity_check == "ok"`, `PRAGMA foreign_key_check == 0`.
4. **Khôi phục bằng SQLite Online Backup API** (`StorageManager::rollback_database_from_backup`), sau đó chạy lại `PRAGMA wal_checkpoint(TRUNCATE);`.
5. **Xác minh mở lại (Reopen & Health Check)**: kiểm tra `PRAGMA integrity_check == "ok"`, `PRAGMA foreign_key_check == 0`, đối chiếu `schema_migrations = []` và đếm lại các bảng (`devices=2`, `workflows=2`, `jobs=2`, `step_events=31`).

**Chứng cứ chạy thực tế (`fp-cli verify-rollback-runbook` trên bản sao `dist-portable`):**
- File log JSON: `artifacts/p2_evidence/p2_1_rollback_runbook_verification.json`
- Kết quả: `walCheckpointBusy = 0`, `preRollbackVersions = [1, 2]`, `postRollbackVersions = []`, `integrityCheck = "ok"`, `foreignKeyViolations = 0`, `reopenVerified = true`.

---

## 4. Đánh giá Bảo mật Producer Key (DPAPI) & Cross-Machine Recovery
- **Phạm vi DPAPI hiện tại (`crates/fp-security/src/dpapi.rs`)**: Sử dụng `CryptProtectData` với cờ `CRYPTPROTECT_UI_FORBIDDEN (0x1)` (KHÔNG dùng `CRYPTPROTECT_LOCAL_MACHINE`), nên gắn chặt với **CurrentUser Windows SID + Machine DPAPI Master Key** (`WIN32_DPAPI_LOCAL_USER`).
- **Hành vi khi đổi tài khoản Windows hoặc chuyển máy**:
  - `CryptUnprotectData` **fail-closed** (`GetLastError` lỗi giải mã), không thể đọc seed trên tài khoản Windows khác hoặc máy khác.
  - `ProducerKeyBackupBundle` (`backup_signing_key`, scheme `WIN32_DPAPI_LOCAL_USER_BACKUP`) **cũng dùng DPAPI CurrentUser**, nên chỉ dùng để khôi phục trên **cùng user & cùng máy** (ví dụ sau rotate hoặc hỏng file keystore), **không** dùng để chuyển máy.
- **Giải pháp Cross-Machine / Cross-User Offline Recovery (Bổ sung ở P2.1)**:
  - Triển khai `export_portable_recovery_bundle` / `import_portable_recovery_bundle` (`PASSPHRASE_PBKDF2_SHA256_ETM_V1`): KDF `PBKDF2-HMAC-SHA256` **200,000 vòng**, mã hóa `SHA-256-CTR`, xác thực `Encrypt-then-MAC (HMAC-SHA256)` kiểm tra constant-time trước khi giải mã, zeroize toàn bộ buffer seed, và tái mã hóa ngay vào Windows DPAPI của máy/user đích khi import.
  - Tuyệt đối không xuất plaintext seed ra ổ đĩa, log hay Evidence Pack.
- **Khả năng Mini Tools cũ tự xác minh chữ ký sau khi Rotate Key**:
  - Khi build Mini Tool EXE, khóa công khai 32-byte Ed25519 của producer được ghim trực tiếp trong PE resource `RT_RCDATA #7777` (cộng thêm `trusted_publisher.pub`).
  - Khi gọi `rotate_producer_key`, khóa cũ được lưu trữ (`archived_producer_ed25519_*.json`) và khóa mới chỉ áp dụng cho **bản build mới về sau**.
  - Các Mini Tools đã phát hành trước đó tiếp tục tự xác thực bundle cũ bình thường bằng khóa công khai cũ nhúng trong PE `RT_RCDATA #7777` (đã chứng minh bằng unit test `test_old_minitool_continues_verifying_old_bundle_after_producer_key_rotation`).
