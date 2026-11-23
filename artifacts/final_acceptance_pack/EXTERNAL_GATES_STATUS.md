# FARMERPRO — EXTERNAL PRODUCTION ACCEPTANCE GATES STATUS

**Overall Development Gate:** `ACCEPTED WITH CONDITIONS` (P6 Closeout)  
**Overall Production Gate:** `NOT APPROVED (BLOCKED)` — 3 External Gates Remain `BLOCKED`

> No live Android hardware, clean isolated Windows VM without WebView2, or commercial CA code-signing certificate is currently available on the build host. TestKit simulation and Self-Signed QA are **never** promoted to live/production `PASS`.

---

## Gate 1 — Clean Windows VM (`BLOCKED`)
- **Current Status:** `BLOCKED` (Host verification: `WindowsSandbox.exe` = `False`; Hyper-V queries require elevated Administrator privileges).
- **What Is Proven Locally:** NSIS Installer (`currentUser`) install/uninstall preserves SQLite DB (`2` projects, `9` tasks); `ReleasePackageManifest` declares `requiresInternetIfWebview2Missing = true` and `offlineReady`; `apps/mini-runtime` exits cleanly with code `3` (`WEBVIEW2_RUNTIME_MISSING`) when WebView2 is absent.
- **Required Evidence When Provisioned:**
  1. Boot clean Windows 10/11 VM (Standard User, no Rust/Cargo/Node SDK).
  2. Verify Offline Mini Tool launch BOTH with WebView2 present (`offlineExecutionReady = true`) and WebView2 absent (exit code `3` / bootstrapper prompt cleanly reported).
  3. Install `FarmerPro_0.1.0_x64-setup.exe`, export/run both Portable Alpha & Beta ZIPs, upgrade `v1.0.0 -> v1.1.0`, rollback `v1.1.0 -> v1.0.0`, verify independent user data preservation, capture screenshots + logs into `artifacts/final_acceptance_pack/gate1_clean_vm_report.md`.

---

## Gate 2 — Live Android ADB Hardware / Emulator (`BLOCKED`)
- **Current Status:** `BLOCKED` (`adb devices -l` reports `emulator-5554 offline` / `0` online ADB devices; `LDPlayer14 vm0` previously stopped ~10s after launch without opening ADB port `5555`).
- **Required Evidence When Hardware/LDPlayer Online:**
  1. Confirm `adb devices -l` shows `device` (not `offline`/`unauthorized`).
  2. Execute end-to-end live flow via Studio AND standalone Mini Runtime:
     - ADB discovery (`device` state + screen geometry)
     - Real device screenshot capture (before & after screenshots + image paths)
     - Vision template matching / OCR coordinate bounds verification (Safe Random Tap policy)
     - Real Tap / Swipe execution
     - Multi-step workflow execution + Pause / Resume / Cancel controls
     - Realtime variable envelope updates from real device actions (`DEMO_SIMULATION` badge NOT present)
     - Checkpoint persistence & recovery
  3. Save before/after screenshots and raw CLI/runtime logs into `artifacts/final_acceptance_pack/gate2_live_adb_report.md`.

---

## Gate 3 — Commercial Authenticode PublicSigned (`BLOCKED`)
- **Current Status:** `BLOCKED` (`Cert:\CurrentUser\My` holds only Self-Signed QA certificates; `0` Commercial CA code-signing certificates exist on host).
- **What Is Proven Locally:** `SigningMode::PublicSigned` and `release_channel = "production"` enforce WinVerifyTrust (`AUTHENTICODE_PUBLIC_TRUST_REQUIRED` / `AUTHENTICODE_PRODUCTION_TRUST_REQUIRED` fail-closed). Self-Signed QA passes structural/publisher checks under QA mode only (`self_signed_untrusted_root`).
- **Required Evidence When Certificate Installed:**
  1. Sign `FarmerPro_0.1.0_x64-setup.exe`, `Auto_P6Alpha.exe`, `Auto_P6Beta.exe`, and `fp-updater.exe` with commercial CA certificate + RFC 3161 timestamp (`signtool.exe` / Studio signing flow).
  2. Run `WinVerifyTrust` / `Get-AuthenticodeSignature` + `fp-updater verify-security` on clean Windows VM verifying `TrustedPublicChain`.
  3. Update SHA-256 manifest and attach signing report at `artifacts/final_acceptance_pack/gate3_commercial_signing_report.md`.
