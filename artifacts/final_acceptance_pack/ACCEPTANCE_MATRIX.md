# FARMERPRO — FINAL ACCEPTANCE PACK & STAGE-GATE MATRIX

**Branch:** `feat/p6-final-validation`  
**Frozen RC Code Baseline:** `802f3162df7fe3ab8fd6c7085c9d9e34caca6648` (Git Tag: `v0.1.0-p6-rc.1`)  
**Decision:** **P6 Development Closeout = APPROVED · Gate 2 Live ADB = PASS · Production Release = NOT APPROVED (`BLOCKED` at Gate 1 & Gate 3)**

---

## 1. Authoritative Development & Gate Statuses
| Gate Identifier | Status | Evidence Scope & Honest Boundaries |
|---|---|---|
| `BUILD_PASS` | **PASS** | `99 / 99` Cargo workspace tests PASS (`cargo test --workspace`), frozen RC binaries (`fp-updater`, `fp-mini-runtime`, NSIS `FarmerPro_0.1.0_x64-setup.exe`) built cleanly on `802f316` and tagged `v0.1.0-p6-rc.1`. |
| `STANDALONE_LAUNCH_PASS` | **PASS** | Both Portable Mini Tools (`Auto_P6Alpha.exe` compact Unsigned, `Auto_P6Beta.exe` expanded Self-Signed QA) launched after Studio uninstall; plus `Auto_Gate2Live.exe` launched standalone against live ADB (`LIVE ADB: 2 ONLINE`). |
| `UPDATE_ROLLBACK_PASS` | **PASS** | Upgrade `v1.0.0 -> v1.1.0`, WAL backup & post-update SQLite data preservation, binary rollback `v1.1.0 -> v1.0.0`, legitimate re-upgrade `v1.0.0 -> v1.1.0` with trusted key, revoked-key/policy/compat regression gates, real NTFS `icacls (RX)` fallback, Win32 `atomic_write_file` retry exhaustion & journal reconciliation PASS. *Full 6-component same-user overwrite resets `trustEpoch` to `1` per Threat Model.* |
| `LIVE_WORKFLOW_EXECUTION_PASS` | **PASS (Gate 2)** | Executed on live Android SDK AVD (`fp_gate2_avd` / `emulator-5554`, WHPX `1280×720`) via **both** frozen `FarmerPro.exe` (`capture_device`, template crop & match `0.931`, `vision.ocr`, `random_safe_tap`, `swipe`, `Pause/Resume`, `Suspend/Resume Checkpoint`, `Cancel`) and standalone `Auto_Gate2Live.exe` (`anyTestkitBadge=false`, `LIVE_ADB_OCR`/`LIVE_STATE`, `22` SQLite events). Full report: [GATE2_LIVE_ANDROID_ADB_REPORT.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/GATE2_LIVE_ANDROID_ADB_REPORT.md). |
| `PRODUCTION_ACCEPTANCE_PASS` | **BLOCKED** | **2 External Gates Remain `BLOCKED`:** Gate 1 (Clean Windows VM / Standard User / WebView2 missing) and Gate 3 (Commercial Authenticode PublicSigned CA Certificate). |

---

## 2. Release Artifact Provenance & Checksums (`802f316` / Tag `v0.1.0-p6-rc.1`)
| Artifact | Location on Disk (Excluded from Git Blobs) | Size (Bytes) | SHA-256 |
|---|---|---|---|
| `FarmerPro_0.1.0_x64-setup.exe` | `artifacts/p6_evidence/dist_releases/FarmerPro_0.1.0_x64-setup.exe` | `15,172,294` | `87984447ec5475c391b44715e93757e1178e6d128f736d26cd7be8a4604468c3` |
| `P6_Alpha_v1.0.0_win-x64.zip` | `artifacts/p6_evidence/dist_releases/P6_Alpha_v1.0.0_win-x64.zip` | `11,911,813` | `19de97cbabe0cc82110e57851f641105dd7922213153ffe680a4d920373123ef` |
| `P6_Beta_v1.0.0_win-x64.zip` | `artifacts/p6_evidence/dist_releases/P6_Beta_v1.0.0_win-x64.zip` | `11,913,469` | `4879dc71e4c4c984d56e379fec80389a38985c0309e1092d694ee6b804bce2aa` |
| `FARMERPRO_P6_ACCEPTANCE_REPORT.pdf` | [FARMERPRO_P6_ACCEPTANCE_REPORT.pdf](file:///e:/DATA/FarmerPro/FARMERPRO_P6_ACCEPTANCE_REPORT.pdf) | `708,938` | `87d46378b1d1d915182d0ef4bd8808e66f434fc99bd548997333cd43eb8e67c6` |
| `EVIDENCE_MANIFEST_SHA256.json` | [EVIDENCE_MANIFEST_SHA256.json](file:///e:/DATA/FarmerPro/artifacts/p6_evidence/EVIDENCE_MANIFEST_SHA256.json) | `64 files` | P6 Development Evidence SHA-256 Manifest |

---

## 3. Acceptance Pack Checklist (Section V)
- **Final Acceptance Matrix:** [ACCEPTANCE_MATRIX.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/ACCEPTANCE_MATRIX.md) (**PRESENT — PASS/BLOCKED**)
- **Security Risk Register & Threat Model:** [SECURITY_RISK_REGISTER_AND_THREAT_MODEL.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/SECURITY_RISK_REGISTER_AND_THREAT_MODEL.md) (**PRESENT**)
- **External Gates Tracker:** [EXTERNAL_GATES_STATUS.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/EXTERNAL_GATES_STATUS.md) (**PRESENT**)
- **Machine-Readable Summary:** [FINAL_ACCEPTANCE_SUMMARY.json](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/FINAL_ACCEPTANCE_SUMMARY.json) (**PRESENT**)
- **Gate 1 — Clean Windows VM Report:** **BLOCKED** (awaiting clean Windows VM/Sandbox host)
- **Gate 2 — Live Android ADB Report:** [GATE2_LIVE_ANDROID_ADB_REPORT.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/GATE2_LIVE_ANDROID_ADB_REPORT.md) (**PASS — VERIFIED WITH STUDIO + MINI TOOL + SCREENSHOTS + LOGS**)
- **Gate 3 — Commercial Signing Report:** **BLOCKED** (awaiting commercial CA code-signing certificate)
