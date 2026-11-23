import os
import json
import hashlib
import base64
import subprocess
import pymupdf

ROOT = r"E:\DATA\FarmerPro"
PACK_DIR = os.path.join(ROOT, "artifacts", "final_acceptance_pack")
G1_DIR = os.path.join(ROOT, "artifacts", "gate1_sandbox")
OFF_DIR = os.path.join(G1_DIR, "evidence_offline")
ON_DIR = os.path.join(G1_DIR, "evidence_online")
RENDER_DIR = os.path.join(G1_DIR, "pdf_gate1_rendered_pages")
os.makedirs(RENDER_DIR, exist_ok=True)

CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"
HTML_OUT = os.path.join(G1_DIR, "FARMERPRO_GATE1_SANDBOX_REPORT.html")
PDF_MAIN = os.path.join(ROOT, "FARMERPRO_GATE1_SANDBOX_REPORT.pdf")
PDF_COPY = os.path.join(PACK_DIR, "FARMERPRO_GATE1_SANDBOX_REPORT.pdf")

off_rep = json.load(open(os.path.join(OFF_DIR, "offline_report.json"), encoding="utf-8-sig"))
tf_rep = json.load(open(os.path.join(OFF_DIR, "test_f_and_create_report.json"), encoding="utf-8-sig"))
std_rep = json.load(open(os.path.join(OFF_DIR, "standard_user_report.json"), encoding="utf-8-sig"))
on_rep = json.load(open(os.path.join(ON_DIR, "online_present_wv2.json"), encoding="utf-8-sig"))
wv2_un = json.load(open(os.path.join(OFF_DIR, "wv2_uninstall_sys.json"), encoding="utf-8-sig"))
sql_rep = json.load(open(os.path.join(G1_DIR, "gate1_sqlite_audit_report.json"), encoding="utf-8-sig"))

def img_b64(p):
    with open(p, "rb") as f:
        return "data:image/png;base64," + base64.b64encode(f.read()).decode("ascii")

def sha256_file(p):
    h = hashlib.sha256()
    with open(p, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()

shots = {
    "alpha": img_b64(os.path.join(OFF_DIR, "screenshots", "01_portable_alpha_standalone.png")),
    "beta": img_b64(os.path.join(OFF_DIR, "screenshots", "02_portable_beta_standalone.png")),
    "studio_create": img_b64(os.path.join(OFF_DIR, "screenshots", "03c_studio_project_created_real.png")),
    "studio_reopen": img_b64(os.path.join(OFF_DIR, "screenshots", "04c_studio_reopened_real.png")),
    "sql_clean": img_b64(os.path.join(OFF_DIR, "screenshots", "05_sql_rc2_v5_clean.png")),
    "online_fw": img_b64(os.path.join(ON_DIR, "screenshots", "07_online_firewall_adb_prompt.png")),
}

env = off_rep["environment"]
vcrt = tf_rep["sandboxVcrtCheck"]
upd = tf_rep["F_updateRollback"]

html = f"""<!doctype html>
<html lang="vi">
<head>
<meta charset="utf-8"/>
<title>FarmerPro RC.4 — Gate 1 Clean Windows Sandbox Report</title>
<style>
  @page {{ size: A4 landscape; margin: 9mm 10mm; }}
  * {{ box-sizing: border-box; }}
  body {{ margin: 0; font-family: 'Segoe UI', Arial, sans-serif; color: #0F172A; background: #FFFFFF; font-size: 8.6pt; line-height: 1.32; }}
  .page {{ page-break-after: always; display: flex; flex-direction: column; min-height: 183mm; justify-content: space-between; }}
  .page:last-child {{ page-break-after: auto; }}
  .hdr {{ display: flex; justify-content: space-between; align-items: center; gap: 10px; border-bottom: 2px solid #0F172A; padding-bottom: 5px; margin-bottom: 7px; }}
  .hdr-title {{ font-size: 12pt; font-weight: 800; color: #0F172A; }}
  .hdr-meta {{ font-size: 8pt; color: #475569; font-family: Consolas, monospace; }}
  .hdr-badges {{ flex-shrink: 0; white-space: nowrap; display: flex; gap: 5px; }}
  .b-pass {{ background: #DCFCE7; color: #166534; border: 1px solid #86EFAC; padding: 1px 6px; border-radius: 4px; font-weight: 700; font-size: 7.8pt; white-space: nowrap; display: inline-block; }}
  .b-fail {{ background: #FEE2E2; color: #991B1B; border: 1px solid #FCA5A5; padding: 1px 6px; border-radius: 4px; font-weight: 700; font-size: 7.8pt; white-space: nowrap; display: inline-block; }}
  .b-blk {{ background: #FEF3C7; color: #92400E; border: 1px solid #FCD34D; padding: 1px 6px; border-radius: 4px; font-weight: 700; font-size: 7.8pt; white-space: nowrap; display: inline-block; }}
  .grid2 {{ display: grid; grid-template-columns: 1.15fr 0.85fr; gap: 8px; }}
  .grid2eq {{ display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }}
  .grid3 {{ display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 6px; }}
  .card {{ border: 1px solid #CBD5E1; border-radius: 6px; padding: 6px 9px; background: #F8FAFC; }}
  .card-alert {{ border: 1px solid #FCA5A5; border-radius: 6px; padding: 6px 9px; background: #FEF2F2; }}
  .card h3, .card-alert h3 {{ margin: 0 0 4px 0; font-size: 9.4pt; color: #0F172A; border-bottom: 1px solid #CBD5E1; padding-bottom: 2px; }}
  table {{ width: 100%; border-collapse: collapse; font-size: 7.8pt; }}
  th, td {{ border: 1px solid #CBD5E1; padding: 2.5px 5px; text-align: left; vertical-align: top; }}
  th {{ background: #E2E8F0; font-weight: 700; }}
  code {{ font-family: Consolas, monospace; font-size: 7.6pt; background: #F1F5F9; padding: 0 3px; border-radius: 3px; }}
  .shot-box {{ border: 1px solid #CBD5E1; border-radius: 5px; padding: 4px; background: #FFFFFF; text-align: center; }}
  .shot-box img {{ max-width: 100%; max-height: 36mm; object-fit: contain; border: 1px solid #E2E8F0; }}
  .shot-cap {{ font-size: 7.4pt; color: #334155; margin-top: 2px; font-weight: 600; }}
  .ftr {{ border-top: 1px solid #CBD5E1; padding-top: 3px; font-size: 7.8pt; color: #64748B; display: flex; justify-content: space-between; }}
</style>
</head>
<body>

<section class="page">
  <div>
    <div class="hdr">
      <div>
        <div class="hdr-title">FARMERPRO RC.4 — GATE 1 CLEAN WINDOWS SANDBOX VALIDATION REPORT</div>
        <div class="hdr-meta">Baseline: abaabaa (v0.1.0-p6-rc.4) | Docs: ec23be9 | Sandbox OS: {env['osCaption']} Build {env['osBuild']} ({env['arch']}, {env['totalMemMb']} MB) | Dev Tools: 0</div>
      </div>
      <div class="hdr-badges"><span class="b-blk">GATE 1: PARTIAL</span><span class="b-fail">UPDATER DEFECT FOUND</span><span class="b-blk">PROD GATE: BLOCKED</span></div>
    </div>

    <div class="grid2" style="margin-bottom:7px;">
      <div class="card">
        <h3>Gate 1 Acceptance Matrix (9 Required Criteria)</h3>
        <table>
          <tr><th>Criterion</th><th>Status</th><th>Observed Evidence inside Windows Sandbox</th></tr>
          <tr><td><code>SANDBOX_BOOT_PASS</code></td><td><span class="b-pass">PASS</span></td><td>Offline (<code>nonLoopbackIpv4=[]</code>) &amp; Online (<code>172.17.238.59</code>) Sandboxes booted via <code>wsb.exe</code>; RO stage SHA-256 verified.</td></tr>
          <tr><td><code>STUDIO_INSTALL_PASS</code></td><td><span class="b-pass">PASS</span></td><td>NSIS <code>/S</code> (<code>ExitCode=0</code>); launched Offline &amp; Online; created <code>Gate1 Sandbox RPG</code> via modal; persisted across restart.</td></tr>
          <tr><td><code>STANDALONE_PORTABLE_PASS</code></td><td><span class="b-pass">PASS</span></td><td><code>Auto_P6Alpha.exe</code> &amp; <code>Auto_P6Beta.exe</code> launched without Studio &amp; post-uninstall; <code>ADB: 0 ONLINE</code>; standalone DBs ok.</td></tr>
          <tr><td><code>WEBVIEW2_ONLINE_PASS</code></td><td><span class="b-blk">PARTIAL</span></td><td>Present + Online = <b>PASS</b> (<code>157.0.4341.60</code>). Missing + Online = <b>BLOCKED</b> (OS-inbox WebView2 uninstall exit <code>{wv2_un['uninstallExitCode']}</code>).</td></tr>
          <tr><td><code>WEBVIEW2_OFFLINE_PASS</code></td><td><span class="b-blk">PARTIAL</span></td><td>Present + Offline = <b>PASS</b>. Missing + Offline = <b>BLOCKED</b> in 24H2 image (exit <code>{wv2_un['uninstallExitCode']}</code>; installer uses <code>embedBootstrapper</code>).</td></tr>
          <tr><td><code>STANDARD_USER_PASS</code></td><td><span class="b-blk">BLOCKED</span></td><td>Non-admin <code>fpstandard</code> created (<code>inAdmins=false</code>), but Sandbox single-session isolation blocks secondary logon (<code>0xC0000142</code> / <code>267011</code>).</td></tr>
          <tr><td><code>SQLITE_V5_TO_V6_PASS</code></td><td><span class="b-pass">PASS</span></td><td>RC.2 <code>clean</code> (0/4), <code>partial_2cols</code> (2/4) &amp; <code>partial_3cols</code> (3/4) DBs opened by Studio -> schema <code>[1..=6]</code>, 18 cols, <code>ok</code>, <code>FK=0</code>, <code>LEGACY_UNVERIFIED</code>.</td></tr>
          <tr><td><code>UPDATE_ROLLBACK_PASS</code></td><td><span class="b-fail">FAIL</span></td><td><code>fp-updater.exe install-initial</code> crashed on Clean Windows: exit <code>{upd['installInitialExitCode']}</code> (<code>{upd['installInitialHex']}</code>) — missing <code>VCRUNTIME140.dll</code>.</td></tr>
          <tr><td><code>UNINSTALL_DATA_PRESERVATION_PASS</code></td><td><span class="b-pass">PASS</span></td><td><code>uninstall.exe /S</code> removed EXEs; <code>farmerpro.db</code> preserved identically (SHA-256 <code>41b81c50...</code>); Portable Alpha still runs.</td></tr>
        </table>
      </div>

      <div style="display:flex; flex-direction:column; gap:7px;">
        <div class="card-alert">
          <h3>Critical Clean-Windows Finding — Minimal Hotfix Required (Approval Pending)</h3>
          <ul style="margin:0; padding-left:15px;">
            <li><b>Observed Failure:</b> Inside Clean Windows Sandbox (<code>System32\\VCRUNTIME140.dll = {str(vcrt['vcruntime140Exists']).lower()}</code>, <code>ucrtbase.dll = true</code>), <code>fp-updater.exe</code> exited immediately with <code>{upd['installInitialHex']}</code> (<code>STATUS_DLL_NOT_FOUND</code>).</li>
            <li><b>PE Import Audit:</b> <code>farmerpro.exe</code> &amp; <code>fp-mini-runtime.exe</code> have <b>0</b> <code>VCRUNTIME140.dll</code> imports (ran fine in Sandbox); <code>fp-updater.exe</code> &amp; <code>fp-cli.exe</code> dynamically import <code>VCRUNTIME140.dll</code> (worked on dev host only because VS/VC++ Redist was pre-installed).</li>
            <li><b>Online Firewall Note:</b> First online Studio launch triggered Windows Firewall prompt for bundled <code>adb.exe</code> (<i>Publisher: Unknown</i>).</li>
            <li><b>Proposed Hotfix:</b> Freeze <code>abaabaa</code> untouched until approved; apply <code>+crt-static</code> (e.g. <code>.cargo/config.toml</code>) so <code>fp-updater.exe</code> &amp; <code>fp-cli.exe</code> drop <code>VCRUNTIME140.dll</code>, rebuild &amp; re-verify Test F in Sandbox.</li>
          </ul>
        </div>
        <div class="card">
          <h3>WebView2 Matrix &amp; Clean Sandbox SQLite Upgrade Verification</h3>
          <table>
            <tr><th>WebView2 Matrix Cell</th><th>Result</th><th>Detail</th></tr>
            <tr><td>Present (<code>157.0.4341.60</code>) + Offline</td><td><span class="b-pass">PASS</span></td><td>Studio + Alpha + Beta rendered Offline cleanly</td></tr>
            <tr><td>Present (<code>157.0.4341.60</code>) + Online</td><td><span class="b-pass">PASS</span></td><td>Studio NSIS installed &amp; launched Online cleanly</td></tr>
            <tr><td>Missing + Online / Missing + Offline</td><td><span class="b-blk">BLOCKED</span></td><td>Sandbox OS image locks WebView2 (<code>exit 93</code>)</td></tr>
          </table>
        </div>
      </div>
    </div>
  </div>
  <div class="ftr">
    <span>FarmerPro v0.1.0-p6-rc.4 · Gate 1 Clean Windows Sandbox Validation (Page 1 of 2)</span>
    <span>Frozen RC.4 Baseline Preserved · Not Approved for Production</span>
  </div>
</section>

<section class="page">
  <div>
    <div class="hdr">
      <div>
        <div class="hdr-title">FARMERPRO RC.4 — REAL SANDBOX SCREENSHOTS &amp; SQLITE MIGRATION EVIDENCE</div>
        <div class="hdr-meta">Captured inside live Windows 11 Enterprise Sandbox (2030x1230 DPI-aware / 1624x984) with zero host dev tools installed</div>
      </div>
      <div class="hdr-badges"><span class="b-pass">STUDIO + PORTABLE + SQLITE VERIFIED</span><span class="b-fail">TEST F FAIL: 0xC0000135</span></div>
    </div>

    <div class="grid3" style="margin-bottom:8px;">
      <div class="shot-box"><img src="{shots['alpha']}"/><div class="shot-cap">Auto_P6Alpha.exe Standalone Before Studio (ADB: 0 OFFLINE)</div></div>
      <div class="shot-box"><img src="{shots['beta']}"/><div class="shot-cap">Auto_P6Beta.exe Standalone Before Studio (5 Tasks, Offline)</div></div>
      <div class="shot-box"><img src="{shots['studio_create']}"/><div class="shot-cap">Studio NSIS Installed: Created 'Gate1 Sandbox RPG' in SQLite</div></div>
      <div class="shot-box"><img src="{shots['studio_reopen']}"/><div class="shot-cap">Studio Restart: Project Reloaded from StudioData\\data\\farmerpro.db</div></div>
      <div class="shot-box"><img src="{shots['sql_clean']}"/><div class="shot-cap">RC.2 v5 DB Upgraded to v6 &amp; Loaded ('Gate1 RC2 Game', FK=0)</div></div>
      <div class="shot-box"><img src="{shots['online_fw']}"/><div class="shot-cap">Online Sandbox: Studio Launched + Windows Firewall adb Prompt</div></div>
    </div>

    <div class="grid2eq">
      <div class="card">
        <h3>Sandbox SQLite Upgrade Audit (Host sqlite3 PRAGMA Verification)</h3>
        <table>
          <tr><th>DB Snapshot Exported From Sandbox</th><th>Migrations</th><th>Cols</th><th>Integrity / FK</th><th>Preserved Rows</th></tr>
          <tr><td><code>studio_after_create_reopen.db</code></td><td><code>[1..=6]</code></td><td>18</td><td><code>ok / 0</code></td><td><code>Gate1 Sandbox RPG</code></td></tr>
          <tr><td><code>upgraded_rc2_v5_clean.db</code> (0/4)</td><td><code>[1..=6]</code></td><td>18</td><td><code>ok / 0</code></td><td><code>Gate1 RC2 Game</code>, <code>LEGACY_UNVERIFIED (v=0)</code></td></tr>
          <tr><td><code>upgraded_rc2_v5_partial_2cols.db</code> (2/4)</td><td><code>[1..=6]</code></td><td>18</td><td><code>ok / 0</code></td><td>Zero dup-col error; <code>LEGACY_UNVERIFIED (v=0)</code></td></tr>
          <tr><td><code>upgraded_rc2_v5_partial_3cols.db</code> (3/4)</td><td><code>[1..=6]</code></td><td>18</td><td><code>ok / 0</code></td><td>Zero dup-col error; <code>LEGACY_UNVERIFIED (v=0)</code></td></tr>
          <tr><td><code>portable_alpha/beta_standalone.db</code></td><td><code>[1..=6]</code></td><td>18</td><td><code>ok / -</code></td><td>Independent runtime DBs next to EXEs</td></tr>
        </table>
      </div>
      <div class="card">
        <h3>Clean Windows Verification Boundaries &amp; Next Actions</h3>
        <table>
          <tr><th>Item</th><th>Decision</th><th>Required Next Step</th></tr>
          <tr><td>Hotfix <code>VCRUNTIME140.dll</code></td><td><span class="b-fail">ACTION</span></td><td>Await approval to add <code>+crt-static</code> for <code>fp-updater</code> &amp; <code>fp-cli</code></td></tr>
          <tr><td>Test F (<code>fp-updater</code>) in Sandbox</td><td><span class="b-fail">RE-TEST</span></td><td>Re-run <code>v1.0.0 -&gt; v1.1.0 -&gt; Rollback</code> after hotfix approval</td></tr>
          <tr><td>Standard User &amp; No-WebView2</td><td><span class="b-blk">VM REQ</span></td><td>Verify on clean standalone Windows 10/11 VM outside Sandbox</td></tr>
          <tr><td>Gate 3 Commercial Signing</td><td><span class="b-blk">BLOCKED</span></td><td>Apply commercial CA certificate prior to PublicSigned release</td></tr>
        </table>
      </div>
    </div>
  </div>
  <div class="ftr">
    <span>FarmerPro v0.1.0-p6-rc.4 · Gate 1 Clean Windows Sandbox Evidence (Page 2 of 2)</span>
    <span>Stop at Gate 1 Acceptance — Awaiting Hotfix Approval</span>
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

for old in os.listdir(RENDER_DIR):
    if old.startswith("gate1_page_") and old.endswith(".png"):
        os.remove(os.path.join(RENDER_DIR, old))

doc = pymupdf.open(PDF_MAIN)
for idx, page in enumerate(doc, 1):
    pix = page.get_pixmap(dpi=140)
    pix.save(os.path.join(RENDER_DIR, f"gate1_page_{idx:02d}.png"))

print(json.dumps({
    "pdfPath": PDF_MAIN,
    "pdfCopyPath": PDF_COPY,
    "pages": len(doc),
    "bytes": os.path.getsize(PDF_MAIN),
    "sha256": sha256_file(PDF_MAIN),
}, indent=2))
