import base64
import hashlib
import json
import os
import re
import shutil
import subprocess
import pymupdf

ROOT = r"E:\DATA\FarmerPro"
EV_DIR = os.path.join(ROOT, "artifacts", "p4_7_evidence")
RENDER_DIR = os.path.join(EV_DIR, "pdf_p4_7_rendered_pages")
os.makedirs(EV_DIR, exist_ok=True)
if os.path.exists(RENDER_DIR):
    for old_f in os.listdir(RENDER_DIR):
        try:
            os.remove(os.path.join(RENDER_DIR, old_f))
        except OSError:
            pass
os.makedirs(RENDER_DIR, exist_ok=True)

CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"
HTML_OUT = os.path.join(EV_DIR, "FARMERPRO_P4_7_ACCEPTANCE_REPORT.html")
PDF_MAIN = os.path.join(ROOT, "FARMERPRO_P4_7_ACCEPTANCE_REPORT.pdf")
PDF_COPY = os.path.join(EV_DIR, "FARMERPRO_P4_7_ACCEPTANCE_REPORT.pdf")
QA_JSON = os.path.join(EV_DIR, "p4_7_pdf_qa_report.json")


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


p47 = json.loads(
    open(os.path.join(EV_DIR, "p4_7_verification_report.json"), encoding="utf-8-sig").read()
)
raw_log_bytes = open(os.path.join(EV_DIR, "cargo_test_workspace_p4_7.log"), "rb").read()
test_log = raw_log_bytes.decode("utf-16" if b"\x00" in raw_log_bytes else "utf-8-sig", errors="replace")
passed_matches = re.findall(r"test result: ok\. (\d+) passed; (\d+) failed;", test_log)
total_passed = sum(int(p) for p, _ in passed_matches)
total_failed = sum(int(f) for _, f in passed_matches)
assert total_passed >= 70 and total_failed == 0, f"Unexpected tests: {total_passed}/{total_failed}"

code_sha = (
    subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT)
    .decode("utf-8", errors="replace")
    .strip()
)
branch_name = (
    subprocess.check_output(["git", "rev-parse", "--abbrev-ref", "HEAD"], cwd=ROOT)
    .decode("utf-8", errors="replace")
    .strip()
)

img_builder = img_data_uri(os.path.join(EV_DIR, "p4_7_builder_wysiwyg_1600x900.png"))
img_omega = img_data_uri(os.path.join(EV_DIR, "p4_7_minitool_omega_compact.png"))
img_titan = img_data_uri(os.path.join(EV_DIR, "p4_7_minitool_titan_expanded.png"))
img_studio_1280 = img_data_uri(os.path.join(EV_DIR, "p4_7_studio_1280x720.png"))
img_studio_1600 = img_data_uri(os.path.join(EV_DIR, "p4_7_studio_1600x900.png"))

omega = p47["omegaStandalone"]
titan = p47["titanStandalone"]
wys = p47["wysiwygReport"]

parity_rows = [
    ("Workflow Library (10 luong JX2)", "WorkflowsView + Import Legacy Modal", "TESTED (Preserved)", "Van giu route `workflows` + nut Import Legacy sang GameProject trong `ProjectsDashboardView.tsx`."),
    ("Multi-Project & Dashboard", "ProjectsDashboardView + ProjectModals", "TESTED", "CRUD, Clone, Delete, SQLite WAL Integrity `FK=0`, khong con hardcode JX2 mac dinh."),
    ("Device Management & LDPlayer", "DevicesView + DeviceLabInspector", "PARTIAL / BLOCKED Live", "USB/IP scan, Health, ROI TestKit, Port Map hoat dong; Live ADB & LDPlayer `dnconsole.exe` BLOCKED (0 dev / UAC)."),
    ("Workflow Studio Canvas", "WorkflowStudioView + CustomNode + Inspector", "TESTED (TestKit)", "Dong bo Project, Breakpoint, Coordinator, Doc. Toi uu `1280x720` (an MiniMap <1536px, tang tuong phan node)."),
    ("Task Planner", "TasksView (Project Tasks & Modules)", "REPLACED / TESTED", "Thay danh sach JX2 tinh bang mapping Task -> Workflow trong SQLite `project_tasks` + Module Matrix."),
    ("Runtime Monitor & Checkpoints", "RunsView (Test & Debug Lab)", "REPLACED / TESTED", "Thay `job-demo-8492` bang `project_jobs`, `device_leases`, `step_events` & Resume Checkpoint that."),
    ("Credential Vault", "AccountsView", "REPLACED / TESTED", "Thay mock `Farmer_Main_01` bang `project_accounts` + tham chieu DPAPI (`secretRef`)."),
    ("Proxy & Network Gateway", "NetworkView", "REPLACED / TESTED", "Thay fake ping 38ms bang `studio_probe_project_proxy` (TCP/SOCKS5/HTTP) & ADB port list that."),
    ("Mini Tool Builder & Exporter", "MiniToolBuilderView + fp-mini-runtime", "TESTED (WYSIWYG 1:1)", "Add/Remove/Hide/Reorder Component, Task priority, Column binding, 5 states (`IDLE..ERROR`), Reload & Export EXE."),
    ("Settings & Design Tokens", "SettingsView + index.css tokens", "IMPLEMENTED", "Dung dung bang mau P1.2 tokens; button VACUUM trong Settings chua goi IPC Optimize rieng (`PARTIAL`)."),
]

parity_html = ""
for mod, loc, st, note in parity_rows:
    cls = "b-pass" if "TESTED" in st or "IMPLEMENTED" in st else "b-blk"
    if "REPLACED" in st:
        cls = "b-cond"
    parity_html += f"<tr><td><strong>{h_esc(mod)}</strong></td><td class='mono'>{h_esc(loc)}</td><td><span class='{cls}'>{h_esc(st)}</span></td><td>{h_esc(note)}</td></tr>"

html = f"""<!DOCTYPE html>
<html lang="vi">
<head>
<meta charset="utf-8">
<title>FarmerPro P4.7 Integration &amp; Verification Report</title>
<style>
  @page {{ size: A4 landscape; margin: 7mm 9mm; }}
  * {{ box-sizing: border-box; }}
  body {{ margin: 0; font-family: "Segoe UI", Arial, sans-serif; color: #0F172A; background: #FFFFFF; font-size: 8.3pt; line-height: 1.28; }}
  .page {{ page-break-after: always; position: relative; height: 194mm; display: flex; flex-direction: column; }}
  .page:last-child {{ page-break-after: auto; }}
  .hdr {{ display: flex; justify-content: space-between; align-items: flex-end; border-bottom: 2px solid #0F172A; padding-bottom: 4px; margin-bottom: 7px; flex-shrink: 0; }}
  .hdr-title {{ font-size: 13pt; font-weight: 800; color: #0F172A; }}
  .hdr-sub {{ font-size: 8.2pt; color: #475569; margin-top: 1px; }}
  .hdr-meta {{ text-align: right; font-size: 7.8pt; color: #334155; }}
  .kpi-row {{ display: grid; grid-template-columns: repeat(5, 1fr); gap: 7px; margin-bottom: 7px; flex-shrink: 0; }}
  .kpi {{ border: 1px solid #CBD5E1; border-radius: 6px; padding: 5px 8px; background: #F8FAFC; }}
  .kpi-lbl {{ font-size: 7.6pt; text-transform: uppercase; color: #475569; font-weight: 700; }}
  .kpi-val {{ font-size: 11.2pt; font-weight: 800; color: #0F172A; margin-top: 1px; }}
  .kpi-ok .kpi-val {{ color: #047857; }}
  .kpi-warn .kpi-val {{ color: #B45309; }}
  .sec-h {{ font-size: 9.5pt; font-weight: 800; color: #0F172A; margin: 4px 0; border-left: 3.5px solid #1878F5; padding-left: 6px; flex-shrink: 0; }}
  table {{ width: 100%; border-collapse: collapse; margin-bottom: 5px; font-size: 7.9pt; }}
  th, td {{ border: 1px solid #CBD5E1; padding: 3.5px 5.5px; text-align: left; vertical-align: top; }}
  th {{ background: #F1F5F9; font-weight: 700; color: #0F172A; }}
  .mono {{ font-family: Consolas, monospace; font-size: 7.6pt; }}
  .b-pass {{ display: inline-block; padding: 1px 5px; border-radius: 4px; background: #D1FAE5; color: #065F46; font-weight: 800; font-size: 7.5pt; }}
  .b-blk {{ display: inline-block; padding: 1px 5px; border-radius: 4px; background: #FEF3C7; color: #92400E; font-weight: 800; font-size: 7.5pt; }}
  .b-cond {{ display: inline-block; padding: 1px 5px; border-radius: 4px; background: #DBEAFE; color: #1E40AF; font-weight: 800; font-size: 7.5pt; }}
  .grid-2 {{ display: grid; grid-template-columns: 1fr 1fr; gap: 10px; flex: 1; min-height: 0; }}
  .shot-box {{ border: 1px solid #CBD5E1; border-radius: 6px; padding: 5px; background: #0B1220; color: #E2E8F0; display: flex; flex-direction: column; min-height: 0; }}
  .shot-box img {{ width: 100%; flex: 1; min-height: 0; object-fit: contain; display: block; border-radius: 4px; background: #090D16; }}
  .shot-cap {{ font-size: 7.8pt; color: #CBD5E1; margin-top: 4px; line-height: 1.28; flex-shrink: 0; }}
  .callout {{ border: 1px solid #F59E0B; background: #FFFBEB; border-radius: 6px; padding: 6px 9px; font-size: 8pt; color: #78350F; margin-bottom: 6px; flex-shrink: 0; }}
  .ftr {{ margin-top: auto; border-top: 1px solid #CBD5E1; padding-top: 3px; display: flex; justify-content: space-between; font-size: 7.6pt; color: #475569; flex-shrink: 0; }}
</style>
</head>
<body>

<!-- PAGE 1 -->
<div class="page">
  <div class="hdr">
    <div>
      <div class="hdr-title">FARMERPRO P4.7 — FINAL STUDIO INTEGRATION, WYSIWYG &amp; REGRESSION REPORT</div>
      <div class="hdr-sub">Bo sung nghiem thu P4.7 theo Quyet dinh Stage-Gate P4 · Tach bach STANDALONE_LAUNCH_PASS vs LIVE_WORKFLOW_EXECUTION_PASS</div>
    </div>
    <div class="hdr-meta">
      <div>Branch: <code>{h_esc(branch_name)}</code> · HEAD: <code>{h_esc(code_sha[:12])}</code></div>
      <div>Verified At: <code>{h_esc(p47["executedAtUtc"])}</code></div>
    </div>
  </div>

  <div class="kpi-row">
    <div class="kpi kpi-ok"><div class="kpi-lbl">Rust Workspace Tests</div><div class="kpi-val">{total_passed} / {total_passed + total_failed} PASS</div></div>
    <div class="kpi kpi-ok"><div class="kpi-lbl">WYSIWYG Save/Reload</div><div class="kpi-val">100% PARITY</div></div>
    <div class="kpi kpi-ok"><div class="kpi-lbl">Standalone Mini Tools</div><div class="kpi-val">2 / 2 LAUNCHED</div></div>
    <div class="kpi kpi-ok"><div class="kpi-lbl">Sidebar UI Regression</div><div class="kpi-val">9 / 9 · 0 ERR</div></div>
    <div class="kpi kpi-warn"><div class="kpi-lbl">Live ADB Execution</div><div class="kpi-val">BLOCKED (UAC/0 DEV)</div></div>
  </div>

  <div class="callout">
    <strong>TRUNG THUC VE LIVE HARDWARE ADB (MUC I P4.7):</strong> Kiem tra truc tiep tren host thay <code>C:\LDPlayer\LDPlayer14\dnconsole.exe</code> va may ao index <code>0</code> (<code>LDPlayer</code>, 1280x720) dang o trang thai <code>stop</code>. Khoi chay LDPlayer yeu cau UAC Elevation cua Windows, va <code>adb.exe devices -l</code> tra ve <strong>0 thiet bi ket noi</strong>. Tuan thu quy tac Stage-Gate: <em>khong tu bypass UAC, khong dung FakeDeviceDriver de gia lap Live ADB</em>. Do do muc I.1–I.10 va <code>LIVE_WORKFLOW_EXECUTION_PASS</code> giu nhan <span class="b-blk">BLOCKED</span> cho toi khi nguoi van hanh cap quyen UAC mo LDPlayer hoac cam dien thoai ADB that.
  </div>

  <div class="grid-2" style="flex:0 0 auto;">
    <div>
      <div class="sec-h">1. Phan biet Standalone Launch vs Live Workflow Execution (Muc III)</div>
      <table>
        <thead><tr><th>Ung dung doc lap</th><th>STANDALONE_LAUNCH_PASS</th><th>LIVE_WORKFLOW_EXECUTION_PASS</th><th>Kiem chung UI &amp; Schema</th></tr></thead>
        <tbody>
          <tr>
            <td><code>Auto_OmegaCompact.exe</code><br><span class='mono'>compact1_col · 640x460</span></td>
            <td><span class='b-pass'>PASS</span> (Studio CLOSED)</td>
            <td><span class='b-blk'>BLOCKED</span> (0 ADB devices)</td>
            <td><code>density-compact=true</code>, <code>single-col=true</code>, tab Utilities bi an khop Builder Preview, ten Task dai wrap khong cat chu.</td>
          </tr>
          <tr>
            <td><code>Auto_TitanExpanded.exe</code><br><span class='mono'>expanded · 1020x660</span></td>
            <td><span class='b-pass'>PASS</span> (Studio CLOSED)</td>
            <td><span class='b-blk'>BLOCKED</span> (0 ADB devices)</td>
            <td>Theme <code>#8B5CF6</code>, cot <code>Rank Titan</code>, tab Utilities &amp; Modules day du, khong bi compact chen ep.</td>
          </tr>
        </tbody>
      </table>
    </div>
    <div>
      <div class="sec-h">2. Tom tat Nang cap P4.7 Theo Yeu cau Audit</div>
      <table>
        <thead><tr><th>Hang muc</th><th>Thay doi thuc te da kiem chung</th></tr></thead>
        <tbody>
          <tr><td><strong>WYSIWYG Builder</strong></td><td>Them Component Tree + Palette (Add/Remove/Hide/Reorder), click component trong Preview de sua thuoc tinh rieng, sap xep Task priority, xem truoc du 5 trang thai (co <code>ERROR</code>), luu SQLite &amp; reload 100% khop.</td></tr>
          <tr><td><strong>Mini Runtime Compact</strong></td><td><code>apps/mini-runtime/ui/index.html</code> ho tro <code>compact1_col</code> flex layout, collapsible sections, <code>hiddenComponents</code> 1:1 voi Preview va khong cat chu ten Task dai.</td></tr>
          <tr><td><strong>Studio 1280x720</strong></td><td>Thu gon NodePalette/Inspector o man hinh &lt;1536px, an MiniMap tranh che Canvas, tang tuong phan chu node.</td></tr>
        </tbody>
      </table>
    </div>
  </div>
  <div class="ftr"><span>FarmerPro P4.7 Integration &amp; Verification Report</span><span>Trang 1 / 5</span></div>
</div>

<!-- PAGE 2 -->
<div class="page">
  <div class="hdr">
    <div>
      <div class="hdr-title">MA TRAN HOI QUY &amp; DOI CHIEU CHUC NANG LEGACY -&gt; STUDIO P4.7 (MUC V)</div>
      <div class="hdr-sub">Phan loai trung thuc tung phan he: TESTED / IMPLEMENTED / PARTIAL / BLOCKED / REPLACED</div>
    </div>
    <div class="hdr-meta"><div>9 / 9 Man hinh Sidebar Render OK · 0 JS Runtime Exceptions</div></div>
  </div>
  <table>
    <thead><tr><th style="width:20%;">Phan he FarmerPro</th><th style="width:22%;">Vi tri Ma nguon P4.7</th><th style="width:15%;">Phan loai</th><th>Bang chung &amp; Doi chieu Chuc nang</th></tr></thead>
    <tbody>{parity_html}</tbody>
  </table>
  <div class="sec-h">Yeu cau Can Thao tac Nguoi Van hanh De Dong Muc I (Live Hardware ADB)</div>
  <table>
    <thead><tr><th>Buoc thuc hien tren may Windows</th><th>Ket qua mong doi sau khi cap quyen</th></tr></thead>
    <tbody>
      <tr><td>1. Mo LDPlayer14 (chap nhan hop thoai UAC Windows) hoac cam dien thoai Android da bat USB Debugging.</td><td><code>.\\bin\\adb.exe devices -l</code> hien it nhat 1 thiet bi trang thai <code>device</code>.</td></tr>
      <tr><td>2. Mo <code>FarmerPro.exe</code> -&gt; Device Lab -&gt; Probe, Capture, Crop ROI &amp; Workflow Studio / Mini Tool chay 1 workflow that.</td><td>Chup anh truoc/sau co serial + timestamp, hoan tat <code>LIVE_WORKFLOW_EXECUTION_PASS</code>.</td></tr>
    </tbody>
  </table>
  <div class="ftr"><span>FarmerPro P4.7 Integration &amp; Verification Report</span><span>Trang 2 / 5</span></div>
</div>

<!-- PAGE 3 -->
<div class="page">
  <div class="hdr">
    <div>
      <div class="hdr-title">BANG CHUNG WYSIWYG MINI TOOL BUILDER &amp; 5 PREVIEW STATES (MUC II)</div>
      <div class="hdr-sub">Thao tac truc quan: Preset compact1_col (640x460), Remove &amp; Re-Add, Hide Utilities, Add Column, Chon Component &amp; Xem truoc ERROR</div>
    </div>
    <div class="hdr-meta"><div>Reload SQLite Verified · Error Preview: <code>{wys["hasErrorPreviewState"]}</code></div></div>
  </div>
  <div class="shot-box" style="flex:1;">
    <img src="{img_builder}" alt="WYSIWYG Builder P4.7 Full" />
    <div class="shot-cap">
      <strong>Hinh 1 (1600x900 Live):</strong> <em>WYSIWYG Mini Tool Builder</em> tren project <code>omega-compact</code>. Ben trai: Preset <code>compact1_col</code> (640x460, Compact), Component Tree voi nut Len/Xuong/An-Hien/Xoa (da an <code>Utilities Switches</code> -> gach ngang), va Inspector cua component dang chon <code>Device ADB Table</code> (cot <code>Runtime Col 1</code> gan bien <code>var_1</code>). Ben phai: <strong>WYSIWYG Live Preview (1:1 Runtime)</strong> o trang thai <strong>ERROR</strong> (<code>ERROR: ADB_MISSING</code>, chip <code>ERR</code> va log Fail-Closed).
    </div>
  </div>
  <div class="ftr"><span>FarmerPro P4.7 Integration &amp; Verification Report</span><span>Trang 3 / 5</span></div>
</div>

<!-- PAGE 4 -->
<div class="page">
  <div class="hdr">
    <div>
      <div class="hdr-title">DOI CHIEU 1:1 BUILDER VOI HAI MINI TOOL CHAY DOC LAP COMPACT vs EXPANDED (MUC III &amp; IV)</div>
      <div class="hdr-sub">FarmerPro Studio da dong hoan toan · Khong cat ten Task dai · An/hien Component khop 100% voi UI Schema</div>
    </div>
    <div class="hdr-meta"><div>Omega: 640x460 Compact · Titan: 1020x660 Expanded</div></div>
  </div>
  <div class="grid-2">
    <div class="shot-box">
      <img src="{img_omega}" alt="Omega Compact Standalone 640x460" />
      <div class="shot-cap">
        <strong>Hinh 2 (Standalone 640x460 Live — Studio Da Dong):</strong> <code>Auto_OmegaCompact.exe</code> ap dung dung <code>compact1_col</code>, mat do compact, an tab <code>TIỆN ÍCH</code> (do Builder da an <code>utilities_panel</code>), hien cot <code>RUNTIME COL 1</code> va hien day du ten Task dai <em>&quot;Nhiệm vụ Chính Tuyến Độ Dài Rất Lớn...&quot;</em> khong bi cat dau <code>...</code>. <code>STANDALONE_LAUNCH_PASS</code> · Live ADB: <code>BLOCKED</code>.
      </div>
    </div>
    <div class="shot-box">
      <img src="{img_titan}" alt="Titan Expanded Standalone 1020x660" />
      <div class="shot-cap">
        <strong>Hinh 3 (Standalone 1020x660 Live — Studio Da Dong):</strong> <code>Auto_TitanExpanded.exe</code> ap dung preset <code>expanded</code>, mau chu dao tim <code>#8B5CF6</code>, cot <code>RANK TITAN</code>, day du 3 tab <code>ĐIỀU KHIỂN / TIỆN ÍCH / MODULES</code> va danh sach nhat ky xac minh chu ky Ed25519 hop le. <code>STANDALONE_LAUNCH_PASS</code> · Live ADB: <code>BLOCKED</code>.
      </div>
    </div>
  </div>
  <div class="ftr"><span>FarmerPro P4.7 Integration &amp; Verification Report</span><span>Trang 4 / 5</span></div>
</div>

<!-- PAGE 5 -->
<div class="page">
  <div class="hdr">
    <div>
      <div class="hdr-title">KIEM TRA DO DOC WORKFLOW STUDIO TAI 1280x720 VA 1600x900 (MUC IV)</div>
      <div class="hdr-sub">Toi uu chieu rong NodePalette/Inspector, an MiniMap duoi 1536px de Canvas khong bi che va de doc node</div>
    </div>
    <div class="hdr-meta"><div>Breakpoints Verified: 1280x720 &amp; 1600x900</div></div>
  </div>
  <div class="grid-2">
    <div class="shot-box">
      <img src="{img_studio_1280}" alt="Workflow Studio 1280x720" />
      <div class="shot-cap">
        <strong>Hinh 4 (Workflow Studio 1280x720 Live):</strong> Da bo MiniMap che khuat vung giua, thu gon Inspector con <code>280px</code> va NodePalette <code>210px</code>; ten node va ma loai (<code>flow.start</code>, <code>state.checkpoint</code>, <code>flow.end</code>) ro net khong bi de len nhau.
      </div>
    </div>
    <div class="shot-box">
      <img src="{img_studio_1600}" alt="Workflow Studio 1600x900" />
      <div class="shot-cap">
        <strong>Hinh 5 (Workflow Studio 1600x900 Live):</strong> Ca 3 node va duong noi hien thi tron ven, Execution Dock &amp; Breakpoint hoat dong on dinh. Chot P4.7, san sang buoc vao kiem thu ADB that khi co thiet bi / quyen UAC LDPlayer truoc P5.
      </div>
    </div>
  </div>
  <div class="ftr"><span>FarmerPro P4.7 Integration &amp; Verification Report</span><span>Trang 5 / 5</span></div>
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
    [1, "1. Tong hop P4.7 & Pham vi Live ADB BLOCKED", 1],
    [1, "2. Ma tran Hoi quy & Doi chieu Chuc nang Legacy", 2],
    [1, "3. Bang chung WYSIWYG Builder (5 States & Reload)", 3],
    [1, "4. Doi chieu 1:1 Compact vs Expanded Mini Tools", 4],
    [1, "5. Do doc Workflow Studio tai 1280x720 & 1600x900", 5],
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
    pix.save(os.path.join(RENDER_DIR, f"p4_7_page_{pno:02d}.png"))
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
        len(qa_doc) == 5
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
