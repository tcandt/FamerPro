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

def studio_shell(active_nav_idx, header_right_btns, main_content_html, title_sub="Võ Hiệp Truyền Kỳ 2 Mobile (v1.0.0)"):
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
            <span class="ctx-val"><span class="dot-on"></span> emulator-5554 (SM_X710N · 1280×720) ▾</span>
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
        <div class="ws-title">ẢNH 02 — GAME PROJECTS (QUẢN LÝ ĐA DỰ ÁN GAME, ASSETS, VERSION & MODULES)</div>
        <div class="ws-desc">Quản lý danh sách nhiều tựa game, tạo/sửa/nhân bản Project, cấu hình Android Package ID, thư viện ảnh mẫu PNG và ma trận 6 module.</div>
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

# Screen 03: Workflow Studio
s03_html = studio_shell(
    2,
    '<span class="btn btn-outline">📐 Auto Layout</span><span class="btn btn-outline">🔍 Fit View</span><span class="btn btn-outline">⏸ Breakpoint</span><span class="btn btn-outline">⏭ Step Run</span><span class="btn btn-primary">▶ Debug trên Lab</span>',
    """
    <div class="studio-3col">
      <div class="panel palette-col">
        <div class="panel-hdr"><span>NODE PALETTE & LUỒNG</span><span class="tag tag-blue">10 Luồng</span></div>
        <div class="sub-title">LUỒNG CỦA PROJECT (jx2-mobile)</div>
        <div class="wf-list-item active">● wf-jx2-live-adb-probe</div>
        <div class="wf-list-item">○ wf-jx2-main-quest</div>
        <div class="wf-list-item">○ wf-jx2-treasure-map</div>
        <div class="wf-list-item">○ wf-jx2-bandit-dungeon</div>
        <div class="sub-title" style="margin-top:8px;">DANH MỤC NODE (KÉO THẢ)</div>
        <div class="node-cat">▼ Điều khiển Luồng (Flow)</div>
        <div class="node-pill">flow.start / flow.end</div>
        <div class="node-pill">flow.wait / flow.branch</div>
        <div class="node-cat">▼ Thị giác & WinRT OCR</div>
        <div class="node-pill purple">vision.capture (PNG)</div>
        <div class="node-pill purple">vision.find_image (SSD)</div>
        <div class="node-pill purple">vision.ocr (WinRT Engine)</div>
        <div class="node-cat">▼ Thao tác Thiết bị (ADB)</div>
        <div class="node-pill green">action.random_safe_tap</div>
        <div class="node-pill green">action.swipe / key_event</div>
        <div class="node-cat">▼ Trạng thái & Phục hồi</div>
        <div class="node-pill amber">state.checkpoint (WAL)</div>
      </div>
      <div class="canvas-col">
        <div class="canvas-toolbar">
          <span><b>CANVAS REACT FLOW:</b> <code>wf-jx2-live-adb-probe</code> (7 Nodes · 6 Edges)</span>
          <div class="toolbar-pills">
            <span class="tag tag-blue">Auto Layout: DAG</span>
            <span class="tag tag-blue">Fit View: 100%</span>
            <span class="tag tag-red">● Breakpoint @ n-tap-sys</span>
            <span class="tag tag-green">Step Run: #4 Verified</span>
          </div>
        </div>
        <div class="rf-canvas">
          <div class="rf-row">
            <div class="rf-node start-node">
              <div class="rf-hdr">1. flow.start</div>
              <div class="rf-body">ID: <code>n-start</code><br/>Khởi tạo phiên ADB</div>
            </div>
            <div class="rf-arrow">➔</div>
            <div class="rf-node vision-node">
              <div class="rf-hdr">2. vision.capture</div>
              <div class="rf-body">ID: <code>n-cap</code><br/>screencap -p (1280×720)</div>
            </div>
            <div class="rf-arrow">➔</div>
            <div class="rf-node vision-node selected-node">
              <div class="rf-hdr">3. vision.find_image <span class="bp-dot" title="Selected">★</span></div>
              <div class="rf-body">ID: <code>n-find-sys</code><br/>Mẫu: <code>system_apps.png</code><br/>Confidence: <b>0.90 (Matched 1.00)</b></div>
            </div>
          </div>
          <div class="rf-row" style="margin-top:18px;">
            <div class="rf-node action-node">
              <div class="rf-hdr">4. action.random_safe_tap <span class="bp-red">● BP</span></div>
              <div class="rf-body">ID: <code>n-tap-sys</code><br/>Safe Margin: <b>18% (0.18)</b><br/>Tap thực tế: <b>(111, 170)</b></div>
            </div>
            <div class="rf-arrow">➔</div>
            <div class="rf-node vision-node">
              <div class="rf-hdr">5. vision.ocr (WinRT)</div>
              <div class="rf-body">ID: <code>n-ocr</code><br/>Biến: <code>ocr_text</code><br/>KQ: <i>"Cài đặt Files... Play"</i></div>
            </div>
            <div class="rf-arrow">➔</div>
            <div class="rf-node cp-node">
              <div class="rf-hdr">6. state.checkpoint</div>
              <div class="rf-body">ID: <code>n-cp</code><br/>Key: <code>live_adb_verified</code><br/>Lưu SQLite WAL</div>
            </div>
          </div>
        </div>
        <div class="exec-dock">
          <div class="dock-hdr">
            <span><b>EXECUTION & DEBUG DOCK</b> — Tabs: [● Step Trace (19)] [Biến Runtime (ocr_text)] [Ảnh Khớp Mẫu] [Checkpoints (1)]</span>
            <span class="tag tag-green">Job: mini-job-cf950745 · SUCCEEDED</span>
          </div>
          <div class="dock-logs">
            <div><code>09:42:01.513Z</code> | <b>n-find-sys</b> | <span class="c-green">verification_passed</span> | SSD score=1.000 &gt;= 0.90, bbox=[74, 135, 88, 88]</div>
            <div><code>09:42:01.585Z</code> | <b>n-tap-sys</b>  | <span class="c-cyan">device_ack</span>          | Safe Random Tap margin=18% -&gt; adb shell input tap 111 170</div>
            <div><code>09:42:04.727Z</code> | <b>n-ocr</b>      | <span class="c-green">verification_passed</span> | WinRT OCR extracted: "16:42 System Apps ... Cài đặt Files ... Cửa hàng Play"</div>
          </div>
        </div>
      </div>
      <div class="panel inspector-col">
        <div class="panel-hdr"><span>NODE INSPECTOR</span><span class="tag tag-purple">n-find-sys</span></div>
        <div class="kv-list">
          <div><span>Loại Node:</span><code>vision.find_image</code></div>
          <div><span>Node ID:</span><code>n-find-sys</code></div>
          <div><span>Ảnh mẫu Project:</span><code>live_ldplayer_system_apps.png</code></div>
          <div><span>Ngưỡng Confidence:</span><code>0.90 (Multiscale SSD)</code></div>
          <div><span>Số lần Retry tối đa:</span><code>3 lần (Backoff: 800 ms)</code></div>
          <div><span>Breakpoint Debug:</span><code>Bật dừng trước khi Tap</code></div>
        </div>
        <div class="sub-box" style="margin-top:8px;text-align:center;">
          <div class="sub-title">ẢNH MẪU ĐANG GẮN (88×88 px)</div>
          <img src="__B64_CROP__" style="width:64px;height:64px;border:1px solid #38BDF8;border-radius:6px;margin:4px auto;display:block;" />
          <div class="muted-xs">Tọa độ gốc: (74, 135) · SHA: 3f7a9c...</div>
        </div>
        <div class="quick-actions" style="margin-top:8px;flex-direction:column;">
          <span class="btn btn-sm btn-outline">✂️ Cắt ảnh mẫu mới từ Device Lab</span>
          <span class="btn btn-sm btn-primary">+ Đưa Workflow vào Task Library</span>
        </div>
      </div>
    </div>
    """.replace("__B64_CROP__", b64_crop)
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
        <span class="tag tag-green">Phiên: mini-job-cf950745 · PASS (19 Events)</span>
        <span class="tag tag-amber">Phiên: mini-job-8812c · CANCELLED (User Stop)</span>
      </div>
    </div>
    <div class="grid-sidebar-main">
      <div class="panel">
        <div class="panel-hdr"><span>1. DANH SÁCH PHIÊN & TIMELINE NODE TRACE (19 SỰ KIỆN)</span><span class="tag tag-green">LIVE_ADB</span></div>
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

# Screen 07: Mini Tool Builder
s07_html = studio_shell(
    6,
    '<span class="btn btn-outline">🔍 Chạy Pre-Flight Audit (6/6)</span><span class="btn btn-amber">🚀 Build & Xuất Mini Tool (.EXE)</span>',
    """
    <div class="ws-header">
      <div>
        <div class="ws-title">ẢNH 07 — MINI TOOL BUILDER (THIẾT KẾ GIAO DIỆN NO-CODE, CHỌN MODULE, LIVE PREVIEW & BUILD EXE)</div>
        <div class="ws-desc">Cấu hình trọn vẹn sản phẩm Mini Auto-Tool cho từng Game Project: Branding, Layout Preset, cột chỉ số OCR động, 5-State Modules và xem trước trực tiếp.</div>
      </div>
    </div>
    <div class="grid-sidebar-main">
      <div class="panel">
        <div class="panel-hdr"><span>1. CẤU HÌNH SẢN PHẨM MINI TOOL (ui-schema.json)</span><span class="tag tag-amber">Project: jx2-mobile</span></div>
        <div class="sub-box">
          <div class="sub-title">A. CHỌN GAME PROJECT, BRANDING & PHIÊN BẢN</div>
          <div class="kv-list">
            <div><span>Game Project:</span><code>Võ Hiệp Truyền Kỳ 2 Mobile (jx2-mobile · v1.0.0)</code></div>
            <div><span>Tên file EXE & Cửa sổ:</span><code>Auto_JX2_Mobile.exe | "Auto Võ Hiệp 2 Mobile v1.0.0"</code></div>
            <div><span>Màu thương hiệu (Accent):</span><code>#F59E0B (Võ Hiệp Amber) · Nền Header: #18131A</code></div>
            <div><span>Biểu tượng nhúng PE:</span><code>projects/jx2-mobile/branding/icon.ico (7 icon layers)</code></div>
          </div>
        </div>
        <div class="sub-box" style="margin-top:6px;">
          <div class="sub-title">B. LAYOUT EDITOR & CỘT CHỈ SỐ OCR ĐỘNG (DEVICE COLUMNS)</div>
          <div class="kv-list">
            <div><span>Bố cục (Layout Preset):</span><code>compact_2_col (740×660 px · Bảng thiết bị + 2 cột Nhiệm vụ/Log)</code></div>
            <div><span>Cột OCR #1:</span><code>Nhãn="Cấp"  | Biến="char_level"  | Fallback="Chưa OCR (-)"</code></div>
            <div><span>Cột OCR #2:</span><code>Nhãn="Vàng" | Biến="char_gold"   | Fallback="Chưa OCR (-)"</code></div>
            <div><span>Cột OCR #3:</span><code>Nhãn="Bạc"  | Biến="char_silver" | Fallback="Chưa OCR (-)"</code></div>
          </div>
        </div>
        <div class="sub-box" style="margin-top:6px;">
          <div class="sub-title">C. CHỌN WORKFLOWS (2), TASKS (5) & MODULES ĐÓNG GÓI (5/6)</div>
          <div class="stat-pills">
            <span class="tag tag-green">☑ mod.adb_core</span>
            <span class="tag tag-green">☑ mod.vision_ocr</span>
            <span class="tag tag-green">☑ mod.account_vault</span>
            <span class="tag tag-green">☑ mod.proxy_network</span>
            <span class="tag tag-red">☐ mod.adb_port_forward</span>
            <span class="tag tag-green">☑ mod.emulator_optimizer</span>
          </div>
        </div>
      </div>
      <div class="panel">
        <div class="panel-hdr"><span>2. LIVE INTERACTIVE PREVIEW (XEM TRƯỚC GIAO DIỆN MINI TOOL NGAY TRONG STUDIO)</span><span class="tag tag-green">Render 100% từ Schema</span></div>
        <div class="mini-preview-frame">
          <div class="mp-hdr">
            <div><b>⚔️ Auto Võ Hiệp 2 Mobile v1.0.0</b> <span class="mp-badge">JX2-MMORPG v1.0</span></div>
            <div class="muted-xs">Pkg: com.blackhole.jx2mobile · Ed25519 Verified (PE #7777)</div>
          </div>
          <table class="mini-tbl" style="margin:6px 0;">
            <tr><th>Chọn</th><th>Thiết bị ADB</th><th>Trạng thái</th><th>Cấp (OCR)</th><th>Vàng (OCR)</th><th>Bạc (OCR)</th></tr>
            <tr><td>☑</td><td><code>emulator-5554</code></td><td><span class="tag tag-green">ONLINE</span></td><td>Chưa OCR (-)</td><td>Chưa OCR (-)</td><td>Chưa OCR (-)</td></tr>
          </table>
          <div class="grid-2" style="gap:8px;">
            <div class="sub-box">
              <div class="sub-title">DANH SÁCH NHIỆM VỤ ([Tất cả] [NV] [PB] [HĐ])</div>
              <div class="mp-task">☑ <span class="tag tag-amber">[NV]</span> Chính Tuyến & Đối thoại Dã Tẩu</div>
              <div class="mp-task">☑ <span class="tag tag-amber">[NV]</span> Kiểm tra Thiết bị ADB Thật + OCR</div>
              <div class="mp-task">☑ <span class="tag tag-amber">[NV]</span> Tìm Kho Báu & Đào Tàng Bảo Đồ</div>
              <div class="mp-task">☑ <span class="tag tag-purple">[PB]</span> Sơn Tặc Mật Quật & Bí Cảnh</div>
            </div>
            <div class="sub-box">
              <div class="sub-title">CẤU HÌNH TASK & NHẬT KÝ THỰC THI</div>
              <div class="muted-xs">☑ Tự động bỏ qua hội thoại NPC</div>
              <div class="muted-xs">Số chuỗi Dã Tẩu tối đa: [ 10 ]</div>
              <div class="code-line" style="margin-top:4px;">[READY] Đã xác thực chữ ký Ed25519 qua khóa PE RT_RCDATA #7777.</div>
            </div>
          </div>
        </div>
        <div class="warn-item ok" style="margin-top:6px;">✅ Pre-Flight Audit 6/6 PASS: Workflows hợp lệ · 4 PNGs tồn tại · Icon 7 lớp · Keystore sẵn sàng · Runtime binary sẵn sàng!</div>
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
          <td><span class="tag tag-green">8/8 PASS (1.09s)</span></td>
        </tr>
        <tr>
          <td><b>rel-slg-1.0.0</b> (kingdom-slg)</td>
          <td><code>v1.0.0</code></td>
          <td><code>releases/poc/Auto_Kingdom_SLG/Auto_Kingdom_SLG.exe</code></td>
          <td>12,516,352 B</td>
          <td><code>95732232077c0747fe611def...</code></td>
          <td><code>acb43ebd9ab499f0... (#7777)</code></td>
          <td><span class="tag tag-green">8/8 PASS (1.12s)</span></td>
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
    ("ui_04_tasks_and_modules.png", s04_html),
    ("ui_05_device_lab.png", s05_html),
    ("ui_06_test_and_debug.png", s06_html),
    ("ui_07_mini_tool_builder.png", s07_html),
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
  .ctx-val { font-size: 12.5px; font-weight: 700; color: #fff; }
  .dot-on {
    display: inline-block; width: 8px; height: 8px;
    border-radius: 50%; background: #3BC38D; margin-right: 3px;
  }
  .hdr-actions { display: flex; gap: 8px; align-items: center; }
  .btn {
    padding: 6px 12px; border-radius: 6px; font-size: 12px; font-weight: 600;
    display: inline-block;
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
