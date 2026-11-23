# FARMERPRO RC.4 — GATE 1 CLEAN WINDOWS SANDBOX VALIDATION REPORT

**Baseline Under Test (Frozen — Zero Source Modifications):**
- **Code Commit:** `abaabaa5806ac28de546dd1618897fb178badc68` (`tag: v0.1.0-p6-rc.4`)
- **RC.4 Acceptance Docs Commit:** `ec23be9bb984c1ebd1589639dfb8299057d46b83`
- **Overall Gate 1 Verdict:** **`PARTIAL (DEFECT FOUND IN CLEAN WINDOWS)`** — `PRODUCTION_ACCEPTANCE_PASS` remains **`BLOCKED`**

---

## 1. Host & Clean Windows Sandbox Environments

- **Host Machine:** Windows 11 Pro Build `26200`, CPU `11th Gen Intel(R) Core(TM) i9-11950H @ 2.60GHz`, shell `IsAdmin = False`, `C:\Windows\system32\WindowsSandbox.exe` (`1,088,840` bytes) + `wsb.exe` (`MicrosoftWindows.WindowsSandbox 0.8.114.0`) operational without UAC bypass.
- **Sandbox Session 1 (Offline):** [gate1_offline.wsb](file:///e:/DATA/FarmerPro/artifacts/gate1_sandbox/gate1_offline.wsb) (`<Networking>Disable</Networking>`, `4095 MB` RAM, OS `Microsoft Windows 11 Enterprise` Build `26100` `AMD64`). Verified inside Sandbox: `nonLoopbackIpv4 = []`, `hasCargo = false`, `hasRustc = false`, `hasNode = false`, `hasAdbOnPath = false`, `VCRUNTIME140.dll = false` (`System32\ucrtbase.dll = true`), WebView2 pre-installed `157.0.4341.60`.
- **Sandbox Session 2 (Online):** [gate1_online.wsb](file:///e:/DATA/FarmerPro/artifacts/gate1_sandbox/gate1_online.wsb) (`<Networking>Enable</Networking>`, `nonLoopbackIpv4 = ["172.17.238.59"]`).
- **Isolation Controls:** RC.4 source artifacts (`FarmerPro_0.1.0_x64-setup.exe`, `P6_Alpha_v1.0.0_win-x64.zip`, `P6_Beta_v1.0.0_win-x64.zip`, `P6_Beta_v1.1.0_win-x64`, v5 SQLite fixtures) staged cleanly at [rc4_ro_stage](file:///e:/DATA/FarmerPro/artifacts/gate1_sandbox/rc4_ro_stage) and shared **ReadOnly** (`C:\gate1_ro`). Zero production DBs, source directories, or signing keys were exposed to Sandbox. All SHA-256 checksums verified inside Sandbox (`match: true`).

---

## 2. Authoritative Gate 1 Acceptance Checklist

| Criterion | Status | Observed Evidence Inside Windows Sandbox |
|---|---|---|
| `SANDBOX_BOOT_PASS` | **PASS** | Both Offline (`Networking=Disable`) and Online (`Networking=Enable`) Windows 11 Enterprise Build `26100` Sandboxes booted via `wsb.exe` with verified RO/RW mappings and zero host dev tools. |
| `STUDIO_INSTALL_PASS` | **PASS** | `FarmerPro_0.1.0_x64-setup.exe /S` installed cleanly (`ExitCode = 0`) to `%LOCALAPPDATA%\FarmerPro\`; launched `FarmerPro.exe` Offline & Online; created `Gate1 Sandbox RPG` (`proj-gate1-sandbox-rpg-a1d888f2`) via UI modal (`StudioData/projects/gate1-sandbox-rpg`) and reloaded cleanly after restart. *(Observation: Online first launch prompted Windows Defender Firewall dialog for `adb` `Publisher: Unknown`.)* |
| `STANDALONE_PORTABLE_PASS` | **PASS** | Unzipped `P6_Alpha_v1.0.0_win-x64.zip` and `P6_Beta_v1.0.0_win-x64.zip` and ran `Auto_P6Alpha.exe` (`Zenith Harvester Pro v1.0.0`) & `Auto_P6Beta.exe` (`Orion Citadel Automator v1.0.0`) both **before Studio installation** and **after Studio uninstall**. `--check-deps` succeeded (`offlineExecutionReady: true`), UI truthfully reported `ADB: 0 ONLINE (OFFLINE)` / `Chưa có thiết bị kết nối`, and standalone `data\runtime_*.db` (`schemaMigrations = [1..=6]`, `integrity_check = ok`) was created. |
| `WEBVIEW2_ONLINE_PASS` | **PARTIAL** | **Present + Online = PASS** (`157.0.4341.60`). **Missing + Online = `BLOCKED`** in Sandbox 24H2 OS image (`setup.exe --uninstall --msedgewebview --system-level --force-uninstall` returned exit code `93` as `System`; WebView2 is OS-embedded and actively held by `SearchHost.exe`). |
| `WEBVIEW2_OFFLINE_PASS` | **PARTIAL** | **Present + Offline = PASS** (`FarmerPro.exe` + both Mini Tools run offline). **Missing + Offline = `BLOCKED`** in Sandbox 24H2 OS image (exit code `93`). Note by inspection of [tauri.conf.json](file:///e:/DATA/FarmerPro/apps/desktop/src-tauri/tauri.conf.json#L51-L54): installer uses `embedBootstrapper`, so an offline host genuinely lacking WebView2 cannot install WebView2 without internet or separate Evergreen Standalone installer. |
| `STANDARD_USER_PASS` | **BLOCKED** | Created genuine non-admin local user `fpstandard` (`isInAdministratorsGroup: false`), but Windows Sandbox enforces single `WDAGUtilityAccount` interactive desktop (`Start-Process -Credential` returned `-1073741502` / `0xC0000142`; Scheduled Task returned `267011`). Requires standalone clean Windows 10/11 VM supporting interactive Standard User sign-in. |
| `SQLITE_V5_TO_V6_PASS` | **PASS** | Opened 3 genuine RC.2 (`b5a0567`) v5 SQLite fixtures inside Sandbox with installed `FarmerPro.exe`: (1) `rc2_v5_clean.db` (`0/4` cols), (2) `rc2_v5_partial_2cols.db` (`2/4` cols: `confidence_source`, `device_id`), (3) `rc2_v5_partial_3cols.db` (`3/4` cols). All 3 upgraded to `schemaMigrations = [1..=6]` (`18` cols, all 4 provenance columns present, `integrity_check = ok`, `fk_violations = 0`, domain tables preserved, legacy `legacy_g1_ocr` normalized to `LEGACY_UNVERIFIED`, `verified = 0`, `confidence = null`). |
| `UPDATE_ROLLBACK_PASS` | **FAIL** | **Real Clean-Windows Defect Discovered:** `fp-updater.exe install-initial` immediately exited inside Clean Windows Sandbox with **`-1073741515` (`0xC0000135` — `STATUS_DLL_NOT_FOUND`)**. PE import inspection confirms `fp-updater.exe` and `fp-cli.exe` dynamically link `VCRUNTIME140.dll`, which is **absent** on clean Windows 11 Enterprise Build `26100` (`sandboxVcrtCheck.vcruntime140Exists = false`), whereas `farmerpro.exe` and `fp-mini-runtime.exe` have no `VCRUNTIME140.dll` import and launch cleanly. |
| `UNINSTALL_DATA_PRESERVATION_PASS` | **PASS** | `uninstall.exe /S` removed `FarmerPro.exe` and `.nsis_installed` while preserving `StudioData\data\farmerpro.db` byte-for-byte (SHA-256 before/after: `41b81c502c6354f4b38ce0a65dd19c933017a73b7f04b7e3a4e2eece86f2f3ce`), and `Auto_P6Alpha.exe` continued launching cleanly afterward. |

---

## 3. Clean-Windows Defect Root Cause & Proposed Minimal Hotfix (Pending Approval)

1. **Defect (`UPDATE_ROLLBACK_PASS = FAIL` in Clean Windows):**
   - `crates/fp-updater` and `apps/cli` lack `tauri-build` and link `VCRUNTIME140.dll` dynamically under default MSVC toolchain settings, causing `0xC0000135` (`STATUS_DLL_NOT_FOUND`) on clean Windows machines without VC++ Redistributable 2015–2022 (`C:\Windows\System32\VCRUNTIME140.dll = false`).
   - *Verified contrast on same RC.4 build:* `farmerpro.exe` and `fp-mini-runtime.exe` do NOT import `VCRUNTIME140.dll` and succeeded inside the exact same clean Sandbox.
2. **Proposed Minimal Hotfix (Not Applied — Awaiting Auditor/User Approval):**
   - Configure static MSVC CRT linkage (`-C target-feature=+crt-static` for `x86_64-pc-windows-msvc` via `.cargo/config.toml` or crate `build.rs`) so `fp-updater.exe` and `fp-cli.exe` eliminate their `VCRUNTIME140.dll` runtime dependency, verify PE import tables (`0` `VCRUNTIME140.dll` imports across all 4 EXEs), rebuild bundled resources (`fp-mini-runtime.exe`, `fp-updater.exe`, NSIS), and re-run Test F (`v1.0.0 -> v1.1.0 -> rollback -> re-upgrade`) inside Windows Sandbox.
   - *(Secondary observations for Production readiness)*: Consider Evergreen WebView2 offline bundling or explicit preflight wording for truly air-gapped missing-WebView2 machines, and plan Windows Defender Firewall allow/loopback expectations for `bin\adb.exe`.

---

## 4. Gate 1 Evidence Files

- **WSB Configs:** [gate1_offline.wsb](file:///e:/DATA/FarmerPro/artifacts/gate1_sandbox/gate1_offline.wsb) · [gate1_online.wsb](file:///e:/DATA/FarmerPro/artifacts/gate1_sandbox/gate1_online.wsb)
- **Logs & JSON Reports:** [offline_report.json](file:///e:/DATA/FarmerPro/artifacts/gate1_sandbox/evidence_offline/offline_report.json) · [test_f_and_create_report.json](file:///e:/DATA/FarmerPro/artifacts/gate1_sandbox/evidence_offline/test_f_and_create_report.json) · [online_present_wv2.json](file:///e:/DATA/FarmerPro/artifacts/gate1_sandbox/evidence_online/online_present_wv2.json) · [standard_user_report.json](file:///e:/DATA/FarmerPro/artifacts/gate1_sandbox/evidence_offline/standard_user_report.json) · [wv2_uninstall_sys.json](file:///e:/DATA/FarmerPro/artifacts/gate1_sandbox/evidence_offline/wv2_uninstall_sys.json) · [gate1_sqlite_audit_report.json](file:///e:/DATA/FarmerPro/artifacts/gate1_sandbox/gate1_sqlite_audit_report.json)
- **Gate 1 PDF Report:** [FARMERPRO_GATE1_SANDBOX_REPORT.pdf](file:///e:/DATA/FarmerPro/FARMERPRO_GATE1_SANDBOX_REPORT.pdf)
- **Manifest SHA-256:** [GATE1_EVIDENCE_MANIFEST_SHA256.json](file:///e:/DATA/FarmerPro/artifacts/gate1_sandbox/GATE1_EVIDENCE_MANIFEST_SHA256.json)
