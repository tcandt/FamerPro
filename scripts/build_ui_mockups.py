import os
import shutil
import base64
import subprocess
import pymupdf

ROOT = r"E:\DATA\FarmerPro"
DIAGRAM_DIR = os.path.join(ROOT, "artifacts", "p1_2_evidence", "diagrams")
IMG_DIR = os.path.join(ROOT, "artifacts", "p1_2_evidence", "images")
os.makedirs(IMG_DIR, exist_ok=True)

# 1. Copy actual screenshots into artifacts/p1_2_evidence/images
ACTUAL_COPIES = [
    (r"screenshots\farmerpro_tabs_20261121\01_tong_quan.png", "actual_studio_01_tong_quan.png"),
    (r"screenshots\farmerpro_tabs_20261121\02_thiet_bi_android.png", "actual_studio_02_thiet_bi_android.png"),
    (r"screenshots\farmerpro_tabs_20261121\03_kho_quy_trinh_jx2.png", "actual_studio_03_kho_quy_trinh_jx2.png"),
    (r"screenshots\farmerpro_tabs_20261121\04_workflow_studio.png", "actual_studio_04_workflow_studio.png"),
    (r"screenshots\farmerpro_tabs_20261121\05_bo_nhiem_vu.png", "actual_studio_05_bo_nhiem_vu.png"),
    (r"screenshots\farmerpro_tabs_20261121\06_phien_chay.png", "actual_studio_06_phien_chay.png"),
    (r"screenshots\farmerpro_tabs_20261121\07_tai_khoan_vault.png", "actual_studio_07_tai_khoan_vault.png"),
    (r"screenshots\farmerpro_tabs_20261121\08_mang_proxy.png", "actual_studio_08_mang_proxy.png"),
    (r"screenshots\farmerpro_tabs_20261121\09_design_tokens.png", "actual_studio_09_design_tokens.png"),
    (r"screenshots\farmerpro_tabs_20261121\10_cai_dat_adb.png", "actual_studio_10_cai_dat_adb.png"),
    (r"screenshots\p1_poc\p1_1_auto_jx2_truthful.png", "actual_mini_tool_jx2_mobile.png"),
    (r"screenshots\p1_poc\p1_1_auto_slg_truthful.png", "actual_mini_tool_kingdom_slg.png"),
    (r"artifacts\p1_1_evidence\08_adb_before_screen.png", "actual_adb_before_screen.png"),
    (r"artifacts\p1_1_evidence\08b_cropped_template_system_apps.png", "actual_adb_cropped_template.png"),
    (r"artifacts\p1_1_evidence\10_adb_after_screen.png", "actual_adb_after_screen.png"),
]

for src_rel, dst_name in ACTUAL_COPIES:
    src_path = os.path.join(ROOT, src_rel)
    dst_path = os.path.join(IMG_DIR, dst_name)
    shutil.copy2(src_path, dst_path)

print(f"Copied {len(ACTUAL_COPIES)} actual implementation screenshots into {IMG_DIR}")

def b64_img(filename):
    path = os.path.join(IMG_DIR, filename)
    with open(path, "rb") as f:
        return "data:image/png;base64," + base64.b64encode(f.read()).decode("ascii")

b64_before = b64_img("actual_adb_before_screen.png")
b64_after = b64_img("actual_adb_after_screen.png")
b64_crop = b64_img("actual_adb_cropped_template.png")

def studio_shell(active_nav_idx, header_right_btns, main_content_html, title_sub="Võ Hiệp 2 (v1.0.0)"):
    nav_items = [
        ("1. Game Projects", "Workspace & Assets"),
        ("2. Workflow Studio", "React Flow & Vision"),
        ("3. Tasks & Modules", "Library, Vault, Proxy"),
        ("4. Device Lab", "ADB & ROI Cropper"),
        ("5. Test & Debug", "Traces & Evidence"),
        ("6. Mini Builder", "UI Schema & Releases"),
    ]
    nav_html = ""
    for idx, (label, sub) in enumerate(nav_items, start=1):
        cls = "nav-item active" if idx == active_nav_idx else "nav-item"
        nav_html += f'<div class="{cls}"><div class="nav-lbl">{label}</div><div class="nav-sub">{sub}</div></div>'

    return f"""
    <div class="studio-win">
      <div class="proposed-ribbon">
        <span>PROPOSED DESIGN — NOT IMPLEMENTED</span>
        <span class="ribbon-note">Bản thiết kế giao diện mục tiêu P1.2 chờ phê duyệt để triển khai ở P4 (Sử dụng dữ liệu thật từ SQLite/Artifacts làm tham chiếu bố cục)</span>
      </div>
      <div class="global-header">
        <div class="brand">
          <div class="brand-logo">FP</div>
          <div>
            <div class="brand-title">FarmerPro Studio IDE</div>
            <div class="brand-sub">Multi-Game Mini Auto-Tool Factory</div>
          </div>
        </div>
        <div class="ctx-selectors">
          <div class="ctx-pill project-pill">
            <span class="ctx-tag">ACTIVE PROJECT</span>
            <span class="ctx-val">🎮 {title_sub} ▾</span>
          </div>
          <div class="ctx-pill device-pill">
            <span class="ctx-tag">LAB DEVICE</span>
            <span class="ctx-val"><span class="dot-on"></span> emulator-5554 · Online ▾</span>
          </div>
        </div>
        <div class="hdr-actions">
          {header_right_btns}
          <span class="btn btn-ghost">⚙️ System Settings</span>
        </div>
      </div>
      <div class="studio-body">
        <div class="nav-rail">
          <div class="nav-section">PRODUCTION WORKSPACES (6)</div>
          {nav_html}
          <div class="nav-footer">
            <div class="sys-stat"><span class="dot-on"></span> ADB Portable: ./bin/adb.exe</div>
            <div class="sys-stat"><span class="dot-on"></span> SQLite WAL: farmerpro.db v2</div>
            <div class="sys-stat"><span class="dot-on"></span> Key: fp-ed25519-acb43ebd</div>
          </div>
        </div>
        <div class="workspace-main">
          {main_content_html}
        </div>
      </div>
      <div class="status-bar">
        <span>🟢 Studio Ready | Active Project: jx2-mobile (proj-jx2-mobile-001) | Lab Target: emulator-5554 (Android 14 API 34)</span>
        <span>PE Root of Trust: RT_RCDATA #7777 | Unit Tests: 34/34 PASS | Mode: P1.2 Design Blueprint</span>
      </div>
    </div>
    """

# Screen 01: New Dashboard
s01_html = studio_shell(
    1,
    '<span class="btn btn-primary">+ Tạo Game Project Mới</span>',
    """
    <div class="ws-header">
      <div>
        <div class="ws-title">ẢNH 01 — WORKSPACE DASHBOARD (TRUNG TÂM ĐIỀU HÀNH SẢN XUẤT ĐA DỰ ÁN)</div>
        <div class="ws-desc">Tổng hợp dự án game đang phát triển, workflow gần đây, thiết bị thử nghiệm, kết quả test, bản build/release, cảnh báo và truy cập nhanh.</div>
      </div>
      <div class="ws-badges">
        <span class="tag tag-blue">2 Active Game Projects</span>
        <span class="tag tag-green">1 Lab Device Online</span>
        <span class="tag tag-amber">2 Verified Releases</span>
      </div>
    </div>
    <div class="grid-3">
      <div class="panel">
        <div class="panel-hdr"><span>1. PROJECT ĐANG PHÁT TRIỂN</span><span class="tag tag-blue">SQLite v2</span></div>
        <div class="proj-mini-card active-border">
          <div class="row-between"><b>🎮 Võ Hiệp Truyền Kỳ 2 Mobile</b><span class="tag tag-green">ACTIVE</span></div>
          <div class="muted-xs">Slug: jx2-mobile · Pkg: com.blackhole.jx2mobile · v1.0.0</div>
          <div class="stat-pills"><span>10 Workflows</span><span>5 Tasks</span><span>4 PNGs</span><span>5 Modules</span></div>
        </div>
        <div class="proj-mini-card">
          <div class="row-between"><b>🏰 Kingdom Rise: Tactics SLG</b><span class="tag tag-blue">READY</span></div>
          <div class="muted-xs">Slug: kingdom-slg · Pkg: com.kingdom.rise.slg · v1.0.0</div>
          <div class="stat-pills"><span>1 Workflow</span><span>4 Tasks</span><span>3 PNGs</span><span>4 Modules</span></div>
        </div>
      </div>
      <div class="panel">
        <div class="panel-hdr"><span>2. WORKFLOW GẦN ĐÂY & TRUY CẬP NHANH</span><span class="tag tag-purple">Studio IDE</span></div>
        <table class="mini-tbl">
          <tr><th>Workflow ID</th><th>Dự án</th><th>Số Node</th><th>Trạng thái</th></tr>
          <tr><td><code>wf-jx2-live-adb-probe</code></td><td>jx2-mobile</td><td>7 nodes</td><td><span class="tag tag-green">VERIFIED ADB</span></td></tr>
          <tr><td><code>wf-jx2-main-quest</code></td><td>jx2-mobile</td><td>7 nodes</td><td><span class="tag tag-blue">VALIDATED</span></td></tr>
          <tr><td><code>wf-slg-harvest-and-patrol</code></td><td>kingdom-slg</td><td>5 nodes</td><td><span class="tag tag-blue">VALIDATED</span></td></tr>
        </table>
        <div class="quick-actions">
          <span class="btn btn-sm btn-primary">▶ Mở Workflow Studio</span>
          <span class="btn btn-sm btn-outline">✂️ Mở ROI Cropper</span>
          <span class="btn btn-sm btn-amber">🚀 Mở Mini Tool Builder</span>
        </div>
      </div>
      <div class="panel">
        <div class="panel-hdr"><span>3. THIẾT BỊ THỬ NGHIỆM (DEVICE LAB)</span><span class="tag tag-green">ONLINE</span></div>
        <div class="kv-list">
          <div><span>Serial kết nối:</span><code>emulator-5554 (LDPlayer)</code></div>
          <div><span>Model & OS:</span><code>Samsung SM_X710N · Android 14 (API 34)</code></div>
          <div><span>Độ phân giải:</span><code>1280×720 (0° Landscape/Portrait norm)</code></div>
          <div><span>Lease hiện tại:</span><code>IDLE (Generation Token #15)</code></div>
          <div><span>ADB Binary:</span><code>./bin/adb.exe (No Raw Shell Interpolation)</code></div>
        </div>
      </div>
    </div>
    <div class="grid-3" style="margin-top:10px;">
      <div class="panel">
        <div class="panel-hdr"><span>4. KẾT QUẢ TEST GẦN ĐÂY (TEST & DEBUG)</span><span class="tag tag-green">PASS</span></div>
        <div class="kv-list">
          <div><span>Job ID:</span><code>mini-job-cf950745-f43a-4dde-bc19</code></div>
          <div><span>Luồng thực thi:</span><code>wf-jx2-live-adb-probe (LIVE_ADB)</code></div>
          <div><span>Sự kiện bước:</span><code>19/19 StepEvents (0 lỗi, 0 retry)</code></div>
          <div><span>Khớp mẫu & Chạm:</span><code>score=1.000 @ (74,135) -> SafeTap(111,170)</code></div>
          <div><span>Checkpoint SQLite:</span><code>live_adb_verified (Đã lưu WAL)</code></div>
        </div>
      </div>
      <div class="panel">
        <div class="panel-hdr"><span>5. BẢN BUILD & RELEASE MỚI NHẤT</span><span class="tag tag-amber">Ed25519 + PE #7777</span></div>
        <table class="mini-tbl">
          <tr><th>File EXE</th><th>Kích thước</th><th>PE #7777</th><th>Trạng thái</th></tr>
          <tr><td><code>Auto_JX2_Mobile.exe</code></td><td>12.51 MB</td><td><code>acb43ebd..</code></td><td><span class="tag tag-green">SIGNED & VERIFIED</span></td></tr>
          <tr><td><code>Auto_Kingdom_SLG.exe</code></td><td>12.51 MB</td><td><code>acb43ebd..</code></td><td><span class="tag tag-green">SIGNED & VERIFIED</span></td></tr>
        </table>
      </div>
      <div class="panel">
        <div class="panel-hdr"><span>6. CẢNH BÁO DỰ ÁN & KIỂM TOÁN (WARNINGS)</span><span class="tag tag-amber">Pre-Flight</span></div>
        <div class="warn-item ok">✅ 0 lỗi thiếu ảnh mẫu Vision PNG trong 2 dự án</div>
        <div class="warn-item ok">✅ 5/5 kịch bản Tamper Suite bị chặn bởi PE RT_RCDATA #7777</div>
        <div class="warn-item warn">⚠️ Lưu ý kỹ thuật: Module SOCKS5 Proxy chỉ áp dụng cho kết nối TCP tương thích; lưu lượng UDP game cần tun2socks/VPN riêng.</div>
        <div class="warn-item warn">⚠️ Binary phát hành chưa ký Windows Authenticode (sẽ tích hợp signtool.exe ở giai đoạn P5).</div>
      </div>
    </div>
    """
)

# Screen 02: Game Projects
s02_html = studio_shell(
    1,
    '<span class="btn btn-outline">📋 Nhân bản Project</span><span class="btn btn-outline">✏️ Sửa Manifest</span><span class="btn btn-primary">+ Tạo Game Project</span>',
    """
    <div class="ws-header">
      <div>
        <div class="ws-title">ẢNH 02 — GAME PROJECTS (QUẢN LÝ ĐA DỰ ÁN GAME, TẠO PROJECT MỚI, ASSETS & MODULES)</div>
        <div class="ws-desc">Quy trình Tạo Project Mới: [1. Tên & Package ID] ➔ [2. Độ phân giải & Icon PE] ➔ [3. Ma trận 5-State Modules] ➔ [4. Khởi tạo projects/&lt;slug&gt;/]</div>
      </div>
      <div class="ws-badges">
        <span class="tag tag-green">Wizard Tạo Project 4 Bước</span>
        <span class="tag tag-blue">Cô lập SQLite + Assets theo Slug</span>
      </div>
    </div>
    <div class="grid-sidebar-main">
      <div class="panel">
        <div class="panel-hdr"><span>DANH SÁCH GAME PROJECTS (MULTI-GAME)</span><span class="tag tag-blue">2 Projects</span></div>
        <div class="proj-mini-card active-border">
          <div class="row-between"><b>1. Võ Hiệp Truyền Kỳ 2 Mobile</b><span class="tag tag-green">ĐANG MỞ</span></div>
          <div class="muted-xs">ID: proj-jx2-mobile-001 · Slug: jx2-mobile</div>
          <div class="muted-xs">Pkg: com.blackhole.jx2mobile · Version: 1.0.0</div>
          <div class="muted-xs">EXE: Auto_JX2_Mobile.exe · Res: 1920×1080</div>
        </div>
        <div class="proj-mini-card">
          <div class="row-between"><b>2. Kingdom Rise: Tactics SLG</b><span class="tag tag-blue">SẴN SÀNG</span></div>
          <div class="muted-xs">ID: proj-kingdom-slg-002 · Slug: kingdom-slg</div>
          <div class="muted-xs">Pkg: com.kingdom.rise.slg · Version: 1.0.0</div>
          <div class="muted-xs">EXE: Auto_Kingdom_SLG.exe · Res: 1280×720</div>
        </div>
        <div class="proj-mini-card dashed-card">
          <b>+ Tạo hoặc Nhân bản Game Project Mới</b>
          <div class="muted-xs">Khởi tạo tự động thư mục projects/&lt;slug&gt;/{workflows,templates,branding}</div>
        </div>
      </div>
      <div class="panel">
        <div class="panel-hdr"><span>CẤU HÌNH CHI TIẾT PROJECT ĐANG CHỌN: VÕ HIỆP TRUYỀN KỲ 2 MOBILE (jx2-mobile)</span><span class="tag tag-amber">project.fpproj.json</span></div>
        <div class="grid-2">
          <div class="sub-box">
            <div class="sub-title">THÔNG TIN ĐỊNH DANH & VERSION</div>
            <div class="kv-list">
              <div><span>Project ID / Slug:</span><code>proj-jx2-mobile-001 / jx2-mobile</code></div>
              <div><span>Android Game Package:</span><code>com.blackhole.jx2mobile</code></div>
              <div><span>Phiên bản (Version):</span><code>1.0.0 (Schema Version 1)</code></div>
              <div><span>Tên file EXE xuất ra:</span><code>Auto_JX2_Mobile.exe</code></div>
              <div><span>Độ phân giải chuẩn:</span><code>1920 × 1080 px (Hỗ trợ co giãn SSD)</code></div>
              <div><span>Biểu tượng PE (.ico):</span><code>projects/jx2-mobile/branding/icon.ico (7 layers)</code></div>
            </div>
          </div>
          <div class="sub-box">
            <div class="sub-title">CÁC MODULE ĐƯỢC CHỌN CHO PROJECT (5-STATE MATRIX)</div>
            <table class="mini-tbl">
              <tr><th>Module ID</th><th>Tên Module</th><th>Đóng gói</th><th>Mặc định</th></tr>
              <tr><td><code>mod.adb_core</code></td><td>ADB & Device Driver Core</td><td><span class="tag tag-green">BẬT</span></td><td>ON</td></tr>
              <tr><td><code>mod.vision_ocr</code></td><td>Multiscale Vision & WinRT OCR</td><td><span class="tag tag-green">BẬT</span></td><td>ON</td></tr>
              <tr><td><code>mod.account_vault</code></td><td>Account Vault (CRED_MGR://)</td><td><span class="tag tag-green">BẬT</span></td><td>ON</td></tr>
              <tr><td><code>mod.proxy_network</code></td><td>SOCKS5/HTTP Application Proxy</td><td><span class="tag tag-green">BẬT</span></td><td>ON</td></tr>
              <tr><td><code>mod.adb_port_forward</code></td><td>ADB Host-Device Socket Forward</td><td><span class="tag tag-red">TẮT</span></td><td>OFF</td></tr>
              <tr><td><code>mod.emulator_optimizer</code></td><td>Emulator Watchdog & Reconnect</td><td><span class="tag tag-green">BẬT</span></td><td>ON</td></tr>
            </table>
          </div>
        </div>
        <div class="sub-box" style="margin-top:8px;">
          <div class="sub-title">THƯ VIỆN TÀI NGUYÊN & ẢNH MẪU VISION CỦA PROJECT (projects/jx2-mobile/templates/*.png)</div>
          <table class="mini-tbl">
            <tr><th>Tên file ảnh mẫu (Template PNG)</th><th>Kích thước</th><th>Tọa độ cắt gốc</th><th>Ngưỡng khớp</th><th>SHA-256 Checksum</th></tr>
            <tr><td><code>templates/live_ldplayer_system_apps.png</code></td><td>88 × 88 px</td><td>(74, 135)</td><td>0.90</td><td><code>3f7a9c8e12b408d9... (Verified Live LDPlayer)</code></td></tr>
            <tr><td><code>templates/btn_quest_ tracker.png</code></td><td>64 × 28 px</td><td>(1120, 240)</td><td>0.88</td><td><code>8a21e44091bc73a1...</code></td></tr>
            <tr><td><code>templates/btn_skip_dialog.png</code></td><td>52 × 24 px</td><td>(1080, 620)</td><td>0.85</td><td><code>c41908f2331a6e9b...</code></td></tr>
            <tr><td><code>templates/icon_dungeon_entry.png</code></td><td>72 × 72 px</td><td>(940, 180)</td><td>0.88</td><td><code>71d049ab552e8c10...</code></td></tr>
          </table>
        </div>
      </div>
    </div>
    """
)

# Screen 03A: Workflow Studio (Canvas-First IDE)
s03_html = studio_shell(
    2,
    '<span class="btn btn-outline">📐 Auto Layout</span><span class="btn btn-outline">🔍 Fit View</span><span class="btn btn-outline">⏸ Breakpoint</span><span class="btn btn-outline">⏭ Step Run</span><span class="btn btn-primary">▶ Chạy Debug trên Lab</span>',
    """
    <div class="studio-3col-wide">
      <div class="panel palette-col">
        <div class="panel-hdr"><span>LUỒNG & NODE</span><span class="tag tag-blue">◀ Ẩn</span></div>
        <div class="sub-title">WORKFLOWS (jx2-mobile)</div>
        <div class="wf-list-item active">● wf-jx2-live-adb-probe</div>
        <div class="wf-list-item">○ wf-jx2-main-quest</div>
        <div class="wf-list-item">○ wf-jx2-treasure-map</div>
        <div class="sub-title" style="margin-top:8px;">NHÓM NODE KÉO THẢ</div>
        <div class="node-cat">▼ Điều khiển Luồng</div>
        <div class="node-pill">flow.start / flow.end</div>
        <div class="node-pill">logic.if_else / wait</div>
        <div class="node-cat">▼ Thị giác & WinRT OCR</div>
        <div class="node-pill purple">vision.capture_png</div>
        <div class="node-pill purple">vision.find_image</div>
        <div class="node-pill purple">vision.ocr_region</div>
        <div class="node-cat">▼ Thao tác ADB & State</div>
        <div class="node-pill green">action.random_safe_tap</div>
        <div class="node-pill amber">state.checkpoint</div>
      </div>
      <div class="canvas-col">
        <div class="canvas-toolbar">
          <span><b>CANVAS REACT FLOW (64% Bề Ngang):</b> <code>wf-jx2-live-adb-probe</code> · <span class="c-green">● Đã khớp 6/6 Nodes</span></span>
          <div class="toolbar-pills">
            <span class="tag tag-blue">1280×720: Tự ẩn Palette (&gt;72%)</span>
            <span class="tag tag-red">● BP @ n-tap-sys</span>
            <span class="tag tag-amber">⏸ Paused Step #4</span>
            <span class="tag tag-green">⏭ Step Over</span>
          </div>
        </div>
        <div class="rf-canvas" style="padding:14px 18px;">
          <div class="rf-row" style="gap:12px;">
            <div class="rf-node start-node">
              <div class="rf-hdr"><span>1. flow.start</span><span class="tag tag-blue">OK</span></div>
              <div class="rf-body">ID: <code>n-start</code><br/>Driver: <code>LIVE_ADB</code></div>
            </div>
            <div class="rf-arrow">➔</div>
            <div class="rf-node vision-node">
              <div class="rf-hdr"><span>2. vision.capture</span><span class="tag tag-green">454ms</span></div>
              <div class="rf-body">ID: <code>n-cap</code><br/>Ảnh PNG 1280×720</div>
            </div>
            <div class="rf-arrow">➔</div>
            <div class="rf-node vision-node selected-node">
              <div class="rf-hdr"><span>3. vision.find_image</span><span class="bp-dot">★ CHỌN</span></div>
              <div class="rf-body">ID: <code>n-find-sys</code><br/>Mẫu: <code>system_apps.png</code><br/>Score: <b class="c-green">1.000 ≥ 0.90</b></div>
            </div>
          </div>
          <div class="rf-row" style="margin-top:18px;gap:12px;">
            <div class="rf-node action-node executing-node">
              <div class="rf-hdr"><span>4. random_safe_tap</span><span class="bp-red">● BP</span></div>
              <div class="rf-body">ID: <code>n-tap-sys</code> · <b class="c-cyan">▶ ĐANG CHẠY</b><br/>Safe Margin: <b>18%</b> · Tap <b>(111, 170)</b></div>
            </div>
            <div class="rf-arrow">➔</div>
            <div class="rf-node vision-node">
              <div class="rf-hdr"><span>5. vision.ocr (WinRT)</span><span class="tag tag-purple">OCR</span></div>
              <div class="rf-body">ID: <code>n-ocr</code> · Biến: <code>ocr_text</code><br/><i>"Cài đặt, Files, Cửa hàng Play"</i></div>
            </div>
            <div class="rf-arrow">➔</div>
            <div class="rf-node cp-node">
              <div class="rf-hdr"><span>6. state.checkpoint</span><span class="tag tag-amber">WAL</span></div>
              <div class="rf-body">ID: <code>n-cp</code><br/>Key: <code>live_adb_verified</code></div>
            </div>
          </div>
        </div>
        <div class="exec-dock" style="height:112px;padding:6px 12px;">
          <div class="dock-hdr">
            <span><b>EXECUTION & DEBUG DOCK</b> — [● Step Trace (19)] [Biến Runtime] [Khung hình Lỗi] · <span class="muted-xs">[▼ Thu gọn thành thanh 34px]</span></span>
            <span class="tag tag-green">Job: mini-job-cf950745 · PASSED</span>
          </div>
          <div class="dock-logs" style="font-size:11px;line-height:1.45;">
            <div><code>09:42:01.513Z</code> | <b>n-find-sys</b> | <span class="c-green">verification_passed</span> | SSD score=1.000 bbox=[74, 135, 88, 88] | [📍 Chọn Node]</div>
            <div><code>09:42:01.585Z</code> | <b>n-tap-sys</b>  | <span class="c-cyan">device_ack</span>          | Safe Tap (margin 18%) -&gt; adb input tap 111 170</div>
          </div>
        </div>
      </div>
      <div class="panel inspector-col" style="padding:10px;">
        <div class="panel-hdr"><span>NODE INSPECTOR</span><span class="tag tag-blue">n-find-sys</span></div>
        <div class="kv-list">
          <div><span>Loại Node:</span><code>vision.find_image</code></div>
          <div><span>Ảnh mẫu:</span><code>system_apps.png</code></div>
          <div><span>Ngưỡng SSD:</span><code>0.90 (Đa tỉ lệ)</code></div>
          <div><span>Retry/Timeout:</span><code>3 lần · 800 ms</code></div>
          <div><span>Breakpoint:</span><code class="c-red">Tạm dừng trước Tap</code></div>
        </div>
        <div class="sub-box" style="margin-top:8px;text-align:center;">
          <div class="sub-title">LIÊN KẾT DEVICE LAB ROI (88×88 px)</div>
          <img src="__B64_CROP__" style="width:62px;height:62px;border:1.5px solid #38BDF8;border-radius:6px;margin:4px auto;display:block;" />
          <div class="muted-xs">Tọa độ gốc: (74, 135) · SHA: 3f7a9c...</div>
        </div>
        <div class="quick-actions" style="margin-top:8px;flex-direction:column;">
          <span class="btn btn-sm btn-outline">✂️ Cắt ảnh mẫu từ Device Lab</span>
          <span class="btn btn-sm btn-primary">+ Đưa Workflow vào Task Library</span>
        </div>
      </div>
    </div>
    """.replace("__B64_CROP__", b64_crop)
)

# Screen 03B: Workflow Studio — 6 Mandatory Interactive States
s03b_html = studio_shell(
    2,
    '<span class="btn btn-outline">1280×720 Compact (&gt;72% Canvas)</span><span class="btn btn-primary">1600×900 IDE (64% Canvas)</span>',
    f"""
    <div class="ws-header" style="margin-bottom:8px;padding-bottom:6px;">
      <div>
        <div class="ws-title">ẢNH 03B — ĐẶC TẢ 6 TRẠNG THÁI TƯƠNG TÁC WORKFLOW STUDIO &amp; CẦU NỐI DEVICE LAB ROI</div>
        <div class="ws-desc">Hiển thị đầy đủ hành vi Canvas ở từng trạng thái vận hành: Mặc định, Chọn Node, Đang Thực thi, Lỗi + Debug, Breakpoint/Step Run và Crop ảnh từ Lab.</div>
      </div>
      <div class="ws-badges"><span class="tag tag-purple">6 Interactive UI States</span></div>
    </div>
    <div class="grid-3" style="gap:10px;flex:1;">
      <div class="panel state-card">
        <div class="panel-hdr"><span>1. CANVAS MẶC ĐỊNH (TỐI ĐA HÓA VÙNG VẼ)</span><span class="tag tag-blue">Default</span></div>
        <div class="muted-xs">Ở <code>1280×720</code>: NavRail thu về 60px, Palette ẩn thành nút nổi <code>[+ Thêm Node]</code>, Dock thu 34px → Canvas đạt <b>&gt;72%</b> diện tích.</div>
        <div class="mini-canvas-box">
          <div class="mc-pill">[+ Node Palette]</div>
          <div class="mc-flow"><span>flow.start</span> ➔ <span>capture</span> ➔ <span>find_image</span> ➔ <span>safe_tap</span></div>
          <div class="muted-xs" style="margin-top:6px;">Dock thu gọn (34px): Sẵn sàng chạy · Fit View 100%</div>
        </div>
      </div>
      <div class="panel state-card">
        <div class="panel-hdr"><span>2. CHỌN NODE &amp; MỞ INSPECTOR</span><span class="tag tag-blue">Selected</span></div>
        <div class="muted-xs">Click Node <code>n-find-sys</code> bật viền sáng xanh <code>#38BDF8</code> và tự mở Inspector bên phải để chỉnh tham số &amp; gán vào Task.</div>
        <div class="mini-canvas-box">
          <div class="mc-node mc-selected"><b>n-find-sys</b> · Confidence 0.90 · Margin 18%</div>
          <div class="mc-insp-mini">Inspector: Đổi mẫu PNG · Retry=3 · <b>[+ Đưa vào Task Library]</b></div>
        </div>
      </div>
      <div class="panel state-card">
        <div class="panel-hdr"><span>3. NODE ĐANG THỰC THI (LIVE RUN)</span><span class="tag tag-green">Executing</span></div>
        <div class="muted-xs">Node đang chạy có viền cyan phát sáng + badge thời gian thực, đường nối <code>edge</code> đổi màu chạy dữ liệu sang bước kế tiếp.</div>
        <div class="mini-canvas-box">
          <div class="mc-node mc-running">▶ RUNNING: <b>n-tap-sys</b> · Latency 52ms</div>
          <div class="mc-dock-mini c-green">Trace: capture [OK] ➔ find_image [1.000] ➔ safe_tap [RUNNING]</div>
        </div>
      </div>
      <div class="panel state-card">
        <div class="panel-hdr"><span>4. WORKFLOW BỊ LỖI &amp; MỞ DEBUG</span><span class="tag tag-red">Error + Debug</span></div>
        <div class="muted-xs">Node thất bại chuyển viền đỏ <code>#F27883</code>, mở khay Error Debug hiển thị ảnh chụp màn hình tại thời điểm lỗi.</div>
        <div class="mini-canvas-box" style="border-color:#F27883;">
          <div class="mc-node mc-error">✕ FAILED: <b>n-find-btn</b> · SSD Score 0.41 &lt; 0.85</div>
          <div class="mc-insp-mini" style="color:#FCA5A5;">[🔍 So sánh ảnh Lỗi] · [📍 Mở Test &amp; Debug] · [✂️ Chụp mẫu lại]</div>
        </div>
      </div>
      <div class="panel state-card">
        <div class="panel-hdr"><span>5. ĐẶT BREAKPOINT &amp; STEP RUN</span><span class="tag tag-amber">Breakpoint</span></div>
        <div class="muted-xs">Gắn chấm đỏ <code>● BP</code> trước Node hành động nguy hiểm; luồng dừng lại chờ người dùng kiểm tra biến trước khi Tap.</div>
        <div class="mini-canvas-box" style="border-color:#E7B15B;">
          <div class="mc-node mc-bp">● PAUSED @ <b>n-tap-sys</b> · target=(111,170)</div>
          <div class="mc-insp-mini" style="color:#FCD34D;">[⏭ Step Over] · [▶ Continue] · [⏹ Stop] · Biến: target_found=true</div>
        </div>
      </div>
      <div class="panel state-card">
        <div class="panel-hdr"><span>6. CROP TỪ DEVICE LAB → VISION NODE</span><span class="tag tag-purple">Lab Bridge</span></div>
        <div class="muted-xs">Bấm <code>[✂️ Cắt mẫu từ Lab]</code> trên Node mở ảnh <code>emulator-5554</code>, kéo vùng ROI lưu vào Project &amp; tự gán lại vào Node.</div>
        <div class="mini-canvas-box" style="display:flex;align-items:center;gap:10px;">
          <img src="{b64_crop}" style="width:52px;height:52px;border:2px solid #3BC38D;border-radius:6px;" />
          <div style="text-align:left;font-size:11px;">
            <div><b>ROI:</b> [74, 135, 88, 88]</div>
            <div><b>Lưu:</b> <code>templates/system_apps.png</code></div>
            <div class="c-green">✓ Đã gắn tự động vào Node <code>n-find-sys</code></div>
          </div>
        </div>
      </div>
    </div>
    """
)

# Screen 04: Tasks & Modules
s04_html = studio_shell(
    3,
    '<span class="btn btn-outline">+ Thêm Nhóm Danh Mục</span><span class="btn btn-primary">+ Tạo Task từ Workflow</span>',
    """
    <div class="ws-header">
      <div>
        <div class="ws-title">ẢNH 04 — TASKS & MODULES (THƯ VIỆN NHIỆM VỤ THEO GAME, CẤU HÌNH THAM SỐ & MODULE DỰ ÁN)</div>
        <div class="ws-desc">Phân nhóm nhiệm vụ động, ánh xạ Workflow -> Task, định nghĩa tham số người dùng, thứ tự ưu tiên Scheduler và cấu hình Vault/Proxy/Utilities.</div>
      </div>
      <div class="ws-badges">
        <span class="tag tag-blue">● Tab 1: Task Library (5)</span>
        <span class="tag tag-purple">Tab 2: Vault (CRED_MGR://)</span>
        <span class="tag tag-amber">Tab 3: Proxy & ADB Socket</span>
        <span class="tag tag-green">Tab 4: Scheduler & Watchdog</span>
      </div>
    </div>
    <div class="grid-sidebar-main">
      <div class="panel">
        <div class="panel-hdr"><span>THỨ TỰ THỰC THI & ÁNH XẠ WORKFLOW (TASK LIBRARY)</span><span class="tag tag-green">Đồng bộ ui-schema.json</span></div>
        <table class="mini-tbl">
          <tr><th>Thứ tự</th><th>Nhóm</th><th>Tên Nhiệm vụ hiển thị trên Mini Tool</th><th>Workflow Mapping</th><th>Tham số</th><th>Trạng thái</th></tr>
          <tr class="active-row"><td><b>#1 ▲▼</b></td><td><span class="tag tag-amber">[NV]</span></td><td><b>Chính Tuyến & Đối thoại Dã Tẩu</b></td><td><code>wf-jx2-main-quest</code></td><td>2 fields</td><td><span class="tag tag-green">Mặc định BẬT</span></td></tr>
          <tr><td><b>#2 ▲▼</b></td><td><span class="tag tag-amber">[NV]</span></td><td>Kiểm tra Thiết bị ADB Thật + OCR</td><td><code>wf-jx2-live-adb-probe</code></td><td>0 fields</td><td><span class="tag tag-green">VERIFIED ADB</span></td></tr>
          <tr><td><b>#3 ▲▼</b></td><td><span class="tag tag-blue">[NV]</span></td><td>Tìm Kho Báu & Đào Tàng Bảo Đồ</td><td><code>wf-jx2-main-quest</code></td><td>1 field</td><td><span class="tag tag-green">Mặc định BẬT</span></td></tr>
          <tr><td><b>#4 ▲▼</b></td><td><span class="tag tag-purple">[PB]</span></td><td>Sơn Tặc Mật Quật & Bí Cảnh Tổ Đội</td><td><code>wf-jx2-main-quest</code></td><td>1 field</td><td><span class="tag tag-green">Mặc định BẬT</span></td></tr>
          <tr><td><b>#5 ▲▼</b></td><td><span class="tag tag-red">[HĐ]</span></td><td>Lửa Trại Bang & Chiến Trường Tống Kim</td><td><code>wf-jx2-main-quest</code></td><td>0 fields</td><td><span class="tag tag-blue">Tùy chọn</span></td></tr>
        </table>
        <div class="sub-box" style="margin-top:8px;">
          <div class="sub-title">CẤU HÌNH THAM SỐ ĐỘNG CHO TASK #1 (settingsSchema -> Tự động sinh Form trên Mini Tool)</div>
          <table class="mini-tbl">
            <tr><th>Key biến</th><th>Kiểu dữ liệu</th><th>Nhãn hiển thị người dùng</th><th>Giá trị mặc định</th></tr>
            <tr><td><code>auto_skip_dialog</code></td><td><code>boolean (Checkbox)</code></td><td>Tự động bỏ qua hội thoại NPC</td><td><code>true</code></td></tr>
            <tr><td><code>max_quest_loops</code></td><td><code>number (1..50)</code></td><td>Số vòng nhiệm vụ Dã Tẩu tối đa</td><td><code>10</code></td></tr>
          </table>
        </div>
      </div>
      <div class="panel">
        <div class="panel-hdr"><span>CẤU HÌNH MODULE: VAULT, PROXY, SCHEDULER & UTILITIES</span><span class="tag tag-purple">Scoped to jx2-mobile</span></div>
        <div class="sub-box">
          <div class="sub-title">1. MODULE ACCOUNT VAULT (mod.account_vault)</div>
          <div class="muted-xs">Lưu tham chiếu khóa bảo mật Windows Credential Manager (Không lưu mật khẩu dạng rõ trong SQLite):</div>
          <div class="code-line">CRED_MGR://FarmerPro/jx2-mobile/acc_caibang_01 (Trạng thái: READY)</div>
        </div>
        <div class="sub-box" style="margin-top:6px;">
          <div class="sub-title">2. MODULE PROXY & MẠNG (mod.proxy_network & mod.adb_port_forward)</div>
          <div class="muted-xs">Kiểm tra độ trễ SOCKS5/HTTP Proxy & quy tắc <code>adb forward</code> (Bảo toàn 100% từ NetworkView cũ):</div>
          <div class="warn-item warn" style="margin-top:4px;">⚠️ Ghi chú kỹ thuật trung thực: SOCKS5/HTTP chỉ áp dụng cho kết nối TCP tương thích; lưu lượng UDP thô của game cần tun2socks/VPN riêng.</div>
        </div>
        <div class="sub-box" style="margin-top:6px;">
          <div class="sub-title">3. SCHEDULER & UTILITIES (mod.emulator_optimizer)</div>
          <div class="kv-list">
            <div><span>Chính sách Lease:</span><code>Exclusive Per-Device Lease + Generation Fencing</code></div>
            <div><span>Tiện ích phụ (Utilities):</span><code>Tự kết nối lại ADB · Tự mở lại App khi văng</code></div>
          </div>
        </div>
      </div>
    </div>
    """
)

# Screen 05: Device Lab
s05_html = studio_shell(
    4,
    '<span class="btn btn-outline">🔄 Quét lại ADB</span><span class="btn btn-outline">🌐 Quét dải IP LAN</span><span class="btn btn-primary">📸 Chụp màn hình PNG mới</span>',
    f"""
    <div class="ws-header">
      <div>
        <div class="ws-title">ẢNH 05 — DEVICE LAB (ADB DISCOVERY, LDPLAYER/NOX/BLUESTACKS, LIVE SCREENSHOT & CÔNG CỤ CẮT MẪU VISION)</div>
        <div class="ws-desc">Quản lý tập trung kết nối thiết bị thật & giả lập, chẩn đoán phần cứng, chụp màn hình theo yêu cầu và kéo cắt ảnh mẫu ROI lưu thẳng vào Project.</div>
      </div>
    </div>
    <div class="grid-sidebar-main">
      <div class="panel">
        <div class="panel-hdr"><span>1. ADB DISCOVERY & GIẢ LẬP</span><span class="tag tag-green">1 ONLINE</span></div>
        <div class="proj-mini-card active-border">
          <div class="row-between"><b>🟢 emulator-5554 (SM_X710N)</b><span class="tag tag-green">CONNECTED</span></div>
          <div class="muted-xs">Android 14 (API 34) · 720×1280 / 1280×720 · Rotation: 0°</div>
          <div class="muted-xs">Driver: LIVE_ADB(emulator-5554) · Latency: 18 ms</div>
        </div>
        <div class="sub-box" style="margin-top:6px;">
          <div class="sub-title">ĐIỀU KHIỂN GIẢ LẬP (LDPLAYER / NOX / BLUESTACKS)</div>
          <div class="kv-list">
            <div><span>Instance #0:</span><code>LDPlayer-Main (PID 14820 · Running)</code></div>
            <div><span>Thao tác nhanh:</span><code>[Khởi động] [Tắt] [Khởi động lại]</code></div>
            <div><span>Kết nối TCP/IP:</span><code>127.0.0.1:5555 / Quét LAN 192.168.1.0/24</code></div>
          </div>
        </div>
        <div class="sub-box" style="margin-top:6px;">
          <div class="sub-title">CHẨN ĐOÁN THIẾT BỊ & PACKAGE MANAGER</div>
          <div class="kv-list">
            <div><span>ADB Binary:</span><code>E:\\DATA\\FarmerPro\\bin\\adb.exe (Verified)</code></div>
            <div><span>wm size / density:</span><code>720x1280 · 320 dpi · 0 deg</code></div>
            <div><span>WinRT OCR Sandbox:</span><code>Sẵn sàng (Tiếng Việt / Tiếng Anh)</code></div>
          </div>
        </div>
      </div>
      <div class="panel">
        <div class="panel-hdr"><span>2. LIVE SCREENSHOT & CÔNG CỤ CHỤP/CẮT ẢNH MẪU VISION (ROI TEMPLATE CROPPER)</span><span class="tag tag-purple">Ảnh chụp thật từ emulator-5554</span></div>
        <div class="grid-2">
          <div class="crop-preview-box">
            <img src="{b64_before}" class="live-shot-img" />
            <div class="roi-overlay">
              <span class="roi-tag">ROI: (74, 135) · 88×88 px</span>
            </div>
          </div>
          <div>
            <div class="sub-box">
              <div class="sub-title">KẾT QUẢ CẮT MẪU & KIỂM THỬ SO KHỚP SSD TRỰC TIẾP</div>
              <div style="display:flex;align-items:center;gap:12px;margin:8px 0;">
                <img src="{b64_crop}" style="width:70px;height:70px;border:2px solid #3BC38D;border-radius:6px;" />
                <div class="kv-list" style="flex:1;">
                  <div><span>Tên file lưu:</span><code>live_ldplayer_system_apps.png</code></div>
                  <div><span>Tọa độ vùng cắt:</span><code>x=74, y=135, w=88, h=88</code></div>
                  <div><span>Điểm khớp SSD:</span><code class="c-green">1.000 (Confidence >= 0.90)</code></div>
                  <div><span>Điểm chạm Safe Tap:</span><code class="c-cyan">(111, 170) [Margin 18%]</code></div>
                </div>
              </div>
              <div class="quick-actions">
                <span class="btn btn-sm btn-primary">💾 Lưu vào Project Templates</span>
                <span class="btn btn-sm btn-outline">🔍 Chạy thử WinRT OCR trên khung hình</span>
              </div>
            </div>
            <div class="sub-box" style="margin-top:8px;">
              <div class="sub-title">KẾT QUẢ QUÉT CHỮ WINRT OCR TRÊN KHUNG HÌNH</div>
              <div class="code-line">"16:42 System Apps Search games ... Võ Hiệp Truyền Kỳ 2 Cài đặt Files ... Cửa hàng Play Chrome Thư viện"</div>
            </div>
          </div>
        </div>
      </div>
    </div>
    """
)

# Screen 06: Test & Debug
s06_html = studio_shell(
    5,
    '<span class="btn btn-outline">🔄 Khôi phục Checkpoint</span><span class="btn btn-danger">■ Dừng khẩn cấp</span><span class="btn btn-primary">▶ Chạy Phiên Kiểm Thử Mới</span>',
    f"""
    <div class="ws-header">
      <div>
        <div class="ws-title">ẢNH 06 — TEST & DEBUG (PHIÊN KIỂM THỬ, TIMELINE, NODE TRACE, ẢNH BẰNG CHỨNG TRƯỚC/SAU & ERROR INSPECTOR)</div>
        <div class="ws-desc">Truy vấn 100% từ bảng SQLite step_events thực tế (đã loại bỏ hoàn toàn job-demo-8492 giả lập), đối chiếu ảnh chụp Trước/Sau và tọa độ Safe Tap.</div>
      </div>
      <div class="ws-badges">
        <span class="tag tag-blue">Lọc: [emulator-5554 ▾] [Tất cả / Lỗi / Checkpoint ▾]</span>
        <span class="tag tag-green">Phiên: mini-job-cf950745 · PASS</span>
      </div>
    </div>
    <div class="grid-sidebar-main">
      <div class="panel">
        <div class="panel-hdr"><span>1. TIMELINE NODE TRACE (TRUY VẤN SQLITE WAL)</span><span class="tag tag-purple">1-Click Nhảy tới Node Lỗi</span></div>
        <table class="mini-tbl">
          <tr><th>Bước</th><th>Node ID</th><th>Loại Sự kiện</th><th>Thời gian (UTC)</th><th>Chi tiết Payload / Kết quả</th></tr>
          <tr><td>#1-2</td><td><code>n-start</code></td><td><span class="tag tag-blue">device_ack</span></td><td>09:42:01.057</td><td>Khởi tạo phiên chạy trên <code>emulator-5554</code></td></tr>
          <tr><td>#3-4</td><td><code>n-cap</code></td><td><span class="tag tag-blue">device_ack</span></td><td>09:42:01.512</td><td>Chụp screencap PNG (720×1280 · 454 ms)</td></tr>
          <tr><td>#5-6</td><td><code>n-find-sys</code></td><td><span class="tag tag-green">verification_passed</span></td><td>09:42:01.533</td><td>Khớp mẫu <code>system_apps.png</code> score=1.00 @ (74,135)</td></tr>
          <tr class="active-row"><td><b>#7-8</b></td><td><code>n-tap-sys</code></td><td><span class="tag tag-green">device_ack</span></td><td><b>09:42:01.585</b></td><td><b>Safe Random Tap (margin 18%) -> tap(111, 170)</b></td></tr>
          <tr><td>#9-12</td><td><code>n-cap-after</code></td><td><span class="tag tag-blue">device_ack</span></td><td>09:42:03.145</td><td>Chờ 1200ms & chụp ảnh xác nhận sau thao tác</td></tr>
          <tr><td>#13-14</td><td><code>n-ocr</code></td><td><span class="tag tag-green">verification_passed</span></td><td>09:42:04.727</td><td>WinRT OCR trích xuất tên các biểu tượng trong thư mục</td></tr>
          <tr><td>#15-17</td><td><code>n-cp</code></td><td><span class="tag tag-amber">checkpoint_saved</span></td><td>09:42:04.729</td><td>Lưu Checkpoint <code>live_adb_verified</code> vào SQLite WAL</td></tr>
          <tr><td>#18-19</td><td><code>n-end</code></td><td><span class="tag tag-green">Succeeded</span></td><td>09:42:04.731</td><td>Hoàn tất toàn bộ quy trình (Tổng thời gian: 3.67s)</td></tr>
        </table>
        <div class="sub-box" style="margin-top:6px;">
          <div class="sub-title">CHECKPOINT / RETRY & ERROR INSPECTOR</div>
          <div class="kv-list">
            <div><span>Checkpoint đã lưu:</span><code>live_adb_verified (target_x=74, target_y=135, target_found=true)</code></div>
            <div><span>Error Inspector:</span><code class="c-green">0 lỗi trong phiên này (Khi có lỗi sẽ hỗ trợ 1-Click nhảy tới Node trên Canvas)</code></div>
          </div>
        </div>
      </div>
      <div class="panel">
        <div class="panel-hdr"><span>2. BẰNG CHỨNG THỊ GIÁC TRƯỚC / SAU THAO TÁC ADB THẬT</span><span class="tag tag-purple">emulator-5554</span></div>
        <div class="grid-2">
          <div class="evidence-shot-card">
            <div class="sub-title">ẢNH TRƯỚC KHI TAP (08_adb_before_screen.png)</div>
            <img src="{b64_before}" class="ev-img" />
            <div class="muted-xs">Phát hiện thư mục <b>System Apps</b> tại (74, 135) · Điểm chạm Safe Tap: <b>(111, 170)</b></div>
          </div>
          <div class="evidence-shot-card">
            <div class="sub-title">ẢNH SAU KHI TAP (10_adb_after_screen.png)</div>
            <img src="{b64_after}" class="ev-img" />
            <div class="muted-xs">Hộp thoại <b>System Apps</b> đã mở thật trên LDPlayer · WinRT OCR đọc thấy "Cài đặt", "Files", "Cửa hàng Play"</div>
          </div>
        </div>
      </div>
    </div>
    """
)

# Screen 07A: Visual No-Code Mini Tool Builder (3-Pane WYSIWYG)
s07_html = studio_shell(
    6,
    '<span class="btn btn-outline">💾 Lưu Schema v1.2</span><span class="btn btn-outline">🔍 Pre-Flight (6/6 PASS)</span><span class="btn btn-amber">🚀 Xuất Mini Tool (.EXE)</span>',
    """
    <div class="ws-header" style="margin-bottom:8px;padding-bottom:6px;">
      <div>
        <div class="ws-title">ẢNH 07A — MINI TOOL BUILDER (TRÌNH DỰNG GIAO DIỆN TRỰC QUAN WYSIWYG — KHÔNG SỬA JSON TAY)</div>
        <div class="ws-desc">Kéo thả sắp xếp khu vực UI, chọn/ẩn nhiệm vụ, click trực tiếp vào thành phần trên Preview để chỉnh thuộc tính &amp; liên kết biến Runtime.</div>
      </div>
      <div class="ws-badges">
        <span class="tag tag-blue">Layout: [1 Cột] [● 2 Cột] [Expanded]</span>
        <span class="tag tag-green">Preview: [● Online] [Offline] [Running] [Error]</span>
      </div>
    </div>
    <div class="builder-3col">
      <div class="panel" style="padding:10px;">
        <div class="panel-hdr"><span>1. KHU VỰC UI &amp; NHIỆM VỤ</span><span class="tag tag-blue">Kéo thả</span></div>
        <div class="sub-title">THỨ TỰ KHU VỰC GIAO DIỆN</div>
        <div class="blk-row"><span>⋮⋮ [1] Header Thương hiệu</span><span>👁 🔒</span></div>
        <div class="blk-row blk-active"><span>⋮⋮ [2] Bảng Thiết bị &amp; OCR</span><span>👁 🗑</span></div>
        <div class="blk-row"><span>⋮⋮ [3] Danh sách Nhiệm vụ</span><span>👁 🗑</span></div>
        <div class="blk-row"><span>⋮⋮ [4] Nhật ký Hoạt động</span><span>👁 🗑</span></div>
        <div class="blk-row"><span>⋮⋮ [5] Thanh Hỗ trợ &amp; Cài đặt</span><span>👁 🗑</span></div>
        <div class="btn btn-sm btn-outline" style="text-align:center;margin:4px 0 8px 0;">+ Thêm Khu vực UI Mới</div>
        <div class="sub-title">CHỌN NHIỆM VỤ ĐƯA VÀO TOOL (4/5)</div>
        <div class="mp-task">☑ <span class="tag tag-amber">[NV]</span> Chính Tuyến &amp; Dã Tẩu</div>
        <div class="mp-task">☑ <span class="tag tag-amber">[NV]</span> Kiểm tra ADB + OCR</div>
        <div class="mp-task">☑ <span class="tag tag-amber">[NV]</span> Tìm Kho Báu</div>
        <div class="mp-task" style="opacity:0.55;">☐ <span class="tag tag-purple">[PB]</span> Bí Cảnh Tổ Đội (Đã ẩn)</div>
      </div>
      <div class="panel" style="padding:10px;background:#0C121B;">
        <div class="panel-hdr"><span>2. INTERACTIVE VISUAL PREVIEW (CLICK THÀNH PHẦN ĐỂ CHỈNH)</span><span class="tag tag-amber">ui-schema v1.2</span></div>
        <div class="mini-preview-frame" style="flex:1;">
          <div class="mp-hdr">
            <div><b>⚔️ Auto Võ Hiệp 2 Mobile v1.0.0</b> <span class="mp-badge">JX2 v1.0</span></div>
            <div class="muted-xs">Ed25519 Verified (PE #7777)</div>
          </div>
          <div class="selected-preview-box">
            <div class="sel-pill-tag">Đang chọn: DeviceGrid ➔ Cột OCR #2 ("Vàng")</div>
            <table class="mini-tbl" style="margin:4px 0;">
              <tr><th>☑</th><th>Thiết bị ADB</th><th>Trạng thái</th><th>Cấp (OCR)</th><th style="background:#3B2D17;color:#FCD34D;border:1.5px solid #F59E0B;">★ Vàng (OCR)</th><th>Bạc (OCR)</th></tr>
              <tr><td>☑</td><td><code>emulator-5554</code></td><td><span class="tag tag-green">ONLINE</span></td><td>Chưa OCR (-)</td><td style="background:#261D10;"><b>Chưa OCR (-)</b></td><td>Chưa OCR (-)</td></tr>
            </table>
          </div>
          <div class="grid-2" style="gap:8px;margin-top:8px;">
            <div class="sub-box">
              <div class="sub-title">ĐIỀU KHIỂN NHIỆM VỤ (4 TASKS)</div>
              <div class="mp-task">☑ [NV] Chính Tuyến &amp; Dã Tẩu ⚙</div>
              <div class="mp-task">☑ [NV] Kiểm tra Thiết bị ADB + OCR</div>
              <div class="mp-task">☑ [NV] Tìm Kho Báu &amp; Đào Bảo Đồ</div>
            </div>
            <div class="sub-box">
              <div class="sub-title">HOẠT ĐỘNG &amp; TRẠNG THÁI RUNTIME</div>
              <div class="code-line">[READY] Khóa PE #7777 hợp lệ. Kết nối emulator-5554 OK.</div>
              <div class="muted-xs" style="margin-top:6px;">Nút chạy: <span class="btn btn-sm btn-primary">▶ Bắt đầu Auto</span></div>
            </div>
          </div>
        </div>
        <div class="warn-item ok" style="margin-top:6px;margin-bottom:0;">✅ Pre-Flight Audit 6/6 PASS · Không cần sửa JSON tay khi cấu hình thông thường.</div>
      </div>
      <div class="panel" style="padding:10px;">
        <div class="panel-hdr"><span>3. THUỘC TÍNH THÀNH PHẦN</span><span class="tag tag-amber">Cột #2: Vàng</span></div>
        <div class="form-grp"><label>Nhãn hiển thị (Column Label)</label><div class="fake-input">Vàng</div></div>
        <div class="form-grp"><label>Liên kết Biến Runtime (Data Binding)</label><div class="fake-input c-cyan">runtime.ocr.char_gold ▾</div></div>
        <div class="form-grp"><label>Giá trị mặc định (Khi chưa OCR)</label><div class="fake-input">Chưa OCR (-)</div></div>
        <div class="form-grp"><label>Màu nhấn Thương hiệu (Branding)</label><div class="fake-input"><span style="color:#F59E0B;">■</span> #F59E0B (Võ Hiệp Amber) ▾</div></div>
        <div class="form-grp"><label>Module Đóng gói (5-State Matrix)</label><div class="muted-xs">☑ adb_core · ☑ vision_ocr · ☑ account_vault · ☑ proxy · ☐ port_fwd</div></div>
        <div class="quick-actions" style="flex-direction:column;margin-top:auto;">
          <span class="btn btn-sm btn-outline">💾 Lưu Phiên bản UI Schema (v1.2)</span>
          <span class="btn btn-sm btn-amber">🚀 Build &amp; Xuất Auto_JX2_Mobile.exe</span>
          <div class="muted-xs" style="text-align:center;">&lt;/&gt; Xem JSON đọc-chỉ (Tùy chọn Dev nâng cao)</div>
        </div>
      </div>
    </div>
    """
)

# Screen 07B: Mini Tool Builder — 3 Layout Modes & 4 Runtime Preview States
s07b_html = studio_shell(
    6,
    '<span class="btn btn-outline">3 Kiểu Bố cục (1 Cột / 2 Cột / Expanded)</span><span class="btn btn-primary">4 Trạng thái Preview Trực quan</span>',
    """
    <div class="ws-header" style="margin-bottom:8px;padding-bottom:6px;">
      <div>
        <div class="ws-title">ẢNH 07B — CHẾ ĐỘ XEM TRƯỚC BỐ CỤC (1 CỘT / 2 CỘT / EXPANDED) &amp; 4 TRẠNG THÁI RUNTIME MINI TOOL</div>
        <div class="ws-desc">Kiểm chứng ngay trong Mini Tool Builder cách giao diện thay đổi theo kích thước cửa sổ và 4 trạng thái thực thi (Online / Offline / Running / Error).</div>
      </div>
    </div>
    <div class="sub-title" style="margin-bottom:6px;">A. CHUYỂN ĐỔI 3 KIỂU BỐ CỤC (LAYOUT PRESETS) TRÊN CÙNG MỘT UI-SCHEMA</div>
    <div class="grid-3" style="gap:10px;margin-bottom:10px;">
      <div class="panel state-card">
        <div class="panel-hdr"><span>1. COMPACT 1 CỘT (520×660)</span><span class="tag tag-blue">Siêu gọn</span></div>
        <div class="mini-canvas-box" style="text-align:left;">
          <div class="mc-hdr-bar">Header + Chọn Máy ADB</div>
          <div class="mc-box-row">Danh sách Nhiệm vụ (Chồng dọc)</div>
          <div class="mc-box-row">Nhật ký Log Thu gọn (3 dòng)</div>
        </div>
      </div>
      <div class="panel state-card" style="border-color:#F59E0B;">
        <div class="panel-hdr"><span>2. BỐ CỤC 2 CỘT (740×660 · MẶC ĐỊNH)</span><span class="tag tag-amber">Standard</span></div>
        <div class="mini-canvas-box" style="text-align:left;">
          <div class="mc-hdr-bar">Header + Bảng Máy &amp; Cột OCR</div>
          <div class="grid-2" style="gap:6px;"><div class="mc-box-row">Trái: Nhiệm vụ</div><div class="mc-box-row">Phải: Hoạt động / Log</div></div>
        </div>
      </div>
      <div class="panel state-card">
        <div class="panel-hdr"><span>3. EXPANDED WORKSTATION (1020×700)</span><span class="tag tag-green">Mở rộng</span></div>
        <div class="mini-canvas-box" style="text-align:left;">
          <div class="mc-hdr-bar">Header + Bảng Đa Thiết Bị Đầy Đủ</div>
          <div class="grid-3" style="gap:6px;"><div class="mc-box-row">Nhiệm vụ</div><div class="mc-box-row">Tham số Task</div><div class="mc-box-row">Log + Trợ giúp</div></div>
        </div>
      </div>
    </div>
    <div class="sub-title" style="margin-bottom:6px;">B. XEM TRƯỚC 4 TRẠNG THÁI RUNTIME TRONG LIVE PREVIEW</div>
    <div class="grid-2" style="gap:10px;flex:1;">
      <div class="grid-2" style="gap:10px;">
        <div class="panel state-card">
          <div class="panel-hdr"><span>1. ONLINE (SẴN SÀNG)</span><span class="tag tag-green">ONLINE</span></div>
          <div class="muted-xs"><code>emulator-5554</code> kết nối OK · Các cột OCR hiển thị trung thực <code>"Chưa OCR (-)"</code> trước khi quét.</div>
          <div class="code-line">[READY] PE #7777 hợp lệ · Bấm Bắt đầu để chạy.</div>
        </div>
        <div class="panel state-card">
          <div class="panel-hdr"><span>2. OFFLINE (CHƯA CÓ ADB)</span><span class="tag tag-amber">OFFLINE</span></div>
          <div class="muted-xs">Nút Chạy tự động khóa an toàn; hiển thị hướng dẫn bật USB Debugging / mở LDPlayer kèm nút <code>Quét lại</code>.</div>
          <div class="code-line" style="color:#E7B15B;">[WAITING] Không tìm thấy thiết bị ADB online.</div>
        </div>
      </div>
      <div class="grid-2" style="gap:10px;">
        <div class="panel state-card">
          <div class="panel-hdr"><span>3. RUNNING (ĐANG THỰC THI)</span><span class="tag tag-blue">RUNNING</span></div>
          <div class="muted-xs">Hiển thị nhiệm vụ đang chạy, bước hiện tại, thanh tiến độ và nút <code>Dừng an toàn (Cancel)</code>.</div>
          <div class="code-line">[RUN] Task #1 (Bước 4/6) · SafeTap(111,170)</div>
        </div>
        <div class="panel state-card" style="border-color:#F27883;">
          <div class="panel-hdr"><span>4. ERROR (LỖI / FAIL-CLOSED)</span><span class="tag tag-red">ERROR</span></div>
          <div class="muted-xs">Hiển thị rõ mã lỗi (<code>ED25519_PUBKEY_UNTRUSTED</code> hoặc mất kết nối ADB giữa chừng) và chặn thực thi.</div>
          <div class="code-line" style="color:#FCA5A5;">[BLOCKED] Chữ ký bundle bị sai lệch — Đã dừng!</div>
        </div>
      </div>
    </div>
    """
)

# Screen 08: Release Management
s08_html = studio_shell(
    6,
    '<span class="btn btn-outline">🧪 Chạy Smoke Test & Tamper Suite</span><span class="btn btn-primary">📦 Xuất Portable ZIP</span>',
    """
    <div class="ws-header">
      <div>
        <div class="ws-title">ẢNH 08 — RELEASE MANAGEMENT (QUẢN LÝ BẢN PHÁT HÀNH, SHA-256, CHỮ KÝ ED25519, PE #7777 & BUILD REPORT)</div>
        <div class="ws-desc">Theo dõi danh sách các bản phát hành Mini Tool đã xuất, trạng thái 8 bước đóng gói, kiểm toán phụ thuộc, chữ ký số và đường dẫn EXE/ZIP.</div>
      </div>
    </div>
    <div class="panel">
      <div class="panel-hdr"><span>1. DANH SÁCH BẢN PHÁT HÀNH ĐÃ XUẤT (RELEASE HISTORY)</span><span class="tag tag-green">2 Verified Packages</span></div>
      <table class="mini-tbl">
        <tr><th>Release ID / Dự án</th><th>Version</th><th>File Thực Thi & Đường Dẫn</th><th>Kích thước</th><th>SHA-256 Binary</th><th>PE #7777 Root Key</th><th>Build Status</th></tr>
        <tr class="active-row">
          <td><b>rel-jx2-1.0.0</b> (jx2-mobile)</td>
          <td><code>v1.0.0</code></td>
          <td><code>releases/poc/Auto_JX2_Mobile/Auto_JX2_Mobile.exe</code></td>
          <td>12,515,840 B</td>
          <td><code>55c051cba31c269c7d307301...</code></td>
          <td><code>acb43ebd9ab499f0... (#7777)</code></td>
          <td><span class="tag tag-green">8/8 PASS · Đo: 1097ms</span></td>
        </tr>
        <tr>
          <td><b>rel-slg-1.0.0</b> (kingdom-slg)</td>
          <td><code>v1.0.0</code></td>
          <td><code>releases/poc/Auto_Kingdom_SLG/Auto_Kingdom_SLG.exe</code></td>
          <td>12,516,352 B</td>
          <td><code>95732232077c0747fe611def...</code></td>
          <td><code>acb43ebd9ab499f0... (#7777)</code></td>
          <td><span class="tag tag-green">8/8 PASS · Đo: 1127ms</span></td>
        </tr>
      </table>
    </div>
    <div class="grid-2" style="margin-top:8px;">
      <div class="panel">
        <div class="panel-hdr"><span>2. BÁO CÁO PIPELINE 8 BƯỚC & CHỮ KÝ BUNDLE (build-report.json)</span><span class="tag tag-amber">Ed25519-RFC8032</span></div>
        <div class="kv-list">
          <div><span>Step 1-3 (Validate & 5-State):</span><code class="c-green">PASS · 2 Workflows, 4 PNGs, 5 Permitted Modules</code></div>
          <div><span>Step 4-5 (Copy & PE Inject):</span><code class="c-green">PASS · 7 Icon Layers + PE RT_RCDATA #7777 PubKey</code></div>
          <div><span>Step 6-7 (Bundle & Sign):</span><code class="c-green">PASS · Signed 7 files (KeyID: fp-ed25519-acb43ebd9ab4)</code></div>
          <div><span>Step 8 (Verify via PE #7777):</span><code class="c-green">PASS · Verified against PE-embedded public key</code></div>
          <div><span>Bundle Signature (Hex):</span><code>7fce5be7be13ac4ea90a25948b8b80c09e068150c94e4d92...</code></div>
          <div><span>Authenticode Status:</span><code class="c-amber">Unsigned Portable PE (Sẵn sàng ký signtool.exe ở P5)</code></div>
        </div>
      </div>
      <div class="panel">
        <div class="panel-hdr"><span>3. KIỂM TOÁN PHỤ THUỘC ĐỘC LẬP (DEPENDENCY AUDIT & SMOKE TEST)</span><span class="tag tag-green">Offline Ready</span></div>
        <div class="kv-list">
          <div><span>ADB Binary & DLLs:</span><code class="c-green">./bin/adb.exe + AdbWinApi.dll + AdbWinUsbApi.dll (OK)</code></div>
          <div><span>WebView2 Runtime:</span><code class="c-green">Evergreen v157.0.4341.60 Installed (Hỗ trợ Fixed Fallback)</code></div>
          <div><span>WinRT OCR Engine:</span><code class="c-green">Available (Windows.Media.Ocr PowerShell Bridge)</code></div>
          <div><span>SQLite WAL Database:</span><code class="c-green">./data/runtime_jx2-mobile.db (Writable + AppData Fallback)</code></div>
          <div><span>Tamper Suite (5 Cases):</span><code class="c-green">5/5 REJECTED (Chặn cả tấn công thay cặp khóa giả mạo)</code></div>
        </div>
      </div>
    </div>
    """
)

def responsive_mockup_html(res_label, width_px, height_px, rail_w, palette_w, canvas_w, canvas_h, insp_w, dock_h, mode_desc):
    rail_collapsed = (rail_w <= 70)
    pal_collapsed = (palette_w == 0)
    return f"""
    <div class="studio-win">
      <div class="proposed-ribbon">
        <span>PROPOSED DESIGN — NOT IMPLEMENTED (RESPONSIVE TARGET: {res_label})</span>
        <span class="ribbon-note">Bản thiết kế bố cục thích ứng mục tiêu cho giai đoạn P4 — Không tràn, không cắt chữ, không trùng lặp thao tác</span>
      </div>
      <div class="global-header">
        <div class="brand">
          <div class="brand-logo">FP</div>
          <div>
            <div class="brand-title">FarmerPro Studio IDE — Khung kiểm tra tỷ lệ {res_label}</div>
            <div class="brand-sub">{mode_desc}</div>
          </div>
        </div>
        <div class="ctx-selectors">
          <div class="ctx-pill project-pill"><span class="ctx-tag">PROJECT</span><span class="ctx-val">🎮 Võ Hiệp 2 (v1.0) ▾</span></div>
          <div class="ctx-pill device-pill"><span class="ctx-tag">DEVICE</span><span class="ctx-val"><span class="dot-on"></span> emulator-5554 ▾</span></div>
        </div>
        <div class="hdr-actions">
          <span class="btn btn-outline">📐 Auto Layout</span>
          <span class="btn btn-outline">🔍 Fit View</span>
          <span class="btn btn-primary">▶ Chạy thử</span>
        </div>
      </div>
      <div style="display:flex;flex:1;overflow:hidden;">
        <div style="width:{rail_w}px;background:#141C28;border-right:1px solid #2A3647;padding:8px;display:flex;flex-direction:column;justify-content:space-between;">
          <div>
            <div class="tag tag-blue" style="display:block;text-align:center;margin-bottom:8px;">NavRail: {rail_w}px</div>
            {"<div class='nav-icon-only'>[1] 🎮<br/>[2] 🔀<br/>[3] 📋<br/>[4] 📱<br/>[5] 🧪<br/>[6] 🚀</div>" if rail_collapsed else "<div class='nav-item'>1. Game Projects</div><div class='nav-item active'>2. Workflow Studio</div><div class='nav-item'>3. Tasks & Modules</div><div class='nav-item'>4. Device Lab</div><div class='nav-item'>5. Test & Debug</div><div class='nav-item'>6. Mini Builder</div>"}
          </div>
          <div class="muted-xs" style="text-align:center;">{"[>>]" if rail_collapsed else "[<< Thu gọn 60px]"}</div>
        </div>
        {"" if pal_collapsed else f'''
        <div style="width:{palette_w}px;background:#16202F;border-right:1px solid #2A3647;padding:10px;">
          <div class="row-between"><span class="sub-title">NODE PALETTE</span><span class="tag tag-purple">{palette_w}px</span></div>
          <div class="wf-list-item active">● wf-jx2-live-adb</div>
          <div class="wf-list-item">○ wf-jx2-main-quest</div>
          <div class="node-cat">▼ Thị giác / OCR</div>
          <div class="node-pill purple">vision.capture</div>
          <div class="node-pill purple">vision.find_image</div>
          <div class="node-pill purple">vision.ocr</div>
          <div class="node-cat">▼ Thao tác ADB</div>
          <div class="node-pill green">action.random_safe_tap</div>
          <div class="node-pill amber">state.checkpoint</div>
        </div>
        '''}
        <div style="flex:1;display:flex;flex-direction:column;background:#0D131D;">
          <div class="canvas-toolbar">
            <span><b>REACT FLOW CANVAS:</b> Diện tích thực tế = <b class="c-cyan">{canvas_w} × {canvas_h} px</b></span>
            <span>{"[+ Mở Node Palette nổi]" if pal_collapsed else "[<< Thu gọn Palette]"} · [Fit View] · [Zoom 100%]</span>
          </div>
          <div style="flex:1;padding:18px;display:flex;flex-direction:column;justify-content:center;align-items:center;position:relative;">
            <div style="border:2px dashed #38BDF8;border-radius:10px;padding:18px;width:94%;background:rgba(22,32,47,0.55);">
              <div class="row-between" style="margin-bottom:12px;">
                <span class="tag tag-blue">VÙNG ĐỒ THỊ REACT FLOW ({canvas_w} × {canvas_h} px)</span>
                <span class="tag tag-green">Không tràn ngang · Không chồng lấp Node</span>
              </div>
              <div class="rf-row">
                <div class="rf-node start-node"><div class="rf-hdr">flow.start</div><div class="rf-body">n-start</div></div>
                <div class="rf-arrow">➔</div>
                <div class="rf-node vision-node"><div class="rf-hdr">vision.capture</div><div class="rf-body">1280×720 PNG</div></div>
                <div class="rf-arrow">➔</div>
                <div class="rf-node vision-node selected-node"><div class="rf-hdr">vision.find_image</div><div class="rf-body">conf: 0.90</div></div>
                <div class="rf-arrow">➔</div>
                <div class="rf-node action-node"><div class="rf-hdr">random_safe_tap</div><div class="rf-body">margin: 18%</div></div>
              </div>
            </div>
          </div>
          <div style="height:{dock_h}px;background:#141C28;border-top:1px solid #2A3647;padding:8px 14px;">
            <div class="row-between">
              <span><b>EXECUTION & DEBUG DOCK (Cao: {dock_h}px)</b> — [● Step Trace (19)] [Biến Runtime] [Ảnh Khớp Mẫu]</span>
              <span class="tag tag-amber">{"Chế độ Thu gọn (36px) / Mở rộng (190px)" if dock_h < 150 else "Chế độ Mở rộng đầy đủ (210px)"}</span>
            </div>
            <div class="dock-logs" style="margin-top:6px;">
              <div><code>09:42:01.533Z</code> | <b>n-find-sys</b> | <span class="c-green">verification_passed</span> | score=1.000 @ [74,135,88,88]</div>
              <div><code>09:42:01.585Z</code> | <b>n-tap-sys</b>  | <span class="c-cyan">device_ack</span>          | input tap 111 170 (Safe Margin 18%)</div>
            </div>
          </div>
        </div>
        <div style="width:{insp_w}px;background:#16202F;border-left:1px solid #2A3647;padding:10px;">
          <div class="row-between"><span class="sub-title">NODE INSPECTOR</span><span class="tag tag-blue">{insp_w}px</span></div>
          <div class="kv-list" style="margin-top:8px;">
            <div><span>Node đang chọn:</span><code>n-find-sys</code></div>
            <div><span>Loại Node:</span><code>vision.find_image</code></div>
            <div><span>Template PNG:</span><code>system_apps.png</code></div>
            <div><span>Confidence:</span><code>0.90</code></div>
            <div><span>Safe Margin:</span><code>0.18 (18%)</code></div>
            <div><span>Retry Policy:</span><code>3 lần · 800ms</code></div>
          </div>
          <div class="quick-actions" style="margin-top:12px;flex-direction:column;">
            <span class="btn btn-sm btn-outline">✂️ Cắt mẫu từ Lab</span>
            <span class="btn btn-sm btn-primary">+ Đưa vào Task Library</span>
          </div>
        </div>
      </div>
      <div class="status-bar">
        <span>📐 Thông số Bố cục {res_label}: Header=44px | Status=26px | NavRail={rail_w}px | Palette={palette_w}px | Canvas={canvas_w}×{canvas_h}px | Inspector={insp_w}px | Dock={dock_h}px</span>
        <span>Zero Clipping · Zero Control Duplication</span>
      </div>
    </div>
    """

r_1280 = responsive_mockup_html(
    "1280 × 720 (HD Compact)", 1280, 720,
    rail_w=60, palette_w=0, canvas_w=920, canvas_h=510, insp_w=300, dock_h=140,
    mode_desc="Tự động thu gọn NavRail (60px icon-only) & ẩn Node Palette vào drawer nổi -> Dành >72% diện tích cho React Flow Canvas"
)

r_1600 = responsive_mockup_html(
    "1600 × 900 (HD+ Standard Target)", 1600, 900,
    rail_w=210, palette_w=230, canvas_w=860, canvas_h=650, insp_w=300, dock_h=180,
    mode_desc="Độ phân giải mục tiêu chuẩn của FarmerPro Studio: Hiển thị đồng thời cả 5 vùng (NavRail + Palette + Canvas + Inspector + Dock)"
)

r_1920 = responsive_mockup_html(
    "1920 × 1080 (Full HD Workstation)", 1920, 1080,
    rail_w=220, palette_w=240, canvas_w=1160, canvas_h=800, insp_w=300, dock_h=210,
    mode_desc="Không gian làm việc tối đa: Canvas rộng 1160×800 px hiển thị trọn vẹn đồ thị phức tạp 15–25 nodes cùng Execution Dock mở rộng"
)

MOCKUP_PAGES = [
    ("ui_01_new_dashboard.png", s01_html),
    ("ui_02_game_projects.png", s02_html),
    ("ui_03_workflow_studio.png", s03_html),
    ("ui_03b_workflow_studio_states.png", s03b_html),
    ("ui_04_tasks_and_modules.png", s04_html),
    ("ui_05_device_lab.png", s05_html),
    ("ui_06_test_and_debug.png", s06_html),
    ("ui_07_mini_tool_builder.png", s07_html),
    ("ui_07b_mini_builder_layouts_states.png", s07b_html),
    ("ui_08_release_management.png", s08_html),
    ("ui_res_1280x720.png", r_1280),
    ("ui_res_1600x900.png", r_1600),
    ("ui_res_1920x1080.png", r_1920),
]

HTML_DOC = """<!DOCTYPE html>
<html lang="vi">
<head>
<meta charset="UTF-8">
<style>
  @page { size: 1600px 900px; margin: 0; }
  * { box-sizing: border-box; margin: 0; padding: 0; }
  body {
    background: #101722;
    color: #EDF3FC;
    font-family: 'Segoe UI', Inter, Roboto, Arial, sans-serif;
    font-size: 13px;
    -webkit-print-color-adjust: exact !important;
    print-color-adjust: exact !important;
  }
  .mockup-page {
    width: 1600px;
    height: 900px;
    page-break-after: always;
    break-after: page;
    overflow: hidden;
    background: #101722;
    padding: 10px;
  }
  .studio-win {
    width: 100%;
    height: 100%;
    border: 2px solid #2A3647;
    border-radius: 10px;
    background: #101722;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .proposed-ribbon {
    background: #2B1F11;
    border-bottom: 1.5px solid #E7B15B;
    color: #FCD34D;
    padding: 5px 16px;
    font-size: 12px;
    font-weight: 700;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .ribbon-note { color: #FDE68A; font-weight: 500; font-size: 11.5px; }
  .global-header {
    height: 52px;
    background: #141C28;
    border-bottom: 1px solid #2A3647;
    padding: 0 16px;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .brand { display: flex; align-items: center; gap: 10px; }
  .brand-logo {
    width: 32px; height: 32px; border-radius: 7px;
    background: #1878F5; color: #fff; font-weight: 800;
    display: flex; align-items: center; justify-content: center;
  }
  .brand-title { font-size: 15px; font-weight: 700; color: #fff; }
  .brand-sub { font-size: 11px; color: #8FA2BC; }
  .ctx-selectors { display: flex; gap: 12px; }
  .ctx-pill {
    background: #192333;
    border: 1px solid #2E3D52;
    border-radius: 6px;
    padding: 5px 12px;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .project-pill { border-color: #1878F5; }
  .device-pill { border-color: #3BC38D; }
  .ctx-tag { font-size: 10px; font-weight: 700; color: #8FA2BC; }
  .ctx-val { font-size: 12px; font-weight: 700; color: #fff; white-space: nowrap; }
  .dot-on {
    display: inline-block; width: 8px; height: 8px;
    border-radius: 50%; background: #3BC38D; margin-right: 3px;
  }
  .hdr-actions { display: flex; gap: 8px; align-items: center; }
  .btn {
    padding: 5px 10px; border-radius: 6px; font-size: 11.5px; font-weight: 600;
    display: inline-block; white-space: nowrap;
  }
  .btn-sm { padding: 5px 10px; font-size: 11.5px; }
  .btn-primary { background: #1878F5; color: #fff; border: 1px solid #38BDF8; }
  .btn-amber { background: #D97706; color: #fff; border: 1px solid #F59E0B; }
  .btn-danger { background: #991B1B; color: #FCA5A5; border: 1px solid #F27883; }
  .btn-outline { background: #192230; color: #EDF3FC; border: 1px solid #2A3647; }
  .btn-ghost { background: #161E29; color: #A9B8CC; border: 1px solid #2A3647; }
  .studio-body { display: flex; flex: 1; overflow: hidden; }
  .nav-rail {
    width: 215px; background: #141C28; border-right: 1px solid #2A3647;
    padding: 12px 10px; display: flex; flex-direction: column;
  }
  .nav-section { font-size: 10px; font-weight: 700; color: #74869D; margin-bottom: 8px; padding-left: 6px; }
  .nav-item {
    padding: 9px 10px; border-radius: 7px; margin-bottom: 5px;
    border: 1px solid transparent; color: #A9B8CC;
  }
  .nav-item.active {
    background: #173355; border-color: #1878F5; color: #fff;
  }
  .nav-lbl { font-size: 13px; font-weight: 700; }
  .nav-sub { font-size: 11px; color: #8FA2BC; margin-top: 2px; }
  .nav-footer {
    margin-top: auto; padding-top: 10px; border-top: 1px solid #2A3647;
    font-size: 11px; color: #8FA2BC;
  }
  .sys-stat { margin-bottom: 4px; }
  .workspace-main { flex: 1; padding: 14px 16px; display: flex; flex-direction: column; overflow: hidden; }
  .ws-header {
    display: flex; justify-content: space-between; align-items: flex-start;
    margin-bottom: 12px; padding-bottom: 10px; border-bottom: 1px solid #2A3647;
  }
  .ws-title { font-size: 17px; font-weight: 800; color: #fff; }
  .ws-desc { font-size: 12px; color: #8FA2BC; margin-top: 3px; }
  .ws-badges { display: flex; gap: 6px; }
  .tag {
    padding: 3px 8px; border-radius: 4px; font-size: 11px; font-weight: 700;
    display: inline-block;
  }
  .tag-blue { background: #173355; color: #38BDF8; border: 1px solid #1878F5; }
  .tag-green { background: #133827; color: #3BC38D; border: 1px solid #3BC38D; }
  .tag-amber { background: #3B2D17; color: #E7B15B; border: 1px solid #E7B15B; }
  .tag-purple { background: #281B3D; color: #B291F7; border: 1px solid #B291F7; }
  .tag-red { background: #381920; color: #F27883; border: 1px solid #F27883; }

  .grid-3 { display: grid; grid-template-columns: 1fr 1.15fr 1fr; gap: 12px; }
  .grid-2 { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
  .grid-sidebar-main { display: grid; grid-template-columns: 380px 1fr; gap: 12px; flex: 1; }
  .panel {
    background: #16202F; border: 1px solid #2A3647; border-radius: 8px;
    padding: 12px; display: flex; flex-direction: column;
  }
  .panel-hdr {
    display: flex; justify-content: space-between; align-items: center;
    font-size: 12px; font-weight: 800; color: #38BDF8;
    padding-bottom: 8px; margin-bottom: 8px; border-bottom: 1px solid #2A3647;
  }
  .proj-mini-card {
    background: #121A26; border: 1px solid #2A3647; border-radius: 7px;
    padding: 10px; margin-bottom: 8px;
  }
  .proj-mini-card.active-border { border-color: #1878F5; background: #152234; }
  .dashed-card { border-style: dashed; color: #8FA2BC; text-align: center; padding: 14px; }
  .row-between { display: flex; justify-content: space-between; align-items: center; }
  .muted-xs { font-size: 11.5px; color: #8FA2BC; margin-top: 3px; }
  .stat-pills { display: flex; gap: 6px; flex-wrap: wrap; margin-top: 6px; }
  .stat-pills span {
    background: #192536; border: 1px solid #2A3647; border-radius: 4px;
    padding: 2px 7px; font-size: 11px; color: #EDF3FC;
  }
  .mini-tbl { width: 100%; border-collapse: collapse; font-size: 11.5px; }
  .mini-tbl th, .mini-tbl td {
    padding: 6px 8px; border-bottom: 1px solid #253142; text-align: left;
  }
  .mini-tbl th { background: #121924; color: #8FA2BC; font-size: 10.5px; text-transform: uppercase; }
  .active-row { background: #172B42; }
  .kv-list div {
    display: flex; justify-content: space-between; padding: 5px 0;
    border-bottom: 1px dashed #243040; font-size: 12px;
  }
  .kv-list span { color: #8FA2BC; }
  code { font-family: Consolas, monospace; color: #38BDF8; font-size: 11.5px; }
  .c-green { color: #3BC38D !important; font-weight: 700; }
  .c-cyan { color: #38BDF8 !important; font-weight: 700; }
  .c-amber { color: #E7B15B !important; font-weight: 700; }
  .quick-actions { display: flex; gap: 8px; margin-top: 10px; }
  .warn-item {
    padding: 7px 10px; border-radius: 6px; font-size: 11.5px; margin-bottom: 6px;
  }
  .warn-item.ok { background: #122B23; border: 1px solid #3BC38D; color: #A7F3D0; }
  .warn-item.warn { background: #2D2315; border: 1px solid #E7B15B; color: #FDE68A; }
  .sub-box {
    background: #121A26; border: 1px solid #2A3647; border-radius: 7px; padding: 10px;
  }
  .sub-title { font-size: 11px; font-weight: 800; color: #E7B15B; margin-bottom: 6px; }
  .code-line {
    background: #0C121A; border: 1px solid #2A3647; border-radius: 5px;
    padding: 7px 10px; font-family: Consolas, monospace; font-size: 11.5px; color: #38BDF8;
    margin-top: 5px;
  }
  .studio-3col { display: grid; grid-template-columns: 230px 1fr 295px; gap: 10px; flex: 1; }
  .wf-list-item {
    padding: 5px 8px; border-radius: 5px; font-size: 12px; margin-bottom: 3px; color: #A9B8CC;
  }
  .wf-list-item.active { background: #173355; color: #38BDF8; font-weight: 700; }
  .node-cat { font-size: 11px; font-weight: 700; color: #8FA2BC; margin: 6px 0 4px 0; }
  .node-pill {
    background: #121A26; border: 1px solid #2A3647; border-left: 4px solid #38BDF8;
    padding: 5px 8px; border-radius: 5px; font-size: 11.5px; margin-bottom: 4px;
  }
  .node-pill.purple { border-left-color: #B291F7; }
  .node-pill.green { border-left-color: #3BC38D; }
  .node-pill.amber { border-left-color: #E7B15B; }
  .canvas-col {
    background: #0D131D; border: 1px solid #2A3647; border-radius: 8px;
    display: flex; flex-direction: column; overflow: hidden;
  }
  .canvas-toolbar {
    background: #141C28; border-bottom: 1px solid #2A3647;
    padding: 8px 12px; display: flex; justify-content: space-between; align-items: center;
    font-size: 12px;
  }
  .toolbar-pills { display: flex; gap: 6px; }
  .rf-canvas {
    flex: 1; padding: 20px; display: flex; flex-direction: column; justify-content: center;
    background-image: radial-gradient(#263244 1px, transparent 1px);
    background-size: 18px 18px;
  }
  .rf-row { display: flex; align-items: center; justify-content: center; gap: 10px; }
  .rf-node {
    width: 210px; background: #16202F; border: 1.5px solid #2A3647; border-radius: 8px;
    overflow: hidden;
  }
  .rf-node.selected-node { border: 2px solid #38BDF8; box-shadow: 0 0 0 2px rgba(56,189,248,0.25); }
  .rf-hdr {
    padding: 6px 10px; font-size: 12px; font-weight: 700; background: #1B283B;
    border-bottom: 1px solid #2A3647; display: flex; justify-content: space-between;
  }
  .start-node .rf-hdr { border-left: 4px solid #38BDF8; }
  .vision-node .rf-hdr { border-left: 4px solid #B291F7; }
  .action-node .rf-hdr { border-left: 4px solid #3BC38D; }
  .cp-node .rf-hdr { border-left: 4px solid #E7B15B; }
  .rf-body { padding: 8px 10px; font-size: 11.5px; color: #CAD5E2; line-height: 1.45; }
  .rf-arrow { font-size: 20px; color: #38BDF8; font-weight: 700; }
  .bp-red { color: #F27883; font-size: 10.5px; }
  .exec-dock {
    height: 145px; background: #141C28; border-top: 1px solid #2A3647; padding: 8px 12px;
  }
  .dock-hdr {
    display: flex; justify-content: space-between; font-size: 11.5px;
    padding-bottom: 6px; border-bottom: 1px solid #253142;
  }
  .dock-logs { font-family: Consolas, monospace; font-size: 11px; line-height: 1.55; margin-top: 6px; color: #CAD5E2; }
  .crop-preview-box {
    background: #080C12; border: 1px solid #2A3647; border-radius: 8px;
    padding: 8px; position: relative; text-align: center; height: 470px;
    display: flex; align-items: center; justify-content: center;
  }
  .live-shot-img { max-height: 445px; max-width: 100%; border-radius: 6px; border: 1px solid #2A3647; }
  .roi-overlay {
    position: absolute; top: 154px; left: 28px; width: 42px; height: 42px;
    border: 2.5px dashed #3BC38D; background: rgba(59,195,141,0.18); border-radius: 6px;
  }
  .roi-tag {
    position: absolute; top: -22px; left: 0; background: #133827; color: #3BC38D;
    font-size: 10px; font-weight: 700; padding: 2px 6px; border-radius: 4px; white-space: nowrap;
  }
  .evidence-shot-card {
    background: #121A26; border: 1px solid #2A3647; border-radius: 8px;
    padding: 8px; text-align: center;
  }
  .ev-img { max-height: 410px; max-width: 100%; border-radius: 6px; margin: 4px auto; display: block; }
  .mini-preview-frame {
    background: #12141C; border: 2px solid #F59E0B; border-radius: 9px; padding: 10px;
  }
  .mp-hdr {
    background: #1F1710; border-bottom: 1px solid #78350F; padding: 8px 10px;
    border-radius: 6px; display: flex; justify-content: space-between; align-items: center;
  }
  .mp-badge {
    background: #F59E0B; color: #000; font-size: 10.5px; font-weight: 800;
    padding: 2px 7px; border-radius: 4px; margin-left: 6px;
  }
  .mp-task { padding: 5px 0; border-bottom: 1px dashed #253142; font-size: 11.5px; }
  .nav-icon-only { font-size: 13px; line-height: 2.3; text-align: center; color: #38BDF8; font-weight: 700; }
  .studio-3col-wide { display: grid; grid-template-columns: 205px 1fr 275px; gap: 10px; flex: 1; }
  .builder-3col { display: grid; grid-template-columns: 310px 1fr 315px; gap: 10px; flex: 1; }
  .executing-node { border: 2px solid #3BC38D; box-shadow: 0 0 0 2px rgba(59,195,141,0.28); }
  .c-red { color: #F27883 !important; font-weight: 700; }
  .state-card { justify-content: flex-start; gap: 6px; }
  .mini-canvas-box { background: #0C121B; border: 1px solid #2A3647; border-radius: 7px; padding: 10px; margin-top: 4px; flex: 1; display: flex; flex-direction: column; justify-content: center; text-align: center; }
  .mc-pill { display: inline-block; background: #173355; color: #38BDF8; border: 1px solid #1878F5; border-radius: 4px; padding: 2px 8px; font-size: 11px; font-weight: 700; margin: 0 auto 8px auto; }
  .mc-flow { font-size: 11.5px; color: #CAD5E2; }
  .mc-flow span { background: #16202F; border: 1px solid #2A3647; padding: 4px 7px; border-radius: 4px; }
  .mc-node { padding: 8px 10px; border-radius: 6px; background: #16202F; font-size: 11.5px; margin-bottom: 6px; }
  .mc-selected { border: 2px solid #38BDF8; }
  .mc-running { border: 2px solid #3BC38D; }
  .mc-error { border: 2px solid #F27883; background: #2A161D; }
  .mc-bp { border: 2px solid #E7B15B; background: #292014; }
  .mc-insp-mini { font-size: 11px; color: #8FA2BC; }
  .mc-dock-mini { font-family: Consolas, monospace; font-size: 10.5px; }
  .mc-hdr-bar { background: #172334; border: 1px solid #2A3647; border-radius: 4px; padding: 5px 8px; font-size: 11px; font-weight: 700; margin-bottom: 6px; }
  .mc-box-row { background: #141C28; border: 1px dashed #334257; border-radius: 4px; padding: 8px; font-size: 11px; color: #CAD5E2; margin-bottom: 4px; }
  .blk-row { background: #121A26; border: 1px solid #2A3647; border-radius: 5px; padding: 6px 8px; margin-bottom: 4px; display: flex; justify-content: space-between; font-size: 11.5px; }
  .blk-row.blk-active { border-color: #38BDF8; background: #172B42; font-weight: 700; }
  .selected-preview-box { border: 2px dashed #38BDF8; border-radius: 7px; padding: 8px 6px 4px 6px; position: relative; margin-top: 10px; background: rgba(24,120,245,0.06); }
  .sel-pill-tag { position: absolute; top: -11px; left: 10px; background: #1878F5; color: #fff; font-size: 10px; font-weight: 700; padding: 1px 8px; border-radius: 4px; }
  .form-grp { margin-bottom: 7px; }
  .form-grp label { display: block; font-size: 11px; color: #8FA2BC; margin-bottom: 3px; font-weight: 600; }
  .fake-input { background: #0C121A; border: 1px solid #2E3D52; border-radius: 5px; padding: 5px 9px; font-size: 12px; color: #EDF3FC; }
  .status-bar {
    height: 28px; background: #121924; border-top: 1px solid #2A3647;
    padding: 0 16px; display: flex; align-items: center; justify-content: space-between;
    font-size: 11px; color: #8FA2BC;
  }
</style>
</head>
<body>
""" + "\n".join(f'<div class="mockup-page">{html_body}</div>' for _, html_body in MOCKUP_PAGES) + """
</body>
</html>
"""

tmp_html = os.path.join(IMG_DIR, "_tmp_mockups.html")
tmp_pdf = os.path.join(IMG_DIR, "_tmp_mockups.pdf")
with open(tmp_html, "w", encoding="utf-8") as f:
    f.write(HTML_DOC)

CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"
subprocess.run([
    CHROME,
    "--headless=new",
    "--disable-gpu",
    "--no-pdf-header-footer",
    f"--print-to-pdf={tmp_pdf}",
    f"file:///{tmp_html.replace(os.sep, '/')}"
], check=True)

doc = pymupdf.open(tmp_pdf)
for idx, (out_name, _) in enumerate(MOCKUP_PAGES):
    pix = doc[idx].get_pixmap(dpi=144)
    pix.save(os.path.join(IMG_DIR, out_name))
doc.close()

os.remove(tmp_html)
os.remove(tmp_pdf)
print(f"Rendered {len(MOCKUP_PAGES)} high-resolution Proposed UI Mockup PNGs into {IMG_DIR}")

# Also rasterize all 14 SVG diagrams to high-DPI PNGs in IMG_DIR via Chrome headless for full CSS & font fidelity
svg_files = sorted([f for f in os.listdir(DIAGRAM_DIR) if f.endswith(".svg")])
svg_pages_html = []
for svg_name in svg_files:
    svg_path = os.path.join(DIAGRAM_DIR, svg_name)
    with open(svg_path, "r", encoding="utf-8") as sf:
        raw_svg = sf.read()
        raw_svg = raw_svg.replace('<?xml version="1.0" encoding="UTF-8"?>', '').strip()
        svg_pages_html.append(f'<div class="svg-page">{raw_svg}</div>')

SVG_HTML_DOC = """<!DOCTYPE html>
<html lang="vi">
<head>
<meta charset="UTF-8">
<style>
  @page { size: 1400px 520px; margin: 0; }
  * { box-sizing: border-box; margin: 0; padding: 0; }
  body { background: #0D131D; -webkit-print-color-adjust: exact !important; print-color-adjust: exact !important; }
  .svg-page { width: 1400px; height: 520px; page-break-after: always; break-after: page; overflow: hidden; }
</style>
</head>
<body>
""" + "\n".join(svg_pages_html) + "\n</body></html>"

tmp_svg_html = os.path.join(IMG_DIR, "_tmp_svgs.html")
tmp_svg_pdf = os.path.join(IMG_DIR, "_tmp_svgs.pdf")
with open(tmp_svg_html, "w", encoding="utf-8") as f:
    f.write(SVG_HTML_DOC)

subprocess.run([
    CHROME,
    "--headless=new",
    "--disable-gpu",
    "--no-pdf-header-footer",
    f"--print-to-pdf={tmp_svg_pdf}",
    f"file:///{tmp_svg_html.replace(os.sep, '/')}"
], check=True)

sdoc = pymupdf.open(tmp_svg_pdf)
for idx, svg_name in enumerate(svg_files):
    png_name = "diag_" + svg_name.replace(".svg", ".png")
    spix = sdoc[idx].get_pixmap(dpi=160)
    spix.save(os.path.join(IMG_DIR, png_name))
sdoc.close()
os.remove(tmp_svg_html)
os.remove(tmp_svg_pdf)

print(f"Rasterized {len(svg_files)} SVG diagrams into high-DPI PNGs in {IMG_DIR}")
