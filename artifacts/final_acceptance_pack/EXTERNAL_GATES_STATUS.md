# FARMERPRO — EXTERNAL PRODUCTION ACCEPTANCE GATES STATUS

**Verified RC Code Baseline:** `abaabaa5806ac28de546dd1618897fb178badc68` (Tag: `v0.1.0-p6-rc.4`)  
**Overall Development Gate:** `ACCEPTED_RC4 (P6 + GATE 2 PLATFORM/SETTINGS ADB + CROSS-PROCESS LOCKFILEEX + SQLITE MIGRATION V6)`  
**Overall Production Gate:** `NOT APPROVED (BLOCKED)` — **Gate 1 & Gate 3 Remain `BLOCKED`; Game-Specific Live Workflows `NOT TESTED`**

> RC.4 (`abaabaa`) resolved both RC.3 supplemental blockers: Win32 `LockFileEx` OS-level device locking (`PASS` across `A1`–`A9`, with `A6` dual-device concurrency verified in Rust cross-process tests on 1-AVD host) and atomic `MIGRATION_V6` SQLite upgrade compatibility (`PASS` across `B1`–`B12`). Regression runs of `verify_p6.js`, `verify_gate2_live_adb.js`, and `verify_rc3_closeout_supplement.js` all passed on RC.4. Production release stays **STOPPED** at the RC.4 Stage-Gate until External Gates 1 and 3 are satisfied.

---

## Gate 1 — Clean Windows VM (`BLOCKED`)
- **Current Status:** `BLOCKED` (`WindowsSandbox.exe` = `False`; Hyper-V VM management requires Administrator elevation).

---

## Gate 2 — Live Android ADB (`PASS: Platform & Settings` · `NOT TESTED: Game Workflows`)
- **Platform Live ADB Integration (Re-verified on RC.4 `abaabaa`):** **`PASS`** — Canonical deduplication (`emulator-5554` + `127.0.0.1:5555` -> `LIVE ADB: 1 ONLINE`), `adb forward`/`reverse` create/list/remove, `capture_device`, template match (`0.931`), OCR provenance (`ConfidenceSource::Heuristic`, `measuredConfidence=null`, badge `LIVE · Đã khớp quy tắc`), `Pause/Resume`, `Suspend/Resume Checkpoint`, `Cancel`, and standalone `Auto_Gate2Live.exe` (`22` events, `anyTestkitBadge=false`).
- **Live Workflow Execution (Android Settings `com.android.settings`):** **`PASS`** — Live OCR, `SafeTap`, `Swipe`, `BACK`, and clean before/after device screenshots.
- **Cross-Process Device Lease (`Win32 LockFileEx` on `%LOCALAPPDATA%\FarmerPro\locks\`):** **`PASS`** (`A1`–`A9` verified across `FarmerPro.exe`, `Auto_CrossMini.exe`, `Auto_CrossMini2.exe`, `127.0.0.1:5555` dual-transport, forced kills of Studio & Mini Runtime with crashed SQLite job recovery to `interrupted`, fail-closed on invalid lock dir, and TTL non-override).
- **SQLite v6 Upgrade Compatibility (`MIGRATION_V6`):** **`PASS`** (`B1`–`B12` verified across genuine RC.2 `b5a0567` v5 DB, RC.3 DB, fresh v6 DB, independent Mini Runtime DB upgrade, rollback/backup SHA-256 match, and fail-closed rejection of `v99` / missing-column DBs).
- **Live Game-Specific Workflow Execution:** **`NOT TESTED`** — To be validated per individual GameProject acceptance pack.

---

## Gate 3 — Commercial Authenticode PublicSigned (`BLOCKED`)
- **Current Status:** `BLOCKED` (`Cert:\CurrentUser\My` holds only Self-Signed QA certificates; `0` Commercial CA code-signing certificates exist on host).
