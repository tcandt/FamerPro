# FARMERPRO — FINAL ACCEPTANCE PACK & STAGE-GATE MATRIX

**Branch:** `feat/p6-final-validation`  
**Verified RC Code Baseline:** `629cce236e173f91642684858e44638948773b9e` (Git Tag: `v0.1.0-p6-rc.3`, frozen untouched)  
**Decision:** **RC.3 Development & Gate 2 Platform Live ADB = ACCEPTED · Supplemental Checks Found Cross-Process Lease Scope Limit & Pre-Existing v5 DB Upgrade Bug · Production Release = `BLOCKED`**

---

## 1. Authoritative Development & Gate Statuses
| Gate Identifier | Status | Evidence Scope & Honest Boundaries |
|---|---|---|
| `BUILD_PASS` | **PASS** | `101 / 101` Cargo workspace tests PASS across `38` suites ([cargo_test_workspace_p6.log](file:///e:/DATA/FarmerPro/artifacts/p6_evidence/cargo_test_workspace_p6.log)), release binaries (`fp-updater`, `fp-mini-runtime`, NSIS `FarmerPro_0.1.0_x64-setup.exe`) built cleanly on `629cce2` (`v0.1.0-p6-rc.3`). |
| `STANDALONE_LAUNCH_PASS` | **PASS** | Portable Mini Tools (`Auto_P6Alpha.exe`, `Auto_P6Beta.exe`) launched after Studio uninstall; plus `Auto_Gate2Live.exe` launched standalone against live ADB (`LIVE ADB: 1 ONLINE` on `emulator-5554`, OCR badge `LIVE · Đã khớp quy tắc`). |
| `UPDATE_ROLLBACK_PASS` | **PASS** | Upgrade `v1.0.0 -> v1.1.0`, WAL backup & data preservation, rollback `v1.1.0 -> v1.0.0`, re-upgrade `v1.0.0 -> v1.1.0` with trusted key, revoked-key/policy/compat gates, real NTFS `icacls (RX)` fallback, Win32 `atomic_write_file` retry exhaustion & journal reconciliation PASS. *Full 6-component same-user overwrite resets `trustEpoch` to `1` per Threat Model.* |
| `GATE2_PLATFORM_LIVE_ADB` | **PASS** | Canonical single-AVD deduplication + 7 canonical regression tests + Mini Runtime canonical scheduler lease, `adb forward`/`reverse` create/list/remove & parser regressions, `capture_device`, template match (`0.931`), OCR provenance (`ConfidenceSource::Heuristic`, `measuredConfidence=null`, badge `LIVE · Đã khớp quy tắc`), `Pause/Resume`, `Suspend/Resume Checkpoint`, `Cancel`, and standalone `Auto_Gate2Live.exe` (`22` events). See [GATE2_ACCEPTANCE_ADDENDUM.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/GATE2_ACCEPTANCE_ADDENDUM.md). |
| `GATE2_SETTINGS_LIVE_WORKFLOW` | **PASS** | Executed end-to-end on Android SDK AVD (`fp_gate2_avd` / `emulator-5554`, API 37 WHPX) against `com.android.settings` with real `Windows.Media.Ocr` text extraction, `SafeTap`, `Swipe`, `BACK`, and clean before/after device screenshots. |
| `CROSS_PROCESS_LEASE` | **NOT VERIFIED (IN-PROCESS ONLY)** | Live test ([RC3_SUPPLEMENTAL_VERIFICATION_REPORT.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/RC3_SUPPLEMENTAL_VERIFICATION_REPORT.md)) confirmed same-process concurrent job is rejected in Studio, **but** separate `Auto_CrossMini.exe` Job B ran to `Succeeded` on `emulator-5554` while `FarmerPro.exe` Job A was `running` (in-process RAM scheduler + separate DBs). Minimal OS lockfile remediation proposed. |
| `SQLITE_V5_UPGRADE_COMPATIBILITY` | **FAIL (ON `629cce2`)** | Opening a genuine RC.2 (`b5a0567`) v5 DB with frozen RC.3 `FarmerPro.exe` kept existing projects/workflows/accounts intact (`integrity_check=ok`, `fk=0`), **but** `migrations.rs:552-559` early-return fast-path skipped adding `confidence_source`, `measured_confidence`, `rule_matched`, `device_id`, causing `no such column: confidence_source` on `studio_get_project_runtime_vars`. Requires versioned `v6` migration before upgrading pre-RC.3 DBs. |
| `GATE2_GAME_SPECIFIC_WORKFLOW` | **NOT TESTED** | **0 game APKs / game-specific workflows tested on live ADB** (accepted per GameProject acceptance later, not merged into platform Gate 2). |
| `PRODUCTION_ACCEPTANCE_PASS` | **BLOCKED** | **External Gates Remain `BLOCKED`:** Gate 1 (`CLEAN_WINDOWS_VM = BLOCKED`) and Gate 3 (`COMMERCIAL_AUTHENTICODE = BLOCKED`). |

---

## 2. Release Artifact Provenance & Checksums (`629cce2` / Tag `v0.1.0-p6-rc.3`)
| Artifact | Location on Disk (Excluded from Git Blobs) | Size (Bytes) | SHA-256 |
|---|---|---|---|
| `FarmerPro_0.1.0_x64-setup.exe` | `artifacts/p6_evidence/dist_releases/FarmerPro_0.1.0_x64-setup.exe` | `15,268,397` | `ab36da44ccdd50f33994bf9fc88f2b81635e5747bdd2dcc154bcc6bc4b57d7f1` |
| `P6_Alpha_v1.0.0_win-x64.zip` | `artifacts/p6_evidence/dist_releases/P6_Alpha_v1.0.0_win-x64.zip` | `11,971,510` | `e94279949710eca5baf0bd0ced6bd58fd92227fd2b8af6df60313bc8021b8339` |
| `P6_Beta_v1.0.0_win-x64.zip` | `artifacts/p6_evidence/dist_releases/P6_Beta_v1.0.0_win-x64.zip` | `11,973,272` | `99512f43ddc10ab8cdcc55171929e230042a58b5a831f4b30e82c1bbacb24771` |
| `Gate2_LiveMini_v1.0.0_win-x64.zip` | `Gate 2 Transient Package (SHA-256 recorded)` | `11,987,054` | `2de448a5e263cbbc84b5aa16acfa4a13ce0e78f0c989d8150f3d5cfcfb1aaa73` |
| `FARMERPRO_P6_ACCEPTANCE_REPORT.pdf` | [FARMERPRO_P6_ACCEPTANCE_REPORT.pdf](file:///e:/DATA/FarmerPro/FARMERPRO_P6_ACCEPTANCE_REPORT.pdf) | `712,963` | `ed6e66a930df0b730c56724a46d70555337bf5fff9457c68c9503b48c16c8836` |
| `EVIDENCE_MANIFEST_SHA256.json` | [EVIDENCE_MANIFEST_SHA256.json](file:///e:/DATA/FarmerPro/artifacts/p6_evidence/EVIDENCE_MANIFEST_SHA256.json) | `64 files` | P6 Development Evidence SHA-256 Manifest |

*(Supersedes pre-Gate-2 baseline `v0.1.0-p6-rc.1` `802f316` and intermediate `v0.1.0-p6-rc.2` `b5a0567`.)*

---

## 3. Acceptance Pack Checklist
- **RC.3 Supplemental Verification Report (Tests A & B):** [RC3_SUPPLEMENTAL_VERIFICATION_REPORT.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/RC3_SUPPLEMENTAL_VERIFICATION_REPORT.md) (**PRESENT — Test A NOT VERIFIED / Test B FAIL DOCUMENTED**)
- **Gate 2 Acceptance Addendum:** [GATE2_ACCEPTANCE_ADDENDUM.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/GATE2_ACCEPTANCE_ADDENDUM.md) (**PRESENT**)
- **Final Acceptance Matrix:** [ACCEPTANCE_MATRIX.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/ACCEPTANCE_MATRIX.md) (**PRESENT**)
- **Security Risk Register & Threat Model:** [SECURITY_RISK_REGISTER_AND_THREAT_MODEL.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/SECURITY_RISK_REGISTER_AND_THREAT_MODEL.md) (**PRESENT**)
- **External Gates Tracker:** [EXTERNAL_GATES_STATUS.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/EXTERNAL_GATES_STATUS.md) (**PRESENT**)
- **Machine-Readable Summary:** [FINAL_ACCEPTANCE_SUMMARY.json](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/FINAL_ACCEPTANCE_SUMMARY.json) (**PRESENT**)
- **Final Pack Manifest:** [FINAL_PACK_MANIFEST_SHA256.json](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/FINAL_PACK_MANIFEST_SHA256.json) (**PRESENT**)
- **Gate 1 — Clean Windows VM Report:** **BLOCKED**
- **Gate 2 — Live Android ADB Report:** [GATE2_LIVE_ANDROID_ADB_REPORT.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/GATE2_LIVE_ANDROID_ADB_REPORT.md) (**PASS FOR PLATFORM & ANDROID SETTINGS · GAME WORKFLOW NOT TESTED**)
- **Gate 3 — Commercial Signing Report:** **BLOCKED**
