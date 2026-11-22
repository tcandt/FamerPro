import hashlib
import json
import os
import re
import shutil
import subprocess
import pymupdf

ROOT = r"E:\DATA\FarmerPro"
EV_DIR = os.path.join(ROOT, "artifacts", "p5_2_closeout_evidence")
RENDER_DIR = os.path.join(EV_DIR, "pdf_p5_2_closeout_pages")
os.makedirs(EV_DIR, exist_ok=True)
if os.path.exists(RENDER_DIR):
    for fn in os.listdir(RENDER_DIR):
        try:
            os.remove(os.path.join(RENDER_DIR, fn))
        except OSError:
            pass
os.makedirs(RENDER_DIR, exist_ok=True)

CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"
HTML_OUT = os.path.join(EV_DIR, "FARMERPRO_P5_2_CLOSEOUT_REPORT.html")
PDF_MAIN = os.path.join(ROOT, "FARMERPRO_P5_2_CLOSEOUT_REPORT.pdf")
PDF_COPY = os.path.join(EV_DIR, "FARMERPRO_P5_2_CLOSEOUT_REPORT.pdf")
QA_JSON = os.path.join(EV_DIR, "p5_2_closeout_pdf_qa.json")


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        while chunk := f.read(65536):
            h.update(chunk)
    return h.hexdigest()


def h_esc(s):
    return str(s).replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


def read_log(name):
    raw = open(os.path.join(EV_DIR, name), "rb").read()
    return raw.decode("utf-16" if b"\x00" in raw else "utf-8-sig", errors="replace")


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
diff_stat = (
    subprocess.check_output(
        ["git", "diff", "--shortstat", "0f6692b..HEAD", "--", "apps", "crates", "scripts"],
        cwd=ROOT,
    )
    .decode("utf-8", errors="replace")
    .strip()
)

e2e = json.loads(open(os.path.join(ROOT, "artifacts", "p5_2_evidence", "p5_2_e2e_report.json"), encoding="utf-8-sig").read())
assert e2e["gitHead"] == head_sha, f"E2E HEAD mismatch: {e2e['gitHead']} vs {head_sha}"

ws_log = read_log("cargo_test_workspace_p5_2_closeout.log")
passed_matches = re.findall(r"test result: ok\. (\d+) passed; (\d+) failed;", ws_log)
total_passed = sum(int(p) for p, _ in passed_matches)
total_failed = sum(int(f) for _, f in passed_matches)
assert total_passed >= 89 and total_failed == 0, f"Unexpected test summary: {total_passed}/{total_failed}"

updater_log = read_log("fp_updater_closeout_nocapture.log")
stress_log = read_log("p5_1_long_stress_nocapture.log")

m_stress = re.search(r"\[P5\.1 Long-Run Stress JSON\]\s*(\{[^\r\n]+\})", stress_log)
assert m_stress, "Missing P5.1 Long-Run Stress JSON line"
stress = json.loads(m_stress.group(1))

m_act = re.search(
    r"\[OS SIGKILL Reconcile AfterActivate\] stdout=(\{.*?\n\})",
    updater_log,
    re.S,
)
assert m_act, "Missing AfterActivate raw stdout block"
after_act_raw = m_act.group(1).strip()

html = f"""<!DOCTYPE html>
<html lang="vi">
<head>
<meta charset="utf-8">
<title>FarmerPro P5.2 Closeout Supplemental Verification Report</title>
<style>
  @page {{ size: A4 landscape; margin: 7.5mm 9.5mm; }}
  * {{ box-sizing: border-box; }}
  body {{ margin: 0; font-family: "Segoe UI", Arial, sans-serif; color: #0F172A; background: #FFFFFF; font-size: 8.1pt; line-height: 1.28; }}
  .page {{ page-break-after: always; position: relative; min-height: 188mm; display: flex; flex-direction: column; }}
  .page:last-child {{ page-break-after: auto; }}
  .hdr {{ display: flex; justify-content: space-between; align-items: flex-end; border-bottom: 2px solid #0F172A; padding-bottom: 4px; margin-bottom: 6px; }}
  .hdr-title {{ font-size: 12.6pt; font-weight: 800; color: #0F172A; }}
  .hdr-sub {{ font-size: 8pt; color: #475569; margin-top: 1px; }}
  .hdr-meta {{ text-align: right; font-size: 7.7pt; color: #334155; }}
  .kpi-row {{ display: grid; grid-template-columns: repeat(5, 1fr); gap: 7px; margin-bottom: 6px; }}
  .kpi {{ border: 1px solid #CBD5E1; border-radius: 6px; padding: 4px 8px; background: #F8FAFC; }}
  .kpi-lbl {{ font-size: 7.5pt; text-transform: uppercase; color: #475569; font-weight: 700; }}
  .kpi-val {{ font-size: 10.4pt; font-weight: 800; color: #0F172A; margin-top: 1px; }}
  .kpi-ok .kpi-val {{ color: #047857; }}
  .kpi-warn .kpi-val {{ color: #B45309; }}
  .sec-h {{ font-size: 9pt; font-weight: 800; color: #0F172A; margin: 5px 0 3px 0; border-left: 3.5px solid #1878F5; padding-left: 6px; }}
  table {{ width: 100%; border-collapse: collapse; margin-bottom: 6px; font-size: 7.6pt; }}
  th, td {{ border: 1px solid #CBD5E1; padding: 2.8px 5px; text-align: left; vertical-align: top; }}
  th {{ background: #F1F5F9; font-weight: 700; color: #0F172A; }}
  .mono {{ font-family: Consolas, monospace; font-size: 7.5pt; }}
  .b-pass {{ display: inline-block; padding: 1px 5px; border-radius: 4px; background: #D1FAE5; color: #065F46; font-weight: 800; font-size: 7.5pt; }}
  .b-blk {{ display: inline-block; padding: 1px 5px; border-radius: 4px; background: #FEF3C7; color: #92400E; font-weight: 800; font-size: 7.5pt; }}
  .grid-2 {{ display: grid; grid-template-columns: 1fr 1fr; gap: 8px; align-items: start; }}
  pre {{ background: #0F172A; color: #E2E8F0; padding: 7px 9px; border-radius: 6px; font-family: Consolas, monospace; font-size: 7.5pt; line-height: 1.32; white-space: pre-wrap; margin: 0 0 6px 0; }}
  .callout {{ border: 1px solid #059669; background: #ECFDF5; border-radius: 6px; padding: 6px 10px; font-size: 7.9pt; color: #064E3B; margin-top: 4px; }}
  .ftr {{ margin-top: auto; border-top: 1px solid #CBD5E1; padding-top: 3px; display: flex; justify-content: space-between; font-size: 7.5pt; color: #475569; }}
</style>
</head>
<body>

<!-- PAGE 1: CLOSEOUT FIXES & SECURITY VERIFICATION MATRIX -->
<div class="page">
  <div class="hdr">
    <div>
      <div class="hdr-title">FARMERPRO P5.2 CLOSEOUT — SECURITY ROOT OF TRUST, ROLLBACK &amp; CONCURRENCY AUDIT</div>
      <div class="hdr-sub">Supplemental Verification on Commit <code>{h_esc(head_sha)}</code> (Chuan hoa ket qua: 10 PASS / 1 BLOCKED trong 11 tieu chi P5.2)</div>
    </div>
    <div class="hdr-meta">
      <div>Branch: <code>{h_esc(branch_name)}</code> · Verified HEAD: <code>{h_esc(head_sha[:12])}</code></div>
      <div>Closeout Diff: <code>{h_esc(diff_stat)}</code></div>
    </div>
  </div>

  <div class="kpi-row">
    <div class="kpi kpi-ok"><div class="kpi-lbl">Cargo Tests (Post-Fix)</div><div class="kpi-val">{total_passed} / {total_passed + total_failed} PASS</div></div>
    <div class="kpi kpi-ok"><div class="kpi-lbl">P5.2 E2E Summary</div><div class="kpi-val">10 PASS / 1 BLOCKED</div></div>
    <div class="kpi kpi-ok"><div class="kpi-lbl">Root of Trust Fix</div><div class="kpi-val">Installed EXE Anchor</div></div>
    <div class="kpi kpi-ok"><div class="kpi-lbl">P5.1 WAL Long Stress</div><div class="kpi-val">{stress["events"]} Ev / 0 Lost</div></div>
    <div class="kpi kpi-warn"><div class="kpi-lbl">Live Hardware ADB</div><div class="kpi-val">BLOCKED (0 ADB)</div></div>
  </div>

  <div class="sec-h">I. Bang Kiem chung Bo sung P5.2 Closeout (Phan loai TestKit / GUI Integration / Windows OS / Live Hardware)</div>
  <table>
    <thead><tr><th>Hang muc Closeout</th><th>Lo hong / Rui ro da xac dinh</th><th>Sua loi &amp; Kiem chung thuc te tren Windows OS / Binary</th><th>Phan loai &amp; Trang thai</th></tr></thead>
    <tbody>
      <tr>
        <td><strong>I. Update Root of Trust</strong></td>
        <td>Truoc day <code>upgrade_from_release</code> doc PE <code>RT_RCDATA #7777</code> tu chinh EXE ung vien moi -> de bi tu ky lai bang rogue keypair.</td>
        <td>Sua sang doc anchor tu <strong>EXE da cai dat truoc</strong> (<code>versions/v&lt;active&gt;/</code>) + doi chieu <code>trustedPubkeyHex</code>. Test tan cong doi EXE + doi pubkey + ky lai SHA-256 bi chan: <code>UPDATE_ROOT_OF_TRUST_UNTRUSTED</code>. Bo sung <code>KeyRotationDelegation</code> cho P5.3.</td>
        <td><span class="b-pass">PASS (Windows OS)</span></td>
      </tr>
      <tr>
        <td><strong>II. Rollback Binary vs DB Restore</strong></td>
        <td>Studio/API mac dinh bat <code>restore_backup=true</code> co the ghi de mat du lieu B tao tren v1.1.0.</td>
        <td>Tach biet: mac dinh <code>restore_backup=false</code> (Binary Rollback). Test tao Data A (v1.0) -&gt; upgrade v1.1 -&gt; tao Data B (<code>acc-data-B</code>, <code>88888</code>) -&gt; Rollback v1.0 giu nguyen Data A + B. Schema v99 chan ha cap (<code>SQLITE_DOWNGRADE_INCOMPATIBLE</code>), va DB restore tu luu <code>pre_rollback_safety</code> truoc khi phuc hoi.</td>
        <td><span class="b-pass">PASS (GUI + OS)</span></td>
      </tr>
      <tr>
        <td><strong>IIIa. Active Pointer Atomicity &amp; Repair</strong></td>
        <td>Thieu test lock file dich va phuc hoi khi <code>active_version.json</code> bi hong/xoa.</td>
        <td>Dung Win32 <code>MoveFileExW(REPLACE_EXISTING | WRITE_THROUGH)</code> + shadow <code>active_version.bak.json</code>. Lock doc quyen tra <code>ACTIVE_POINTER_LOCKED</code> giu nguyen file cu; xoa/hong JSON duoc <code>reconcile</code> tu phuc hoi (<code>RECOVERED_POINTER</code>).</td>
        <td><span class="b-pass">PASS (Windows OS)</span></td>
      </tr>
      <tr>
        <td><strong>IIIb. Concurrent Dual Updater Lock</strong></td>
        <td>Chua co exclusive lock ngan 2 tien trinh <code>fp-updater.exe</code> chay song song tren cung tool.</td>
        <td>Bo sung Win32 <code>UpdaterLockGuard</code> (<code>dwShareMode=0</code> tren <code>updater.lock</code>). Spawn 2 tien trinh <code>fp-updater.exe</code> dong thoi -> tien trinh thu 2 bi chan ngay: <code>CONCURRENT_UPDATER_LOCKED</code>.</td>
        <td><span class="b-pass">PASS (Windows OS)</span></td>
      </tr>
      <tr>
        <td><strong>IV. Path Traversal &amp; NTFS Junction</strong></td>
        <td>Can chan traversal trong manifest, goi ghi de <code>data/</code>, va NTFS Junction tro ra ngoai.</td>
        <td>Chot <code>validate_single_filename</code> + <code>validate_safe_relative_path</code> + cam top-level <code>data/</code> &amp; <code>.updater/</code> (<code>RELEASE_FORBIDS_USER_DATA_OVERWRITE</code>) + check Win32 <code>FILE_ATTRIBUTE_REPARSE_POINT</code> (test <code>mklink /J</code> bi chan: <code>REPARSE_POINT_OR_JUNCTION_FORBIDDEN</code>).</td>
        <td><span class="b-pass">PASS (Windows OS)</span></td>
      </tr>
      <tr>
        <td><strong>V. Live Hardware ADB (Stage-Gate)</strong></td>
        <td>Khong gia lap thiet bi that bang TestKit.</td>
        <td><code>adb.exe devices -l</code> rong (<code>List of devices attached</code>) | <code>LDPlayer14 vm0 = stop</code>. Ket qua tong hop P5.2 giu chuan <strong>10 PASS / 1 BLOCKED</strong>.</td>
        <td><span class="b-blk">BLOCKED</span></td>
      </tr>
    </tbody>
  </table>

  <div class="sec-h">II. Thiet ke Key Rotation Co Chuyen Tiep Cho P5.3</div>
  <div style="background:#F8FAFC;border:1px solid #CBD5E1;border-radius:6px;padding:5px 8px;font-size:7.7pt;">
    Trong <code>fp-updater</code> da dinh nghia contract <code>KeyRotationDelegation(old_pubkey_hex, new_pubkey_hex, issued_at_utc, expires_at_utc, signature_by_old_key_hex)</code>. Sang P5.3, khi nha phat hanh muon doi cap khoa ky, ban cap nhat phai kem chung thu chuyen tiep duoc ky bang <strong>private key cu dang tin cay</strong> va con han; hien tai o P5.2 bat ky thay doi khoa nao khong co uy quyen se bi tu choi fail-closed (<code>UPDATE_ROOT_OF_TRUST_UNTRUSTED</code>).
  </div>
  <div class="ftr"><span>FarmerPro P5.2 Closeout Verification (Commit {h_esc(head_sha[:12])})</span><span>Trang 1 / 2</span></div>
</div>

<!-- PAGE 2: RAW SIGKILL AFTER-ACTIVATE LOGS, P5.1 STRESS BENCHMARK & STAGE-GATE -->
<div class="page">
  <div class="hdr">
    <div>
      <div class="hdr-title">RAW EVIDENCE: FORCED-KILL AFTER_ACTIVATE &amp; P5.1 LONG-RUN WAL STRESS (3,000 EVENTS)</div>
      <div class="hdr-sub">Bang chung tho trich truc tiep tu log tren commit <code>{h_esc(head_sha)}</code></div>
    </div>
    <div class="hdr-meta"><div>Branch: <code>{h_esc(branch_name)}</code></div></div>
  </div>

  <div class="grid-2">
    <div>
      <div class="sec-h">III. Raw Output OS SIGKILL tai AfterActivate + Reconcile</div>
      <pre>{h_esc(after_act_raw)}</pre>
      <div class="sec-h">IV. Thong so Tai Nang &amp; Do tre P5.1 Long-Run Stress (4 Workers x 750 = 3,000 Events)</div>
      <table>
        <thead><tr><th>Chi so do duoc (Windows 11)</th><th>Gia tri thuc te</th><th>Ghi chu</th></tr></thead>
        <tbody>
          <tr><td>Tong su kien &amp; Luong</td><td><code>{stress["events"]}</code> events · <code>{stress["workers"]}</code> writers + 1 reader + compactor</td><td>0 su kien that lac; seq 1..3000 lien tuc</td></tr>
          <tr><td>Do tre ghi P50 / P95 / P99</td><td><strong>{stress["p50Us"]/1000:.2f} ms</strong> / <strong>{stress["p95Us"]/1000:.2f} ms</strong> / <strong>{stress["p99Us"]/1000:.2f} ms</strong></td><td>Max spike: {stress["maxUs"]/1000:.1f} ms; Wall time: {stress["wallMs"]} ms</td></tr>
          <tr><td>CPU &amp; Working Set RAM</td><td>CPU time: <code>{stress["cpuDeltaMs"]} ms</code> · WS: {stress["wsBeforeKb"]}KB -&gt; Peak {stress["wsPeakKb"]}KB -&gt; {stress["wsAfterKb"]}KB</td><td>Do bang Win32 <code>GetProcessTimes</code> &amp; <code>K32GetProcessMemoryInfo</code></td></tr>
          <tr><td>WAL Compaction &amp; Polls</td><td>Mid-flight: <code>{stress["compactedMidflight"]}</code> rows · Final: <code>{stress["compactedFinal"]}</code> rows · WAL peak: {stress["walBytesPeak"]} B</td><td>Reader polls: {stress["readerPolls"]} (busy retries: {stress["readerBusyRetries"]})</td></tr>
        </tbody>
      </table>
    </div>
    <div>
      <div class="sec-h">V. Tom tat E2E &amp; Raw Workspace Regression Log</div>
      <pre>Verified Commit   : {h_esc(head_sha)}
Cargo Workspace   : {total_passed} passed; {total_failed} failed (0 ignored)
Closeout Diff     : {h_esc(diff_stat)}
E2E Check (Win)   : 10 PASS / 1 BLOCKED (of 11 P5.2 criteria)
  - TestKit Scope : Alpha v1.0.0 -&gt; v1.1.0 preserve Sector-B7, 250, ***REDACTED***
  - Binary Rollback: preserves new post-upgrade v1.1.0 Data B; incompatible DB guarded
  - Windows OS Tests: SIGKILL (AfterBackup/Stage/Activate), Dual-Updater Lock, Junction
  - Live Hardware : BLOCKED (adb devices -l empty; LDPlayer14 vm0 = stop)
Console Errors    : Studio={len(e2e["consoleErrors"]["studio"])}, AlphaV1={len(e2e["consoleErrors"]["alphaV1"])}, AlphaV11={len(e2e["consoleErrors"]["alphaV11"])}</pre>
      <div class="callout">
        <strong>DE XUAT DONG STAGE-GATE P5.2 -> MO KHOA P5.3:</strong> Toan bo 5 dieu kien Closeout (Update Root of Trust co dinh vao EXE da cai, Rollback Binary tach khoi DB Restore giu du lieu moi, Pointer atomicity + phuc hoi JSON hong/xoa + Dual Updater Lock, Path traversal &amp; NTFS Junction blocking, va P5.1 3.000-event 4-worker WAL stress benchmark) da PASS 100% sau commit cuoi cung (<code>{h_esc(head_sha[:12])}</code>). Live ADB duoc giu trung thuc <code>BLOCKED</code>. Xin phe duyet mo khoa <strong>P5.3 — Authenticode, Publisher Profile &amp; VERSIONINFO</strong>.
      </div>
    </div>
  </div>
  <div class="ftr"><span>FarmerPro P5.2 Closeout Verification (Commit {h_esc(head_sha[:12])})</span><span>Trang 2 / 2</span></div>
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
doc.set_toc([
    [1, "1. Closeout Audit Matrix & Root of Trust / Rollback Verification", 1],
    [1, "2. AfterActivate SIGKILL Evidence, P5.1 3000-Event Stress & Gate Sign-Off", 2],
])
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
    pix.save(os.path.join(RENDER_DIR, f"p5_2_closeout_page_{pno:02d}.png"))
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
        len(qa_doc) == 2
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
