# FARMERPRO — EXTERNAL PRODUCTION ACCEPTANCE GATES STATUS

**Frozen RC Code Baseline:** `802f3162df7fe3ab8fd6c7085c9d9e34caca6648` (Tag: `v0.1.0-p6-rc.1`)  
**Overall Development Gate:** `ACCEPTED WITH CONDITIONS` (P6 Closeout Approved)  
**Overall Production Gate:** `NOT APPROVED (BLOCKED)` — **Gate 1 & Gate 3 Remain `BLOCKED`; Gate 2 Now `PASS`**

> Gate 2 (`Live Android ADB`) has now been verified end-to-end on a live Android SDK WHPX emulator (`fp_gate2_avd` / `emulator-5554`). Gate 1 (Clean Windows VM) and Gate 3 (Commercial Authenticode CA) remain `BLOCKED` due to missing external host infrastructure / commercial certificate.

---

## Gate 1 — Clean Windows VM (`BLOCKED`)
- **Current Status:** `BLOCKED` (Host verification: `WindowsSandbox.exe` = `False`; Hyper-V VM management requires Administrator elevation).
- **What Is Proven Locally:** NSIS Installer (`currentUser`) install/uninstall preserves SQLite DB (`2` projects, `9` tasks); `ReleasePackageManifest` declares `requiresInternetIfWebview2Missing = true` and `offlineReady`; `apps/mini-runtime` exits cleanly with code `3` (`WEBVIEW2_RUNTIME_MISSING`) when WebView2 is absent.
- **Required Evidence When Provisioned:** Clean Windows 10/11 VM (Standard User, no dev SDK), WebView2 present & absent, online/offline launch, installer/uninstaller, both Portable ZIPs, update/rollback & data isolation report at `artifacts/final_acceptance_pack/gate1_clean_vm_report.md`.

---

## Gate 2 — Live Android ADB Hardware / Emulator (`PASS`)
- **Current Status:** **`PASS (LIVE_ANDROID_ADB_VERIFIED)`** — Full report: [GATE2_LIVE_ANDROID_ADB_REPORT.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/GATE2_LIVE_ANDROID_ADB_REPORT.md)
- **LDPlayer Root Cause Diagnosis:** Diagnosed `C:\LDPlayer\LDPlayer9\vms\leidian0\Logs\VBox.log`: `HM: HMR3Init: Attempting fall back to NEM: VT-x is not available` while WHv is active, leading to `Heartbeat flatlined` / `PDMR3PowerOff` at `00:01:34` (in addition to shell JobObject termination when headless subshells exit). Switched cleanly to Android SDK Emulator (`fp_gate2_avd`, `1280×720`, `WHPX 10.0.26200 usable`) per Gate 2 directive.
- **Verified Live Evidence (`802f316` Frozen Binaries):**
  - **Studio (`FarmerPro.exe`):** Discovery (`emulator-5554` Ready), live `capture_device` + crop, template match (`0.931`), `vision.ocr` (`Search`), `action.random_safe_tap`, `action.swipe`, `Pause/Resume`, `Suspend to Checkpoint / Resume from Checkpoint`, `Cancel` (`cancelled`), `24` SQLite step events.
  - **Independent Mini Tool (`Auto_Gate2Live.exe`, Studio closed):** `LIVE ADB: 2 ONLINE`, executed via `▶ CHẠY NHIỆM VỤ (ADB THẬT)`, `anyTestkitBadge = false`, `LIVE_ADB_OCR` (`confidence = 0.94`, `verified = true`) & `LIVE_STATE` realtime deltas, `22` SQLite step events, before/after device & UI screenshots archived.

---

## Gate 3 — Commercial Authenticode PublicSigned (`BLOCKED`)
- **Current Status:** `BLOCKED` (`Cert:\CurrentUser\My` holds only Self-Signed QA certificates; `0` Commercial CA code-signing certificates exist on host).
- **What Is Proven Locally:** `SigningMode::PublicSigned` and `release_channel = "production"` enforce WinVerifyTrust fail-closed (`AUTHENTICODE_PUBLIC_TRUST_REQUIRED` / `AUTHENTICODE_PRODUCTION_TRUST_REQUIRED`). Self-Signed QA passes QA checks only (`self_signed_untrusted_root`) and never substitutes for `PublicSigned`.
- **Required Evidence When Certificate Installed:** Sign `FarmerPro_0.1.0_x64-setup.exe`, `Auto_P6Alpha.exe`, `Auto_P6Beta.exe`, and `fp-updater.exe` with commercial CA certificate + RFC 3161 timestamp, verify `TrustedPublicChain` on clean Windows VM, and archive `artifacts/final_acceptance_pack/gate3_commercial_signing_report.md`.
