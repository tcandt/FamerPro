# FARMERPRO P2.2 HOTFIX — CHÍNH SÁCH BẢO MẬT ARGON2ID + AES-256-GCM & MÔ HÌNH CRASH RECOVERY

**Code Commit (đã kiểm thử toàn bộ workspace + forced crash):** `0218e2e3b0c9567115a07970e762a286b7b6c06a` (`0218e2e`)  
**Phạm vi sửa đổi mã nguồn từ `b3d3879`:** Chỉ `Cargo.lock`, `crates/fp-security/*`, `crates/fp-storage/*`, `crates/fp-builder/src/lib.rs`, `apps/cli/src/main.rs` (`0` dòng thay đổi trong `apps/desktop/src` và không đụng P3–P6).

---

## I. CHÍNH SÁCH BẢO MẬT PORTABLE PRODUCER KEY (HOTFIX I)

### 1. Chuẩn mã hóa sản phẩm: `ARGON2ID_AES_256_GCM_V3` (`formatVersion = 3`)
- **Loại bỏ hoàn toàn `SHA-256-CTR` tự xây dựng:** Các hàm `xor_sha256_ctr_stream`, `pbkdf2_hmac_sha256`, `hmac_sha256`, `compute_portable_mac` đã bị xóa hoàn toàn khỏi `crates/fp-security/src/keystore.rs`.
- **KDF chuẩn Argon2id (`argon2 v0.5.3`):**
  - Thuật toán: `argon2id`, phiên bản `0x13`.
  - Tham số đo trên workstation: `memoryKib = 65536` (64 MiB), `iterations = 3`, `parallelism = 1`, `outputLenBytes = 32`.
  - Thời gian KDF đo thực tế bằng bản release (`fp-cli.exe verify-hotfix`): **108 ms** (export) / **114 ms** (import).
  - Kiểm tra biên an toàn KDF trước khi chạy Argon2id khi import: `19456 <= memoryKib <= 524288`, `2 <= iterations <= 20`, `1 <= parallelism <= 16`, `outputLenBytes == 32`.
- **AEAD chuẩn AES-256-GCM (`aes-gcm v0.10.3`):**
  - Nguồn ngẫu nhiên CSPRNG OS (`rand::rngs::OsRng`) cho Ed25519 seed (32 bytes), Salt (16 bytes), và AES-GCM Nonce (12 bytes / 96-bit).
  - Ciphertext xuất 48 bytes (32-byte seed + 16-byte GHASH authentication tag).
  - **Authenticated Additional Data (AAD):** Gắn chặt toàn bộ metadata JSON (`formatVersion`, `protectionScheme`, `aeadAlgorithm`, `kdfParams`, `keyId`, `publicKeyHex`, `createdAtUtc`, `backedUpAtUtc`, `saltB64`, `nonceB64`). Bất kỳ chỉnh sửa nào ở passphrase, ciphertext, tag hoặc metadata đều thất bại fail-closed với `RECOVERY_AUTH_FAILED`.
- **Zeroization bộ nhớ:** Sử dụng `zeroize::Zeroizing<[u8; 32]>` / `Zeroizing<Vec<u8>>` cho khóa AES dẫn xuất và seed Ed25519 giải mã. Không ghi plaintext seed ra PDF, JSON evidence, log hoặc `releases/`.

### 2. Kế hoạch ngừng phát hành & chuyển đổi định dạng cũ `PASSPHRASE_PBKDF2_SHA256_ETM_V1`
- **Tình trạng định dạng cũ trên thực tế:** Đã quét toàn bộ repo (`artifacts/`, `backups/`, `projects/`, `releases/`) và `%LOCALAPPDATA%\FarmerPro` — **0 tệp Recovery Bundle V1 (`PASSPHRASE_PBKDF2_SHA256_ETM_V1`) tồn tại trên đĩa hoặc từng được phát hành tới người dùng cuối.**
- **Chính sách sản phẩm:** Định dạng `PASSPHRASE_PBKDF2_SHA256_ETM_V1` (dùng `SHA-256-CTR`) được phân loại chính thức là **định dạng thử nghiệm nội bộ trước hotfix và đã ngừng phát hành vĩnh viễn**.
- **Chặn mặc định khi import:** `ProducerKeystore::import_portable_recovery_bundle` kiểm tra sớm trước giải mã và trả lỗi tường minh `RECOVERY_RETIRED_EXPERIMENTAL_FORMAT` nếu gặp `"PASSPHRASE_PBKDF2_SHA256_ETM_V1"`.
- **Kế hoạch xử lý có kiểm soát nếu có bản thử nghiệm nội bộ:** Người vận hành xuất lại Recovery Bundle mới theo chuẩn `ARGON2ID_AES_256_GCM_V3` trực tiếp từ Windows DPAPI Keystore đang hoạt động (`studio_export_portable_producer_key`) và hủy tệp thử nghiệm cũ. Không giữ mã giải mã `SHA-256-CTR` trong binary production.

### 3. Giới hạn kiểm thử Windows User thứ hai thật
- Môi trường kiểm thử hiện tại chỉ có 1 tài khoản đăng nhập tương tác (`desktop-n916ap3\tinh-nguyen`); các tài khoản hệ thống khác (`Administrator`, `CodexSandboxOffline`, `CodexSandboxOnline`) không có credential phiên tương tác và không được phép vượt quyền kiểm soát truy cập.
- Do đó hạng mục **"Khôi phục trên Windows User thứ hai thật ở mức OS session"** được ghi nhận trung thực là **BLOCKED (môi trường thiếu credential Windows User thứ 2)**, trong khi cơ chế bảo vệ CurrentUser DPAPI (Win32 `CryptProtectData` không dùng cờ `CRYPTPROTECT_LOCAL_MACHINE`) và cơ chế giải thoát cross-user bằng `PortableProducerKeyRecoveryBundle V3` đã được kiểm thử đầy đủ.

---

## II. MÔ HÌNH CRASH CONSISTENCY & RECOVERY (HOTFIX II)

### 1. Phát biểu kiến trúc chính xác
FarmerPro **KHÔNG** gọi ghép nối giữa SQLite transaction và thao tác thư mục/tệp NTFS là một "giao dịch nguyên tử tuyệt đối đa hệ thống" chỉ dựa vào `tx.commit()`. Thay vào đó, SQLite WAL và NTFS nằm trên **hai miền bền vững tách biệt**, được đảm bảo nhất quán qua mô hình:

> **SQLite WAL ACID (`IMMEDIATE` transaction) + Intent-Journaled Two-Phase Staging/Trash (`.fp_staging`, `.fp_trash`, `.fp_journal`) + Startup Idempotent Reconciliation (`StorageManager::open_and_reconcile`).**

### 2. Luồng hai pha & Chứng cứ cam kết SQLite (`SqliteCommitWitness`)
1. **Tạo mới / Nhân bản Project (`create_<project_id>.json`):**
   - Dựng cây thư mục trong `.fp_staging/<slug>.<uuid>`, ghi intent journal `.fp_journal/create_<id>.json`, chuyển đổi thư mục ra `projects/<slug>`, rồi mới chạy `tx.commit()`, cuối cùng xóa journal.
   - Nếu tiến trình bị OS kill (`TerminateProcess`) trong `.fp_staging` **hoặc ngay sau `rename` nhưng trước `tx.commit()`**, khi khởi động lại `SqliteCommitWitness::has_project` trả về `false` ⇒ reconciler tự động xóa thư mục `projects/<slug>` chưa commit (sau khi xác minh không trùng active dir của project khác & đúng `projectId`) và dọn `.fp_staging` + journal. Nếu kill sau `tx.commit()`, `has_project` trả `true` ⇒ giữ thư mục và dọn journal.
2. **Cập nhật Manifest / UI Schema / Workflow / Template (`mutation_<uuid>.json`):**
   - Trước khi ghi đè tệp thật, sao lưu pre-image các tệp sẽ sửa vào `.fp_staging/bak_<id>/` và ghi `.fp_journal/mutation_<id>.json` chứa `MutationCommitWitness` (`projects.updated_at`, `workflow_versions.sha256`, hoặc `vision_templates.sha256`).
   - Khi chạy `reconcile_project_storage_with_witness`, thư mục `.fp_journal/*` được xử lý **TRƯỚC** khi quét dọn `.fp_staging/*`:
     - Nếu `matches_mutation_witness == false` (kill trước `tx.commit()`): khôi phục chính xác từng byte pre-image từ `.fp_staging/bak_<id>/` hoặc xóa tệp mới chưa commit, quét sạch `.*.tmp.*`.
     - Nếu `matches_mutation_witness == true` (kill sau `tx.commit()`): giữ nguyên trạng thái đã commit và dọn `.fp_staging/bak_<id>/` + journal.
3. **Xóa Project (`delete_<project_id>.json` + `.fp_trash`):**
   - Ghi `.fp_journal/delete_<id>.json` và dời `projects/<slug>` vào `.fp_trash/<slug>.<tag>` trước `tx.commit()`.
   - Kill trước `tx.commit()` ⇒ `has_project == true` ⇒ dời ngược `.fp_trash` về `projects/<slug>`. Kill sau `tx.commit()` ⇒ `has_project == false` ⇒ xóa vĩnh viễn `.fp_trash` và journal.
