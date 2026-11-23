# FARMERPRO RC.4 HOTFIX REPORT — CROSS-PROCESS `LockFileEx` & SQLITE `MIGRATION_V6`

**Verified RC.4 Code Commit:** `abaabaa5806ac28de546dd1618897fb178badc68` (`v0.1.0-p6-rc.4`)  
**Previous Baselines (Preserved Untouched):** `v0.1.0-p6-rc.3` (`629cce2`), `v0.1.0-p6-rc.2` (`b5a0567`), `v0.1.0-p6-rc.1` (`802f316`)  
**Rust Workspace Test Log:** [rust_workspace_test.log](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/rc4_evidence/rust_workspace_test.log) (`106 / 106` passed, `0` failed across `38` suites)  
**RC.4 Supplement Verification JSON & Log:** [rc4_hotfix_verification.json](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/rc4_evidence/rc4_hotfix_verification.json) · [verify_rc4_supplement.log](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/rc4_evidence/verify_rc4_supplement.log)  
**Gate 2 Regression JSON & Log:** [gate2_live_adb_report.json](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/gate2_live_adb/gate2_live_adb_report.json) · [verify_gate2_live_adb.log](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/rc4_evidence/verify_gate2_live_adb.log)

---

## I. Fix A — Cross-Process Device Lease Architecture (`Win32 LockFileEx`)

### 1. Architecture Summary ([crates/fp-scheduler/src/os_lock.rs](file:///e:/DATA/FarmerPro/crates/fp-scheduler/src/os_lock.rs) & [crates/fp-scheduler/src/lib.rs](file:///e:/DATA/FarmerPro/crates/fp-scheduler/src/lib.rs))
- **Shared Lock Directory:** Both `FarmerPro.exe` (Studio) and every exported `fp-mini-runtime.exe` Mini Tool resolve `%LOCALAPPDATA%\FarmerPro\locks\` (with `FARMERPRO_LOCK_DIR` override only for isolated test sandboxes and NTFS reparse/symlink rejection).
- **Stable Canonical Hash Lockfiles:** Canonical device identity (`local-emu:<even_port>`, `android-boot:<boot_sn>:<boot_id>`, or `serial:<id>`) is hashed with SHA-256 (`dev_<sha256_hex>.lock`).
- **Kernel-Enforced Mutual Exclusion:** `CreateFileW` opens `dev_<hash>.lock` with `GENERIC_READ | GENERIC_WRITE`, `FILE_SHARE_READ | FILE_SHARE_WRITE`, `OPEN_ALWAYS`, and `LockFileEx` acquires an exclusive non-blocking lock (`LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY`) over byte range `[0..64)`.
- **Competing Owner Diagnostics Without Lock Contention:** JSON metadata (`{pid, exePath, jobId, deviceId, canonicalKey, generation, acquiredAtUtc, expiresAtUtc}`) is written starting at byte offset `64`, allowing a blocked process receiving `ERROR_LOCK_VIOLATION` (`33`) to read owner metadata and return a structured `DEVICE_LEASE_CONFLICT` error.
- **No Unlink Race Conditions:** Lockfiles are truncated back to offset `64` while still holding `[0..64)` before `UnlockFileEx` + `CloseHandle` and are never deleted on release, preventing concurrent open/unlink/recreate races.
- **TTL Non-Override Rule:** Because kernel `[0..64)` remains locked while the holder handle is alive, elapsed lease TTL alone never steals an OS lock from a living process (verified by cross-process child test in `fp-scheduler` and live E2E).
- **RAII Drop Safety & Crash Recovery:** `DeviceLeaseGuard` and `CoordinatorLeaseDropGuard` ensure unlock on normal completion, error, cancel, or future drop. Windows kernel closes the handle immediately on `taskkill /F`; on next startup, `StorageManager::recover_interrupted_jobs_on_startup` marks orphaned `running`/`verifying`/`queued` DB rows `interrupted`, bumps `fencing_generation`, and clears stale `device_leases` rows only when `inspect_os_lock_holder` reports unlocked.
- **Fail-Closed Guarantee:** If the lock directory or lockfile cannot be initialized safely, `MultiDeviceScheduler` returns `DEVICE_LOCK_INIT_FAILED` and refuses to run unlocked.
- **Zero-Bypass Coverage:** Applied across `RuntimeCoordinator` (`studio_run_project_workflow`, `studio_start_project_workflow`, `resume_from_checkpoint`), V1 `execute_workflow`, `apps/mini-runtime` (`mini_run_task`, `mini_start_task`, `--run-task`), and `adb_disconnect_endpoint` cleanup.

### 2. Test A Results (`A1`–`A9`, Observed on `695a8b6` / `abaabaa`)
| Case | Scenario | Outcome | Scope / Evidence |
|---|---|---|---|
| `A1` | Studio holds `emulator-5554`, Mini `Job B` tries concurrent run | **PASS** (`Blocked: DEVICE_LEASE_CONFLICT`) | Live AVD `emulator-5554` (`pid`, `jobId`, `dev_293bc69f...lock`) |
| `A2` | Studio `Job A` ends (`succeeded`), Mini `Job B` retries | **PASS** (`Succeeded`, `LIVE_ADB(emulator-5554)`) | Live AVD `emulator-5554` |
| `A3` | Forced-kill Studio (`taskkill /F`) -> Mini takes device -> Studio restarts | **PASS** (Mini `Succeeded` immediately; `Job C -> interrupted`, `Job D -> succeeded`) | Live AVD `emulator-5554` |
| `A4` | Forced-kill Mini Runtime (`Auto_CrossMini.exe`) -> Studio blocked then takes device after kill | **PASS** (`studioBlockedWhileMiniRunning=true`, Studio `Job E succeeded`, crashed Mini row `interrupted`) | Live AVD `emulator-5554` |
| `A5` | Dual transports (`emulator-5554` + `127.0.0.1:5555`) from two EXEs | **PASS** (`DEVICE_LEASE_CONFLICT`) | Live AVD `emulator-5554` + loopback |
| `A6` | Two genuinely distinct devices running concurrently | **PASS (Rust + Synthetic)** | Host runs 1 live AVD; verified by Rust subprocess & coordinator tests (`emulator-5554` + `emulator-5556`) |
| `A7` | ADB disconnect / reconnect cycle | **PASS** (`disconnected 127.0.0.1:5555`, leases clean) | Live AVD `emulator-5554` |
| `A8` | Two independent Mini EXEs (`Auto_CrossMini.exe` vs `Auto_CrossMini2.exe`, Studio closed) | **PASS** (Mini 2 blocked while Mini 1 ran; Mini 2 `Succeeded` after release) | Live AVD `emulator-5554` |
| `A9` | Uncreatable OS lock path (`FARMERPRO_LOCK_DIR` = regular file) | **PASS** (Fail-closed `DEVICE_LOCK_INIT_FAILED`) | Standalone CLI `Auto_CrossMini.exe` |

---

## II. Fix B — SQLite Schema Migration `MIGRATION_V6`

### 1. Schema & Compatibility Implementation
- **Version Bump:** `CURRENT_SQLITE_SCHEMA_VERSION` raised from `5` -> `6` in [crates/fp-bundle-contract/src/lib.rs](file:///e:/DATA/FarmerPro/crates/fp-bundle-contract/src/lib.rs#L11); `RuntimeCompatibilityProfile::default()` and `CompatibilityVersions::default()` updated consistently across Builder (`fp-builder`), Runtime (`fp-mini-runtime`), and Updater (`fp-updater`).
- **History Intact & Transactional `MIGRATION_V6` ([crates/fp-storage/src/migrations.rs](file:///e:/DATA/FarmerPro/crates/fp-storage/src/migrations.rs)):**
  - `MIGRATION_V1_SQL`–`MIGRATION_V5_PUBLISHER_PROFILES_SQL` preserved untouched.
  - Fast-path requires `1..=6` AND runs `verify_v6_provenance_columns(conn)` (`PRAGMA table_info(runtime_variable_envelopes)`).
  - Unversioned trailing `ALTER TABLE` block removed and moved inside atomic `BEGIN IMMEDIATE` `if !applied.contains(&6)`: adds `confidence_source`, `measured_confidence`, `rule_matched`, `device_id` if absent; normalizes unknown/blank or `LEGACY_UNVERIFIED` rows to `verified = 0, confidence = NULL, confidence_source = NULL, measured_confidence = NULL` (never claiming `LIVE` provenance); re-checks `verify_v6_provenance_columns` & `PRAGMA foreign_key_check = 0`; records `INSERT INTO schema_migrations (version, applied_at) VALUES (6, ?1)` in the same transaction.
- **Fail-Closed Opening & Raw Pre-Update Backup:**
  - Removed silent `StorageManager::new_in_memory()` fallbacks in Studio ([apps/desktop/src-tauri/src/main.rs](file:///e:/DATA/FarmerPro/apps/desktop/src-tauri/src/main.rs)) and Mini Runtime ([apps/mini-runtime/src/main.rs](file:///e:/DATA/FarmerPro/apps/mini-runtime/src/main.rs)).
  - Added `StorageManager::online_backup_raw_file_to` in [crates/fp-storage/src/lib.rs](file:///e:/DATA/FarmerPro/crates/fp-storage/src/lib.rs) and wired it into [crates/fp-updater/src/lib.rs](file:///e:/DATA/FarmerPro/crates/fp-updater/src/lib.rs#L1556) so pre-update backups capture raw source DB prior to migration.

### 2. Test B Results (`B1`–`B12`, Verified on Genuine RC.2 `b5a0567` v5 DB)
- `B1`–`B4`: RC.2 `b5a0567` v5 DB (`[1..=5]`, 14 columns), RC.3 v5 DB, and fresh DB all converge deterministically to `[1, 2, 3, 4, 5, 6]` across repeated opens with all 4 provenance columns present.
- `B5`–`B7`: `projects`, `workflows`, `workflow_versions`, `task_definitions`, `project_releases`, `accounts` (`dpapi_ref_rc2_token`), and `publisher_profiles` preserved `100%`; `legacy_ocr_val` remains `LEGACY_UNVERIFIED`, `verified=false`, `confidence=null`, `confidenceSource=null`; `studio_get_project_runtime_vars` returns `varsError: null`.
- `B8`–`B10`: `PRAGMA integrity_check = ["ok"]`, `PRAGMA foreign_key_check = []`, simulated v6 transaction failure rolls back cleanly, and restored backup matches SHA-256 `1a777afb768c0d3d94eaa0a1d9f6457120611f91b2f7cc470bb5fdc3d04fa6ad`.
- `B11`–`B12`: Independent `Auto_CrossMini.exe` upgrades pre-existing RC.2 v5 `runtime_<slug>.db` to `sqliteSchemaVersion: 6` and runs `Succeeded`; future v99 DB is blocked with `SQLITE_DOWNGRADE_INCOMPATIBLE` and missing-column v6 DB is blocked with `SQLITE_SCHEMA_INCOMPATIBLE`.

---

## III. RC.4 Release Artifact Checksums (`abaabaa` / `v0.1.0-p6-rc.4`)

| Artifact | Size (Bytes) | SHA-256 |
|---|---|---|
| `FarmerPro_0.1.0_x64-setup.exe` (NSIS) | `15,435,692` | `3cd7124874dd2763ac6843fa5fecc466b0f1eb7f2db442c91054e418bae4f210` |
| `target/release/farmerpro.exe` | `20,677,632` | `270101d8179f408ecfdf2b2672c6fdd4bed0b84d88d154f143f996ea03bb9ad0` |
| `target/release/fp-mini-runtime.exe` | `16,178,176` | `27db65970ca36796dddbb217f8c7a47090dbd5670074d17f05e93aa982a42aa6` |
| `target/release/fp-updater.exe` | `5,843,456` | `6d92e62630c6a5a991017aa382707495c1c5a46fb4969fe992563009c040bb44` |
| `P6_Alpha_v1.0.0_win-x64.zip` | `12,156,104` | `06ed29a78896dc6791acbc24fc7665fe9c466b91a0748f9d21045ad85bec70e3` |
| `P6_Beta_v1.0.0_win-x64.zip` | `12,157,696` | `3e8785f7f4f6114a95f2b3b2c571b858769615aed2ade6f2c22885c5301562ea` |
| `Gate2_LiveMini_v1.0.0_win-x64.zip` | `12,171,644` | `37f2d813f4b9b3cc8099ec05c69d10ef642d0c87781b0ec78ef2e63eadd11da0` |

**Stage-Gate Stop:** `PRODUCTION_ACCEPTANCE_PASS` remains **`BLOCKED`** (`Gate 1 CLEAN_WINDOWS_VM = BLOCKED`, `Gate 3 COMMERCIAL_AUTHENTICODE = BLOCKED`, `GATE2_GAME_SPECIFIC_WORKFLOW = NOT TESTED`).
