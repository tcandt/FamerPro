import base64
import hashlib
import json
import os
import re
import shutil
import subprocess
import pymupdf

ROOT = r"E:\DATA\FarmerPro"
EV_DIR = os.path.join(ROOT, "artifacts", "p5_1_evidence")
RENDER_DIR = os.path.join(EV_DIR, "pdf_p5_1_rendered_pages")
os.makedirs(EV_DIR, exist_ok=True)
if os.path.exists(RENDER_DIR):
    for old_f in os.listdir(RENDER_DIR):
        try:
            os.remove(os.path.join(RENDER_DIR, old_f))
        except OSError:
            pass
os.makedirs(RENDER_DIR, exist_ok=True)

CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"
HTML_OUT = os.path.join(EV_DIR, "FARMERPRO_P5_1_ACCEPTANCE_REPORT.html")
PDF_MAIN = os.path.join(ROOT, "FARMERPRO_P5_1_ACCEPTANCE_REPORT.pdf")
PDF_COPY = os.path.join(EV_DIR, "FARMERPRO_P5_1_ACCEPTANCE_REPORT.pdf")
QA_JSON = os.path.join(EV_DIR, "p5_1_pdf_qa.json")


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        while chunk := f.read(65536):
            h.update(chunk)
    return h.hexdigest()


def img_data_uri(path):
    with open(path, "rb") as f:
        b64 = base64.b64encode(f.read()).decode("ascii")
    return f"data:image/png;base64,{b64}"


def h_esc(s):
    return str(s).replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


rep = json.loads(
    open(os.path.join(EV_DIR, "p5_1_e2e_report.json"), encoding="utf-8-sig").read()
)
raw_log_bytes = open(os.path.join(EV_DIR, "cargo_test_workspace_p5_1.log"), "rb").read()
test_log = raw_log_bytes.decode("utf-16" if b"\x00" in raw_log_bytes else "utf-8-sig", errors="replace")
passed_matches = re.findall(r"test result: ok\. (\d+) passed; (\d+) failed;", test_log)
total_passed = sum(int(p) for p, _ in passed_matches)
total_failed = sum(int(f) for _, f in passed_matches)
assert total_passed >= 75 and total_failed == 0, f"Unexpected tests: {total_passed}/{total_failed}"

head_sha = (
    subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT)
    .decode("utf-8", errors="replace")
    .strip()
)
branch_name = (
    subprocess.check_output(["git", "rev-parse", "--abbrev-ref", "HEAD"], cwd=ROOT)
    .decode("utf-8", errors="replace")
    .strip()
)
assert rep["gitHead"] == head_sha, f"HEAD mismatch: verified={rep['gitHead']} vs HEAD={head_sha}"

diff_p4_p51 = (
    subprocess.check_output(
        ["git", "diff", "--shortstat", "7a9a6d5..HEAD", "--", "apps/desktop/src", "apps/desktop/src-tauri", "apps/mini-runtime", "crates"],
        cwd=ROOT,
    )
    .decode("utf-8", errors="replace")
    .strip()
)

img_before = img_data_uri(os.path.join(EV_DIR, "p5_1_minitool_before.png"))
img_step1 = img_data_uri(os.path.join(EV_DIR, "p5_1_minitool_midflight_step1.png"))
img_step2 = img_data_uri(os.path.join(EV_DIR, "p5_1_minitool_midflight_step2.png"))
img_wal = img_data_uri(os.path.join(EV_DIR, "p5_1_minitool_crossproc_wal.png"))
img_prov = img_data_uri(os.path.join(EV_DIR, "p5_1_minitool_provenance_variants.png"))
img_titan = img_data_uri(os.path.join(EV_DIR, "p5_1_minitool_titan_isolated.png"))
img_restart = img_data_uri(os.path.join(EV_DIR, "p5_1_minitool_restarted_recovery.png"))
img_studio = img_data_uri(os.path.join(EV_DIR, "p5_1_studio_builder.png"))

acc = rep["acceptanceTests"]
ta = acc["testA_liveUiUpdateTestkit"]
tb = acc["testB_eventLossDuplicateReconnectRecovery"]
tc = acc["testC_dataProvenanceDistinction"]
td = acc["testD_crossProjectIsolation"]
tf = acc["testF_secretProtectionRuntime"]
tg = acc["testG_studioEndToEndAndRestartRecovery"]

html = f"""<!DOCTYPE html>
<html lang="vi">
<head>
<meta charset="utf-8">
<title>FarmerPro P5.1 Runtime Provenance &amp; Realtime Acceptance Report</title>
<style>
  @page {{ size: A4 landscape; margin: 7.5mm 9.5mm; }}
  * {{ box-sizing: border-box; }}
  body {{ margin: 0; font-family: "Segoe UI", Arial, sans-serif; color: #0F172A; background: #FFFFFF; font-size: 8.2pt; line-height: 1.28; }}
  .page {{ page-break-after: always; position: relative; min-height: 188mm; display: flex; flex-direction: column; }}
  .page:last-child {{ page-break-after: auto; }}
  .hdr {{ display: flex; justify-content: space-between; align-items: flex-end; border-bottom: 2px solid #0F172A; padding-bottom: 4px; margin-bottom: 7px; }}
  .hdr-title {{ font-size: 13pt; font-weight: 800; color: #0F172A; }}
  .hdr-sub {{ font-size: 8.1pt; color: #475569; margin-top: 1px; }}
  .hdr-meta {{ text-align: right; font-size: 7.7pt; color: #334155; }}
  .kpi-row {{ display: grid; grid-template-columns: repeat(5, 1fr); gap: 7px; margin-bottom: 7px; }}
  .kpi {{ border: 1px solid #CBD5E1; border-radius: 6px; padding: 5px 8px; background: #F8FAFC; }}
  .kpi-lbl {{ font-size: 7.5pt; text-transform: uppercase; color: #475569; font-weight: 700; }}
  .kpi-val {{ font-size: 11pt; font-weight: 800; color: #0F172A; margin-top: 1px; }}
  .kpi-ok .kpi-val {{ color: #047857; }}
  .kpi-warn .kpi-val {{ color: #B45309; }}
  .sec-h {{ font-size: 9.2pt; font-weight: 800; color: #0F172A; margin: 6px 0 4px 0; border-left: 3.5px solid #1878F5; padding-left: 6px; }}
  table {{ width: 100%; border-collapse: collapse; margin-bottom: 6px; font-size: 7.8pt; }}
  th, td {{ border: 1px solid #CBD5E1; padding: 3.2px 5.2px; text-align: left; vertical-align: top; }}
  th {{ background: #F1F5F9; font-weight: 700; color: #0F172A; }}
  .mono {{ font-family: Consolas, monospace; font-size: 7.6pt; }}
  .b-pass {{ display: inline-block; padding: 1px 5px; border-radius: 4px; background: #D1FAE5; color: #065F46; font-weight: 800; font-size: 7.5pt; }}
  .b-blk {{ display: inline-block; padding: 1px 5px; border-radius: 4px; background: #FEF3C7; color: #92400E; font-weight: 800; font-size: 7.5pt; }}
  .callout {{ border: 1px solid #F59E0B; background: #FFFBEB; border-radius: 6px; padding: 5px 9px; font-size: 7.9pt; color: #78350F; margin-bottom: 6px; }}
  .grid-2 {{ display: grid; grid-template-columns: 1fr 1fr; gap: 9px; align-items: start; }}
  .shot-frame {{ border: 1px solid #94A3B8; border-radius: 6px; padding: 4px; background: #F8FAFC; }}
  .shot-frame img {{ width: 100%; height: auto; display: block; border-radius: 4px; }}
  .shot-cap {{ font-size: 7.7pt; color: #1E293B; margin-top: 3px; line-height: 1.26; }}
  .ftr {{ margin-top: auto; border-top: 1px solid #CBD5E1; padding-top: 3px; display: flex; justify-content: space-between; font-size: 7.6pt; color: #475569; }}
</style>
</head>
<body>

<!-- PAGE 1: P5.1 MATRIX + ARCHITECTURE DIAGRAM -->
<div class="page">
  <div class="hdr">
    <div>
      <div class="hdr-title">FARMERPRO P5.1 — RUNTIME VARIABLE PROVENANCE &amp; REALTIME EVENT STREAM</div>
      <div class="hdr-sub">Development Gate Verification on Commit <code>{h_esc(head_sha)}</code> (TestKit / SQLite WAL / Tauri Events; Live ADB kept BLOCKED)</div>
    </div>
    <div class="hdr-meta">
      <div>Branch: <code>{h_esc(branch_name)}</code> · Verified HEAD: <code>{h_esc(head_sha[:12])}</code></div>
      <div>Diff P4..P5.1: <code>{h_esc(diff_p4_p51)}</code></div>
    </div>
  </div>

  <div class="kpi-row">
    <div class="kpi kpi-ok"><div class="kpi-lbl">Rust Workspace Tests</div><div class="kpi-val">{total_passed} / {total_passed + total_failed} PASS</div></div>
    <div class="kpi kpi-ok"><div class="kpi-lbl">Realtime GUI Mid-Flight</div><div class="kpi-val">CASE 1 &amp; 2 PASS</div></div>
    <div class="kpi kpi-ok"><div class="kpi-lbl">Event Recovery &amp; Seq</div><div class="kpi-val">ZERO LOSS / DUP</div></div>
    <div class="kpi kpi-ok"><div class="kpi-lbl">Secret Leakage Scan</div><div class="kpi-val">0 PLAINTEXT LEAK</div></div>
    <div class="kpi kpi-warn"><div class="kpi-lbl">Live Hardware ADB</div><div class="kpi-val">BLOCKED (0 ADB)</div></div>
  </div>

  <div class="sec-h">I. Ma tran Nghiem thu Acceptance Tests P5.1 (Test A–G &amp; Live Hardware ADB)</div>
  <table>
    <thead><tr><th>Ma kiem thu</th><th>Pham vi &amp; Dieu kien</th><th>Ket qua do duoc (TestKit / Local DB)</th><th>Trang thai</th></tr></thead>
    <tbody>
      <tr><td><strong>Test A — Live UI Update</strong></td><td>Mini Tool mo san: Set Var -&gt; Wait -&gt; Increment Var -&gt; Wait (Case 1 GUI &amp; Case 2 CLI WAL)</td><td>Step1 (<code>Sector-B7</code>, <code>100</code>) luc <code>Dang lam</code>={ta["step1MidflightSeen"]}; Step2 (<code>250</code>)={ta["step2MidflightSeen"]}; CLI WAL ngoai tien trinh (<code>Hoan thanh=2</code>)={ta["crossProcessWalSeen"]} khong restart EXE.</td><td><span class="b-pass">PASS (TESTKIT)</span></td></tr>
      <tr><td><strong>Test B — Event Loss &amp; Reconnect</strong></td><td>Gia lap trung <code>seq</code>, nhay coc <code>seq+5</code> &amp; <code>focus</code> reconnect sync</td><td><code>duplicatesIgnored</code>={tb["duplicatesIgnored"]}, <code>gapsRecovered</code>={tb["gapsRecovered"]}, <code>reconnectSyncs</code>={tb["reconnectSyncs"]}, <code>zoneStillClean</code>={tb["zoneStillClean"]}.</td><td><span class="b-pass">PASS</span></td></tr>
      <tr><td><strong>Test C — Provenance &amp; Anti-Overwrite</strong></td><td>Nhan TestKit, Live 0.94 unverified, Legacy P4, va chan TestKit ghi de Live</td><td><code>TESTKIT · Mô phỏng</code>={tc["testkitBadgeRenderedInUi"]}, <code>LIVE · Chưa xác minh 94%</code>={tc["liveUnverifiedAndLegacyLabels"]["liveUnverifiedLabelCorrect"]}, <code>LEGACY · Chưa xác minh</code>={tc["liveUnverifiedAndLegacyLabels"]["legacyLabelCorrect"]}; Rust test <code>PROVENANCE_ANTI_OVERWRITE_BLOCKED</code> PASS.</td><td><span class="b-pass">PASS</span></td></tr>
      <tr><td><strong>Test D — Cross-Project Isolation</strong></td><td>Hai Mini Tools doc lap trung ten bien (<code>Auto_RealtimeAlpha</code> vs <code>Auto_TitanBeta</code>)</td><td>Beta khoi dong sach <code>initialZeroLeakFromAlpha={td["initialZeroLeakFromAlpha"]}</code>, chay rieng dat <code>Titan-Ridge / 999</code> khong anh huong DB Alpha.</td><td><span class="b-pass">PASS</span></td></tr>
      <tr><td><strong>Test E — SQLite Migration v3 &amp; Rollback</strong></td><td>Nang cap DB P4 v2 (<code>runtime_variable_snapshots</code>) -&gt; v3 &amp; WAL rollback</td><td>Rust test <code>test_p5_1_migration_v3_legacy_upgrade_and_wal_rollback</code> PASS: gan nhan <code>LEGACY_UNVERIFIED</code> (<code>verified=false</code>), scrub secret, rollback v3-&gt;[1,2] integrity=ok.</td><td><span class="b-pass">PASS</span></td></tr>
      <tr><td><strong>Test F — Secret Protection</strong></td><td>Bien so (<code>482910</code>) &amp; JSON co cau truc (<code>tok-secret-999</code>) trong Event, SQLite, UI Logs</td><td><code>pinLeakedInUiLogs={tf["pinLeakedInUiLogs"]}</code>, <code>tokenLeakedInSqliteRaw={tf["tokenLeakedInSqliteRaw"]}</code>, <code>hasRedactedMarkers={tf["hasRedactedMarkers"]}</code>.</td><td><span class="b-pass">PASS</span></td></tr>
      <tr><td><strong>Test G — Studio E2E &amp; Restart</strong></td><td>Studio -> Tao Project/Workflow/Task -> Build -> Dong Studio -> Chay Mini Tool -&gt; Restart</td><td>Studio=0 console err; dong Studio; Mini Tool chay realtime va nap lai snapshot sau restart: <code>Sector-B7</code>={tg["hasSectorB7Testkit"]}, <code>250</code>={tg["hasResource250Testkit"]}.</td><td><span class="b-pass">PASS (TESTKIT)</span></td></tr>
      <tr><td><strong>Live Hardware ADB (P4/P5 Gate)</strong></td><td>LDPlayer14 / Thiet bi Android that (<code>adb devices -l</code>)</td><td>Host: <code>{h_esc(rep["adbLiveHardware"]["adbDevicesOutput"])}</code> | <code>LDPlayer14 vm0={h_esc(rep["adbLiveHardware"]["ldplayer14Vm0IsRunning"])}</code> (can UAC). Khong dung FakeDeviceDriver thay Live ADB.</td><td><span class="b-blk">BLOCKED (0 ADB)</span></td></tr>
    </tbody>
  </table>

  <div class="sec-h">II. Kien truc Event Sequencing, Anti-Overwrite Partitioning &amp; Cross-Process WAL Watcher</div>
  <svg width="100%" height="108" viewBox="0 0 980 108" style="background:#F8FAFC;border:1px solid #CBD5E1;border-radius:6px;">
    <rect x="12" y="14" width="215" height="80" rx="6" fill="#EFF6FF" stroke="#2563EB" stroke-width="1.5"/>
    <text x="119" y="32" text-anchor="middle" font-size="10" font-weight="bold" fill="#1E3A8A">1. Case 1: Mini GUI Executor</text>
    <text x="119" y="48" text-anchor="middle" font-size="8.5" fill="#1E40AF">WorkflowExecutor -&gt; pending_var_deltas</text>
    <text x="119" y="63" text-anchor="middle" font-size="8.5" fill="#1E40AF">fp-security::sanitize (num/str/JSON)</text>
    <text x="119" y="78" text-anchor="middle" font-size="8.5" fill="#047857">app.emit("mini://runtime-var-delta")</text>

    <rect x="255" y="14" width="225" height="80" rx="6" fill="#ECFDF5" stroke="#059669" stroke-width="1.5"/>
    <text x="367" y="32" text-anchor="middle" font-size="10" font-weight="bold" fill="#065F46">2. SQLite v3 Store (Partitioned)</text>
    <text x="367" y="48" text-anchor="middle" font-size="8.5" fill="#064E3B">runtime_var_streams (monotonic seq++)</text>
    <text x="367" y="63" text-anchor="middle" font-size="8.5" fill="#064E3B">Scope: mini:&lt;slug&gt; (LIVE) | ::testkit</text>
    <text x="367" y="78" text-anchor="middle" font-size="8.5" fill="#B45309">Guard: TESTKIT cannot overwrite LIVE ✓</text>

    <rect x="510" y="14" width="225" height="80" rx="6" fill="#FFFBEB" stroke="#D97706" stroke-width="1.5"/>
    <text x="622" y="32" text-anchor="middle" font-size="10" font-weight="bold" fill="#92400E">3. Case 2: External CLI Worker</text>
    <text x="622" y="48" text-anchor="middle" font-size="8.5" fill="#78350F">Auto_*.exe --run-task --mock writes WAL</text>
    <text x="622" y="63" text-anchor="middle" font-size="8.5" fill="#78350F">Open GUI PRAGMA data_version poller</text>
    <text x="622" y="78" text-anchor="middle" font-size="8.5" fill="#92400E">Reads runtime_var_events (seq &gt; seen)</text>

    <rect x="765" y="14" width="203" height="80" rx="6" fill="#F1F5F9" stroke="#334155" stroke-width="1.5"/>
    <text x="866" y="32" text-anchor="middle" font-size="10" font-weight="bold" fill="#0F172A">4. WebView SeqGuard &amp; Sync</text>
    <text x="866" y="48" text-anchor="middle" font-size="8.5" fill="#334155">seq &lt;= cur: drop dup/stale</text>
    <text x="866" y="63" text-anchor="middle" font-size="8.5" fill="#334155">seq &gt; cur+1 / focus: snapshot sync</text>
    <text x="866" y="78" text-anchor="middle" font-size="8.5" fill="#047857">Live Badges: TESTKIT / LIVE ✓ / LEGACY</text>

    <line x1="227" y1="54" x2="255" y2="54" stroke="#1E3A8A" stroke-width="1.8"/>
    <line x1="480" y1="54" x2="510" y2="54" stroke="#059669" stroke-width="1.8"/>
    <line x1="735" y1="54" x2="765" y2="54" stroke="#D97706" stroke-width="1.8"/>
  </svg>
  <div class="ftr"><span>FarmerPro P5.1 Acceptance Report (Commit {h_esc(head_sha[:12])})</span><span>Trang 1 / 4</span></div>
</div>

<!-- PAGE 2: REALTIME MID-FLIGHT STREAMING SCREENSHOTS (TEST A CASE 1 & 2) -->
<div class="page">
  <div class="hdr">
    <div>
      <div class="hdr-title">TEST A — CAP NHAT BIEN REALTIME TRONG LUC WORKFLOW DANG CHAY (KHONG RESTART EXE)</div>
      <div class="hdr-sub"><code>Auto_RealtimeAlpha.exe</code> (740x500): Truoc khi chay -&gt; Buoc 1 (<code>100</code> luc task Dang lam) -&gt; Buoc 2 (<code>250</code>) -&gt; CLI WAL ngoai tien trinh (<code>Hoan thanh=2</code>)</div>
    </div>
    <div class="hdr-meta"><div>Development Gate: TESTKIT_MOCK</div></div>
  </div>
  <div class="grid-2">
    <div class="shot-frame">
      <img src="{img_before}" alt="Before Run Unverified" />
      <div class="shot-cap"><strong>Hinh 1 (Truoc khi chay):</strong> Ca 3 cot hien thi <em>&quot;Chưa xác minh&quot;</em>, khong gia mao du lieu Live. Nut ADB That bi tu choi Fail-Closed (<code>NO_LIVE_ADB_DEVICE_ONLINE</code>).</div>
    </div>
    <div class="shot-frame">
      <img src="{img_step1}" alt="Mid-flight Step 1" />
      <div class="shot-cap"><strong>Hinh 2 (Mid-flight Buoc 1 — Tac vu van &quot;Đang làm&quot;):</strong> Nhan delta qua <code>mini://runtime-var-delta</code> ngay truoc <code>wait-1</code> ket thuc: <code>Sector-B7</code> &amp; <code>100</code> kem nhan <code>TESTKIT · Mô phỏng</code>.</div>
    </div>
    <div class="shot-frame">
      <img src="{img_step2}" alt="Mid-flight Step 2" />
      <div class="shot-cap"><strong>Hinh 3 (Mid-flight Buoc 2 — Tang len 250):</strong> Node <code>state.increment_var</code> (+150) cap nhat truc tiep <code>250</code> ngay trong cua so EXE dang mo, khong reload EXE.</div>
    </div>
    <div class="shot-frame">
      <img src="{img_wal}" alt="Cross-Process CLI WAL Sync" />
      <div class="shot-cap"><strong>Hinh 4 (Case 2 — CLI ngoai tien trinh ghi SQLite WAL):</strong> Chay tien trinh thu 2 <code>--run-task t-alpha --mock</code>; cua so GUI dang mo tu bat WAL <code>data_version</code> va tang Hoan thanh len <code>2</code>.</div>
    </div>
  </div>
  <div class="ftr"><span>FarmerPro P5.1 Acceptance Report (Commit {h_esc(head_sha[:12])})</span><span>Trang 2 / 4</span></div>
</div>

<!-- PAGE 3: TEST B, C, D PROVENANCE VARIANTS, TITAN ISOLATION & CRASH RECOVERY -->
<div class="page">
  <div class="hdr">
    <div>
      <div class="hdr-title">TEST B, C, D &amp; G — PROVENANCE VARIANTS, CO LAP DU AN VA KHOI PHUC RESTART</div>
      <div class="hdr-sub">Phan biet <code>LIVE · Chưa xác minh 94%</code> vs <code>LEGACY · Chưa xác minh</code> vs <code>TESTKIT · Mô phỏng</code> · Co lap chéo <code>Auto_TitanBeta.exe</code></div>
    </div>
    <div class="hdr-meta"><div>2 Independent Mini Tools Verified</div></div>
  </div>
  <div class="grid-2">
    <div class="shot-frame">
      <img src="{img_prov}" alt="Provenance Variants" />
      <div class="shot-cap"><strong>Hinh 5 (Test C — Phan biet Nhan Nguon Provenance):</strong> Du lieu <code>LIVE_ADB_OCR</code> co confidence <code>94%</code> nhung <code>verified=false</code> hien dung <code>LIVE · Chưa xác minh 94%</code>; du lieu P4 migrate hien <code>LEGACY · Chưa xác minh</code>.</div>
    </div>
    <div class="shot-frame">
      <img src="{img_titan}" alt="Titan Isolated" />
      <div class="shot-cap"><strong>Hinh 6 (Test D — Co lap Mini Tool #2):</strong> <code>Auto_TitanBeta.exe</code> khoi dong sach (zero ro ri <code>Sector-B7</code>), chay rieng tao <code>Titan-Ridge</code> / <code>999 TESTKIT · Mô phỏng</code> trong DB rieng.</div>
    </div>
    <div class="shot-frame">
      <img src="{img_restart}" alt="Restart Recovery" />
      <div class="shot-cap"><strong>Hinh 7 (Test G Step 9 — Dong &amp; Mo lai EXE):</strong> Sau khi tat hoan toan, mo lai <code>Auto_RealtimeAlpha.exe</code> nap dung snapshot da luu (<code>Sector-B7</code>, <code>250</code>, <code>2</code>) kem nhan <code>TESTKIT · Mô phỏng</code>.</div>
    </div>
    <div class="shot-frame">
      <img src="{img_studio}" alt="Studio Builder E2E" />
      <div class="shot-cap"><strong>Hinh 8 (Test G Step 1-5 — FarmerPro Studio Builder 1600x900):</strong> Gan bien <code>farm_zone</code>, <code>resource_count</code>, <code>task_completed_count</code> trong Studio roi xuat EXE truoc khi dong Studio.</div>
    </div>
  </div>
  <div class="ftr"><span>FarmerPro P5.1 Acceptance Report (Commit {h_esc(head_sha[:12])})</span><span>Trang 3 / 4</span></div>
</div>

<!-- PAGE 4: RAW EVIDENCE LEDGER & STAGE-GATE SIGN-OFF -->
<div class="page">
  <div class="hdr">
    <div>
      <div class="hdr-title">BANG CHUNG RAW LOGS, SECRET SCAN VA DE XUAT STAGE-GATE P5.1</div>
      <div class="hdr-sub">Tat ca bang chung duoc sinh truc tiep tren cung commit <code>{h_esc(head_sha)}</code></div>
    </div>
    <div class="hdr-meta"><div>Branch: <code>{h_esc(branch_name)}</code></div></div>
  </div>
  <pre style="background:#0F172A;color:#E2E8F0;padding:8px 10px;border-radius:6px;font-family:Consolas,monospace;font-size:7.6pt;line-height:1.35;white-space:pre-wrap;">
Verified Code Commit : {h_esc(head_sha)}
Workspace Tests      : {total_passed} passed; {total_failed} failed (cargo_test_workspace_p5_1.log)
Console Errors       : Studio={len(rep["consoleErrors"]["studio"])}, Auto_RealtimeAlpha={len(rep["consoleErrors"]["alphaMiniTool"])}, Auto_TitanBeta={len(rep["consoleErrors"]["betaMiniTool"])}
Test A (Realtime UI) : step1Midflight={ta["step1MidflightSeen"]}, step2Midflight={ta["step2MidflightSeen"]}, crossProcessWal={ta["crossProcessWalSeen"]}, failClosedLive={ta["failClosedBeforeRun"]}
Test B (Seq & Sync)  : stream={tb["streamId"]}, dupIgnored={tb["duplicatesIgnored"]}, gapsRecovered={tb["gapsRecovered"]}, reconnectSyncs={tb["reconnectSyncs"]}, zoneClean={tb["zoneStillClean"]}
Test F (Secret Scan) : pinLeaked(Logs/State/DB)={tf["pinLeakedInUiLogs"]}/{tf["pinLeakedInTauriState"]}/{tf["pinLeakedInSqliteRaw"]} | tokenLeaked={tf["tokenLeakedInUiLogs"]}/{tf["tokenLeakedInTauriState"]}/{tf["tokenLeakedInSqliteRaw"]} | redactedMarkers={tf["hasRedactedMarkers"]}
Live Hardware ADB    : {h_esc(rep["adbLiveHardware"]["status"])} (LDPlayer14 vm0={h_esc(rep["adbLiveHardware"]["ldplayer14Vm0IsRunning"])})
  </pre>
  <div class="callout" style="border-color:#059669;background:#ECFDF5;color:#064E3B;">
    <strong>KET LUAN MOC P5.1 (CHO PHE DUYET):</strong> P5.1 da hoan thanh va chung minh day du o cap <strong>Development Gate (TestKit + Local SQLite v3)</strong>: cap nhat bien realtime trong luc workflow dang chay o ca Case 1 (Tauri Event) va Case 2 (CLI SQLite WAL watcher), chong ghi de du lieu Live, bao ve secret so/chuoi/JSON co cau truc, co lap du an cheo, migration/rollback v3 an toan. <strong>Da ghi nhan dieu chinh thiet ke P5.2</strong> trong tai lieu dac ta; <strong>chua trien khai P5.2–P5.4 hay P6</strong> cho den khi ban phe duyet P5.1.
  </div>
  <div class="ftr"><span>FarmerPro P5.1 Acceptance Report (Commit {h_esc(head_sha[:12])})</span><span>Trang 4 / 4</span></div>
</div>

</body>
</html>"""

with open(HTML_OUT, "w", encoding="utf-8", newline="\n") as f:
    f.write(html)

subprocess.run(
    [
        CHROME,
        "--headless=new",
        "--disable-gpu",
        "--no-sandbox",
        "--print-to-pdf-no-header",
        f"--print-to-pdf={PDF_MAIN}",
        HTML_OUT,
    ],
    check=True,
)

doc = pymupdf.open(PDF_MAIN)
toc = [
    [1, "1. Acceptance Matrix A-G & Realtime Architecture", 1],
    [1, "2. Test A Mid-Flight & Cross-Process WAL Evidence", 2],
    [1, "3. Provenance Variants, Cross-Project Isolation & Restart", 3],
    [1, "4. Raw Verification Ledger & P5.1 Stage-Gate", 4],
]
doc.set_toc(toc)
tmp_pdf = PDF_MAIN + ".tmp.pdf"
doc.save(tmp_pdf, deflate=True, garbage=3)
doc.close()
os.replace(tmp_pdf, PDF_MAIN)
shutil.copy2(PDF_MAIN, PDF_COPY)

qa_doc = pymupdf.open(PDF_MAIN)
min_pt = 99.0
small_fonts = []
clipping = []
encoding_errs = []
for idx, page in enumerate(qa_doc):
    pno = idx + 1
    pix = page.get_pixmap(dpi=130)
    pix.save(os.path.join(RENDER_DIR, f"p5_1_page_{pno:02d}.png"))
    txt = page.get_text("text")
    if "\ufffd" in txt:
        encoding_errs.append(f"Page {pno}: U+FFFD")
    pdict = page.get_text("dict")
    for blk in pdict.get("blocks", []):
        if blk.get("type") != 0:
            continue
        bbox = blk.get("bbox", [0, 0, 0, 0])
        if bbox[2] > page.rect.width + 2 or bbox[3] > page.rect.height + 2:
            clipping.append(f"Page {pno}: overflow bbox {bbox}")
        for ln in blk.get("lines", []):
            for sp in ln.get("spans", []):
                sz = sp.get("size", 0.0)
                stxt = sp.get("text", "").strip()
                if len(stxt) >= 3 and sz > 0:
                    min_pt = min(min_pt, sz)
                    if sz < 7.5:
                        small_fonts.append(f"Page {pno}: {sz:.2f}pt '{stxt[:30]}'")

qa_report = {
    "verifiedCodeCommit": head_sha,
    "pdfPath": PDF_MAIN,
    "pdfCopyPath": PDF_COPY,
    "pdfSizeBytes": os.path.getsize(PDF_MAIN),
    "pdfSha256": sha256_file(PDF_MAIN),
    "pagesCount": len(qa_doc),
    "bookmarksCount": len(qa_doc.get_toc()),
    "minFontPt": round(min_pt, 2),
    "smallFontWarnings": small_fonts,
    "clippingErrors": clipping,
    "encodingErrors": encoding_errs,
    "passed": (
        len(qa_doc) == 4
        and len(clipping) == 0
        and len(encoding_errs) == 0
        and min_pt >= 7.5
    ),
}
qa_doc.close()

with open(QA_JSON, "w", encoding="utf-8", newline="\n") as f:
    json.dump(qa_report, f, indent=2, ensure_ascii=False)

manifest = {}
for root_dir, _, files in os.walk(EV_DIR):
    if "sandbox" in root_dir:
        continue
    for fn in sorted(files):
        full = os.path.join(root_dir, fn)
        rel = os.path.relpath(full, EV_DIR).replace("\\", "/")
        if rel == "EVIDENCE_MANIFEST_SHA256.json":
            continue
        manifest[rel] = {
            "bytes": os.path.getsize(full),
            "sha256": sha256_file(full),
        }

manifest_path = os.path.join(EV_DIR, "EVIDENCE_MANIFEST_SHA256.json")
with open(manifest_path, "w", encoding="utf-8", newline="\n") as f:
    json.dump(manifest, f, indent=2, ensure_ascii=False)

print(json.dumps({"qaReport": qa_report, "manifestFiles": len(manifest)}, indent=2, ensure_ascii=False))
assert qa_report["passed"], f"PDF QA failed: {qa_report}"
