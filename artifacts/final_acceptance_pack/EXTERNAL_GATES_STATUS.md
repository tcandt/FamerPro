# FARMERPRO — EXTERNAL PRODUCTION ACCEPTANCE GATES STATUS

**Verified RC Code Baseline:** `abaabaa5806ac28de546dd1618897fb178badc68` (Tag: `v0.1.0-p6-rc.4`)  
**Overall Development Gate:** `ACCEPTED_RC4 (P6 + GATE 2 PLATFORM/SETTINGS ADB + CROSS-PROCESS LOCKFILEEX + SQLITE MIGRATION V6)`  
**Overall Production Gate:** `NOT APPROVED (BLOCKED)` — **Gate 1 = `PARTIAL (fp-updater VCRUNTIME140.dll FAIL)` · Gate 3 = `BLOCKED` · Game Workflows = `NOT TESTED`**

> Gate 1 Clean Windows Sandbox validation executed directly against frozen RC.4 (`abaabaa`) via Windows 11 Enterprise Build `26100` Sandbox (`wsb.exe`). Studio install/persist/uninstall, Portable Mini Tools standalone launch, and SQLite `v5 -> v6` upgrades (clean + partial 2/4 & 3/4 columns) all passed cleanly; however, Clean Windows testing caught a real defect in `fp-updater.exe` (`0xC0000135` missing `VCRUNTIME140.dll`), while `Standard User` and `Missing WebView2` remain `BLOCKED` due to Sandbox OS image constraints. Per governance, RC.4 code remains untouched pending hotfix approval.

---

## Gate 1 — Clean Windows VM / Sandbox (`PARTIAL — DEFECT FOUND`)
- **Environment:** `WindowsSandbox.exe` / `wsb.exe` (`MicrosoftWindows.WindowsSandbox 0.8.114.0`, Windows 11 Enterprise Build `26100`, `4095 MB`, zero dev tools, `VCRUNTIME140.dll = false`).
- **Passed Inside Clean Sandbox:** `SANDBOX_BOOT_PASS`, `STUDIO_INSTALL_PASS`, `STANDALONE_PORTABLE_PASS`, `SQLITE_V5_TO_V6_PASS` (including `rc2_v5_clean.db`, `rc2_v5_partial_2cols.db`, `rc2_v5_partial_3cols.db`), `UNINSTALL_DATA_PRESERVATION_PASS`.
- **Failed Inside Clean Sandbox:** `UPDATE_ROLLBACK_PASS` = **`FAIL`** — `fp-updater.exe` (and `fp-cli.exe`) dynamically import `VCRUNTIME140.dll`, crashing on clean Windows (`-1073741515` / `0xC0000135`). Minimal CRT-static hotfix proposed for approval.
- **Blocked Inside Sandbox Image:** `STANDARD_USER_PASS` (`BLOCKED`, Sandbox enforces single `WDAGUtilityAccount` interactive session: `0xC0000142` / `267011`) and Missing-WebView2 cells (`BLOCKED`, Sandbox 24H2 image embeds WebView2 `157.0.4341.60` locked at exit code `93`). See [GATE1_SANDBOX_REPORT.md](file:///e:/DATA/FarmerPro/artifacts/final_acceptance_pack/GATE1_SANDBOX_REPORT.md).

---

## Gate 2 — Live Android ADB (`PASS: Platform & Settings` · `NOT TESTED: Game Workflows`)
- **Platform Live ADB Integration (Re-verified on RC.4 `abaabaa`):** **`PASS`** — Canonical deduplication (`emulator-5554` + `127.0.0.1:5555` -> `LIVE ADB: 1 ONLINE`), `adb forward`/`reverse`, `capture_device`, template match (`0.931`), OCR provenance (`ConfidenceSource::Heuristic`, `measuredConfidence=null`, badge `LIVE · Đã khớp quy tắc`), `Pause/Resume`, `Suspend/Resume Checkpoint`, `Cancel`, and standalone `Auto_Gate2Live.exe` (`22` events, `anyTestkitBadge=false`).
- **Live Workflow Execution (Android Settings `com.android.settings`):** **`PASS`** — Live OCR, `SafeTap`, `Swipe`, `BACK`.
- **Cross-Process Device Lease (`Win32 LockFileEx`):** **`PASS`** (`A1`–`A9`; `A6` dual-device verified in Rust cross-process/coordinator tests on 1-AVD host; same-Windows-User `%LOCALAPPDATA%` scope; offset-64 JSON metadata is diagnostic-only and kernel lock state is authoritative).
- **SQLite v6 Upgrade Compatibility (`MIGRATION_V6`):** **`PASS`** (`B1`–`B12` + Gate 1 Sandbox `clean`, `partial_2cols`, `partial_3cols`).
- **Live Game-Specific Workflow Execution:** **`NOT TESTED`** — To be validated per individual GameProject acceptance pack.

---

## Gate 3 — Commercial Authenticode PublicSigned (`BLOCKED`)
- **Current Status:** `BLOCKED` (`Cert:\CurrentUser\My` holds only Self-Signed QA certificates; `0` Commercial CA code-signing certificates exist on host).
