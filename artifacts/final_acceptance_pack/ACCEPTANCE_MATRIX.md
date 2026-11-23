# FARMERPRO — FINAL ACCEPTANCE PACK & STAGE-GATE MATRIX

**Branch:** `feat/p6-final-validation`  
**Verified RC Code Baseline:** `b5a05673b1c0ad10dd4c6ce4463eaabbd3be40fe` (Git Tag: `v0.1.0-p6-rc.2`; Gate 2 surgical hotfix trên nền `v0.1.0-p6-rc.1` `802f316`)  
**Decision:** **P6 Development Closeout = APPROVED · Gate 2 Platform/Settings Live ADB = PASS · Game Workflows = NOT TESTED · Production Release = NOT APPROVED (`BLOCKED` at Gate 1 & Gate 3)**

---

## 1. Authoritative Development & Gate Statuses
| Gate Identifier | Status | Evidence Scope & Honest Boundaries |
|---|---|---|
| `BUILD_PASS` | **PASS** | `101 / 101` Cargo workspace tests PASS across `38` test suites ([cargo_test_workspace_p6.log](file:///e:/DATA/FarmerPro/artifacts/p6_evidence/cargo_test_workspace_p6.log)), release binaries (`fp-updater`, `fp-mini-runtime`, NSIS `FarmerPro_0.1.0_x64-setup.exe`) rebuilt cleanly on `b5a0567` (`v0.1.0-p6-rc.2`). |
| `STANDALONE_LAUNCH_PASS` | **PASS** | Both Portable Mini Tools (`Auto_P6Alpha.exe` compact Unsigned, `Auto_P6Beta.exe` expanded Self-Signed QA) launched after Studio uninstall; plus `Auto_Gate2Live.exe` launched standalone against live ADB (`LIVE ADB: 1 ONLINE` on `emulator-5554`). |
| `UPDATE_ROLLBACK_PASS` | **PASS** | Upgrade `v1.0.0 -> v1.1.0`, WAL backup & post-update SQLite data preservation, binary rollback `v1.1.0 -> v1.0.0`, legitimate re-upgrade `v1.0.0 -> v1.1.0` with trusted key, revoked-key/policy/compat regression gates, real NTFS `icacls (RX)` fallback, Win32 `atomic_write_file` retry exhaustion & journal reconciliation PASS. *Full 6-component same-user overwrite resets `trustEpoch` to `1` per Threat Model.* |
| `GATE2_PLATFORM_LIVE_ADB` | **PASS** | Canonical single-AVD deduplication (`emulator-5554` + `127.0.0.1:5555` → `1 ONLINE`), `MultiDeviceScheduler` canonical lease locking, `capture_device`, template match (`0.931`), `adb forward`/`reverse` create/list/remove, `Pause/Resume`, `Suspend/Resume Checkpoint`, `Cancel`, and standalone `Auto_Gate2Live.exe` (`22` events, `anyTestkitBadge=false`). See [GATE2_ACCEPTANCE_ADDENDUM.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/GATE2_ACCEPTANCE_ADDENDUM.md). |
| `GATE2_SETTINGS_LIVE_WORKFLOW` | **PASS** | Executed end-to-end on Android SDK AVD (`fp_gate2_avd` / `emulator-5554`, API 37 WHPX) against `com.android.settings` with real `Windows.Media.Ocr` text extraction, `SafeTap`, `Swipe`, `BACK`, and clean before/after device screenshots. |
| `GATE2_GAME_SPECIFIC_WORKFLOW` | **NOT TESTED / PENDING** | **0 game APKs / game-specific workflows tested on live ADB** (JX2 Mobile, SLG, or other titles remain `PENDING / NOT TESTED`). |
| `PRODUCTION_ACCEPTANCE_PASS` | **BLOCKED** | **2 External Gates Remain `BLOCKED`:** Gate 1 (Clean Windows VM / Standard User / WebView2 missing) and Gate 3 (Commercial Authenticode PublicSigned CA Certificate). |

---

## 2. Release Artifact Provenance & Checksums (`b5a0567` / Tag `v0.1.0-p6-rc.2`)
| Artifact | Location on Disk (Excluded from Git Blobs) | Size (Bytes) | SHA-256 |
|---|---|---|---|
| `FarmerPro_0.1.0_x64-setup.exe` | `artifacts/p6_evidence/dist_releases/FarmerPro_0.1.0_x64-setup.exe` | `15,223,807` | `4ffc3fc3d49d9b6f4b3ab82bf035a82dfb6e4411319bbec416c3412567d4d0fa` |
| `P6_Alpha_v1.0.0_win-x64.zip` | `artifacts/p6_evidence/dist_releases/P6_Alpha_v1.0.0_win-x64.zip` | `11,929,011` | `7b46389bed95c08678a24455d9b1e1c6215c5a61385508b8bddac35b4e9bce05` |
| `P6_Beta_v1.0.0_win-x64.zip` | `artifacts/p6_evidence/dist_releases/P6_Beta_v1.0.0_win-x64.zip` | `11,930,781` | `cf77b69ad5fcd5bf893f73ae0d4732a01957d31243b555af55ce065c1e6b1db2` |
| `Gate2_LiveMini_v1.0.0_win-x64.zip` | `Gate 2 Transient Package (SHA-256 recorded)` | `11,944,570` | `ddb899b491df8697cc7fe817987e9a13ae2d1f2819da77568c46290afbae9424` |
| `FARMERPRO_P6_ACCEPTANCE_REPORT.pdf` | [FARMERPRO_P6_ACCEPTANCE_REPORT.pdf](file:///e:/DATA/FarmerPro/FARMERPRO_P6_ACCEPTANCE_REPORT.pdf) | `712,030` | `79821bec5b38028c3a86dc2cadb193a1d99358c061d912d0ddf6fa884e986651` |
| `EVIDENCE_MANIFEST_SHA256.json` | [EVIDENCE_MANIFEST_SHA256.json](file:///e:/DATA/FarmerPro/artifacts/p6_evidence/EVIDENCE_MANIFEST_SHA256.json) | `64 files` | P6 Development Evidence SHA-256 Manifest |

*(Pre-hotfix `v0.1.0-p6-rc.1` baseline `802f316`: installer `15,172,294` B `87984447...`, pre-Gate-2 PDF `708,938` B `87d46378...` — retained in Git tag history `v0.1.0-p6-rc.1`.)*

---

## 3. Acceptance Pack Checklist
- **Gate 2 Acceptance Addendum:** [GATE2_ACCEPTANCE_ADDENDUM.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/GATE2_ACCEPTANCE_ADDENDUM.md) (**PRESENT — PASS**)
- **Final Acceptance Matrix:** [ACCEPTANCE_MATRIX.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/ACCEPTANCE_MATRIX.md) (**PRESENT**)
- **Security Risk Register & Threat Model:** [SECURITY_RISK_REGISTER_AND_THREAT_MODEL.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/SECURITY_RISK_REGISTER_AND_THREAT_MODEL.md) (**PRESENT**)
- **External Gates Tracker:** [EXTERNAL_GATES_STATUS.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/EXTERNAL_GATES_STATUS.md) (**PRESENT**)
- **Machine-Readable Summary:** [FINAL_ACCEPTANCE_SUMMARY.json](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/FINAL_ACCEPTANCE_SUMMARY.json) (**PRESENT**)
- **Final Pack Manifest:** [FINAL_PACK_MANIFEST_SHA256.json](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/FINAL_PACK_MANIFEST_SHA256.json) (**PRESENT**)
- **Gate 1 — Clean Windows VM Report:** **BLOCKED** (awaiting clean Windows VM/Sandbox host)
- **Gate 2 — Live Android ADB Report:** [GATE2_LIVE_ANDROID_ADB_REPORT.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/GATE2_LIVE_ANDROID_ADB_REPORT.md) (**PASS FOR PLATFORM & ANDROID SETTINGS · GAME WORKFLOW PENDING**)
- **Gate 3 — Commercial Signing Report:** **BLOCKED** (awaiting commercial CA code-signing certificate)
