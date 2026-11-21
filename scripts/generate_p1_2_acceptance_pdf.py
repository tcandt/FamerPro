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

TOTAL_PAGES = 43

def img_b64(fname):
    path = os.path.join(IMG_DIR, fname)
    with open(path, "rb") as f:
        return "data:image/png;base64," + base64.b64encode(f.read()).decode("ascii")

def page_wrap(page_no, section_badge, page_title, inner_html):
    return f"""
    <section class="pdf-page" id="page-{page_no:02d}">
      <div class="page-top-bar">
        <div class="pt-left">
          <span class="pt-badge">{section_badge}</span>
          <span class="pt-title">{page_title}</span>
        </div>
        <div class="pt-right">
          <span>FARMERPRO P1.2 · v1.2.1-REVIEW · 2026-11-21</span>
        </div>
      </div>
      <div class="page-content">
        {inner_html}
      </div>
      <div class="page-bottom-bar">
        <span>Hồ sơ nghiệm thu Kiến trúc & Giao diện FarmerPro Studio P1.2 (v1.2.1-REVIEW — Giữ nguyên mã nguồn UI & production DB trước khi duyệt)</span>
        <span>Evidence Pack: artifacts/p1_2_v1_2_1_evidence_pack/</span>
        <span class="pg-num">Trang {page_no:02d} / {TOTAL_PAGES:02d}</span>
      </div>
    </section>
    """

def fig_box(b64_src, fig_num, title, source_path, status_type, max_h="126mm", compact=False):
    if status_type == "ACTUAL":
        badge_cls = "st-actual"
        badge_txt = "ACTUAL IMPLEMENTATION" if compact else "ACTUAL IMPLEMENTATION (Ảnh chụp ứng dụng / thiết bị đang chạy thực tế)"
    elif status_type == "PROPOSED":
        badge_cls = "st-proposed"
        badge_txt = "PROPOSED DESIGN — NOT IMPLEMENTED" if compact else "PROPOSED DESIGN — NOT IMPLEMENTED (Thiết kế P1.2 chờ duyệt cho P4)"
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
# PAGE 01: COVER PAGE (v1.2.1-REVIEW)
# ==============================================================================
pages.append(page_wrap(1, "HỒ SƠ NGHIỆM THU P1.2", "Trang Bìa & Thông Tin Hồ Sơ Nghiệm Thu Kiến Trúc Sản Phẩm (v1.2.1-REVIEW)", """
  <div class="cover-hero" style="padding:2.8mm 4.5mm;">
    <div class="cover-pill-row">
      <span class="st-badge st-proposed">STAGE-GATE P1.2 (v1.2.1-REVIEW): TRÌNH DUYỆT LẠI SAU RÀ SOÁT</span>
      <span class="st-badge st-actual">GIỮ NGUYÊN 100% CODE UI (apps/desktop/src) &amp; PRODUCTION DB</span>
      <span class="st-badge st-diagram">34/34 UNIT TESTS · PE RT_RCDATA #7777 · EVIDENCE MANIFEST SHA-256</span>
    </div>
    <h1 class="cover-h1">FARMERPRO P1.2 — BÁO CÁO NGHIỆM THU KIẾN TRÚC SẢN PHẨM &amp; THIẾT KẾ STUDIO IDE (v1.2.1-REVIEW)</h1>
    <div class="cover-sub">
      Hồ sơ thiết kế chuyển đổi <b>FarmerPro.exe</b> thành <b>Nền tảng Sản xuất Đa Auto-Tools (Multi-Game Mini Auto-Tool Software Factory)</b>, kèm trực quan hóa giao diện Studio mới, đặc tả 6 trạng thái tương tác Workflow Studio, trình dựng WYSIWYG Mini Tool Builder không cần sửa JSON tay, 10 sơ đồ kiến trúc kỹ thuật (1 sơ đồ/trang), ma trận phân loại thực tế 5 trạng thái và gói bằng chứng đối chiếu SHA-256.
    </div>
  </div>

  <div class="grid-2" style="margin-top:2.5mm;">
    <div class="card">
      <div class="card-hdr">1. THÔNG TIN ĐỊNH DANH HỒ SƠ NGHIỆM THU (v1.2.1-REVIEW)</div>
      <table class="tbl-compact tbl-dense">
        <tr><th style="width:32%;">Thuộc tính Hồ sơ</th><th style="width:68%;">Giá trị Thực tế &amp; Trạng thái</th></tr>
        <tr><td><b>Tệp PDF bàn giao chính</b></td><td><code>FARMERPRO_P1_2_ARCHITECTURE_UX_ACCEPTANCE_REPORT.pdf</code></td></tr>
        <tr><td><b>Gói Evidence di động</b></td><td><code>artifacts/p1_2_v1_2_1_evidence_pack/</code> (kèm <code>EVIDENCE_MANIFEST_SHA256.json</code>)</td></tr>
        <tr><td><b>Blueprint đối chiếu</b></td><td><code>artifacts/p1_2_evidence/FARMERPRO_P1_2_STUDIO_BLUEPRINT.md</code></td></tr>
        <tr><td><b>Phiên bản &amp; Ngày lập</b></td><td><b>v1.2.1-REVIEW</b> (Nâng cấp từ v1.2.0-P1.2-GATE) · Ngày lập: <b>2026-11-21</b></td></tr>
        <tr><td><b>Quy chuẩn trình bày</b></td><td>A4 Ngang (297×210mm) · 43 Trang · PDF Bookmarks đầy đủ · Cỡ chữ đọc chuẩn 100–125%</td></tr>
        <tr><td><b>Trạng thái Phê duyệt</b></td><td><span class="st-badge st-proposed">CHƯA PHÊ DUYỆT — TRÌNH DUYỆT BẢN HIỆU CHỈNH v1.2.1-REVIEW</span></td></tr>
      </table>
    </div>
    <div class="card">
      <div class="card-hdr">2. TÓM TẮT BẰNG CHỨNG KỸ THUẬT &amp; PHẠM VI KIỂM CHỨNG</div>
      <table class="tbl-compact tbl-dense">
        <tr><th style="width:34%;">Hạng mục Kỹ thuật</th><th style="width:66%;">Hiện trạng Thực tế (Ghi nhận từ Workspace &amp; Raw Logs)</th></tr>
        <tr><td><b>Unit Tests Workspace</b></td><td><b>34/34 PASS (0 FAILED)</b> trên 14 crates (Vừa chạy lại trên <code>rustc 1.96.0</code>)</td></tr>
        <tr><td><b>Cô lập Builder/Runtime</b></td><td><code>cargo tree -p fp-mini-runtime</code> xác nhận <b>0 phụ thuộc</b> vào <code>fp-builder</code> hay khóa bí mật</td></tr>
        <tr><td><b>Ed25519 PE Root of Trust</b></td><td>Nhúng PubKey 32-byte vào <b>PE <code>RT_RCDATA #7777</code></b>; chặn <b>5/5 Tamper Cases</b> (Authenticode chờ P5)</td></tr>
        <tr><td><b>Live ADB &amp; WinRT OCR</b></td><td>Chạy qua CLI trên <b>LDPlayer <code>emulator-5554</code></b>: 19 events, Safe Tap (111,170), OCR PASS</td></tr>
        <tr><td><b>2 Mini Tools PoC (CLI)</b></td><td><code>Auto_JX2_Mobile.exe</code> &amp; <code>Auto_Kingdom_SLG.exe</code> chạy độc lập khi đóng Studio (chờ Builder GUI ở P4–P5)</td></tr>
        <tr><td><b>Bảo toàn UI vs Backend</b></td><td>Phân loại minh bạch theo 5 mức: <code>IMPLEMENTED</code>, <code>PARTIAL</code>, <code>MOCK</code>, <code>MISSING</code>, <code>PLANNED</code></td></tr>
      </table>
    </div>
  </div>

  <div class="card" style="margin-top:2.5mm;">
    <div class="card-hdr">3. NGUYÊN TẮC PHÂN ĐỊNH TRẠNG THÁI TRUNG THỰC TRONG TOÀN BỘ BÁO CÁO</div>
    <div class="grid-3">
      <div class="sub-card">
        <span class="st-badge st-actual">ACTUAL IMPLEMENTATION</span>
        <p class="p-xs" style="margin-top:1.5mm;">Áp dụng cho ảnh chụp từ <code>FarmerPro.exe</code> v0.1.0 đang chạy, 2 Mini Tools xuất qua CLI (<code>Auto_JX2_Mobile.exe</code>, <code>Auto_Kingdom_SLG.exe</code>) và ảnh chụp thao tác ADB thật trên LDPlayer <code>emulator-5554</code>.</p>
      </div>
      <div class="sub-card">
        <span class="st-badge st-proposed">PROPOSED DESIGN — NOT IMPLEMENTED</span>
        <p class="p-xs" style="margin-top:1.5mm;">Áp dụng cho 10 màn hình giao diện Studio mới (<code>Ảnh 01</code> → <code>Ảnh 08</code> gồm <code>03B</code> &amp; <code>07B</code>) và 3 bố cục Responsive. Đây là bản thiết kế chờ phê duyệt để triển khai ở giai đoạn P4.</p>
      </div>
      <div class="sub-card">
        <span class="st-badge st-diagram">5 TRẠNG THÁI BACKEND MINH BẠCH</span>
        <p class="p-xs" style="margin-top:1.5mm;">Không dùng nhãn <i>"bảo toàn 100% tính năng"</i>. Tách riêng <b>Thiết kế/Cấu trúc UI</b> khỏi <b>Trạng thái Backend</b> (<code>IMPLEMENTED / PARTIAL / MOCK / MISSING / PLANNED</code>), chỉ rõ phần còn mock.</p>
      </div>
    </div>
  </div>
"""))

# ==============================================================================
# PAGE 02: TABLE OF CONTENTS & CHANGELOG v1.2.0 -> v1.2.1-REVIEW
# ==============================================================================
pages.append(page_wrap(2, "MỤC LỤC & CHANGELOG", "Mục Lục Báo Cáo (43 Trang + PDF Bookmarks) & Tóm Tắt Thay Đổi Từ v1.2.0 → v1.2.1-REVIEW", """
  <div class="grid-2">
    <div class="card">
      <div class="card-hdr">MỤC LỤC TỔNG THỂ HỒ SƠ NGHIỆM THU (CLICK ĐỂ NHẢY TRANG · CÓ PDF BOOKMARKS)</div>
      <table class="tbl-compact">
        <tr><th style="width:16%;">Phần / Mục</th><th style="width:72%;">Nội dung Chi tiết</th><th style="width:12%;">Trang</th></tr>
        <tr><td><b>Mở đầu</b></td><td><a href="#page-01">Trang bìa v1.2.1-REVIEW, Mục lục, Changelog &amp; Danh mục Hình ảnh / Evidence Pack</a></td><td><a href="#page-01">01–03</a></td></tr>
        <tr><td><b>PHẦN 1</b></td><td><a href="#page-04"><b>Tổng quan Dự án:</b> Sơ đồ 1.1, Quan hệ Studio–Engine–Builder–Runtime (Sơ đồ 1.2), UI cũ &amp; 2 Mini Tools PoC</a></td><td><a href="#page-04">04–07</a></td></tr>
        <tr><td><b>PHẦN 2</b></td><td><a href="#page-08"><b>So sánh Kiến trúc &amp; Ma trận Phân loại Thực tế:</b> Sơ đồ 2.1 + Bảng 10 màn hình &amp; 6 lõi backend (5 Trạng thái)</a></td><td><a href="#page-08">08–10</a></td></tr>
        <tr><td><b>PHẦN 3</b></td><td><a href="#page-11"><b>Thiết kế Giao diện Studio Mới (10 Màn hình cỡ lớn):</b> Dashboard, Projects, <b>Workflow Studio (03A &amp; 03B)</b>, Tasks, Device Lab, Test &amp; Debug, <b>Mini Tool Builder WYSIWYG (07A &amp; 07B)</b>, Releases</a></td><td><a href="#page-11">11–20</a></td></tr>
        <tr><td><b>PHẦN 4</b></td><td><a href="#page-21"><b>10 Sơ đồ Kiến trúc Kỹ thuật (Mỗi sơ đồ 1 trang phóng lớn):</b> Sơ đồ IV.1 → Sơ đồ IV.10 đọc rõ nét 100%</a></td><td><a href="#page-21">21–30</a></td></tr>
        <tr><td><b>PHẦN 5</b></td><td><a href="#page-31"><b>Bằng chứng Bảo mật, Kiểm thử &amp; Evidence Pack:</b> 34/34 Unit Tests, cargo tree, Ed25519 PE #7777, Tamper Suite, Live ADB, SQLite, ACL, TC-01..TC-14 &amp; Manifest SHA-256</a></td><td><a href="#page-31">31–35</a></td></tr>
        <tr><td><b>PHẦN 6</b></td><td><a href="#page-36"><b>Minh họa Quy trình 9 Bước:</b> Đối chiếu <code>jx2-mobile</code> &amp; <code>kingdom-slg</code> + Phân định rạch ròi phần PoC CLI vs GUI P4–P6</a></td><td><a href="#page-36">36–37</a></td></tr>
        <tr><td><b>PHẦN 7</b></td><td><a href="#page-38"><b>Đánh giá UI/UX &amp; Responsive:</b> Ba độ phân giải <code>1280×720</code>, <code>1600×900</code>, <code>1920×1080</code> &amp; 9 tiêu chí chi tiết</a></td><td><a href="#page-38">38–39</a></td></tr>
        <tr><td><b>PHẦN 8</b></td><td><a href="#page-40"><b>Lộ trình P2–P6 &amp; Acceptance Criteria chuẩn hóa:</b> Timeline, Telemetry đo từng bước &amp; E2E GUI bắt buộc</a></td><td><a href="#page-40">40–41</a></td></tr>
        <tr><td><b>PHẦN 9</b></td><td><a href="#page-42"><b>Kết luận &amp; Hồ sơ Phê duyệt v1.2.1-REVIEW:</b> Tổng hợp 5 trạng thái, Rủi ro, Pre-P2 Gate &amp; 7 Quyết định QD-01..QD-07</a></td><td><a href="#page-42">42–43</a></td></tr>
      </table>
    </div>
    <div class="card">
      <div class="card-hdr">DANH SÁCH CÁC ĐIỂM ĐÃ SỬA TỪ v1.2.0-P1.2-GATE SANG v1.2.1-REVIEW (7 YÊU CẦU)</div>
      <table class="tbl-compact tbl-dense">
        <tr><th style="width:11%;">Yêu cầu</th><th style="width:89%;">Hành động Hiệu chỉnh Đã Thực hiện trong Phiên bản v1.2.1-REVIEW</th></tr>
        <tr><td><b>YC #1</b><br/><span class="muted">PDF UX</span></td><td>Tăng cỡ chữ bảng &amp; nội dung lên <code>8.1–9.4pt</code> (xóa cỡ <code>6.5pt</code>); tách <code>IV.1–IV.10</code> thành <b>1 sơ đồ/trang</b> (~132mm); phóng to ảnh UI lên ~128mm; sửa ngắt chữ giữa từ; bổ sung <b>PDF Bookmarks</b>; lấp đầy khoảng trống đáy các trang bảng.</td></tr>
        <tr><td><b>YC #2</b><br/><span class="muted">Studio</span></td><td>Thiết kế lại <code>Ảnh 03A</code> (Canvas 64% @ 1600×900, &gt;72% @ 1280×720) và thêm <code>Ảnh 03B</code> minh họa đủ <b>6 trạng thái tương tác</b>: Mặc định, Chọn Node, Đang chạy, Lỗi+Debug, Breakpoint/Step Run, Crop ảnh từ Lab vào Vision Node.</td></tr>
        <tr><td><b>YC #3</b><br/><span class="muted">Builder</span></td><td>Thiết kế lại <code>Ảnh 07A</code> dạng <b>WYSIWYG 3 khung</b> (thêm/xóa/kéo khu vực UI, chọn Task, click Preview chỉnh nhãn &amp; biến — không sửa JSON tay) + <code>Ảnh 07B</code> (3 Layouts &amp; 4 Preview States Online/Offline/Running/Error).</td></tr>
        <tr><td><b>YC #4</b><br/><span class="muted">Matrix</span></td><td>Bỏ câu <i>"bảo toàn 100% tính năng"</i>; phân loại từng thành phần theo <b>5 trạng thái</b> (`IMPLEMENTED / PARTIAL / MOCK / MISSING / PLANNED`), kiểm toán đúng <code>CredentialVault (HashMap)</code>, <code>NetworkView (setTimeout)</code>, <code>Checkpoint</code>, <code>RunsView (job-demo-8492)</code>, <code>TasksView</code>.</td></tr>
        <tr><td><b>YC #5</b><br/><span class="muted">Evidence</span></td><td>Đóng gói thư mục độc lập <code>artifacts/p1_2_v1_2_1_evidence_pack/</code> chứa raw logs, ảnh gốc, nguồn SVG/MMD, Git/OS metadata và tệp kiểm toán <code>EVIDENCE_MANIFEST_SHA256.json</code>.</td></tr>
        <tr><td><b>YC #6</b><br/><span class="muted">Criteria</span></td><td>Loại bỏ ràng buộc thời gian cứng <i>"1,1 giây tuyệt đối"</i>; báo cáo telemetry thực đo từng bước Step 1–8; bắt buộc kiểm thử E2E P4–P6 qua thao tác GUI thật trên <code>FarmerPro.exe</code>.</td></tr>
        <tr><td><b>YC #7</b><br/><span class="muted">Gate</span></td><td>Giữ nguyên 100% <code>apps/desktop/src</code> và production <code>farmerpro.db</code>; tạo QA report &amp; render toàn bộ 43 trang để kiểm chứng.</td></tr>
      </table>
    </div>
  </div>
"""))

# ==============================================================================
# PAGE 03: LIST OF FIGURES & PORTABLE EVIDENCE PACK STRUCTURE
# ==============================================================================
pages.append(page_wrap(3, "DANH MỤC HÌNH & EVIDENCE", "Danh Mục Đầy Đủ 27 Hình Ảnh + 15 Sơ Đồ Kỹ Thuật & Cấu Trúc Gói Evidence Pack", """
  <div class="grid-2">
    <div class="card">
      <div class="card-hdr">DANH MỤC HÌNH ẢNH GIAO DIỆN &amp; SƠ ĐỒ KỸ THUẬT TRONG PDF</div>
      <table class="tbl-compact tbl-dense">
        <tr><th style="width:17%;">Mã Hình</th><th style="width:53%;">Tên Hình Ảnh / Sơ Đồ Minh Họa</th><th style="width:20%;">Trạng Thái</th><th style="width:10%;">Trang</th></tr>
        <tr><td><b>Sơ đồ 1.1–1.2</b></td><td>Kiến trúc tổng quan &amp; Quan hệ Studio–Engine–Builder–Runtime</td><td><span class="st-badge st-diagram">DIAGRAM</span></td><td>04–05</td></tr>
        <tr><td><b>Hình 1.2–1.9</b></td><td>UI Studio v0.1.0 hiện tại (6 tab) &amp; 2 Mini Tools độc lập PoC</td><td><span class="st-badge st-actual">ACTUAL</span></td><td>06–07</td></tr>
        <tr><td><b>Sơ đồ 2.1</b></td><td>So sánh Kiến trúc Cũ (10 Tab) vs Đề xuất P1.2 (6 Khu vực)</td><td><span class="st-badge st-diagram">DIAGRAM</span></td><td>08</td></tr>
        <tr><td><b>Ảnh 01–02</b></td><td>Dashboard mới &amp; Game Projects (kèm Wizard Tạo Project 4 Bước)</td><td><span class="st-badge st-proposed">PROPOSED</span></td><td>11–12</td></tr>
        <tr><td><b>Ảnh 03A–03B</b></td><td>Workflow Studio (Canvas Priority) &amp; 6 Trạng thái Tương tác</td><td><span class="st-badge st-proposed">PROPOSED</span></td><td>13–14</td></tr>
        <tr><td><b>Ảnh 04–06</b></td><td>Tasks &amp; Modules, Device Lab (ROI Cropper) &amp; Test &amp; Debug</td><td><span class="st-badge st-proposed">PROPOSED</span></td><td>15–17</td></tr>
        <tr><td><b>Ảnh 07A–07B</b></td><td>Mini Tool Builder WYSIWYG (Zero-JSON) &amp; 3 Layout / 4 States</td><td><span class="st-badge st-proposed">PROPOSED</span></td><td>18–19</td></tr>
        <tr><td><b>Ảnh 08</b></td><td>Release Management (Telemetry từng bước, SHA-256, PE #7777)</td><td><span class="st-badge st-proposed">PROPOSED</span></td><td>20</td></tr>
        <tr><td><b>Sơ đồ IV.1–10</b></td><td>10 Sơ đồ Kiến trúc Kỹ thuật bắt buộc (<b>1 sơ đồ / trang cỡ lớn</b>)</td><td><span class="st-badge st-diagram">DIAGRAM</span></td><td>21–30</td></tr>
        <tr><td><b>Hình 5.1–5.3</b></td><td>Ảnh LDPlayer thật Trước/Sau Safe Tap &amp; Template 88×88 px</td><td><span class="st-badge st-actual">ACTUAL</span></td><td>33</td></tr>
        <tr><td><b>Sơ đồ VI.1 / Hình 6.1–2</b></td><td>Quy trình sản xuất 9 bước &amp; Chạy độc lập 2 Mini Tools</td><td><span class="st-badge st-diagram">MIXED</span></td><td>36–37</td></tr>
        <tr><td><b>Hình 7.1–7.3</b></td><td>Bố cục Responsive <code>1280×720</code>, <code>1600×900</code> và <code>1920×1080</code></td><td><span class="st-badge st-proposed">PROPOSED</span></td><td>38</td></tr>
        <tr><td><b>Sơ đồ VIII.1</b></td><td>Timeline / Roadmap triển khai P2 → P6 (Stage-Gate)</td><td><span class="st-badge st-diagram">DIAGRAM</span></td><td>40</td></tr>
      </table>
    </div>
    <div class="card">
      <div class="card-hdr">CẤU TRÚC GÓI EVIDENCE DI ĐỘNG (artifacts/p1_2_v1_2_1_evidence_pack/)</div>
      <table class="tbl-compact tbl-dense">
        <tr><th style="width:32%;">Thư mục Con</th><th style="width:68%;">Nội dung Tệp Gốc Đính kèm để Kiểm chứng Độc lập</th></tr>
        <tr><td><code>01_pdf_deliverables/</code></td><td>Bản sao PDF <code>v1.2.1-REVIEW</code>, <code>CHANGELOG_v1_2_0_TO_v1_2_1.md</code> và <code>pdf_quality_assurance_report.json</code>.</td></tr>
        <tr><td><code>02_blueprint_and_specs/</code></td><td>Tài liệu <code>FARMERPRO_P1_2_STUDIO_BLUEPRINT.md</code> cập nhật &amp; 2 tệp mẫu <code>ui-schema.*.json</code>.</td></tr>
        <tr><td><code>03_raw_logs_and_json/</code></td><td>Raw log <code>cargo test --workspace</code> (chạy mới), <code>02_cargo_tree_mini_runtime.log</code>, JSON 5 Tamper Cases, PoC build report, Live ADB JSON, SQLite Backup &amp; NTFS ACL proofs.</td></tr>
        <tr><td><code>04_screenshots_actual/</code></td><td>10 ảnh chụp nguyên bản các tab Studio v0.1.0, 2 ảnh Mini Tools thật và ảnh chụp màn hình LDPlayer Trước/Sau.</td></tr>
        <tr><td><code>05_ui_mockups_fullres/</code></td><td>13 tệp ảnh PNG mockup độ phân giải cao (<code>ui_01</code>..<code>ui_08</code>, <code>ui_03b</code>, <code>ui_07b</code>, <code>ui_res_*</code>).</td></tr>
        <tr><td><code>06_diagrams_source/</code></td><td>15 tệp Vector <code>.svg</code> chỉnh sửa được + 15 tệp nguồn Mermaid <code>.mmd</code> + 15 PNG raster độ nét cao.</td></tr>
        <tr><td><code>07_git_and_environment/</code></td><td><code>git_and_environment_metadata.json</code> (Commit SHA, lịch sử 8 commit gần nhất, trạng thái sạch của <code>apps/desktop/src</code>, <code>rustc 1.96.0</code>, Windows 11 Pro).</td></tr>
        <tr><td><b>Root Manifest</b></td><td><code>EVIDENCE_MANIFEST_SHA256.json</code> &amp; <code>.txt</code> liệt kê đường dẫn tương đối, kích thước byte và mã băm SHA-256 của 100% tệp bằng chứng.</td></tr>
      </table>
    </div>
  </div>
"""))

# ==============================================================================
# PAGES 04 TO 07: PART 1 — PROJECT OVERVIEW, SƠ ĐỒ 1.1, SƠ ĐỒ 1.2, CURRENT UI, 2 MINI TOOLS
# ==============================================================================
pages.append(page_wrap(4, "PHẦN 1 — TỔNG QUAN DỰ ÁN (1/4)", "Định Vị Sản Phẩm FarmerPro, Vai Trò Studio IDE / Mini Runtime & Sơ Đồ 1.1", f"""
  <div class="grid-3" style="margin-bottom:2.2mm;">
    <div class="card">
      <div class="card-hdr">1. FARMERPRO LÀ NỀN TẢNG SẢN XUẤT ĐA AUTO TOOLS</div>
      <p class="p-xs"><b>FarmerPro</b> là Nhà máy Sản xuất Phần mềm Tự động hóa Game Android trên Windows, vận hành trên lõi <b>14 Rust Crates</b> và <b>Tauri v2 + React 19</b>. Một Studio IDE duy nhất quản lý nhiều <b>Game Projects</b> (MMORPG, SLG, RPG...) và đóng gói thành các Mini Auto-Tool độc lập.</p>
    </div>
    <div class="card">
      <div class="card-hdr">2. TÁCH BIỆT STUDIO IDE VS MINI RUNTIME</div>
      <p class="p-xs"><b>FarmerPro.exe (Studio):</b> Giữ <code>fp-builder</code> &amp; khóa ký bí mật ngoài dự án; thiết kế quy trình, cắt mẫu ROI và xuất bản. <b>Auto_&lt;Game&gt;.exe (Mini Runtime):</b> Không chứa <code>fp-builder</code>, không chứa khóa bí mật, đọc PubKey từ <b>PE <code>RT_RCDATA #7777</code></b> và dựng UI từ <code>ui-schema.json</code>.</p>
    </div>
    <div class="card">
      <div class="card-hdr">3. TIẾN TRÌNH P1 → P1.1 → P1.2 (v1.2.1-REVIEW)</div>
      <p class="p-xs">P1 chứng minh runtime đa game qua CLI; P1.1 tách 3 crate đóng gói &amp; nâng cấp Ed25519 + Live ADB LDPlayer; P1.2 khôi phục <b>34/34 unit tests</b>, ghim PubKey vào <b>PE <code>#7777</code></b> và chuẩn hóa thiết kế UX/Evidence chờ duyệt trước P2.</p>
    </div>
  </div>
  {fig_box(img_b64("diag_00_part1_overall_product_architecture.png"), "Sơ đồ 1.1", "Kiến trúc tổng quan FarmerPro (Nền tảng Sản xuất Đa Auto Tools — Multi-Game Software Factory)", "artifacts/p1_2_evidence/diagrams/00_part1_overall_product_architecture.svg", "DIAGRAM", max_h="116mm")}
"""))

pages.append(page_wrap(5, "PHẦN 1 — TỔNG QUAN DỰ ÁN (2/4)", "Sơ Đồ Quan Hệ Giữa FarmerPro Studio IDE, Shared Workflow Engine, fp-builder Và Mini Runtime", f"""
  <div class="grid-3" style="margin-bottom:2.2mm;">
    <div class="card">
      <div class="card-hdr">1. CÔ LẬP BIÊN DỊCH &amp; MẬT MÃ (CRATE BOUNDARY)</div>
      <p class="p-xs"><b>Studio IDE</b> liên kết với <code>fp-builder</code> và giữ SigningKey ngoài thư mục dự án tại <code>%LOCALAPPDATA%\\FarmerPro\\ProducerKeys\\</code>. <b>Mini Runtime</b> không phụ thuộc <code>fp-builder</code> (xác thực qua <code>cargo tree</code>) và chỉ mang PubKey 32-byte trong <code>PE RT_RCDATA #7777</code>.</p>
    </div>
    <div class="card">
      <div class="card-hdr">2. CHIA SẺ LÕI TỰ ĐỘNG HÓA RUST DÙNG CHUNG</div>
      <p class="p-xs">Cả Studio Lab lẫn Mini Tool đều chạy trên cùng 8 core crates: <code>fp-workflow-schema</code>, <code>fp-workflow-runtime</code>, <code>fp-vision</code> (SSD + WinRT OCR), <code>fp-actions</code> (18% Safe Tap), <code>fp-adb</code>, <code>fp-device</code>, <code>fp-scheduler</code> và <code>fp-storage</code> — đảm bảo kết quả test khớp hành vi runtime.</p>
    </div>
    <div class="card">
      <div class="card-hdr">3. LUỒNG CHUYỂN GIAO ĐẶC TẢ KHÔNG MÃ NGUỒN</div>
      <p class="p-xs">Để xuất Mini Tool cho game mới, Studio chuyển đặc tả JSON/PNG (<code>workflows/*.json</code>, <code>templates/*.png</code>, <code>ui-schema.json</code>, <code>modules.json</code>) qua pipeline 8 bước của <code>fp-builder</code>, nhân bản binary mẫu <code>fp-mini-runtime.exe</code> mà không cần biên dịch lại Rust.</p>
    </div>
  </div>
  {fig_box(img_b64("diag_00_part1b_studio_engine_builder_runtime_relationship.png"), "Sơ đồ 1.2", "Sơ đồ Quan hệ chi tiết giữa Studio IDE, Shared Workflow Engine, fp-builder và Standalone Mini Runtime", "artifacts/p1_2_evidence/diagrams/00_part1b_studio_engine_builder_runtime_relationship.svg", "DIAGRAM", max_h="116mm")}
"""))

pages.append(page_wrap(6, "PHẦN 1 — TỔNG QUAN DỰ ÁN (3/4)", "Hình Ảnh FarmerPro Studio Hiện Tại (v0.1.0) & Giới Hạn Cần Khắc Phục Trước P4", f"""
  <div class="card" style="margin-bottom:2mm;">
    <div class="card-hdr">NHỮNG GIỚI HẠN CỐT LÕI CỦA GIAO DIỆN VÀ MÃ NGUỒN STUDIO HIỆN TẠI (v0.1.0 — CHƯA CHỈNH SỬA UI)</div>
    <div class="grid-3">
      <div class="p-xs"><b>① Cấu trúc 10 tab phẳng đơn game:</b> Thanh điều hướng ngang hàng phục vụ riêng <i>Võ Hiệp 2 Mobile</i>, thiếu ngữ cảnh <code>GameProject</code> đa trò chơi ở cấp toàn cục.</div>
      <div class="p-xs"><b>② Dữ liệu hardcode &amp; mock trên UI:</b> <code>jx2Workflows.ts</code> gắn cứng 10 quy trình JX2; <code>RunsView</code> có fallback <code>job-demo-8492</code>; <code>NetworkView</code> &amp; <code>AccountsView</code> còn mô phỏng/dữ liệu tĩnh.</div>
      <div class="p-xs"><b>③ Thiếu Builder GUI &amp; ROI Cropper:</b> Xuất Mini Tool hiện chạy bằng CLI <code>fp-cli export-poc</code>; Studio chưa có trình kéo cắt ROI và WYSIWYG Mini Tool Builder.</div>
    </div>
  </div>
  <div class="grid-3">
    {fig_box(img_b64("actual_studio_01_tong_quan.png"), "Hình 1.2", "Tab 01: Tổng quan (Đơn game)", "screenshots/farmerpro_tabs_20261121/01_tong_quan.png", "ACTUAL", max_h="46mm", compact=True)}
    {fig_box(img_b64("actual_studio_03_kho_quy_trinh_jx2.png"), "Hình 1.3", "Tab 03: Kho Quy trình JX2", "screenshots/farmerpro_tabs_20261121/03_kho_quy_trinh_jx2.png", "ACTUAL", max_h="46mm", compact=True)}
    {fig_box(img_b64("actual_studio_04_workflow_studio.png"), "Hình 1.4", "Tab 04: Workflow Studio cũ", "screenshots/farmerpro_tabs_20261121/04_workflow_studio.png", "ACTUAL", max_h="46mm", compact=True)}
  </div>
  <div class="grid-3" style="margin-top:2mm;">
    {fig_box(img_b64("actual_studio_05_bo_nhiem_vu.png"), "Hình 1.5", "Tab 05: Bộ Nhiệm vụ (Dữ liệu tĩnh)", "screenshots/farmerpro_tabs_20261121/05_bo_nhiem_vu.png", "ACTUAL", max_h="46mm", compact=True)}
    {fig_box(img_b64("actual_studio_06_phien_chay.png"), "Hình 1.6", "Tab 06: Phiên chạy (Fallback demo)", "screenshots/farmerpro_tabs_20261121/06_phien_chay.png", "ACTUAL", max_h="46mm", compact=True)}
    {fig_box(img_b64("actual_studio_02_thiet_bi_android.png"), "Hình 1.7", "Tab 02: Thiết bị Android (Chưa có ROI)", "screenshots/farmerpro_tabs_20261121/02_thiet_bi_android.png", "ACTUAL", max_h="46mm", compact=True)}
  </div>
"""))

pages.append(page_wrap(7, "PHẦN 1 — TỔNG QUAN DỰ ÁN (4/4)", "Hai Mini Tools Độc Lập Đã Xuất Qua CLI Từ Cùng Bộ Rust Core Crates (PoC Runtime)", f"""
  <div class="grid-2">
    {fig_box(img_b64("actual_mini_tool_jx2_mobile.png"), "Hình 1.8", "Auto_JX2_Mobile.exe (Amber #F59E0B · 5 Tasks · 3 cột OCR)", "screenshots/p1_poc/p1_1_auto_jx2_truthful.png", "ACTUAL", max_h="78mm", compact=True)}
    {fig_box(img_b64("actual_mini_tool_kingdom_slg.png"), "Hình 1.9", "Auto_Kingdom_SLG.exe (Emerald #10B981 · 4 Tasks · 4 cột OCR)", "screenshots/p1_poc/p1_1_auto_slg_truthful.png", "ACTUAL", max_h="78mm", compact=True)}
  </div>
  <div class="card" style="margin-top:2.2mm;">
    <div class="card-hdr">ĐỐI CHIẾU HAI MINI TOOLS ĐỘC LẬP ĐÃ XUẤT BẢN (CHỨNG MINH KIẾN TRÚC ĐA GAME Ở TẦNG RUNTIME)</div>
    <table class="tbl-compact">
      <tr><th style="width:22%;">Thuộc tính Sản phẩm</th><th style="width:39%;">Mini Tool #1: <code>Auto_JX2_Mobile.exe</code> (MMORPG)</th><th style="width:39%;">Mini Tool #2: <code>Auto_Kingdom_SLG.exe</code> (Strategy SLG)</th></tr>
      <tr><td><b>Đường dẫn &amp; SHA-256</b></td><td><code>releases/poc/Auto_JX2_Mobile/Auto_JX2_Mobile.exe</code> (<span class="hash"><code>55c051cba31c269c...</code></span>)</td><td><code>releases/poc/Auto_Kingdom_SLG/Auto_Kingdom_SLG.exe</code> (<span class="hash"><code>95732232077c0747...</code></span>)</td></tr>
      <tr><td><b>PE <code>RT_RCDATA #7777</code> Key</b></td><td><span class="hash"><code>acb43ebd9ab499f095fe9d4bc5be6db468a6cb2966a5cc817ad545da473df455</code></span></td><td><span class="hash"><code>acb43ebd9ab499f095fe9d4bc5be6db468a6cb2966a5cc817ad545da473df455</code></span></td></tr>
      <tr><td><b>Cột OCR động (Trung thực)</b></td><td><code>Cấp</code> · <code>Vàng</code> · <code>Bạc</code> → Hiển thị <code>"Chưa OCR (-)"</code> khi chưa quét thật</td><td><code>Thành Chính</code> · <code>Lương Thực</code> · <code>Gỗ</code> · <code>Quân Lực</code> → Hiển thị <code>"Chưa OCR (-)"</code></td></tr>
      <tr><td><b>Tasks &amp; Modules &amp; Cách xuất</b></td><td>5 Tasks · 5/6 Modules (Proxy BẬT) · Xuất qua <code>fp-cli export-poc</code></td><td>4 Tasks · 4/6 Modules (Proxy TẮT) · Xuất qua <code>fp-cli export-poc</code></td></tr>
    </table>
  </div>
"""))

# ==============================================================================
# PAGES 08 TO 10: PART 2 — BEFORE VS PROPOSED + HONEST 5-STATUS FEATURE AUDIT MATRIX
# ==============================================================================
pages.append(page_wrap(8, "PHẦN 2 — SO SÁNH KIẾN TRÚC (1/3)", "So Sánh Trực Quan Kiến Trúc Cũ (10 Tab Phẳng) Và Kiến Trúc Đề Xuất P1.2 (6 Khu Vực)", f"""
  {fig_box(img_b64("diag_00_part2_before_vs_proposed_architecture.png"), "Sơ đồ 2.1", "So sánh Kiến trúc Cũ (10 Tab) vs Đề xuất P1.2 (Context-Driven Studio IDE — Phân tách Thiết kế UI vs Trạng thái Backend)", "artifacts/p1_2_evidence/diagrams/00_part2_before_vs_proposed_architecture.svg", "DIAGRAM", max_h="112mm")}
  <div class="grid-2" style="margin-top:2.2mm;">
    <div class="card">
      <div class="card-hdr">5 NHƯỢC ĐIỂM CỦA KIẾN TRÚC &amp; MÃ NGUỒN CŨ (v0.1.0)</div>
      <ul class="list-xs">
        <li><b>Dashboard vận hành đơn game:</b> Chưa hỗ trợ nhiều dự án game trên cùng một Studio.</li>
        <li><b>Hardcode &amp; Mock phân tán:</b> <code>jx2Workflows.ts</code> hardcode; <code>CredentialVault (HashMap)</code>, <code>NetworkView (setTimeout)</code>, <code>TasksView</code> và <code>RunsView (job-demo-8492)</code> còn dữ liệu giả lập.</li>
        <li><b>Thiếu quy trình sản xuất trên GUI:</b> Chưa có ROI Cropper, WYSIWYG Builder và Release UI.</li>
      </ul>
    </div>
    <div class="card">
      <div class="card-hdr">6 KHU VỰC SẢN XUẤT ĐỀ XUẤT &amp; NGUYÊN TẮC TÁCH TRẠNG THÁI</div>
      <ul class="list-xs">
        <li><b>6 Khu vực chính:</b> <code>1. Game Projects</code> · <code>2. Workflow Studio</code> · <code>3. Tasks &amp; Modules</code> · <code>4. Device Lab</code> · <code>5. Test &amp; Debug</code> · <code>6. Mini Tool Builder &amp; Releases</code> + <code>System Settings</code>.</li>
        <li><b>Nguyên tắc trung thực:</b> Không tuyên bố <i>"bảo toàn 100% tính năng"</i> khi backend cũ còn mock; tách rõ <b>Di dời thiết kế/cấu trúc UI</b> khỏi <b>Trạng thái Backend (`IMPLEMENTED / PARTIAL / MOCK / MISSING / PLANNED`)</b> ở Trang 09–10.</li>
      </ul>
    </div>
  </div>
"""))

pages.append(page_wrap(9, "PHẦN 2 — MA TRẬN PHÂN LOẠI (2/3)", "Phân Loại Thực Tế Màn Hình 01 → 05 (Tách Thiết Kế UI vs Trạng Thái Backend)", """
  <div class="card">
    <div class="card-hdr">BẢNG ÁNH XẠ &amp; PHÂN LOẠI THỰC TẾ MÀN HÌNH 01 → 05 (KHÔNG ĐỒNG NHẤT TỒN TẠI UI VỚI BACKEND HOÀN THIỆN)</div>
    <table class="tbl-compact">
      <tr>
        <th style="width:5%;">STT</th>
        <th style="width:15%;">Màn hình Cũ (v0.1.0)</th>
        <th style="width:17%;">Khu vực Đích (P4)</th>
        <th style="width:14%;">Thiết kế UI / Dữ liệu</th>
        <th style="width:14%;">Trạng thái Backend</th>
        <th style="width:35%;">Đánh giá Trung thực &amp; Kế hoạch Hoàn thiện Backend (P2–P4)</th>
      </tr>
      <tr>
        <td><b>01</b></td>
        <td><b>Tổng quan</b><br/><code>OverviewView.tsx</code></td>
        <td><b>1. Game Projects</b> (Workspace Dashboard)</td>
        <td><span class="st-badge st-proposed">DI DỜI + TÁCH LỌC</span><br/>Xóa nút lặp Mở/Tắt App</td>
        <td><span class="st-badge st-proposed">PARTIAL</span></td>
        <td>Hiện chỉ hiển thị tổng quan 1 game cục bộ. Ở <b>P2–P4</b> sẽ nâng cấp truy vấn thật đa dự án từ SQLite v2 (<code>projects</code>, <code>jobs</code>, <code>project_releases</code>) và chuyển hẳn điều khiển thiết bị về <code>4. Device Lab</code>.</td>
      </tr>
      <tr>
        <td><b>02</b></td>
        <td><b>Thiết bị Android</b><br/><code>DevicesView.tsx</code></td>
        <td><b>4. Device Lab</b></td>
        <td><span class="st-badge st-actual">GIỮ TOÀN BỘ UI</span><br/>+ Thêm ROI Cropper</td>
        <td><span class="st-badge st-actual">IMPLEMENTED</span></td>
        <td>Lõi <code>fp-adb</code> (quét USB/TCP, IP range scan, <code>ldAction</code>, screencap PNG thật không nội suy shell) đã hoạt động trên LDPlayer thật. Ở <b>P3–P4</b> bổ sung công cụ kéo cắt vùng ROI lưu vào <code>vision_templates</code>.</td>
      </tr>
      <tr>
        <td><b>03</b></td>
        <td><b>Kho Quy trình JX2</b><br/><code>WorkflowsView.tsx</code></td>
        <td><b>1. Game Projects</b> &amp; <b>2. Workflow Studio</b></td>
        <td><span class="st-badge st-proposed">BỎ HARDCODE</span><br/>Lọc theo Project đang chọn</td>
        <td><span class="st-badge st-proposed">PARTIAL</span></td>
        <td><code>jx2Workflows.ts</code> hiện đang hardcode tĩnh 10 đồ thị Võ Hiệp 2 trên frontend. Ở <b>P2</b> sẽ migrate toàn bộ 10 đồ thị này thành dữ liệu DB của dự án mẫu <code>proj-jx2-mobile-001</code> thay vì gắn cứng trong code.</td>
      </tr>
      <tr>
        <td><b>04</b></td>
        <td><b>Workflow Studio</b><br/><code>WorkflowStudioView.tsx</code></td>
        <td><b>2. Workflow Studio</b> (IDE Core)</td>
        <td><span class="st-badge st-proposed">MỞ RỘNG CANVAS</span><br/>+ 6 Trạng thái (Ảnh 03B)</td>
        <td><span class="st-badge st-proposed">PARTIAL</span></td>
        <td>React Flow canvas &amp; <code>fp-workflow-schema/runtime</code> đã thực thi được đồ thị JSON, nhưng Breakpoint / Step Run tương tác trên UI và cầu nối mở thẳng ROI Cropper từ Vision Node hiện mới ở mức thiết kế (<b>PLANNED P3–P4</b>).</td>
      </tr>
      <tr>
        <td><b>05</b></td>
        <td><b>Bộ Nhiệm vụ</b><br/><code>TasksView.tsx</code></td>
        <td><b>3. Tasks &amp; Modules</b> (Task Library)</td>
        <td><span class="st-badge st-proposed">DI DỜI UI + TASK</span></td>
        <td><span class="st-badge st-red">MOCK (UI)</span><br/><span class="st-badge st-actual">CORE IMPL</span></td>
        <td>Lõi <code>fp-scheduler</code> (priority queue, device/account lease, fencing generation) đã <code>IMPLEMENTED</code> kèm unit tests; tuy nhiên <code>TasksView.tsx</code> hiện dùng mảng <code>useState</code> tĩnh (<code>cat-01..06</code>), chưa nối SQLite <code>task_definitions</code> (<b>PLANNED P2–P4</b>).</td>
      </tr>
    </table>
  </div>
  <div class="card" style="margin-top:2.5mm;">
    <div class="card-hdr">ĐỊNH NGHĨA CHUẨN HÓA 5 TRẠNG THÁI NGHIỆM THU TRONG MA TRẬN (CHUẨN BẮT BUỘC CHO TOÀN BỘ DỰ ÁN)</div>
    <div class="grid-3">
      <div class="sub-card"><span class="st-badge st-actual">IMPLEMENTED</span><p class="p-xs" style="margin-top:1.2mm;">Mã nguồn sản phẩm đã viết xong, biên dịch sạch và có unit test / CLI test hoặc kiểm chứng thực tế trên thiết bị.</p></div>
      <div class="sub-card"><span class="st-badge st-proposed">PARTIAL</span><p class="p-xs" style="margin-top:1.2mm;">Đã có lõi xử lý hoặc dữ liệu mẫu cục bộ, nhưng thiếu luồng đồng bộ đầy đủ (ví dụ thiếu DB persistence hoặc resume engine).</p></div>
      <div class="sub-card"><span class="st-badge st-red">MOCK / MISSING / PLANNED</span><p class="p-xs" style="margin-top:1.2mm;"><b>MOCK:</b> Dữ liệu/hành vi giả lập (HashMap/setTimeout/demo array). <b>MISSING:</b> Chưa có code (GUI/Authenticode). <b>PLANNED:</b> Đã lên lịch khắc phục ở P2–P5.</p></div>
    </div>
  </div>
"""))

pages.append(page_wrap(10, "PHẦN 2 — MA TRẬN PHÂN LOẠI (3/3)", "Phân Loại Màn Hình 06 → 10, Phân Hệ Mới & Kiểm Toán 6 Lõi Backend", """
  <div class="card">
    <div class="card-hdr">BẢNG PHÂN LOẠI MÀN HÌNH 06 → 10 + PHÂN HỆ MỚI (5 TRẠNG THÁI: IMPLEMENTED / PARTIAL / MOCK / MISSING / PLANNED)</div>
    <table class="tbl-compact tbl-dense">
      <tr>
        <th style="width:5%;">STT</th>
        <th style="width:15%;">Màn hình / Phân hệ</th>
        <th style="width:16%;">Khu vực Đích (P4)</th>
        <th style="width:13%;">Thiết kế / Dữ liệu</th>
        <th style="width:14%;">Trạng thái Backend</th>
        <th style="width:37%;">Kiểm toán Thực tế Trong Mã Nguồn &amp; Lộ trình Khắc phục (P2–P5)</th>
      </tr>
      <tr>
        <td><b>06</b></td>
        <td><b>Phiên chạy &amp; Checkpoint</b><br/><code>RunsView.tsx</code></td>
        <td><b>5. Test &amp; Debug</b></td>
        <td><span class="st-badge st-red">XÓA DEMO FALLBACK</span></td>
        <td><span class="st-badge st-red">MOCK (UI)</span><br/><span class="st-badge st-proposed">PARTIAL (CORE)</span></td>
        <td><code>RunsView.tsx</code> (L22–90) fallback sang <code>job-demo-8492</code> giả lập và <code>handleRestoreCheckpoint</code> chỉ bật state React bằng <code>setTimeout</code>. Backend <code>fp-storage</code> đã lưu bảng <code>checkpoints/step_events</code>, nhưng <code>fp-workflow-runtime</code> (L574) chỉ ghi timestamp vào biến nhớ, chưa có hàm <code>resume_from_checkpoint</code> (<b>PLANNED P3–P4</b>).</td>
      </tr>
      <tr>
        <td><b>07</b></td>
        <td><b>Tài khoản Vault</b><br/><code>AccountsView.tsx</code></td>
        <td><b>3. Tasks &amp; Modules</b> (Sub-tab Vault)</td>
        <td><span class="st-badge st-proposed">GIỮ UI + PROJECT</span></td>
        <td><span class="st-badge st-red">MOCK</span><br/>(<code>HashMap</code>)</td>
        <td>Kiểm tra <code>crates/fp-security/src/lib.rs</code> (L32–53): <code>CredentialVault</code> hiện là <code>Arc&lt;Mutex&lt;HashMap&gt;&gt;</code> trong RAM và <code>AccountsView</code> dùng state tĩnh (chỉ <code>SecretRedactor</code> &amp; <code>DestructiveActionGuard</code> là <code>IMPLEMENTED</code>). Tích hợp Windows Credential Manager API thật nằm ở <b>P2–P3 (PLANNED)</b>.</td>
      </tr>
      <tr>
        <td><b>08</b></td>
        <td><b>Mạng &amp; Proxy</b><br/><code>NetworkView.tsx</code></td>
        <td><b>3. Tasks &amp; Modules</b> (Sub-tab Proxy)</td>
        <td><span class="st-badge st-proposed">GIỮ CẢNH BÁO UDP</span></td>
        <td><span class="st-badge st-red">MOCK</span><br/>(<code>setTimeout</code>)</td>
        <td>Kiểm tra <code>NetworkView.tsx</code> (L26–57): nút Test Proxy dùng <code>setTimeout(..., 600)</code> trả ping giả <code>38ms</code> và bảng <code>forwardRules</code> là mảng tĩnh; <code>crates/fp-adb</code> chưa triển khai <code>adb forward/reverse</code>. Đo latency SOCKS5/HTTP thật &amp; forward lệnh ADB sẽ làm ở <b>P3 (PLANNED)</b>.</td>
      </tr>
      <tr>
        <td><b>09–10</b></td>
        <td><b>Design Tokens &amp; Cài đặt ADB</b></td>
        <td><b>⚙️ System Settings</b> Drawer</td>
        <td><span class="st-badge st-proposed">GOM VÀO SETTINGS</span></td>
        <td><span class="st-badge st-actual">ADB/WAL IMPL</span><br/><span class="st-badge st-diagram">KEY/BACKUP P2</span></td>
        <td>Chẩn đoán ADB binary &amp; SQLite WAL đã hoạt động; bảng Design Tokens chuyển vào tab Settings để nhường Sidebar. Tab quản lý <code>ProducerKeyManager</code> &amp; <code>Online Backup UI</code> chờ nối IPC ở <b>P2 (PLANNED)</b>.</td>
      </tr>
      <tr>
        <td><b>+1</b></td>
        <td><b>Mini Tool Builder &amp; Releases</b></td>
        <td><b>6. Mini Tool Builder &amp; Releases</b></td>
        <td><span class="st-badge st-proposed">MỚI (ẢNH 07A–08)</span></td>
        <td><span class="st-badge st-red">MISSING (GUI)</span><br/><span class="st-badge st-actual">CLI READY</span></td>
        <td>Pipeline 8 bước của <code>fp-builder</code> &amp; xác thực PE <code>RT_RCDATA #7777</code> đã chạy thật qua <code>fp-cli export-poc</code>, nhưng <b>chưa có giao diện Builder GUI &amp; Release UI</b> trong Studio và chưa có chữ ký OS Windows Authenticode (<b>PLANNED P4–P5</b>).</td>
      </tr>
    </table>
  </div>
  <div class="card" style="margin-top:2.5mm;">
    <div class="card-hdr">TÓM TẮT KIỂM TOÁN 6 LÕI BACKEND TRỌNG ĐIỂM (TRÁNH NHẦM LẪN GIỮA ĐÃ CÓ UI VỚI ĐÃ HOẠT ĐỘNG THẬT)</div>
    <table class="tbl-compact tbl-dense">
      <tr><th style="width:20%;">Thành phần</th><th style="width:14%;">Trạng thái</th><th style="width:66%;">Bằng chứng Trích từ Mã nguồn Đã Rà soát</th></tr>
      <tr><td><b>1. CredentialVault</b></td><td><span class="st-badge st-red">MOCK</span></td><td><code>crates/fp-security/src/lib.rs (L32–53)</code> hiện lưu bằng <code>HashMap</code> trong RAM; <code>AccountsView</code> dùng state tĩnh (chỉ <code>SecretRedactor</code>/<code>Guard</code> là thật).</td></tr>
      <tr><td><b>2. Proxy &amp; ADB Forward</b></td><td><span class="st-badge st-red">MOCK</span></td><td><code>NetworkView.tsx (L26–57)</code> giả lập test bằng <code>setTimeout(600)</code>; <code>forwardRules</code> tĩnh, <code>fp-adb</code> chưa có <code>forward/reverse</code>.</td></tr>
      <tr><td><b>3. Tasks &amp; Scheduler</b></td><td><span class="st-badge st-proposed">PARTIAL</span></td><td><code>crates/fp-scheduler</code> có hàng đợi ưu tiên &amp; device lease thật; <code>TasksView.tsx</code> hiện dùng state tĩnh.</td></tr>
      <tr><td><b>4. Checkpoint Restore</b></td><td><span class="st-badge st-proposed">PARTIAL</span></td><td><code>fp-storage</code> có bảng <code>checkpoints</code>; <code>fp-workflow-runtime</code> chưa có hàm <code>resume_from_checkpoint</code>; UI giả lập <code>setTimeout</code>.</td></tr>
      <tr><td><b>5. JX2 Workflow Lib</b></td><td><span class="st-badge st-proposed">PARTIAL</span></td><td><code>jx2Workflows.ts</code> hardcode 10 quy trình tĩnh cho 1 game; chờ migrate sang bảng DB đa dự án ở P2.</td></tr>
      <tr><td><b>6. RunsView Fallback</b></td><td><span class="st-badge st-red">MOCK</span></td><td><code>RunsView.tsx (L22–80)</code> tự hiển thị <code>job-demo-8492</code> giả lập khi DB chưa có job; sẽ xóa bỏ hoàn toàn ở P3–P4.</td></tr>
    </table>
  </div>
"""))

# ==============================================================================
# PAGES 11 TO 20: PART 3 — 10 PROPOSED UI SCREENS (ENLARGED ~128mm, INCLUDES 03B & 07B)
# ==============================================================================
UI_SCREENS_SPEC = [
    (11, "PHẦN 3 — THIẾT KẾ GIAO DIỆN MỚI (1/10)", "Ảnh 01 — Dashboard Mới (Workspace Hub Đa Dự Án Game)", "ui_01_new_dashboard.png",
     "Ảnh 01 — Giao diện Dashboard mới (1. Game Projects / Workspace Hub)",
     "artifacts/p1_2_evidence/images/ui_01_new_dashboard.png",
     [
         ("1. Tổng hợp Dự án & Truy cập nhanh", "Hiển thị thẻ tóm tắt các Game Projects đang mở, danh sách Workflow gần đây và 3 nút nhảy nhanh (Workflow Studio, ROI Cropper, Mini Tool Builder)."),
         ("2. Trạng thái Lab & Phiên kiểm thử", "Lấy trạng thái online thật từ Device Lab (emulator-5554) và kết quả Job mới nhất từ bảng step_events trong SQLite (không dùng số liệu giả)."),
         ("3. Bản Release & Cảnh báo Kiểm toán", "Liệt kê các gói Mini Tool đã ký Ed25519 + PE #7777 kèm cảnh báo trung thực về giới hạn Proxy UDP và tình trạng Authenticode.")
     ]),
    (12, "PHẦN 3 — THIẾT KẾ GIAO DIỆN MỚI (2/10)", "Ảnh 02 — Game Projects (Tạo Project Mới 4 Bước, Assets, Version & Modules)", "ui_02_game_projects.png",
     "Ảnh 02 — Quản lý đa dự án Game Projects & Wizard Tạo Project Mới 4 Bước",
     "artifacts/p1_2_evidence/images/ui_02_game_projects.png",
     [
         ("1. Quy trình Tạo Project Mới rõ ràng", "Wizard 4 bước: [1. Tên & Package ID] → [2. Độ phân giải & Icon .ico] → [3. Chọn Module 5 trạng thái] → [4. Khởi tạo projects/<slug>/] mà không đụng tới code."),
         ("2. Cấu hình Manifest & Biểu tượng PE", "Quản lý định danh slug, game_package_id, version phát hành và tệp icon đa lớp (.ico) tách riêng cho từng tựa game."),
         ("3. Thư viện Assets PNG & Ma trận Modules", "Quản lý ảnh mẫu PNG kèm tọa độ cắt gốc, mã băm SHA-256 và bật/tắt 6 module hệ thống (adb_core, vision_ocr, vault, proxy, port_fwd, optimizer).")
     ]),
    (13, "PHẦN 3 — THIẾT KẾ GIAO DIỆN MỚI (3/10)", "Ảnh 03A — Workflow Studio (Ưu Tiên Vùng Vẽ Canvas & Điều Khiển Lab)", "ui_03_workflow_studio.png",
     "Ảnh 03A — Workflow Studio (Canvas React Flow chiếm 64% @ 1600×900 và >72% @ 1280×720)",
     "artifacts/p1_2_evidence/images/ui_03_workflow_studio.png",
     [
         ("1. Ưu tiên Tuyệt đối cho React Flow Canvas", "Ở 1280×720 tự thu gọn NavRail (60px) + ẩn Palette vào nút nổi + thu gọn Dock (34px) giúp Canvas đạt >72%; ở 1600×900 đạt ~64% bề ngang."),
         ("2. Thẻ Node Tương phản Cao & Trực quan", "Mỗi Node hiển thị rõ trạng thái (OK, ★ Chọn, ▶ Đang chạy, ● BP), độ trễ đo được, ảnh mẫu PNG, điểm khớp SSD và tọa độ Safe Tap."),
         ("3. Cầu nối Device Lab & Task Library", "Inspector bên phải hiển thị mẫu cắt 88×88 px kèm nút [✂️ Cắt mẫu từ Device Lab] và nút [+ Đưa Workflow vào Task Library].")
     ]),
    (14, "PHẦN 3 — THIẾT KẾ GIAO DIỆN MỚI (4/10)", "Ảnh 03B — Đặc Tả 6 Trạng Thái Tương Tác Bắt Buộc Của Workflow Studio", "ui_03b_workflow_studio_states.png",
     "Ảnh 03B — Minh họa chi tiết 6 Trạng thái tương tác trên Canvas Workflow Studio",
     "artifacts/p1_2_evidence/images/ui_03b_workflow_studio_states.png",
     [
         ("Trạng thái 1–3 (Default / Selected / Running)", "(1) Mặc định tối ưu vùng vẽ; (2) Chọn Node bật viền #38BDF8 + mở Inspector; (3) Node đang chạy phát sáng xanh kèm latency & trace thời gian thực."),
         ("Trạng thái 4–5 (Error+Debug / Breakpoint)", "(4) Lỗi tô viền đỏ #F27883 kèm nút mở khung hình lỗi & nhảy sang Test & Debug; (5) Gắn chấm ● BP tạm dừng để Step Over / Inspect biến."),
         ("Trạng thái 6 (Lab Crop → Vision Node)", "Bấm [✂️ Cắt mẫu từ Lab] từ Vision Node để chụp emulator-5554, kéo khung ROI [74,135,88,88], lưu PNG vào Project và tự gán ngược lại Node.")
     ]),
    (15, "PHẦN 3 — THIẾT KẾ GIAO DIỆN MỚI (5/10)", "Ảnh 04 — Tasks & Modules (Ánh Xạ Workflow → Task, Tham Số, Vault & Proxy)", "ui_04_tasks_and_modules.png",
     "Ảnh 04 — Tasks & Modules (Đóng gói nhiệm vụ từ Workflow & cấu hình tài nguyên theo Project)",
     "artifacts/p1_2_evidence/images/ui_04_tasks_and_modules.png",
     [
         ("1. Luồng chuyển từ Workflow → Task", "Nhiệm vụ trong Task Library liên kết 1-1 tới workflow_id đã kiểm thử, phân nhóm tùy biến ([NV]/[PB]/[HĐ] hoặc [Quân Sự]) và sắp xếp thứ tự chạy."),
         ("2. Cấu hình Tham số Động không cần JSON", "Định nghĩa các trường boolean / number / select trực tiếp để Mini Tool tự sinh bảng tùy chọn cho người dùng cuối."),
         ("3. Vault, Proxy & Scheduler theo Project", "Tích hợp sub-tab Account Vault, Proxy và Watchdog theo ngữ cảnh Project; các phần đang MOCK (HashMap/setTimeout) sẽ hoàn thiện backend ở P2–P3.")
     ]),
    (16, "PHẦN 3 — THIẾT KẾ GIAO DIỆN MỚI (6/10)", "Ảnh 05 — Device Lab (ADB, Giả Lập, Chụp Ảnh Theo Yêu Cầu & ROI Cropper)", "ui_05_device_lab.png",
     "Ảnh 05 — Device Lab (Kết nối ADB, điều khiển giả lập, kéo cắt mẫu ROI & thử khớp SSD/OCR)",
     "artifacts/p1_2_evidence/images/ui_05_device_lab.png",
     [
         ("1. Trung tâm Phần cứng Duy nhất", "Gom toàn bộ ADB USB/TCP, quét dải IP LAN, điều khiển LDPlayer/Nox/BlueStacks và quản lý App vào 1 màn hình (loại bỏ trùng lặp trên Header)."),
         ("2. ROI Template Cropper Trực quan", "Chụp PNG không nén từ emulator-5554, kéo chọn khung vùng [74,135,88,88] và lưu thẳng vào projects/<slug>/templates/ kèm checksum SHA-256."),
         ("3. Thử nghiệm Khớp mẫu SSD & WinRT OCR", "Kiểm tra ngay điểm khớp đa tỉ lệ (score=1.000), điểm chạm Safe Random Tap (margin 18% → 111,170) và kết quả đọc WinRT OCR.")
     ]),
    (17, "PHẦN 3 — THIẾT KẾ GIAO DIỆN MỚI (7/10)", "Ảnh 06 — Test & Debug (Lọc Sự Kiện SQLite, Node Trace, Ảnh Trước/Sau & Lỗi)", "ui_06_test_and_debug.png",
     "Ảnh 06 — Test & Debug (Truy vấn SQLite WAL thật, lọc sự kiện, điều tra lỗi & ảnh Trước/Sau)",
     "artifacts/p1_2_evidence/images/ui_06_test_and_debug.png",
     [
         ("1. Bộ lọc Phiên, Loại sự kiện & 1-Click Debug", "Hỗ trợ lọc theo thiết bị, lọc sự kiện (Tất cả / Lỗi / Checkpoint) và nút [1-Click Nhảy tới Node Lỗi] quay lại Canvas Workflow Studio."),
         ("2. Đối chiếu Ảnh Trước / Sau Thao tác ADB", "Đặt cạnh nhau ảnh trước và sau khi Safe Tap để xác nhận trực quan thay đổi màn hình kèm vùng bbox khớp mẫu và kết quả OCR."),
         ("3. Xóa bỏ job-demo-8492 & Checkpoint Thật", "Cam kết xóa 100% mảng demo fallback khi chưa chạy thật; khôi phục Checkpoint được chuẩn hóa xuống engine SQLite ở P3.")
     ]),
    (18, "PHẦN 3 — THIẾT KẾ GIAO DIỆN MỚI (8/10)", "Ảnh 07A — Mini Tool Builder WYSIWYG (Trực Quan — Không Sửa JSON)", "ui_07_mini_tool_builder.png",
     "Ảnh 07A — Mini Tool Builder dạng 3 khung WYSIWYG (Kéo khu vực, Chọn Component sửa thuộc tính)",
     "artifacts/p1_2_evidence/images/ui_07_mini_tool_builder.png",
     [
         ("1. Thêm, Xóa, Ẩn & Kéo thả Sắp xếp Khu vực", "Cột trái cho phép bật/tắt con mắt (👁), xóa (🗑), kéo thả thứ tự 5 khu vực UI (Header, Bảng Máy & OCR, Nhiệm vụ, Log, Hỗ trợ) và chọn danh sách Tasks đưa vào tool."),
         ("2. Click Thành phần trên Preview để Chỉnh", "Khung giữa hiển thị Interactive Visual Canvas: click vào một thành phần (ví dụ Cột OCR 'Vàng') sẽ nổi khung chọn đứt nét để chỉnh trực tiếp bên phải."),
         ("3. Liên kết Biến Runtime Không cần JSON", "Cột phải cung cấp ô nhập nhãn, dropdown chọn biến (runtime.ocr.char_gold), fallback 'Chưa OCR (-)', lưu phiên bản Schema v1.2 và bấm Build EXE.")
     ]),
    (19, "PHẦN 3 — THIẾT KẾ GIAO DIỆN MỚI (9/10)", "Ảnh 07B — Chuyển Đổi 3 Bố Cục & 4 Trạng Thái Preview Trong Mini Tool Builder", "ui_07b_mini_builder_layouts_states.png",
     "Ảnh 07B — Kiểm chứng 3 chế độ bố cục (1 Cột / 2 Cột / Expanded) & 4 trạng thái Preview Runtime",
     "artifacts/p1_2_evidence/images/ui_07b_mini_builder_layouts_states.png",
     [
         ("1. Chuyển đổi Mượt 3 Layout Presets", "Hỗ trợ chuyển tức thời ngay trên Preview giữa Compact 1 Cột (520×660), 2 Cột chuẩn (740×660) và Expanded Workstation (1020×700)."),
         ("2. Preview 4 Trạng thái Thực thi", "Giả lập giao diện ở 4 trạng thái: Online (Sẵn sàng), Offline (Khóa nút chạy + hướng dẫn ADB), Running (Tiến độ + nút Cancel) và Error (Cảnh báo sai PE #7777 / mất kết nối)."),
         ("3. Khẳng định Giá trị No-Code", "Người thiết kế Mini Tool hoàn toàn không phải viết JSON thủ công cho các thành phần thông thường; nút JSON chỉ là chế độ đọc-chỉ cho Dev.")
     ]),
    (20, "PHẦN 3 — THIẾT KẾ GIAO DIỆN MỚI (10/10)", "Ảnh 08 — Release Management (Lịch Sử Phiên Bản, Telemetry Từng Bước, SHA-256 & PE #7777)", "ui_08_release_management.png",
     "Ảnh 08 — Release Management (Quản lý phiên bản, telemetry đóng gói, chữ ký Ed25519 & Authenticode)",
     "artifacts/p1_2_evidence/images/ui_08_release_management.png",
     [
         ("1. Lịch sử Bản dựng, Version & Trạng thái", "Quản lý danh sách phát hành theo từng Game Project (rel-jx2-1.0.0, rel-slg-1.0.0), phân biệt bản Draft / Verified PE #7777 / Signed Authenticode."),
         ("2. Telemetry Đo Thực tế Từng Bước (Step 1–8)", "Ghi nhận thời gian đo từng bước đóng gói (PoC: 1097ms / 1127ms) và kích thước nhị phân (~12.5 MB), không áp ngưỡng thời gian cứng làm điều kiện Pass/Fail."),
         ("3. SHA-256, Chữ ký Ed25519 & Authenticode P5", "Hiển thị SHA-256 binary, PubKey PE RT_RCDATA #7777, kiểm toán ./bin/adb.exe + WebView2 + WinRT OCR, và trạng thái chờ tích hợp signtool.exe ở P5.")
     ]),
]

for pg_no, sec_badge, pg_title, img_file, fig_num_str, src_path_str, notes in UI_SCREENS_SPEC:
    cards_html = "".join(
        f'<div class="card"><div class="card-hdr">{h}</div><p class="p-xs">{p}</p></div>'
        for h, p in notes
    )
    pages.append(page_wrap(pg_no, sec_badge, pg_title, f"""
      {fig_box(img_b64(img_file), fig_num_str, pg_title, src_path_str, "PROPOSED", max_h="128mm")}
      <div class="grid-3" style="margin-top:2.2mm;">
        {cards_html}
      </div>
    """))

# ==============================================================================
# PAGES 21 TO 30: PART 4 — 10 TECHNICAL ARCHITECTURE DIAGRAMS (1 DIAGRAM PER PAGE!)
# ==============================================================================
TECH_DIAG_PAGES = [
    (21, "PHẦN 4 — KIẾN TRÚC KỸ THUẬT (1/10)", "Sơ Đồ IV.1 — Rust Cargo Workspace Dependency Graph (14 Crates & 3 Apps)",
     "diag_IV_01_cargo_workspace_dependency_graph.png", "Sơ đồ IV.1",
     "Rust Cargo Workspace Dependency Graph — Cô lập biên dịch giữa fp-builder và fp-mini-runtime",
     "artifacts/p1_2_evidence/diagrams/IV_01_cargo_workspace_dependency_graph.svg",
     ("1. Ranh giới Phụ thuộc Nghiêm ngặt (Compile-Time Isolation)",
      "<code>apps/mini-runtime</code> chỉ phụ thuộc vào <code>fp-bundle-contract</code> (schema thuần, 0 mật mã ký) và <code>fp-bundle-verify</code> (chỉ chứa <code>VerifyingKey</code> &amp; đọc PE <code>#7777</code>). Crate đóng gói và ký số <code>fp-builder</code> chỉ tồn tại trong <code>apps/desktop</code> (Studio) và <code>apps/cli</code>."),
     ("2. Bằng chứng Kiểm chứng Thực tế (TC-02)",
      "Kết quả chạy <code>cargo tree -p fp-mini-runtime</code> trong gói Evidence (<code>02_cargo_tree_mini_runtime.log</code>) xác nhận <b>0 nhánh phụ thuộc</b> trỏ vào <code>fp-builder</code> hay bất kỳ thư viện sinh khóa bí mật nào trong file thực thi phát hành.")),

    (22, "PHẦN 4 — KIẾN TRÚC KỸ THUẬT (2/10)", "Sơ Đồ IV.2 — FarmerPro Studio IDE Internal Architecture (React 19 ↔ Tauri v2 ↔ Rust)",
     "diag_IV_02_farmerpro_studio_architecture.png", "Sơ đồ IV.2",
     "FarmerPro Studio IDE Internal Architecture (Frontend 6 Khu vực + Zustand ↔ Typed Tauri IPC ↔ Rust Core Services)",
     "artifacts/p1_2_evidence/diagrams/IV_02_farmerpro_studio_architecture.svg",
     ("1. Luồng Giao tiếp Hai chiều Qua Tauri v2 Typed IPC",
      "Giao diện React 19 + React Flow + Zustand đóng vai trò trình biên soạn trực quan và kiểm tra kết quả; toàn bộ thao tác nhạy cảm (ADB, SSD Vision, WinRT OCR, SQLite WAL, Ký số &amp; Đóng gói) chạy an toàn trong tầng dịch vụ Rust."),
     ("2. Phân tách Trạng thái Hiện tại vs Lộ trình P2–P4",
      "Các dịch vụ lõi Rust (`fp-adb`, `fp-vision`, `fp-storage`, `fp-builder`) đã sẵn sàng và có unit test; 11 lệnh Studio IPC mới (`studio_list_projects`, `studio_crop_and_save_template`, `studio_build_mini_tool_release`...) đang ở trạng thái <code>PLANNED</code> cho P2–P3.")),

    (23, "PHẦN 4 — KIẾN TRÚC KỸ THUẬT (3/10)", "Sơ Đồ IV.3 — GameProject Data Model & SQLite WAL Migration v1 → v2",
     "diag_IV_03_gameproject_data_model.png", "Sơ đồ IV.3",
     "GameProject Data Model & SQLite Migration v1 -> v2 ER Diagram (6 Bảng Đa Dự Án + Bảo Toàn Dữ Liệu Cũ)",
     "artifacts/p1_2_evidence/diagrams/IV_03_gameproject_data_model.svg",
     ("1. Mở rộng Từ Đơn Game Sang Đa Dự án Không Mất Dữ liệu",
      "Migration v2 bổ sung 6 bảng (<code>projects</code>, <code>project_modules</code>, <code>vision_templates</code>, <code>task_definitions</code>, <code>mini_tool_schemas</code>, <code>project_releases</code>) và thêm cột khóa ngoại <code>project_id</code> vào các bảng quy trình/phiên chạy cũ, gán mặc định vào <code>proj-jx2-mobile-001</code>."),
     ("2. Kiểm chứng Trên Bản sao & Bảo toàn Production DB",
      "Quy trình Online Backup + <code>PRAGMA integrity_check</code> + Rollback v3 giả lập lỗi đã được kiểm chứng trên bản sao <code>restored_migration_test.db</code>; tệp <code>farmerpro.db</code> sản xuất giữ nguyên vẹn tuyệt đối chờ phê duyệt P2.")) ,

    (24, "PHẦN 4 — KIẾN TRÚC KỸ THUẬT (4/10)", "Sơ Đồ IV.4 — Workflow Runtime Execution Architecture (State Machine + Vision + Safe Tap)",
     "diag_IV_04_workflow_runtime_execution_architecture.png", "Sơ đồ IV.4",
     "Workflow Runtime Execution Architecture (State Machine -> Chụp PNG -> SSD Match -> Safe Random Tap 18% -> WinRT OCR -> Checkpoint)",
     "artifacts/p1_2_evidence/diagrams/IV_04_workflow_runtime_execution_architecture.svg",
     ("1. Chuỗi Thực thi An toàn Không Cố định Tọa độ (Safe Random Tap Policy)",
      "Mỗi bước thao tác tự động trải qua: Chụp PNG theo yêu cầu → So khớp mẫu đa tỉ lệ SSD (<code>fp-vision</code>) → Thu hẹp vùng lề an toàn 18% &amp; chọn ngẫu nhiên điểm chạm bên trong vùng giao → Chụp ảnh xác nhận / WinRT OCR → Lưu <code>StepEvent</code> vào SQLite WAL."),
     ("2. Điểm cần Hoàn thiện ở P3 (Checkpoint Resume)",
      "Hiện tại nút <code>state.checkpoint</code> đã ghi <code>StepEventType::CheckpointSaved</code> vào SQLite (<code>fp-storage</code>) và biến thời gian vào context RAM (<code>fp-workflow-runtime</code>); cơ chế <code>resume_from_checkpoint</code> khôi phục đồ thị từ bước gián đoạn sẽ hoàn thiện ở P3.")),

    (25, "PHẦN 4 — KIẾN TRÚC KỸ THUẬT (5/10)", "Sơ Đồ IV.5 — ADB & Emulator Adapter Architecture (Zero Raw Shell Interpolation)",
     "diag_IV_05_adb_emulator_adapter_architecture.png", "Sơ đồ IV.5",
     "ADB & Emulator Adapter Architecture (tokio::process::Command Vector Argv + DeviceActor Generation Fencing + Orientation)",
     "artifacts/p1_2_evidence/diagrams/IV_05_adb_emulator_adapter_architecture.svg",
     ("1. Tuân thủ Tuyệt đối Zero Shell Interpolation (AGENTS.md Rule #3)",
      "Mọi lệnh ADB đều đi qua <code>tokio::process::Command</code> với vector tham số tường minh (<code>CREATE_NO_WINDOW</code> trên Windows tránh nháy console), không bao giờ ghép chuỗi shell thô; hỗ trợ tự nhận diện LDPlayer, Nox, BlueStacks và dải IP LAN."),
     ("2. Chuẩn hóa Tọa độ Xoay & Chụp Ảnh Theo Yêu cầu",
      "Không truyền video stream liên tục ở MVP; khung hình PNG được lấy trực tiếp qua <code>adb exec-out screencap -p</code>, tự động quy đổi ma trận tọa độ theo góc xoay màn hình (0°/90°/180°/270°) từ <code>fp-device</code>.")),

    (26, "PHẦN 4 — KIẾN TRÚC KỸ THUẬT (6/10)", "Sơ Đồ IV.6 — Task Scheduler & Multi-Device Execution Architecture",
     "diag_IV_06_task_scheduler_multidevice_execution.png", "Sơ đồ IV.6",
     "Task Scheduler & Multi-Device Execution (BinaryHeap Priority Queue + Exclusive Leases + Generation Fencing Tokens)",
     "artifacts/p1_2_evidence/diagrams/IV_06_task_scheduler_multidevice_execution.svg",
     ("1. Cô lập Tài nguyên & Chống Xung đột Đa Thiết bị",
      "<code>MultiDeviceScheduler</code> dùng hàng đợi ưu tiên <code>BinaryHeap</code>, cấp phát khóa thuê độc quyền theo từng <code>device_id</code> &amp; <code>account_id</code> kèm <b>Generation Token đơn điệu</b>, đảm bảo các lệnh trễ từ phiên cũ bị từ chối ngay lập tức."),
     ("2. Giới hạn Tài nguyên Vision Workers (Tokio Semaphore)",
      "Sử dụng <code>tokio::sync::Semaphore</code> điều tiết số tác vụ so khớp ảnh chạy song song nhằm tránh quá tải CPU/RAM khi điều khiển nhiều giả lập cùng lúc (đã kiểm chứng qua 2 unit tests trong <code>fp-scheduler</code>).")),

    (27, "PHẦN 4 — KIẾN TRÚC KỸ THUẬT (7/10)", "Sơ Đồ IV.7 — Mini Tool UI Schema Renderer Architecture (100% Data-Driven UI)",
     "diag_IV_07_mini_tool_ui_schema_renderer.png", "Sơ đồ IV.7",
     "Mini Tool UI Schema Renderer Architecture (Dựng giao diện động từ bundle/ui-schema.json & Kiểm soát Module 5 trạng thái)",
     "artifacts/p1_2_evidence/diagrams/IV_07_mini_tool_ui_schema_renderer.svg",
     ("1. Một Binary Runtime Phục vụ Mọi Tựa Game Qua Schema",
      "Khi khởi chạy, <code>fp-mini-runtime.exe</code> đọc <code>bundle/ui-schema.json</code> (đã qua xác thực chữ ký Ed25519) để dựng màu sắc thương hiệu, layout 1/2 cột, cột chỉ số OCR động và danh sách nhiệm vụ đặc thù cho từng game."),
     ("2. Kiểm soát Module 5 Trạng thái & Trung thực Trạng thái OCR",
      "Các module không đóng gói hoặc bị tắt trong <code>manifest.fpbundle.json</code> bị chặn khởi tạo ngay ở tầng Rust; mọi chỉ số nhân vật chưa có dữ liệu WinRT OCR thật đều hiển thị trung thực chuỗi fallback <code>\"Chưa OCR (-)\"</code>.")),

    (28, "PHẦN 4 — KIẾN TRÚC KỸ THUẬT (8/10)", "Sơ Đồ IV.8 — Mini Tool Build & Release Pipeline (8-Step Packager Telemetry)",
     "diag_IV_08_mini_tool_build_release_pipeline.png", "Sơ đồ IV.8",
     "Mini Tool Build & Release Pipeline (Step 1 Validate -> Step 5 PE Icon & RT_RCDATA #7777 -> Step 7 Ed25519 -> Step 8 Verify)",
     "artifacts/p1_2_evidence/diagrams/IV_08_mini_tool_build_release_pipeline.svg",
     ("1. Dây chuyền Đóng gói Khép kín 8 Bước Trên Rust (`fp-builder`)",
      "Gồm: (1–3) Kiểm tra đồ thị, chống Path Traversal <code>../</code> &amp; khóa Module 5 trạng thái; (4–5) Copy runtime &amp; nhúng 7 lớp Icon + 32-byte PubKey vào <b>PE <code>RT_RCDATA #7777</code></b>; (6–7) Đóng gói ADB + ký RFC 8032 Ed25519; (8) Tự đọc lại PE <code>#7777</code> xác thực &amp; kiểm toán phụ thuộc."),
     ("2. Chuẩn hóa Báo cáo Telemetry Thời gian Thực (Không Áp Ngưỡng Cứng)",
      "Thay vì đặt mốc <i>\"~1,1s\"</i> làm điều kiện tuyệt đối, Step 8 xuất <code>build-report.json</code> ghi thời gian từng bước theo tải thực tế (PoC đo được: Step 1–4 = ~28ms, Step 5 PE = ~1038–1058ms, Step 6–8 = ~36–39ms; tổng 1097–1127 ms).")),

    (29, "PHẦN 4 — KIẾN TRÚC KỸ THUẬT (9/10)", "Sơ Đồ IV.9 — Security Architecture: Ed25519 PE RT_RCDATA #7777 Root of Trust & Authenticode",
     "diag_IV_09_security_architecture_ed25519_root_of_trust.png", "Sơ đồ IV.9",
     "Security Architecture — Ed25519 PE RT_RCDATA #7777 Root of Trust, 5 Kịch bản Chống Giả mạo & Vai trò Windows Authenticode",
     "artifacts/p1_2_evidence/diagrams/IV_09_security_architecture_ed25519_root_of_trust.svg",
     ("1. Cơ chế Chặn Giả mạo Thư mục Bundle Trên Đĩa (5/5 Tamper Cases)",
      "Vì <code>fp-mini-runtime.exe</code> đọc khóa công khai gốc trực tiếp từ vùng tài nguyên PE <code>RT_RCDATA #7777</code> của tiến trình đang chạy, mọi hành vi sửa file bundle hay ghi đè đồng thời cả <code>trusted_publisher.pub</code> + <code>signature.fpbundle.json</code> bằng cặp khóa giả đều bị từ chối."),
     ("2. Giới hạn của PE #7777 & Chuỗi Tin cậy Đầy đủ Ở P5",
      "Chính khóa PE <code>#7777</code> không tự ngăn được kẻ tấn công thay thế hoặc vá toàn bộ file <code>.exe</code> khi file chưa ký số hệ điều hành. Vì vậy ở <b>P5</b> bắt buộc kết hợp ký <b>Windows Authenticode (<code>signtool.exe</code>)</b> sau Step 5 để OS bảo vệ toàn vẹn chính file <code>.exe</code>.")),

    (30, "PHẦN 4 — KIẾN TRÚC KỸ THUẬT (10/10)", "Sơ Đồ IV.10 — Studio → Multi-Game Mini Tool Production Lifecycle",
     "diag_IV_10_studio_to_mini_tool_production_lifecycle.png", "Sơ đồ IV.10",
     "Studio -> Multi-Game Mini Tool Production Lifecycle (Một nền tảng Studio IDE sản xuất hàng loạt Mini Tools độc lập)",
     "artifacts/p1_2_evidence/diagrams/IV_10_studio_to_mini_tool_production_lifecycle.svg",
     ("1. Mô hình Nhà máy Phần mềm (Software Factory) Đã Chứng minh Ở Tầng Core",
      "Cùng một bộ 14 Rust crates và 1 binary mẫu <code>fp-mini-runtime.exe</code> đã xuất ra hai sản phẩm hoàn toàn khác nhau về giao diện, bộ nhiệm vụ, cột OCR và danh mục module (<code>Auto_JX2_Mobile.exe</code> vs <code>Auto_Kingdom_SLG.exe</code>)."),
     ("2. Hành trình Khép kín Trên GUI Studio (Mục tiêu P4–P6)",
      "Sau khi hoàn thành Studio UI (P4) và Builder UI/Signing (P5), quy trình sản xuất sẽ diễn ra 100% trên giao diện đồ họa không cần CLI hay sửa JSON tay, và được kiểm chứng bằng E2E Test GUI ở P6.")),
]

for pg_no, sec_badge, pg_title, img_f, fnum, fdesc, fsrc, c1, c2 in TECH_DIAG_PAGES:
    pages.append(page_wrap(pg_no, sec_badge, pg_title, f"""
      {fig_box(img_b64(img_f), fnum, fdesc, fsrc, "DIAGRAM", max_h="138mm")}
      <div class="grid-2" style="margin-top:2.2mm;">
        <div class="card"><div class="card-hdr">{c1[0]}</div><p class="p-xs">{c1[1]}</p></div>
        <div class="card"><div class="card-hdr">{c2[0]}</div><p class="p-xs">{c2[1]}</p></div>
      </div>
    """))

# ==============================================================================
# PAGES 31 TO 35: PART 5 — SECURITY, TEST PROOFS, FULL-HEIGHT MATRICES & EVIDENCE MANIFEST
# ==============================================================================
pages.append(page_wrap(31, "PHẦN 5 — KIỂM THỬ & BẢO MẬT (1/5)", "Bằng Chứng cargo test --workspace (34/34 PASS), Đối Chiếu 27→32→31→34 & Ranh Giới cargo tree", """
  <div class="grid-2">
    <div class="card">
      <div class="card-hdr">1. ĐỐI CHIẾU BIẾN ĐỘNG UNIT TESTS: 27 → 32 → 31 → 34 PASS (ĐẦY ĐỦ 14 CRATES)</div>
      <p class="p-xs" style="margin-bottom:1.5mm;">• <b>Baseline P0-E (27) → P1-PoC (32):</b> Thêm 5 tests trong <code>fp-builder</code>. <b>→ P1.1 (31):</b> Khi tách thành 3 crates, thêm 1 test vector RFC 8032 (<code>+1</code>) nhưng sót 2 hàm test path traversal &amp; readonly fallback (<code>-2</code>). <b>→ P1.2 (34/34 PASS):</b> Khôi phục 2 test bị sót (<code>+2</code>) và thêm 1 test PE <code>RT_RCDATA #7777</code> (<code>+1</code>).</p>
      <table class="tbl-compact tbl-dense">
        <tr><th style="width:24%;">Crate</th><th style="width:7%;">Số</th><th style="width:56%;">Hàm Kiểm thử Thực tế</th><th style="width:13%;">Kết quả</th></tr>
        <tr><td><code>fp-actions</code></td><td><b>4</b></td><td>Coordinate transform, rotation, safe margin 18%, safe point sampling</td><td><span class="c-green">4/4 PASS</span></td></tr>
        <tr><td><code>fp-adb</code></td><td><b>2</b></td><td>Discover ADB path, parse ADB devices output</td><td><span class="c-green">2/2 PASS</span></td></tr>
        <tr><td><code>fp-builder</code></td><td><b>4</b></td><td>5-state modules, Ed25519 rotation/tamper, multi-device fencing, <b>PE #7777 root trust [Mới]</b></td><td><span class="c-green">4/4 PASS</span></td></tr>
        <tr><td><code>fp-bundle-contract</code></td><td><b>1</b></td><td><b>Readonly dir fallback to %LOCALAPPDATA% [Khôi phục P1.2]</b></td><td><span class="c-green">1/1 PASS</span></td></tr>
        <tr><td><code>fp-bundle-verify</code></td><td><b>2</b></td><td><b>Path traversal guard [Khôi phục P1.2]</b>, RFC 8032 Sec 7.1 Ed25519 vectors</td><td><span class="c-green">2/2 PASS</span></td></tr>
        <tr><td><code>fp-device</code></td><td><b>3</b></td><td>Device actor ops, hardware error resilience, generation fencing</td><td><span class="c-green">3/3 PASS</span></td></tr>
        <tr><td><code>fp-domain</code> &amp; <code>scheduler</code></td><td><b>4</b></td><td>Bounds intersection (2), priority queue &amp; exclusive device lease (2)</td><td><span class="c-green">4/4 PASS</span></td></tr>
        <tr><td><code>fp-security</code> &amp; <code>storage</code></td><td><b>5</b></td><td>Secret redactor, destructive guard (2), SQLite WAL CRUD &amp; checkpoint DB reopen (3)</td><td><span class="c-green">5/5 PASS</span></td></tr>
        <tr><td><code>fp-vision</code> &amp; <code>testkit</code></td><td><b>4</b></td><td>Fake driver flow (1), SSD template match speed/multiscale/exact (3)</td><td><span class="c-green">4/4 PASS</span></td></tr>
        <tr><td><code>fp-workflow-*</code></td><td><b>5</b></td><td>Executor quest flow, OCR/control nodes, retry logic (3) &amp; JSON validator (2)</td><td><span class="c-green">5/5 PASS</span></td></tr>
      </table>
    </div>
    <div class="card">
      <div class="card-hdr">2. RAW LOG CHẠY THỰC TẾ (cargo_test_workspace_p1_2.log &amp; 02_cargo_tree_mini_runtime.log)</div>
      <pre class="code-pre">=== Lệnh thực thi lại tại commit 130d254 (rustc 1.96.0 / Windows 11 Pro) ===
running 4 tests (fp-actions)        -&gt; test result: ok. 4 passed; 0 failed
running 2 tests (fp-adb)            -&gt; test result: ok. 2 passed; 0 failed
running 4 tests (fp-builder)        -&gt; test result: ok. 4 passed; 0 failed
running 1 test  (fp-bundle-contract)-&gt; test result: ok. 1 passed; 0 failed
running 2 tests (fp-bundle-verify)  -&gt; test result: ok. 2 passed; 0 failed
running 3 tests (fp-device)         -&gt; test result: ok. 3 passed; 0 failed
running 2 tests (fp-domain)         -&gt; test result: ok. 2 passed; 0 failed
running 2 tests (fp-scheduler)      -&gt; test result: ok. 2 passed; 0 failed
running 2 tests (fp-security)       -&gt; test result: ok. 2 passed; 0 failed
running 3 tests (fp-storage)        -&gt; test result: ok. 3 passed; 0 failed
running 1 test  (fp-testkit)        -&gt; test result: ok. 1 passed; 0 failed
running 3 tests (fp-vision)         -&gt; test result: ok. 3 passed; 0 failed
running 3 tests (fp-workflow-runtime)-&gt;test result: ok. 3 passed; 0 failed
running 2 tests (fp-workflow-schema)-&gt; test result: ok. 2 passed; 0 failed
---------------------------------------------------------------------------
TỔNG CỘNG: 34 PASSED; 0 FAILED; 0 IGNORED. (Cần bổ sung GUI E2E ở P4–P6)

=== cargo tree -p fp-mini-runtime (Cô lập Thời điểm Biên dịch) ===
fp-mini-runtime v0.1.0
├── fp-actions / fp-adb / fp-device / fp-domain
├── fp-bundle-contract (Zero-Crypto) &amp; fp-bundle-verify (PE #7777 Reader)
└── fp-scheduler / fp-security / fp-storage / fp-vision / fp-workflow-*
[XÁC NHẬN]: fp-builder KHÔNG tồn tại trong cây phụ thuộc của Mini Runtime!</pre>
    </div>
  </div>
"""))

pages.append(page_wrap(32, "PHẦN 5 — KIỂM THỬ & BẢO MẬT (2/5)", "Bằng Chứng Ed25519 RFC 8032, PE RT_RCDATA #7777, 5 Tamper Cases & Giới Hạn Root of Trust", """
  <div class="grid-2">
    <div class="card">
      <div class="card-hdr">1. RAW LOG 5 KỊCH BẢN TẤN CÔNG GIẢ MẠO (tamper_suite_5_cases_pe_root_of_trust.json)</div>
      <pre class="code-pre">{
  "algorithm": "Ed25519-RFC8032-SHA256",
  "allPassed": true,
  "rootOfTrustSource": "PE_RT_RCDATA_7777",
  "cases": [
    { "case": "1_modify_bundle_and_recompute_sha256", "rejected": true,
      "error": "ED25519_SIGNATURE_INVALID" },
    { "case": "2_replace_disk_trusted_publisher_pub", "rejected": true,
      "error": "PE_ROOT_OF_TRUST_MISMATCH (Disk pubkey != PE #7777)" },
    { "case": "3_replace_signature_bytes", "rejected": true,
      "error": "ED25519_SIGNATURE_INVALID" },
    { "case": "4_missing_manifest_or_file", "rejected": true,
      "error": "INTEGRITY_MISSING_MANIFEST" },
    { "case": "5_rogue_keypair_replace_both_disk_pubkey_and_signature",
      "rejected": true, "rogueSignatureValidAgainstDiskPubkey": true,
      "error": "ED25519_PUBKEY_UNTRUSTED (Blocked by PE #7777!)" }
  ]
}</pre>
      <div class="sub-card" style="margin-top:2.2mm;">
        <p class="p-xs"><b>Quét Nhị phân (<code>05_binary_no_secret_key_proof.jsonl</code>):</b> Kiểm tra 12.5 MB của cả <code>Auto_JX2_Mobile.exe</code> và <code>Auto_Kingdom_SLG.exe</code> xác nhận <b>0 byte</b> hạt giống khóa bí mật (Secret Seed) tồn tại trong binary.</p>
      </div>
    </div>
    <div class="card">
      <div class="card-hdr">2. PHÂN TÍCH TRUNG THỰC PHẠM VI BẢO VỆ CỦA PE RT_RCDATA #7777 &amp; VAI TRÒ AUTHENTICODE</div>
      <div class="warn-box">
        <p class="p-xs"><b>① Những gì cơ chế PE <code>RT_RCDATA #7777</code> BẢO VỆ ĐƯỢC:</b><br/>
        • Đọc khóa công khai Ed25519 trực tiếp từ vùng tài nguyên <code>.rsrc</code> của tiến trình đang chạy → Chặn triệt để việc sửa cấu hình trong <code>bundle/</code> hoặc tự tạo cặp khóa giả mạo ghi đè lên <code>bundle/trusted_publisher.pub</code> &amp; <code>signature.fpbundle.json</code> (<b>Case 2 &amp; Case 5</b>).</p>
        <p class="p-xs" style="margin-top:2mm;"><b>② Những gì cơ chế PE <code>#7777</code> KHÔNG TỰ BẢO VỆ ĐƯỢC:</b><br/>
        • Không tự chống lại việc kẻ tấn công thay thế toàn bộ tệp <code>.exe</code> bằng một file <code>.exe</code> giả mạo khác, hoặc dùng công cụ chỉnh sửa PE để vá chính vùng <code>RT_RCDATA #7777</code> khi file <code>.exe</code> chưa có chữ ký số hệ điều hành. Do đó <b>hệ thống hiện tại CHƯA đạt bảo mật phát hành thương mại đầy đủ</b>.</p>
        <p class="p-xs" style="margin-top:2mm;"><b>③ Bổ sung Bắt buộc Ở Giai đoạn P5 (Windows Authenticode):</b><br/>
        • Sau Step 5, file <code>Auto_&lt;Game&gt;.exe</code> phải được ký bằng <code>signtool.exe</code> (Code Signing OV/EV) để hệ điều hành Windows bảo vệ toàn vẹn toàn bộ tệp <code>.exe</code> (gồm cả mã máy &amp; PE <code>#7777</code>), tạo chuỗi tin cậy đầy đủ: <b>OS Authenticode → PE #7777 → Ed25519 Bundle</b>.</p>
      </div>
      <div class="card-hdr" style="margin-top:3mm;">3. SO SÁNH MỨC ĐỘ BẢO VỆ THEO 3 TẦNG KIỂM SOÁT</div>
      <table class="tbl-compact tbl-dense">
        <tr><th style="width:28%;">Mối đe dọa / Kịch bản</th><th style="width:24%;">Chỉ PubKey Đĩa</th><th style="width:24%;">PE RT_RCDATA #7777 (P1.2)</th><th style="width:24%;">PE #7777 + Authenticode (P5)</th></tr>
        <tr><td>Sửa file trong <code>bundle/</code></td><td><span class="st-badge st-actual">CHẶN</span></td><td><span class="st-badge st-actual">CHẶN</span></td><td><span class="st-badge st-actual">CHẶN</span></td></tr>
        <tr><td>Đổi PubKey + Ký lại giả mạo (Case 5)</td><td><span class="st-badge st-red">BỊ VƯỢT</span></td><td><span class="st-badge st-actual">CHẶN (5/5)</span></td><td><span class="st-badge st-actual">CHẶN</span></td></tr>
        <tr><td>Thay toàn bộ <code>.exe</code> / Vá PE <code>#7777</code></td><td><span class="st-badge st-red">BỊ VƯỢT</span></td><td><span class="st-badge st-red">CHƯA CHẶN</span></td><td><span class="st-badge st-actual">CHẶN BỞI OS</span></td></tr>
      </table>
    </div>
  </div>
"""))

pages.append(page_wrap(33, "PHẦN 5 — KIỂM THỬ & BẢO MẬT (3/5)", "Bằng Chứng Live ADB Trên LDPlayer Thật, Ảnh Trước/Sau, SQLite Backup & NTFS ACL", f"""
  <div class="grid-3">
    {fig_box(img_b64("actual_adb_before_screen.png"), "Hình 5.1", "LDPlayer thật TRƯỚC Tap (System Apps @ 74,135)", "08_adb_before_screen.png", "ACTUAL", max_h="54mm", compact=True)}
    {fig_box(img_b64("actual_adb_after_screen.png"), "Hình 5.2", "LDPlayer thật SAU Safe Tap (111,170) — Hộp thoại mở", "10_adb_after_screen.png", "ACTUAL", max_h="54mm", compact=True)}
    <div class="card">
      <div class="card-hdr">1. LOG THỰC THI LIVE ADB &amp; WINRT OCR</div>
      <div style="display:flex;align-items:center;gap:8px;margin-bottom:6px;">
        <img src="{img_b64('actual_adb_cropped_template.png')}" style="width:42px;height:42px;border:1.5px solid #3BC38D;border-radius:5px;" />
        <div class="p-xs"><b>Hình 5.3:</b> Mẫu <code>system_apps.png</code> (88×88 px · ACTUAL)</div>
      </div>
      <pre class="code-pre">Nguồn: 09_live_adb_workflow_execution.json
• Device: emulator-5554 (SM_X710N, API 34)
• Driver: LIVE_ADB | simulated: false
• Events: 19 StepEvents | State: Succeeded
• Match: SSD score=1.000 bbox=[74,135,88,88]
• Safe Tap (18%): tap(111.0, 170.0)
• WinRT OCR: "System Apps... Cài đặt Files Play"
• Checkpoint WAL: "live_adb_verified"
• Cancellation Test (11_live...json): PASS</pre>
    </div>
  </div>
  <div class="grid-2" style="margin-top:2.2mm;">
    <div class="card">
      <div class="card-hdr">2. SQLITE ONLINE BACKUP, RESTORE &amp; ROLLBACK TEST (13_sqlite_backup_restore_migration_proof.json)</div>
      <pre class="code-pre">• Source DB: dist-portable/FarmerPro/data/farmerpro.db (integrity = "ok")
• Online Backup: backups/p1_1_verified_backup/farmerpro_online_backup.db (FK violations = 0)
• Bảo toàn trên bản sao kiểm thử: devices=2 | workflows=2 | jobs=2 | step_events=31 -> game_projects=1
• Rollback Faulty V3 Migration: Lỗi DDL trong transaction v3 được rollback sạch về v2!
• Production DB Untouched: productionDbUntouched = true (Chưa chạm DB chính trước duyệt P2)</pre>
    </div>
    <div class="card">
      <div class="card-hdr">3. WINDOWS NTFS ACL READ-ONLY FALLBACK (12_windows_acl_readonly_proof.json)</div>
      <pre class="code-pre">• Lệnh khóa ghi OS: icacls .../Auto_Kingdom_SLG/data /deny (OI)(CI)(W,DC)
• Kiểm chứng OS Kernel chặn ghi trực tiếp: [Errno 13] Permission denied / os error 5
• Automatic Fallback: Mini Runtime tự chuyển SQLite sang %LOCALAPPDATA%\\FarmerPro\\MiniTools\\...
• Kết quả: Mini Tool khởi chạy &amp; mở DB WAL sạch, không bị crash (fallbackToLocalAppdataUsed=true)</pre>
    </div>
  </div>
"""))

pages.append(page_wrap(34, "PHẦN 5 — KIỂM THỬ & BẢO MẬT (4/5)", "Ma Trận Kiểm Thử Chuẩn Hóa — Phần A: Workspace, Crate Boundary & Ed25519 (TC-01 → TC-07)", """
  <div class="card" style="flex:1;display:flex;flex-direction:column;">
    <div class="card-hdr">MA TRẬN KIỂM THỬ NGHIỆM THU KỸ THUẬT (PHẦN A: TC-01 ĐẾN TC-07 — ĐẦY ĐỦ 7 THUỘC TÍNH &amp; CỠ CHỮ ĐỌC CHUẨN)</div>
    <table class="tbl-compact tbl-spacious" style="flex:1;">
      <tr>
        <th style="width:6%;">ID</th>
        <th style="width:16%;">Điều kiện Kiểm thử</th>
        <th style="width:17%;">Kết quả Kỳ vọng</th>
        <th style="width:18%;">Kết quả Thực tế Đo được</th>
        <th style="width:8%;">Trạng thái</th>
        <th style="width:19%;">Tệp Bằng chứng (Pack)</th>
        <th style="width:16%;">Giới hạn &amp; Phần Chờ P2–P6</th>
      </tr>
      <tr>
        <td><b>TC-01</b></td>
        <td>Chạy <code>cargo test --workspace</code> trên toàn bộ 14 Rust crates tại commit hiện tại.</td>
        <td>Toàn bộ 34 unit tests biên dịch thành công và đạt 100% PASS không hồi quy.</td>
        <td><b>34/34 unit tests PASS</b> (0 failed, 0 ignored) trên <code>rustc 1.96.0</code> / Windows 11.</td>
        <td><span class="st-badge st-actual">PASS</span></td>
        <td><code style="font-size:9.8px;">03_raw_logs_and_json/<br/>cargo_test_workspace_p1_2.log</code></td>
        <td>Chưa bao gồm E2E WebDriver test cho giao diện Studio P4 mới (sẽ làm ở P4 &amp; P6).</td>
      </tr>
      <tr>
        <td><b>TC-02</b></td>
        <td>Chạy <code>cargo tree -p fp-mini-runtime</code> kiểm tra đồ thị phụ thuộc biên dịch.</td>
        <td><code>fp-mini-runtime</code> hoàn toàn không phụ thuộc <code>fp-builder</code> hay trình sinh khóa.</td>
        <td>Cây phụ thuộc chỉ có <code>fp-bundle-contract</code> &amp; <code>fp-bundle-verify</code> (0 nhánh tới <code>fp-builder</code>).</td>
        <td><span class="st-badge st-actual">PASS</span></td>
        <td><code style="font-size:9.8px;">03_raw_logs_and_json/<br/>02_cargo_tree_mini_runtime.log</code></td>
        <td>Cần gắn thành bước kiểm tra tự động trong CI / script sau mỗi chỉnh sửa <code>Cargo.toml</code>.</td>
      </tr>
      <tr>
        <td><b>TC-03</b></td>
        <td>Kiểm thử vector chuẩn quốc tế IETF RFC 8032 Section 7.1 cho thuật toán Ed25519.</td>
        <td>Hàm xác thực chữ ký khớp vector chuẩn RFC 8032 &amp; từ chối nếu sai lệch 1 bit.</td>
        <td>Khớp vector chuẩn RFC 8032 và từ chối tức thì chữ ký bị sửa đổi trong <code>fp-bundle-verify</code>.</td>
        <td><span class="st-badge st-actual">PASS</span></td>
        <td><code style="font-size:9.8px;">03_raw_logs_and_json/<br/>03_ed25519_unit_tests.log</code></td>
        <td>Chưa đo hiệu năng ký số đối với các gói tài nguyên game kích thước lớn &gt;500 MB.</td>
      </tr>
      <tr>
        <td><b>TC-04</b></td>
        <td>Đóng gói 2 Mini Tools và đọc lại khóa công khai từ phân vùng PE <code>RT_RCDATA #7777</code>.</td>
        <td>PubKey 32-byte được ghi vào <code>.rsrc #7777</code> ở Step 5 và readback khớp 100%.</td>
        <td>Cả <code>Auto_JX2_Mobile.exe</code> &amp; <code>Auto_Kingdom_SLG.exe</code> chứa đúng PubKey <code>acb43ebd...</code> tại ID #7777.</td>
        <td><span class="st-badge st-actual">PASS</span></td>
        <td><code style="font-size:9.8px;">03_raw_logs_and_json/<br/>export_poc_tools_pe_root_of_trust.json</code></td>
        <td>Nhị phân chưa ký số hệ điều hành Windows Authenticode (sẽ tích hợp <code>signtool.exe</code> ở P5).</td>
      </tr>
      <tr>
        <td><b>TC-05</b></td>
        <td>Quét chuỗi byte nhị phân trên toàn bộ 12.5 MB của 2 tệp <code>.exe</code> tìm hạt giống khóa bí mật.</td>
        <td>Không tồn tại bất kỳ byte khóa bí mật Ed25519 (SigningKey Seed) nào trong file phát hành.</td>
        <td>Quét toàn tệp xác nhận <code>secretSeedFoundInBinary = false</code> trên cả 2 công cụ Mini Tools.</td>
        <td><span class="st-badge st-actual">PASS</span></td>
        <td><code style="font-size:9.8px;">03_raw_logs_and_json/<br/>05_binary_no_secret_key_proof.jsonl</code></td>
        <td>Kho khóa ngoài của Studio tại <code>%LOCALAPPDATA%</code> cần bọc bảo vệ thêm bằng Windows DPAPI ở P2.</td>
      </tr>
      <tr>
        <td><b>TC-06</b></td>
        <td>Chạy bộ 5 kịch bản tấn công giả mạo trên <code>Auto_JX2_Mobile.exe --test-tamper-suite</code>.</td>
        <td>Chặn toàn bộ 5 kịch bản giả mạo, kể cả Case 5 thay đồng thời PubKey &amp; chữ ký trên đĩa.</td>
        <td><b>5/5 kịch bản bị chặn đứng</b> (<code>allPassed: true</code>, nguồn khóa <code>PE_RT_RCDATA_7777</code>).</td>
        <td><span class="st-badge st-actual">PASS</span></td>
        <td><code style="font-size:9.8px;">03_raw_logs_and_json/<br/>tamper_suite_5_cases_pe_root_of_trust.json</code></td>
        <td>Không chặn được hành vi vá trực tiếp tài nguyên PE <code>.rsrc</code> nếu thiếu chữ ký Authenticode (P5).</td>
      </tr>
      <tr>
        <td><b>TC-07</b></td>
        <td>Chạy lặp lại bộ 5 kịch bản tấn công giả mạo trên <code>Auto_Kingdom_SLG.exe</code>.</td>
        <td>Chặn toàn bộ 5 kịch bản giả mạo trên công cụ của dự án game chiến thuật SLG thứ 2.</td>
        <td><b>5/5 kịch bản bị chặn đứng</b> (<code>allPassed: true</code>, nguồn khóa <code>PE_RT_RCDATA_7777</code>).</td>
        <td><span class="st-badge st-actual">PASS</span></td>
        <td><code style="font-size:9.8px;">03_raw_logs_and_json/<br/>tamper_suite_kingdom_slg_pe_root_of_trust.json</code></td>
        <td>Tương tự TC-06: cần ký <b>Authenticode</b> ở P5 để bảo vệ toàn vẹn chính tệp thực thi <code>.exe</code>.</td>
      </tr>
    </table>
  </div>
"""))

pages.append(page_wrap(35, "PHẦN 5 — KIỂM THỬ & BẢO MẬT (5/5)", "Ma Trận Kiểm Thử — Phần B (TC-08 → TC-14) & Bằng Chứng Môi Trường / Evidence Manifest", """
  <div class="card" style="margin-bottom:2mm;">
    <div class="card-hdr">MA TRẬN KIỂM THỬ NGHIỆM THU KỸ THUẬT (PHẦN B: TC-08 ĐẾN TC-14)</div>
    <table class="tbl-compact tbl-dense">
      <tr>
        <th style="width:6%;">ID</th>
        <th style="width:16%;">Điều kiện Kiểm thử</th>
        <th style="width:17%;">Kết quả Kỳ vọng</th>
        <th style="width:18%;">Kết quả Thực tế Đo được</th>
        <th style="width:8%;">Trạng thái</th>
        <th style="width:19%;">Tệp Bằng chứng (Pack)</th>
        <th style="width:16%;">Giới hạn &amp; Phần Chờ P2–P6</th>
      </tr>
      <tr>
        <td><b>TC-08</b></td>
        <td>Chạy luồng ADB + Vision SSD + Safe Tap + WinRT OCR thật trên LDPlayer <code>emulator-5554</code>.</td>
        <td>Chụp ảnh PNG thật, khớp mẫu SSD, Safe Tap 18%, mở hộp thoại &amp; đọc OCR thành công.</td>
        <td>Khớp <code>System Apps</code> score=1.00 @ (74,135), chạm (111,170), WinRT OCR đọc đúng nội dung (19 events).</td>
        <td><span class="st-badge st-actual">PASS</span></td>
        <td><code style="font-size:9.8px;">03_raw_logs_and_json/<br/>09_live_adb_workflow_execution.json</code></td>
        <td>Mới thử trên LDPlayer; cần thử thêm trên thiết bị Android vật lý kết nối USB ở P6.</td>
      </tr>
      <tr>
        <td><b>TC-09</b></td>
        <td>Hủy giữa chừng luồng đang thực thi (Cancellation / Stop Job) qua kết nối ADB thật.</td>
        <td>Dừng luồng an toàn, thu hồi Device Lease và ghi nhận trạng thái <code>Cancelled</code>.</td>
        <td>Luồng dừng đúng điểm chờ, giải phóng lease và trả về <code>Cancelled</code>.</td>
        <td><span class="st-badge st-actual">PASS</span></td>
        <td><code style="font-size:9.8px;">03_raw_logs_and_json/<br/>11_live_adb_workflow_cancelled.json</code></td>
        <td>Chưa kiểm thử tình huống rút đột ngột cáp USB giữa lúc truyền luồng ảnh screencap.</td>
      </tr>
      <tr>
        <td><b>TC-10</b></td>
        <td>Khóa quyền ghi thư mục <code>data/</code> bằng Windows NTFS ACL (<code>icacls /deny</code>).</td>
        <td>Mini Runtime không crash, phát hiện lỗi quyền OS và fallback sang <code>%LOCALAPPDATA%</code>.</td>
        <td>Phát hiện <code>os error 5</code> và chuyển DB WAL sạch sang <code>%LOCALAPPDATA%\\FarmerPro\\MiniTools\\...</code>.</td>
        <td><span class="st-badge st-actual">PASS</span></td>
        <td><code style="font-size:9.8px;">03_raw_logs_and_json/<br/>12_windows_acl_readonly_proof.json</code></td>
        <td>Chưa kiểm thử trường hợp phân vùng ổ đĩa hệ thống <code>C:</code> đầy 100%.</td>
      </tr>
      <tr>
        <td><b>TC-11</b></td>
        <td>Chạy SQLite Online Backup, <code>integrity_check</code>, Migration v1→v2 &amp; rollback v3 lỗi trên bản sao.</td>
        <td>Không mất dòng dữ liệu cũ nào và rollback sạch khi gặp lỗi DDL ở transaction v3.</td>
        <td>Bảo toàn đủ 2 devices, 2 workflows, 2 jobs, 31 step_events; rollback sạch về v2 khi lỗi v3.</td>
        <td><span class="st-badge st-actual">PASS</span></td>
        <td><code style="font-size:9.8px;">03_raw_logs_and_json/<br/>13_sqlite_backup_restore_migration_proof.json</code></td>
        <td>Chưa áp vào DB chính <code>farmerpro.db</code> (chờ duyệt P2); chưa có UI Checkpoint Restore P3.</td>
      </tr>
      <tr>
        <td><b>TC-12</b></td>
        <td>Kiểm thử cách ly đa thiết bị song song và Generation Fencing Token.</td>
        <td>2 thiết bị chạy độc lập; mọi lệnh đến từ thế hệ lease cũ bị chặn đứng.</td>
        <td>Unit tests <code>test_multi_device_concurrent_isolation_and_fencing</code> &amp; <code>test_generation_fencing</code> PASS.</td>
        <td><span class="st-badge st-actual">PASS</span></td>
        <td><code style="font-size:9.8px;">03_raw_logs_and_json/<br/>cargo_test_workspace_p1_2.log</code></td>
        <td>Mới thử trong bộ unit test đa luồng; cần kiểm thử 2+ giả lập LDPlayer thật đồng thời ở P6.</td>
      </tr>
      <tr>
        <td><b>TC-13</b></td>
        <td>Kiểm thử khóa chính sách Module 5 trạng thái và bộ lọc chống Path Traversal (<code>../</code>).</td>
        <td>Module ngoài danh mục đóng gói bị chặn; đường dẫn thoát khỏi thư mục gốc bị từ chối.</td>
        <td><code>test_five_state_module_and_capability_enforcement</code> &amp; <code>test_path_traversal_guard_blocks_escape</code> PASS.</td>
        <td><span class="st-badge st-actual">PASS</span></td>
        <td><code style="font-size:9.8px;">03_raw_logs_and_json/<br/>cargo_test_workspace_p1_2.log</code></td>
        <td>Chưa kiểm thử trên symlink phức tạp hoặc thư mục chia sẻ mạng SMB.</td>
      </tr>
      <tr>
        <td><b>TC-14</b></td>
        <td>Đóng FarmerPro Studio và khởi chạy độc lập 2 Mini Tools đã xuất qua CLI.</td>
        <td>Cả 2 Mini Tools tự đọc PE <code>#7777</code>, dựng đúng UI theo <code>ui-schema.json</code> và kết nối ADB.</td>
        <td><code>Auto_JX2_Mobile.exe</code> &amp; <code>Auto_Kingdom_SLG.exe</code> chạy độc lập và hiện trung thực <code>"Chưa OCR (-)"</code>.</td>
        <td><span class="st-badge st-actual">PASS</span></td>
        <td><code style="font-size:9.8px;">03_raw_logs_and_json/<br/>verify_bundle_pe_root_of_trust.json</code></td>
        <td>Xuất qua CLI (PoC); từ <b>P4–P6</b> bắt buộc kiểm chứng toàn bộ hành trình qua GUI Studio thật.</td>
      </tr>
    </table>
  </div>
  <div class="card">
    <div class="card-hdr">ĐỐI CHIẾU THÔNG SỐ MÔI TRƯỜNG, COMMIT GIT &amp; MANIFEST SHA-256 TRONG GÓI EVIDENCE PACK</div>
    <p class="p-xs">Mọi tệp raw log, JSON, ảnh gốc, nguồn <code>.svg/.mmd</code> và tệp <code>git_and_environment_metadata.json</code> (ghi nhận <code>rustc 1.96.0</code>, <code>cargo 1.96.0</code>, <code>Windows 11 Pro</code>, lịch sử commit Git &amp; xác nhận <code>apps/desktop/src</code> sạch) đều được đóng gói trọn vẹn trong <code>artifacts/p1_2_v1_2_1_evidence_pack/</code> kèm danh sách kiểm toán băm tại <code>EVIDENCE_MANIFEST_SHA256.json</code> để kiểm chứng độc lập không phụ thuộc đường dẫn máy cục bộ.</p>
  </div>
"""))

# ==============================================================================
# PAGES 36 TO 37: PART 6 — 9-STEP WORKFLOW ILLUSTRATION (WITH CLI POC VS P4-P6 GUI CLARITY)
# ==============================================================================
pages.append(page_wrap(36, "PHẦN 6 — QUY TRÌNH 9 BƯỚC (1/2)", "Sơ Đồ Quy Trình 9 Bước Sản Xuất Mini Tool & Minh Họa Bước 1 → Bước 5 Trên 2 Game Projects", f"""
  {fig_box(img_b64("diag_VI_01_9step_production_workflow.png"), "Sơ đồ VI.1", "Quy trình khép kín 9 bước sản xuất Mini Auto-Tool cho nhiều tựa game khác nhau", "artifacts/p1_2_evidence/diagrams/VI_01_9step_production_workflow.svg", "DIAGRAM", max_h="66mm")}
  <div class="card" style="margin-top:2.2mm;">
    <div class="card-hdr">MINH HỌA BƯỚC 1 → BƯỚC 5 TRÊN 2 GAME PROJECTS (LƯU Ý: GIAO DIỆN STUDIO MỚI ĐANG Ở TRẠNG THÁI PROPOSED P4)</div>
    <table class="tbl-compact">
      <tr>
        <th style="width:16%;">Bước Sản Xuất</th>
        <th style="width:16%;">Khu Vực Thiết kế</th>
        <th style="width:34%;">Game Project #1: <code>jx2-mobile</code> (Võ Hiệp 2 MMORPG)</th>
        <th style="width:34%;">Game Project #2: <code>kingdom-slg</code> (Kingdom Rise SLG)</th>
      </tr>
      <tr>
        <td><b>Bước 1: Tạo Project</b></td>
        <td><code>1. Game Projects</code></td>
        <td>Khởi tạo <code>proj-jx2-mobile-001</code> · Pkg <code>com.blackhole.jx2mobile</code> · Chuẩn <code>1920×1080</code> · Icon Võ Hiệp 7 lớp.</td>
        <td>Khởi tạo <code>proj-kingdom-slg-002</code> · Pkg <code>com.kingdom.rise.slg</code> · Chuẩn <code>1280×720</code> · Icon SLG 7 lớp.</td>
      </tr>
      <tr>
        <td><b>Bước 2: Kết nối &amp; ROI</b></td>
        <td><code>4. Device Lab</code></td>
        <td>Kết nối LDPlayer <code>emulator-5554</code>, chụp màn hình thật &amp; cắt 4 mẫu PNG (<code>system_apps.png</code>...).</td>
        <td>Chụp màn hình giả lập và cắt 3 ảnh mẫu tài nguyên chiến thuật (<code>farm_collect.png</code>, <code>cavalry_train.png</code>...).</td>
      </tr>
      <tr>
        <td><b>Bước 3: Vẽ Workflow</b></td>
        <td><code>2. Workflow Studio</code></td>
        <td>Đồ thị <code>wf-jx2-live-adb-probe</code> &amp; <code>wf-jx2-main-quest</code> (7 nodes, Safe Tap 18%, OCR, Checkpoint).</td>
        <td>Đồ thị <code>wf-slg-harvest-and-patrol</code> (5 nodes, Safe Tap 20%, dùng chung engine <code>fp-workflow-runtime</code>).</td>
      </tr>
      <tr>
        <td><b>Bước 4: Tasks &amp; Modules</b></td>
        <td><code>3. Tasks &amp; Modules</code></td>
        <td><b>5 Tasks</b> (<code>[NV]</code>, <code>[PB]</code>, <code>[HĐ]</code>) · Tham số động · Bật <b>5/6 Modules</b> (gồm <code>mod.proxy_network</code>).</td>
        <td><b>4 Tasks</b> (<code>[Nông Nghiệp]</code>, <code>[Quân Sự]</code>, <code>[Liên Minh]</code>) · Bật <b>4/6 Modules</b> (Tắt Proxy &amp; Port Forward).</td>
      </tr>
      <tr>
        <td><b>Bước 5: Test &amp; Debug</b></td>
        <td><code>5. Test &amp; Debug</code></td>
        <td>Chạy thật trên <code>emulator-5554</code>: 19 StepEvents thật, khớp mẫu 1.000, chạm (111,170), OCR PASS.</td>
        <td>Kiểm tra hợp lệ đồ thị qua <code>WorkflowValidator</code> &amp; thử khớp mẫu tài nguyên đạt <code>VALIDATED</code>.</td>
      </tr>
    </table>
  </div>
"""))

pages.append(page_wrap(37, "PHẦN 6 — QUY TRÌNH 9 BƯỚC (2/2)", "Bước 6 → Bước 9, Chạy Độc Lập 2 Mini Tools & Phân Định PoC CLI vs Nghiệm Thu GUI P4–P6", f"""
  <div class="card" style="margin-bottom:2.2mm;">
    <div class="card-hdr">BƯỚC 6 → BƯỚC 9: TỪ THIẾT KẾ UI SCHEMA ĐẾN ĐÓNG STUDIO CHẠY ĐỘC LẬP</div>
    <table class="tbl-compact">
      <tr>
        <th style="width:16%;">Bước Sản Xuất</th>
        <th style="width:16%;">Thực tế PoC vs Mục tiêu P4–P6</th>
        <th style="width:34%;">Game Project #1: <code>jx2-mobile</code></th>
        <th style="width:34%;">Game Project #2: <code>kingdom-slg</code></th>
      </tr>
      <tr>
        <td><b>Bước 6: Thiết kế UI Tool</b></td>
        <td>PoC: Schema JSON<br/><b>P4–P5: WYSIWYG GUI</b></td>
        <td>Layout <code>compact_2_col</code>, Amber <code>#F59E0B</code>, 3 cột OCR (<code>Cấp</code>, <code>Vàng</code>, <code>Bạc</code>) fallback <code>"Chưa OCR (-)"</code>.</td>
        <td>Layout <code>compact_2_col</code>, Emerald <code>#10B981</code>, 4 cột OCR (<code>Thành Chính</code>, <code>Lương Thực</code>, <code>Gỗ</code>, <code>Quân Lực</code>).</td>
      </tr>
      <tr>
        <td><b>Bước 7: Build EXE (8 Bước)</b></td>
        <td>PoC: <code>fp-cli export-poc</code><br/><b>P5: Nút Build trên GUI</b></td>
        <td>Copy <code>fp-mini-runtime.exe</code> + Icon + PE <code>#7777</code> + ký Ed25519 7 tệp (Đo thực tế PoC: <b>1,097 ms</b>; phụ thuộc máy &amp; AV).</td>
        <td>Copy <code>fp-mini-runtime.exe</code> + Icon + PE <code>#7777</code> + ký Ed25519 6 tệp (Đo thực tế PoC: <b>1,127 ms</b>; phụ thuộc máy &amp; AV).</td>
      </tr>
      <tr>
        <td><b>Bước 8: Kiểm tra Release</b></td>
        <td>Smoke &amp; Tamper Suite</td>
        <td>Verify PE <code>#7777</code> + Tamper 5/5 PASS tại <code>releases/poc/Auto_JX2_Mobile/Auto_JX2_Mobile.exe</code>.</td>
        <td>Verify PE <code>#7777</code> + Tamper 5/5 PASS tại <code>releases/poc/Auto_Kingdom_SLG/Auto_Kingdom_SLG.exe</code>.</td>
      </tr>
      <tr>
        <td><b>Bước 9: Chạy Độc lập</b></td>
        <td><b>ACTUAL (Đã test PoC)</b></td>
        <td>Đóng Studio hoàn toàn → Chạy <code>Auto_JX2_Mobile.exe</code> dùng ADB riêng &amp; <code>runtime_jx2-mobile.db</code>.</td>
        <td>Đóng Studio hoàn toàn → Chạy <code>Auto_Kingdom_SLG.exe</code> ẩn module Proxy &amp; dùng <code>runtime_kingdom-slg.db</code>.</td>
      </tr>
    </table>
  </div>
  <div class="grid-2">
    {fig_box(img_b64("actual_mini_tool_jx2_mobile.png"), "Hình 6.1 (Bước 9)", "Auto_JX2_Mobile.exe chạy độc lập hoàn toàn sau khi đóng Studio (xuất qua CLI PoC)", "screenshots/p1_poc/p1_1_auto_jx2_truthful.png", "ACTUAL", max_h="64mm", compact=True)}
    {fig_box(img_b64("actual_mini_tool_kingdom_slg.png"), "Hình 6.2 (Bước 9)", "Auto_Kingdom_SLG.exe chạy độc lập hoàn toàn sau khi đóng Studio (xuất qua CLI PoC)", "screenshots/p1_poc/p1_1_auto_slg_truthful.png", "ACTUAL", max_h="64mm", compact=True)}
  </div>
"""))

# ==============================================================================
# PAGES 38 TO 39: PART 7 — RESPONSIVE DESIGN AT 3 RESOLUTIONS & DETAILED RUBRIC
# ==============================================================================
pages.append(page_wrap(38, "PHẦN 7 — RESPONSIVE UI/UX (1/2)", "Hình Minh Họa Bố Cục Mục Tiêu Ở 3 Độ Phân Giải: 1280×720, 1600×900 & 1920×1080", f"""
  <div class="warn-box" style="margin-bottom:2mm;padding:1.8mm 3mm;">
    <b>TRẠNG THÁI BẮT BUỘC:</b> Ba ảnh bố cục dưới đây là <b>Thiết kế Thích ứng Mục tiêu (<code>PROPOSED DESIGN — NOT IMPLEMENTED</code>)</b> phục vụ chốt thiết kế P1.2. Khi triển khai ở P4, Workflow Studio ưu tiên tối đa vùng vẽ Canvas ở cả <code>1280×720</code> (&gt;72% diện tích) lẫn <code>1600×900</code> (~64% diện tích).
  </div>
  <div class="grid-2">
    {fig_box(img_b64("ui_res_1280x720.png"), "Hình 7.1 (1280 × 720 — HD Compact)", "Thu gọn NavRail (60px) & ẩn Palette vào drawer -> Canvas đạt 920×510 px (>72%)", "artifacts/p1_2_evidence/images/ui_res_1280x720.png", "PROPOSED", max_h="58mm", compact=True)}
    {fig_box(img_b64("ui_res_1600x900.png"), "Hình 7.2 (1600 × 900 — HD+ Target)", "Độ phân giải chuẩn: Hiển thị đủ 5 vùng không cuộn ngang, Canvas rộng rõ node", "artifacts/p1_2_evidence/images/ui_res_1600x900.png", "PROPOSED", max_h="58mm", compact=True)}
  </div>
  <div style="margin-top:2.2mm;">
    {fig_box(img_b64("ui_res_1920x1080.png"), "Hình 7.3 (1920 × 1080 — Full HD Workstation)", "Không gian mở rộng: Canvas đạt 1160×800 px hiển thị trọn đồ thị lớn & Execution Dock 210px", "artifacts/p1_2_evidence/images/ui_res_1920x1080.png", "PROPOSED", max_h="60mm", compact=True)}
  </div>
"""))

pages.append(page_wrap(39, "PHẦN 7 — RESPONSIVE UI/UX (2/2)", "Bảng Đánh Giá Chi Tiết 9 Tiêu Chí UI/UX & Responsive Trên Ba Độ Phân Giải Mục Tiêu", """
  <div class="card" style="flex:1;display:flex;flex-direction:column;">
    <div class="card-hdr">ĐÁNH GIÁ KỸ THUẬT 9 TIÊU CHÍ BỐ CỤC, ƯU TIÊN CANVAS &amp; KHẢ NĂNG THÍCH ỨNG (PROPOSED FOR P4)</div>
    <table class="tbl-compact tbl-spacious" style="flex:1;">
      <tr>
        <th style="width:17%;">Tiêu chí Đánh giá</th>
        <th style="width:24%;">Độ phân giải <code>1280 × 720</code> (HD)</th>
        <th style="width:25%;">Độ phân giải <code>1600 × 900</code> (Target)</th>
        <th style="width:24%;">Độ phân giải <code>1920 × 1080</code> (FHD)</th>
        <th style="width:10%;">Mục tiêu</th>
      </tr>
      <tr>
        <td><b>1. Sidebar &amp; Header</b></td>
        <td>Global Header <code>44px</code>; NavRail tự động thu gọn về <b><code>60px</code></b> (icon-only + tooltip) để nhường chiều ngang.</td>
        <td>Header <code>44px</code>; NavRail mở rộng <b><code>210px</code></b> hiển thị đầy đủ 6 khu vực &amp; phụ đề.</td>
        <td>Header <code>44px</code>; NavRail <b><code>220px</code></b> kèm thanh trạng thái ADB / SQLite / Key ID ở đáy.</td>
        <td><span class="st-badge st-proposed">ĐẠT CHUẨN</span></td>
      </tr>
      <tr>
        <td><b>2. Ưu tiên React Flow Canvas</b></td>
        <td>Ẩn Node Palette vào nút nổi <code>[+ Node]</code> &amp; thu gọn Dock → Canvas đạt <b><code>920 × 510–614 px</code> (&gt;72%)</b>.</td>
        <td>Canvas đạt <b><code>860 × 650 px</code></b> khi mở đủ 5 vùng, và tới <b><code>1090 × 794 px</code></b> khi thu gọn Palette.</td>
        <td>Canvas rộng <b><code>1160 × 800 px</code></b> (mở đủ mọi panel) và lên tới <b><code>1400 × 974 px</code></b>.</td>
        <td><span class="st-badge st-proposed">ƯU TIÊN</span></td>
      </tr>
      <tr>
        <td><b>3. Node / Property Inspector</b></td>
        <td>Rộng <b><code>280–300px</code></b>, chỉ tự bung khi click chọn Node hoặc thành phần Preview, đóng gọn bằng 1 click.</td>
        <td>Rộng <b><code>290–300px</code></b>, hiển thị thông số Node, thumbnail ảnh mẫu &amp; nút <i>Cắt mẫu từ Lab</i>.</td>
        <td>Rộng <b><code>300–312px</code></b> đúng thông số thiết kế, không che lấp đồ thị hay form builder.</td>
        <td><span class="st-badge st-proposed">CÂN ĐỐI</span></td>
      </tr>
      <tr>
        <td><b>4. Execution &amp; Debug Dock</b></td>
        <td>Mặc định thu gọn <b><code>34px</code></b> (chỉ hiện thanh log tóm tắt), bấm mở lên <b><code>140px</code></b> khi cần debug.</td>
        <td>Mở <b><code>140–180px</code></b> theo tab (Step Trace, Biến Runtime, Ảnh Khớp Mẫu, Checkpoints).</td>
        <td>Mở <b><code>210–220px</code></b> hiển thị đồng thời 6–8 sự kiện và nút nhảy thẳng tới Node lỗi.</td>
        <td><span class="st-badge st-proposed">2 NẤC</span></td>
      </tr>
      <tr>
        <td><b>5. Mini Tool Builder WYSIWYG</b></td>
        <td>Gom 3 khung thành tab chuyển đổi nhanh <i>[Khu vực ↔ Preview ↔ Thuộc tính]</i>, giữ Live Preview rõ nét.</td>
        <td>Hiển thị song song <b>3 khung WYSIWYG</b> (Khu vực UI + Preview tương tác + Trình sửa thuộc tính).</td>
        <td>Hiển thị trọn khung Preview kích thước thật <code>740×660</code> + toàn bộ thuộc tính &amp; 4 trạng thái.</td>
        <td><span class="st-badge st-proposed">TRỰC QUAN</span></td>
      </tr>
      <tr>
        <td><b>6. Khả năng Đọc &amp; Typography</b></td>
        <td>Body <code>12.5–13px</code>, bảng <code>11.5–12px</code>, tương phản Dark Precision <code>#101722 / #EDF3FC</code>.</td>
        <td>Không co tỉ lệ chữ bằng transform scale; giữ sắc nét dấu tiếng Việt ở zoom 100%.</td>
        <td>Font đơn cách <code>Consolas</code> cho ID, tọa độ, SHA-256; không ngắt giữa từ khóa/tên file.</td>
        <td><span class="st-badge st-proposed">SẮC NÉT</span></td>
      </tr>
      <tr>
        <td><b>7. Thu gọn Panel (4 Vùng)</b></td>
        <td colspan="3">Cả 4 panel xung quanh (<b>NavRail</b>, <b>Palette / UI Blocks</b>, <b>Inspector</b>, <b>Execution Dock</b>) đều có nút đóng/mở bằng 1 click hoặc phím tắt, tự lưu trạng thái theo từng độ phân giải.</td>
        <td><span class="st-badge st-proposed">4 PANEL</span></td>
      </tr>
      <tr>
        <td><b>8. Chống Trùng lặp Điều khiển</b></td>
        <td colspan="3">Xóa hoàn toàn nút <i>Mở/Tắt App</i> khỏi Header &amp; Tổng quan; Global Header chỉ còn bộ chọn ngữ cảnh (<code>Active Project</code> &amp; <code>Lab Device</code>) + hành động riêng của khu vực đang mở.</td>
        <td><span class="st-badge st-proposed">0% TRÙNG</span></td>
      </tr>
      <tr>
        <td><b>9. Chống Tràn &amp; Cắt Chữ</b></td>
        <td colspan="3">Mọi bảng dài nằm trong vùng cuộn dọc cục bộ, không cuộn ngang toàn trang; tên dài dùng <code>ellipsis</code> + tooltip + drawer chi tiết; bắt buộc chụp ảnh kiểm chứng thật ở P4 &amp; P6.</td>
        <td><span class="st-badge st-proposed">ZERO CLIP</span></td>
      </tr>
    </table>
  </div>
"""))

# ==============================================================================
# PAGES 40 TO 41: PART 8 — ROADMAP P2-P6 & STANDARDIZED ACCEPTANCE CRITERIA
# ==============================================================================
pages.append(page_wrap(40, "PHẦN 8 — LỘ TRÌNH P2–P6 (1/2)", "Sơ Đồ Roadmap P2 → P6 & Đặc Tả Giai Đoạn P2 (Storage/Security) Và P3 (Engine/Lab/Mock Fix)", f"""
  {fig_box(img_b64("diag_VIII_01_p2_to_p6_roadmap_timeline.png"), "Sơ đồ VIII.1", "Lộ trình triển khai kỹ thuật P2 -> P6 theo cơ chế Stage-Gate (Chỉ mở khóa từng giai đoạn sau khi duyệt)", "artifacts/p1_2_evidence/diagrams/VIII_01_p2_to_p6_roadmap_timeline.svg", "DIAGRAM", max_h="70mm")}
  <div class="card" style="margin-top:2.2mm;">
    <div class="card-hdr">ĐẶC TẢ CHI TIẾT GIAI ĐOẠN P2 VÀ GIAI ĐOẠN P3 (BAO GỒM KHẮC PHỤC TOÀN BỘ CÁC PHẦN MOCK Ở MỤC 3)</div>
    <table class="tbl-compact tbl-spacious">
      <tr>
        <th style="width:14%;">Thuộc tính</th>
        <th style="width:43%;">Giai đoạn P2 — Storage Migration v2, Vault Thật &amp; Keystore</th>
        <th style="width:43%;">Giai đoạn P3 — Engine, Proxy/Forward Thật, Checkpoint Resume &amp; Lab</th>
      </tr>
      <tr>
        <td><b>1. Phạm vi &amp; Module</b></td>
        <td><code>fp-storage</code>, <code>fp-domain</code>, <code>fp-security</code>, <code>fp-bundle-contract</code>, <code>src-tauri</code>.</td>
        <td><code>fp-adb</code>, <code>fp-device</code>, <code>fp-vision</code>, <code>fp-workflow-runtime</code>, <code>fp-scheduler</code>, <code>src-tauri</code>.</td>
      </tr>
      <tr>
        <td><b>2. Deliverables</b></td>
        <td>• Migration SQLite v1→v2 (6 bảng mới + cột <code>project_id</code>), migrate 10 luồng <code>jx2Workflows.ts</code> vào <code>proj-jx2-mobile-001</code>.<br/>• Thay thế <code>CredentialVault (HashMap)</code> bằng Windows Credential Manager API thật + bọc DPAPI cho producer key.<br/>• IPC: <code>studio_list_projects</code>, <code>studio_create_project</code>, <code>studio_update_project</code>.</td>
        <td>• Implement <b><code>resume_from_checkpoint</code></b> trong <code>fp-workflow-runtime</code> đọc từ bảng SQLite <code>checkpoints</code>.<br/>• Thay <code>NetworkView setTimeout</code> &amp; <code>forwardRules</code> tĩnh bằng SOCKS5 TCP probe + <code>adb forward/reverse</code> thật trong <code>fp-adb</code>.<br/>• IPC <code>studio_crop_and_save_template</code>, <code>studio_test_template_match</code> &amp; xóa sạch <code>job-demo-8492</code>.</td>
      </tr>
      <tr>
        <td><b>3. Dependency</b></td>
        <td>Chỉ khởi động sau khi <b>Người Phê duyệt mở khóa P2</b> trên bản v1.2.1-REVIEW.</td>
        <td>Phụ thuộc hoàn thành toàn bộ kiểm thử Migration v2 &amp; Credential Manager ở <b>P2</b>.</td>
      </tr>
      <tr>
        <td><b>4. Acceptance Criteria &amp; Test</b></td>
        <td>Online Backup + <code>integrity_check=ok</code> trước migrate; 0 mất dữ liệu; unit &amp; integration tests cho Windows Keyring, Migration &amp; Rollback.</td>
        <td>Cắt ROI lưu đúng DB/thư mục Project; Checkpoint resume tiếp tục được phiên dừng; Proxy probe &amp; <code>adb forward</code> chạy thật, 0 dữ liệu giả lập.</td>
      </tr>
    </table>
  </div>
"""))

pages.append(page_wrap(41, "PHẦN 8 — LỘ TRÌNH P2–P6 (2/2)", "Đặc Tả P4, P5, P6 & Chuẩn Hóa Tiêu Chí Chấp Thuận (Bỏ Mốc Thời Gian Cứng · Bắt Buộc E2E GUI)", """
  <div class="card" style="flex:1;display:flex;flex-direction:column;">
    <div class="card-hdr">ĐẶC TẢ GIAI ĐOẠN P4, P5, P6 — CHUẨN HÓA ACCEPTANCE CRITERIA (THEO ĐO LƯỜNG TELEMETRY &amp; GUI E2E THẬT)</div>
    <table class="tbl-compact tbl-spacious" style="flex:1;">
      <tr>
        <th style="width:13%;">Thuộc tính</th>
        <th style="width:29%;">Giai đoạn P4 — Studio UX Redesign &amp; WYSIWYG Builder UI</th>
        <th style="width:29%;">Giai đoạn P5 — Release Pipeline GUI &amp; Windows Authenticode</th>
        <th style="width:29%;">Giai đoạn P6 — End-to-End Multi-Game Acceptance (GUI Thực tế)</th>
      </tr>
      <tr>
        <td><b>1. Phạm vi &amp; Module</b></td>
        <td><code>apps/desktop/src/components/*</code>, <code>features/*</code>, <code>stores/*</code> (Tái cấu trúc 10 tab → 6 khu vực + Settings).</td>
        <td><code>fp-builder</code>, <code>fp-bundle-verify</code>, <code>apps/mini-runtime</code>, khu vực <code>6. Mini Tool Builder &amp; Releases</code>.</td>
        <td>Toàn bộ 14 Rust crates, <code>FarmerPro.exe</code> GUI và các gói <code>Auto_&lt;Game&gt;.exe</code> chạy độc lập.</td>
      </tr>
      <tr>
        <td><b>2. Deliverables</b></td>
        <td>• Global Context Header + 6 Workspaces + System Settings Drawer.<br/>• Workflow Studio đủ <b>6 trạng thái tương tác</b> (<code>Ảnh 03B</code>) &amp; ưu tiên Canvas.<br/>• Trình dựng <b>WYSIWYG Mini Tool Builder</b> không cần sửa JSON (<code>Ảnh 07A/07B</code>).</td>
        <td>• Nối IPC <code>studio_preflight_check</code>, <code>studio_build_mini_tool_release</code>, <code>studio_smoke_test</code>.<br/>• Báo cáo <code>build-report.json</code> ghi <b>telemetry thời gian từng bước (Step 1–8)</b>.<br/>• Tích hợp ký số <b>Windows Authenticode (<code>signtool.exe</code>)</b> sau Step 5.</td>
        <td>• Kịch bản E2E đầy đủ trên <b>2 Game Projects thật</b> (<code>jx2-mobile</code> &amp; <code>kingdom-slg</code>).<br/>• Hồ sơ nghiệm thu cuối kèm ảnh/log chứng minh thao tác GUI 100%, đóng Studio và chạy 2 Mini Tools trên thiết bị ADB thật.</td>
      </tr>
      <tr>
        <td><b>3. Dependency &amp; Rủi ro</b></td>
        <td>Phụ thuộc P2–P3; kiểm soát hồi quy state khi chuyển đổi nhanh Project bằng store tập trung.</td>
        <td>Phụ thuộc P4; bắt buộc chạy <code>signtool.exe</code> <b>sau</b> Step 5 PE <code>#7777</code> để không làm sai lệch chữ ký binary.</td>
        <td>Phụ thuộc P2–P5; kiểm thử thêm trên máy Windows trắng chưa cài sẵn WebView2 Evergreen.</td>
      </tr>
      <tr>
        <td><b>4. Acceptance Criteria (Đã Chuẩn hóa)</b></td>
        <td>• Ảnh chụp thực tế ở <code>1280×720</code>, <code>1600×900</code>, <code>1920×1080</code> đạt chuẩn 0 clip, Canvas &gt;72% ở 1280×720.<br/>• Người dùng thực hiện toàn bộ chỉnh sửa Mini Tool trên Canvas Preview mà <b>không phải mở JSON</b>.</td>
        <td>• <b>Không áp mốc cứng 1,1 giây</b>: hệ thống đo và ghi nhận trung thực <code>durationMs</code> từng bước 1..8 (phụ thuộc kích thước gói, AV, ổ đĩa).<br/>• Nút Smoke Test trên GUI chạy Verify PE <code>#7777</code> + Tamper 5/5 PASS + Authenticode hợp lệ.</td>
        <td>• <b>Bắt buộc thực thi 100% từ GUI Studio</b> (tạo Project → cắt ROI → vẽ Workflow → tạo Task → dựng UI WYSIWYG → bấm Build EXE → đóng Studio → chạy độc lập 2 EXE trên ADB thật), <b>không nghiệm thu chỉ bằng CLI hay mockup</b>.</td>
      </tr>
      <tr>
        <td><b>5. Test bắt buộc</b></td>
        <td>Test UI tự động/thủ công 3 độ phân giải + 6 trạng thái Workflow + 3 Layout/4 Preview States Builder.</td>
        <td>Test Build Telemetry 8 bước, Authenticode signature verify (<code>signtool verify /pa</code>) &amp; Tamper Suite từ Studio UI.</td>
        <td>Full Workspace Unit Tests + Real GUI E2E Multi-Game Acceptance Suite + Physical/Emulator ADB.</td>
      </tr>
    </table>
  </div>
"""))

# ==============================================================================
# PAGES 42 TO 43: PART 9 — CONCLUSION, 5-STATUS AUDIT SUMMARY, DECISIONS & DELIVERABLES
# ==============================================================================
pages.append(page_wrap(42, "PHẦN 9 — KẾT LUẬN & PHÊ DUYỆT (1/2)", "Tổng Hợp Hiện Trạng Theo 5 Trạng Thái Thực Tế & Rủi Ro Kỹ Thuật", """
  <div class="grid-2">
    <div class="card" style="display:flex;flex-direction:column;gap:2.2mm;">
      <div>
        <div class="card-hdr">1. IMPLEMENTED (ĐÃ TRIỂN KHAI &amp; CÓ TEST TRONG RUST / CLI)</div>
        <ul class="list-xs">
          <li><b>Lõi 14 Crates &amp; 34/34 Unit Tests PASS:</b> <code>fp-adb</code> (Command argv an toàn), <code>fp-vision</code> (Multiscale SSD + WinRT OCR), <code>fp-actions</code> (Safe Tap 18%), <code>fp-device</code> (Fencing), <code>fp-scheduler</code> (Core queues/leases), <code>fp-storage</code> (SQLite WAL), <code>fp-security</code> (<code>SecretRedactor</code> &amp; <code>DestructiveGuard</code>).</li>
          <li><b>Đóng gói &amp; Cô lập Biên dịch (<code>fp-builder</code> CLI + <code>fp-bundle-verify</code>):</b> Tách khỏi <code>fp-mini-runtime</code> (<code>cargo tree</code>); nhúng Ed25519 PubKey vào <b>PE <code>RT_RCDATA #7777</code></b> chặn <b>5/5 Tamper Cases</b>; Live ADB trên LDPlayer thật (19 events); NTFS ACL fallback sang <code>%LOCALAPPDATA%</code>.</li>
        </ul>
      </div>
      <div>
        <div class="card-hdr">2. PARTIAL (ĐÃ CÓ MỘT PHẦN, CẦN NÂNG CẤP ĐỒNG BỘ)</div>
        <ul class="list-xs">
          <li><b>Workflow Library JX2 (<code>jx2Workflows.ts</code>):</b> Có 10 đồ thị chuẩn chạy được nhưng hardcode tĩnh trên frontend; cần migrate vào SQLite đa dự án ở <b>P2</b>.</li>
          <li><b>Checkpoint Restore:</b> <code>fp-storage</code> đã lưu sự kiện/bảng <code>checkpoints</code>, nhưng <code>fp-workflow-runtime</code> mới ghi timestamp vào RAM, chưa có engine khôi phục <code>resume_from_checkpoint</code> từ SQLite (cần hoàn thiện ở <b>P3</b>).</li>
        </ul>
      </div>
    </div>

    <div class="card" style="display:flex;flex-direction:column;gap:2.2mm;">
      <div>
        <div class="card-hdr">3. MOCK (HIỆN CÒN GIẢ LẬP TRONG MÃ NGUỒN — BẮT BUỘC THAY Ở P2–P4)</div>
        <ul class="list-xs">
          <li><b><code>CredentialVault</code> (<code>crates/fp-security/L34</code>) &amp; <code>AccountsView</code>:</b> Backend đang dùng <code>Arc&lt;Mutex&lt;HashMap&gt;&gt;</code> trong RAM và UI dùng mảng tĩnh — chưa nối Windows Credential Manager API.</li>
          <li><b><code>NetworkView.tsx</code> (L26–57):</b> Nút test Proxy dùng <code>setTimeout(600)</code> trả ping giả 38ms; bảng <code>adb forwardRules</code> là dữ liệu tĩnh.</li>
          <li><b><code>TasksView.tsx</code> &amp; <code>RunsView.tsx</code>:</b> Nhiệm vụ dùng state tĩnh; Phiên chạy còn fallback demo <code>job-demo-8492</code> &amp; giả lập nút khôi phục checkpoint.</li>
        </ul>
      </div>
      <div>
        <div class="card-hdr">4. MISSING / PLANNED &amp; 5. RỦI RO KỸ THUẬT</div>
        <ul class="list-xs">
          <li><b><code>MISSING</code> (Chờ <code>PLANNED</code> P2–P5):</b> Migration v2 trên <code>farmerpro.db</code> chính (P2), 11 Tauri Studio IPC (P2–P3), Giao diện 6 khu vực Studio + ROI Cropper + WYSIWYG Builder (P4–P5) và chữ ký <b>Windows Authenticode (<code>signtool.exe</code>)</b> (P5).</li>
          <li><b>Rủi ro cốt lõi:</b> Khi chưa có Authenticode, PE <code>#7777</code> chưa tự chống sửa/thay file <code>.exe</code>; lưu lượng UDP game không đi qua SOCKS5 TCP proxy; máy Windows cũ có thể thiếu WebView2 Evergreen.</li>
        </ul>
      </div>
    </div>
  </div>
  <div class="card" style="margin-top:2.5mm;">
    <div class="card-hdr">TỔNG KẾT MỨC ĐỘ SẴN SÀNG TỪNG PHÂN HỆ TRƯỚC KHI MỞ KHÓA TRIỂN KHAI P2 → P6</div>
    <table class="tbl-compact">
      <tr>
        <th style="width:24%;">Nhóm Phân hệ</th>
        <th style="width:16%;">Thiết kế &amp; Đặc tả</th>
        <th style="width:16%;">Mã nguồn Thực tế</th>
        <th style="width:44%;">Kết luận Kiểm định Trung thực &amp; Hướng Đi Tiếp Theo</th>
      </tr>
      <tr>
        <td><b>1. Lõi Rust Automation &amp; ADB</b></td>
        <td><span class="st-badge st-actual">HOÀN TẤT</span></td>
        <td><span class="st-badge st-actual">IMPLEMENTED</span></td>
        <td>Đã chứng minh qua 34/34 unit tests, Live LDPlayer (19 events), NTFS ACL fallback và 2 Mini Tools chạy độc lập qua CLI.</td>
      </tr>
      <tr>
        <td><b>2. Bảo mật Ed25519 &amp; PE #7777</b></td>
        <td><span class="st-badge st-actual">HOÀN TẤT</span></td>
        <td><span class="st-badge st-proposed">PARTIAL (CHỜ P5)</span></td>
        <td>Chống giả mạo thư mục <code>bundle/</code> (5/5 Tamper Cases PASS), nhưng chưa chống thay/vá chính file <code>.exe</code> cho tới khi ký Authenticode ở P5.</td>
      </tr>
      <tr>
        <td><b>3. Vault, Proxy, Checkpoint &amp; Demo</b></td>
        <td><span class="st-badge st-actual">ĐÃ ÁNH XẠ UI</span></td>
        <td><span class="st-badge st-red">MOCK / PARTIAL</span></td>
        <td>Không coi UI hiện có là đã hoàn chỉnh; bắt buộc thay thế <code>HashMap</code> Vault, <code>setTimeout</code> Proxy, <code>job-demo-8492</code> &amp; Resume Engine ở P2–P4.</td>
      </tr>
      <tr>
        <td><b>4. Studio IDE 6 Khu vực &amp; Builder</b></td>
        <td><span class="st-badge st-proposed">v1.2.1 PROPOSED</span></td>
        <td><span class="st-badge st-red">MISSING (CHƯA CODE)</span></td>
        <td>Giữ nguyên 100% <code>apps/desktop/src</code> tại P1.2; chỉ triển khai sau khi Người Phê duyệt thông qua 7 Quyết định tại Trang 43.</td>
      </tr>
    </table>
  </div>
"""))

pages.append(page_wrap(43, "PHẦN 9 — KẾT LUẬN & PHÊ DUYỆT (2/2)", "Pre-P2 Gate, 7 Quyết Định Kiến Trúc Cần Duyệt & Bàn Giao v1.2.1-REVIEW", """
  <div class="grid-2">
    <div class="card">
      <div class="card-hdr">6. CÁC ĐIỀU KIỆN TIÊN QUYẾT TRƯỚC KHI MỞ KHÓA P2 (PRE-P2 GATE)</div>
      <table class="tbl-compact">
        <tr><th style="width:13%;">Mã ĐK</th><th style="width:65%;">Nội dung Điều kiện Tiên quyết</th><th style="width:22%;">Trạng thái</th></tr>
        <tr><td><b>ĐK-01</b></td><td>Chạy &amp; kiểm chứng <b>34/34 Unit Tests PASS</b> trên toàn workspace và đóng gói raw log.</td><td><span class="st-badge st-actual">HOÀN THÀNH</span></td></tr>
        <tr><td><b>ĐK-02</b></td><td>Cứng hóa khóa công khai Ed25519 vào <b>PE <code>RT_RCDATA #7777</code></b> &amp; qua <b>5/5 Tamper Cases</b>.</td><td><span class="st-badge st-actual">HOÀN THÀNH</span></td></tr>
        <tr><td><b>ĐK-03</b></td><td>Kiểm chứng Online Backup + Migration v1→v2 + Rollback v3 trên bản sao DB.</td><td><span class="st-badge st-actual">HOÀN THÀNH</span></td></tr>
        <tr><td><b>ĐK-04</b></td><td>Giữ nguyên 100% <code>apps/desktop/src</code> và production <code>farmerpro.db</code> chưa sửa đổi.</td><td><span class="st-badge st-actual">ĐẢM BẢO 100%</span></td></tr>
        <tr><td><b>ĐK-05</b></td><td>Xuất bản hồ sơ sửa đổi <b>v1.2.1-REVIEW</b> kèm gói Evidence Pack SHA-256 &amp; chờ duyệt.</td><td><span class="st-badge st-proposed">CHỜ DUYỆT</span></td></tr>
      </table>

      <div class="card-hdr" style="margin-top:2.2mm;">DANH MỤC BÀN GIAO PHIÊN BẢN v1.2.1-REVIEW</div>
      <table class="tbl-compact tbl-dense">
        <tr><th style="width:7%;">#</th><th style="width:34%;">Hạng mục Bàn giao</th><th style="width:59%;">Đường dẫn Tương đối trong Repository</th></tr>
        <tr><td><b>1</b></td><td>Báo cáo PDF v1.2.1-REVIEW (Chính + Sao)</td><td><code>FARMERPRO_P1_2_ARCHITECTURE_UX_ACCEPTANCE_REPORT.pdf</code> &amp; <code>artifacts/p1_2_evidence/</code></td></tr>
        <tr><td><b>2</b></td><td>Gói Evidence Pack &amp; Manifest SHA-256</td><td><code>artifacts/p1_2_v1_2_1_evidence_pack/</code> (<code>EVIDENCE_MANIFEST_SHA256.json</code> &amp; <code>.txt</code>)</td></tr>
        <tr><td><b>3</b></td><td>Changelog &amp; Blueprint cập nhật</td><td><code>01_pdf_deliverables/CHANGELOG_v1_2_0_TO_v1_2_1.md</code> &amp; <code>FARMERPRO_P1_2_STUDIO_BLUEPRINT.md</code></td></tr>
        <tr><td><b>4</b></td><td>Ảnh gốc, Mockups &amp; Nguồn Sơ đồ</td><td><code>04_screenshots_actual/</code> · <code>05_ui_mockups_fullres/</code> · <code>06_diagrams_source/</code> (15 SVG + 15 MMD)</td></tr>
        <tr><td><b>5</b></td><td>Báo cáo QA PDF &amp; Git/Env Metadata</td><td><code>artifacts/p1_2_evidence/pdf_quality_assurance_report.json</code> &amp; <code>07_git_and_environment/</code></td></tr>
      </table>
    </div>

    <div class="card">
      <div class="card-hdr">7. BẢNG 7 QUYẾT ĐỊNH KIẾN TRÚC CẦN NGƯỜI PHÊ DUYỆT XÁC NHẬN MỞ KHÓA P2</div>
      <table class="tbl-compact tbl-dense">
        <tr><th style="width:11%;">Mã QĐ</th><th style="width:68%;">Nội dung Quyết định Kiến trúc Trình Phê duyệt</th><th style="width:21%;">Xác nhận</th></tr>
        <tr>
          <td><b>QD-01</b></td>
          <td><b>Kiến trúc Studio 6 Khu vực &amp; Ma trận Thực tế:</b> Duyệt chuyển đổi UI từ 10 tab sang 6 khu vực + Header + Settings (Trang 08–10), tách minh bạch phần thiết kế khỏi phần backend còn <code>MOCK/MISSING</code> để hoàn thiện ở P2–P5.</td>
          <td>☐ Đồng ý<br/>☐ Sửa thêm</td>
        </tr>
        <tr>
          <td><b>QD-02</b></td>
          <td><b>Schema SQLite Đa dự án (Migration v1 → v2):</b> Thêm 6 bảng đa dự án và migrate 10 quy trình JX2 vào <code>proj-jx2-mobile-001</code> sau bước Online Backup tự động.</td>
          <td>☐ Đồng ý<br/>☐ Sửa thêm</td>
        </tr>
        <tr>
          <td><b>QD-03</b></td>
          <td><b>Workflow Studio Canvas &amp; 6 Trạng thái (Ảnh 03A/03B):</b> Duyệt ưu tiên diện tích Canvas (&gt;72% @ 1280×720) và 6 trạng thái tương tác gồm liên thông cắt ROI từ Device Lab.</td>
          <td>☐ Đồng ý<br/>☐ Sửa thêm</td>
        </tr>
        <tr>
          <td><b>QD-04</b></td>
          <td><b>Loại bỏ 100% Mock Legacy (Vault/Proxy/Demo/Checkpoint):</b> Thay <code>CredentialVault HashMap</code>, <code>NetworkView setTimeout</code>, <code>job-demo-8492</code> bằng backend thật ở P2–P4.</td>
          <td>☐ Đồng ý<br/>☐ Sửa thêm</td>
        </tr>
        <tr>
          <td><b>QD-05</b></td>
          <td><b>Mini Tool Builder WYSIWYG (Ảnh 07A/07B):</b> Duyệt trình dựng UI 3 khung không cần sửa JSON tay, hỗ trợ kéo thả khu vực, chọn component sửa thuộc tính &amp; 3 Layout / 4 Preview States.</td>
          <td>☐ Đồng ý<br/>☐ Sửa thêm</td>
        </tr>
        <tr>
          <td><b>QD-06</b></td>
          <td><b>Bảo mật PE <code>#7777</code> + Windows Authenticode + Telemetry:</b> Kết hợp ký <code>signtool.exe</code> ở P5 và dùng telemetry từng bước (không áp ngưỡng cứng ~1,1s) cho pipeline.</td>
          <td>☐ Đồng ý<br/>☐ Sửa thêm</td>
        </tr>
        <tr>
          <td><b>QD-07</b></td>
          <td><b>Mở khóa Triển khai Có kiểm soát P2 → P6:</b> Triển khai tuần tự từng giai đoạn và bắt buộc nghiệm thu P4–P6 qua thao tác E2E trên GUI thật của <code>FarmerPro.exe</code>.</td>
          <td>☐ Mở khóa P2<br/>☐ Tạm dừng</td>
        </tr>
      </table>
      <div class="warn-box" style="margin-top:2mm;padding:2mm 2.8mm;">
        <b>CAM KẾT STAGE-GATE P1.2:</b> Tài liệu dừng nghiêm ngặt tại cổng kiểm định <b>v1.2.1-REVIEW</b>; mã nguồn giao diện và cơ sở dữ liệu chính chỉ được phép cập nhật sau khi nhận phê duyệt chính thức.
      </div>
    </div>
  </div>
"""))

assert len(pages) == TOTAL_PAGES, f"Expected {TOTAL_PAGES} pages, built {len(pages)}"

MASTER_HTML = """<!DOCTYPE html>
<html lang="vi">
<head>
<meta charset="UTF-8">
<title>FARMERPRO P1.2 (v1.2.1-REVIEW) — ARCHITECTURE & STUDIO UX ACCEPTANCE REPORT</title>
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
    font-size: 11.8px;
    line-height: 1.42;
    -webkit-print-color-adjust: exact !important;
    print-color-adjust: exact !important;
  }
  a { color: #38BDF8; text-decoration: none; }
  code {
    font-family: Consolas, 'Courier New', monospace;
    color: #38BDF8;
    font-size: 10.6px;
    word-break: normal;
    overflow-wrap: break-word;
  }
  .hash code, .hash {
    overflow-wrap: anywhere;
  }
  .muted { color: #8FA2BC; font-size: 10px; }
  .c-green { color: #3BC38D !important; font-weight: 700; }

  .pdf-page {
    width: 297mm;
    height: 210mm;
    padding: 5.5mm 8mm;
    page-break-after: always;
    break-after: page;
    overflow: hidden;
    background: #101722;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
  }
  .page-top-bar {
    height: 9mm;
    background: #141C28;
    border: 1px solid #2A3647;
    border-radius: 5px;
    padding: 0 3.5mm;
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-shrink: 0;
  }
  .pt-left { display: flex; align-items: center; gap: 3mm; min-width: 0; }
  .pt-badge {
    background: #1878F5;
    color: #FFFFFF;
    font-weight: 800;
    font-size: 10px;
    padding: 0.9mm 2.4mm;
    border-radius: 4px;
    letter-spacing: 0.25px;
    white-space: nowrap;
  }
  .pt-title { font-size: 11.4px; font-weight: 800; color: #FFFFFF; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .pt-right { font-size: 9.8px; font-weight: 600; color: #94A8C2; font-family: Consolas, monospace; white-space: nowrap; }

  .page-content {
    flex: 1;
    margin: 2.2mm 0;
    display: flex;
    flex-direction: column;
    justify-content: flex-start;
    overflow: hidden;
  }

  .page-bottom-bar {
    height: 7.2mm;
    background: #121924;
    border: 1px solid #2A3647;
    border-radius: 5px;
    padding: 0 3.5mm;
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 9.8px;
    color: #94A8C2;
    flex-shrink: 0;
  }
  .pg-num { font-weight: 800; color: #38BDF8; font-size: 10.6px; }

  .cover-hero {
    background: #141E2D;
    border: 1.5px solid #1878F5;
    border-radius: 8px;
    padding: 3.8mm 5mm;
    flex-shrink: 0;
  }
  .cover-pill-row { display: flex; gap: 2.5mm; margin-bottom: 2.2mm; flex-wrap: wrap; }
  .cover-h1 { font-size: 18px; font-weight: 800; color: #FFFFFF; line-height: 1.28; margin-bottom: 1.8mm; }
  .cover-sub { font-size: 11.4px; color: #D4DFEE; line-height: 1.44; }

  .grid-2 { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); gap: 3mm; }
  .grid-3 { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr) minmax(0, 1fr); gap: 3mm; }

  .card {
    background: #16202F;
    border: 1px solid #2A3647;
    border-radius: 6px;
    padding: 2.6mm 3.2mm;
    min-width: 0;
  }
  .sub-card {
    background: #121A26;
    border: 1px solid #2A3647;
    border-radius: 5px;
    padding: 2.2mm 2.8mm;
    min-width: 0;
  }
  .warn-box {
    background: #231C12;
    border: 1.5px solid #E7B15B;
    border-radius: 6px;
    padding: 2.4mm 3.2mm;
    color: #FDE68A;
  }
  .card-hdr {
    font-size: 11.2px;
    font-weight: 800;
    color: #38BDF8;
    padding-bottom: 1.3mm;
    margin-bottom: 1.8mm;
    border-bottom: 1px solid #2A3647;
    text-transform: uppercase;
    letter-spacing: 0.25px;
  }
  .p-xs { font-size: 11.2px; color: #DCE6F4; line-height: 1.42; }
  .list-xs { padding-left: 4.2mm; font-size: 11px; color: #DCE6F4; line-height: 1.42; }
  .list-xs li { margin-bottom: 1.4mm; }

  .tbl-compact {
    width: 100%;
    table-layout: fixed;
    border-collapse: collapse;
    font-size: 11px;
  }
  .tbl-compact th, .tbl-compact td {
    padding: 1.55mm 2.1mm;
    border: 1px solid #263244;
    text-align: left;
    vertical-align: top;
    line-height: 1.38;
    word-wrap: break-word;
    overflow-wrap: break-word;
  }
  .tbl-compact th {
    background: #121924;
    color: #A6B8D0;
    font-weight: 800;
    font-size: 10.2px;
    text-transform: uppercase;
  }
  .tbl-dense th, .tbl-dense td {
    padding: 1.15mm 1.75mm;
    font-size: 10.4px;
    line-height: 1.34;
  }
  .tbl-spacious th, .tbl-spacious td {
    padding: 2.1mm 2.3mm;
    font-size: 10.9px;
    line-height: 1.42;
  }

  .st-badge {
    display: inline-block;
    padding: 0.65mm 2mm;
    border-radius: 4px;
    font-size: 9.8px;
    font-weight: 800;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .st-actual { background: #133827; color: #3BC38D; border: 1px solid #3BC38D; }
  .st-proposed { background: #3B2D17; color: #FCD34D; border: 1px solid #E7B15B; }
  .st-diagram { background: #173355; color: #38BDF8; border: 1px solid #1878F5; }
  .st-red { background: #3D1820; color: #FCA5A5; border: 1px solid #F27883; }

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
    padding: 1.2mm 2.8mm;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 2mm;
    font-size: 10.4px;
    color: #EDF3FC;
  }
  .fc-left { display: flex; align-items: center; gap: 1.8mm; flex-wrap: wrap; min-width: 0; }
  .fc-src { color: #94A8C2; font-size: 9.8px; }

  .code-pre {
    background: #0B1018;
    border: 1px solid #263244;
    border-radius: 5px;
    padding: 2.2mm 2.8mm;
    font-family: Consolas, 'Courier New', monospace;
    font-size: 10.2px;
    color: #B4D0F0;
    line-height: 1.4;
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

TMP_PDF = os.path.join(ROOT, "_tmp_chrome_render.pdf")
CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"
subprocess.run([
    CHROME,
    "--headless=new",
    "--disable-gpu",
    "--no-pdf-header-footer",
    f"--print-to-pdf={TMP_PDF}",
    f"file:///{HTML_OUT.replace(os.sep, '/')}"
], check=True)

# Attach real hierarchical PDF Bookmarks (Outline) using PyMuPDF
PDF_BOOKMARKS = [
    [1, "Mở đầu — Trang Bìa (v1.2.1-REVIEW)", 1],
    [1, "Mục lục & Changelog v1.2.0 → v1.2.1-REVIEW", 2],
    [1, "Danh mục Hình ảnh, Sơ đồ & Gói Evidence Pack", 3],
    [1, "PHẦN 1 — Tổng quan Dự án", 4],
    [2, "Sơ đồ 1.1: Kiến trúc Tổng quan FarmerPro", 4],
    [2, "Sơ đồ 1.2: Quan hệ Studio–Engine–Builder–Runtime", 5],
    [2, "Giao diện Studio v0.1.0 Hiện tại (ACTUAL)", 6],
    [2, "Hai Mini Tools Độc lập PoC (ACTUAL)", 7],
    [1, "PHẦN 2 — So sánh Kiến trúc & Ma trận Thực tế 5 Trạng thái", 8],
    [2, "Sơ đồ 2.1: Before vs Proposed Architecture", 8],
    [2, "Ma trận Phân loại Thực tế Màn hình 01–05", 9],
    [2, "Ma trận Phân loại Thực tế Màn hình 06–10 & 6 Lõi Backend", 10],
    [1, "PHẦN 3 — Thiết kế Giao diện FarmerPro Studio Mới (10 Màn hình)", 11],
    [2, "Ảnh 01: Dashboard Mới", 11],
    [2, "Ảnh 02: Game Projects (Wizard 4 Bước)", 12],
    [2, "Ảnh 03A: Workflow Studio (Ưu tiên Canvas)", 13],
    [2, "Ảnh 03B: Workflow Studio — 6 Trạng thái Tương tác", 14],
    [2, "Ảnh 04: Tasks & Modules", 15],
    [2, "Ảnh 05: Device Lab (ROI Cropper & SSD Match)", 16],
    [2, "Ảnh 06: Test & Debug (Lọc sự kiện & Ảnh Trước/Sau)", 17],
    [2, "Ảnh 07A: Mini Tool Builder WYSIWYG (Zero JSON)", 18],
    [2, "Ảnh 07B: Mini Tool Builder — 3 Layout & 4 States", 19],
    [2, "Ảnh 08: Release Management", 20],
    [1, "PHẦN 4 — 10 Sơ đồ Kiến trúc Kỹ thuật (1 Sơ đồ / Trang)", 21],
    [2, "Sơ đồ IV.1: Cargo Workspace Dependency Graph", 21],
    [2, "Sơ đồ IV.2: FarmerPro Studio Architecture", 22],
    [2, "Sơ đồ IV.3: GameProject Data Model (SQLite v2)", 23],
    [2, "Sơ đồ IV.4: Workflow Runtime Execution Architecture", 24],
    [2, "Sơ đồ IV.5: ADB & Emulator Adapter Architecture", 25],
    [2, "Sơ đồ IV.6: Task Scheduler & Multi-Device Execution", 26],
    [2, "Sơ đồ IV.7: Mini Tool UI Schema Renderer", 27],
    [2, "Sơ đồ IV.8: Mini Tool Build & Release Pipeline (8 Bước)", 28],
    [2, "Sơ đồ IV.9: Security Ed25519 PE #7777 & Authenticode", 29],
    [2, "Sơ đồ IV.10: Studio → Mini Tool Production Lifecycle", 30],
    [1, "PHẦN 5 — Bằng chứng Bảo mật, Kiểm thử & Evidence Pack", 31],
    [2, "34/34 Unit Tests PASS & cargo tree", 31],
    [2, "Ed25519 PE #7777, 5 Tamper Cases & Giới hạn Root Trust", 32],
    [2, "Live ADB LDPlayer, SQLite Backup & NTFS ACL Fallback", 33],
    [2, "Ma trận Kiểm thử Phần A (TC-01 → TC-07)", 34],
    [2, "Ma trận Kiểm thử Phần B (TC-08 → TC-14) & Evidence Manifest", 35],
    [1, "PHẦN 6 — Minh họa Quy trình 9 Bước Sản xuất Mini Tool", 36],
    [2, "Sơ đồ VI.1 & Bước 1 → Bước 5 trên 2 Game Projects", 36],
    [2, "Bước 6 → Bước 9 & Đóng Studio Chạy Độc lập", 37],
    [1, "PHẦN 7 — Đánh giá UI/UX & Responsive (3 Độ phân giải)", 38],
    [2, "Minh họa 1280×720, 1600×900 & 1920×1080", 38],
    [2, "Bảng Đánh giá 9 Tiêu chí UI/UX Responsive", 39],
    [1, "PHẦN 8 — Lộ trình P2–P6 & Acceptance Criteria Chuẩn hóa", 40],
    [2, "Sơ đồ VIII.1 & Đặc tả P2–P3", 40],
    [2, "Đặc tả P4–P6 (Telemetry Từng bước & E2E GUI Thật)", 41],
    [1, "PHẦN 9 — Kết luận 5 Trạng thái & Hồ sơ Phê duyệt v1.2.1-REVIEW", 42],
    [2, "Tổng hợp Hiện trạng (IMPLEMENTED / PARTIAL / MOCK / MISSING)", 42],
    [2, "Pre-P2 Gate, Danh mục Bàn giao & 7 Quyết định QD-01..07", 43],
]

doc = pymupdf.open(TMP_PDF)
doc.set_toc(PDF_BOOKMARKS)
doc.save(PDF_OUT, garbage=3, deflate=True)
doc.close()
if os.path.exists(TMP_PDF):
    os.remove(TMP_PDF)

shutil.copy2(PDF_OUT, PDF_COPY)

pdf_size = os.path.getsize(PDF_OUT)
with open(PDF_OUT, "rb") as pf:
    pdf_sha256 = hashlib.sha256(pf.read()).hexdigest()

print(f"Generated PDF: {PDF_OUT} (with {len(PDF_BOOKMARKS)} PDF Bookmarks)")
print(f"Copied PDF to: {PDF_COPY}")
print(f"File Size    : {pdf_size:,} bytes ({pdf_size / (1024*1024):.2f} MB)")
print(f"SHA-256      : {pdf_sha256}")
