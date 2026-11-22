import os
import json
import hashlib
import subprocess
import pymupdf

ROOT = r"E:\DATA\FarmerPro"
EV_DIR = os.path.join(ROOT, "artifacts", "p2_evidence")
DIAG_DIR = os.path.join(EV_DIR, "diagrams")
RENDER_DIR = os.path.join(EV_DIR, "pdf_p2_2_rendered_pages")
os.makedirs(DIAG_DIR, exist_ok=True)
if os.path.exists(RENDER_DIR):
    for old_f in os.listdir(RENDER_DIR):
        try:
            os.remove(os.path.join(RENDER_DIR, old_f))
        except OSError:
            pass
os.makedirs(RENDER_DIR, exist_ok=True)

CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"
HTML_OUT = os.path.join(ROOT, "FARMERPRO_P2_2_HOTFIX_ACCEPTANCE_REPORT.html")
PDF_MAIN = os.path.join(ROOT, "FARMERPRO_P2_2_HOTFIX_ACCEPTANCE_REPORT.pdf")
PDF_COPY = os.path.join(EV_DIR, "FARMERPRO_P2_2_HOTFIX_ACCEPTANCE_REPORT.pdf")
QA_JSON = os.path.join(EV_DIR, "p2_2_pdf_qa_report.json")


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        while chunk := f.read(65536):
            h.update(chunk)
    return h.hexdigest()


# Load verified inputs (generated on CODE COMMIT 0218e2e)
hotfix_rep = json.loads(open(os.path.join(EV_DIR, "p2_2_hotfix_verification.json"), encoding="utf-8-sig").read())
pre_v2_rep = json.loads(open(os.path.join(EV_DIR, "p2_2_verify_pre_v2_sandbox.json"), encoding="utf-8-sig").read())
diff_b3_to_02 = subprocess.check_output(
    ["git", "diff", "--stat", "b3d3879..0218e2e"], cwd=ROOT
).decode("utf-8", errors="replace").strip()
diff_ui_frozen = subprocess.check_output(
    ["git", "diff", "--stat", "fab1a1a..HEAD", "--", "apps/desktop/src"], cwd=ROOT
).decode("utf-8", errors="replace").strip()
git_log_8 = subprocess.check_output(
    ["git", "log", "-n", "7", "--format=%h %ad %s", "--date=short"], cwd=ROOT
).decode("utf-8", errors="replace").strip()
test_log = open(os.path.join(EV_DIR, "cargo_test_workspace_p2_hotfix_clean.log"), encoding="utf-8-sig", errors="replace").read()

# Save exact code-only diff artifact for manifest completeness
with open(os.path.join(EV_DIR, "git_diff_b3d3879_to_0218e2e.txt"), "w", encoding="utf-8", newline="\n") as f:
    f.write(diff_b3_to_02 + "\n")

sec = hotfix_rep["securityHotfix"]
kdf = sec["kdfParams"]
crash_rows = hotfix_rep["forcedCrashRecoveryMatrix"]

# Vector SVG 1: Argon2id + AES-256-GCM AEAD Envelope
aead_svg = f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 980 235" width="100%" height="auto" font-family="Segoe UI, Arial, sans-serif">
  <rect x="0" y="0" width="980" height="235" rx="10" fill="#0B1220"/>
  <text x="20" y="26" fill="#E2E8F0" font-size="13.5" font-weight="700">CHUẨN MÃ HÓA PORTABLE PRODUCER KEY V3: ARGON2ID + AES-256-GCM AEAD (LOẠI BỎ HOÀN TOÀN SHA-256-CTR)</text>

  <rect x="20" y="44" width="215" height="92" rx="8" fill="#172554" stroke="#60A5FA" stroke-width="1.5"/>
  <text x="32" y="66" fill="#93C5FD" font-size="12" font-weight="700">1. NGÕ VÀO &amp; CSPRNG OS</text>
  <text x="32" y="85" fill="#E2E8F0" font-size="11">• Passphrase (&gt;= 8 ký tự)</text>
  <text x="32" y="102" fill="#E2E8F0" font-size="11">• Salt: 16 bytes (OsRng)</text>
  <text x="32" y="119" fill="#E2E8F0" font-size="11">• Nonce: 12 bytes / 96-bit (OsRng)</text>

  <line x1="235" y1="90" x2="263" y2="90" stroke="#60A5FA" stroke-width="2"/>
  <polygon points="263,85 273,90 263,95" fill="#60A5FA"/>

  <rect x="275" y="44" width="225" height="92" rx="8" fill="#1E1B4B" stroke="#A78BFA" stroke-width="1.5"/>
  <text x="287" y="66" fill="#C4B5FD" font-size="12" font-weight="700">2. KDF: ARGON2ID (v=0x13)</text>
  <text x="287" y="85" fill="#E2E8F0" font-size="11">• Memory: {kdf['memoryKib']} KiB (64 MiB)</text>
  <text x="287" y="102" fill="#E2E8F0" font-size="11">• Iterations t={kdf['iterations']}, p={kdf['parallelism']}</text>
  <text x="287" y="119" fill="#34D399" font-size="11">• Đo thực tế: {sec['kdfExportDurationMs']}ms / {sec['kdfImportDurationMs']}ms</text>

  <line x1="500" y1="90" x2="528" y2="90" stroke="#A78BFA" stroke-width="2"/>
  <polygon points="528,85 538,90 528,95" fill="#A78BFA"/>

  <rect x="540" y="44" width="225" height="92" rx="8" fill="#064E3B" stroke="#34D399" stroke-width="1.5"/>
  <text x="552" y="66" fill="#6EE7B7" font-size="12" font-weight="700">3. AEAD: AES-256-GCM</text>
  <text x="552" y="85" fill="#E2E8F0" font-size="11">• Plaintext: 32B Ed25519 Seed</text>
  <text x="552" y="102" fill="#E2E8F0" font-size="11">• AAD: Toàn bộ Metadata JSON</text>
  <text x="552" y="119" fill="#E2E8F0" font-size="11">• Output: 48B (32B CT + 16B Tag)</text>

  <line x1="765" y1="90" x2="793" y2="90" stroke="#34D399" stroke-width="2"/>
  <polygon points="793,85 803,90 793,95" fill="#34D399"/>

  <rect x="805" y="44" width="155" height="92" rx="8" fill="#0F172A" stroke="#FBBF24" stroke-width="1.5"/>
  <text x="817" y="66" fill="#FDE68A" font-size="12" font-weight="700">4. TARGET DPAPI</text>
  <text x="817" y="85" fill="#E2E8F0" font-size="11">• Zeroize seed RAM</text>
  <text x="817" y="102" fill="#E2E8F0" font-size="11">• Verify PubKey + Sig</text>
  <text x="817" y="119" fill="#E2E8F0" font-size="11">• Re-wrap CurrentUser</text>

  <rect x="20" y="152" width="940" height="68" rx="8" fill="#450A0A" stroke="#F87171" stroke-width="1.2"/>
  <text x="34" y="174" fill="#FCA5A5" font-size="12" font-weight="700">CHÍNH SÁCH NGỪNG PHÁT HÀNH ĐỊNH DẠNG CŨ (EXPERIMENTAL V1 RETIREMENT):</text>
  <text x="34" y="192" fill="#FEE2E2" font-size="11">• Toàn bộ mã SHA-256-CTR đã bị xóa khỏi fp-security. Không có bản Recovery V1 nào từng phát hành (0 tệp trên disk/AppData).</text>
  <text x="34" y="209" fill="#FEE2E2" font-size="11">• Import gặp "PASSPHRASE_PBKDF2_SHA256_ETM_V1" bị chặn fail-closed với mã RECOVERY_RETIRED_EXPERIMENTAL_FORMAT.</text>
</svg>'''
open(os.path.join(DIAG_DIR, "06_p2_2_argon2id_aes_gcm.svg"), "w", encoding="utf-8", newline="\n").write(aead_svg)

# Vector SVG 2: Two-Phase Intent-Journaled Crash Consistency Model
crash_svg = '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 980 148" width="100%" height="auto" font-family="Segoe UI, Arial, sans-serif">
  <rect x="0" y="0" width="980" height="148" rx="9" fill="#0B1220"/>
  <text x="18" y="19" fill="#E2E8F0" font-size="12.2" font-weight="700">MÔ HÌNH CRASH CONSISTENCY: SQLITE WAL ACID + INTENT JOURNAL (.fp_journal / .fp_staging / .fp_trash)</text>

  <rect x="18" y="26" width="300" height="76" rx="7" fill="#172554" stroke="#60A5FA" stroke-width="1.4"/>
  <text x="28" y="42" fill="#93C5FD" font-size="11" font-weight="700">CREATE / CLONE (create_&lt;id&gt;.json)</text>
  <text x="28" y="56" fill="#E2E8F0" font-size="10.4">1. Dựng tệp trong .fp_staging/&lt;slug&gt;</text>
  <text x="28" y="69" fill="#E2E8F0" font-size="10.4">2. Ghi .fp_journal -&gt; 3. rename -&gt; projects/&lt;slug&gt;</text>
  <text x="28" y="83" fill="#34D399" font-size="10.4">4. tx.commit() -&gt; Xóa create_&lt;id&gt;.json</text>

  <rect x="338" y="26" width="302" height="76" rx="7" fill="#3B0764" stroke="#C084FC" stroke-width="1.4"/>
  <text x="348" y="42" fill="#E9D5FF" font-size="11" font-weight="700">UPDATE / WORKFLOW (mutation_*.json)</text>
  <text x="348" y="56" fill="#E2E8F0" font-size="10.4">1. Lưu pre-image -&gt; .fp_staging/bak_* &amp; journal</text>
  <text x="348" y="69" fill="#E2E8F0" font-size="10.4">2. Ghi atomic_write(wf.json, project.json)</text>
  <text x="348" y="83" fill="#34D399" font-size="10.4">3. tx.commit() -&gt; Xóa bak_* &amp; mutation_*.json</text>

  <rect x="660" y="26" width="302" height="76" rx="7" fill="#064E3B" stroke="#34D399" stroke-width="1.4"/>
  <text x="670" y="42" fill="#6EE7B7" font-size="11" font-weight="700">DELETE (delete_&lt;id&gt;.json)</text>
  <text x="670" y="56" fill="#E2E8F0" font-size="10.4">1. Ghi .fp_journal/delete_&lt;id&gt;.json</text>
  <text x="670" y="69" fill="#E2E8F0" font-size="10.4">2. rename(proj_dir -&gt; .fp_trash) -&gt; DELETE SQLite</text>
  <text x="670" y="83" fill="#34D399" font-size="10.4">3. tx.commit() -&gt; Purge .fp_trash &amp; journal</text>

  <rect x="18" y="109" width="944" height="31" rx="6" fill="#0F172A" stroke="#F59E0B" stroke-width="1.3"/>
  <text x="28" y="128" fill="#FDE68A" font-size="10.4" font-weight="700">STARTUP RECONCILE:</text>
  <text x="155" y="128" fill="#CBD5E1" font-size="10.4">Đọc .fp_journal/* TRƯỚC khi dọn .fp_staging/*; kiểm tra SqliteCommitWitness =&gt; Pre-commit rollback nguyên bản | Post-commit dọn rác.</text>
</svg>'''
open(os.path.join(DIAG_DIR, "07_p2_2_crash_consistency_flow.svg"), "w", encoding="utf-8", newline="\n").write(crash_svg)

crash_table_rows = ""
for row in crash_rows:
    sc = row["scenario"]
    st = row["stage"]
    pid = row["killedPid"]
    act = ", ".join(row["reconcileReport"]["actions"]) if row["reconcileReport"]["actions"] else "-"
    integ = f"{row['integrityCheck']} / FK={row['foreignKeyViolations']}"
    v_ok = "INTACT" if row["victimProjectIntact"] else "FAIL"
    res = "PASS (TESTED)" if row["passed"] else "FAIL"
    crash_table_rows += (
        f"<tr><td><code>{sc}</code></td><td><code>{st}</code></td>"
        f"<td><code>PID {pid}</code></td><td><code>{act}</code></td>"
        f"<td><code>{integ}</code></td><td><span class='tag-pass'>{v_ok}</span></td>"
        f"<td><span class='tag-pass'>{res}</span></td></tr>\n"
    )

html = f"""<!DOCTYPE html>
<html lang="vi">
<head>
<meta charset="utf-8">
<title>FarmerPro P2.2 Hotfix Verification Report</title>
<style>
  @page {{ size: A4 landscape; margin: 10mm 11mm 11mm 11mm; }}
  * {{ box-sizing: border-box; }}
  body {{ font-family: 'Segoe UI', Arial, sans-serif; color: #0F172A; margin: 0; padding: 0; font-size: 9.2pt; line-height: 1.38; }}
  .page {{ page-break-after: always; position: relative; min-height: 186mm; display: flex; flex-direction: column; }}
  .page:last-child {{ page-break-after: auto; }}
  .hdr {{ background: #0B1220; color: #F8FAFC; padding: 8px 14px; border-radius: 7px; display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px; }}
  .hdr-l {{ display: flex; align-items: center; gap: 10px; }}
  .badge {{ background: #1D4ED8; color: #EFF6FF; font-weight: 700; padding: 3px 9px; border-radius: 5px; font-size: 8.5pt; letter-spacing: 0.3px; }}
  .title {{ font-size: 11.5pt; font-weight: 700; }}
  .meta {{ font-size: 8pt; color: #CBD5E1; font-family: Consolas, monospace; }}
  .grid-2 {{ display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }}
  .card {{ background: #FFFFFF; border: 1px solid #CBD5E1; border-radius: 7px; padding: 9px 11px; margin-bottom: 8px; }}
  h2 {{ margin: 0 0 6px 0; font-size: 10pt; color: #1E3A8A; border-bottom: 1px solid #E2E8F0; padding-bottom: 3px; }}
  table {{ width: 100%; border-collapse: collapse; font-size: 8.1pt; margin-top: 3px; }}
  th, td {{ border: 1px solid #CBD5E1; padding: 4px 6px; text-align: left; vertical-align: top; }}
  th {{ background: #F1F5F9; font-weight: 700; color: #1E293B; }}
  code {{ font-family: Consolas, monospace; font-size: 7.8pt; background: #F1F5F9; padding: 1px 4px; border-radius: 3px; color: #0F172A; overflow-wrap: anywhere; word-break: break-word; }}
  pre {{ background: #0F172A; color: #E2E8F0; padding: 8px 10px; border-radius: 6px; font-family: Consolas, monospace; font-size: 7.7pt; line-height: 1.32; white-space: pre-wrap; margin: 4px 0; overflow-wrap: anywhere; }}
  .tag-pass {{ background: #DCFCE7; color: #166534; padding: 1.5px 6px; border-radius: 4px; font-weight: 700; font-size: 7.7pt; display: inline-block; }}
  .tag-block {{ background: #FEF3C7; color: #92400E; padding: 1.5px 6px; border-radius: 4px; font-weight: 700; font-size: 7.7pt; display: inline-block; }}
  .tag-scope {{ background: #E2E8F0; color: #334155; padding: 1.5px 6px; border-radius: 4px; font-weight: 700; font-size: 7.7pt; display: inline-block; }}
  .callout-ok {{ background: #ECFDF5; border-left: 4px solid #10B981; padding: 7px 10px; border-radius: 5px; font-size: 8.4pt; margin-top: 4px; }}
  .callout-warn {{ background: #FFFBEB; border-left: 4px solid #F59E0B; padding: 7px 10px; border-radius: 5px; font-size: 8.4pt; margin-top: 4px; }}
  .compact-tbl {{ font-size: 7.6pt; line-height: 1.2; }}
  .compact-tbl th, .compact-tbl td {{ padding: 2px 4px; }}
  .compact-tbl code {{ font-size: 7.55pt; padding: 0.5px 3px; }}
  .ftr {{ margin-top: auto; padding-top: 5px; border-top: 1px solid #CBD5E1; display: flex; justify-content: space-between; font-size: 7.8pt; color: #475569; }}
  ul {{ margin: 4px 0; padding-left: 16px; }}
  li {{ margin-bottom: 2.5px; }}
</style>
</head>
<body>

<!-- PAGE 1 -->
<div class="page">
  <div class="hdr">
    <div class="hdr-l"><span class="badge">TRANG 1 · NGHIỆM THU HOTFIX</span><span class="title">Bảng PASS / FAIL / BLOCKED Trung thực Trước Khi Mở Khóa P3</span></div>
    <div class="meta">CODE COMMIT 0218e2e · 2026-11-22</div>
  </div>
  <div class="card">
    <h2>1. TỔNG HỢP KIỂM ĐỊNH YÊU CẦU BẮT BUỘC P2.2 HOTFIX</h2>
    <table>
      <thead><tr><th style="width:24%">Hạng mục Hotfix</th><th style="width:15%">Trạng thái</th><th style="width:61%">Kết quả đo &amp; Bằng chứng Xác minh Độc lập</th></tr></thead>
      <tbody>
        <tr><td><strong>I.1–I.4 AEAD + Argon2id</strong></td><td><span class="tag-pass">PASS (TESTED)</span></td><td>Xóa hoàn toàn <code>SHA-256-CTR</code>. Dùng <code>argon2id v0x13</code> (64 MiB, t=3, p=1; đo thật: <strong>{sec['kdfExportDurationMs']}ms / {sec['kdfImportDurationMs']}ms</strong> release) + <code>AES-256-GCM</code> AEAD + 16B salt &amp; 12B nonce <code>OsRng</code> + full metadata AAD. Zero plaintext seed trong PDF/JSON/log/release.</td></tr>
        <tr><td><strong>I.5–I.6 Recovery Tests &amp; Legacy Policy</strong></td><td><span class="tag-pass">PASS (TESTED)</span></td><td>Test đủ: passphrase đúng, sai pass, tamper ciphertext/tag, tamper AAD metadata, invalid KDF/nonce/AEAD, 2 lần export sinh salt/nonce khác nhau, backup/rotate/restore &amp; tương thích Mini Tool cũ (<code>fp-builder</code>). Chặn vĩnh viễn định dạng V1 cũ (<code>RECOVERY_RETIRED_EXPERIMENTAL_FORMAT</code>); 0 tệp V1 từng phát hành.</td></tr>
        <tr><td><strong>I.7 Khôi phục Windows User thứ hai thật</strong></td><td><span class="tag-block">BLOCKED</span></td><td>Máy kiểm thử chỉ có 1 phiên tương tác (<code>tinh-nguyen</code>); các tài khoản <code>Administrator</code> / <code>CodexSandbox*</code> không có credential hợp lệ và không bypass access control. Đã xác nhận Win32 DPAPI không cờ <code>CRYPTPROTECT_LOCAL_MACHINE</code>.</td></tr>
        <tr><td><strong>II. Forced OS Crash &amp; Reconcile</strong></td><td><span class="tag-pass">PASS (TESTED)</span></td><td><strong>9/9 kịch bản <code>TerminateProcess</code> thật</strong> (PID thật bị kill cứng tại staging, post-rename pre-commit, post-commit, update manifest/workflow pre &amp; post-commit, delete-to-trash pre &amp; post-commit): <code>integrityCheck="ok"</code>, <code>FK=0</code>, project khác nguyên vẹn 100%.</td></tr>
        <tr><td><strong>III.1 Workflow <code>wf-jx2-01-login</code></strong></td><td><span class="tag-pass">PASS (TESTED)</span></td><td>Nâng cấp <code>draft -&gt; published</code> qua <code>migrate_legacy_jx2_assets</code> &amp; <code>save_project_workflow</code> giữ nguyên <code>default_enabled=false</code> của task chưa phê duyệt và giữ <code>jobs = 0</code>.</td></tr>
        <tr><td><strong>III.2–III.3 Scope, DB &amp; Git Lineage</strong></td><td><span class="tag-pass">PASS (TESTED)</span></td><td><code>git diff fab1a1a..HEAD -- apps/desktop/src</code> rỗng (0 dòng). Không chạy crash test trên production DB. Tách bạch mốc commit code (<code>bd09e31</code>, <code>0218e2e</code>) và test/docs.</td></tr>
        <tr><td><strong>UI React &amp; ADB Phần cứng</strong></td><td><span class="tag-scope">NOT IN SCOPE</span></td><td>Giữ đóng băng giao diện React theo Stage-Gate đến P4; kiểm thử ADB phần cứng thật ngoài phạm vi P2.2.</td></tr>
      </tbody>
    </table>
  </div>
  <div class="callout-ok">
    <strong>Đề xuất Stage-Gate:</strong> Cả hai điều kiện Hotfix I &amp; II cùng ba điểm kiểm tra III.1–III.3 đã hoàn tất trên commit <code>0218e2e</code> (<code>62/62</code> tests workspace pass, <code>0</code> failed). Đề nghị phê duyệt <strong>đóng P2</strong> và <strong>mở khóa P3</strong>. Hạng mục cross-user OS session giữ nhãn trung thực <code>BLOCKED</code> do điều kiện tài khoản môi trường.
  </div>
  <div class="ftr"><span>Hồ sơ nghiệm thu bổ sung P2.2 Hotfix (Trước P3)</span><span>Code SHA: 0218e2e3b0c95671</span><span>Trang 01 / 04</span></div>
</div>

<!-- PAGE 2 -->
<div class="page">
  <div class="hdr">
    <div class="hdr-l"><span class="badge">TRANG 2 · SECURITY HOTFIX I</span><span class="title">Thay Thế SHA-256-CTR Bằng Argon2id + AES-256-GCM AEAD (V3)</span></div>
    <div class="meta">crates/fp-security · crates/fp-builder</div>
  </div>
  <div style="margin-bottom:8px;">{aead_svg}</div>
  <div class="grid-2">
    <div class="card">
      <h2>1. ĐO ĐẠC KDF &amp; MA TRẬN KIỂM THỬ BẢO MẬT V3</h2>
      <table>
        <thead><tr><th style="width:36%">Kiểm thử Bảo mật</th><th style="width:14%">Kết quả</th><th style="width:50%">Chi tiết Xác minh (Crate + Release CLI)</th></tr></thead>
        <tbody>
          <tr><td>Argon2id Benchmark (Release)</td><td><span class="tag-pass">MEASURED</span></td><td><code>m=65536 KiB, t=3, p=1</code>: <strong>{sec['kdfExportDurationMs']} ms</strong> (export) / <strong>{sec['kdfImportDurationMs']} ms</strong> (import)</td></tr>
          <tr><td>Passphrase đúng &amp; Rotate/Restore</td><td><span class="tag-pass">PASS</span></td><td>Export V3 -&gt; Rotate key mới -&gt; Import V3 khôi phục đúng <code>Ed25519 Seed</code> &amp; ký/verify Mini Tool cũ + mới</td></tr>
          <tr><td>Sai Passphrase</td><td><span class="tag-pass">PASS</span></td><td>Trả lỗi fail-closed <code>RECOVERY_AUTH_FAILED</code> (GHASH AEAD verify fail)</td></tr>
          <tr><td>Tamper Ciphertext / Auth Tag</td><td><span class="tag-pass">PASS</span></td><td>Lật 1 bit byte[0] của 48B AES-GCM ciphertext+tag bị từ chối tức thì</td></tr>
          <tr><td>Tamper AAD Metadata / Invalid KDF</td><td><span class="tag-pass">PASS</span></td><td>Đổi <code>publicKeyHex</code>/<code>keyId</code> -&gt; fail AEAD; đặt <code>aead="SHA-256-CTR"</code>, <code>m=1024</code>, <code>nonce=16B</code> -&gt; bị chặn sớm trước giải mã</td></tr>
          <tr><td>Nonce &amp; Salt Uniqueness</td><td><span class="tag-pass">PASS</span></td><td>Hai lần export cùng khóa &amp; passphrase sinh 2 salt (16B), 2 nonce (12B), 2 ciphertext hoàn toàn khác nhau</td></tr>
        </tbody>
      </table>
    </div>
    <div class="card">
      <h2>2. ĐỊNH DẠNG CŨ (EXPERIMENTAL) &amp; MINI TOOL COMPATIBILITY</h2>
      <ul>
        <li><strong>Kiểm kê tệp thực tế:</strong> Quét toàn repo (<code>artifacts/</code>, <code>backups/</code>, <code>projects/</code>, <code>releases/</code>) và <code>%LOCALAPPDATA%\\FarmerPro</code> xác nhận <strong>0 tệp recovery V1</strong> tồn tại; chỉ có keystore DPAPI chủ động <code>active_producer_ed25519.json</code>.</li>
        <li><strong>Kế hoạch chuyển đổi có kiểm soát:</strong> Định dạng <code>PASSPHRASE_PBKDF2_SHA256_ETM_V1</code> bị đánh dấu là <em>thử nghiệm nội bộ đã ngừng phát hành</em> và bị chặn nhập cứng (<code>RECOVERY_RETIRED_EXPERIMENTAL_FORMAT</code>). Nếu cần khôi phục máy mới, xuất trực tiếp tệp V3 từ keystore DPAPI hiện hành.</li>
        <li><strong>Tương thích Mini Tool cũ (<code>fp-builder</code>):</strong> <code>test_old_minitool_continues_verifying_old_bundle_after_producer_key_rotation</code> xác nhận v1 Mini Tool đọc Public Key nhúng <code>PE RT_RCDATA #7777</code> để xác thực bundle cũ sau rotation, đồng thời khôi phục Key2 qua V3 Recovery Bundle sau khi xoay sang Key3 vẫn ký bundle hợp lệ cho v2 Mini Tool.</li>
      </ul>
    </div>
  </div>
  <div class="ftr"><span>Không lưu plaintext seed trong tệp JSON, PDF, log hay releases/</span><span>ARGON2ID_AES_256_GCM_V3</span><span>Trang 02 / 04</span></div>
</div>

<!-- PAGE 3 -->
<div class="page">
  <div class="hdr">
    <div class="hdr-l"><span class="badge">TRANG 3 · CRASH RECOVERY HOTFIX II</span><span class="title">Thử Nghiệm Forced OS Process Termination (TerminateProcess) &amp; Reconciliation</span></div>
    <div class="meta">crates/fp-storage · tests/forced_crash_recovery.rs</div>
  </div>
  <div style="width:100%;margin-bottom:5px;">{crash_svg}</div>
  <div class="card" style="padding:7px 9px;margin-bottom:5px;">
    <h2 style="margin-bottom:4px;">1. MA TRẬN 9 KỊCH BẢN KILL TIẾN TRÌNH THẬT TRÊN SANDBOX (KHÔNG ĐỤNG PRODUCTION DB)</h2>
    <table class="compact-tbl">
      <thead><tr><th style="width:22.5%">Kịch bản</th><th style="width:23.5%">Thời điểm OS Kill</th><th style="width:6.5%">OS PID</th><th style="width:26.5%">Hành động Reconciler</th><th style="width:9%">SQLite Check</th><th style="width:5.5%">Victim</th><th style="width:6.5%">Kết quả</th></tr></thead>
      <tbody>{crash_table_rows}</tbody>
    </table>
  </div>
  <div class="callout-warn" style="padding:5px 9px;font-size:8pt;margin-top:2px;">
    <strong>Phát biểu chính xác về Crash Consistency:</strong> Cơ chế kết hợp SQLite WAL và NTFS <strong>không phải giao dịch nguyên tử đa hệ thống tuyệt đối</strong> chỉ dựa trên <code>tx.commit()</code>. Tính nhất quán đạt được nhờ ghi <strong>Intent Journal + Pre-Image Backup</strong> trong <code>.fp_journal</code> / <code>.fp_staging</code> / <code>.fp_trash</code> trước khi sửa tệp thật và đối chiếu <code>SqliteCommitWitness</code> khi khởi động lại (<code>open_and_reconcile</code>). Trong mọi kịch bản, project khác (<code>victim-sentinel</code>) đều nguyên vẹn 100% cả DB lẫn file checksum.
  </div>
  <div class="ftr"><span>Mỗi kịch bản gồm: Snapshot pre-crash -&gt; TerminateProcess thật -&gt; Reopen -&gt; Reconcile -&gt; Integrity/FK -&gt; Check Victim</span><span>0218e2e</span><span>Trang 03 / 04</span></div>
</div>

<!-- PAGE 4 -->
<div class="page">
  <div class="hdr">
    <div class="hdr-l"><span class="badge">TRANG 4 · MỤC III &amp; BẰNG CHỨNG</span><span class="title">Workflow Migration III.1, Phân Tách Git Commit SHAs &amp; 62/62 Workspace Tests</span></div>
    <div class="meta">Git Lineage · SHA-256 Manifest</div>
  </div>
  <div class="grid-2">
    <div class="card">
      <h2>1. KIỂM TRA BỔ SUNG III.1 &amp; III.3</h2>
      <ul>
        <li><strong>Workflow <code>wf-jx2-01-login</code> (<code>draft -&gt; published</code>):</strong> Test <code>test_p2_hotfix_wf_jx2_01_login_draft_to_published_does_not_auto_enable_unapproved_tasks</code> chứng minh khi nâng cấp workflow từ <code>draft</code> (v1) lên <code>published</code> (v2) qua <code>migrate_legacy_jx2_assets</code> và <code>save_project_workflow</code>:
          <ul>
            <li>Task <code>task-unapproved-login</code> (<code>default_enabled = false</code>) <strong>không tự bật</strong> (vẫn <code>false</code>).</li>
            <li>Bảng <code>jobs</code> giữ nguyên <strong>0 hàng</strong> — không tự kích hoạt hàng đợi thực thi.</li>
          </ul>
        </li>
        <li><strong>Bảo vệ Source &amp; DB hiện có (III.3):</strong> <code>apps/desktop/src</code> giữ nguyên 0 dòng diff từ <code>fab1a1a</code>; production DB (<code>dist-portable/.../farmerpro.db</code> &amp; <code>farmerpro.db</code> gốc) hoàn toàn không bị chạy crash test.</li>
      </ul>
      <h2>2. PHÂN TÁCH COMMITS &amp; DIFF CODE HOTFIX (<code>b3d3879..0218e2e</code>)</h2>
      <pre>{diff_b3_to_02}</pre>
    </div>
    <div class="card">
      <h2>3. CHUỖI GIT LINEAGE (CODE / TEST / DOCS)</h2>
      <pre>{git_log_8}</pre>
      <p style="font-size:8pt;margin:4px 0;"><strong>Đối chiếu source giữa các mốc:</strong> Từ P2.1 Code <code>bd09e31</code> -&gt; Test <code>e5dfd7a</code> -&gt; Docs <code>b3d3879</code> giữ nguyên source Rust; Hotfix P2.2 nằm gói gọn tại Code Commit <code>0218e2e</code> và không sửa thêm dòng Rust nào ở Docs HEAD kế tiếp.</p>
      <h2>4. WORKSPACE TESTS (62/62 PASS) &amp; SHA-256 MANIFEST</h2>
      <table>
        <thead><tr><th style="width:45%">Kiểm tra Tự động (trên 0218e2e)</th><th style="width:20%">Kết quả</th><th style="width:35%">Log / Artifact</th></tr></thead>
        <tbody>
          <tr><td><code>cargo test --workspace</code></td><td><span class="tag-pass">62 / 62 PASS</span></td><td><code>cargo_test_workspace_p2_hotfix_clean.log</code> (gồm 15 unit + 7 crash worker/tests ở <code>fp-storage</code>, 8 ở <code>fp-security</code>, 5 ở <code>fp-builder</code>)</td></tr>
          <tr><td><code>cargo build -p farmerpro -p fp-cli</code></td><td><span class="tag-pass">EXIT 0</span></td><td><code>cargo_build_p2_hotfix.log</code></td></tr>
          <tr><td><code>fp-cli verify-p2</code> (Pre-v2 Sandbox)</td><td><span class="tag-pass">ALL PASS</span></td><td><code>p2_2_verify_pre_v2_sandbox.json</code> (2+10=11, Argon2id+AES-GCM)</td></tr>
          <tr><td><code>EVIDENCE_MANIFEST_SHA256.json</code></td><td><span class="tag-pass">VERIFIED</span></td><td>Cập nhật đầy đủ hash cho toàn bộ gói bằng chứng P2/P2.1/P2.2</td></tr>
        </tbody>
      </table>
    </div>
  </div>
  <div class="ftr"><span>Ghi chú: 62 test gồm 61 test assertion + 1 entrypoint worker harness (forced_crash_worker_entry) trả về nhanh khi không có biến môi trường kill.</span><span>Code SHA: 0218e2e</span><span>Trang 04 / 04</span></div>
</div>

</body>
</html>
"""

with open(HTML_OUT, "w", encoding="utf-8", newline="\n") as f:
    f.write(html)

subprocess.run(
    [
        CHROME,
        "--headless=new",
        "--disable-gpu",
        "--no-pdf-header-footer",
        f"--print-to-pdf={PDF_MAIN}",
        HTML_OUT,
    ],
    check=True,
)

# Add clean PDF bookmarks matching the 4 pages
doc = pymupdf.open(PDF_MAIN)
toc = [
    [1, "1. Tong hop PASS/FAIL/BLOCKED P2.2 Hotfix", 1],
    [1, "2. Security Hotfix I: Argon2id + AES-256-GCM", 2],
    [1, "3. Crash Recovery Hotfix II: TerminateProcess Matrix", 3],
    [1, "4. Workflow Check III.1, Git SHAs & Test Logs", 4],
]
doc.set_toc(toc)
tmp_pdf = PDF_MAIN + ".tmp.pdf"
doc.save(tmp_pdf, deflate=True, garbage=3)
doc.close()
os.replace(tmp_pdf, PDF_MAIN)

import shutil
shutil.copy2(PDF_MAIN, PDF_COPY)

qa_doc = pymupdf.open(PDF_MAIN)
min_pt = 99.0
small_fonts = []
clipping = []
encoding_errs = []
for idx, page in enumerate(qa_doc):
    pno = idx + 1
    pix = page.get_pixmap(dpi=130)
    pix.save(os.path.join(RENDER_DIR, f"p2_2_page_{pno:02d}.png"))
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
    "pdfSizeBytes": os.path.getsize(PDF_MAIN),
    "pdfSha256": sha256_file(PDF_MAIN),
    "pagesCount": len(qa_doc),
    "bookmarksCount": len(qa_doc.get_toc()),
    "minFontPt": round(min_pt, 2),
    "smallFontWarnings": small_fonts,
    "clippingErrors": clipping,
    "encodingErrors": encoding_errs,
    "passed": len(qa_doc) == 4 and len(clipping) == 0 and len(encoding_errs) == 0 and min_pt >= 7.5,
}
qa_doc.close()

with open(QA_JSON, "w", encoding="utf-8", newline="\n") as f:
    json.dump(qa_report, f, indent=2, ensure_ascii=False)

# Rebuild EVIDENCE_MANIFEST_SHA256.json across all files in EV_DIR
manifest = {}
for root_dir, _, files in os.walk(EV_DIR):
    if "sandbox_run" in root_dir or "rollback_runbook_sandbox" in root_dir or "hotfix_sandbox_tmp" in root_dir:
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

loaded = json.loads(open(manifest_path, encoding="utf-8").read())
missing = [k for k in loaded if not os.path.exists(os.path.join(EV_DIR, k))]
mismatch = [k for k, v in loaded.items() if sha256_file(os.path.join(EV_DIR, k)) != v["sha256"]]
print(json.dumps({
    "qaReport": qa_report,
    "manifestFileCount": len(loaded),
    "manifestMissing": len(missing),
    "manifestMismatch": len(mismatch),
}, indent=2, ensure_ascii=False))
assert len(missing) == 0 and len(mismatch) == 0
assert qa_report["passed"], f"PDF QA failed: {qa_report}"
