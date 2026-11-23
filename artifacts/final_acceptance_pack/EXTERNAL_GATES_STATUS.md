# FARMERPRO — EXTERNAL PRODUCTION ACCEPTANCE GATES STATUS

**Verified RC Code Baseline:** `629cce236e173f91642684858e44638948773b9e` (Tag: `v0.1.0-p6-rc.3`, frozen untouched)  
**Overall Development Gate:** `ACCEPTED (P6 CLOSEOUT + GATE 2 PLATFORM/SETTINGS ADB; WITH TEST A IN-PROCESS LIMIT & TEST B UPGRADE DEFECT NOTED)`  
**Overall Production Gate:** `NOT APPROVED (BLOCKED)` — **Gate 1 & Gate 3 Remain `BLOCKED`; Game-Specific Live Workflows `NOT TESTED`**

> Gate 2 (`Platform Live Android ADB` & `Android System Settings Workflow`) is `PASS` on Android SDK WHPX emulator (`fp_gate2_avd` / `emulator-5554`). Supplemental RC.3 checks ([RC3_SUPPLEMENTAL_VERIFICATION_REPORT.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/RC3_SUPPLEMENTAL_VERIFICATION_REPORT.md)) verified `CROSS_PROCESS_LEASE = NOT VERIFIED (IN_PROCESS_ONLY)` and `SQLITE_V5_UPGRADE_COMPATIBILITY = FAIL` (pre-existing RC.2 v5 DB triggers `no such column: confidence_source` because `migrations.rs:552-559` returns early before adding RC.3 columns). Game-specific live execution remains `NOT TESTED`, and Gate 1 (`CLEAN_WINDOWS_VM`) + Gate 3 (`COMMERCIAL_AUTHENTICODE`) remain `BLOCKED`.

---

## Gate 1 — Clean Windows VM (`BLOCKED`)
- **Current Status:** `BLOCKED` (`WindowsSandbox.exe` = `False`; Hyper-V VM management requires Administrator elevation).

---

## Gate 2 — Live Android ADB (`PASS: Platform & Settings` · `NOT TESTED: Game Workflows`)
- **Platform Live ADB Integration:** **`PASS`** — Canonical deduplication (`emulator-5554` + `127.0.0.1:5555` → `LIVE ADB: 1 ONLINE`), `MultiDeviceScheduler` canonical lease locking (in-process), `capture_device`, template match (`0.931`), `adb forward`/`reverse` create/list/remove + parser/cleanup regressions, OCR provenance (`ConfidenceSource::Heuristic`, `measuredConfidence=null`, badge `LIVE · Đã khớp quy tắc`), `Pause/Resume`, `Suspend/Resume Checkpoint`, `Cancel`, and standalone `Auto_Gate2Live.exe` (`22` SQLite events, `anyTestkitBadge=false`).
- **Live Workflow Execution (Android Settings `com.android.settings`):** **`PASS`** — Live OCR, `SafeTap`, `Swipe`, `BACK`, and clean before/after device screenshots ([GATE2_LIVE_ANDROID_ADB_REPORT.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/GATE2_LIVE_ANDROID_ADB_REPORT.md)).
- **Cross-Process Device Lease (`FarmerPro.exe` vs `Auto_CrossMini.exe`):** **`NOT VERIFIED (IN-PROCESS ONLY)`** — Competing EXE succeeded on `emulator-5554` while Studio Job A held in-process lease; minimal Win32 lockfile remediation proposed.
- **SQLite v5 Upgrade Compatibility (RC.2 DB → RC.3 Binary):** **`FAIL` on `629cce2`** — Pre-existing data preserved (`integrity_check=ok`, `fk=0`), but `migrations.rs:552-559` early-return fast-path skips adding `confidence_source`, `measured_confidence`, `rule_matched`, `device_id` on pre-existing v5 DBs; `MIGRATION_V6` patch proposed prior to Gate 1.
- **Live Game-Specific Workflow Execution:** **`NOT TESTED`** — To be validated per individual GameProject acceptance pack.

---

## Gate 3 — Commercial Authenticode PublicSigned (`BLOCKED`)
- **Current Status:** `BLOCKED` (`Cert:\CurrentUser\My` holds only Self-Signed QA certificates; `0` Commercial CA code-signing certificates exist on host).
