# FARMERPRO — EXTERNAL PRODUCTION ACCEPTANCE GATES STATUS

**Verified RC Code Baseline:** `b5a05673b1c0ad10dd4c6ce4463eaabbd3be40fe` (Tag: `v0.1.0-p6-rc.2`, Gate 2 hotfix over `v0.1.0-p6-rc.1` `802f316`)  
**Overall Development Gate:** `APPROVED (P6 CLOSEOUT + GATE 2 PLATFORM/SETTINGS ADB)`  
**Overall Production Gate:** `NOT APPROVED (BLOCKED)` — **Gate 1 & Gate 3 Remain `BLOCKED`; Game-Specific Live Workflows `NOT TESTED`**

> Gate 2 (`Platform Live Android ADB` & `Android System Settings Workflow`) is `PASS` on Android SDK WHPX emulator (`fp_gate2_avd` / `emulator-5554`). Game-specific live execution remains `NOT TESTED / PENDING`, and Gate 1 (Clean Windows VM) + Gate 3 (Commercial Authenticode CA) remain `BLOCKED`. Full reconciliation details: [GATE2_ACCEPTANCE_ADDENDUM.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/GATE2_ACCEPTANCE_ADDENDUM.md).

---

## Gate 1 — Clean Windows VM (`BLOCKED`)
- **Current Status:** `BLOCKED` (`WindowsSandbox.exe` = `False`; Hyper-V VM management requires Administrator elevation).
- **What Is Proven Locally:** NSIS Installer (`currentUser`) install/uninstall preserves SQLite DB (`2` projects, `9` tasks); `ReleasePackageManifest` declares `requiresInternetIfWebview2Missing = true` and `offlineReady`; `apps/mini-runtime` exits cleanly with code `3` (`WEBVIEW2_RUNTIME_MISSING`) when WebView2 is absent.

---

## Gate 2 — Live Android ADB (`PASS: Platform & Settings` · `PENDING: Game Workflows`)
- **Platform Live ADB Integration:** **`PASS`** — Canonical deduplication (`emulator-5554` + `127.0.0.1:5555` → `LIVE ADB: 1 ONLINE`), `MultiDeviceScheduler` canonical lease locking, `capture_device`, template crop/match (`0.931`), `adb forward` (`tcp:19555 -> tcp:19556`) & `adb reverse` (`tcp:19557 -> tcp:19558`) create/list/remove, `Pause/Resume`, `Suspend/Resume Checkpoint`, `Cancel`, and standalone `Auto_Gate2Live.exe` (`anyTestkitBadge = false`, `22` SQLite events).
- **Live Workflow Execution (Android Settings `com.android.settings`):** **`PASS`** — Executed live OCR (`"11:27 Q Search Settings..."`, `verified = true`), `SafeTap`, `Swipe`, `BACK`, with verified before/after device screenshots ([GATE2_LIVE_ANDROID_ADB_REPORT.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/GATE2_LIVE_ANDROID_ADB_REPORT.md)).
- **Live Game-Specific Workflow Execution (JX2 Mobile / others):** **`NOT TESTED / PENDING`** — No game APK workflow run was executed.

---

## Gate 3 — Commercial Authenticode PublicSigned (`BLOCKED`)
- **Current Status:** `BLOCKED` (`Cert:\CurrentUser\My` holds only Self-Signed QA certificates; `0` Commercial CA code-signing certificates exist on host).
- **What Is Proven Locally:** `SigningMode::PublicSigned` and `release_channel = "production"` enforce WinVerifyTrust fail-closed (`AUTHENTICODE_PUBLIC_TRUST_REQUIRED` / `AUTHENTICODE_PRODUCTION_TRUST_REQUIRED`). Self-Signed QA passes QA checks only (`self_signed_untrusted_root`).
