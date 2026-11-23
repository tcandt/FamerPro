# FARMERPRO — SECURITY RISK REGISTER & SAME-USER THREAT MODEL

**Scope:** Mini Tool Packaging, Updater Trust Chain, NTFS Permissions & Windows Atomic I/O  
**Rule:** Never claim `TAMPER_PROOF` or `ABSOLUTE_ROLLBACK_PROTECTION` under same-user Windows ACLs.

---

## 1. Official Threat Model & Trust Boundary (`ExternalTrustAnchor` + `.updater`)

| Scenario / Capability | Protected? | Observed Evidence & Error Codes | Boundary Note |
|---|---|---|---|
| **Partial `.updater/` Tampering** (`trust-state.json`, `.bak.json`, or all 3 files in `.updater/` replaced from older snapshot) | **YES (Protected)** | Blocked by cross-check against `ExternalTrustAnchor` (`%LOCALAPPDATA%\FarmerPro\TrustAnchors` + `HKCU\Software\FarmerPro\TrustAnchors`): `FULL_TRIAD_ROLLBACK_DETECTED`. | Protects against accidental restore, partial copy, and accidental/scripted `.updater/` wipe (`install_initial` over wiped `.updater/` is rejected). |
| **Untrusted Key Rotation Nonce Replay** (reusing delegation nonce for a key NOT in `trusted_pubkeys`) | **YES (Protected)** | Blocked with `KEY_ROTATION_NONCE_REPLAYED`. Legitimate re-upgrade after rollback (`v1.0 -> v1.1 -> rollback -> re-upgrade v1.1`) works when key is already trusted and NOT revoked. | `key_already_trusted` is explicitly gated on `!is_revoked`, Authenticode `SigningMode` policy, and `CompatibilityVersions`. |
| **Revoked Key Re-Upgrade / Rollback** (previously trusted key later revoked) | **YES (Protected)** | Blocked with `KEY_REVOKED` / `UPDATE_ROOT_OF_TRUST_UNTRUSTED` / `ROLLBACK_ROOT_OF_TRUST_UNTRUSTED` and caught across all `versions/v*` by `fp-updater verify-security`. | Tested in `test_p6_closeout_revoked_previously_trusted_key_policy_and_compat_gates`. |
| **Rogue / Altered Version Directory & Release Manifest Tamper** | **YES (Protected)** | Blocked with `ROGUE_VERSION_DIR_DETECTED`, `PE_ROOT_OF_TRUST_MISMATCH`, and `RELEASE_MANIFEST_TAMPERED`. | Covers non-bundle distributed files (e.g., `THIRD_PARTY_NOTICES.txt`, binaries) in every version directory. |
| **Windows Read-Only NTFS Directory (`icacls RX`) & Atomic Write Exhaustion** | **YES (Fail-Closed)** | Portable Mini Tool falls back to `%LOCALAPPDATA%\FarmerPro\MiniTools`; updater refuses write (`INSTALL_DIR_NOT_WRITABLE`). `atomic_write_file` retries `MoveFileExW` on Win32 errors `5`/`32` (5 attempts × 35 ms = ~140 ms backoff), exhausts cleanly on permanent ACL / prolonged lock (`ACTIVE_POINTER_LOCKED`), leaves 0 `.tmp.*` files, and never corrupts `active_version.json`. | Step 6 now commits `active_version.json` before mutating `trust-state.json` / `used_rotation_nonces`; failed activation stays at `StagedVerified` so `reconcile_incomplete_journal` cleans unactivated `versions/` and restores clean `v1.0.0`. |
| **Full 6-Component Same-User Compromise** (same Windows user simultaneously overwrites `.updater/*`, `active_version.json`, `versions/*`, External Anchor JSON, and `HKCU\Software\FarmerPro\TrustAnchors`) | **NO (Out of Local ACL Boundary)** | Empirically demonstrated in `verify_p6.js` and Rust test: `hkcuWriteAcl = FullControl`, `extFileAcl = Authenticated Users:(I)(M)` — overwriting all 6 stores resets `trustEpoch` to `1`. | `HKCU` and `%LOCALAPPDATA%` share the same Windows User SID security boundary; cannot isolate against a malicious/elevated payload running inside the same user session. |

### Future Hardening Options (If Same-User Compromise Prevention Is Required Later — Not Implemented in P6)
1. **Privilege Separation (Admin / Service ACL):** Install Mini Tools under `%ProgramFiles%`, run privileged updates via a Windows Service (`LocalSystem`), and store monotonic trust anchors in `HKLM` / `%ProgramData%` with Administrators/SYSTEM-only Write ACL (`Users:RX`).
2. **External Online Trust Server:** Anchor monotonic publisher version floor, key revocation list (CRL/OCSP-equivalent), and entitlement/manifest checksums on an independent server out of reach of the client Windows account.

---

## 2. Security Risk Register

| ID | Risk Description | Likelihood | Impact | Current Control & Status | Residual Risk & Disposition |
|---|---|---|---|---|---|
| **RISK-01** | Same-user attacker or script replaces all 6 local user-writable trust components (`HKCU`, `%LOCALAPPDATA%`, `.updater/*`, `versions/*`) to downgrade to an older signed version. | Low–Med (Local access) | High (Binary downgrade) | Blocks partial `.updater/` tampering (`FULL_TRIAD_ROLLBACK_DETECTED`) and documents boundary explicitly (never labeled tamper-proof). | **Accepted for Local Mode** — Mitigate in future only via Admin Service + `HKLM` or online Trust Server. |
| **RISK-02** | Previously trusted publisher key is later revoked, yet an attacker replays a signed re-upgrade bundle (`v1.1.0` after rollback). | Low | High | `upgrade_from_release` checks `revoked_pubkeys` BEFORE `key_already_trusted`, and `key_already_trusted` excludes revoked keys (`test_p6_closeout_revoked_previously_trusted_key_policy_and_compat_gates`). | **Mitigated (Code + Regression Test)** |
| **RISK-03** | Prolonged antivirus scan or permanent ACL denies `MoveFileExW` replacement of `active_version.json`, leaving journal stuck or prematurely consuming rotation nonce. | Medium (Windows AV) | Medium | `atomic_write_file` retries 5×35 ms (~140 ms) then fails closed with `ACTIVE_POINTER_LOCKED`; Step 6 writes `active_version.json` before `trust-state`; `reconcile_incomplete_journal` cleans unactivated `versions/` dir. | **Mitigated (Code + Win32 Lock Regression Test)** |
| **RISK-04** | Clean Windows 10/11 target without Edge WebView2 Runtime attempts offline launch of Mini Tool. | Medium | Medium | `ReleasePackageManifest` declares `requiresInternetIfWebview2Missing: true` & `offlineReady`; `mini-runtime` exits cleanly with code `3` (`WEBVIEW2_RUNTIME_MISSING`). Clean Windows VM execution not yet verified. | **Open — External Gate 1 (`BLOCKED`: Clean VM)** |
| **RISK-05** | Real Android / LDPlayer workflow execution (capture, OCR/template matching, tap/swipe, pause/resume/cancel, realtime variables) has not run on live connected hardware. | High (Hardware gap) | High (Production readiness) | All unit/TestKit tests use fake driver labeled `[TESTKIT · Mô phỏng]`. `LIVE_WORKFLOW_EXECUTION_PASS = BLOCKED` until real hardware/LDPlayer ADB online. | **Open — External Gate 2 (`BLOCKED`: Live ADB)** |
| **RISK-06** | Production Authenticode trust chain (`PublicSigned`) cannot be verified without a commercial CA code-signing certificate; SmartScreen warnings expected on unsigned/self-signed binaries. | High (Cert missing) | High (Distribution) | `SigningMode::PublicSigned` and `release_channel="production"` enforce WinVerifyTrust fail-closed (`AUTHENTICODE_PUBLIC_TRUST_REQUIRED`). | **Open — External Gate 3 (`BLOCKED`: Commercial CA)** |
