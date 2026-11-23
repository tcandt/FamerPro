# FARMERPRO — FINAL ACCEPTANCE PACK & STAGE-GATE MATRIX

**Branch:** `feat/p6-final-validation`  
**Verified Code HEAD:** `802f3162df7fe3ab8fd6c7085c9d9e34caca6648` (`802f316`)  
**Decision:** **P6 Development Closeout = ACCEPTED WITH CONDITIONS · Production Release = NOT APPROVED (`BLOCKED`)**

---

## 1. Authoritative Development Gate Statuses (Section I)
| Gate Identifier | Status | Evidence Scope & Honest Boundaries |
|---|---|---|
| `BUILD_PASS` | **PASS** | `99 / 99` Cargo workspace tests PASS (`cargo test --workspace`), release binaries (`fp-updater`, `fp-mini-runtime`, NSIS `FarmerPro_0.1.0_x64-setup.exe`) built cleanly on `802f316`. |
| `STANDALONE_LAUNCH_PASS` | **PASS** | Both Portable Mini Tools (`Auto_P6Alpha.exe` 740×660 compact Unsigned, `Auto_P6Beta.exe` 980×720 expanded Self-Signed QA) launched after full Studio uninstall, displaying 4–5 tasks and TestKit runtime columns (`overflowX=false`, `consoleErrors=[]`). |
| `UPDATE_ROLLBACK_PASS` | **PASS** | Upgrade `v1.0.0 -> v1.1.0`, WAL backup & post-update SQLite data preservation, binary rollback `v1.1.0 -> v1.0.0`, legitimate re-upgrade `v1.0.0 -> v1.1.0` with trusted key, revoked-key/policy/compat regression gates, real NTFS `icacls (RX)` fallback, Win32 `atomic_write_file` retry exhaustion & journal reconciliation PASS. *Note: Full 6-component same-user overwrite resets `trustEpoch` to `1` per Threat Model.* |
| `LIVE_WORKFLOW_EXECUTION_PASS` | **BLOCKED** | `0` online ADB devices (`adb devices -l` empty / `LDPlayer14 vm0 = stop`, stopped ~10s without opening port `5555`). TestKit simulation logs (`17`/`21` events labeled `[TESTKIT · Mô phỏng]`) do NOT satisfy live gameplay acceptance. |
| `PRODUCTION_ACCEPTANCE_PASS` | **BLOCKED** | Blocked by 3 External Acceptance Gates: (1) Clean Windows VM, (2) Live Android Hardware ADB, (3) Commercial Authenticode PublicSigned Certificate. |

---

## 2. Release Artifact Provenance & Checksums (`802f316`)
| Artifact | Location on Disk (Excluded from Git Blobs) | Size (Bytes) | SHA-256 |
|---|---|---|---|
| `FarmerPro_0.1.0_x64-setup.exe` | `artifacts/p6_evidence/dist_releases/FarmerPro_0.1.0_x64-setup.exe` | `15,172,294` | `87984447ec5475c391b44715e93757e1178e6d128f736d26cd7be8a4604468c3` |
| `P6_Alpha_v1.0.0_win-x64.zip` | `artifacts/p6_evidence/dist_releases/P6_Alpha_v1.0.0_win-x64.zip` | `11,911,813` | `19de97cbabe0cc82110e57851f641105dd7922213153ffe680a4d920373123ef` |
| `P6_Beta_v1.0.0_win-x64.zip` | `artifacts/p6_evidence/dist_releases/P6_Beta_v1.0.0_win-x64.zip` | `11,913,469` | `4879dc71e4c4c984d56e379fec80389a38985c0309e1092d694ee6b804bce2aa` |
| `FARMERPRO_P6_ACCEPTANCE_REPORT.pdf` | `FARMERPRO_P6_ACCEPTANCE_REPORT.pdf` | `708,938` | `87d46378b1d1d915182d0ef4bd8808e66f434fc99bd548997333cd43eb8e67c6` |
| `EVIDENCE_MANIFEST_SHA256.json` | `artifacts/p6_evidence/EVIDENCE_MANIFEST_SHA256.json` | `64 files` | Updated cleanly from `802f316` E2E run |

---

## 3. Acceptance Pack Checklist (Section V)
- **Final Acceptance Matrix:** `artifacts/final_acceptance_pack/ACCEPTANCE_MATRIX.md` (**PRESENT — PASS/BLOCKED**)
- **Security Risk Register & Threat Model:** `artifacts/final_acceptance_pack/SECURITY_RISK_REGISTER_AND_THREAT_MODEL.md` (**PRESENT**)
- **External Gates Tracker:** `artifacts/final_acceptance_pack/EXTERNAL_GATES_STATUS.md` (**PRESENT**)
- **Machine-Readable Summary:** `artifacts/final_acceptance_pack/FINAL_ACCEPTANCE_SUMMARY.json` (**PRESENT**)
- **Clean Windows VM Report:** **BLOCKED** (Gate 1 — awaiting clean Windows VM/Sandbox host)
- **Live ADB Report:** **BLOCKED** (Gate 2 — awaiting online Android device or LDPlayer ADB)
- **Commercial Signing Report:** **BLOCKED** (Gate 3 — awaiting commercial CA code-signing certificate)
