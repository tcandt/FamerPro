# FARMERPRO — FINAL ACCEPTANCE PACK & STAGE-GATE MATRIX

**Branch:** `feat/p6-final-validation`  
**Frozen RC.4 Code Commit:** `abaabaa5806ac28de546dd1618897fb178badc68` (Git Tag: `v0.1.0-p6-rc.4` · Docs Commit: `ec23be9bb984c1ebd1589639dfb8299057d46b83`)  
**Decision:** **RC.4 Development Hotfix Gate `APPROVED` · Gate 1 Clean Windows Sandbox `PARTIAL (fp-updater VCRUNTIME140.dll DEFECT FOUND)` · Production Release = `BLOCKED`**

---

## 1. Authoritative Development & Stage-Gate Statuses
| Gate Identifier | Status | Evidence Scope & Honest Boundaries |
|---|---|---|
| `BUILD_PASS` | **PASS** | `106 / 106` Cargo workspace tests PASS across `38` suites ([rust_workspace_test.log](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/rc4_evidence/rust_workspace_test.log)), release binaries & NSIS rebuilt on `abaabaa`. *(PE audit post-Gate 1: `fp-updater.exe` & `fp-cli.exe` import `VCRUNTIME140.dll`; `farmerpro.exe` & `fp-mini-runtime.exe` do not.)* |
| `STANDALONE_LAUNCH_PASS` | **PASS** | Verified both on host and inside **Clean Windows 11 Enterprise Sandbox** (`Auto_P6Alpha.exe` & `Auto_P6Beta.exe` launched with Studio absent and after Studio uninstall; `0` dev tools on Sandbox). |
| `UPDATE_ROLLBACK_PASS` | **FAIL (CLEAN WINDOWS) / PASS (HOST)** | **Dev Host (`VCRUNTIME140.dll` present):** upgrade `v1.0.0 -> v1.1.0`, WAL raw backup, rollback, re-upgrade, revocation, `icacls` & triad checks **PASS**. **Clean Windows Sandbox (`VCRUNTIME140.dll = false`):** `fp-updater.exe` failed immediately with **`0xC0000135` (`STATUS_DLL_NOT_FOUND`)** — minimal CRT-static hotfix proposed for approval. |
| `CLEAN_WINDOWS_SANDBOX_GATE1` | **PARTIAL** | Inside real Windows Sandbox (`Build 26100`): `SANDBOX_BOOT`, `STUDIO_INSTALL`, `STANDALONE_PORTABLE`, `SQLITE_V5_TO_V6` (`clean`, `partial_2cols`, `partial_3cols`), `UNINSTALL_DATA_PRESERVATION` = **PASS**; `WEBVIEW2_PRESENT` (Online/Offline) = **PASS**; `WEBVIEW2_MISSING` (Online/Offline) = **BLOCKED** (OS-embedded `157.0.4341.60` exit `93`); `STANDARD_USER` = **BLOCKED** (Sandbox `WDAGUtilityAccount` single-session isolation); `UPDATE_ROLLBACK` = **FAIL** (`fp-updater` `VCRUNTIME140.dll`). See [GATE1_SANDBOX_REPORT.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/GATE1_SANDBOX_REPORT.md). |
| `GATE2_PLATFORM_LIVE_ADB` | **PASS** | Re-verified on `abaabaa` (`v0.1.0-p6-rc.4`): single-AVD dual-transport deduplication, `adb forward`/`reverse`, `capture_device`, template match (`0.931`), OCR provenance (`ConfidenceSource::Heuristic`), `Pause/Resume`, `Suspend/Resume`, `Cancel`, standalone `Auto_Gate2Live.exe` (`22` events). |
| `GATE2_SETTINGS_LIVE_WORKFLOW` | **PASS** | Executed on Android SDK AVD (`emulator-5554`) against `com.android.settings`. |
| `CROSS_PROCESS_LEASE` | **PASS (IN TESTED SCOPE)** | Live `A1`–`A9` via Win32 `LockFileEx`. Limitations recorded per Auditor Directive: (1) `A6` distinct dual-device verified in Rust cross-process/coordinator tests (1 live AVD on host); (2) `%LOCALAPPDATA%` lock scope verified within single Windows User; (3) offset-64 JSON metadata is diagnostic-only while kernel `[0..64)` byte-range lock is authoritative; (4) lockfiles are never unlinked on release. |
| `SQLITE_V6_MIGRATION_COMPATIBILITY` | **PASS** | Verified across `B1`–`B12` on host + Gate 1 Sandbox against RC.2 (`b5a0567`) clean v5 DB, RC.3 v5 DB, and **partial-provenance v5 DBs (`2/4` and `3/4` cols)**: all converge cleanly to `[1..=6]` (`18` cols, `integrity_check = ok`, `fk_violations = 0`, `LEGACY_UNVERIFIED` / `verified=0`). |
| `GATE2_GAME_SPECIFIC_WORKFLOW` | **NOT TESTED** | **0 game APKs / game-specific workflows tested on live ADB** (to be validated per GameProject acceptance later). |
| `COMMERCIAL_AUTHENTICODE` | **BLOCKED** | External Gate 3: 0 commercial CA certificates present on host. |
| `PRODUCTION_ACCEPTANCE_PASS` | **BLOCKED** | **NOT APPROVED:** Blocked at Gate 1 (`fp-updater VCRUNTIME140.dll`, `Standard User`, `Missing WebView2`), Gate 3 (`Commercial Authenticode`), and per-game Gate 2 workflows. |

---

## 2. Release Artifact Provenance & Checksums (`abaabaa` / Tag `v0.1.0-p6-rc.4`)
| Artifact | Location on Disk (Excluded from Git Blobs) | Size (Bytes) | SHA-256 |
|---|---|---|---|
| `FarmerPro_0.1.0_x64-setup.exe` | `artifacts/p6_evidence/dist_releases/FarmerPro_0.1.0_x64-setup.exe` | `15,435,692` | `3cd7124874dd2763ac6843fa5fecc466b0f1eb7f2db442c91054e418bae4f210` |
| `P6_Alpha_v1.0.0_win-x64.zip` | `artifacts/p6_evidence/dist_releases/P6_Alpha_v1.0.0_win-x64.zip` | `12,156,104` | `06ed29a78896dc6791acbc24fc7665fe9c466b91a0748f9d21045ad85bec70e3` |
| `P6_Beta_v1.0.0_win-x64.zip` | `artifacts/p6_evidence/dist_releases/P6_Beta_v1.0.0_win-x64.zip` | `12,157,696` | `3e8785f7f4f6114a95f2b3b2c571b858769615aed2ade6f2c22885c5301562ea` |
| `Gate2_LiveMini_v1.0.0_win-x64.zip` | `Gate 2 Transient Package (SHA-256 recorded)` | `12,171,644` | `37f2d813f4b9b3cc8099ec05c69d10ef642d0c87781b0ec78ef2e63eadd11da0` |
| `FARMERPRO_GATE1_SANDBOX_REPORT.pdf` | [FARMERPRO_GATE1_SANDBOX_REPORT.pdf](file:///e:/DATA/FarmerPro/FARMERPRO_GATE1_SANDBOX_REPORT.pdf) | `4,510,646` | `057d062c17b639602e871acff1288eb8ecc81f80a65e816f6e12ebb9cbf30f88` |
| `FARMERPRO_RC4_ADDENDUM_REPORT.pdf` | [FARMERPRO_RC4_ADDENDUM_REPORT.pdf](file:///e:/DATA/FarmerPro/FARMERPRO_RC4_ADDENDUM_REPORT.pdf) | `525,049` | `d76ef4d4aa6ce1933c28009606dc349f297118adfd978ed6243af3a2a2058fe6` |
| `FARMERPRO_P6_ACCEPTANCE_REPORT.pdf` | [FARMERPRO_P6_ACCEPTANCE_REPORT.pdf](file:///e:/DATA/FarmerPro/FARMERPRO_P6_ACCEPTANCE_REPORT.pdf) | `712,963` | `ed6e66a930df0b730c56724a46d70555337bf5fff9457c68c9503b48c16c8836` |

*(Baseline progression: `v0.1.0-p6-rc.1` `802f316` -> `v0.1.0-p6-rc.2` `b5a0567` -> `v0.1.0-p6-rc.3` `629cce2` -> `v0.1.0-p6-rc.4` `abaabaa`.)*

---

## 3. Acceptance Pack Checklist
- **Gate 1 Clean Windows Sandbox Report:** [GATE1_SANDBOX_REPORT.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/GATE1_SANDBOX_REPORT.md) (**PARTIAL — `fp-updater` `VCRUNTIME140.dll` Defect Found**)
- **RC.4 Hotfix Report:** [RC4_HOTFIX_REPORT.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/RC4_HOTFIX_REPORT.md) (**PASS**)
- **RC.4 Hotfix Raw JSON:** [rc4_hotfix_verification.json](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/rc4_evidence/rc4_hotfix_verification.json) (**PASS**)
- **Gate 2 Acceptance Addendum & Live Report:** [GATE2_ACCEPTANCE_ADDENDUM.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/GATE2_ACCEPTANCE_ADDENDUM.md) · [GATE2_LIVE_ANDROID_ADB_REPORT.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/GATE2_LIVE_ANDROID_ADB_REPORT.md) (**PASS ON RC.4**)
- **External Gates Tracker:** [EXTERNAL_GATES_STATUS.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/EXTERNAL_GATES_STATUS.md)
- **Machine-Readable Summary & Manifest:** [FINAL_ACCEPTANCE_SUMMARY.json](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/FINAL_ACCEPTANCE_SUMMARY.json) · [FINAL_PACK_MANIFEST_SHA256.json](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/FINAL_PACK_MANIFEST_SHA256.json)
