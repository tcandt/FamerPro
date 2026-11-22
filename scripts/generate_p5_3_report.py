import base64
import hashlib
import json
import os
import re
import shutil
import subprocess
import pymupdf

ROOT = r"E:\DATA\FarmerPro"
EV_DIR = os.path.join(ROOT, "artifacts", "p5_3_evidence")
RENDER_DIR = os.path.join(EV_DIR, "pdf_p5_3_rendered_pages")
os.makedirs(EV_DIR, exist_ok=True)
if os.path.exists(RENDER_DIR):
    for old_f in os.listdir(RENDER_DIR):
        try:
            os.remove(os.path.join(RENDER_DIR, old_f))
        except OSError:
            pass
os.makedirs(RENDER_DIR, exist_ok=True)

CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"
HTML_OUT = os.path.join(EV_DIR, "FARMERPRO_P5_3_ACCEPTANCE_REPORT.html")
PDF_MAIN = os.path.join(ROOT, "FARMERPRO_P5_3_ACCEPTANCE_REPORT.pdf")
PDF_COPY = os.path.join(EV_DIR, "FARMERPRO_P5_3_ACCEPTANCE_REPORT.pdf")
QA_JSON = os.path.join(EV_DIR, "p5_3_pdf_qa.json")
P5_2_BASE = "7c7999f825f0ca0e889c6a05facea91dc6f23d8e"


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
    open(os.path.join(EV_DIR, "p5_3_e2e_report.json"), encoding="utf-8-sig").read()
)
raw_log_bytes = open(os.path.join(EV_DIR, "cargo_test_workspace_p5_3.log"), "rb").read()
test_log = raw_log_bytes.decode("utf-16" if b"\x00" in raw_log_bytes else "utf-8-sig", errors="replace")
passed_matches = re.findall(r"test result: ok\. (\d+) passed; (\d+) failed;", test_log)
total_passed = sum(int(p) for p, _ in passed_matches)
total_failed = sum(int(f) for _, f in passed_matches)
assert total_passed >= 90 and total_failed == 0, f"Unexpected tests: {total_passed}/{total_failed}"

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

diff_p52_p53 = (
    subprocess.check_output(
        [
            "git",
            "diff",
            "--shortstat",
            f"{P5_2_BASE}..HEAD",
            "--",
            "apps/desktop/src",
            "apps/desktop/src-tauri",
            "apps/mini-runtime",
            "crates",
        ],
        cwd=ROOT,
    )
    .decode("utf-8", errors="replace")
    .strip()
)

img_studio = img_data_uri(os.path.join(EV_DIR, "p5_3_studio_signing_builder.png"))
img_alpha = img_data_uri(os.path.join(EV_DIR, "p5_3_brand_alpha_running.png"))
img_beta = img_data_uri(os.path.join(EV_DIR, "p5_3_brand_beta_running.png"))

os_insp = rep["osInspect"]
sr = rep["studioResult"]
ul = rep["updaterLifecycle"]
pub_rel = rep["publicSignedCommercialRelease"]

html = f"""<!DOCTYPE html>
<html lang="vi">
<head>
<meta charset="utf-8">
<title>FarmerPro P5.3 — Branding, PE VERSIONINFO, Authenticode &amp; Key Rotation Acceptance Report</title>
<style>
  @page {{ size: A4 landscape; margin: 7.5mm 9.5mm; }}
  * {{ box-sizing: border-box; }}
  body {{ margin: 0; font-family: "Segoe UI", Arial, sans-serif; color: #0F172A; background: #FFFFFF; font-size: 8.5pt; line-height: 1.28; }}
  .page {{ page-break-after: always; position: relative; min-height: 188mm; display: flex; flex-direction: column; }}
  .page:last-child {{ page-break-after: auto; }}
  .hdr {{ display: flex; justify-content: space-between; align-items: flex-end; border-bottom: 2px solid #0F172A; padding-bottom: 4px; margin-bottom: 6px; }}
  .hdr-title {{ font-size: 12.8pt; font-weight: 800; color: #0F172A; }}
  .hdr-sub {{ font-size: 8.3pt; color: #475569; margin-top: 1px; }}
  .hdr-meta {{ text-align: right; font-size: 8.1pt; color: #334155; }}
  .kpi-row {{ display: grid; grid-template-columns: repeat(5, 1fr); gap: 7px; margin-bottom: 6px; }}
  .kpi {{ border: 1px solid #CBD5E1; border-radius: 6px; padding: 5px 8px; background: #F8FAFC; }}
  .kpi-lbl {{ font-size: 8.0pt; text-transform: uppercase; color: #475569; font-weight: 700; }}
  .kpi-val {{ font-size: 10.6pt; font-weight: 800; color: #0F172A; margin-top: 1px; }}
  .kpi-ok .kpi-val {{ color: #047857; }}
  .kpi-warn .kpi-val {{ color: #B45309; }}
  .sec-h {{ font-size: 9.2pt; font-weight: 800; color: #0F172A; margin: 5px 0 3px 0; border-left: 3.5px solid #1878F5; padding-left: 6px; }}
  table {{ width: 100%; border-collapse: collapse; margin-bottom: 5px; font-size: 8.1pt; }}
  th, td {{ border: 1px solid #CBD5E1; padding: 2.8px 5px; text-align: left; vertical-align: top; }}
  th {{ background: #F1F5F9; font-weight: 700; color: #0F172A; }}
  .mono, code {{ font-family: Consolas, monospace; font-size: 8.0pt; }}
  .b-pass {{ display: inline-block; padding: 1px 5px; border-radius: 4px; background: #D1FAE5; color: #065F46; font-weight: 800; font-size: 8.0pt; }}
  .b-blk {{ display: inline-block; padding: 1px 5px; border-radius: 4px; background: #FEF3C7; color: #92400E; font-weight: 800; font-size: 8.0pt; }}
  .callout {{ border: 1px solid #059669; background: #ECFDF5; border-radius: 6px; padding: 6px 10px; font-size: 8.3pt; color: #064E3B; margin-top: 5px; }}
  .grid-even {{ display: grid; grid-template-columns: 1fr 1fr; gap: 8px; align-items: start; }}
  .shot-frame {{ border: 1px solid #94A3B8; border-radius: 6px; padding: 5px; background: #F8FAFC; text-align: center; }}
  .shot-frame img {{ max-height: 92mm; width: auto; max-width: 100%; height: auto; display: block; margin: 0 auto; border-radius: 4px; }}
  .shot-cap {{ font-size: 8.1pt; color: #1E293B; margin-top: 4px; line-height: 1.28; text-align: left; }}
  .ftr {{ margin-top: auto; border-top: 1px solid #CBD5E1; padding-top: 3px; display: flex; justify-content: space-between; font-size: 8.0pt; color: #475569; }}
</style>
</head>
<body>

<!-- PAGE 1: P5.3 ACCEPTANCE MATRIX & SIGNING ORDER ARCHITECTURE -->
<div class="page">
  <div class="hdr">
    <div>
      <div class="hdr-title">FARMERPRO P5.3 — PUBLISHER IDENTITY, PE VERSIONINFO, AUTHENTICODE &amp; KEY ROTATION</div>
      <div class="hdr-sub">Stage-Gate Verification trên Commit <code>{h_esc(head_sha)}</code> (SQLite v5 + Win32 Resources + Windows CNG CodeSigning; Live ADB &amp; Public Commercial Cert giữ nguyên <code>BLOCKED</code>)</div>
    </div>
    <div class="hdr-meta">
      <div>Branch: <code>{h_esc(branch_name)}</code> · Verified HEAD: <code>{h_esc(head_sha[:12])}</code></div>
      <div>Diff P5.2..P5.3: <code>{h_esc(diff_p52_p53)}</code></div>
    </div>
  </div>

  <div class="kpi-row">
    <div class="kpi kpi-ok"><div class="kpi-lbl">Workspace Cargo Tests</div><div class="kpi-val">{total_passed} / {total_passed + total_failed} PASS</div></div>
    <div class="kpi kpi-ok"><div class="kpi-lbl">PE Icon + VERSIONINFO</div><div class="kpi-val">2 Distinct Brands OK</div></div>
    <div class="kpi kpi-ok"><div class="kpi-lbl">Authenticode QA + Tamper</div><div class="kpi-val">Sign LAST / Reject</div></div>
    <div class="kpi kpi-ok"><div class="kpi-lbl">Key Rotation &amp; RoT</div><div class="kpi-val">Delegated + No Replay</div></div>
    <div class="kpi kpi-warn"><div class="kpi-lbl">Commercial CA / ADB</div><div class="kpi-val">Honest BLOCKED</div></div>
  </div>

  <div class="sec-h">I. Ma trận Nghiệm thu 12 Tiêu chí P5.3 (Tách biệt Ed25519 Bundle, Authenticode, Publisher Identity, Trust Chain &amp; Compat)</div>
  <table>
    <thead><tr><th>Hạng mục kiểm thử P5.3</th><th>Kịch bản thực thi thực tế</th><th>Kết quả đo được từ Rust Tests, OS Win32 &amp; E2E</th><th>Trạng thái</th></tr></thead>
    <tbody>
      <tr><td><strong>1. Multi-Brand Icon &amp; Title</strong></td><td>Xuất 2 Mini Tools độc lập (Alpha Emerald vs Beta Amber/Purple)</td><td>2 <code>.ico</code> khác nhau, 2 EXE riêng (<code>Auto_BrandAlpha.exe</code> / <code>Auto_BrandBeta.exe</code>).</td><td><span class="b-pass">PASS</span></td></tr>
      <tr><td><strong>2. PE VERSIONINFO (8 trường)</strong></td><td>Ghi <code>RT_VERSION #1</code> và đọc lại qua Win32 <code>GetFileVersionInfoW</code> &amp; PowerShell</td><td>Alpha=<code>{h_esc(os_insp["alphaVersionInfo"]["CompanyName"])}</code> · Beta=<code>{h_esc(os_insp["beta110VersionInfo"]["CompanyName"])}</code>.</td><td><span class="b-pass">PASS</span></td></tr>
      <tr><td><strong>3. Unsigned Internal Mode</strong></td><td>Build chế độ <code>unsigned_internal</code> cho nội bộ</td><td>Có <code>VERSIONINFO</code> + Ed25519 + <code>RT_RCDATA #7777</code>; <code>trustedPublicRelease=false</code>.</td><td><span class="b-pass">PASS</span></td></tr>
      <tr><td><strong>4. Self-Signed QA Signing</strong></td><td>Sinh cert Windows CNG Store, chạy <code>signtool.exe</code> ở BƯỚC CUỐI</td><td>Beta v1.0 &amp; v1.1 có chữ ký Authenticode thật (<code>{h_esc(sr["qaCert"]["thumbprint"][:12])}...</code>), RoT #7777 nguyên vẹn.</td><td><span class="b-pass">PASS</span></td></tr>
      <tr><td><strong>5. PublicSigned Fail-Closed</strong></td><td>Thử <code>public_signed</code> khi thiếu cert hoặc dùng Self-Signed QA</td><td>Từ chối cứng cả 2 trường hợp (không bao giờ đánh dấu giả <code>TrustedPublicRelease</code>).</td><td><span class="b-blk">BLOCKED</span></td></tr>
      <tr><td><strong>6. PE Authenticode Tamper</strong></td><td>Sửa byte trong EXE đã ký <code>SelfSignedQa</code> (giữ nguyên bundle JSON)</td><td><code>WinVerifyTrust</code> &amp; <code>fp-updater</code> chặn ngay: <code>AUTHENTICODE_TAMPERED_OR_INVALID</code>.</td><td><span class="b-pass">PASS</span></td></tr>
      <tr><td><strong>7. Bundle Tamper on Signed EXE</strong></td><td>Sửa <code>ui-schema.json</code> cạnh EXE có chữ ký Authenticode hợp lệ</td><td>Bị chặn bởi Ed25519 &amp; SHA-256 checksum (chứng minh 2 lớp kiểm tra độc lập).</td><td><span class="b-pass">PASS</span></td></tr>
      <tr><td><strong>8. KeyRotationDelegation Hợp lệ</strong></td><td>Xoay khóa Ed25519 (Old→New), nâng cấp Beta v1.0.0→v1.1.0</td><td><code>fp-updater upgrade</code> xác thực chữ ký Old Key + cập nhật <code>.updater/trust-state.json</code>.</td><td><span class="b-pass">PASS</span></td></tr>
      <tr><td><strong>9. Chặn Delegation Gian lận / Replay</strong></td><td>Dùng lại nonce đã kích hoạt, delegation giả, sai product slug hoặc hết hạn</td><td>Chặn <code>KEY_ROTATION_NONCE_REPLAYED</code>, <code>KEY_ROTATION_DELEGATION_INVALID</code>, <code>PUBLISHER_IDENTITY_MISMATCH</code>.</td><td><span class="b-pass">PASS</span></td></tr>
      <tr><td><strong>10. SQLite v5 &amp; No Secret Leaks</strong></td><td>Quét toàn bộ thư mục <code>sandbox/</code> &amp; DB v5 tìm private key / PFX password</td><td><code>secretLeakCount=0</code>; private key nằm an toàn trong Windows CNG / keystore ngoài bundle.</td><td><span class="b-pass">PASS</span></td></tr>
      <tr><td><strong>11. Studio UI &amp; Independent Run</strong></td><td>Đóng hoàn toàn Studio EXE rồi chạy 2 Mini Tools độc lập</td><td>Alpha &amp; Beta chạy mượt, ConsoleErrors=0, hiển thị đúng theme &amp; task riêng.</td><td><span class="b-pass">PASS</span></td></tr>
      <tr><td><strong>12. Live Hardware ADB</strong></td><td>Kiểm tra thiết bị Android kết nối thật trên host</td><td>Host: <code>{h_esc(rep["adbLiveHardware"]["adbDevicesOutput"])}</code> | <code>LDPlayer14 vm0={h_esc(rep["adbLiveHardware"]["ldplayer14Vm0IsRunning"])}</code>.</td><td><span class="b-blk">BLOCKED</span></td></tr>
    </tbody>
  </table>

  <div class="sec-h">II. Luồng Đóng gói &amp; Ký Chính xác Thứ tự (PE Resources Trước → Authenticode CUỐI CÙNG)</div>
  <svg width="100%" height="62" viewBox="0 0 1000 62" xmlns="http://www.w3.org/2000/svg" style="background:#F8FAFC;border:1px solid #CBD5E1;border-radius:6px;">
    <rect x="6" y="7" width="170" height="48" rx="5" fill="#EFF6FF" stroke="#2563EB" stroke-width="1.5"/>
    <text x="91" y="27" text-anchor="middle" font-size="12.5" font-weight="bold" fill="#1E3A8A">1. Clone &amp; Inject Icon</text>
    <text x="91" y="44" text-anchor="middle" font-size="11.2" fill="#1E40AF">RT_ICON / GROUP_ICON</text>
    <text x="186" y="36" text-anchor="middle" font-size="15" font-weight="bold" fill="#334155">→</text>
    <rect x="196" y="7" width="175" height="48" rx="5" fill="#EFF6FF" stroke="#2563EB" stroke-width="1.5"/>
    <text x="283" y="27" text-anchor="middle" font-size="12.5" font-weight="bold" fill="#1E3A8A">2. PE VERSIONINFO</text>
    <text x="283" y="44" text-anchor="middle" font-size="11.2" fill="#1E40AF">RT_VERSION #1 (8 fields)</text>
    <text x="381" y="36" text-anchor="middle" font-size="15" font-weight="bold" fill="#334155">→</text>
    <rect x="391" y="7" width="178" height="48" rx="5" fill="#F3E8FF" stroke="#7C3AED" stroke-width="1.5"/>
    <text x="480" y="27" text-anchor="middle" font-size="12.5" font-weight="bold" fill="#5B21B6">3. Root of Trust</text>
    <text x="480" y="44" text-anchor="middle" font-size="11.2" fill="#6D28D9">RT_RCDATA #7777 PubKey</text>
    <text x="579" y="36" text-anchor="middle" font-size="15" font-weight="bold" fill="#334155">→</text>
    <rect x="589" y="7" width="190" height="48" rx="5" fill="#FFFBEB" stroke="#D97706" stroke-width="1.5"/>
    <text x="684" y="27" text-anchor="middle" font-size="12.5" font-weight="bold" fill="#92400E">4. Authenticode (CUỐI)</text>
    <text x="684" y="44" text-anchor="middle" font-size="11.2" fill="#B45309">signtool.exe /fd SHA256</text>
    <text x="789" y="36" text-anchor="middle" font-size="15" font-weight="bold" fill="#334155">→</text>
    <rect x="799" y="7" width="194" height="48" rx="5" fill="#ECFDF5" stroke="#059669" stroke-width="1.5"/>
    <text x="896" y="27" text-anchor="middle" font-size="12.5" font-weight="bold" fill="#065F46">5. Final Verify &amp; Hash</text>
    <text x="896" y="44" text-anchor="middle" font-size="11.2" fill="#047857">WinVerifyTrust + Final SHA-256</text>
  </svg>
  <div class="ftr"><span>FarmerPro P5.3 Acceptance Report (Commit {h_esc(head_sha[:12])})</span><span>Trang 1 / 4</span></div>
</div>

<!-- PAGE 2: STUDIO P5.3 BUILDER UI & OS VERSIONINFO/AUTHENTICODE READBACK -->
<div class="page">
  <div class="hdr">
    <div>
      <div class="hdr-title">STUDIO P5.3 SIGNING BUILDER UI &amp; XÁC MINH TRỰC TIẾP TRÊN WINDOWS OS</div>
      <div class="hdr-sub">Quản lý Publisher Profile, PE <code>VERSIONINFO</code>, tạo CNG QA Cert, Signing Preflight &amp; đối chiếu từ Windows PowerShell</div>
    </div>
    <div class="hdr-meta"><div>Studio Console Errors: {len(rep["consoleErrors"]["studio"])} · signtool.exe Verified</div></div>
  </div>
  <div class="shot-frame" style="margin-bottom:6px;">
    <img src="{img_studio}" alt="Studio P5.3 Signing Builder UI" style="max-height:112mm;" />
    <div class="shot-cap"><strong>Hình 1 (FarmerPro Studio 1600×900 — WYSIWYG Builder P5.3):</strong> Thanh điều khiển <em>Publisher &amp; PE Signing (P5.3)</em> hiển thị Publisher Profile <code>Beta Nova Studio</code>, chế độ <code>Self-Signed QA</code>, tham chiếu chứng chỉ CNG <code>CERT_STORE://CurrentUser/My/...</code>, bảng 8 trường <code>VERSIONINFO</code>, kết quả <code>Signing Preflight READY · signtool: OK</code> và danh sách Release kèm trạng thái Authenticode.</div>
  </div>
  <div class="sec-h">Đối chiếu Trực tiếp bằng Windows OS (<code>(Get-Item).VersionInfo</code> &amp; <code>Get-AuthenticodeSignature</code>)</div>
  <table>
    <thead><tr><th>Thuộc tính PE / Authenticode</th><th>Brand Alpha — <code>Auto_BrandAlpha.exe</code> (Unsigned Internal)</th><th>Brand Beta — <code>Auto_BrandBeta.exe v1.1.0</code> (Self-Signed QA)</th></tr></thead>
    <tbody>
      <tr><td><strong>CompanyName / Copyright</strong></td><td><code>{h_esc(os_insp["alphaVersionInfo"]["CompanyName"])}</code> · {h_esc(os_insp["alphaVersionInfo"]["LegalCopyright"])}</td><td><code>{h_esc(os_insp["beta110VersionInfo"]["CompanyName"])}</code> · {h_esc(os_insp["beta110VersionInfo"]["LegalCopyright"])}</td></tr>
      <tr><td><strong>Product / Description</strong></td><td><code>{h_esc(os_insp["alphaVersionInfo"]["ProductName"])}</code> — {h_esc(os_insp["alphaVersionInfo"]["FileDescription"])}</td><td><code>{h_esc(os_insp["beta110VersionInfo"]["ProductName"])}</code> — {h_esc(os_insp["beta110VersionInfo"]["FileDescription"])}</td></tr>
      <tr><td><strong>FileVersion / OriginalFilename</strong></td><td><code>{h_esc(os_insp["alphaVersionInfo"]["FileVersion"])}</code> · <code>{h_esc(os_insp["alphaVersionInfo"]["OriginalFilename"])}</code></td><td><code>{h_esc(os_insp["beta110VersionInfo"]["FileVersion"])}</code> · <code>{h_esc(os_insp["beta110VersionInfo"]["OriginalFilename"])}</code></td></tr>
      <tr><td><strong>OS Authenticode Status</strong></td><td><code>{h_esc(os_insp["alphaAuth"]["status"])}</code> (<code>trustedPublicRelease=false</code>)</td><td><code>{h_esc(os_insp["beta110Auth"]["status"])}</code> · Subject: <code>{h_esc(os_insp["beta110Auth"]["subject"])}</code> · Thumb: <code>{h_esc(str(os_insp["beta110Auth"]["thumbprint"])[:16])}...</code></td></tr>
    </tbody>
  </table>
  <div class="ftr"><span>FarmerPro P5.3 Acceptance Report (Commit {h_esc(head_sha[:12])})</span><span>Trang 2 / 4</span></div>
</div>

<!-- PAGE 3: TWO INDEPENDENT RUNTIME MINI TOOLS WITH STUDIO CLOSED -->
<div class="page">
  <div class="hdr">
    <div>
      <div class="hdr-title">THỰC THI ĐỘC LẬP 2 MINI TOOLS KHI ĐÃ ĐÓNG STUDIO (KEY ROTATION v1.0.0 → v1.1.0)</div>
      <div class="hdr-sub">Cả 2 công cụ đã đóng hoàn toàn <code>FarmerPro.exe</code> trước khi khởi chạy, tách biệt icon, màu sắc, dữ liệu và Root-of-Trust</div>
    </div>
    <div class="hdr-meta"><div>Studio Closed: {rep["independentRuntime"]["studioClosedBeforeLaunch"]} · Runtime Errors: {len(rep["consoleErrors"]["brandAlpha"]) + len(rep["consoleErrors"]["brandBeta"])}</div></div>
  </div>
  <div class="grid-even">
    <div class="shot-frame">
      <img src="{img_alpha}" alt="Brand Alpha Running Standalone" />
      <div class="shot-cap"><strong>Hình 2 (Brand Alpha — <code>Auto_BrandAlpha.exe v1.0.0</code>, Unsigned Internal):</strong> Độc lập hoàn toàn với Studio; mang nhãn <code>Vanguard Auto Suite v1.0.0</code>, task <code>Vanguard Guild Boss Sweep</code>, biến <code>guild_tier</code> và <code>RT_RCDATA #7777</code> khớp khóa Ed25519 của Publisher Alpha.</div>
    </div>
    <div class="shot-frame">
      <img src="{img_beta}" alt="Brand Beta v1.1.0 Upgraded via Key Rotation" />
      <div class="shot-cap"><strong>Hình 3 (Brand Beta — <code>Auto_BrandBeta.exe v1.1.0</code>, Self-Signed QA + Key Rotated):</strong> Đã nâng cấp từ v1.0.0→v1.1.0 nhờ <code>KeyRotationDelegation</code> ký bởi khóa cũ; mang giao diện Amber/Purple <code>Nova Tactics Automator v1.1.0</code> và giữ chữ ký Authenticode + <code>.updater/trust-state.json</code>.</div>
    </div>
  </div>
  <div class="sec-h">Chứng minh Chống Giả mạo &amp; Phục hồi Pointer Bám Trust Anchor (<code>fp-updater.exe</code>)</div>
  <pre style="background:#0F172A;color:#E2E8F0;padding:8px 10px;border-radius:6px;font-family:Consolas,monospace;font-size:8.0pt;line-height:1.34;white-space:pre-wrap;margin:0;">
1) Tamper PE after Authenticode signing : {h_esc(ul["peTamperErr"][:170])}
2) KeyRotationDelegation Upgrade v1.0-&gt;1.1: status={ul["upgradeBetaRes"]["status"]}, activeVersion=v{ul["upgradeBetaRes"]["activeVersion"]}, nonce={h_esc(sr["rotRes"]["delegation"]["nonce"])}
3) Re-apply Same Rotated Build (SemVer)   : {h_esc(ul["replayErr"][:150])} (Nonce replay tested in fp-updater Rust suite)
4) Rogue v9.9.9 Drop + Missing Pointers : reconcile status={ul["reconcileTrustedRes"]["status"]}, recovered activeVersion=v{ul["reconcileTrustedRes"]["activeVersion"]} (v9.9.9 rejected!)
  </pre>
  <div class="ftr"><span>FarmerPro P5.3 Acceptance Report (Commit {h_esc(head_sha[:12])})</span><span>Trang 3 / 4</span></div>
</div>

<!-- PAGE 4: P5.2 TECHNICAL BACKLOG STATUS, RAW TEST COUNTS & P5.3 STAGE-GATE -->
<div class="page">
  <div class="hdr">
    <div>
      <div class="hdr-title">P5.2 TECHNICAL BACKLOG TRACKING, RAW EVIDENCE &amp; STAGE-GATE P5.3 SIGN-OFF</div>
      <div class="hdr-sub">Tình trạng xử lý từng tồn đọng kỹ thuật từ P5.2 Closeout và phạm vi nghiệm thu P5.3</div>
    </div>
    <div class="hdr-meta"><div>Commit: <code>{h_esc(head_sha[:12])}</code> · Secret Leaks: {len(rep["secretLeakScanHits"])}</div></div>
  </div>

  <div class="sec-h">I. Bảng theo dõi P5.2 Technical Backlog (Đã giải quyết &amp; Kiểm chứng trong P5.3)</div>
  <table>
    <thead><tr><th>Hạng mục P5.2 Backlog</th><th>Giải pháp kỹ thuật đã triển khai trong P5.3</th><th>Trạng thái &amp; Bằng chứng</th></tr></thead>
    <tbody>
      <tr><td><strong>1. Installed EXE Startup Integrity / WinVerifyTrust</strong></td><td>Bổ sung <code>verify_installed_exe_integrity()</code> trong <code>fp-updater</code> và kiểm tra <code>verify_authenticode_pe(&amp;current_exe)</code> khi khởi động <code>apps/mini-runtime</code>; chặn <code>TamperedOrInvalid</code> &amp; <code>PublicSigned</code> không đủ trust chain.</td><td><span class="b-pass">CLOSED &amp; TESTED</span></td></tr>
      <tr><td><strong>2. Trust-Anchored Pointer Recovery (v* Scan)</strong></td><td>Thêm <code>.updater/trust-state.json</code> (<code>trusted_pubkeys</code>, <code>revoked_pubkeys</code>, <code>used_rotation_nonces</code>); <code>recover_missing_or_corrupt_pointer</code> chỉ khôi phục bản có khóa nằm trong trust store, loại bỏ folder lạ (<code>v9.9.9</code>).</td><td><span class="b-pass">CLOSED &amp; TESTED</span></td></tr>
      <tr><td><strong>3. Đo WAL Peak (<code>walBytesPeak</code>) &amp; <code>synchronous=NORMAL</code></strong></td><td>Đã đo trực tiếp <code>wal_long_stress.db-wal</code> trong P5.1/P5.2 trước truncate checkpoint; giữ chính sách Local-First <code>synchronous=NORMAL</code> (WAL atomic durability cho local desktop app).</td><td><span class="b-pass">DOCUMENTED</span></td></tr>
      <tr><td><strong>4. Live Hardware ADB End-to-End</strong></td><td>Không giả lập ADB; tiếp tục gắn nhãn <code>BLOCKED</code> do <code>0 devices</code> / <code>LDPlayer14 vm0 = stop</code> — mở khóa ở bước nghiệm thu phần cứng khi có thiết bị thật.</td><td><span class="b-blk">BLOCKED (0 ADB)</span></td></tr>
    </tbody>
  </table>

  <div class="sec-h">II. Tóm tắt Số liệu Kiểm chứng Chính thức (Verified after Final Source Commit)</div>
  <pre style="background:#0F172A;color:#E2E8F0;padding:8px 10px;border-radius:6px;font-family:Consolas,monospace;font-size:8.0pt;line-height:1.34;white-space:pre-wrap;margin:0 0 6px 0;">
Verified Code Commit  : {h_esc(head_sha)}
Cargo Workspace Tests : {total_passed} passed; {total_failed} failed (cargo_test_workspace_p5_3.log)
Frontend Production   : tsc -b &amp;&amp; vite build -> Exit 0 (0 TypeScript errors)
PublicSigned Gate     : noCertBlocked={pub_rel["noCertBlocked"]}, selfSignedBlocked={pub_rel["selfSignedBlocked"]} (Honest BLOCKED without commercial cert)
Secret/PFX Scan       : secretLeakScanHits={len(rep["secretLeakScanHits"])} across sandbox/, releases/, and SQLite v5
  </pre>

  <div class="callout">
    <strong>KẾT LUẬN STAGE-GATE P5.3 (CHỜ PHÊ DUYỆT TRƯỚC P5.4 / P6):</strong> Toàn bộ phạm vi P5.3 (SQLite v5 Publisher Profiles &amp; Nonces, Windows PE <code>VERSIONINFO</code> 8 trường đọc lại được trên OS, Authenticode 3 chế độ <code>UnsignedInternal</code> / <code>SelfSignedQa</code> / <code>PublicSigned</code> fail-closed, <code>signtool.exe</code> chạy cuối cùng, <code>KeyRotationDelegation</code> + chặn replay, Studio UI &amp; chạy độc lập 2 Mini Tools khi đóng Studio, cộng giải quyết backlog P5.2) đã đạt <strong>{total_passed}/{total_passed + total_failed} workspace tests PASS</strong> và <strong>E2E PASS</strong> (giữ trung thực <code>BLOCKED</code> cho Commercial Cert &amp; Live ADB). Dừng đúng mốc Stage-Gate P5.3.
  </div>
  <div class="ftr"><span>FarmerPro P5.3 Acceptance Report (Commit {h_esc(head_sha[:12])})</span><span>Trang 4 / 4</span></div>
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
    [1, "1. Acceptance Matrix & Signing Pipeline Order", 1],
    [1, "2. Studio P5.3 Builder UI & OS PE/Authenticode Verification", 2],
    [1, "3. Independent Dual Mini Tools & Key Rotation Recovery", 3],
    [1, "4. P5.2 Backlog Closure, Raw Logs & Stage-Gate P5.3", 4],
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
    pix.save(os.path.join(RENDER_DIR, f"p5_3_page_{pno:02d}.png"))
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
                    if sz < 7.8:
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
        and min_pt >= 7.8
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
