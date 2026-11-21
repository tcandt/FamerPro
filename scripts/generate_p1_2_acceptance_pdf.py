import os
import shutil
import base64
import hashlib
import subprocess
import pymupdf

ROOT = r"E:\DATA\FarmerPro"
EVIDENCE_DIR = os.path.join(ROOT, "artifacts", "p1_2_evidence")
IMG_DIR = os.path.join(EVIDENCE_DIR, "images")
DIAG_DIR = os.path.join(EVIDENCE_DIR, "diagrams")

HTML_OUT = os.path.join(ROOT, "FARMERPRO_P1_2_ARCHITECTURE_UX_ACCEPTANCE_REPORT.html")
PDF_OUT = os.path.join(ROOT, "FARMERPRO_P1_2_ARCHITECTURE_UX_ACCEPTANCE_REPORT.pdf")
PDF_COPY = os.path.join(EVIDENCE_DIR, "FARMERPRO_P1_2_ARCHITECTURE_UX_ACCEPTANCE_REPORT.pdf")

def img_b64(fname):
    path = os.path.join(IMG_DIR, fname)
    with open(path, "rb") as f:
        return "data:image/png;base64," + base64.b64encode(f.read()).decode("ascii")

TOTAL_PAGES = 33

def page_wrap(page_no, section_badge, page_title, inner_html):
    return f"""
    <section class="pdf-page" id="page-{page_no:02d}">
      <div class="page-top-bar">
        <div class="pt-left">
          <span class="pt-badge">{section_badge}</span>
          <span class="pt-title">{page_title}</span>
        </div>
        <div class="pt-right">
          <span>FARMERPRO P1.2 ACCEPTANCE DOSSIER · v1.2.0-GATE · 2026-11-21</span>
        </div>
      </div>
      <div class="page-content">
        {inner_html}
      </div>
      <div class="page-bottom-bar">
        <span>Hồ sơ nghiệm thu Kiến trúc & Giao diện FarmerPro Studio P1.2 (Chưa chuyển sang P2–P6 trước khi được phê duyệt)</span>
        <span>Nguồn đối chiếu: FARMERPRO_P1_2_STUDIO_BLUEPRINT.md & artifacts/p1_2_evidence/</span>
        <span class="pg-num">Trang {page_no:02d} / {TOTAL_PAGES:02d}</span>
      </div>
    </section>
    """

def fig_box(b64_src, fig_num, title, source_path, status_type, max_h="120mm", compact=False):
    if status_type == "ACTUAL":
        badge_cls = "st-actual"
        badge_txt = "ACTUAL IMPLEMENTATION" if compact else "ACTUAL IMPLEMENTATION (Ảnh chụp ứng dụng / thiết bị đang chạy thực tế)"
    elif status_type == "PROPOSED":
        badge_cls = "st-proposed"
        badge_txt = "PROPOSED DESIGN — NOT IMPLEMENTED" if compact else "PROPOSED DESIGN — NOT IMPLEMENTED (Bản thiết kế mục tiêu P1.2 chờ duyệt cho P4)"
    else:
        badge_cls = "st-diagram"
        badge_txt = "TECHNICAL ARCHITECTURE DIAGRAM" if compact else "TECHNICAL ARCHITECTURE DIAGRAM (Sơ đồ kiến trúc kỹ thuật P1.2)"

    return f"""
    <div class="fig-container">
      <div class="fig-img-box">
        <img src="{b64_src}" style="max-height:{max_h};" />
      </div>
      <div class="fig-caption">
        <div class="fc-left">
          <b>{fig_num}:</b> <span>{title}</span>
          <span class="fc-src">Nguồn: <code>{source_path}</code></span>
        </div>
        <span class="st-badge {badge_cls}">{badge_txt}</span>
      </div>
    </div>
    """

pages = []

# ==============================================================================
# PAGE 01: COVER PAGE
# ==============================================================================
pages.append(page_wrap(1, "HỒ SƠ NGHIỆM THU P1.2", "Trang Bìa & Thông Tin Hồ Sơ Nghiệm Thu Kiến Trúc Sản Phẩm", """
  <div class="cover-hero">
    <div class="cover-pill-row">
      <span class="st-badge st-proposed">STAGE-GATE P1.2: TRÌNH PHÊ DUYỆT KIẾN TRÚC &amp; UX</span>
      <span class="st-badge st-actual">CHƯA TRIỂN KHAI P2–P6 (GIỮ NGUYÊN 100% CODE UI &amp; PRODUCTION DB)</span>
      <span class="st-badge st-diagram">34/34 UNIT TESTS PASS · PE RT_RCDATA #7777 VERIFIED</span>
    </div>
    <h1 class="cover-h1">FARMERPRO P1.2 — BÁO CÁO NGHIỆM THU KIẾN TRÚC SẢN PHẨM &amp; THIẾT KẾ GIAO DIỆN STUDIO IDE</h1>
    <div class="cover-sub">
      Hồ sơ thiết kế tổng thể chuyển đổi <b>FarmerPro.exe</b> từ giao diện điều khiển đơn game thành <b>Nền tảng Sản xuất Đa Auto-Tools (Multi-Game Mini Auto-Tool Software Factory)</b>, kèm trực quan hóa toàn bộ giao diện Studio mới, 10 sơ đồ kiến trúc kỹ thuật, bằng chứng mật mã Ed25519 PE Root of Trust và lộ trình P2–P6.
    </div>
  </div>

  <div class="grid-2" style="margin-top:2mm;">
    <div class="card">
      <div class="card-hdr">1. THÔNG TIN ĐỊNH DANH HỒ SƠ NGHIỆM THU</div>
      <table class="tbl-compact">
        <tr><th style="width:32%;">Thuộc tính Hồ sơ</th><th style="width:68%;">Giá trị Thực tế &amp; Trạng thái</th></tr>
        <tr><td><b>Tệp bàn giao chính</b></td><td><code>E:\\DATA\\FarmerPro\\FARMERPRO_P1_2_ARCHITECTURE_UX_ACCEPTANCE_REPORT.pdf</code></td></tr>
        <tr><td><b>Bản sao bằng chứng</b></td><td><code>E:\\DATA\\FarmerPro\\artifacts\\p1_2_evidence\\FARMERPRO_P1_2_ARCHITECTURE_UX_ACCEPTANCE_REPORT.pdf</code></td></tr>
        <tr><td><b>Blueprint đối chiếu</b></td><td><code>E:\\DATA\\FarmerPro\\artifacts\\p1_2_evidence\\FARMERPRO_P1_2_STUDIO_BLUEPRINT.md</code></td></tr>
        <tr><td><b>Phiên bản &amp; Ngày lập</b></td><td><b>Phiên bản v1.2.0-P1.2-GATE</b> · Ngày lập: <b>2026-11-21</b></td></tr>
        <tr><td><b>Quy chuẩn trình bày</b></td><td>A4 Ngang (297mm × 210mm) · 33 Trang · 100% Ảnh &amp; Vector nhúng trực tiếp trong PDF</td></tr>
        <tr><td><b>Trạng thái Phê duyệt</b></td><td><span class="st-badge st-proposed">CHỜ NGƯỜI PHÊ DUYỆT XEM XÉT &amp; XÁC NHẬN (CHƯA CHUYỂN SANG P2–P6)</span></td></tr>
      </table>
    </div>
    <div class="card">
      <div class="card-hdr">2. TÓM TẮT KẾT QUẢ KỸ THUẬT ĐÃ KIỂM CHỨNG ĐẾN P1.2</div>
      <table class="tbl-compact">
        <tr><th style="width:34%;">Hạng mục Kỹ thuật</th><th style="width:66%;">Bằng chứng Thực tế Đã Xác thực</th></tr>
        <tr><td><b>Unit Tests Workspace</b></td><td><b>34/34 PASS (0 FAILED)</b> trên 14 crates (Khôi phục 2 test bị sót ở P1.1 + thêm 1 test PE #7777)</td></tr>
        <tr><td><b>Cô lập Builder/Runtime</b></td><td><code>cargo tree -p fp-mini-runtime</code> xác nhận <b>0 phụ thuộc</b> vào <code>fp-builder</code> hay khóa bí mật</td></tr>
        <tr><td><b>Ed25519 Root of Trust</b></td><td>Nhúng khóa công khai 32-byte vào <b>PE <code>RT_RCDATA (#10) ID #7777</code></b> của file <code>.exe</code> ở Step 5</td></tr>
        <tr><td><b>Tamper Suite (5 Cases)</b></td><td><b>5/5 Kịch bản bị chặn đứng</b> (Bao gồm Case 5 thay cả <code>trusted_publisher.pub</code> + <code>signature</code>)</td></tr>
        <tr><td><b>Live ADB &amp; WinRT OCR</b></td><td>Chạy thật trên <b>LDPlayer <code>emulator-5554</code> (SM_X710N)</b>: 19 events, Safe Tap (111,170), OCR PASS</td></tr>
        <tr><td><b>Hai Mini Tools Độc lập</b></td><td><code>Auto_JX2_Mobile.exe</code> (5 tasks, Amber) &amp; <code>Auto_Kingdom_SLG.exe</code> (4 tasks, Emerald)</td></tr>
      </table>
    </div>
  </div>

  <div class="card" style="margin-top:2mm;">
    <div class="card-hdr">3. NGUYÊN TẮC TRUNG THỰC KỸ THUẬT &amp; PHÂN ĐỊNH RÕ TRẠNG THÁI TRONG BÁO CÁO (AGENTS.MD RULE #2)</div>
    <div class="grid-3">
      <div class="sub-card">
        <span class="st-badge st-actual">ACTUAL IMPLEMENTATION</span>
        <p style="margin-top:1mm;font-size:9.3px;">Áp dụng cho ảnh chụp màn hình từ ứng dụng <code>FarmerPro.exe</code> v0.1.0 đang chạy thực tế, hai Mini Tools độc lập (<code>Auto_JX2_Mobile.exe</code>, <code>Auto_Kingdom_SLG.exe</code>) và ảnh chụp thao tác thật trên giả lập LDPlayer <code>emulator-5554</code>.</p>
      </div>
      <div class="sub-card">
        <span class="st-badge st-proposed">PROPOSED DESIGN — NOT IMPLEMENTED</span>
        <p style="margin-top:1mm;font-size:9.3px;">Áp dụng cho 8 màn hình thiết kế giao diện FarmerPro Studio mới (<code>Ảnh 01</code> → <code>Ảnh 08</code>) và 3 bố cục độ phân giải mục tiêu (<code>1280×720</code>, <code>1600×900</code>, <code>1920×1080</code>). Đây là bản thiết kế chờ phê duyệt để triển khai ở giai đoạn P4.</p>
      </div>
      <div class="sub-card">
        <span class="st-badge st-diagram">KHÔNG TỰ CHẤM ĐIỂM / KHÔNG MOCK PASS</span>
        <p style="margin-top:1mm;font-size:9.3px;">Báo cáo tuyệt đối không dùng nhãn tự khen, không trình bày bản thiết kế như tính năng đã hoàn tất, và giải thích rõ giới hạn bảo mật của PE Resource khi chưa có chữ ký số Windows Authenticode của hệ điều hành.</p>
      </div>
    </div>
  </div>
"""))

# ==============================================================================
# PAGE 02: TABLE OF CONTENTS & LIST OF FIGURES
# ==============================================================================
pages.append(page_wrap(2, "MỤC LỤC & DANH MỤC HÌNH ẢNH", "Mục Lục Hồ Sơ Nghiệm Thu & Danh Mục 25 Hình Ảnh + 14 Sơ Đồ Kỹ Thuật", """
  <div class="grid-2">
    <div class="card">
      <div class="card-hdr">MỤC LỤC CHI TIẾT BÁO CÁO NGHIỆM THU (33 TRANG A4 NGANG)</div>
      <table class="tbl-compact">
        <tr><th style="width:15%;">Phần / Mục</th><th style="width:73%;">Nội dung Chi tiết</th><th style="width:12%;">Trang</th></tr>
        <tr><td><b>Mở đầu</b></td><td><a href="#page-01">Trang bìa, Thông tin định danh hồ sơ &amp; Tóm tắt kết quả kiểm chứng P1.2</a></td><td><a href="#page-01">01–02</a></td></tr>
        <tr><td><b>PHẦN 1</b></td><td><a href="#page-03"><b>Tổng quan Dự án:</b> Định vị Studio IDE, Mini Runtime, giới hạn UI cũ, lịch sử P1→P1.2, ảnh Studio hiện tại &amp; 2 Mini Tools</a></td><td><a href="#page-03">03–05</a></td></tr>
        <tr><td><b>PHẦN 2</b></td><td><a href="#page-06"><b>So sánh Kiến trúc Cũ và Mới:</b> Sơ đồ Before vs Proposed &amp; Bảng ánh xạ 10 màn hình cũ sang 6 khu vực mới (0% mất tính năng)</a></td><td><a href="#page-06">06–07</a></td></tr>
        <tr><td><b>PHẦN 3</b></td><td><a href="#page-08"><b>Thiết kế Giao diện FarmerPro Studio Mới (Ảnh 01 → Ảnh 08):</b><br/>
          • Ảnh 01: Dashboard mới (Trang 08) · Ảnh 02: Game Projects (Trang 09)<br/>
          • Ảnh 03: Workflow Studio (Trang 10) · Ảnh 04: Tasks &amp; Modules (Trang 11)<br/>
          • Ảnh 05: Device Lab (Trang 12) · Ảnh 06: Test &amp; Debug (Trang 13)<br/>
          • Ảnh 07: Mini Tool Builder (Trang 14) · Ảnh 08: Release Management (Trang 15)</a></td><td><a href="#page-08">08–15</a></td></tr>
        <tr><td><b>PHẦN 4</b></td><td><a href="#page-16"><b>10 Sơ đồ Kiến trúc Kỹ thuật Chất lượng cao (Sơ đồ IV.1 → IV.10):</b><br/>
          • IV.1 Cargo Dependency Graph &amp; IV.2 Studio Architecture (Trang 16)<br/>
          • IV.3 GameProject Data Model &amp; IV.4 Workflow Runtime Execution (Trang 17)<br/>
          • IV.5 ADB/Emulator Adapter &amp; IV.6 Task Scheduler Multi-Device (Trang 18)<br/>
          • IV.7 Mini Tool UI Schema Renderer &amp; IV.8 8-Step Build Pipeline (Trang 19)<br/>
          • IV.9 Security Ed25519 Root of Trust &amp; IV.10 Multi-Game Lifecycle (Trang 20)</a></td><td><a href="#page-16">16–20</a></td></tr>
        <tr><td><b>PHẦN 5</b></td><td><a href="#page-21"><b>Bằng chứng Bảo mật &amp; Kiểm thử:</b> 34/34 Unit Tests, đối chiếu 27→32→31→34, cargo tree, Ed25519 PE #7777, 5 Tamper Cases, Live ADB, SQLite Backup, NTFS ACL &amp; Ma trận 14 Test Cases (TC-01..TC-14)</a></td><td><a href="#page-21">21–25</a></td></tr>
        <tr><td><b>PHẦN 6</b></td><td><a href="#page-26"><b>Minh họa Quy trình Sản xuất Mini Tool 9 Bước:</b> Đối chiếu trực quan trên 2 Game Projects (<code>jx2-mobile</code> &amp; <code>kingdom-slg</code>) &amp; chạy độc lập</a></td><td><a href="#page-26">26–27</a></td></tr>
        <tr><td><b>PHẦN 7</b></td><td><a href="#page-28"><b>Đánh giá UI/UX &amp; Responsive:</b> Hình minh họa 3 độ phân giải (<code>1280×720</code>, <code>1600×900</code>, <code>1920×1080</code>) &amp; Bảng đánh giá 9 tiêu chí</a></td><td><a href="#page-28">28–29</a></td></tr>
        <tr><td><b>PHẦN 8</b></td><td><a href="#page-30"><b>Lộ trình P2–P6:</b> Sơ đồ Roadmap &amp; Đặc tả 7 thuộc tính bắt buộc cho từng giai đoạn P2, P3, P4, P5, P6</a></td><td><a href="#page-30">30–31</a></td></tr>
        <tr><td><b>PHẦN 9</b></td><td><a href="#page-32"><b>Kết luận &amp; Hồ sơ Phê duyệt:</b> Tổng hợp hiện trạng (Mục 1–5), Điều kiện trước P2 (Mục 6) &amp; 7 Quyết định Kiến trúc chờ duyệt (Mục 7)</a></td><td><a href="#page-32">32–33</a></td></tr>
      </table>
    </div>
    <div class="card">
      <div class="card-hdr">DANH MỤC HÌNH ẢNH GIAO DIỆN &amp; SƠ ĐỒ KỸ THUẬT NHÚNG TRONG PDF</div>
      <table class="tbl-compact tbl-dense">
        <tr><th style="width:16%;">Mã Hình</th><th style="width:52%;">Tên Hình Ảnh / Sơ Đồ Minh Họa</th><th style="width:22%;">Trạng Thái</th><th style="width:10%;">Trang</th></tr>
        <tr><td><b>Sơ đồ 1.1</b></td><td>Kiến trúc tổng quan FarmerPro &amp; Quan hệ Studio–Engine–Builder–Runtime</td><td><span class="st-badge st-diagram">DIAGRAM</span></td><td>03</td></tr>
        <tr><td><b>Hình 1.2–1.7</b></td><td>Giao diện FarmerPro Studio v0.1.0 hiện tại (6 màn hình tiêu biểu trong 10 tab cũ)</td><td><span class="st-badge st-actual">ACTUAL</span></td><td>04</td></tr>
        <tr><td><b>Hình 1.8–1.9</b></td><td>Hai Mini Tools độc lập đang chạy: <code>Auto_JX2_Mobile.exe</code> &amp; <code>Auto_Kingdom_SLG.exe</code></td><td><span class="st-badge st-actual">ACTUAL</span></td><td>05</td></tr>
        <tr><td><b>Sơ đồ 2.1</b></td><td>So sánh trực quan Kiến trúc Cũ (10 Tab) vs Kiến trúc Đề xuất (6 Khu vực + Hệ thống)</td><td><span class="st-badge st-diagram">DIAGRAM</span></td><td>06</td></tr>
        <tr><td><b>Ảnh 01</b></td><td>Thiết kế Giao diện Dashboard mới (<code>1. Game Projects / Workspace Hub</code>)</td><td><span class="st-badge st-proposed">PROPOSED</span></td><td>08</td></tr>
        <tr><td><b>Ảnh 02</b></td><td>Thiết kế Giao diện <code>Game Projects</code> (Quản lý Đa game, Assets, Version, Modules)</td><td><span class="st-badge st-proposed">PROPOSED</span></td><td>09</td></tr>
        <tr><td><b>Ảnh 03</b></td><td>Thiết kế Giao diện <code>Workflow Studio</code> (Palette, React Flow, Inspector, Dock)</td><td><span class="st-badge st-proposed">PROPOSED</span></td><td>10</td></tr>
        <tr><td><b>Ảnh 04</b></td><td>Thiết kế Giao diện <code>Tasks &amp; Modules</code> (Task Library, Tham số, Vault, Proxy)</td><td><span class="st-badge st-proposed">PROPOSED</span></td><td>11</td></tr>
        <tr><td><b>Ảnh 05</b></td><td>Thiết kế Giao diện <code>Device Lab</code> (ADB, LDPlayer, Live Screenshot &amp; ROI Cropper)</td><td><span class="st-badge st-proposed">PROPOSED</span></td><td>12</td></tr>
        <tr><td><b>Ảnh 06</b></td><td>Thiết kế Giao diện <code>Test &amp; Debug</code> (Timeline, Node Trace, Ảnh Trước/Sau)</td><td><span class="st-badge st-proposed">PROPOSED</span></td><td>13</td></tr>
        <tr><td><b>Ảnh 07</b></td><td>Thiết kế Giao diện <code>Mini Tool Builder</code> (Layout Editor, Live Preview, Build EXE)</td><td><span class="st-badge st-proposed">PROPOSED</span></td><td>14</td></tr>
        <tr><td><b>Ảnh 08</b></td><td>Thiết kế Giao diện <code>Release Management</code> (Releases, SHA-256, Ed25519, PE #7777)</td><td><span class="st-badge st-proposed">PROPOSED</span></td><td>15</td></tr>
        <tr><td><b>Sơ đồ IV.1–10</b></td><td>Bộ 10 Sơ đồ Kiến trúc Kỹ thuật bắt buộc (Mục IV.1 đến IV.10)</td><td><span class="st-badge st-diagram">DIAGRAM</span></td><td>16–20</td></tr>
        <tr><td><b>Hình 5.1–5.3</b></td><td>Ảnh chụp màn hình LDPlayer thật Trước/Sau thao tác ADB &amp; Mẫu cắt <code>System Apps</code></td><td><span class="st-badge st-actual">ACTUAL</span></td><td>23</td></tr>
        <tr><td><b>Sơ đồ VI.1</b></td><td>Quy trình 9 bước sản xuất Mini Tool đa dự án game trên FarmerPro Studio</td><td><span class="st-badge st-diagram">DIAGRAM</span></td><td>26</td></tr>
        <tr><td><b>Hình 6.1–6.2</b></td><td>Chạy độc lập 2 Mini Tools sau khi đóng hoàn toàn FarmerPro Studio (Bước 9)</td><td><span class="st-badge st-actual">ACTUAL</span></td><td>27</td></tr>
        <tr><td><b>Hình 7.1–7.3</b></td><td>Bố cục Responsive FarmerPro Studio ở <code>1280×720</code>, <code>1600×900</code> và <code>1920×1080</code></td><td><span class="st-badge st-proposed">PROPOSED</span></td><td>28</td></tr>
        <tr><td><b>Sơ đồ VIII.1</b></td><td>Timeline / Roadmap triển khai kỹ thuật giai đoạn P2 → P6</td><td><span class="st-badge st-diagram">DIAGRAM</span></td><td>30</td></tr>
      </table>
    </div>
  </div>
"""))

# ==============================================================================
# PAGE 03: PART 1 (1/3) — PROJECT OVERVIEW & OVERALL ARCHITECTURE DIAGRAM
# ==============================================================================
pages.append(page_wrap(3, "PHẦN 1 — TỔNG QUAN DỰ ÁN (1/3)", "Định Vị Sản Phẩm FarmerPro, Vai Trò Studio IDE / Mini Runtime & Sơ Đồ Kiến Trúc Tổng Quan", f"""
  <div class="grid-3" style="margin-bottom:2mm;">
    <div class="card">
      <div class="card-hdr">1. FARMERPRO LÀ GÌ &amp; VÌ SAO LÀ NỀN TẢNG ĐA AUTO TOOLS?</div>
      <p class="p-xs"><b>FarmerPro</b> là một <b>Nhà máy Sản xuất Phần mềm Tự động hóa Game Android (Multi-Game Auto-Tool Software Factory)</b> chạy trên Windows, được xây dựng trên lõi tự động hóa <b>14 Rust Crates</b> kết hợp giao diện <b>Tauri v2 + React 19</b>.</p>
      <p class="p-xs" style="margin-top:1mm;">Thay vì viết lại mã nguồn cho từng tựa game, FarmerPro cho phép nhà phát triển dùng một Studio duy nhất để tạo nhiều <b>Game Projects</b> (MMORPG, Chiến thuật SLG, Idle RPG...) và đóng gói thành các sản phẩm thương mại độc lập.</p>
    </div>
    <div class="card">
      <div class="card-hdr">2. VAI TRÒ CỦA STUDIO IDE VS MINI RUNTIME</div>
      <p class="p-xs">• <b><code>FarmerPro.exe</code> (FarmerPro Studio IDE):</b> Môi trường dành cho kỹ sư thiết kế quy trình — quản lý đa dự án game, kết nối ADB/LDPlayer chụp cắt mẫu ảnh PNG, vẽ đồ thị React Flow, kiểm thử bước, thiết kế giao diện Mini Tool và ký số phát hành.</p>
      <p class="p-xs" style="margin-top:1mm;">• <b><code>fp-mini-runtime.exe</code> (<code>Auto_&lt;Game&gt;.exe</code>):</b> Tệp thực thi gọn nhẹ giao cho người chơi cuối. Không chứa bộ đóng gói <code>fp-builder</code>, không chứa khóa bí mật, tự xác thực chữ ký Ed25519 qua khóa công khai nhúng trong <b>PE <code>RT_RCDATA #7777</code></b> và dựng giao diện 100% từ <code>bundle/ui-schema.json</code>.</p>
    </div>
    <div class="card">
      <div class="card-hdr">3. NHỮNG THAY ĐỔI QUA P1 → P1.1 → P1.2</div>
      <p class="p-xs">• <b>P1 (PoC):</b> Xây dựng pipeline 8 bước xuất 2 Mini Tools (<code>Auto_JX2_Mobile.exe</code> &amp; <code>Auto_Kingdom_SLG.exe</code>), mô hình 5 trạng thái module, nhúng PE Icon.</p>
      <p class="p-xs">• <b>P1.1:</b> Tách 3 crates (<code>fp-bundle-contract</code>, <code>fp-bundle-verify</code>, <code>fp-builder</code>), nâng cấp chữ ký bất đối xứng <b>RFC 8032 Ed25519</b>, kiểm thử ADB + WinRT OCR thật trên LDPlayer <code>emulator-5554</code>, kiểm chứng NTFS Read-Only fallback &amp; SQLite Online Backup.</p>
      <p class="p-xs">• <b>P1.2:</b> Khôi phục &amp; nâng cấp <b>34/34 Unit Tests PASS</b>, cứng hóa khóa công khai gốc vào <b>PE <code>RT_RCDATA #7777</code></b> (chặn 5/5 kịch bản Tamper) &amp; hoàn tất Blueprint UX Studio mới.</p>
    </div>
  </div>
  {fig_box(img_b64("diag_00_part1_overall_product_architecture.png"), "Sơ đồ 1.1", "Kiến trúc tổng quan FarmerPro & Quan hệ giữa FarmerPro Studio IDE, Shared Rust Workflow Engine, fp-builder và Standalone Mini Runtime", "artifacts/p1_2_evidence/diagrams/00_part1_overall_product_architecture.svg", "DIAGRAM", max_h="102mm")}
"""))

# ==============================================================================
# PAGE 04: PART 1 (2/3) — CURRENT FARMERPRO STUDIO SCREENSHOTS & OLD UI LIMITS
# ==============================================================================
pages.append(page_wrap(4, "PHẦN 1 — TỔNG QUAN DỰ ÁN (2/3)", "Hình Ảnh FarmerPro Studio Hiện Tại (v0.1.0) & Phân Tích Giới Hạn Kiến Trúc Cũ", f"""
  <div class="card" style="margin-bottom:1.8mm;">
    <div class="card-hdr">5. NHỮNG GIỚI HẠN CỐT LÕI CỦA GIAO DIỆN VÀ KIẾN TRÚC FARMERPRO STUDIO CŨ (v0.1.0 — TRƯỚC KHI REFACTOR P4)</div>
    <div class="grid-3">
      <div class="p-xs"><b>① Cấu trúc 10 tab phẳng thiên về vận hành 1 game:</b> Toàn bộ 10 màn hình trên thanh điều hướng trái được bố trí ngang hàng như một bảng điều khiển chạy auto cho riêng <i>Võ Hiệp Truyền Kỳ 2 Mobile</i>, thiếu khái niệm <code>GameProject</code> ở cấp toàn cục.</div>
      <div class="p-xs"><b>② Hardcode dữ liệu &amp; trùng lặp nút điều khiển:</b> Tab <i>Kho Quy trình JX2</i> gắn cứng với <code>jx2Workflows.ts</code>; các nút <i>Mở App / Tắt App / Điều khiển LDPlayer</i> bị lặp lại ở cả Header, Tổng quan và Thiết bị; tab <i>Phiên chạy</i> còn chứa dữ liệu demo <code>job-demo-8492</code> khi chưa chạy thật.</div>
      <div class="p-xs"><b>③ Chưa tích hợp giao diện Mini Tool Builder:</b> Dù bộ đóng gói Rust <code>fp-builder</code> đã hoạt động hoàn chỉnh qua CLI, người dùng Studio chưa có màn hình đồ họa để cắt mẫu ROI lưu thẳng vào Project, cấu hình <code>ui-schema.json</code> và bấm nút xuất <code>Auto_&lt;Game&gt;.exe</code>.</div>
    </div>
  </div>
  <div class="grid-3">
    {fig_box(img_b64("actual_studio_01_tong_quan.png"), "Hình 1.2", "Tab 01: Tổng quan (Vận hành đơn lẻ)", "screenshots/farmerpro_tabs_20261121/01_tong_quan.png", "ACTUAL", max_h="44mm", compact=True)}
    {fig_box(img_b64("actual_studio_03_kho_quy_trinh_jx2.png"), "Hình 1.3", "Tab 03: Kho Quy trình JX2 (Hardcode)", "screenshots/farmerpro_tabs_20261121/03_kho_quy_trinh_jx2.png", "ACTUAL", max_h="44mm", compact=True)}
    {fig_box(img_b64("actual_studio_04_workflow_studio.png"), "Hình 1.4", "Tab 04: Workflow Studio hiện tại", "screenshots/farmerpro_tabs_20261121/04_workflow_studio.png", "ACTUAL", max_h="44mm", compact=True)}
  </div>
  <div class="grid-3" style="margin-top:1.8mm;">
    {fig_box(img_b64("actual_studio_05_bo_nhiem_vu.png"), "Hình 1.5", "Tab 05: Bộ Nhiệm vụ (Tách rời)", "screenshots/farmerpro_tabs_20261121/05_bo_nhiem_vu.png", "ACTUAL", max_h="44mm", compact=True)}
    {fig_box(img_b64("actual_studio_06_phien_chay.png"), "Hình 1.6", "Tab 06: Phiên chạy (Có fallback demo)", "screenshots/farmerpro_tabs_20261121/06_phien_chay.png", "ACTUAL", max_h="44mm", compact=True)}
    {fig_box(img_b64("actual_studio_02_thiet_bi_android.png"), "Hình 1.7", "Tab 02: Thiết bị Android (Chưa có ROI)", "screenshots/farmerpro_tabs_20261121/02_thiet_bi_android.png", "ACTUAL", max_h="44mm", compact=True)}
  </div>
"""))

# ==============================================================================
# PAGE 05: PART 1 (3/3) — TWO EXPORTED STANDALONE MINI TOOLS (ACTUAL)
# ==============================================================================
pages.append(page_wrap(5, "PHẦN 1 — TỔNG QUAN DỰ ÁN (3/3)", "Hình Ảnh Thực Tế Hai Mini Tools Độc Lập Đã Xuất Từ Cùng Bộ Rust Core Crates", f"""
  <div class="grid-2">
    {fig_box(img_b64("actual_mini_tool_jx2_mobile.png"), "Hình 1.8", "Auto_JX2_Mobile.exe (Theme Võ Hiệp Amber #F59E0B, 5 Tasks, 3 cột OCR)", "screenshots/p1_poc/p1_1_auto_jx2_truthful.png", "ACTUAL", max_h="80mm", compact=True)}
    {fig_box(img_b64("actual_mini_tool_kingdom_slg.png"), "Hình 1.9", "Auto_Kingdom_SLG.exe (Theme Chiến Thuật Emerald #10B981, 4 Tasks, 4 cột OCR)", "screenshots/p1_poc/p1_1_auto_slg_truthful.png", "ACTUAL", max_h="80mm", compact=True)}
  </div>
  <div class="card" style="margin-top:2mm;">
    <div class="card-hdr">ĐỐI CHIẾU KỸ THUẬT HAI SẢN PHẨM MINI TOOLS ĐỘC LẬP ĐÃ XUẤT BẢN (CHỨNG MINH KIẾN TRÚC ĐA GAME)</div>
    <table class="tbl-compact tbl-dense">
      <tr><th style="width:22%;">Thuộc tính Sản phẩm</th><th style="width:39%;">Mini Tool #1: <code>Auto_JX2_Mobile.exe</code> (MMORPG)</th><th style="width:39%;">Mini Tool #2: <code>Auto_Kingdom_SLG.exe</code> (Strategy SLG)</th></tr>
      <tr><td><b>Đường dẫn &amp; SHA-256 Binary</b></td><td><code>releases/poc/Auto_JX2_Mobile/Auto_JX2_Mobile.exe</code> (SHA: <code>55c051cba31c269c...</code>)</td><td><code>releases/poc/Auto_Kingdom_SLG/Auto_Kingdom_SLG.exe</code> (SHA: <code>95732232077c0747...</code>)</td></tr>
      <tr><td><b>Khóa công khai PE <code>RT_RCDATA #7777</code></b></td><td><code>acb43ebd9ab499f095fe9d4bc5be6db468a6cb2966a5cc817ad545da473df455</code></td><td><code>acb43ebd9ab499f095fe9d4bc5be6db468a6cb2966a5cc817ad545da473df455</code></td></tr>
      <tr><td><b>Chữ ký số Ed25519 của Bundle</b></td><td><code>7fce5be7be13ac4ea90a25948b8b80c09e068150c94e4d92a19ee9ec7ca1397a...</code> (7 tệp)</td><td><code>07f165fcecf6de7797b268a525ff10e10e272120c7541b2ea22b6f7713d0d96f...</code> (6 tệp)</td></tr>
      <tr><td><b>Cột chỉ số OCR động (Trung thực 100%)</b></td><td><code>Cấp (char_level)</code> · <code>Vàng (char_gold)</code> · <code>Bạc (char_silver)</code> → Hiển thị <code>"Chưa OCR (-)"</code></td><td><code>Thành Chính</code> · <code>Lương Thực</code> · <code>Gỗ</code> · <code>Quân Lực</code> → Hiển thị <code>"Chưa OCR (-)"</code></td></tr>
      <tr><td><b>Danh mục Nhiệm vụ &amp; Modules đóng gói</b></td><td>5 Nhiệm vụ (<code>[NV]</code>, <code>[PB]</code>, <code>[HĐ]</code>) · 5/6 Modules (Bật <code>mod.proxy_network</code>)</td><td>4 Nhiệm vụ (<code>[Nông Nghiệp]</code>, <code>[Quân Sự]</code>, <code>[Liên Minh]</code>) · 4/6 Modules (Tắt Proxy &amp; Port Forward)</td></tr>
    </table>
  </div>
"""))

# ==============================================================================
# PAGE 06: PART 2 (1/2) — BEFORE VS PROPOSED ARCHITECTURE VISUAL COMPARISON
# ==============================================================================
pages.append(page_wrap(6, "PHẦN 2 — SO SÁNH KIẾN TRÚC CŨ VÀ MỚI (1/2)", "So Sánh Trực Quan Kiến Trúc Cũ (Before) Và Kiến Trúc Đề Xuất P1.2 (Proposed Architecture)", f"""
  {fig_box(img_b64("diag_00_part2_before_vs_proposed_architecture.png"), "Sơ đồ 2.1", "So sánh trực quan Kiến trúc Cũ (10 Tab Vận hành Phẳng) và Kiến trúc Đề xuất P1.2 (Context-Driven Multi-Game Studio IDE gồm 6 Khu vực + Hệ thống)", "artifacts/p1_2_evidence/diagrams/00_part2_before_vs_proposed_architecture.svg", "DIAGRAM", max_h="100mm")}
  <div class="grid-2" style="margin-top:2mm;">
    <div class="card">
      <div class="card-hdr">TÓM TẮT 5 NHƯỢC ĐIỂM CỦA KIẾN TRÚC CŨ (BEFORE)</div>
      <ul class="list-xs">
        <li><b>Dashboard tập trung vào vận hành đơn lẻ:</b> Không quản lý được danh mục nhiều dự án game cùng lúc.</li>
        <li><b>Workflow JX2 hardcode:</b> Gắn cứng tên và danh sách quy trình Võ Hiệp 2 vào mã nguồn frontend.</li>
        <li><b>Task Library tách rời:</b> Mảng nhiệm vụ tĩnh không liên kết trực tiếp với <code>workflowId</code> và không xuất ra <code>ui-schema.json</code>.</li>
        <li><b>ADB, Vault và Proxy phân tán:</b> Chiếm 3 tab riêng trên Sidebar nhưng lại lặp lại các nút Mở/Tắt App trên Header và Tổng quan.</li>
        <li><b>Thiếu giao diện sản xuất Mini Tool:</b> Chưa có màn hình Mini Tool Builder &amp; Release Management ngay trong Studio.</li>
      </ul>
    </div>
    <div class="card">
      <div class="card-hdr">TÓM TẮT 6 KHU VỰC SẢN XUẤT CỦA KIẾN TRÚC ĐỀ XUẤT (PROPOSED)</div>
      <ul class="list-xs">
        <li><b>1. Game Projects / Workspace:</b> Quản lý đa dự án game, manifest, thư viện ảnh mẫu PNG, kiểm tra sức khỏe dự án.</li>
        <li><b>2. Workflow Studio:</b> Thiết kế đồ thị React Flow theo Project đang chọn, tích hợp nút cắt mẫu từ Lab và đẩy thẳng sang Task Library.</li>
        <li><b>3. Tasks &amp; Modules:</b> Gom Task Library, phân nhóm động, cấu hình tham số, Account Vault, Proxy và Scheduler vào ngữ cảnh của từng Project.</li>
        <li><b>4. Device Lab &amp; 5. Test &amp; Debug:</b> Quản lý tập trung ADB/LDPlayer, công cụ cắt mẫu ROI, nhật ký StepEvent thật từ SQLite và ảnh đối chiếu Trước/Sau.</li>
        <li><b>6. Mini Tool Builder &amp; Releases:</b> Thiết kế giao diện Mini Tool no-code, xem Live Preview, chạy Pre-Flight 6 điểm và đóng gói <code>.exe</code> 8 bước.</li>
      </ul>
    </div>
  </div>
"""))

# ==============================================================================
# PAGE 07: PART 2 (2/2) — COMPLETE 10-SCREEN TO 6-WORKSPACE MAPPING TABLE
# ==============================================================================
pages.append(page_wrap(7, "PHẦN 2 — SO SÁNH KIẾN TRÚC CŨ VÀ MỚI (2/2)", "Bảng Ánh Xạ Đầy Đủ 10 Màn Hình Hiện Có Sang Cấu Trúc 6 Khu Vực Mới (0% Bỏ Sót Chức Năng)", """
  <div class="card">
    <div class="card-hdr">BẢNG ĐỐI CHIẾU &amp; ÁNH XẠ TOÀN BỘ 10 MÀN HÌNH HIỆN TẠI (apps/desktop/src/features/*) SANG KIẾN TRÚC STUDIO MỚI</div>
    <table class="tbl-compact tbl-dense">
      <tr>
        <th style="width:4%;">STT</th>
        <th style="width:15%;">Màn hình Cũ (v0.1.0)</th>
        <th style="width:20%;">File Nguồn Hiện Tại</th>
        <th style="width:20%;">Khu Vực Mới Đích Đến</th>
        <th style="width:41%;">Chi Tiết Toàn Bộ Chức Năng Được Bảo Toàn 100% &amp; Nâng Cấp</th>
      </tr>
      <tr>
        <td><b>01</b></td>
        <td><b>Tổng quan</b> (<code>overview</code>)</td>
        <td><code>features/overview/OverviewView.tsx</code></td>
        <td><b>1. Game Projects / Workspace</b> (Workspace Dashboard)</td>
        <td>• Nâng cấp thành <b>Workspace Dashboard</b> đa dự án: Hiển thị các Game Projects đang phát triển, workflow gần đây, thiết bị Lab online, kết quả test, bản build/release và cảnh báo.<br/>• Các nút điều khiển phần cứng rời rạc (Mở/Tắt app, bật/tắt LDPlayer) được chuyển hẳn về <code>4. Device Lab</code> để loại bỏ trùng lặp.</td>
      </tr>
      <tr>
        <td><b>02</b></td>
        <td><b>Thiết bị Android</b> (<code>devices</code>)</td>
        <td><code>features/devices/DevicesView.tsx</code></td>
        <td><b>4. Device Lab</b></td>
        <td>• <b>Giữ nguyên 100%:</b> Danh sách ADB USB/TCP, kết nối IP:Port, <b>Quét dải IP LAN (<code>scanRange</code>)</b>, <b>Auto-Discover giả lập (<code>autoDiscover</code>)</b>, điều khiển <b>LDPlayer (<code>ldAction</code>: launch/quit/reboot)</b>, danh sách Packages (<code>launchApp</code>/<code>stopApp</code>), chụp màn hình PNG.<br/>• <b>Bổ sung mới ở P4:</b> Công cụ kéo cắt ảnh mẫu trực quan (<b>ROI Template Cropper</b>) lưu thẳng vào <code>projects/&lt;slug&gt;/templates/</code>.</td>
      </tr>
      <tr>
        <td><b>03</b></td>
        <td><b>Kho Quy trình JX2</b> (<code>workflows</code>)</td>
        <td><code>features/workflows/WorkflowsView.tsx</code></td>
        <td><b>2. Workflow Studio</b> &amp; <b>1. Game Projects</b></td>
        <td>• Bỏ tên cứng <i>"Kho Quy trình JX2"</i>. <b>Bảo toàn 100% toàn bộ 10 quy trình Võ Hiệp 2 Mobile hiện có</b> (<code>jx2Workflows.ts</code>) bằng cách tự động migrate vào dự án mặc định <code>proj-jx2-mobile-001</code>.<br/>• Trở thành <b>Project Workflow Explorer</b> lọc danh sách quy trình theo dự án đang chọn trên Global Header.</td>
      </tr>
      <tr>
        <td><b>04</b></td>
        <td><b>Workflow Studio</b> (<code>studio</code>)</td>
        <td><code>features/studio/WorkflowStudioView.tsx</code></td>
        <td><b>2. Workflow Studio</b> (IDE Core)</td>
        <td>• <b>Giữ nguyên 100%:</b> React Flow Canvas, <code>CustomWorkflowNode</code>, <code>NodePalette</code>, <code>NodeInspector</code>, <code>ExecutionDock</code>, <code>autoLayout</code>, <code>fitView</code>, <code>MiniMap</code>.<br/>• <b>Bổ sung mới ở P4:</b> Nút cầu nối <i>"+ Đưa vào Task Library"</i>, chọn ảnh mẫu trực tiếp từ thư viện Project Templates, đặt Breakpoint/Step Run và 1-Click nhảy từ dòng log lỗi tới Node.</td>
      </tr>
      <tr>
        <td><b>05</b></td>
        <td><b>Bộ Nhiệm vụ</b> (<code>tasks</code>)</td>
        <td><code>features/tasks/TasksView.tsx</code></td>
        <td><b>3. Tasks &amp; Modules</b> (Sub-tab 1: Task Library)</td>
        <td>• <b>Giữ nguyên 100%:</b> Danh mục nhiệm vụ, sắp xếp thứ tự ưu tiên (<code>priority</code>), bật/tắt nhiệm vụ, thời gian ước tính (<code>estimateSec</code>).<br/>• <b>Nâng cấp:</b> Liên kết trực tiếp mỗi Task với <code>workflow_id</code> của Project, cho phép định nghĩa trường tham số động (<code>boolean</code>, <code>number</code>, <code>select</code>) để xuất sang <code>ui-schema.json</code>.</td>
      </tr>
      <tr>
        <td><b>06</b></td>
        <td><b>Phiên chạy</b> (<code>runs</code>)</td>
        <td><code>features/runs/RunsView.tsx</code></td>
        <td><b>5. Test &amp; Debug</b></td>
        <td>• <b>Giữ nguyên 100%:</b> Bảng sự kiện bước (<code>StepEvent</code>), chi tiết JSON payload, lọc theo thiết bị, khôi phục từ Checkpoint (<code>handleRestoreCheckpoint</code>).<br/>• <b>Nâng cấp &amp; Làm sạch:</b> Loại bỏ hoàn toàn mảng sự kiện giả lập <code>job-demo-8492</code>; bổ sung khung đối chiếu ảnh chụp Trước/Sau thao tác ADB thật và điểm chạm Safe Random Tap.</td>
      </tr>
      <tr>
        <td><b>07</b></td>
        <td><b>Tài khoản (Vault)</b> (<code>accounts</code>)</td>
        <td><code>features/accounts/AccountsView.tsx</code></td>
        <td><b>3. Tasks &amp; Modules</b> (Sub-tab 2: Account Vault)</td>
        <td>• <b>Giữ nguyên 100%:</b> Quản lý hồ sơ tài khoản, trạng thái (<code>ready</code>, <code>in_session</code>, <code>cooldown</code>, <code>expired</code>) và tham chiếu khóa bảo mật <code>CRED_MGR://</code> (không lưu mật khẩu dạng rõ).<br/>• <b>Nâng cấp:</b> Phân tách không gian tài khoản (namespace) theo từng <code>project_id</code> tương ứng với module <code>mod.account_vault</code>.</td>
      </tr>
      <tr>
        <td><b>08</b></td>
        <td><b>Mạng &amp; Proxy</b> (<code>network</code>)</td>
        <td><code>features/network/NetworkView.tsx</code></td>
        <td><b>3. Tasks &amp; Modules</b> (Sub-tab 3: Proxy &amp; Network)</td>
        <td>• <b>Giữ nguyên 100%:</b> Công cụ kiểm tra độ trễ SOCKS5/HTTP Proxy, bảng quy tắc chuyển tiếp cổng ADB (<code>adb forward</code> / <code>adb reverse</code>), và <b>Banner cảnh báo kỹ thuật trung thực về giới hạn Android System Proxy / UDP game traffic</b>.<br/>• <b>Nâng cấp:</b> Gắn trực tiếp vào cấu hình của hai module <code>mod.proxy_network</code> và <code>mod.adb_port_forward</code>.</td>
      </tr>
      <tr>
        <td><b>09</b></td>
        <td><b>Design Tokens</b> (<code>gallery</code>)</td>
        <td><code>features/gallery/ComponentGalleryView.tsx</code></td>
        <td><b>⚙️ System Settings</b> (System Tab 4)</td>
        <td>• <b>Giữ nguyên 100%:</b> Bảng tra cứu màu sắc Dark Precision, typography và component rubric từ <code>FARMERPRO_DESIGN_TOKENS.json</code>, chuyển vào ngăn System Settings để nhường không gian cho 6 khu vực sản xuất chính.</td>
      </tr>
      <tr>
        <td><b>10</b></td>
        <td><b>Cài đặt &amp; ADB</b> (<code>settings</code>)</td>
        <td><code>features/settings/SettingsView.tsx</code></td>
        <td><b>⚙️ System Settings</b> (System Tabs 1–3)</td>
        <td>• <b>Giữ nguyên 100%:</b> Chẩn đoán đường dẫn ADB binary, trạng thái cơ sở dữ liệu SQLite WAL, thông tin môi trường.<br/>• <b>Bổ sung mới:</b> Quản lý kho khóa ký Ed25519 của nhà sản xuất (<code>ProducerKeyManager</code>: xem Public Key fingerprint, xoay vòng khóa) và công cụ sao lưu trực tuyến SQLite (<code>Online Backup</code> + <code>PRAGMA integrity_check</code>).</td>
      </tr>
      <tr>
        <td><b>+1</b></td>
        <td><i>(Chưa có ở UI cũ)</i></td>
        <td><i>(Phân hệ mới ở P4)</i></td>
        <td><b>6. Mini Tool Builder &amp; Releases</b></td>
        <td>• <b>Phân hệ mới hoàn toàn:</b> Tích hợp trực tiếp <code>fp-bundle-contract</code> và <code>fp-builder</code> lên giao diện đồ họa — Thiết kế Branding, Layout Editor, cột OCR động, chọn Module 5 trạng thái, xem Live Preview, chạy Pre-Flight Check 6 điểm, đóng gói <code>.exe</code> 8 bước và quản lý lịch sử Release.</td>
      </tr>
    </table>
  </div>
"""))

# ==============================================================================
# PAGES 08 TO 15: PART 3 — 8 PROPOSED UI SCREENS (ẢNH 01 -> ẢNH 08)
# ==============================================================================
UI_SCREENS_SPEC = [
    (8, "PHẦN 3 — THIẾT KẾ GIAO DIỆN MỚI (1/8)", "Ảnh 01 — Dashboard Mới (Workspace Hub Đa Dự Án Game)", "ui_01_new_dashboard.png",
     "Ảnh 01 — Giao diện Dashboard mới (1. Game Projects / Workspace Hub)",
     "artifacts/p1_2_evidence/images/ui_01_new_dashboard.png",
     [
         ("1. Project đang phát triển & Workflow gần đây", "Hiển thị thẻ tóm tắt các dự án game (Võ Hiệp 2 Mobile, Kingdom Rise SLG), số lượng workflows/tasks/templates và danh sách các workflow vừa chỉnh sửa kèm nút mở nhanh."),
         ("2. Thiết bị thử nghiệm & Kết quả Test gần nhất", "Hiển thị trạng thái trực tuyến của thiết bị Lab đang chọn (emulator-5554 · SM_X710N) và tóm tắt phiên chạy kiểm thử thật gần nhất (19 StepEvents, tọa độ Safe Tap, Checkpoint)."),
         ("3. Bản Build/Release, Cảnh báo & Truy cập nhanh", "Liệt kê các gói Auto_<Game>.exe đã ký số Ed25519 + PE #7777, bảng kiểm toán cảnh báo thiếu tài nguyên hoặc lưu ý kỹ thuật (UDP Proxy, Authenticode) và cụm nút chuyển nhanh.")
     ]),
    (9, "PHẦN 3 — THIẾT KẾ GIAO DIỆN MỚI (2/8)", "Ảnh 02 — Game Projects (Quản Lý Đa Dự Án Game, Assets, Version & Modules)", "ui_02_game_projects.png",
     "Ảnh 02 — Giao diện quản lý đa dự án Game Projects (Manifest, Templates PNG, Version & 5-State Modules)",
     "artifacts/p1_2_evidence/images/ui_02_game_projects.png",
     [
         ("1. Danh sách nhiều Game & Tạo/Sửa/Nhân bản Project", "Cột trái cho phép chuyển đổi giữa nhiều tựa game khác nhau, tạo dự án mới hoặc nhân bản cấu trúc dự án hiện có mà không ảnh hưởng dữ liệu của game khác."),
         ("2. Cấu hình Game Package, Version & Biểu tượng PE", "Quản lý định danh slug, Android game_package_id (com.blackhole.jx2mobile), phiên bản phát hành (v1.0.0), độ phân giải mục tiêu và file icon.ico đa lớp."),
         ("3. Thư viện Assets/Templates PNG & Ma trận Modules", "Quản lý tập trung toàn bộ ảnh mẫu Vision PNG kèm kích thước, tọa độ cắt gốc, mã băm SHA-256 và cấu hình bật/tắt 6 module hệ thống theo mô hình 5 trạng thái.")
     ]),
    (10, "PHẦN 3 — THIẾT KẾ GIAO DIỆN MỚI (3/8)", "Ảnh 03 — Workflow Studio (Node Palette, React Flow Canvas, Inspector & Execution Dock)", "ui_03_workflow_studio.png",
     "Ảnh 03 — Giao diện Workflow Studio (Môi trường thiết kế đồ thị tự động hóa & gỡ lỗi trực quan)",
     "artifacts/p1_2_evidence/images/ui_03_workflow_studio.png",
     [
         ("1. Project Workflow Explorer & Collapsible Node Palette", "Lọc danh sách quy trình theo Game Project đang mở; cung cấp 4 nhóm Node kéo thả (Điều khiển luồng, Thị giác/WinRT OCR, Thao tác ADB Safe Tap, Checkpoint). Có thể thu gọn để mở rộng Canvas."),
         ("2. React Flow Canvas, Auto Layout, Fit View & Breakpoint", "Đồ thị trực quan hiển thị rõ trạng thái từng bước, hỗ trợ sắp xếp tự động (Auto Layout DAG), căn khung (Fit View), đặt điểm dừng (Breakpoint) và chạy từng bước (Step Run)."),
         ("3. Node Inspector & Execution/Debug Dock", "Cột phải cấu hình tham số Node (chọn ảnh mẫu từ Project, ngưỡng Confidence 0.90, Safe Margin 18%, Retry) kèm nút cầu nối '+ Đưa vào Task Library' và khung Execution Dock hiển thị Step Trace thời gian thực.")
     ]),
    (11, "PHẦN 3 — THIẾT KẾ GIAO DIỆN MỚI (4/8)", "Ảnh 04 — Tasks & Modules (Task Library Theo Game, Tham Số Động, Vault, Proxy & Scheduler)", "ui_04_tasks_and_modules.png",
     "Ảnh 04 — Giao diện Tasks & Modules (Đóng gói nghiệp vụ người dùng & cấu hình tài nguyên theo Project)",
     "artifacts/p1_2_evidence/images/ui_04_tasks_and_modules.png",
     [
         ("1. Task Library theo Game & Phân nhóm nhiệm vụ động", "Tổ chức danh sách nhiệm vụ hiển thị trên Mini Tool theo các nhãn danh mục tùy biến ([NV], [PB], [HĐ] hoặc [Nông Nghiệp], [Quân Sự]) và sắp xếp thứ tự thực thi ưu tiên (#1 → #5)."),
         ("2. Workflow Mapping & Cấu hình tham số động", "Liên kết mỗi Task với một Workflow đã kiểm chứng và định nghĩa danh sách trường cài đặt (boolean, number, select) để tự động sinh form cấu hình trong Mini Tool."),
         ("3. Tích hợp Module Vault, Proxy, Scheduler & Utilities", "Bảo toàn 100% tính năng quản lý tài khoản CRED_MGR://, kiểm tra SOCKS5/HTTP Proxy + bảng adb forward (kèm ghi chú trung thực về giới hạn UDP) và cấu hình Watchdog trong cùng khu vực.")
     ]),
    (12, "PHẦN 3 — THIẾT KẾ GIAO DIỆN MỚI (5/8)", "Ảnh 05 — Device Lab (ADB Discovery, Giả Lập, Live Screenshot & Công Cụ Cắt Mẫu Vision)", "ui_05_device_lab.png",
     "Ảnh 05 — Giao diện Device Lab (Phòng thử nghiệm thiết bị thật/giả lập & công cụ kéo cắt ảnh mẫu ROI)",
     "artifacts/p1_2_evidence/images/ui_05_device_lab.png",
     [
         ("1. ADB Discovery & Điều khiển LDPlayer/Nox/BlueStacks", "Tập trung toàn bộ thao tác quét thiết bị USB/TCP, quét dải IP LAN, điều khiển khởi động/tắt/khởi động lại giả lập và quản lý đóng/mở ứng dụng game về một nơi duy nhất."),
         ("2. Live On-Demand Screenshot & Công cụ cắt ảnh Vision (ROI Cropper)", "Chụp màn hình PNG không nén từ thiết bị thật (như emulator-5554), cho phép kéo khung chọn vùng ROI (ví dụ biểu tượng System Apps 88×88 tại tọa độ 74, 135) và lưu thẳng vào Project Templates."),
         ("3. Chẩn đoán Thiết bị & Thử nghiệm khớp mẫu SSD / WinRT OCR", "Kiểm tra nhanh điểm số khớp mẫu đa tỉ lệ (score=1.000), điểm chạm Safe Random Tap dự kiến (111, 170) và kết quả đọc chữ WinRT OCR ngay trên ảnh vừa chụp.")
     ]),
    (13, "PHẦN 3 — THIẾT KẾ GIAO DIỆN MỚI (6/8)", "Ảnh 06 — Test & Debug (Danh Sách Phiên, Timeline, Node Trace, Ảnh Bằng Chứng & Error Inspector)", "ui_06_test_and_debug.png",
     "Ảnh 06 — Giao diện Test & Debug (Trung tâm đối chiếu bằng chứng kiểm thử & khôi phục Checkpoint SQLite WAL)",
     "artifacts/p1_2_evidence/images/ui_06_test_and_debug.png",
     [
         ("1. Danh sách phiên kiểm thử, PASS/FAIL & Timeline Node Trace", "Hiển thị dữ liệu thật 100% từ bảng SQLite step_events (loại bỏ hoàn toàn dữ liệu giả lập job-demo-8492), theo dõi chi tiết từng bước command_sent -> device_ack -> verification_passed."),
         ("2. Ảnh chụp màn hình Trước / Sau làm bằng chứng thị giác", "Đặt cạnh nhau ảnh chụp màn hình thiết bị trước và sau thao tác tap thật, đánh dấu rõ khung khớp mẫu Vision [74,135,88,88] và tọa độ điểm chạm Safe Random Tap (111, 170)."),
         ("3. Checkpoint / Retry Recovery & Error Inspector", "Hiển thị trạng thái các điểm lưu Checkpoint trong SQLite WAL (live_adb_verified), cho phép khôi phục chạy tiếp phiên gián đoạn và nhảy trực tiếp từ lỗi tới Node trên Canvas.")
     ]),
    (14, "PHẦN 3 — THIẾT KẾ GIAO DIỆN MỚI (7/8)", "Ảnh 07 — Mini Tool Builder (Chọn Project/Task/Modules, Branding, Layout Editor, Live Preview & Build)", "ui_07_mini_tool_builder.png",
     "Ảnh 07 — Giao diện Mini Tool Builder (Thiết kế sản phẩm Mini Auto-Tool trực quan & xem trước thời gian thực)",
     "artifacts/p1_2_evidence/images/ui_07_mini_tool_builder.png",
     [
         ("1. Cấu hình Branding, Phiên bản & Layout Editor", "Cho phép tùy chỉnh tên file EXE xuất ra, tiêu đề cửa sổ, màu nhấn thương hiệu (#F59E0B hoặc #10B981), biểu tượng PE (.ico), kiểu bố cục (1 cột / 2 cột / mở rộng) và danh sách cột chỉ số OCR động."),
         ("2. Chọn Workflows, Tasks & Modules đóng gói (5-State)", "Tích chọn các quy trình, nhiệm vụ và module được phép đóng gói vào bản phát hành (ví dụ bật Proxy cho JX2 nhưng tắt Proxy cho Kingdom SLG)."),
         ("3. Live Interactive Preview, Pre-Flight Check & Nút Build EXE", "Khung bên phải dựng trực tiếp giao diện Mini Tool thật từ ui-schema.json ngay trong Studio; thanh kiểm tra Pre-Flight 6 điểm xác nhận đủ tài nguyên trước khi kích hoạt pipeline đóng gói 8 bước.")
     ]),
    (15, "PHẦN 3 — THIẾT KẾ GIAO DIỆN MỚI (8/8)", "Ảnh 08 — Release Management (Danh Sách Bản Phát Hành, Dependencies, SHA-256, Ed25519 & Build Report)", "ui_08_release_management.png",
     "Ảnh 08 — Giao diện Release Management (Quản lý lịch sử phát hành, chữ ký số Ed25519, PE #7777 & kiểm toán phụ thuộc)",
     "artifacts/p1_2_evidence/images/ui_08_release_management.png",
     [
         ("1. Danh sách bản phát hành, Version & Build Status", "Quản lý lịch sử các bản dựng Mini Tool theo từng dự án game (rel-jx2-1.0.0, rel-slg-1.0.0) kèm kích thước binary, thời gian đóng gói (~1.1 giây) và đường dẫn tới thư mục EXE / Portable ZIP."),
         ("2. Mã băm SHA-256, Bundle Signature & Khóa gốc PE RT_RCDATA #7777", "Hiển thị đầy đủ SHA-256 của file .exe, chữ ký số RFC 8032 Ed25519 của gói bundle/, định danh khóa nhà sản xuất (fp-ed25519-acb43ebd9ab4) và trạng thái xác thực khóa nhúng trong PE Resource #7777."),
         ("3. Kiểm toán Dependencies & Báo cáo Build 8 Bước", "Xác nhận sự hiện diện của ./bin/adb.exe + 2 DLL, WebView2 Evergreen Runtime, WinRT OCR Engine, quyền ghi SQLite WAL và kết quả chạy Smoke Test / Tamper Suite 5 kịch bản.")
     ]),
]

for pg_no, sec_badge, pg_title, img_file, fig_num_str, src_path_str, notes in UI_SCREENS_SPEC:
    cards_html = "".join(
        f'<div class="card"><div class="card-hdr">{h}</div><p class="p-xs">{p}</p></div>'
        for h, p in notes
    )
    pages.append(page_wrap(pg_no, sec_badge, pg_title, f"""
      {fig_box(img_b64(img_file), fig_num_str, pg_title, src_path_str, "PROPOSED", max_h="114mm")}
      <div class="grid-3" style="margin-top:2mm;">
        {cards_html}
      </div>
    """))

# ==============================================================================
# PAGES 16 TO 20: PART 4 — 10 TECHNICAL ARCHITECTURE DIAGRAMS (2 PER PAGE)
# ==============================================================================
TECH_DIAG_PAGES = [
    (16, "PHẦN 4 — SƠ ĐỒ KIẾN TRÚC KỸ THUẬT (1/5)", "Sơ Đồ IV.1 (Rust Cargo Workspace Dependency Graph) & Sơ Đồ IV.2 (FarmerPro Studio Architecture)",
     ("diag_IV_01_cargo_workspace_dependency_graph.png", "Sơ đồ IV.1", "Rust Cargo Workspace Dependency Graph (14 Crates & 3 Apps — Cô lập hoàn toàn fp-builder khỏi fp-mini-runtime)", "artifacts/p1_2_evidence/diagrams/IV_01_cargo_workspace_dependency_graph.svg"),
     ("diag_IV_02_farmerpro_studio_architecture.png", "Sơ đồ IV.2", "FarmerPro Studio Architecture (React 19 + React Flow + Zustand <-> Tauri v2 Typed IPC <-> Rust Core Services)", "artifacts/p1_2_evidence/diagrams/IV_02_farmerpro_studio_architecture.svg")),
    (17, "PHẦN 4 — SƠ ĐỒ KIẾN TRÚC KỸ THUẬT (2/5)", "Sơ Đồ IV.3 (GameProject Data Model SQLite v2) & Sơ Đồ IV.4 (Workflow Runtime Execution Architecture)",
     ("diag_IV_03_gameproject_data_model.png", "Sơ đồ IV.3", "GameProject Data Model & SQLite WAL Migration v1 -> v2 (6 bảng đa dự án mới + mở rộng khóa ngoại project_id không mất dữ liệu)", "artifacts/p1_2_evidence/diagrams/IV_03_gameproject_data_model.svg"),
     ("diag_IV_04_workflow_runtime_execution_architecture.png", "Sơ đồ IV.4", "Workflow Runtime Execution Architecture (State Machine, Multiscale SSD Match, 18% Safe Random Tap, WinRT OCR & Checkpoints)", "artifacts/p1_2_evidence/diagrams/IV_04_workflow_runtime_execution_architecture.svg")),
    (18, "PHẦN 4 — SƠ ĐỒ KIẾN TRÚC KỸ THUẬT (3/5)", "Sơ Đồ IV.5 (ADB / Emulator Adapter Architecture) & Sơ Đồ IV.6 (Task Scheduler & Multi-Device Execution)",
     ("diag_IV_05_adb_emulator_adapter_architecture.png", "Sơ đồ IV.5", "ADB / Emulator Adapter Architecture (Zero Raw Shell Interpolation, DeviceActor Generation Fencing & Orientation Normalizer)", "artifacts/p1_2_evidence/diagrams/IV_05_adb_emulator_adapter_architecture.svg"),
     ("diag_IV_06_task_scheduler_multidevice_execution.png", "Sơ đồ IV.6", "Task Scheduler & Multi-Device Execution Architecture (Priority Queue, Exclusive Device Leases & Fencing Tokens)", "artifacts/p1_2_evidence/diagrams/IV_06_task_scheduler_multidevice_execution.svg")),
    (19, "PHẦN 4 — SƠ ĐỒ KIẾN TRÚC KỸ THUẬT (4/5)", "Sơ Đồ IV.7 (Mini Tool UI Schema Renderer) & Sơ Đồ IV.8 (Mini Tool Build & Release Pipeline 8 Bước)",
     ("diag_IV_07_mini_tool_ui_schema_renderer.png", "Sơ đồ IV.7", "Mini Tool UI Schema Renderer Architecture (Dựng giao diện động 100% từ bundle/ui-schema.json & kiểm soát Module 5 trạng thái)", "artifacts/p1_2_evidence/diagrams/IV_07_mini_tool_ui_schema_renderer.svg"),
     ("diag_IV_08_mini_tool_build_release_pipeline.png", "Sơ đồ IV.8", "Mini Tool Build & Release Pipeline (8-Step Packager: Validate -> PE Icon & RT_RCDATA #7777 -> Ed25519 Sign -> Verify)", "artifacts/p1_2_evidence/diagrams/IV_08_mini_tool_build_release_pipeline.svg")),
    (20, "PHẦN 4 — SƠ ĐỒ KIẾN TRÚC KỸ THUẬT (5/5)", "Sơ Đồ IV.9 (Security Architecture — Ed25519 Root of Trust) & Sơ Đồ IV.10 (Studio → Mini Tool Production Lifecycle)",
     ("diag_IV_09_security_architecture_ed25519_root_of_trust.png", "Sơ đồ IV.9", "Security Architecture — Ed25519 PE RT_RCDATA #7777 Root of Trust, 5 Kịch bản Chống Giả mạo & Vai trò Windows Authenticode", "artifacts/p1_2_evidence/diagrams/IV_09_security_architecture_ed25519_root_of_trust.svg"),
     ("diag_IV_10_studio_to_mini_tool_production_lifecycle.png", "Sơ đồ IV.10", "Studio -> Mini Tool Production Lifecycle (Cách 1 Studio dùng chung 14 Rust Crates để xuất nhiều Mini Tools không cần sửa Core Runtime)", "artifacts/p1_2_evidence/diagrams/IV_10_studio_to_mini_tool_production_lifecycle.svg")),
]

for pg_no, sec_badge, pg_title, d1, d2 in TECH_DIAG_PAGES:
    pages.append(page_wrap(pg_no, sec_badge, pg_title, f"""
      {fig_box(img_b64(d1[0]), d1[1], d1[2], d1[3], "DIAGRAM", max_h="65mm")}
      <div style="height:2mm;"></div>
      {fig_box(img_b64(d2[0]), d2[1], d2[2], d2[3], "DIAGRAM", max_h="65mm")}
    """))

# ==============================================================================
# PAGE 21: PART 5 (1/5) — UNIT TESTS (34/34), RECONCILIATION 27->32->31->34 & CARGO TREE
# ==============================================================================
pages.append(page_wrap(21, "PHẦN 5 — BẰNG CHỨNG BẢO MẬT & KIỂM THỬ (1/5)", "Bằng Chứng cargo test --workspace (34/34 PASS), Đối Chiếu 27→32→31→34 Tests & Ranh Giới cargo tree", """
  <div class="grid-2">
    <div class="card">
      <div class="card-hdr">1. ĐỐI CHIẾU CHÍNH XÁC SỰ BIẾN ĐỘNG SỐ LƯỢNG UNIT TESTS: 27 → 32 → 31 → 34 PASS</div>
      <p class="p-xs">• <b>Baseline P0-E (27 tests):</b> 11 crates nền tảng ban đầu có tổng cộng <b>27 unit tests</b>.</p>
      <p class="p-xs">• <b>P1-PoC (32 tests):</b> Bổ sung crate <code>fp-builder</code> gồm 5 bài kiểm thử đóng gói và module 5 trạng thái (<code>27 + 5 = 32 tests</code>).</p>
      <p class="p-xs">• <b>P1.1 (31 tests):</b> Khi tách <code>fp-builder</code> thành 3 crates (<code>fp-bundle-contract</code>, <code>fp-bundle-verify</code>, <code>fp-builder</code>), thêm 1 bài test vector chuẩn quốc tế <code>test_rfc8032_section_7_1_official_ed25519_test_vectors</code> (<code>+1</code>), nhưng 2 hàm kiểm thử <code>test_path_traversal_guard_blocks_escape</code> và <code>test_readonly_directory_fallback_to_local_appdata</code> bị sót chưa chuyển sang 2 crate mới (<code>-2</code>) → còn <code>31 tests</code>.</p>
      <p class="p-xs">• <b>P1.2 (34/34 PASS):</b> Đã <b>khôi phục đầy đủ 2 bài test bị sót</b> vào đúng <code>fp-bundle-verify</code> và <code>fp-bundle-contract</code> (<code>+2</code>), đồng thời bổ sung <b>1 bài test bảo mật mới</b> <code>test_pe_embedded_public_key_root_of_trust_blocks_disk_pubkey_replacement</code> (<code>+1</code>) → Đạt <b>34/34 PASS (0 FAILED)</b>.</p>
      <table class="tbl-compact tbl-dense" style="margin-top:1mm;">
        <tr><th style="width:23%;">Crate (14 Crates)</th><th style="width:7%;">Số</th><th style="width:58%;">Danh sách Unit Tests Thực tế (cargo_test_workspace_p1_2.log)</th><th style="width:12%;">Kết quả</th></tr>
        <tr><td><code>fp-actions</code></td><td><b>4</b></td><td><code>test_coordinate_transformation</code>, <code>test_point_rotation_and_landscape_transform</code>, <code>test_safe_margin_shrink</code>, <code>test_sample_safe_point_inside_bounds</code></td><td><span class="c-green">4/4 PASS</span></td></tr>
        <tr><td><code>fp-adb</code></td><td><b>2</b></td><td><code>test_discover_adb_path</code>, <code>test_parse_adb_devices_output</code></td><td><span class="c-green">2/2 PASS</span></td></tr>
        <tr><td><code>fp-builder</code></td><td><b>4</b></td><td><code>test_ed25519_signing_key_rotation_and_tamper_scenarios</code>, <code>test_five_state_module_and_capability_enforcement</code>, <code>test_multi_device_concurrent_isolation_and_fencing</code>, <code>test_pe_embedded_public_key_root_of_trust_blocks_disk_pubkey_replacement</code> <b>[Mới P1.2]</b></td><td><span class="c-green">4/4 PASS</span></td></tr>
        <tr><td><code>fp-bundle-contract</code></td><td><b>1</b></td><td><code>test_readonly_directory_fallback_to_local_appdata</code> <b>[Khôi phục P1.2]</b></td><td><span class="c-green">1/1 PASS</span></td></tr>
        <tr><td><code>fp-bundle-verify</code></td><td><b>2</b></td><td><code>test_path_traversal_guard_blocks_escape</code> <b>[Khôi phục P1.2]</b>, <code>test_rfc8032_section_7_1_official_ed25519_test_vectors</code></td><td><span class="c-green">2/2 PASS</span></td></tr>
        <tr><td><code>fp-device</code></td><td><b>3</b></td><td><code>test_device_actor_hardware_error_resilience</code>, <code>test_device_actor_operations</code>, <code>test_generation_fencing</code></td><td><span class="c-green">3/3 PASS</span></td></tr>
        <tr><td><code>fp-domain</code></td><td><b>2</b></td><td><code>test_bounds_intersection</code>, <code>test_bounds_no_intersection</code></td><td><span class="c-green">2/2 PASS</span></td></tr>
        <tr><td><code>fp-scheduler</code></td><td><b>2</b></td><td><code>test_device_lease_exclusion_and_fencing</code>, <code>test_priority_queue_ordering</code></td><td><span class="c-green">2/2 PASS</span></td></tr>
        <tr><td><code>fp-security</code></td><td><b>2</b></td><td><code>test_destructive_action_guard</code>, <code>test_secret_redaction</code></td><td><span class="c-green">2/2 PASS</span></td></tr>
        <tr><td><code>fp-storage</code></td><td><b>3</b></td><td><code>test_sqlite_file_persistence_across_reopen_and_checkpoint_recovery</code>, <code>test_sqlite_in_memory_migrations_and_crud</code>, <code>test_sqlite_job_and_step_events_persistence</code></td><td><span class="c-green">3/3 PASS</span></td></tr>
        <tr><td><code>fp-testkit</code> &amp; <code>vision</code></td><td><b>4</b></td><td><code>test_fake_driver_flow</code> (1) + <code>test_large_screen_template_match_speed_and_accuracy</code>, <code>test_multiscale_template_match</code>, <code>test_perfect_template_match</code> (3)</td><td><span class="c-green">4/4 PASS</span></td></tr>
        <tr><td><code>fp-workflow-runtime</code> &amp; <code>schema</code></td><td><b>5</b></td><td><code>test_executor_beginner_quest_workflow_with_real_matching_and_verification</code>, <code>test_executor_ocr_and_control_nodes</code>, <code>test_executor_retry_logic</code> (3) + <code>test_missing_start_node</code>, <code>test_parse_sample_workflow_json</code> (2)</td><td><span class="c-green">5/5 PASS</span></td></tr>
      </table>
    </div>
    <div class="card">
      <div class="card-hdr">2. TRÍCH XUẤT RAW LOG THỰC TẾ: cargo test --workspace &amp; cargo tree</div>
      <pre class="code-pre" style="font-size:8.2px;line-height:1.28;">=== Nguồn: artifacts/p1_2_evidence/cargo_test_workspace_p1_2.log ===
running 4 tests (crates/fp-actions)          -&gt; test result: ok. 4 passed; 0 failed
running 2 tests (crates/fp-adb)              -&gt; test result: ok. 2 passed; 0 failed
running 4 tests (crates/fp-builder)
  test tests::test_five_state_module_and_capability_enforcement ... ok
  test tests::test_ed25519_signing_key_rotation_and_tamper_scenarios ... ok
  test tests::test_pe_embedded_public_key_root_of_trust_blocks_disk_pubkey_replacement ... ok
  test tests::test_multi_device_concurrent_isolation_and_fencing ... ok
  test result: ok. 4 passed; 0 failed; finished in 1.19s
running 1 test  (crates/fp-bundle-contract)  -&gt; test_readonly_directory_fallback_to_local_appdata ... ok
running 2 tests (crates/fp-bundle-verify)    -&gt; test_path_traversal_guard_blocks_escape ... ok
                                             -&gt; test_rfc8032_section_7_1_official_ed25519_test_vectors ... ok
running 3 tests (crates/fp-device)           -&gt; test result: ok. 3 passed; 0 failed
running 2 tests (crates/fp-domain)           -&gt; test result: ok. 2 passed; 0 failed
running 2 tests (crates/fp-scheduler)        -&gt; test result: ok. 2 passed; 0 failed
running 2 tests (crates/fp-security)         -&gt; test result: ok. 2 passed; 0 failed
running 3 tests (crates/fp-storage)          -&gt; test result: ok. 3 passed; 0 failed
running 1 test  (crates/fp-testkit)          -&gt; test result: ok. 1 passed; 0 failed
running 3 tests (crates/fp-vision)           -&gt; test result: ok. 3 passed; 0 failed
running 3 tests (crates/fp-workflow-runtime) -&gt; test result: ok. 3 passed; 0 failed
running 2 tests (crates/fp-workflow-schema)  -&gt; test result: ok. 2 passed; 0 failed
--------------------------------------------------------------------------------
TOTAL WORKSPACE UNIT TESTS: 34 PASSED; 0 FAILED; 0 IGNORED.

=== Nguồn: artifacts/p1_1_evidence/02_cargo_tree_mini_runtime.log ===
fp-mini-runtime v0.1.0 (E:\\DATA\\FarmerPro\\apps\\mini-runtime)
├── fp-actions v0.1.0 / fp-adb v0.1.0 / fp-device v0.1.0 / fp-domain v0.1.0
├── fp-bundle-contract v0.1.0   &lt;-- Chỉ chứa cấu trúc dữ liệu (Zero-Crypto)
├── fp-bundle-verify v0.1.0     &lt;-- Chỉ chứa VerifyingKey &amp; đọc PE RT_RCDATA #7777
├── fp-scheduler v0.1.0 / fp-security v0.1.0 / fp-storage v0.1.0 / fp-vision v0.1.0
└── fp-workflow-runtime v0.1.0 / fp-workflow-schema v0.1.0
[XÁC NHẬN RANH GIỚI]: fp-builder KHÔNG xuất hiện trong cây phụ thuộc của fp-mini-runtime!</pre>
    </div>
  </div>
"""))

# ==============================================================================
# PAGE 22: PART 5 (2/5) — ED25519 RFC 8032, PE RT_RCDATA #7777, 5 TAMPER CASES & ROOT OF TRUST DISCLOSURE
# ==============================================================================
pages.append(page_wrap(22, "PHẦN 5 — BẰNG CHỨNG BẢO MẬT & KIỂM THỬ (2/5)", "Bằng Chứng Ed25519 RFC 8032, PE RT_RCDATA #7777, 5 Kịch Bản Tamper Test & Phân Tích Root of Trust", """
  <div class="grid-2">
    <div class="card">
      <div class="card-hdr">1. RAW LOG 5 KỊCH BẢN TẤN CÔNG GIẢ MẠO (tamper_suite_5_cases_pe_root_of_trust.json)</div>
      <pre class="code-pre">{
  "algorithm": "Ed25519-RFC8032-SHA256",
  "allPassed": true,
  "command": "test-tamper-suite",
  "rootOfTrustSource": "PE_RT_RCDATA_7777",
  "cases": [
    {
      "case": "1_modify_bundle_and_recompute_sha256",
      "rejected": true,
      "error": "ED25519_SIGNATURE_INVALID: RFC 8032 Ed25519 signature verification failed"
    },
    {
      "case": "2_replace_disk_trusted_publisher_pub",
      "rejected": true,
      "error": "PE_ROOT_OF_TRUST_MISMATCH: Disk trusted_publisher.pub 'd75a9801...' does not match PE-embedded Root of Trust public key 'acb43ebd...' (RT_RCDATA #7777)"
    },
    {
      "case": "3_replace_signature_bytes",
      "rejected": true,
      "error": "ED25519_SIGNATURE_INVALID: RFC 8032 Ed25519 signature verification failed"
    },
    {
      "case": "4_missing_manifest_or_file",
      "rejected": true,
      "error": "INTEGRITY_MISSING_MANIFEST: manifest.fpbundle.json is missing from bundle"
    },
    {
      "case": "5_rogue_keypair_replace_both_disk_pubkey_and_signature_blocked_by_pe_rcdata_7777",
      "rejected": true,
      "rogueSignatureValidAgainstDiskPubkey": true,
      "error": "ED25519_PUBKEY_UNTRUSTED: Bundle public key 'd75a9801...' does not match trusted public key 'acb43ebd...'"
    }
  ]
}</pre>
      <div class="sub-card" style="margin-top:2mm;">
        <b>Bằng chứng Quét Nhị phân (<code>05_binary_no_secret_key_proof.jsonl</code>):</b> Quét toàn bộ 12,515,840 bytes của <code>Auto_JX2_Mobile.exe</code> và 12,516,352 bytes của <code>Auto_Kingdom_SLG.exe</code> xác nhận <b>0 byte</b> hạt giống khóa bí mật (Secret Seed) tồn tại trong tệp thực thi.
      </div>
    </div>
    <div class="card">
      <div class="card-hdr">2. PHÂN TÍCH CHÍNH XÁC PHẠM VI BẢO VỆ CỦA PE RT_RCDATA #7777 &amp; VAI TRÒ AUTHENTICODE</div>
      <div class="warn-box">
        <b>LƯU Ý KỸ THUẬT BẮT BUỘC VỀ ROOT OF TRUST (KHÔNG TUYÊN BỐ QUÁ PHẠM VI):</b>
        <p class="p-xs" style="margin-top:1.5mm;"><b>① Những gì cơ chế nhúng Public Key vào PE <code>RT_RCDATA #7777</code> BẢO VỆ ĐƯỢC:</b><br/>
        • Chống triệt để việc kẻ tấn công chỉ sửa nội dung các tệp cấu hình/quy trình trong thư mục <code>bundle/</code> hoặc tự sinh một cặp khóa Ed25519 giả mạo mới rồi ghi đè đồng thời cả <code>bundle/trusted_publisher.pub</code> lẫn <code>bundle/signature.fpbundle.json</code> trên đĩa (như <b>Case 2</b> và <b>Case 5</b> ở bên trái), bởi vì <code>Auto_&lt;Game&gt;.exe</code> đọc khóa công khai gốc trực tiếp từ vùng tài nguyên nhị phân <code>.rsrc (RT_RCDATA #7777)</code> của chính tiến trình đang chạy.</p>
        <p class="p-xs" style="margin-top:1.5mm;"><b>② Những gì cơ chế PE <code>RT_RCDATA #7777</code> KHÔNG TỰ BẢO VỆ ĐƯỢC:</b><br/>
        • Việc nhúng khóa công khai vào tài nguyên PE <b>KHÔNG tự bảo vệ trước kịch bản kẻ tấn công thay thế toàn bộ file <code>.exe</code> bằng một file <code>.exe</code> giả mạo khác</b>, hoặc dùng công cụ chỉnh sửa nhị phân (PE Resource Editor / Hex Editor / <code>UpdateResourceW</code>) để vá lại chính vùng nhớ <code>RT_RCDATA #7777</code> bên trong một file <code>.exe</code> chưa được ký số hệ điều hành.</p>
        <p class="p-xs" style="margin-top:1.5mm;"><b>③ Vai trò của Windows Authenticode, Tính toàn vẹn Binary &amp; Cơ chế Tin cậy Nhà phát hành (Giai đoạn P5):</b><br/>
        • Để bảo vệ tính toàn vẹn của chính file thực thi <code>Auto_&lt;Game&gt;.exe</code> (bao gồm cả mã máy <code>.text</code> lẫn tài nguyên <code>.rsrc / RT_RCDATA #7777</code>), ở giai đoạn phát hành thương mại <b>P5</b>, file <code>.exe</code> sau khi hoàn tất Step 5 phải được ký chữ ký số <b>Windows Authenticode (<code>signtool.exe sign /fd SHA256 /tr ...</code>)</b> bằng chứng thư số Code Signing (OV/EV) của nhà phát hành.<br/>
        • Khi đó, chuỗi tin cậy hoàn chỉnh (Defense-in-Depth) sẽ là: <b>Windows OS / SmartScreen &amp; WinVerifyTrust xác thực chữ ký Authenticode của nhà phát hành trên file <code>.exe</code></b> ➔ Đảm bảo vùng <code>PE RT_RCDATA #7777</code> nguyên vẹn 100% ➔ <b>Khóa công khai trong <code>PE RT_RCDATA #7777</code> xác thực chữ ký RFC 8032 Ed25519 của toàn bộ thư mục <code>bundle/</code></b>.</p>
      </div>
    </div>
  </div>
"""))

# ==============================================================================
# PAGE 23: PART 5 (3/5) — LIVE ADB WORKFLOW, BEFORE/AFTER SCREENSHOTS, SQLITE BACKUP & NTFS ACL
# ==============================================================================
pages.append(page_wrap(23, "PHẦN 5 — BẰNG CHỨNG BẢO MẬT & KIỂM THỬ (3/5)", "Bằng Chứng Live ADB Workflow Trên LDPlayer Thật, Ảnh Trước/Sau, SQLite Backup/Restore & NTFS ACL", f"""
  <div class="grid-3">
    {fig_box(img_b64("actual_adb_before_screen.png"), "Hình 5.1", "Ảnh LDPlayer thật TRƯỚC khi Tap (Phát hiện System Apps tại 74, 135)", "artifacts/p1_1_evidence/08_adb_before_screen.png", "ACTUAL", max_h="56mm", compact=True)}
    {fig_box(img_b64("actual_adb_after_screen.png"), "Hình 5.2", "Ảnh LDPlayer thật SAU khi Safe Tap (111, 170) — Hộp thoại System Apps mở", "artifacts/p1_1_evidence/10_adb_after_screen.png", "ACTUAL", max_h="56mm", compact=True)}
    <div class="card">
      <div class="card-hdr">1. LOG THỰC THI LIVE ADB &amp; WINRT OCR</div>
      <div style="display:flex;align-items:center;gap:8px;margin-bottom:4px;">
        <img src="{img_b64('actual_adb_cropped_template.png')}" style="width:40px;height:40px;border:1.5px solid #3BC38D;border-radius:5px;" />
        <div class="p-xs"><b>Hình 5.3:</b> Mẫu cắt thật <code>08b_cropped_template_system_apps.png</code> (88×88 px · ACTUAL)</div>
      </div>
      <pre class="code-pre" style="font-size:8.6px;">Nguồn: 09_live_adb_workflow_execution.json
• Device: emulator-5554 (Samsung SM_X710N, API 34)
• Driver: LIVE_ADB(emulator-5554) | simulated: false
• State: Succeeded | StepEvents: 19 events
• Match: score=1.000 at bbox=[74, 135, 88, 88]
• Safe Random Tap (18% margin): (111.0, 170.0)
• WinRT OCR: "16:42 System Apps ... Võ Hiệp Truyền
  Kỳ 2 Cài đặt Files ... Cửa hàng Play Chrome"
• Checkpoint: "live_adb_verified" saved in WAL
• Cancellation Test (11_live_adb...json): PASS</pre>
    </div>
  </div>
  <div class="grid-2" style="margin-top:2mm;">
    <div class="card">
      <div class="card-hdr">2. BẰNG CHỨNG SQLITE ONLINE BACKUP, RESTORE &amp; MIGRATION ROLLBACK (13_sqlite_backup_restore_migration_proof.json)</div>
      <pre class="code-pre">• Source DB: dist-portable/FarmerPro/data/farmerpro.db (PRAGMA integrity_check = "ok")
• Online Backup DB: backups/p1_1_verified_backup/farmerpro_online_backup.db (integrity = "ok", FK violations = 0)
• Row Counts Preserved (100% Zero Data Loss on Restored Copy):
  devices=2 | workflows=2 | workflow_versions=2 | jobs=2 | step_events=31 -&gt; game_projects=1 (Migration v1 -&gt; v2)
• Faulty V3 Migration Test: Injected intentional SQL error in v3 transaction -&gt; Rolled back cleanly to v2!
• Production Database Untouched: productionDbUntouched = true | passed = true</pre>
    </div>
    <div class="card">
      <div class="card-hdr">3. BẰNG CHỨNG WINDOWS NTFS ACL READ-ONLY FALLBACK (12_windows_acl_readonly_proof.json)</div>
      <pre class="code-pre">• Command: icacls artifacts\\p1_1_evidence\\acl_readonly_test\\Auto_Kingdom_SLG\\data /deny TINH-NGUYEN:(OI)(CI)(W,DC)
• OS Enforcement Proof: Direct Python write blocked by Windows Kernel -&gt; [Errno 13] Permission denied
• Mini Runtime Write Probe: Detected "Access is denied. (os error 5)" on ./data/ directory
• Automatic Fallback Activated: fallbackToLocalAppdataUsed = true
• Resolved Writable SQLite Path: C:\\Users\\TINH-NGUYEN\\AppData\\Local\\FarmerPro\\MiniTools\\kingdom-slg\\data\\runtime_kingdom-slg.db
• Result: Mini Tool started and initialized SQLite WAL database cleanly without crash (passed = true)</pre>
    </div>
  </div>
"""))

# ==============================================================================
# PAGE 24: PART 5 (4/5) — STANDARDIZED TEST EXECUTION MATRIX (TC-01 TO TC-07)
# ==============================================================================
pages.append(page_wrap(24, "PHẦN 5 — BẰNG CHỨNG BẢO MẬT & KIỂM THỬ (4/5)", "Bảng Hồ Sơ Kiểm Thử Chuẩn Hóa — Phần A: Kiểm Thử Workspace, Crate Boundary & Mật Mã Ed25519 (TC-01 → TC-07)", """
  <div class="card">
    <div class="card-hdr">MA TRẬN HỒ SƠ KIỂM THỬ NGHIỆM THU KỸ THUẬT (PHẦN A: TC-01 ĐẾN TC-07 — ĐẦY ĐỦ 7 THUỘC TÍNH BẮT BUỘC)</div>
    <table class="tbl-compact">
      <tr>
        <th style="width:7%;">Test ID</th>
        <th style="width:16%;">Điều kiện (Preconditions)</th>
        <th style="width:18%;">Kết quả Kỳ vọng (Expected)</th>
        <th style="width:20%;">Kết quả Thực tế (Actual)</th>
        <th style="width:7%;">Trạng thái</th>
        <th style="width:16%;">Bằng chứng (Evidence)</th>
        <th style="width:16%;">Giới hạn Chưa Kiểm chứng</th>
      </tr>
      <tr>
        <td><b>TC-01</b></td>
        <td>Chạy <code>cargo test --workspace</code> trên toàn bộ 14 crates trong repository</td>
        <td>Toàn bộ 34 unit tests biên dịch và chạy PASS 100%, không có lỗi hồi quy</td>
        <td><b>34/34 unit tests PASS</b> (0 failed, 0 ignored) hoàn tất trong ~30.9 giây</td>
        <td><span class="st-badge st-actual">PASS</span></td>
        <td><code>artifacts/p1_2_evidence/cargo_test_workspace_p1_2.log</code></td>
        <td>Chưa gồm E2E WebDriver test cho giao diện Studio P4 mới (sẽ thực hiện ở P4/P6)</td>
      </tr>
      <tr>
        <td><b>TC-02</b></td>
        <td>Kiểm tra đồ thị phụ thuộc <code>cargo tree -p fp-mini-runtime</code></td>
        <td><code>fp-mini-runtime</code> hoàn toàn không phụ thuộc <code>fp-builder</code> hay trình tạo khóa</td>
        <td>Cây phụ thuộc chỉ chứa <code>fp-bundle-contract</code> &amp; <code>fp-bundle-verify</code> (0 <code>fp-builder</code>)</td>
        <td><span class="st-badge st-actual">PASS</span></td>
        <td><code>artifacts/p1_1_evidence/02_cargo_tree_mini_runtime.log</code></td>
        <td>Cần duy trì kiểm tra ranh giới này tự động sau mỗi lần sửa <code>Cargo.toml</code> ở P2–P6</td>
      </tr>
      <tr>
        <td><b>TC-03</b></td>
        <td>Kiểm thử Vector chuẩn quốc tế RFC 8032 Section 7.1 cho thuật toán Ed25519</td>
        <td>Hàm xác thực Ed25519 khớp 100% test vectors chuẩn IETF RFC 8032</td>
        <td>Xác thực chính xác vector RFC 8032 &amp; từ chối ngay khi chữ ký bị đảo 1 bit</td>
        <td><span class="st-badge st-actual">PASS</span></td>
        <td><code>artifacts/p1_1_evidence/03_ed25519_unit_tests.log</code></td>
        <td>Chưa kiểm thử hiệu năng ký trên gói tài nguyên lớn &gt;500MB</td>
      </tr>
      <tr>
        <td><b>TC-04</b></td>
        <td>Nhúng khóa công khai Ed25519 vào PE <code>RT_RCDATA #7777</code> ở Step 5 của <code>fp-builder</code></td>
        <td>32-byte PubKey được nhúng vào <code>.rsrc #7777</code> của <code>.exe</code> và đọc lại khớp 100%</td>
        <td>Cả 2 file <code>Auto_JX2_Mobile.exe</code> &amp; <code>Auto_Kingdom_SLG.exe</code> đều chứa khóa <code>acb43ebd...</code> tại <code>#7777</code></td>
        <td><span class="st-badge st-actual">PASS</span></td>
        <td><code>artifacts/p1_2_evidence/export_poc_tools_pe_root_of_trust.json</code></td>
        <td>File <code>.exe</code> hiện chưa ký Windows Authenticode (sẽ tích hợp <code>signtool.exe</code> ở P5)</td>
      </tr>
      <tr>
        <td><b>TC-05</b></td>
        <td>Quét nhị phân toàn bộ 2 file <code>.exe</code> tìm hạt giống khóa bí mật Ed25519 (32-byte Seed)</td>
        <td>Không tồn tại chuỗi byte khóa bí mật trong file thực thi Mini Tool</td>
        <td>Quét ~12.51 MB binary: <code>secretSeedFoundInBinary = false</code> trên cả 2 công cụ</td>
        <td><span class="st-badge st-actual">PASS</span></td>
        <td><code>artifacts/p1_1_evidence/05_binary_no_secret_key_proof.jsonl</code></td>
        <td>Chỉ quét tĩnh trên đĩa; kho khóa ngoài tại <code>%LOCALAPPDATA%</code> cần bảo vệ thêm bằng DPAPI ở P2</td>
      </tr>
      <tr>
        <td><b>TC-06</b></td>
        <td>Chạy bộ 5 kịch bản Tamper Suite trên <code>Auto_JX2_Mobile.exe --test-tamper-suite</code></td>
        <td>Chặn đứng cả 5 trường hợp giả mạo (gồm cả Case 5 thay cặp khóa mới trên đĩa)</td>
        <td><b>5/5 trường hợp bị từ chối khởi chạy</b> (<code>allPassed: true</code>, nguồn <code>PE_RT_RCDATA_7777</code>)</td>
        <td><span class="st-badge st-actual">PASS</span></td>
        <td><code>artifacts/p1_2_evidence/tamper_suite_5_cases_pe_root_of_trust.json</code></td>
        <td>Không chặn được việc kẻ tấn công dùng PE Editor sửa trực tiếp file <code>.exe</code> nếu chưa ký Authenticode</td>
      </tr>
      <tr>
        <td><b>TC-07</b></td>
        <td>Chạy bộ 5 kịch bản Tamper Suite trên <code>Auto_Kingdom_SLG.exe --test-tamper-suite</code></td>
        <td>Chặn đứng cả 5 trường hợp giả mạo trên công cụ game chiến thuật thứ 2</td>
        <td><b>5/5 trường hợp bị từ chối khởi chạy</b> (<code>allPassed: true</code>, nguồn <code>PE_RT_RCDATA_7777</code>)</td>
        <td><span class="st-badge st-actual">PASS</span></td>
        <td><code>artifacts/p1_2_evidence/tamper_suite_kingdom_slg_pe_root_of_trust.json</code></td>
        <td>Tương tự TC-06: Cần kết hợp chữ ký Authenticode ở P5 để bảo vệ toàn vẹn file <code>.exe</code></td>
      </tr>
    </table>
  </div>
"""))

# ==============================================================================
# PAGE 25: PART 5 (5/5) — STANDARDIZED TEST EXECUTION MATRIX (TC-08 TO TC-14)
# ==============================================================================
pages.append(page_wrap(25, "PHẦN 5 — BẰNG CHỨNG BẢO MẬT & KIỂM THỬ (5/5)", "Bảng Hồ Sơ Kiểm Thử Chuẩn Hóa — Phần B: Kiểm Thử Live ADB, NTFS ACL, SQLite Backup & Đa Thiết Bị (TC-08 → TC-14)", """
  <div class="card">
    <div class="card-hdr">MA TRẬN HỒ SƠ KIỂM THỬ NGHIỆM THU KỸ THUẬT (PHẦN B: TC-08 ĐẾN TC-14 — ĐẦY ĐỦ 7 THUỘC TÍNH BẮT BUỘC)</div>
    <table class="tbl-compact">
      <tr>
        <th style="width:7%;">Test ID</th>
        <th style="width:16%;">Điều kiện (Preconditions)</th>
        <th style="width:18%;">Kết quả Kỳ vọng (Expected)</th>
        <th style="width:20%;">Kết quả Thực tế (Actual)</th>
        <th style="width:7%;">Trạng thái</th>
        <th style="width:16%;">Bằng chứng (Evidence)</th>
        <th style="width:16%;">Giới hạn Chưa Kiểm chứng</th>
      </tr>
      <tr>
        <td><b>TC-08</b></td>
        <td>Thực thi luồng ADB + Vision + Safe Tap + WinRT OCR thật trên giả lập LDPlayer đang mở</td>
        <td>Chụp ảnh PNG thật, khớp mẫu SSD, chạm Safe Tap 18%, mở thư mục &amp; đọc OCR</td>
        <td>Khớp <code>System Apps</code> score=1.00 @ (74,135), chạm (111,170), mở hộp thoại &amp; đọc WinRT OCR thành công (19 events)</td>
        <td><span class="st-badge st-actual">PASS</span></td>
        <td><code>artifacts/p1_1_evidence/09_live_adb_workflow_execution.json</code></td>
        <td>Mới kiểm thử trên LDPlayer <code>emulator-5554</code>; chưa thử trên điện thoại vật lý cắm cáp USB</td>
      </tr>
      <tr>
        <td><b>TC-09</b></td>
        <td>Kiểm thử hủy luồng đang chạy (Cancellation / Stop Job) trên kết nối ADB thật</td>
        <td>Dừng luồng ngay lập tức, giải phóng Device Lease và ghi trạng thái <code>Cancelled</code></td>
        <td>Luồng dừng ngay ở bước chờ, trạng thái <code>Cancelled</code>, Lease được thu hồi an toàn</td>
        <td><span class="st-badge st-actual">PASS</span></td>
        <td><code>artifacts/p1_1_evidence/11_live_adb_workflow_cancelled.json</code></td>
        <td>Chưa kiểm thử kịch bản rút cáp ADB vật lý đột ngột giữa lúc đang truyền luồng PNG</td>
      </tr>
      <tr>
        <td><b>TC-10</b></td>
        <td>Khóa quyền ghi thư mục <code>data/</code> bằng Windows NTFS ACL (<code>icacls /deny (OI)(CI)(W,DC)</code>)</td>
        <td>Mini Tool không bị crash, tự động phát hiện Read-Only và chuyển sang <code>%LOCALAPPDATA%</code></td>
        <td>Phát hiện <code>os error 5 (Access is denied)</code> và chuyển DB sang <code>%LOCALAPPDATA%\\FarmerPro\\MiniTools\\...</code></td>
        <td><span class="st-badge st-actual">PASS</span></td>
        <td><code>artifacts/p1_1_evidence/12_windows_acl_readonly_proof.json</code></td>
        <td>Chưa kiểm thử trường hợp ổ đĩa hệ thống <code>C:</code> bị đầy 100% dung lượng trống</td>
      </tr>
      <tr>
        <td><b>TC-11</b></td>
        <td>Sao lưu trực tuyến SQLite WAL, kiểm tra toàn vẹn &amp; chạy thử Migration v1→v2 + Rollback v3 lỗi</td>
        <td>Sao lưu thành công, <code>integrity_check=ok</code>, giữ nguyên 100% số dòng dữ liệu cũ</td>
        <td>Bảo toàn đủ 2 devices, 2 workflows, 2 jobs, 31 step_events; rollback sạch khi giả lập lỗi v3</td>
        <td><span class="st-badge st-actual">PASS</span></td>
        <td><code>artifacts/p1_1_evidence/13_sqlite_backup_restore_migration_proof.json</code></td>
        <td>Migration v2 mới chạy trên bản sao kiểm thử; chưa áp vào DB chính trước khi duyệt P2</td>
      </tr>
      <tr>
        <td><b>TC-12</b></td>
        <td>Kiểm thử cách ly đa thiết bị song song &amp; cơ chế Generation Token Fencing</td>
        <td>2 thiết bị chạy độc lập; mọi lệnh trễ từ lease thế hệ cũ bị chặn đứng</td>
        <td>Unit test <code>test_multi_device_concurrent_isolation_and_fencing</code> &amp; <code>test_generation_fencing</code> PASS</td>
        <td><span class="st-badge st-actual">PASS</span></td>
        <td><code>artifacts/p1_2_evidence/cargo_test_workspace_p1_2.log</code></td>
        <td>Mới kiểm thử đa luồng bằng driver trong bộ test; cần chạy 2+ instance LDPlayer thật ở P6</td>
      </tr>
      <tr>
        <td><b>TC-13</b></td>
        <td>Kiểm thử thực thi Module 5 trạng thái &amp; bộ lọc chống Path Traversal (<code>../</code>)</td>
        <td>Module không đóng gói bị chặn khởi tạo; đường dẫn thoát thư mục bị từ chối</td>
        <td><code>test_five_state_module_and_capability_enforcement</code> &amp; <code>test_path_traversal_guard_blocks_escape</code> PASS</td>
        <td><span class="st-badge st-actual">PASS</span></td>
        <td><code>artifacts/p1_2_evidence/cargo_test_workspace_p1_2.log</code></td>
        <td>Chưa kiểm thử symlink hoặc junction point phức tạp trên ổ đĩa mạng SMB</td>
      </tr>
      <tr>
        <td><b>TC-14</b></td>
        <td>Đóng hoàn toàn FarmerPro Studio và khởi chạy độc lập 2 Mini Tools đã xuất</td>
        <td>Cả 2 Mini Tools tự xác thực PE #7777, dựng đúng giao diện riêng &amp; kết nối ADB</td>
        <td><code>Auto_JX2_Mobile.exe</code> &amp; <code>Auto_Kingdom_SLG.exe</code> khởi chạy độc lập, hiển thị trung thực <code>"Chưa OCR (-)"</code></td>
        <td><span class="st-badge st-actual">PASS</span></td>
        <td><code>artifacts/p1_2_evidence/verify_bundle_pe_root_of_trust.json</code></td>
        <td>Cần kiểm thử thêm trên máy Windows trắng chưa cài sẵn WebView2 Evergreen ở P6</td>
      </tr>
    </table>
  </div>
"""))

# ==============================================================================
# PAGE 26: PART 6 (1/2) — 9-STEP MINI TOOL PRODUCTION LIFECYCLE (STEPS 1 -> 5)
# ==============================================================================
pages.append(page_wrap(26, "PHẦN 6 — QUY TRÌNH SẢN XUẤT MINI TOOL (1/2)", "Sơ Đồ Quy Trình 9 Bước & Minh Họa Chi Tiết Bước 1 → Bước 5 Trên 2 Game Projects Khác Nhau", f"""
  {fig_box(img_b64("diag_VI_01_9step_production_workflow.png"), "Sơ đồ VI.1", "Quy trình khép kín 9 bước sản xuất Mini Auto-Tool cho nhiều tựa game khác nhau trên FarmerPro Studio", "artifacts/p1_2_evidence/diagrams/VI_01_9step_production_workflow.svg", "DIAGRAM", max_h="64mm")}
  <div class="card" style="margin-top:1.8mm;">
    <div class="card-hdr">MINH HỌA TRỰC QUAN BƯỚC 1 → BƯỚC 5 VỚI HAI DỰ ÁN GAME KHÁC NHAU (CHỨNG MINH KIẾN TRÚC ĐA GAME)</div>
    <table class="tbl-compact tbl-dense">
      <tr>
        <th style="width:15%;">Bước Sản Xuất</th>
        <th style="width:15%;">Khu Vực Trên Studio</th>
        <th style="width:35%;">Thực Hiện Cho Game Project #1: <code>jx2-mobile</code> (Võ Hiệp 2 MMORPG)</th>
        <th style="width:35%;">Thực Hiện Cho Game Project #2: <code>kingdom-slg</code> (Kingdom Rise SLG)</th>
      </tr>
      <tr>
        <td><b>Bước 1: Tạo Game Project</b></td>
        <td><code>1. Game Projects</code> (Xem <b>Ảnh 02</b>)</td>
        <td>Khởi tạo <code>proj-jx2-mobile-001</code> · Slug: <code>jx2-mobile</code> · Package: <code>com.blackhole.jx2mobile</code> · Độ phân giải chuẩn: <code>1920×1080</code> · Icon: <code>projects/jx2-mobile/branding/icon.ico</code>.</td>
        <td>Khởi tạo <code>proj-kingdom-slg-002</code> · Slug: <code>kingdom-slg</code> · Package: <code>com.kingdom.rise.slg</code> · Độ phân giải chuẩn: <code>1280×720</code> · Icon: <code>projects/kingdom-slg/branding/icon.ico</code>.</td>
      </tr>
      <tr>
        <td><b>Bước 2: Kết nối Thiết bị Android</b></td>
        <td><code>4. Device Lab</code> (Xem <b>Ảnh 05</b>)</td>
        <td>Kết nối LDPlayer <code>emulator-5554</code>, chụp màn hình PNG theo yêu cầu và dùng <b>ROI Cropper</b> cắt 4 ảnh mẫu (<code>live_ldplayer_system_apps.png</code>, <code>btn_quest_tracker.png</code>...).</td>
        <td>Trên cùng thiết bị Lab hoặc giả lập thứ 2, chụp màn hình và cắt 3 ảnh mẫu đặc thù của game chiến thuật (<code>farm_collect.png</code>, <code>cavalry_train.png</code>, <code>world_map.png</code>).</td>
      </tr>
      <tr>
        <td><b>Bước 3: Thiết kế Workflow</b></td>
        <td><code>2. Workflow Studio</code> (Xem <b>Ảnh 03</b>)</td>
        <td>Thiết kế đồ thị <code>wf-jx2-main-quest</code> &amp; <code>wf-jx2-live-adb-probe</code> (7 nodes: <code>start → capture → find_image(0.90) → random_safe_tap(18%) → wait → ocr → checkpoint</code>).</td>
        <td>Thiết kế đồ thị <code>wf-slg-harvest-and-patrol</code> (5 nodes: <code>start → capture → find_image(0.88) → random_safe_tap(20%) → end</code>) không cần viết bất kỳ dòng code Rust nào.</td>
      </tr>
      <tr>
        <td><b>Bước 4: Tạo Task &amp; Cấu hình Modules</b></td>
        <td><code>3. Tasks &amp; Modules</code> (Xem <b>Ảnh 04</b>)</td>
        <td>Tạo <b>5 Tasks</b> thuộc 3 nhóm <code>[NV]</code>, <code>[PB]</code>, <code>[HĐ]</code> với tham số động (<code>auto_skip_dialog</code>, <code>max_quest_loops</code>). Bật <b>5/6 Modules</b> (gồm cả <code>mod.proxy_network</code>).</td>
        <td>Tạo <b>4 Tasks</b> thuộc 3 nhóm <code>[Nông Nghiệp]</code>, <code>[Quân Sự]</code>, <code>[Liên Minh]</code> với tham số (<code>keep_shield_active</code>, <code>cavalry_tier</code>). Chỉ bật <b>4/6 Modules</b> (Tắt hoàn toàn Proxy).</td>
      </tr>
      <tr>
        <td><b>Bước 5: Chạy Test &amp; Debug</b></td>
        <td><code>5. Test &amp; Debug</code> (Xem <b>Ảnh 06</b>)</td>
        <td>Chạy kiểm thử trực tiếp trên <code>emulator-5554</code>: Ghi nhận 19 StepEvents thật trong SQLite, đối chiếu ảnh Trước/Sau (<b>Hình 5.1 &amp; 5.2</b>), xác nhận tọa độ chạm <code>(111, 170)</code> &amp; OCR PASS.</td>
        <td>Kiểm tra đồ thị <code>wf-slg-harvest-and-patrol</code> vượt qua <code>WorkflowValidator</code> và thử nghiệm khớp mẫu trên các ảnh mẫu tài nguyên SLG đạt trạng thái <code>VALIDATED</code>.</td>
      </tr>
    </table>
  </div>
"""))

# ==============================================================================
# PAGE 27: PART 6 (2/2) — 9-STEP MINI TOOL PRODUCTION LIFECYCLE (STEPS 6 -> 9)
# ==============================================================================
pages.append(page_wrap(27, "PHẦN 6 — QUY TRÌNH SẢN XUẤT MINI TOOL (2/2)", "Minh Họa Chi Tiết Bước 6 → Bước 9 & Bằng Chứng Đóng Studio Chạy Độc Lập 2 Mini Tools", f"""
  <div class="card" style="margin-bottom:2mm;">
    <div class="card-hdr">MINH HỌA TRỰC QUAN BƯỚC 6 → BƯỚC 9 VỚI HAI DỰ ÁN GAME KHÁC NHAU (TỪ THIẾT KẾ UI ĐẾN CHẠY ĐỘC LẬP)</div>
    <table class="tbl-compact tbl-dense">
      <tr>
        <th style="width:15%;">Bước Sản Xuất</th>
        <th style="width:15%;">Khu Vực Trên Studio</th>
        <th style="width:35%;">Thực Hiện Cho Game Project #1: <code>jx2-mobile</code> (Võ Hiệp 2 MMORPG)</th>
        <th style="width:35%;">Thực Hiện Cho Game Project #2: <code>kingdom-slg</code> (Kingdom Rise SLG)</th>
      </tr>
      <tr>
        <td><b>Bước 6: Thiết kế Giao diện Mini Tool</b></td>
        <td><code>6. Mini Tool Builder</code> (Xem <b>Ảnh 07</b>)</td>
        <td>Chọn Layout <code>compact_2_col</code>, màu nhấn <b>Amber <code>#F59E0B</code></b>, cấu hình <b>3 cột chỉ số OCR động</b> (<code>Cấp</code>, <code>Vàng</code>, <code>Bạc</code>) với fallback trung thực <code>"Chưa OCR (-)"</code>. Kiểm tra trên Live Preview.</td>
        <td>Chọn Layout <code>compact_2_col</code>, màu nhấn <b>Emerald <code>#10B981</code></b>, cấu hình <b>4 cột chỉ số OCR động</b> (<code>Thành Chính</code>, <code>Lương Thực</code>, <code>Gỗ</code>, <code>Quân Lực</code>). Kiểm tra trên Live Preview.</td>
      </tr>
      <tr>
        <td><b>Bước 7: Build EXE (Pipeline 8 Bước)</b></td>
        <td><code>6. Mini Tool Builder</code> (<code>fp-builder</code>)</td>
        <td>Bấm <i>Build EXE</i>: Copy <code>fp-mini-runtime.exe</code> → Nhúng 7 lớp Icon Võ Hiệp + nhúng khóa công khai vào <b>PE <code>RT_RCDATA #7777</code></b> → Ký Ed25519 lên 7 tệp trong <code>bundle/</code> (1,097 ms).</td>
        <td>Bấm <i>Build EXE</i>: Copy <code>fp-mini-runtime.exe</code> → Nhúng 7 lớp Icon Chiến Thuật + nhúng khóa công khai vào <b>PE <code>RT_RCDATA #7777</code></b> → Ký Ed25519 lên 6 tệp trong <code>bundle/</code> (1,127 ms).</td>
      </tr>
      <tr>
        <td><b>Bước 8: Kiểm tra &amp; Xuất Release</b></td>
        <td><code>6. Release Management</code> (Xem <b>Ảnh 08</b>)</td>
        <td>Chạy Smoke Test <code>--verify-bundle</code> &amp; <code>--test-tamper-suite</code> (5/5 PASS). Xuất gói hoàn chỉnh tại <code>releases/poc/Auto_JX2_Mobile/Auto_JX2_Mobile.exe</code> (SHA: <code>55c051cb...</code>).</td>
        <td>Chạy Smoke Test <code>--verify-bundle</code> &amp; <code>--test-tamper-suite</code> (5/5 PASS). Xuất gói hoàn chỉnh tại <code>releases/poc/Auto_Kingdom_SLG/Auto_Kingdom_SLG.exe</code> (SHA: <code>95732232...</code>).</td>
      </tr>
      <tr>
        <td><b>Bước 9: Đóng Studio &amp; Chạy Độc lập</b></td>
        <td><b>Môi trường Người dùng cuối (Standalone)</b></td>
        <td>Đóng hoàn toàn <code>FarmerPro.exe</code>. Mở trực tiếp <code>Auto_JX2_Mobile.exe</code>: Tự xác thực chữ ký qua PE #7777, kết nối <code>./bin/adb.exe</code> và ghi vào <code>data/runtime_jx2-mobile.db</code>.</td>
        <td>Đóng hoàn toàn <code>FarmerPro.exe</code>. Mở trực tiếp <code>Auto_Kingdom_SLG.exe</code>: Tự xác thực chữ ký qua PE #7777, ẩn module Proxy và ghi vào <code>data/runtime_kingdom-slg.db</code>.</td>
      </tr>
    </table>
  </div>
  <div class="grid-2">
    {fig_box(img_b64("actual_mini_tool_jx2_mobile.png"), "Hình 6.1 (Bước 9 - Game Project #1)", "Auto_JX2_Mobile.exe chạy độc lập hoàn toàn sau khi đóng FarmerPro Studio", "screenshots/p1_poc/p1_1_auto_jx2_truthful.png", "ACTUAL", max_h="68mm", compact=True)}
    {fig_box(img_b64("actual_mini_tool_kingdom_slg.png"), "Hình 6.2 (Bước 9 - Game Project #2)", "Auto_Kingdom_SLG.exe chạy độc lập hoàn toàn sau khi đóng FarmerPro Studio", "screenshots/p1_poc/p1_1_auto_slg_truthful.png", "ACTUAL", max_h="68mm", compact=True)}
  </div>
"""))

# ==============================================================================
# PAGE 28: PART 7 (1/2) — UI/UX & RESPONSIVE DESIGN AT 3 RESOLUTIONS
# ==============================================================================
pages.append(page_wrap(28, "PHẦN 7 — ĐÁNH GIÁ UI/UX & RESPONSIVE (1/2)", "Hình Ảnh Minh Họa Bố Cục FarmerPro Studio Ở Ba Độ Phân Giải: 1280×720, 1600×900 Và 1920×1080", f"""
  <div class="warn-box" style="margin-bottom:2mm;padding:1.5mm 3mm;">
    <b>GHI CHÚ TRẠNG THÁI BẮT BUỘC (MỤC VII):</b> Ba hình ảnh dưới đây là <b>Bản thiết kế bố cục thích ứng mục tiêu (<code>PROPOSED DESIGN — NOT IMPLEMENTED</code>)</b> phục vụ nghiệm thu kiến trúc giao diện P1.2, chưa phải kết quả kiểm thử ứng dụng sau khi triển khai giai đoạn P4.
  </div>
  <div class="grid-2">
    {fig_box(img_b64("ui_res_1280x720.png"), "Hình 7.1 (1280 × 720 — HD Compact)", "Thu gọn NavRail (60px) & ẩn Palette -> Canvas đạt 920×510 px (>72% diện tích)", "artifacts/p1_2_evidence/images/ui_res_1280x720.png", "PROPOSED", max_h="58mm", compact=True)}
    {fig_box(img_b64("ui_res_1600x900.png"), "Hình 7.2 (1600 × 900 — HD+ Target Standard)", "Độ phân giải mục tiêu chuẩn: Hiển thị đồng thời cả 5 vùng không cuộn ngang", "artifacts/p1_2_evidence/images/ui_res_1600x900.png", "PROPOSED", max_h="58mm", compact=True)}
  </div>
  <div style="margin-top:2mm;">
    {fig_box(img_b64("ui_res_1920x1080.png"), "Hình 7.3 (1920 × 1080 — Full HD Workstation)", "Không gian mở rộng tối đa: React Flow Canvas đạt 1160×800 px hiển thị trọn vẹn đồ thị lớn & Execution Dock 210px", "artifacts/p1_2_evidence/images/ui_res_1920x1080.png", "PROPOSED", max_h="60mm", compact=True)}
  </div>
"""))

# ==============================================================================
# PAGE 29: PART 7 (2/2) — DETAILED 9-CRITERIA UI/UX & RESPONSIVE EVALUATION TABLE
# ==============================================================================
pages.append(page_wrap(29, "PHẦN 7 — ĐÁNH GIÁ UI/UX & RESPONSIVE (2/2)", "Bảng Đánh Giá Chi Tiết 9 Tiêu Chí UI/UX & Responsive Trên Ba Độ Phân Giải Mục Tiêu", """
  <div class="card">
    <div class="card-hdr">ĐÁNH GIÁ KỸ THUẬT 9 TIÊU CHÍ BỐ CỤC, MẬT ĐỘ THÔNG TIN &amp; KHẢ NĂNG THÍCH ỨNG (1280×720 · 1600×900 · 1920×1080)</div>
    <table class="tbl-compact tbl-dense">
      <tr>
        <th style="width:16%;">Tiêu chí Đánh giá</th>
        <th style="width:24%;">Ở Độ phân giải <code>1280 × 720</code> (Compact HD)</th>
        <th style="width:25%;">Ở Độ phân giải <code>1600 × 900</code> (Target HD+)</th>
        <th style="width:25%;">Ở Độ phân giải <code>1920 × 1080</code> (Full HD)</th>
        <th style="width:10%;">Đánh giá</th>
      </tr>
      <tr>
        <td><b>1. Sidebar / Global Header</b></td>
        <td>• Global Header cao cố định <code>44px</code>.<br/>• NavRail tự động thu gọn về <b><code>60px</code></b> (chỉ hiện icon + số thứ tự 1..6 + tooltip khi di chuột).</td>
        <td>• Global Header cao <code>44px</code>.<br/>• NavRail mở rộng đầy đủ <b><code>210px</code></b> hiển thị rõ tên 6 khu vực làm việc và phụ đề chức năng.</td>
        <td>• Global Header cao <code>44px</code>.<br/>• NavRail mở rộng <b><code>220px</code></b> kèm khung trạng thái hệ thống (ADB, SQLite WAL, Key ID) ở đáy.</td>
        <td><span class="st-badge st-proposed">ĐẠT CHUẨN</span></td>
      </tr>
      <tr>
        <td><b>2. React Flow Canvas</b></td>
        <td>• Khi ẩn Node Palette vào drawer nổi, diện tích Canvas đạt <b><code>920 × 510 px</code></b> (hoặc <code>920 × 614 px</code> khi thu gọn Dock), chiếm <b>&gt;72%</b> diện tích cửa sổ.</td>
        <td>• Diện tích Canvas đạt <b><code>860 × 650 px</code></b> khi mở đồng thời mọi panel, và lên tới <b><code>1090 × 794 px</code></b> khi thu gọn Palette &amp; Dock.</td>
        <td>• Diện tích Canvas đạt <b><code>1160 × 800 px</code></b> (mở đủ mọi panel) và lên tới <b><code>1400 × 974 px</code></b> khi thu gọn panel phụ.</td>
        <td><span class="st-badge st-proposed">RỘNG &amp; RÕ</span></td>
      </tr>
      <tr>
        <td><b>3. Node Inspector (Bên phải)</b></td>
        <td>• Rộng cố định <b><code>300px</code></b>, cuộn dọc độc lập bên trong panel; có nút đóng tạm khi chưa chọn Node để nhường chỗ cho Canvas.</td>
        <td>• Rộng <b><code>300px</code></b>, luôn hiển thị đầy đủ tham số Node, hình thu nhỏ ảnh mẫu Vision (88×88) và 2 nút cầu nối.</td>
        <td>• Rộng <b><code>300px–312px</code></b> (đúng thông số <code>inspectorWidth: 312</code> trong <code>FARMERPRO_DESIGN_TOKENS.json</code>).</td>
        <td><span class="st-badge st-proposed">CÂN ĐỐI</span></td>
      </tr>
      <tr>
        <td><b>4. Execution &amp; Debug Dock</b></td>
        <td>• Hỗ trợ 2 nấc chiều cao: <b><code>36px</code></b> (thu gọn chỉ hiện dòng log cuối) và <b><code>140px</code></b> (mở xem bảng Step Trace).</td>
        <td>• Mở rộng mặc định <b><code>180px</code></b> hiển thị rõ 4 tab: Step Trace, Biến Runtime, Ảnh khớp mẫu và Checkpoints.</td>
        <td>• Mở rộng thoải mái <b><code>210px–220px</code></b> (đúng <code>executionDockDefaultHeight: 220</code>) hiển thị 6–8 dòng log cùng lúc.</td>
        <td><span class="st-badge st-proposed">2 NẤC MƯỢT</span></td>
      </tr>
      <tr>
        <td><b>5. Mật độ Thông tin (Information Density)</b></td>
        <td>• Tối đa 2 cột thẻ trên các trang quản lý; ẩn các cột mã băm dài vào tooltip/copy-button để tránh chật chội.</td>
        <td>• Bố cục 3 cột cân đối trên Dashboard và 2 cột Sidebar-Main trên các trang Projects, Tasks, Device Lab, Builder.</td>
        <td>• Hiển thị đầy đủ mã SHA-256, đường dẫn tệp và khung Live Preview kích thước thực <code>740×660</code> trong Mini Tool Builder.</td>
        <td><span class="st-badge st-proposed">DỄ QUÉT</span></td>
      </tr>
      <tr>
        <td><b>6. Khả năng Đọc (Readability &amp; Typography)</b></td>
        <td>• Cỡ chữ nội dung <code>12.5px–13px</code>, bảng <code>11.5px–12px</code>, tiêu đề <code>16px–18px</code>, độ tương phản nền <code>#101722</code> / chữ <code>#EDF3FC</code> đạt chuẩn WCAG AAA.</td>
        <td>• Giữ nguyên kích thước font chuẩn (không thu nhỏ chữ bằng CSS transform scale), đảm bảo chữ tiếng Việt có dấu sắc nét 100%.</td>
        <td>• Khoảng cách dòng <code>19px</code>, font chữ đơn cách <code>Consolas</code> cho toàn bộ ID, tọa độ, SHA-256 và lệnh ADB.</td>
        <td><span class="st-badge st-proposed">SẮC NÉT</span></td>
      </tr>
      <tr>
        <td><b>7. Khả năng Thu gọn Panel (Collapsible Panels)</b></td>
        <td>• Cả 3 panel xung quanh Canvas (<b>NavRail</b>, <b>Node Palette</b>, <b>Execution Dock</b>) và <b>Node Inspector</b> đều có nút đóng/mở độc lập.</td>
        <td>• Người dùng có thể thu gọn từng panel bằng 1 cú nhấp chuột hoặc phím tắt để tập trung vào đồ thị hoặc khung cắt ảnh ROI.</td>
        <td>• Trạng thái đóng/mở của từng panel được lưu tự động vào <code>localStorage</code> theo từng độ phân giải màn hình.</td>
        <td><span class="st-badge st-proposed">4 PANEL</span></td>
      </tr>
      <tr>
        <td><b>8. Không Trùng lặp Thao tác (Zero Duplication)</b></td>
        <td colspan="3">• <b>Loại bỏ hoàn toàn</b> cụm nút <i>Mở App / Tắt App / Quét ADB</i> bị lặp lại trên Global Header và Overview cũ.<br/>• Global Header chỉ chứa <b>2 bộ chọn ngữ cảnh</b> (<code>Active Project ▾</code> &amp; <code>Lab Device ▾</code>) + 1 cụm nút hành động ngữ cảnh của riêng khu vực đang mở.</td>
        <td><span class="st-badge st-proposed">0% TRÙNG</span></td>
      </tr>
      <tr>
        <td><b>9. Không Tràn, Cắt hoặc Chồng Nội dung</b></td>
        <td colspan="3">• Mọi bảng dài và danh sách Node đều nằm trong vùng cuộn dọc cục bộ (<code>overflow-y: auto</code>); tuyệt đối không phát sinh thanh cuộn ngang toàn trang.<br/>• Các nhãn nhiệm vụ dài có thuộc tính <code>text-overflow: ellipsis</code> kèm <code>title</code> tooltip và drawer xem chi tiết đầy đủ.</td>
        <td><span class="st-badge st-proposed">0% OVERFLOW</span></td>
      </tr>
    </table>
  </div>
"""))

# ==============================================================================
# PAGE 30: PART 8 (1/2) — ROADMAP P2-P6 VISUAL TIMELINE & DETAILS FOR P2, P3
# ==============================================================================
pages.append(page_wrap(30, "PHẦN 8 — LỘ TRÌNH P2–P6 (1/2)", "Sơ Đồ Roadmap Trực Quan P2 → P6 & Đặc Tả Chi Tiết Giai Đoạn P2, P3", f"""
  {fig_box(img_b64("diag_VIII_01_p2_to_p6_roadmap_timeline.png"), "Sơ đồ VIII.1", "Lộ trình triển khai kỹ thuật P2 -> P6 theo cơ chế Stage-Gate (Chỉ mở khóa sau khi Báo cáo Nghiệm thu P1.2 được phê duyệt)", "artifacts/p1_2_evidence/diagrams/VIII_01_p2_to_p6_roadmap_timeline.svg", "DIAGRAM", max_h="64mm")}
  <div class="card" style="margin-top:1.8mm;">
    <div class="card-hdr">ĐẶC TẢ KỸ THUẬT CHI TIẾT GIAI ĐOẠN P2 VÀ GIAI ĐOẠN P3 (ĐẦY ĐỦ 7 THUỘC TÍNH BẮT BUỘC)</div>
    <table class="tbl-compact tbl-dense">
      <tr>
        <th style="width:14%;">Thuộc tính</th>
        <th style="width:43%;">Giai đoạn P2 — GameProject, Storage, Migration v2 &amp; Security</th>
        <th style="width:43%;">Giai đoạn P3 — Workflow Engine, Scheduler &amp; Device Lab Integration</th>
      </tr>
      <tr>
        <td><b>1. Phạm vi (Scope)</b></td>
        <td>Nâng cấp tầng lưu trữ SQLite WAL lên kiến trúc đa dự án (Migration v1 → v2), tích hợp cơ chế sao lưu trực tuyến tự động trước khi migrate và quản lý kho khóa Ed25519 qua IPC.</td>
        <td>Mở rộng các dịch vụ thực thi quy trình, công cụ cắt ảnh mẫu ROI trực tiếp từ khung hình ADB, thử nghiệm khớp mẫu SSD đa tỉ lệ trực tiếp và làm sạch dữ liệu sự kiện kiểm thử.</td>
      </tr>
      <tr>
        <td><b>2. Module bị ảnh hưởng</b></td>
        <td><code>crates/fp-storage</code>, <code>crates/fp-domain</code>, <code>crates/fp-security</code>, <code>crates/fp-bundle-contract</code>, <code>apps/desktop/src-tauri</code>.</td>
        <td><code>crates/fp-adb</code>, <code>crates/fp-device</code>, <code>crates/fp-vision</code>, <code>crates/fp-actions</code>, <code>crates/fp-workflow-runtime</code>, <code>crates/fp-scheduler</code>, <code>apps/desktop/src-tauri</code>.</td>
      </tr>
      <tr>
        <td><b>3. Deliverables</b></td>
        <td>• Migration SQL v2 (6 bảng mới: <code>projects</code>, <code>project_modules</code>, <code>vision_templates</code>, <code>task_definitions</code>, <code>mini_tool_schemas</code>, <code>project_releases</code> + cột <code>project_id</code>).<br/>• Tự động chuyển 10 quy trình JX2 hiện có vào <code>proj-jx2-mobile-001</code>.<br/>• Các lệnh IPC: <code>studio_list_projects</code>, <code>studio_create_project</code>, <code>studio_update_project</code>.</td>
        <td>• Lệnh IPC cắt ảnh mẫu: <code>studio_crop_and_save_template</code> (tính SHA-256, lưu PNG vào <code>projects/&lt;slug&gt;/templates/</code> và ghi bảng <code>vision_templates</code>).<br/>• Lệnh IPC thử khớp mẫu: <code>studio_test_template_match</code> (trả về bbox + điểm chạm Safe Tap).<br/>• Loại bỏ hoàn toàn mảng sự kiện giả lập <code>job-demo-8492</code> khỏi backend/frontend.</td>
      </tr>
      <tr>
        <td><b>4. Dependency</b></td>
        <td>Phụ thuộc trực tiếp vào <b>Phê duyệt Báo cáo P1.2 này</b> (đặc biệt là Quyết định QD-01 &amp; QD-02).</td>
        <td>Phụ thuộc vào hoàn thành <b>Giai đoạn P2</b> (cần bảng <code>projects</code> và <code>vision_templates</code> đã sẵn sàng).</td>
      </tr>
      <tr>
        <td><b>5. Acceptance Criteria</b></td>
        <td>• Chạy Online Backup + <code>PRAGMA integrity_check = ok</code> trước khi migrate.<br/>• Không mất bất kỳ dòng dữ liệu nào trong <code>farmerpro.db</code> hiện tại.<br/>• Transaction nguyên tử: nếu lỗi phải rollback 100% về v1.</td>
        <td>• Cắt ảnh mẫu từ thiết bị ADB thật lưu đúng vào thư mục Project và hiển thị ngay trong danh sách Template của Node <code>vision.find_image</code>.<br/>• Mọi phiên chạy hiển thị trong Test &amp; Debug đều truy vấn 100% từ bảng <code>step_events</code> thật.</td>
      </tr>
      <tr>
        <td><b>6. Risk &amp; Kiểm soát</b></td>
        <td>• <i>Rủi ro:</i> Lỗi khóa ngoại khi gán <code>project_id</code> cho dữ liệu cũ.<br/>• <i>Kiểm soát:</i> Tạo bản sao lưu <code>backups/pre_p2_migration.db</code> tự động trước khi chạy DDL.</td>
        <td>• <i>Rủi ro:</i> Tọa độ cắt ảnh ROI trên khung hình thu nhỏ lệch với ảnh gốc <code>1280×720</code>.<br/>• <i>Kiểm soát:</i> Quy đổi tỷ lệ <code>naturalWidth / clientWidth</code> chính xác ở cả frontend và Rust.</td>
      </tr>
      <tr>
        <td><b>7. Test bắt buộc</b></td>
        <td>• Unit tests Migration v1→v2, Rollback test, Foreign-key check và CRUD cho 6 bảng mới.</td>
        <td>• Unit &amp; Integration tests cho ROI Cropper, Multiscale SSD Sandbox, Lease Fencing và Step Trace.</td>
      </tr>
    </table>
  </div>
"""))

# ==============================================================================
# PAGE 31: PART 8 (2/2) — ROADMAP DETAILS FOR P4, P5, P6
# ==============================================================================
pages.append(page_wrap(31, "PHẦN 8 — LỘ TRÌNH P2–P6 (2/2)", "Đặc Tả Chi Tiết Giai Đoạn P4, P5 Và P6 (Đầy Đủ 7 Thuộc Tính Bắt Buộc Cho Từng Giai Đoạn)", """
  <div class="card">
    <div class="card-hdr">ĐẶC TẢ KỸ THUẬT CHI TIẾT GIAI ĐOẠN P4, GIAI ĐOẠN P5 VÀ GIAI ĐOẠN P6</div>
    <table class="tbl-compact">
      <tr>
        <th style="width:13%;">Thuộc tính</th>
        <th style="width:29%;">Giai đoạn P4 — Studio UX Redesign (6 Khu Vực) &amp; Mini Tool Builder UI</th>
        <th style="width:29%;">Giai đoạn P5 — Build/Release Integration, Logo, Packaging &amp; Signing</th>
        <th style="width:29%;">Giai đoạn P6 — End-to-End Multi-Game Acceptance Tests</th>
      </tr>
      <tr>
        <td><b>1. Phạm vi (Scope)</b></td>
        <td>Tái cấu trúc giao diện <code>apps/desktop/src</code> từ 10 tab phẳng sang kiến trúc 6 khu vực sản xuất + Global Context Header + System Settings Drawer theo đúng <code>Ảnh 01</code> → <code>Ảnh 08</code>.</td>
        <td>Nối giao diện <code>6. Mini Tool Builder &amp; Releases</code> với pipeline đóng gói 8 bước của <code>fp-builder</code>, kiểm tra Pre-Flight 6 điểm, nhúng PE Icon + <code>RT_RCDATA #7777</code> và chạy Smoke Test.</td>
        <td>Kiểm thử nghiệm thu toàn trình (End-to-End) quy trình 9 bước sản xuất nhiều Mini Tools cho các tựa game khác nhau hoàn toàn trên giao diện Studio và thiết bị ADB thật.</td>
      </tr>
      <tr>
        <td><b>2. Module bị ảnh hưởng</b></td>
        <td><code>apps/desktop/src/components/*</code>, <code>apps/desktop/src/features/*</code>, <code>apps/desktop/src/store/*</code>.</td>
        <td><code>crates/fp-builder</code>, <code>crates/fp-bundle-verify</code>, <code>apps/mini-runtime</code>, <code>apps/desktop/src/features/builder/*</code>.</td>
        <td>Toàn bộ 14 Rust crates, <code>FarmerPro.exe</code> (Studio IDE) và các gói <code>Auto_&lt;Game&gt;.exe</code> xuất ra.</td>
      </tr>
      <tr>
        <td><b>3. Deliverables</b></td>
        <td>• Global Context Header (<code>Project Switcher ▾</code> + <code>Lab Device ▾</code>).<br/>• 6 Khu vực giao diện hoàn chỉnh + Ngăn System Settings.<br/>• Khung <b>Live Interactive Mini Tool Preview</b> dựng trực tiếp từ <code>MiniToolUiSchema</code>.<br/>• Bố cục responsive chuẩn cho <code>1280×720</code>, <code>1600×900</code>, <code>1920×1080</code>.</td>
        <td>• Các lệnh IPC: <code>studio_preflight_check_release</code>, <code>studio_build_mini_tool_release</code>, <code>studio_smoke_test_release</code>.<br/>• Tính năng đóng gói thư mục phát hành &amp; nén Portable ZIP.<br/>• Tích hợp bước tùy chọn ký số <b>Windows Authenticode (<code>signtool.exe</code>)</b> sau khi nhúng tài nguyên PE Step 5.</td>
        <td>• Bộ kịch bản kiểm thử tự động E2E cho ít nhất 2 Game Projects (<code>jx2-mobile</code> &amp; <code>kingdom-slg</code>).<br/>• Báo cáo nghiệm thu cuối cùng kèm video/screenshots thực thi từ tạo Project trên UI đến chạy Mini Tool độc lập.</td>
      </tr>
      <tr>
        <td><b>4. Dependency</b></td>
        <td>Phụ thuộc vào hoàn thành <b>Giai đoạn P2 &amp; P3</b> (các lệnh IPC backend đã sẵn sàng).</td>
        <td>Phụ thuộc vào hoàn thành <b>Giai đoạn P4</b> (giao diện Mini Tool Builder &amp; Release Management).</td>
        <td>Phụ thuộc vào hoàn thành toàn bộ <b>Giai đoạn P2, P3, P4 và P5</b>.</td>
      </tr>
      <tr>
        <td><b>5. Acceptance Criteria</b></td>
        <td>• Bảo toàn 100% tính năng của 10 màn hình cũ.<br/>• 0% nút điều khiển trùng lặp trên Header.<br/>• Chụp ảnh thực tế ở cả 3 độ phân giải xác nhận không cắt chữ, không tràn ngang, thu gọn mượt cả 4 panel.</td>
        <td>• Người dùng bấm 1 nút trên UI Studio xuất ra được file <code>Auto_&lt;Game&gt;.exe</code> trong ~1.1s.<br/>• Nút Smoke Test trên Studio chạy <code>--verify-bundle</code> &amp; <code>--test-tamper-suite</code> đạt 5/5 PASS.</td>
        <td>• Toàn bộ 9 bước sản xuất thực hiện 100% trên GUI Studio (không sửa JSON tay, không mở Cargo CLI).<br/>• Đóng Studio, chạy độc lập cả 2 Mini Tools trên giả lập LDPlayer thật đạt 100% PASS.</td>
      </tr>
      <tr>
        <td><b>6. Risk &amp; Kiểm soát</b></td>
        <td>• <i>Rủi ro:</i> Hồi quy trạng thái khi chuyển đổi Project đang mở.<br/>• <i>Kiểm soát:</i> Tập trung state vào Zustand store với khóa <code>activeProjectId</code> duy nhất; commit và test sau từng khu vực.</td>
        <td>• <i>Rủi ro:</i> Ký Authenticode làm thay đổi bảng băm nếu thực hiện sai thứ tự.<br/>• <i>Kiểm soát:</i> Bắt buộc nhúng PE Icon &amp; <code>RT_RCDATA #7777</code> ở Step 5 trước, sau đó mới ký Authenticode lên file <code>.exe</code>.</td>
        <td>• <i>Rủi ro:</i> Máy người dùng cuối thiếu WebView2 hoặc quyền ghi thư mục <code>Program Files</code>.<br/>• <i>Kiểm soát:</i> Kiểm chứng lại Dependency Audit và NTFS Read-Only fallback sang <code>%LOCALAPPDATA%</code>.</td>
      </tr>
      <tr>
        <td><b>7. Test bắt buộc</b></td>
        <td>• Kiểm thử UI trên 3 độ phân giải + kiểm thử hồi quy từng khu vực 1..6.</td>
        <td>• Kiểm thử Pre-Flight 6 điểm, PE #7777 readback và Tamper Suite 5 kịch bản từ Studio UI.</td>
        <td>• Full Workspace Unit Tests + Live ADB E2E Acceptance Suite trên 2 Game Projects.</td>
      </tr>
    </table>
  </div>
"""))

# ==============================================================================
# PAGE 32: PART 9 (1/2) — CONCLUSION & TRANSPARENT STATUS SUMMARY (ITEMS 1 -> 5)
# ==============================================================================
pages.append(page_wrap(32, "PHẦN 9 — KẾT LUẬN & HỒ SƠ PHÊ DUYỆT (1/2)", "Tổng Hợp Minh Bạch Hiện Trạng Kỹ Thuật: Đã Triển Khai, Còn Thiếu, Còn Mock/Hardcode & Rủi Ro", """
  <div class="grid-2">
    <div class="card">
      <div class="card-hdr">1. TỔNG HỢP CÁC CHỨC NĂNG ĐÃ TRIỂN KHAI &amp; KIỂM CHỨNG (IMPLEMENTED &amp; TESTED)</div>
      <ul class="list-xs">
        <li><b>Lõi Rust Workspace 14 Crates (<code>34/34 Unit Tests PASS</code>):</b> Đầy đủ <code>fp-domain</code>, <code>fp-adb</code> (không nội suy shell), <code>fp-device</code> (Generation Fencing), <code>fp-vision</code> (Multiscale SSD + WinRT OCR), <code>fp-actions</code> (Safe Random Tap 18%), <code>fp-workflow-schema</code>, <code>fp-workflow-runtime</code>, <code>fp-scheduler</code> (Exclusive Device Leases), <code>fp-storage</code> (SQLite WAL), <code>fp-security</code> (<code>CRED_MGR://</code>).</li>
        <li><b>Dây chuyền Đóng gói &amp; Xác thực Độc lập (<code>fp-builder</code>, <code>fp-bundle-contract</code>, <code>fp-bundle-verify</code>):</b> Pipeline 8 bước xuất 2 Mini Tools độc lập (<code>Auto_JX2_Mobile.exe</code> &amp; <code>Auto_Kingdom_SLG.exe</code>), tách biệt hoàn toàn khỏi <code>fp-builder</code> ở thời điểm biên dịch.</li>
        <li><b>Cứng hóa Nguồn Tin Cậy <code>PE RT_RCDATA #7777</code>:</b> Nhúng trực tiếp khóa công khai Ed25519 32-byte vào tài nguyên PE của <code>.exe</code>, chặn đứng cả 5/5 kịch bản Tamper Suite.</li>
        <li><b>Thực thi ADB &amp; WinRT OCR Thật + NTFS Fallback + SQLite Backup:</b> Đã kiểm chứng vật lý trên giả lập LDPlayer <code>emulator-5554</code> (19 StepEvents), tự động chuyển DB sang <code>%LOCALAPPDATA%</code> khi thư mục cài đặt bị khóa quyền ghi NTFS, và sao lưu trực tuyến SQLite an toàn.</li>
      </ul>

      <div class="card-hdr" style="margin-top:2.5mm;">2. NHỮNG CHỨC NĂNG CÒN THIẾU (MISSING — CHƯA TRIỂN KHAI TRONG CODE)</div>
      <ul class="list-xs">
        <li><b>Chưa áp dụng Migration v2 vào DB chính:</b> Schema đa dự án (6 bảng mới) mới được kiểm chứng trên bản sao <code>restored_migration_test.db</code>, chưa chạy trên <code>farmerpro.db</code> chính thức (chờ P2).</li>
        <li><b>Chưa có 11 lệnh Tauri IPC mới trong <code>apps/desktop/src-tauri</code>:</b> Các lệnh <code>studio_list_projects</code>, <code>studio_crop_and_save_template</code>, <code>studio_build_mini_tool_release</code>... mới được đặc tả ở P1.2, sẽ viết ở P2–P3.</li>
        <li><b>Chưa có màn hình <code>Mini Tool Builder &amp; Releases</code> trên GUI Studio:</b> Hiện tại việc xuất 2 Mini Tools đang gọi qua <code>fp-cli.exe export-poc</code>, chưa có nút bấm trên giao diện <code>FarmerPro.exe</code> (chờ P4–P5).</li>
        <li><b>Chưa tích hợp ký số Windows Authenticode (<code>signtool.exe</code>):</b> File <code>.exe</code> phát hành hiện là Unsigned Portable PE Binary (chờ P5).</li>
      </ul>
    </div>

    <div class="card">
      <div class="card-hdr">3. NHỮNG CHỨC NĂNG CÒN MOCK / HARDCODE TRONG UI STUDIO HIỆN TẠI (v0.1.0)</div>
      <ul class="list-xs">
        <li><b>Danh sách 10 quy trình JX2 tĩnh:</b> Trong <code>apps/desktop/src/features/workflows/jx2Workflows.ts</code> vẫn đang gắn cứng cho Võ Hiệp 2 Mobile, chưa lọc động theo <code>project_id</code> từ SQLite (sẽ chuyển vào bảng <code>workflows</code> của project <code>jx2-mobile</code> ở P2/P4).</li>
        <li><b>Mảng nhiệm vụ tĩnh trong <code>TasksView.tsx</code>:</b> Chưa đọc/ghi trực tiếp từ bảng <code>task_definitions</code> và chưa đồng bộ tự động ra <code>ui-schema.json</code> (sẽ thay thế ở P3/P4).</li>
        <li><b>Dữ liệu sự kiện mẫu <code>job-demo-8492</code> trong <code>RunsView.tsx</code>:</b> Khi cơ sở dữ liệu chưa có phiên chạy nào, giao diện Studio v0.1.0 cũ vẫn hiển thị 1 phiên demo mẫu (trong khi ở Mini Runtime đã loại bỏ 100%; ở Studio sẽ xóa bỏ hoàn toàn ở P3/P4).</li>
      </ul>

      <div class="card-hdr" style="margin-top:2.5mm;">4. NHỮNG THIẾT KẾ ĐANG CHỜ TRIỂN KHAI SAU KHI DUYỆT P1.2 (WAITING FOR P2–P6)</div>
      <ul class="list-xs">
        <li>Toàn bộ <b>8 màn hình giao diện FarmerPro Studio mới (<code>Ảnh 01</code> → <code>Ảnh 08</code>)</b>, thanh <b>Global Context Header</b>, công cụ kéo cắt ảnh <b>ROI Template Cropper</b>, khung <b>Live Mini Tool Preview</b> và bố cục <b>Responsive 3 độ phân giải</b> đang ở trạng thái <code>PROPOSED DESIGN</code> chờ phê duyệt để triển khai ở P2–P5.</li>
      </ul>

      <div class="card-hdr" style="margin-top:2.5mm;">5. CÁC RỦI RO KỸ THUẬT CẦN QUẢN TRỊ Ở P2–P6 (TECHNICAL RISKS)</div>
      <ul class="list-xs">
        <li><b>Rủi ro 1 (Thay thế hoặc vá binary <code>.exe</code> chưa ký Authenticode):</b> Cần tích hợp <code>signtool.exe</code> ở P5 để hệ điều hành bảo vệ toàn vẹn cả file <code>.exe</code> lẫn vùng tài nguyên <code>PE RT_RCDATA #7777</code>.</li>
        <li><b>Rủi ro 2 (Lưu lượng UDP của Game Engine không đi qua SOCKS5 Proxy ứng dụng):</b> Đã và phải tiếp tục hiển thị cảnh báo kỹ thuật trung thực trên cả Studio lẫn Mini Tool rằng lưu lượng UDP cần lớp VPN/tun2socks riêng.</li>
        <li><b>Rủi ro 3 (Phụ thuộc WebView2 Runtime trên máy Windows cũ):</b> Cần giữ cơ chế kiểm toán <code>webview2EvergreenInstalled</code> và hỗ trợ tùy chọn đóng gói Fixed Version WebView2 cho môi trường hoàn toàn ngoại tuyến.</li>
      </ul>
    </div>
  </div>
"""))

# ==============================================================================
# PAGE 33: PART 9 (2/2) — PRE-P2 CONDITIONS, 7 ARCHITECTURAL DECISIONS FOR APPROVAL & DELIVERABLES
# ==============================================================================
pages.append(page_wrap(33, "PHẦN 9 — KẾT LUẬN & HỒ SƠ PHÊ DUYỆT (2/2)", "Điều Kiện Tiên Quyết Trước P2, 7 Quyết Định Kiến Trúc Cần Xác Nhận & Danh Mục Bàn Giao Cuối Cùng", """
  <div class="grid-2">
    <div class="card">
      <div class="card-hdr">6. CÁC ĐIỀU KIỆN BẮT BUỘC CẦN HOÀN THÀNH TRƯỚC KHI BƯỚC SANG P2 (PRE-P2 GATE)</div>
      <table class="tbl-compact tbl-dense">
        <tr><th style="width:12%;">Mã ĐK</th><th style="width:66%;">Nội dung Điều kiện Tiên quyết</th><th style="width:22%;">Trạng thái</th></tr>
        <tr><td><b>ĐK-01</b></td><td>Khôi phục đầy đủ các unit tests bị sót khi tách crate và đạt <b>34/34 PASS</b> trên toàn workspace</td><td><span class="st-badge st-actual">HOÀN THÀNH</span></td></tr>
        <tr><td><b>ĐK-02</b></td><td>Cứng hóa khóa công khai Ed25519 vào <b>PE <code>RT_RCDATA #7777</code></b> và vượt qua <b>5/5 kịch bản Tamper Suite</b></td><td><span class="st-badge st-actual">HOÀN THÀNH</span></td></tr>
        <tr><td><b>ĐK-03</b></td><td>Kiểm chứng quy trình Online Backup + <code>PRAGMA integrity_check</code> + Migration v1→v2 không mất dữ liệu trên bản sao</td><td><span class="st-badge st-actual">HOÀN THÀNH</span></td></tr>
        <tr><td><b>ĐK-04</b></td><td>Giữ nguyên 100% mã nguồn giao diện <code>apps/desktop/src</code> và DB chính <code>farmerpro.db</code> chưa bị sửa đổi trước khi duyệt</td><td><span class="st-badge st-actual">ĐẢM BẢO 100%</span></td></tr>
        <tr><td><b>ĐK-05</b></td><td>Xuất Báo cáo Nghiệm thu PDF trực quan đầy đủ (tài liệu này) và nhận được <b>Phê duyệt chính thức của Người Phê Duyệt</b></td><td><span class="st-badge st-proposed">CHỜ DUYỆT</span></td></tr>
      </table>

      <div class="card-hdr" style="margin-top:1.8mm;">DANH MỤC 7 HẠNG MỤC BÀN GIAO CUỐI CÙNG (MỤC XI)</div>
      <table class="tbl-compact tbl-dense">
        <tr><th style="width:6%;">#</th><th style="width:32%;">Hạng mục Bàn giao</th><th style="width:62%;">Đường dẫn Lưu trữ Thực tế trong Workspace</th></tr>
        <tr><td><b>1</b></td><td>Báo cáo Nghiệm thu PDF (Bản chính + Bản sao)</td><td><code>E:\\DATA\\FarmerPro\\FARMERPRO_P1_2_ARCHITECTURE_UX_ACCEPTANCE_REPORT.pdf</code><br/><code>E:\\DATA\\FarmerPro\\artifacts\\p1_2_evidence\\FARMERPRO_P1_2_ARCHITECTURE_UX_ACCEPTANCE_REPORT.pdf</code></td></tr>
        <tr><td><b>2</b></td><td>Tài liệu Blueprint Kỹ thuật P1.2</td><td><code>E:\\DATA\\FarmerPro\\artifacts\\p1_2_evidence\\FARMERPRO_P1_2_STUDIO_BLUEPRINT.md</code></td></tr>
        <tr><td><b>3</b></td><td>Thư mục Ảnh gốc nhúng trong PDF (40 ảnh PNG)</td><td><code>E:\\DATA\\FarmerPro\\artifacts\\p1_2_evidence\\images\\</code> (15 <code>actual_*</code>, 11 <code>ui_*</code>, 14 <code>diag_*</code>)</td></tr>
        <tr><td><b>4</b></td><td>Thư mục Diagram Source chỉnh sửa được (26 tệp)</td><td><code>E:\\DATA\\FarmerPro\\artifacts\\p1_2_evidence\\diagrams\\</code> (14 tệp Vector <code>.svg</code> + 12 tệp Mermaid <code>.mmd</code>)</td></tr>
        <tr><td><b>5</b></td><td>Thư mục Raw Evidence &amp; Logs thực tế</td><td><code>E:\\DATA\\FarmerPro\\artifacts\\p1_2_evidence\\</code> &amp; <code>E:\\DATA\\FarmerPro\\artifacts\\p1_1_evidence\\</code></td></tr>
        <tr><td><b>6</b></td><td>Báo cáo Kiểm định Chất lượng PDF &amp; SHA-256</td><td><code>E:\\DATA\\FarmerPro\\artifacts\\p1_2_evidence\\pdf_quality_assurance_report.json</code></td></tr>
        <tr><td><b>7</b></td><td>Trạng thái Git &amp; Bảo toàn Dữ liệu</td><td><code>farmerpro.db</code> &amp; <code>apps/desktop/src</code> giữ nguyên 100% (chưa thực thi P2–P6)</td></tr>
      </table>
    </div>

    <div class="card">
      <div class="card-hdr">7. BẢNG 7 QUYẾT ĐỊNH KIẾN TRÚC CẦN NGƯỜI PHÊ DUYỆT XÁC NHẬN TRƯỚC KHI MỞ KHÓA P2–P6</div>
      <table class="tbl-compact tbl-dense">
        <tr><th style="width:11%;">Mã QĐ</th><th style="width:67%;">Nội dung Quyết định Kiến trúc Trình Phê duyệt</th><th style="width:22%;">Xác nhận</th></tr>
        <tr>
          <td><b>QD-01</b></td>
          <td><b>Cấu trúc điều hướng FarmerPro Studio mới:</b> Chuyển từ 10 tab phẳng sang <b>6 Khu vực Sản xuất chính + Global Context Header + Ngăn System Settings</b> (bảo toàn 100% tính năng cũ theo Trang 07).</td>
          <td>☐ Đồng ý<br/>☐ Chỉnh sửa</td>
        </tr>
        <tr>
          <td><b>QD-02</b></td>
          <td><b>Schema SQLite Đa dự án (Migration v1 → v2):</b> Bổ sung 6 bảng mới (<code>projects</code>, <code>project_modules</code>, <code>vision_templates</code>, <code>task_definitions</code>, <code>mini_tool_schemas</code>, <code>project_releases</code>) và tự động gán dữ liệu cũ vào <code>proj-jx2-mobile-001</code>.</td>
          <td>☐ Đồng ý<br/>☐ Chỉnh sửa</td>
        </tr>
        <tr>
          <td><b>QD-03</b></td>
          <td><b>Quy tắc Chống nhân bản thao tác:</b> Xóa cụm nút <i>Mở App / Tắt App</i> khỏi Global Header; gom toàn bộ điều khiển phần cứng ADB/LDPlayer và công cụ cắt ảnh mẫu ROI về duy nhất <code>4. Device Lab</code>.</td>
          <td>☐ Đồng ý<br/>☐ Chỉnh sửa</td>
        </tr>
        <tr>
          <td><b>QD-04</b></td>
          <td><b>Loại bỏ 100% dữ liệu giả lập (Zero Demo Fallback):</b> Xóa mảng sự kiện mẫu <code>job-demo-8492</code> trong Studio; mọi chỉ số chưa qua WinRT OCR thật bắt buộc hiển thị <code>"Chưa OCR (-)"</code>.</td>
          <td>☐ Đồng ý<br/>☐ Chỉnh sửa</td>
        </tr>
        <tr>
          <td><b>QD-05</b></td>
          <td><b>Tích hợp Mini Tool Builder &amp; Live Preview trong Studio:</b> Cấu hình <code>MiniToolUiSchema</code>, chọn 5-State Modules, kiểm tra Pre-Flight 6 điểm và đóng gói <code>Auto_&lt;Game&gt;.exe</code> hoàn toàn trên GUI.</td>
          <td>☐ Đồng ý<br/>☐ Chỉnh sửa</td>
        </tr>
        <tr>
          <td><b>QD-06</b></td>
          <td><b>Bảo mật Ed25519 + PE <code>RT_RCDATA #7777</code> + Authenticode:</b> Duy trì nhúng khóa công khai vào tài nguyên PE <code>#7777</code> ở Step 5 và bổ sung bước tùy chọn ký số Windows Authenticode (<code>signtool.exe</code>) ở P5.</td>
          <td>☐ Đồng ý<br/>☐ Chỉnh sửa</td>
        </tr>
        <tr>
          <td><b>QD-07</b></td>
          <td><b>Lộ trình Triển khai P2 → P6:</b> Mở khóa triển khai tuần tự từ P2 (Storage/Migration) → P3 (Engine/IPC) → P4 (Studio UI) → P5 (Builder/Release) → P6 (E2E Acceptance).</td>
          <td>☐ Mở khóa P2<br/>☐ Tạm dừng</td>
        </tr>
      </table>
      <div class="warn-box" style="margin-top:1.5mm;padding:1.5mm 2.5mm;">
        <b>CAM KẾT KỶ LUẬT STAGE-GATE:</b> Báo cáo này tuyệt đối <b>không tự đánh dấu "Đã nghiệm thu toàn bộ P1.2"</b>. Hệ thống hiện dừng nghiêm ngặt tại cổng P1.2 và chờ quyết định phê duyệt chính thức của Người Phê Duyệt.
      </div>
    </div>
  </div>
"""))

MASTER_HTML = """<!DOCTYPE html>
<html lang="vi">
<head>
<meta charset="UTF-8">
<title>FARMERPRO P1.2 — ARCHITECTURE & STUDIO UX ACCEPTANCE REPORT</title>
<style>
  @page {
    size: A4 landscape;
    margin: 0;
  }
  * {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
  }
  body {
    background: #101722;
    color: #EDF3FC;
    font-family: 'Segoe UI', Inter, Roboto, Arial, sans-serif;
    font-size: 10px;
    line-height: 1.36;
    -webkit-print-color-adjust: exact !important;
    print-color-adjust: exact !important;
  }
  a { color: #38BDF8; text-decoration: none; }
  code {
    font-family: Consolas, 'Courier New', monospace;
    color: #38BDF8;
    font-size: 9.2px;
    word-break: break-all;
  }
  .c-green { color: #3BC38D !important; font-weight: 700; }

  .pdf-page {
    width: 297mm;
    height: 210mm;
    padding: 5.5mm 8.5mm;
    page-break-after: always;
    break-after: page;
    overflow: hidden;
    background: #101722;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
  }
  .page-top-bar {
    height: 8.5mm;
    background: #141C28;
    border: 1px solid #2A3647;
    border-radius: 5px;
    padding: 0 3.5mm;
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-shrink: 0;
  }
  .pt-left { display: flex; align-items: center; gap: 3mm; }
  .pt-badge {
    background: #1878F5;
    color: #FFFFFF;
    font-weight: 800;
    font-size: 9px;
    padding: 0.8mm 2.2mm;
    border-radius: 4px;
    letter-spacing: 0.3px;
    white-space: nowrap;
  }
  .pt-title { font-size: 11.8px; font-weight: 800; color: #FFFFFF; }
  .pt-right { font-size: 9px; font-weight: 600; color: #8FA2BC; font-family: Consolas, monospace; white-space: nowrap; }

  .page-content {
    flex: 1;
    margin: 2mm 0;
    display: flex;
    flex-direction: column;
    justify-content: flex-start;
    overflow: hidden;
  }

  .page-bottom-bar {
    height: 6.8mm;
    background: #121924;
    border: 1px solid #2A3647;
    border-radius: 5px;
    padding: 0 3.5mm;
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 8.8px;
    color: #8FA2BC;
    flex-shrink: 0;
  }
  .pg-num { font-weight: 800; color: #38BDF8; font-size: 9.8px; }

  .cover-hero {
    background: #141E2D;
    border: 1.5px solid #1878F5;
    border-radius: 8px;
    padding: 3.2mm 4.5mm;
    flex-shrink: 0;
  }
  .cover-pill-row { display: flex; gap: 2mm; margin-bottom: 2mm; flex-wrap: wrap; }
  .cover-h1 { font-size: 17.5px; font-weight: 800; color: #FFFFFF; line-height: 1.25; margin-bottom: 1.5mm; }
  .cover-sub { font-size: 10.2px; color: #CAD5E2; line-height: 1.4; }

  .grid-2 { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); gap: 2.8mm; }
  .grid-3 { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr) minmax(0, 1fr); gap: 2.8mm; }

  .card {
    background: #16202F;
    border: 1px solid #2A3647;
    border-radius: 6px;
    padding: 2.3mm 3mm;
    min-width: 0;
  }
  .sub-card {
    background: #121A26;
    border: 1px solid #2A3647;
    border-radius: 5px;
    padding: 1.8mm 2.4mm;
    min-width: 0;
  }
  .warn-box {
    background: #231C12;
    border: 1.5px solid #E7B15B;
    border-radius: 6px;
    padding: 2.2mm 3mm;
    color: #FDE68A;
  }
  .card-hdr {
    font-size: 10px;
    font-weight: 800;
    color: #38BDF8;
    padding-bottom: 1.2mm;
    margin-bottom: 1.5mm;
    border-bottom: 1px solid #2A3647;
    text-transform: uppercase;
    letter-spacing: 0.25px;
  }
  .p-xs { font-size: 9.6px; color: #D4DFEE; line-height: 1.38; }
  .list-xs { padding-left: 3.8mm; font-size: 9.4px; color: #D4DFEE; line-height: 1.38; }
  .list-xs li { margin-bottom: 1mm; }

  .tbl-compact {
    width: 100%;
    table-layout: fixed;
    border-collapse: collapse;
    font-size: 9.2px;
  }
  .tbl-compact th, .tbl-compact td {
    padding: 1.15mm 1.8mm;
    border: 1px solid #263244;
    text-align: left;
    vertical-align: top;
    line-height: 1.32;
    word-wrap: break-word;
    overflow-wrap: break-word;
  }
  .tbl-compact th {
    background: #121924;
    color: #9FB2CC;
    font-weight: 800;
    font-size: 8.8px;
    text-transform: uppercase;
  }
  .tbl-dense th, .tbl-dense td {
    padding: 0.85mm 1.5mm;
    font-size: 8.7px;
    line-height: 1.26;
  }

  .st-badge {
    display: inline-block;
    padding: 0.5mm 1.8mm;
    border-radius: 4px;
    font-size: 8.3px;
    font-weight: 800;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .st-actual { background: #133827; color: #3BC38D; border: 1px solid #3BC38D; }
  .st-proposed { background: #3B2D17; color: #FCD34D; border: 1px solid #E7B15B; }
  .st-diagram { background: #173355; color: #38BDF8; border: 1px solid #1878F5; }

  .fig-container {
    background: #141C28;
    border: 1px solid #2A3647;
    border-radius: 6px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    min-width: 0;
  }
  .fig-img-box {
    background: #0A0F17;
    padding: 1.2mm;
    text-align: center;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .fig-img-box img {
    max-width: 100%;
    width: auto;
    height: auto;
    object-fit: contain;
    display: block;
    border-radius: 4px;
  }
  .fig-caption {
    background: #16202F;
    border-top: 1px solid #2A3647;
    padding: 1mm 2.5mm;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 2mm;
    font-size: 8.8px;
    color: #EDF3FC;
  }
  .fc-left { display: flex; align-items: center; gap: 1.5mm; flex-wrap: wrap; min-width: 0; }
  .fc-src { color: #8FA2BC; font-size: 8.2px; }

  .code-pre {
    background: #0B1018;
    border: 1px solid #263244;
    border-radius: 5px;
    padding: 1.8mm 2.2mm;
    font-family: Consolas, 'Courier New', monospace;
    font-size: 8.8px;
    color: #A5C4E8;
    line-height: 1.34;
    white-space: pre-wrap;
    word-break: break-word;
  }
</style>
</head>
<body>
""" + "\n".join(pages) + """
</body>
</html>
"""

with open(HTML_OUT, "w", encoding="utf-8") as f:
    f.write(MASTER_HTML)

print(f"Wrote master HTML: {HTML_OUT} ({os.path.getsize(HTML_OUT):,} bytes)")

CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"
subprocess.run([
    CHROME,
    "--headless=new",
    "--disable-gpu",
    "--no-pdf-header-footer",
    f"--print-to-pdf={PDF_OUT}",
    f"file:///{HTML_OUT.replace(os.sep, '/')}"
], check=True)

shutil.copy2(PDF_OUT, PDF_COPY)

pdf_size = os.path.getsize(PDF_OUT)
with open(PDF_OUT, "rb") as pf:
    pdf_sha256 = hashlib.sha256(pf.read()).hexdigest()

print(f"Generated PDF: {PDF_OUT}")
print(f"Copied PDF to: {PDF_COPY}")
print(f"File Size    : {pdf_size:,} bytes ({pdf_size / (1024*1024):.2f} MB)")
print(f"SHA-256      : {pdf_sha256}")
