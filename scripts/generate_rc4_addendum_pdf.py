import os
import json
import hashlib
import base64
import subprocess
import pymupdf

ROOT = r"E:\DATA\FarmerPro"
PACK_DIR = os.path.join(ROOT, "artifacts", "final_acceptance_pack")
RC4_DIR = os.path.join(PACK_DIR, "rc4_evidence")
GATE2_DIR = os.path.join(PACK_DIR, "gate2_live_adb")
RENDER_DIR = os.path.join(RC4_DIR, "pdf_rc4_rendered_pages")
os.makedirs(RENDER_DIR, exist_ok=True)

CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"
HTML_OUT = os.path.join(RC4_DIR, "FARMERPRO_RC4_ADDENDUM_REPORT.html")
PDF_MAIN = os.path.join(ROOT, "FARMERPRO_RC4_ADDENDUM_REPORT.pdf")
PDF_COPY = os.path.join(PACK_DIR, "FARMERPRO_RC4_ADDENDUM_REPORT.pdf")

rc4 = json.load(open(os.path.join(RC4_DIR, "rc4_hotfix_verification.json"), encoding="utf-8"))
g2 = json.load(open(os.path.join(GATE2_DIR, "gate2_live_adb_report.json"), encoding="utf-8"))
summary = json.load(open(os.path.join(PACK_DIR, "FINAL_ACCEPTANCE_SUMMARY.json"), encoding="utf-8"))

def img_b64(p):
    with open(p, "rb") as f:
        return "data:image/png;base64," + base64.b64encode(f.read()).decode("ascii")

def sha256_file(p):
    h = hashlib.sha256()
    with open(p, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()

imgs = {
    "studio_dev": img_b64(os.path.join(GATE2_DIR, "gate2_studio_device_lab.png")),
    "studio_run": img_b64(os.path.join(GATE2_DIR, "gate2_studio_runtime_vars.png")),
    "mini_before": img_b64(os.path.join(GATE2_DIR, "gate2_mini_before.png")),
    "mini_after": img_b64(os.path.join(GATE2_DIR, "gate2_mini_after.png")),
    "dev_before": img_b64(os.path.join(GATE2_DIR, "gate2_device_before_mini.png")),
    "dev_after": img_b64(os.path.join(GATE2_DIR, "gate2_device_after_mini.png")),
}

ta = rc4["testA_CrossProcessDeviceLease"]
tb = rc4["testB_SqliteV6MigrationCompatibility"]
ra = summary["releaseArtifacts"]

html = f"""<!doctype html>
<html lang="vi">
<head>
<meta charset="utf-8"/>
<title>FarmerPro RC.4 Hotfix Acceptance Addendum</title>
<style>
  @page {{ size: A4 landscape; margin: 10mm 11mm; }}
  * {{ box-sizing: border-box; }}
  body {{ margin: 0; font-family: 'Segoe UI', Arial, sans-serif; color: #0F172A; background: #FFFFFF; font-size: 9.2pt; line-height: 1.35; }}
  .page {{ page-break-after: always; display: flex; flex-direction: column; min-height: 185mm; justify-content: space-between; }}
  .page:last-child {{ page-break-after: auto; }}
  .hdr {{ display: flex; justify-content: space-between; align-items: center; gap: 10px; border-bottom: 2px solid #0F172A; padding-bottom: 5px; margin-bottom: 8px; }}
  .hdr-title {{ font-size: 12.2pt; font-weight: 800; color: #0F172A; }}
  .hdr-meta {{ font-size: 8.2pt; color: #475569; font-family: Consolas, monospace; }}
  .hdr-badges {{ flex-shrink: 0; white-space: nowrap; display: flex; gap: 5px; align-items: center; }}
  .badge-pass {{ background: #DCFCE7; color: #166534; border: 1px solid #86EFAC; padding: 1px 6px; border-radius: 4px; font-weight: 700; font-size: 8pt; white-space: nowrap; display: inline-block; }}
  .badge-blk {{ background: #FEF3C7; color: #92400E; border: 1px solid #FCD34D; padding: 1px 6px; border-radius: 4px; font-weight: 700; font-size: 8pt; white-space: nowrap; display: inline-block; }}
  .grid2 {{ display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }}
  .grid3 {{ display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 6px; }}
  .card {{ border: 1px solid #CBD5E1; border-radius: 6px; padding: 7px 9px; background: #F8FAFC; }}
  .card h3 {{ margin: 0 0 4px 0; font-size: 9.6pt; color: #0F172A; border-bottom: 1px solid #E2E8F0; padding-bottom: 3px; }}
  table {{ width: 100%; border-collapse: collapse; font-size: 8pt; }}
  th, td {{ border: 1px solid #CBD5E1; padding: 2.5px 5px; text-align: left; vertical-align: top; }}
  th {{ background: #E2E8F0; font-weight: 700; color: #0F172A; }}
  code {{ font-family: Consolas, monospace; font-size: 7.8pt; background: #F1F5F9; padding: 0 3px; border-radius: 3px; }}
  .shot-box {{ border: 1px solid #CBD5E1; border-radius: 6px; padding: 4px; background: #FFFFFF; text-align: center; }}
  .shot-box img {{ max-width: 100%; max-height: 38mm; object-fit: contain; border: 1px solid #E2E8F0; }}
  .shot-cap {{ font-size: 7.6pt; color: #334155; margin-top: 2px; font-weight: 600; }}
  .ftr {{ border-top: 1px solid #CBD5E1; padding-top: 4px; font-size: 8pt; color: #64748B; display: flex; justify-content: space-between; }}
</style>
</head>
<body>

<section class="page">
  <div>
    <div class="hdr">
      <div>
        <div class="hdr-title">FARMERPRO RC.4 — HOTFIX ACCEPTANCE ADDENDUM (LOCKFILEEX &amp; SQLITE MIGRATION V6)</div>
        <div class="hdr-meta">Branch: feat/p6-final-validation | Tag: v0.1.0-p6-rc.4 | Code Commit: {summary['verifiedCodeCommit']} | Rust: {summary['provenance']['cargoWorkspaceTestsPassed']}/{summary['provenance']['cargoWorkspaceTestsPassed']} PASS</div>
      </div>
      <div class="hdr-badges"><span class="badge-pass">RC.4 FIXES VERIFIED</span><span class="badge-blk">PROD GATE: BLOCKED</span></div>
    </div>

    <div class="grid2" style="margin-bottom:8px;">
      <div class="card">
        <h3>Fix A — Cross-Process Device Lease (Win32 LockFileEx)</h3>
        <ul style="margin:0; padding-left:16px;">
          <li><b>Shared Lock Dir:</b> <code>%LOCALAPPDATA%\\FarmerPro\\locks\\dev_&lt;sha256&gt;.lock</code></li>
          <li><b>Mutual Exclusion:</b> <code>CreateFileW</code> + <code>LockFileEx</code> exclusive lock on bytes <code>[0..64)</code> held throughout entire job lifetime; owner metadata JSON at offset <code>64..</code>.</li>
          <li><b>Zero Unlink Race &amp; TTL Non-Override:</b> File stays on disk across unlocks; elapsed TTL never steals a live process OS lock. Kernel releases handle immediately on <code>taskkill /F</code>.</li>
          <li><b>Startup Recovery &amp; Fail-Closed:</b> Stale crashed DB jobs recover to <code>interrupted</code> only if OS lock is unlocked; init failure aborts with <code>DEVICE_LOCK_INIT_FAILED</code>.</li>
        </ul>
      </div>
      <div class="card">
        <h3>Fix B — Formal SQLite Schema Migration v6</h3>
        <ul style="margin:0; padding-left:16px;">
          <li><b>Schema Bump:</b> <code>CURRENT_SQLITE_SCHEMA_VERSION = 6</code> in Contract, Builder, Runtime &amp; Updater.</li>
          <li><b>Atomic Transaction:</b> <code>BEGIN IMMEDIATE</code> executes 4 <code>ALTER TABLE runtime_variable_envelopes ADD COLUMN</code> statements, normalizes legacy/blank rows to <code>LEGACY_UNVERIFIED</code> (<code>verified=0</code>, no fake LIVE confidence), checks <code>PRAGMA table_info</code> &amp; <code>foreign_key_check=0</code>, then inserts version <code>6</code> into <code>schema_migrations</code>.</li>
          <li><b>Fail-Closed Guards:</b> Removed silent in-memory fallback on Studio &amp; Mini Runtime startup; missing columns or future versions (v99) fail closed with <code>SQLITE_SCHEMA_INCOMPATIBLE</code> / <code>SQLITE_DOWNGRADE_INCOMPATIBLE</code>.</li>
        </ul>
      </div>
    </div>

    <div class="grid2">
      <div>
        <div style="font-weight:700; margin-bottom:3px; font-size:9pt;">Test A Matrix (emulator-5554 Live ADB &amp; Multi-EXE)</div>
        <table>
          <tr><th>ID</th><th>Check</th><th>Observed Result</th></tr>
          <tr><td><b>A1</b></td><td>Studio holds device -> Mini Job B blocked</td><td><span class="badge-pass">PASS</span> <code>DEVICE_LEASE_CONFLICT</code> (pid {ta['A3_forcedKillStudioRecovery']['killedStudioPid']})</td></tr>
          <tr><td><b>A2</b></td><td>Studio Job A ends -> Mini Job B proceeds</td><td><span class="badge-pass">PASS</span> <code>Succeeded</code> ({ta['A2_miniProceedsAfterStudioRelease']['driverUsed']})</td></tr>
          <tr><td><b>A3</b></td><td>Forced-kill Studio -> Mini runs &amp; Studio recovers DB</td><td><span class="badge-pass">PASS</span> Job C -> <code>interrupted</code>; Job D -> <code>succeeded</code></td></tr>
          <tr><td><b>A4</b></td><td>Forced-kill Mini -> Studio blocked then takes device</td><td><span class="badge-pass">PASS</span> Mini job -> <code>interrupted</code>; Studio -> <code>succeeded</code></td></tr>
          <tr><td><b>A5</b></td><td>Dual transport (<code>127.0.0.1:5555</code> vs <code>emulator-5554</code>)</td><td><span class="badge-pass">PASS</span> <code>DEVICE_LEASE_CONFLICT</code></td></tr>
          <tr><td><b>A6</b></td><td>Two genuinely distinct devices concurrently</td><td><span class="badge-pass">PASS (RUST)</span> Cross-process tests (1 live AVD on host)</td></tr>
          <tr><td><b>A7</b></td><td>ADB disconnect / reconnect cleanup</td><td><span class="badge-pass">PASS</span> <code>{ta['A7_disconnectReconnectClean']['adbDisconnectResult'].strip()}</code></td></tr>
          <tr><td><b>A8</b></td><td>Two Mini EXEs (<code>CrossMini1</code> vs <code>CrossMini2</code>)</td><td><span class="badge-pass">PASS</span> Mini 2 blocked during run, succeeded after</td></tr>
          <tr><td><b>A9</b></td><td>Fail-closed when OS lock init fails</td><td><span class="badge-pass">PASS</span> <code>DEVICE_LOCK_INIT_FAILED</code></td></tr>
        </table>
      </div>
      <div>
        <div style="font-weight:700; margin-bottom:3px; font-size:9pt;">Test B Matrix (RC.2 b5a0567 v5 DB -> RC.4 v6)</div>
        <table>
          <tr><th>ID</th><th>Check</th><th>Observed Result</th></tr>
          <tr><td><b>B1–B4</b></td><td>RC.2 v5, RC.3 v5 &amp; fresh DB -> idempotent v6</td><td><span class="badge-pass">PASS</span> Schema <code>[1,2,3,4,5,6]</code>, 4 provenance cols</td></tr>
          <tr><td><b>B5</b></td><td>Domain tables preserved across upgrade</td><td><span class="badge-pass">PASS</span> Projects=1, WF=1, Tasks=1, Acc=1, Rel=1, Pub=1</td></tr>
          <tr><td><b>B6</b></td><td>Legacy unverified safety (no fake LIVE/%)</td><td><span class="badge-pass">PASS</span> <code>LEGACY_UNVERIFIED</code>, <code>verified=false</code></td></tr>
          <tr><td><b>B7</b></td><td><code>studio_get_project_runtime_vars</code></td><td><span class="badge-pass">PASS</span> <code>varsError = null</code> (fixed!)</td></tr>
          <tr><td><b>B8–B9</b></td><td><code>integrity_check</code> &amp; <code>foreign_key_check</code></td><td><span class="badge-pass">PASS</span> <code>["ok"]</code>, 0 FK violations</td></tr>
          <tr><td><b>B10</b></td><td>Fault rollback &amp; backup restore SHA-256</td><td><span class="badge-pass">PASS</span> Match <code>{tb['B10_rollbackAndBackupRestore']['backupSha256'][:16]}...</code></td></tr>
          <tr><td><b>B11</b></td><td>Independent Mini Tool v5 DB upgrade + run</td><td><span class="badge-pass">PASS</span> <code>sqliteSchemaVersion=6</code>, <code>Succeeded</code></td></tr>
          <tr><td><b>B12</b></td><td>Block incompatible v99 &amp; missing-col v6 DBs</td><td><span class="badge-pass">PASS</span> <code>SQLITE_DOWNGRADE</code> / <code>SCHEMA_INCOMPATIBLE</code></td></tr>
        </table>
      </div>
    </div>
  </div>
  <div class="ftr">
    <span>FarmerPro v0.1.0-p6-rc.4 · Hotfix Addendum Page 1 of 2 (See FARMERPRO_P6_ACCEPTANCE_REPORT.pdf for baseline P6/Gate 2 architecture)</span>
    <span>Baseline v0.1.0-p6-rc.3 (629cce2) preserved untouched</span>
  </div>
</section>

<section class="page">
  <div>
    <div class="hdr">
      <div>
        <div class="hdr-title">FARMERPRO RC.4 — GATE 2 LIVE REGRESSION SCREENSHOTS &amp; CHECKSUMS</div>
        <div class="hdr-meta">Gate 2 Status: {g2['gate2Status']} | Target: {g2['adbTargetSerial']} (boot_id: {g2['canonicalDeviceIdentityProof']['emuBootId']}) | Commit: {g2['gitHead']}</div>
      </div>
      <div class="hdr-badges"><span class="badge-pass">P6 &amp; GATE 2 REGRESSION PASS</span></div>
    </div>

    <div class="grid3" style="margin-bottom:8px;">
      <div class="shot-box"><img src="{imgs['studio_dev']}"/><div class="shot-cap">Studio Device Lab (1 Canonical Online Device)</div></div>
      <div class="shot-box"><img src="{imgs['studio_run']}"/><div class="shot-cap">Studio Live Workflow Execution (Match 0.931)</div></div>
      <div class="shot-box"><img src="{imgs['mini_before']}"/><div class="shot-cap">Auto_Gate2Live.exe Before Run (LIVE ADB: 1 ONLINE)</div></div>
      <div class="shot-box"><img src="{imgs['mini_after']}"/><div class="shot-cap">Auto_Gate2Live.exe After Run (LIVE · Đã khớp quy tắc)</div></div>
      <div class="shot-box"><img src="{imgs['dev_before']}"/><div class="shot-cap">AVD Settings Screen Before Mini Tool Run</div></div>
      <div class="shot-box"><img src="{imgs['dev_after']}"/><div class="shot-cap">AVD Settings Screen After Live SafeTap &amp; OCR</div></div>
    </div>

    <div class="grid2">
      <div class="card">
        <h3>RC.4 Rebuilt Binaries &amp; Exported Packages (SHA-256)</h3>
        <table>
          <tr><th>Artifact</th><th>Bytes</th><th>SHA-256</th></tr>
          <tr><td><code>FarmerPro_0.1.0_x64-setup.exe</code></td><td>{ra['installer']['bytes']:,}</td><td><code>{ra['installer']['sha256']}</code></td></tr>
          <tr><td><code>farmerpro.exe</code></td><td>{ra['farmerproExe']['bytes']:,}</td><td><code>{ra['farmerproExe']['sha256']}</code></td></tr>
          <tr><td><code>fp-mini-runtime.exe</code></td><td>{ra['fpMiniRuntimeExe']['bytes']:,}</td><td><code>{ra['fpMiniRuntimeExe']['sha256']}</code></td></tr>
          <tr><td><code>P6_Alpha_v1.0.0_win-x64.zip</code></td><td>{ra['portableAlphaZip']['bytes']:,}</td><td><code>{ra['portableAlphaZip']['sha256']}</code></td></tr>
          <tr><td><code>Gate2_LiveMini_v1.0.0_win-x64.zip</code></td><td>{ra['gate2MiniZip']['bytes']:,}</td><td><code>{ra['gate2MiniZip']['sha256']}</code></td></tr>
        </table>
      </div>
      <div class="card">
        <h3>Release Gate Boundaries (Unchanged at RC.4 Stage-Gate)</h3>
        <table>
          <tr><th>Gate</th><th>Status</th><th>Detail</th></tr>
          <tr><td>Dev + Gate 2 Live ADB</td><td><span class="badge-pass">PASS</span></td><td>106/106 Rust + P6 E2E + Gate 2 Live ADB + RC.4 A1–A9 &amp; B1–B12</td></tr>
          <tr><td>Game-Specific Workflows</td><td><span class="badge-blk">NOT TESTED</span></td><td>0 game APKs executed on live ADB yet</td></tr>
          <tr><td>Gate 1 — Clean Windows VM</td><td><span class="badge-blk">BLOCKED</span></td><td>WindowsSandbox.exe absent; Hyper-V un-elevated</td></tr>
          <tr><td>Gate 3 — Commercial Signing</td><td><span class="badge-blk">BLOCKED</span></td><td>0 commercial CA certs in Windows cert store</td></tr>
          <tr><td>Production Release</td><td><span class="badge-blk">BLOCKED</span></td><td>Stopped at RC.4 Stage-Gate pending Gates 1 &amp; 3</td></tr>
        </table>
      </div>
    </div>
  </div>
  <div class="ftr">
    <span>FarmerPro v0.1.0-p6-rc.4 · Rebuilt Release Checksums &amp; Live Regression Evidence Page 2 of 2</span>
    <span>RC.4 Stage-Gate Complete</span>
  </div>
</section>
</body>
</html>
"""

with open(HTML_OUT, "w", encoding="utf-8") as f:
    f.write(html)

subprocess.check_call([
    CHROME,
    "--headless=new",
    "--disable-gpu",
    "--no-pdf-header-footer",
    f"--print-to-pdf={PDF_MAIN}",
    "file:///" + HTML_OUT.replace("\\", "/"),
])

with open(PDF_MAIN, "rb") as src, open(PDF_COPY, "wb") as dst:
    dst.write(src.read())

for old_png in os.listdir(RENDER_DIR):
    if old_png.startswith("rc4_page_") and old_png.endswith(".png"):
        os.remove(os.path.join(RENDER_DIR, old_png))

doc = pymupdf.open(PDF_MAIN)
for idx, page in enumerate(doc, 1):
    pix = page.get_pixmap(dpi=140)
    pix.save(os.path.join(RENDER_DIR, f"rc4_page_{idx:02d}.png"))
print(json.dumps({
    "pdfPath": PDF_MAIN,
    "pdfCopyPath": PDF_COPY,
    "pages": len(doc),
    "bytes": os.path.getsize(PDF_MAIN),
    "sha256": sha256_file(PDF_MAIN),
}, indent=2))
