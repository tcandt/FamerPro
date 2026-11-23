# FARMERPRO — EXTERNAL PRODUCTION ACCEPTANCE GATES STATUS

**Verified RC Code Baseline:** `629cce236e173f91642684858e44638948773b9e` (Tag: `v0.1.0-p6-rc.3`)  
**Overall Development Gate:** `APPROVED (P6 CLOSEOUT + GATE 2 PLATFORM/SETTINGS ADB)`  
**Overall Production Gate:** `NOT APPROVED (BLOCKED)` — **Gate 1 & Gate 3 Remain `BLOCKED`; Game-Specific Live Workflows `NOT TESTED`**

> Gate 2 (`Platform Live Android ADB` & `Android System Settings Workflow`) is `PASS` on Android SDK WHPX emulator (`fp_gate2_avd` / `emulator-5554`), superseding pre-Gate-2 `RC1` (`802f316`) and intermediate `RC2` (`b5a0567`). Game-specific live execution remains `NOT TESTED`, and Gate 1 (Clean Windows VM) + Gate 3 (Commercial Authenticode CA) remain `BLOCKED`. Full details: [GATE2_ACCEPTANCE_ADDENDUM.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/GATE2_ACCEPTANCE_ADDENDUM.md).

---

## Gate 1 — Clean Windows VM (`BLOCKED`)
- **Current Status:** `BLOCKED` (`WindowsSandbox.exe` = `False`; Hyper-V VM management requires Administrator elevation).

---

## Gate 2 — Live Android ADB (`PASS: Platform & Settings` · `NOT TESTED: Game Workflows`)
- **Platform Live ADB Integration:** **`PASS`** — Canonical deduplication (`emulator-5554` + `127.0.0.1:5555` → `LIVE ADB: 1 ONLINE`), `MultiDeviceScheduler` canonical lease locking (shared across Studio & Mini Runtime + 7 regression scenarios), `capture_device`, template match (`0.931`), `adb forward`/`reverse` create/list/remove + parser/cleanup regressions, OCR provenance (`ConfidenceSource::Heuristic`, `measuredConfidence=null`, badge `LIVE · Đã khớp quy tắc`), `Pause/Resume`, `Suspend/Resume Checkpoint`, `Cancel`, and standalone `Auto_Gate2Live.exe` (`22` SQLite events, `anyTestkitBadge=false`).
- **Live Workflow Execution (Android Settings `com.android.settings`):** **`PASS`** — Live OCR, `SafeTap`, `Swipe`, `BACK`, and clean before/after device screenshots ([GATE2_LIVE_ANDROID_ADB_REPORT.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/GATE2_LIVE_ANDROID_ADB_REPORT.md)).
- **Live Game-Specific Workflow Execution:** **`NOT TESTED`** — To be validated per individual GameProject acceptance pack.

---

## Gate 3 — Commercial Authenticode PublicSigned (`BLOCKED`)
- **Current Status:** `BLOCKED` (`Cert:\CurrentUser\My` holds only Self-Signed QA certificates; `0` Commercial CA code-signing certificates exist on host).
