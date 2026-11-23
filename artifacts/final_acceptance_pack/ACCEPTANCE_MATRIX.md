# FARMERPRO — FINAL ACCEPTANCE PACK & STAGE-GATE MATRIX

**Branch:** `feat/p6-final-validation`  
**Verified RC.4 Code Commit:** `abaabaa5806ac28de546dd1618897fb178badc68` (Git Tag: `v0.1.0-p6-rc.4`)  
**Decision:** **RC.4 Hotfixes Verified (`CROSS_PROCESS_LEASE = PASS` · `SQLITE_V6_MIGRATION = PASS` · `GATE2_PLATFORM_LIVE_ADB = PASS`) · Production Release = `BLOCKED`**

---

## 1. Authoritative Development & Gate Statuses
| Gate Identifier | Status | Evidence Scope & Honest Boundaries |
|---|---|---|
| `BUILD_PASS` | **PASS** | `106 / 106` Cargo workspace tests PASS across `38` suites ([rust_workspace_test.log](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/rc4_evidence/rust_workspace_test.log)), release binaries (`fp-updater`, `fp-mini-runtime`, `farmerpro`, NSIS `FarmerPro_0.1.0_x64-setup.exe`) rebuilt cleanly on RC.4. |
| `STANDALONE_LAUNCH_PASS` | **PASS** | Portable Mini Tools (`Auto_P6Alpha.exe`, `Auto_P6Beta.exe`) launched after Studio uninstall; `Auto_Gate2Live.exe`, `Auto_CrossMini.exe`, `Auto_CrossMini2.exe` launched standalone against live ADB (`LIVE ADB: 1 ONLINE`). |
| `UPDATE_ROLLBACK_PASS` | **PASS** | Upgrade `v1.0.0 -> v1.1.0`, pre-migration WAL raw backup & data preservation, rollback `v1.1.0 -> v1.0.0`, re-upgrade `v1.0.0 -> v1.1.0` with trusted key, revoked-key/policy/compat gates, real NTFS `icacls (RX)` fallback, Win32 `atomic_write_file` retry exhaustion & journal reconciliation PASS. |
| `GATE2_PLATFORM_LIVE_ADB` | **PASS (RE-VERIFIED ON RC.4)** | Re-run on `abaabaa` (`v0.1.0-p6-rc.4`): single-AVD dual-transport deduplication, `adb forward`/`reverse`, `capture_device`, template match (`0.931`), OCR provenance (`ConfidenceSource::Heuristic`, badge `LIVE · Đã khớp quy tắc`), `Pause/Resume`, `Suspend/Resume`, `Cancel`, and standalone `Auto_Gate2Live.exe` (`22` events). See [GATE2_LIVE_ANDROID_ADB_REPORT.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/GATE2_LIVE_ANDROID_ADB_REPORT.md). |
| `GATE2_SETTINGS_LIVE_WORKFLOW` | **PASS (RE-VERIFIED ON RC.4)** | Executed on Android SDK AVD (`fp_gate2_avd` / `emulator-5554`, API 37 WHPX) against `com.android.settings` with real `Windows.Media.Ocr`, `SafeTap`, `Swipe`, `BACK`. |
| `CROSS_PROCESS_LEASE` | **PASS (RC.4 `LockFileEx`)** | Live E2E (`A1`–`A9` in [RC4_HOTFIX_REPORT.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/RC4_HOTFIX_REPORT.md)): Win32 `LockFileEx` (`%LOCALAPPDATA%\FarmerPro\locks\dev_<sha256>.lock`) blocks `Auto_CrossMini.exe` & `127.0.0.1:5555` while Studio holds device (`DEVICE_LEASE_CONFLICT`), blocks `Auto_CrossMini2.exe` while `Auto_CrossMini.exe` holds device, unlocks cleanly on release/disconnect or `taskkill /F` (with crashed SQLite job startup recovery to `interrupted`), fails closed on invalid lock dir (`DEVICE_LOCK_INIT_FAILED`), and preserves TTL non-override. (`A6` distinct dual-device verified in Rust cross-process test; 1 live AVD on host.) |
| `SQLITE_V6_MIGRATION_COMPATIBILITY` | **PASS (RC.4 `MIGRATION_V6`)** | Genuine RC.2 (`b5a0567`) v5 DB + RC.3 v5 DB + fresh DB all upgrade atomically to schema `v6` (`[1..=6]`, all 4 provenance columns verified via `PRAGMA table_info`, `integrity_check = ok`, `fk = 0`). Zero `no such column: confidence_source` error; legacy rows keep `verified=false, confidence=null`; independent `Auto_CrossMini.exe` upgrades v5 `runtime_*.db` to v6; incompatible future `v99` / missing-column DBs fail closed (`SQLITE_DOWNGRADE_INCOMPATIBLE` / `SQLITE_SCHEMA_INCOMPATIBLE`). |
| `GATE2_GAME_SPECIFIC_WORKFLOW` | **NOT TESTED** | **0 game APKs / game-specific workflows tested on live ADB** (to be validated per GameProject acceptance later). |
| `PRODUCTION_ACCEPTANCE_PASS` | **BLOCKED** | **External Gates Remain `BLOCKED`:** Gate 1 (`CLEAN_WINDOWS_VM = BLOCKED`) and Gate 3 (`COMMERCIAL_AUTHENTICODE = BLOCKED`). |

---

## 2. Release Artifact Provenance & Checksums (`abaabaa` / Tag `v0.1.0-p6-rc.4`)
| Artifact | Location on Disk (Excluded from Git Blobs) | Size (Bytes) | SHA-256 |
|---|---|---|---|
| `FarmerPro_0.1.0_x64-setup.exe` | `artifacts/p6_evidence/dist_releases/FarmerPro_0.1.0_x64-setup.exe` | `15,435,692` | `3cd7124874dd2763ac6843fa5fecc466b0f1eb7f2db442c91054e418bae4f210` |
| `P6_Alpha_v1.0.0_win-x64.zip` | `artifacts/p6_evidence/dist_releases/P6_Alpha_v1.0.0_win-x64.zip` | `12,156,104` | `06ed29a78896dc6791acbc24fc7665fe9c466b91a0748f9d21045ad85bec70e3` |
| `P6_Beta_v1.0.0_win-x64.zip` | `artifacts/p6_evidence/dist_releases/P6_Beta_v1.0.0_win-x64.zip` | `12,157,696` | `3e8785f7f4f6114a95f2b3b2c571b858769615aed2ade6f2c22885c5301562ea` |
| `Gate2_LiveMini_v1.0.0_win-x64.zip` | `Gate 2 Transient Package (SHA-256 recorded)` | `12,171,644` | `37f2d813f4b9b3cc8099ec05c69d10ef642d0c87781b0ec78ef2e63eadd11da0` |
| `FARMERPRO_RC4_ADDENDUM_REPORT.pdf` | [FARMERPRO_RC4_ADDENDUM_REPORT.pdf](file:///e:/DATA/FarmerPro/FARMERPRO_RC4_ADDENDUM_REPORT.pdf) | `525,049` | `d76ef4d4aa6ce1933c28009606dc349f297118adfd978ed6243af3a2a2058fe6` |
| `FARMERPRO_P6_ACCEPTANCE_REPORT.pdf` | [FARMERPRO_P6_ACCEPTANCE_REPORT.pdf](file:///e:/DATA/FarmerPro/FARMERPRO_P6_ACCEPTANCE_REPORT.pdf) | `712,963` | `ed6e66a930df0b730c56724a46d70555337bf5fff9457c68c9503b48c16c8836` |

*(Baseline progression: `v0.1.0-p6-rc.1` `802f316` -> `v0.1.0-p6-rc.2` `b5a0567` -> `v0.1.0-p6-rc.3` `629cce2` -> `v0.1.0-p6-rc.4` `abaabaa`.)*

---

## 3. Acceptance Pack Checklist
- **RC.4 Hotfix Report:** [RC4_HOTFIX_REPORT.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/RC4_HOTFIX_REPORT.md) (**PASS**)
- **RC.4 Hotfix Raw JSON:** [rc4_hotfix_verification.json](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/rc4_evidence/rc4_hotfix_verification.json) (**PASS**)
- **RC.3 Supplemental Archive:** [RC3_SUPPLEMENTAL_VERIFICATION_REPORT.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/RC3_SUPPLEMENTAL_VERIFICATION_REPORT.md) (**HISTORICAL BASELINE**)
- **Gate 2 Acceptance Addendum & Live Report:** [GATE2_ACCEPTANCE_ADDENDUM.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/GATE2_ACCEPTANCE_ADDENDUM.md) · [GATE2_LIVE_ANDROID_ADB_REPORT.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/GATE2_LIVE_ANDROID_ADB_REPORT.md) (**PASS ON RC.4**)
- **External Gates Tracker:** [EXTERNAL_GATES_STATUS.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/EXTERNAL_GATES_STATUS.md)
- **Machine-Readable Summary & Manifest:** [FINAL_ACCEPTANCE_SUMMARY.json](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/FINAL_ACCEPTANCE_SUMMARY.json) · [FINAL_PACK_MANIFEST_SHA256.json](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/FINAL_PACK_MANIFEST_SHA256.json)
