import os
import html

DIAGRAM_DIR = r"E:\DATA\FarmerPro\artifacts\p1_2_evidence\diagrams"
os.makedirs(DIAGRAM_DIR, exist_ok=True)

def svg_wrap(width, height, title, subtitle, body_svg):
    return f"""<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="{width}" height="{height}">
  <defs>
    <style>
      text {{ font-family: 'Segoe UI', Inter, Roboto, Arial, sans-serif; fill: #EDF3FC; }}
      .mono {{ font-family: Consolas, 'Courier New', monospace; }}
      .title {{ font-size: 20px; font-weight: 700; fill: #FFFFFF; }}
      .subtitle {{ font-size: 12.5px; font-weight: 500; fill: #8FA2BC; }}
      .zone-title {{ font-size: 13.5px; font-weight: 700; letter-spacing: 0.4px; }}
      .box-title {{ font-size: 13px; font-weight: 700; fill: #FFFFFF; }}
      .box-sub {{ font-size: 11px; font-weight: 400; fill: #A9B8CC; }}
      .box-code {{ font-family: Consolas, monospace; font-size: 10.5px; fill: #38BDF8; }}
      .badge {{ font-size: 10px; font-weight: 700; }}
      .edge-label {{ font-size: 10px; font-weight: 600; fill: #38BDF8; }}
    </style>
    <marker id="arrow-blue" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
      <path d="M 0 1 L 10 5 L 0 9 z" fill="#38BDF8"/>
    </marker>
    <marker id="arrow-green" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
      <path d="M 0 1 L 10 5 L 0 9 z" fill="#3BC38D"/>
    </marker>
    <marker id="arrow-amber" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
      <path d="M 0 1 L 10 5 L 0 9 z" fill="#E7B15B"/>
    </marker>
    <marker id="arrow-red" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
      <path d="M 0 1 L 10 5 L 0 9 z" fill="#F27883"/>
    </marker>
    <marker id="arrow-purple" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
      <path d="M 0 1 L 10 5 L 0 9 z" fill="#B291F7"/>
    </marker>
  </defs>
  <rect x="0" y="0" width="{width}" height="{height}" rx="10" fill="#0D131D" stroke="#2A3647" stroke-width="2"/>
  <rect x="0" y="0" width="{width}" height="56" rx="10" fill="#141C28"/>
  <line x1="0" y1="56" x2="{width}" y2="56" stroke="#2A3647" stroke-width="1.5"/>
  <text x="22" y="25" fill="#FFFFFF" font-family="Segoe UI, Inter, Arial, sans-serif" font-size="18.5" font-weight="700">{html.escape(title)}</text>
  <text x="22" y="45" fill="#9FB2CC" font-family="Segoe UI, Inter, Arial, sans-serif" font-size="12.5" font-weight="500">{html.escape(subtitle)}</text>
  {body_svg}
</svg>"""

def zone(x, y, w, h, title, border_color="#2A3647", bg_color="#121A26", title_color="#38BDF8"):
    pill_w = min(w - 24, len(title) * 7.4 + 24)
    return f"""
  <rect x="{x}" y="{y}" width="{w}" height="{h}" rx="8" fill="{bg_color}" stroke="{border_color}" stroke-width="1.5" stroke-dasharray="4,2"/>
  <rect x="{x+12}" y="{y-11}" width="{pill_w:.0f}" height="21" rx="4" fill="#16202F" stroke="{border_color}" stroke-width="1.2"/>
  <text x="{x+20}" y="{y+4}" fill="{title_color}" font-family="Segoe UI, Inter, Arial, sans-serif" font-size="12.2" font-weight="700">{html.escape(title)}</text>"""

def fit_text_svg(x, y, avail_w, txt, fill, ff, base_fs, weight="400"):
    char_factor = 0.57 if "Consolas" in ff else (0.56 if weight == "700" else 0.52)
    est_w = len(txt) * (base_fs * char_factor)
    fs = base_fs
    extra = ""
    if est_w > avail_w:
        scaled_fs = base_fs * (avail_w / est_w)
        if scaled_fs >= 8.8:
            fs = round(scaled_fs, 2)
        else:
            fs = 8.8
            extra = f' textLength="{avail_w:.1f}" lengthAdjust="spacingAndGlyphs"'
    fw_attr = f' font-weight="{weight}"' if weight != "400" else ""
    return f'<text x="{x}" y="{y}" fill="{fill}" font-family="{ff}" font-size="{fs}"{fw_attr}{extra}>{html.escape(txt)}</text>'

def node_box(x, y, w, h, title, lines, border="#2A3647", bg="#16202F", accent="#1878F5"):
    avail_w = w - 22
    out = [
        f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="6" fill="{bg}" stroke="{border}" stroke-width="1.5"/>',
        f'<rect x="{x}" y="{y}" width="5" height="{h}" rx="2" fill="{accent}"/>',
        fit_text_svg(x + 14, y + 20, avail_w, title, "#FFFFFF", "Segoe UI, Inter, Arial, sans-serif", 12.2, "700")
    ]
    curr_y = y + 37
    for line in lines:
        is_code = line.startswith("code:")
        txt = line[5:] if is_code else line
        fill = "#38BDF8" if is_code else "#CAD5E2"
        ff = "Consolas, monospace" if is_code else "Segoe UI, Inter, Arial, sans-serif"
        base_fs = 10.2 if is_code else 10.8
        out.append(fit_text_svg(x + 14, curr_y, avail_w, txt, fill, ff, base_fs, "400"))
        curr_y += 14.5
    return "\n  ".join(out)

def arrow(x1, y1, x2, y2, color="#38BDF8", marker="arrow-blue", label="", dashed=False):
    dash = ' stroke-dasharray="5,4"' if dashed else ''
    mx, my = (x1 + x2) / 2.0, (y1 + y2) / 2.0
    lbl_svg = ""
    if label:
        dx = abs(x2 - x1)
        dy = abs(y2 - y1)
        natural_w = len(label) * 5.8 + 12.0
        # Constrain pill width to strictly fit inside horizontal gap when connecting two columns
        if 24 <= dx <= 160:
            max_w = dx - 6.0
            lw = min(natural_w, max_w)
        else:
            lw = natural_w
        lh = 15.0
        if dx < 24 and 18 <= dy <= 60:
            lh = min(15.0, dy - 4.0)
        fs = 9.5
        text_extra = ""
        if len(label) * (fs * 0.56) > lw - 6.0:
            scaled = fs * ((lw - 6.0) / (len(label) * (fs * 0.56)))
            if scaled >= 7.8:
                fs = round(scaled, 2)
            else:
                fs = 7.8
                text_extra = f' textLength="{max(10.0, lw - 6.0):.1f}" lengthAdjust="spacingAndGlyphs"'
        lbl_svg = f"""
  <rect x="{mx - lw/2.0:.1f}" y="{my - lh/2.0:.1f}" width="{lw:.1f}" height="{lh:.1f}" rx="3" fill="#0D131D" stroke="{color}" stroke-width="1"/>
  <text x="{mx:.1f}" y="{my + 3.2:.1f}" text-anchor="middle" fill="{color}" font-family="Segoe UI, Inter, Arial, sans-serif" font-size="{fs}" font-weight="700"{text_extra}>{html.escape(label)}</text>"""
    return f'<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{color}" stroke-width="2"{dash} marker-end="url(#{marker})"/>' + lbl_svg

# -------------------------------------------------------------------------
# 1. PART 1A: OVERALL ARCHITECTURE (SƠ ĐỒ 1.1)
# -------------------------------------------------------------------------
part1_body = "\n".join([
    zone(18, 72, 430, 430, "1. FARMERPRO STUDIO IDE (FarmerPro.exe - Nhà Máy Sản Xuất)", "#1878F5", "#111927", "#38BDF8"),
    node_box(34, 98, 398, 48, "Global Context Header (44px)", ["Active Game Project Switcher | Active Lab Device | System Settings"], "#1878F5", "#172437", "#38BDF8"),
    node_box(34, 156, 192, 74, "1. Game Projects Hub", ["Multi-Game Manifests", "Templates PNG & Assets", "code:projects/<slug>/"], "#2A3647", "#16202F", "#1878F5"),
    node_box(240, 156, 192, 74, "2. Workflow Studio", ["React Flow Graph Canvas", "Vision/OCR & Safe Tap", "code:NodePalette + Inspector"], "#2A3647", "#16202F", "#B291F7"),
    node_box(34, 240, 192, 74, "3. Tasks & Modules", ["Task Library & Priority", "Vault, Proxy, Watchdog", "code:5-State Module Config"], "#2A3647", "#16202F", "#3BC38D"),
    node_box(240, 240, 192, 74, "4. Device Lab", ["ADB USB/TCP & LDPlayer", "On-Demand PNG Cropper", "code:ROI -> Project Templates"], "#2A3647", "#16202F", "#38BDF8"),
    node_box(34, 324, 192, 74, "5. Test & Debug", ["Live Step Trace (SQLite)", "Before/After Verification", "code:Checkpoints & Recovery"], "#2A3647", "#16202F", "#E7B15B"),
    node_box(240, 324, 192, 74, "6. Mini Tool Builder", ["UI Schema Form Designer", "Live Mini Tool Preview", "code:8-Step EXE Packager"], "#E7B15B", "#1F2421", "#E7B15B"),
    node_box(34, 410, 398, 76, "Studio Storage & Producer Keystore", [
        "SQLite WAL (farmerpro.db v2 Multi-Project Schema)",
        "External Keystore: %LOCALAPPDATA%\\FarmerPro\\ProducerKeys\\",
        "code:32-byte Ed25519 SigningKey NEVER enters bundle/ or git"
    ], "#2A3647", "#141D2B", "#F27883"),

    zone(478, 72, 415, 430, "2. SHARED RUST CORE & PACKAGER (14 Cargo Crates)", "#B291F7", "#131926", "#B291F7"),
    node_box(494, 98, 383, 108, "Automation & Workflow Engine Crates (Shared)", [
        "• fp-domain / fp-workflow-schema / fp-workflow-runtime",
        "• fp-adb (Argv Command) / fp-device (Generation Fencing)",
        "• fp-vision (Multiscale SSD + WinRT OCR) / fp-actions (Safe Tap)",
        "• fp-scheduler (Device Leases) / fp-storage (SQLite WAL) / fp-security",
        "code:100% Dùng chung cho cả Studio và mọi Mini Tool không cần sửa code"
    ], "#2A3647", "#16202F", "#B291F7"),
    node_box(494, 220, 383, 128, "fp-builder (8-Step Packager - Studio/CLI Only)", [
        "Step 1-3: Validate Graph, Path Guard, 5-State Module Matrix",
        "Step 4-5: Copy fp-mini-runtime.exe + Inject PE Icon & RT_RCDATA #7777",
        "Step 6-7: Copy bundle/ + ./bin/adb.exe + Sign Ed25519 (RFC 8032)",
        "Step 8: Verify Signature via PE RT_RCDATA #7777 & Audit Dependencies",
        "code:MiniToolPackager::build_and_export_release()"
    ], "#E7B15B", "#1E222A", "#E7B15B"),
    node_box(494, 362, 383, 124, "fp-bundle-contract & fp-bundle-verify (Runtime Guard)", [
        "• fp-bundle-contract: Zero-Crypto Schemas, 5-State Module Matrix",
        "• fp-bundle-verify: Reads Trusted PubKey from PE RT_RCDATA #7777",
        "• Verifies RFC 8032 Ed25519 Signature & SHA-256 of every file",
        "• Automatic Read-Only NTFS ACL Fallback -> %LOCALAPPDATA%",
        "code:Compile-time isolated: ZERO dependency on fp-builder"
    ], "#3BC38D", "#132325", "#3BC38D"),

    zone(922, 72, 458, 430, "3. STANDALONE MINI AUTO-TOOLS (Chạy Độc Lập 100%)", "#3BC38D", "#101E22", "#3BC38D"),
    node_box(938, 100, 426, 178, "Release Package #1: Auto_JX2_Mobile/", [
        "• Auto_JX2_Mobile.exe (12.51 MB | SHA256: 55c051cba31c269c...)",
        "• PE RT_ICON (7 layers) + PE RT_RCDATA #7777 (PubKey: acb43ebd...)",
        "• bundle/: manifest + ui-schema (Amber #F59E0B, 5 Tasks, 3 Cột OCR)",
        "• bundle/: 2 Workflows + 4 PNG Templates + signature.fpbundle.json",
        "• bin/: adb.exe + AdbWinApi.dll + AdbWinUsbApi.dll",
        "• data/: runtime_jx2-mobile.db (SQLite WAL độc lập)",
        "code:Chạy trực tiếp cho người chơi Võ Hiệp 2 — Không cần Studio!"
    ], "#E7B15B", "#19222B", "#E7B15B"),
    node_box(938, 298, 426, 188, "Release Package #2: Auto_Kingdom_SLG/", [
        "• Auto_Kingdom_SLG.exe (12.51 MB | SHA256: 95732232077c0747...)",
        "• PE RT_ICON (7 layers) + PE RT_RCDATA #7777 (PubKey: acb43ebd...)",
        "• bundle/: manifest + ui-schema (Emerald #10B981, 4 Tasks, 4 Cột OCR)",
        "• bundle/: 1 Workflow + 3 PNG Templates + signature.fpbundle.json",
        "• Modules: ADB Core, Vision/OCR, Account Vault, Watchdog (Proxy OFF)",
        "• data/: runtime_kingdom-slg.db (SQLite WAL độc lập)",
        "code:Chứng minh 1 Studio xuất nhiều Auto Game khác nhau 100%!"
    ], "#3BC38D", "#152524", "#3BC38D"),

    arrow(432, 195, 494, 155, "#B291F7", "arrow-purple", "IPC"),
    arrow(432, 360, 494, 280, "#E7B15B", "arrow-amber", "Build"),
    arrow(685, 348, 685, 362, "#3BC38D", "arrow-green", ""),
    arrow(877, 250, 938, 190, "#E7B15B", "arrow-amber", "Export 1"),
    arrow(877, 310, 938, 380, "#3BC38D", "arrow-green", "Export 2"),
])

# -------------------------------------------------------------------------
# 1B. PART 1B: RELATIONSHIP BETWEEN STUDIO, WORKFLOW ENGINE, BUILDER & MINI RUNTIME (SƠ ĐỒ 1.2)
# -------------------------------------------------------------------------
part1b_body = "\n".join([
    zone(18, 68, 310, 412, "1. FARMERPRO STUDIO IDE", "#1878F5", "#111927", "#38BDF8"),
    node_box(32, 92, 282, 110, "A. Authoring & Design Layer", [
        "• Quản lý Đa dự án GameProjectManifest",
        "• Kéo cắt mẫu ảnh PNG từ Device Lab",
        "• Thiết kế đồ thị Node trên React Flow",
        "• Cấu hình Task Library & 5-State Modules",
        "code:projects/<slug>/{workflows,templates}"
    ], "#1878F5", "#162438", "#38BDF8"),
    node_box(32, 214, 282, 114, "B. Live Lab Testing & Verification", [
        "• Gửi đồ thị JSON xuống Workflow Engine",
        "• Thực thi thật trên LDPlayer/ADB",
        "• Thu thập 19 StepEvents & ảnh Trước/Sau",
        "• Kiểm chứng Safe Tap 18% & WinRT OCR",
        "code:SQLite WAL farmerpro.db (Studio DB)"
    ], "#38BDF8", "#152333", "#38BDF8"),
    node_box(32, 340, 282, 126, "C. Mini Tool UI Schema Designer", [
        "• Thiết kế Branding, Màu sắc, Icon .ico",
        "• Cấu hình cột chỉ số OCR động & Fallback",
        "• Xem trước trên Live Interactive Preview",
        "• Xuất đặc tả sang bộ đóng gói fp-builder",
        "code:bundle/ui-schema.json"
    ], "#E7B15B", "#22211C", "#E7B15B"),

    zone(368, 68, 320, 412, "2. SHARED WORKFLOW ENGINE", "#B291F7", "#131826", "#B291F7"),
    node_box(382, 92, 292, 114, "Core Graph & Vision Crates", [
        "• fp-workflow-schema (DAG Validator)",
        "• fp-workflow-runtime (State Machine)",
        "• fp-vision (Multiscale SSD + WinRT OCR)",
        "• fp-actions (18% Safe Random Tap Policy)",
        "code:100% Mã nguồn Rust dùng chung"
    ], "#B291F7", "#1B1E32", "#B291F7"),
    node_box(382, 218, 292, 116, "Device, Lease & Storage Crates", [
        "• fp-adb (Argv Command, Zero Shell String)",
        "• fp-device (DeviceActor + Gen Fencing)",
        "• fp-scheduler (Exclusive Per-Device Lease)",
        "• fp-storage (SQLite WAL + Online Backup)",
        "code:Chạy giống hệt nhau ở Studio & Mini Tool"
    ], "#38BDF8", "#152333", "#38BDF8"),
    node_box(382, 346, 292, 120, "Contract & Security Guard Crates", [
        "• fp-bundle-contract (Zero-Crypto Schema)",
        "• fp-bundle-verify (PE #7777 Verifier)",
        "• fp-security (CRED_MGR:// Vault & Redact)",
        "code:Liên kết tĩnh vào cả Studio & Runtime"
    ], "#3BC38D", "#142624", "#3BC38D"),

    zone(728, 68, 305, 412, "3. FP-BUILDER (PACKAGER)", "#E7B15B", "#171A22", "#E7B15B"),
    node_box(742, 92, 277, 114, "Pre-Flight & Asset Guard (Step 1-4)", [
        "• Kiểm tra toàn vẹn Workflow & PNGs",
        "• Chặn Path Traversal (../)",
        "• Khóa ma trận Module 5 trạng thái",
        "• Sao chép binary mẫu fp-mini-runtime.exe",
        "code:Chỉ tồn tại trong Studio & fp-cli"
    ], "#E7B15B", "#22211C", "#E7B15B"),
    node_box(742, 218, 277, 116, "PE Resource & Ed25519 Signer (Step 5-7)", [
        "• Win32 UpdateResourceW nhúng 7-layer Icon",
        "• Nhúng cứng 32-byte PubKey vào PE #7777",
        "• Đọc khóa bí mật từ %LOCALAPPDATA%",
        "• Ký chữ ký số RFC 8032 Ed25519 lên bundle/",
        "code:SigningKey không bao giờ rời máy SX"
    ], "#F27883", "#251920", "#F27883"),
    node_box(742, 346, 277, 120, "Self-Verify & Release Output (Step 8)", [
        "• Đọc lại khóa từ PE RT_RCDATA #7777",
        "• Kiểm tra chữ ký & bảng băm SHA-256",
        "• Đóng gói kèm ./bin/adb.exe + 2 DLLs",
        "code:Xuất build-report.json (đo thời gian từng bước)"
    ], "#3BC38D", "#142624", "#3BC38D"),

    zone(1073, 68, 309, 412, "4. STANDALONE MINI RUNTIME", "#3BC38D", "#101E22", "#3BC38D"),
    node_box(1087, 92, 281, 114, "PE #7777 Gatekeeper Startup", [
        "• Đọc khóa công khai gốc từ PE #7777",
        "• Đối chiếu trusted_publisher.pub",
        "• Xác thực chữ ký Ed25519 của bundle/",
        "• Chặn đứng 5/5 kịch bản Tamper Suite",
        "code:0 phụ thuộc vào fp-builder (cargo tree)"
    ], "#3BC38D", "#142624", "#3BC38D"),
    node_box(1087, 218, 281, 116, "Dynamic UI Schema Renderer", [
        "• Dựng giao diện 100% từ ui-schema.json",
        "• Áp dụng màu thương hiệu & cột OCR động",
        "• Hiển thị trung thực 'Chưa OCR (-)'",
        "• Ẩn hoàn toàn các Module không đóng gói",
        "code:Auto_JX2_Mobile.exe / Auto_Kingdom_SLG"
    ], "#38BDF8", "#152333", "#38BDF8"),
    node_box(1087, 346, 281, 120, "Isolated Execution & NTFS Fallback", [
        "• Chạy quy trình bằng Shared Engine",
        "• Kết nối trực tiếp ./bin/adb.exe",
        "• Tự chuyển SQLite sang %LOCALAPPDATA%",
        "  nếu thư mục cài đặt bị khóa Read-Only",
        "code:Chạy độc lập khi đã đóng Studio!"
    ], "#E7B15B", "#22211C", "#E7B15B"),

    arrow(314, 275, 382, 275, "#38BDF8", "arrow-blue", "Test"),
    arrow(674, 150, 742, 150, "#B291F7", "arrow-purple", "Link"),
    arrow(1019, 275, 1087, 150, "#E7B15B", "arrow-amber", "EXE"),
    # Orthogonal gutter routing along bottom channel (y=495..508) so no arrow crosses over text boxes
    '<path d="M 314 435 L 348 435 L 348 494 L 708 494 L 708 175 L 742 175" fill="none" stroke="#E7B15B" stroke-width="2" marker-end="url(#arrow-amber)"/>',
    '<rect x="442" y="484" width="172" height="18" rx="4" fill="#16202F" stroke="#E7B15B" stroke-width="1"/>',
    '<text x="528" y="497" text-anchor="middle" fill="#E7B15B" font-family="Segoe UI, Inter, Arial, sans-serif" font-size="9.5" font-weight="700">Specs (ui-schema + workflows)</text>',
    '<path d="M 674 435 L 696 435 L 696 508 L 1054 508 L 1054 415 L 1087 415" fill="none" stroke="#3BC38D" stroke-width="2" marker-end="url(#arrow-green)"/>',
    '<rect x="780" y="498" width="214" height="18" rx="4" fill="#16202F" stroke="#3BC38D" stroke-width="1"/>',
    '<text x="887" y="511" text-anchor="middle" fill="#3BC38D" font-family="Segoe UI, Inter, Arial, sans-serif" font-size="9.5" font-weight="700">Compile-Time Link (Zero fp-builder)</text>',
])

# -------------------------------------------------------------------------
# 2. PART 2: BEFORE VS PROPOSED ARCHITECTURE & 10-SCREEN MAPPING
# -------------------------------------------------------------------------
part2_body = "\n".join([
    zone(18, 72, 560, 430, "KIẾN TRÚC CŨ (v0.1.0 - 10 Tab Vận Hành Phẳng & Hardcode JX2)", "#F27883", "#1A141B", "#F27883"),
    node_box(34, 98, 255, 58, "01. Tổng quan (OverviewView)", ["Dashboard vận hành 1 game đơn lẻ", "code:Trùng lặp nút Mở/Tắt App, LDPlayer"], "#752932", "#221820", "#F27883"),
    node_box(305, 98, 255, 58, "02. Thiết bị Android (DevicesView)", ["ADB USB/TCP, Quét LAN, LDPlayer", "code:Chưa có công cụ cắt mẫu ROI"], "#752932", "#221820", "#F27883"),
    node_box(34, 166, 255, 58, "03. Kho Quy trình JX2 (WorkflowsView)", ["Hardcode 10 quy trình Võ Hiệp 2", "code:jx2Workflows.ts gắn cứng"], "#752932", "#221820", "#F27883"),
    node_box(305, 166, 255, 58, "04. Workflow Studio (StudioView)", ["React Flow Canvas + Node Inspector", "code:Chưa gắn với Project Templates"], "#752932", "#221820", "#F27883"),
    node_box(34, 234, 255, 58, "05. Bộ Nhiệm vụ (TasksView)", ["Mảng nhiệm vụ tĩnh tách rời workflow", "code:Chưa đồng bộ sang Mini Tool"], "#752932", "#221820", "#F27883"),
    node_box(305, 234, 255, 58, "06. Phiên chạy (RunsView)", ["Bảng sự kiện bước & Checkpoint", "code:Có fallback job-demo-8492 giả lập"], "#752932", "#221820", "#F27883"),
    node_box(34, 302, 255, 58, "07. Tài khoản Vault (AccountsView)", ["Hồ sơ CRED_MGR:// toàn cục", "code:Chưa phân tách theo Game Project"], "#752932", "#221820", "#F27883"),
    node_box(305, 302, 255, 58, "08. Mạng & Proxy (NetworkView)", ["SOCKS5/HTTP & ADB Forward rời", "code:Nằm tách biệt trên Sidebar chính"], "#752932", "#221820", "#F27883"),
    node_box(34, 370, 255, 58, "09. Design Tokens (GalleryView)", ["Bảng màu & Rubric chiếm 1 tab chính", "code:Nên đưa vào System Settings"], "#752932", "#221820", "#F27883"),
    node_box(305, 370, 255, 58, "10. Cài đặt & ADB (SettingsView)", ["Chẩn đoán ADB & SQLite WAL", "code:Thiếu quản lý khóa ký Ed25519"], "#752932", "#221820", "#F27883"),
    node_box(34, 438, 526, 52, "Giới hạn cốt lõi của Kiến trúc Cũ", [
        "❌ Không có khái niệm GameProject | ❌ Không có giao diện Mini Tool Builder & Release Management"
    ], "#F27883", "#2A151D", "#F27883"),

    zone(660, 72, 720, 430, "KIẾN TRÚC ĐỀ XUẤT P1.2 (Context-Driven Multi-Game Studio IDE — Phân Loại Thực Tế)", "#3BC38D", "#101D24", "#3BC38D"),
    node_box(678, 98, 684, 46, "Global Context Header (44px): [Game Project Switcher ▾] + [Active Lab Device ▾] + [⚙️ System Settings]", [
        "Loại bỏ hoàn toàn nút Mở/Tắt App trùng lặp trên Header — Mọi khu vực tự động lọc theo Project đang chọn"
    ], "#1878F5", "#16263B", "#38BDF8"),
    node_box(678, 154, 334, 78, "1. Game Projects / Workspace", [
        "◀ Ánh xạ từ: [01. Tổng quan] + [03. Kho Quy trình]",
        "• Quản lý đa dự án game, Package ID, Assets, Templates",
        "code:Migrate 10 quy trình JX2 vào project jx2-mobile (P2)"
    ], "#2A3647", "#16202F", "#1878F5"),
    node_box(1028, 154, 334, 78, "2. Workflow Studio (IDE Core)", [
        "◀ Ánh xạ từ: [04. Workflow Studio] + [03. Kho Quy trình]",
        "• Project Workflow Explorer + Node Palette + React Flow",
        "code:+ Cầu nối 'Đưa vào Task Library' & 'Cắt mẫu từ Lab'"
    ], "#2A3647", "#16202F", "#B291F7"),
    node_box(678, 242, 334, 78, "3. Tasks & Modules (Scoped to Project)", [
        "◀ Ánh xạ từ: [05. Bộ Nhiệm vụ] + [07. Vault] + [08. Proxy]",
        "• Task Library động + Sub-tabs: Vault, Proxy, Watchdog",
        "code:Giữ UI Vault/Proxy/Forward; Backend Mock -> Thật ở P2–P3"
    ], "#2A3647", "#16202F", "#3BC38D"),
    node_box(1028, 242, 334, 78, "4. Device Lab (Hardware & Vision Lab)", [
        "◀ Ánh xạ từ: [02. Thiết bị Android] + [01. Điều khiển LD]",
        "• Giữ Quét LAN, Auto-Discover, LDPlayer, Packages",
        "code:+ Mới: On-Demand Screen Inspector & ROI Cropper"
    ], "#2A3647", "#16202F", "#38BDF8"),
    node_box(678, 330, 334, 78, "5. Test & Debug (Real Evidence Center)", [
        "◀ Ánh xạ từ: [06. Phiên chạy]",
        "• Bảng StepEvent thật từ SQLite + Ảnh Trước/Sau + OCR",
        "code:Xóa bỏ hoàn toàn dữ liệu giả lập job-demo-8492"
    ], "#2A3647", "#16202F", "#E7B15B"),
    node_box(1028, 330, 334, 78, "6. Mini Tool Builder & Releases (MỚI 100%)", [
        "◀ Phân hệ mới tích hợp fp-builder & fp-bundle-contract",
        "• Thiết kế UI Schema, chọn 5-State Modules, Live Preview",
        "code:Pre-Flight 6 điểm -> Build EXE 8 bước -> Smoke Test"
    ], "#E7B15B", "#1F2520", "#E7B15B"),
    node_box(678, 418, 684, 72, "⚙️ System Settings Drawer (Mở từ Header/Sidebar Footer)", [
        "◀ Ánh xạ từ: [09. Design Tokens] + [10. Cài đặt & ADB]",
        "• Tab 1: Đường dẫn ADB & Giả lập | Tab 2: Producer Ed25519 Keystore (Rotate/Fingerprint)",
        "code:• Tab 3: SQLite Online Backup & Integrity Check | Tab 4: Design Tokens & IDE Rubric"
    ], "#2A3647", "#141D2A", "#38BDF8"),

    arrow(578, 190, 660, 190, "#38BDF8", "arrow-blue", "Refactor"),
    arrow(578, 280, 660, 280, "#3BC38D", "arrow-green", "Map UI"),
    arrow(578, 390, 660, 440, "#E7B15B", "arrow-amber", "Settings"),
])

# -------------------------------------------------------------------------
# 3. IV.1: RUST CARGO WORKSPACE DEPENDENCY GRAPH
# -------------------------------------------------------------------------
iv01_body = "\n".join([
    zone(18, 72, 1364, 96, "LAYER 1: APPLICATION BINARIES (apps/*)", "#38BDF8", "#111A27", "#38BDF8"),
    node_box(36, 94, 420, 64, "apps/desktop (FarmerPro.exe - Studio IDE)", [
        "Tauri v2 Desktop Host + Studio IPC Commands + Builder UI Host",
        "code:Phụ thuộc cả Runtime Core lẫn fp-builder (Ký & Xuất gói)"
    ], "#1878F5", "#162234", "#1878F5"),
    node_box(486, 94, 410, 64, "apps/cli (fp-cli.exe - Diagnostic & Build Runner)", [
        "CLI kiểm thử tự động, export-poc, live ADB runner, keygen",
        "code:Phụ thuộc Runtime Core + fp-builder"
    ], "#2A3647", "#16202F", "#38BDF8"),
    node_box(926, 94, 438, 64, "apps/mini-runtime (fp-mini-runtime.exe -> Auto_<Game>.exe)", [
        "Binary độc lập giao cho người dùng cuối của từng tựa game",
        "code:CẤM PHỤ THUỘC fp-builder (Đã kiểm chứng bằng cargo tree)"
    ], "#3BC38D", "#122624", "#3BC38D"),

    zone(18, 192, 1364, 114, "LAYER 2: BUNDLE & PACKAGING", "#E7B15B", "#161B24", "#E7B15B"),
    node_box(36, 214, 420, 82, "crates/fp-builder (Producer Packager & Signer)", [
        "• MiniToolPackager (Pipeline đóng gói 8 bước)",
        "• ProducerKeyManager (Đọc khóa bí mật từ %LOCALAPPDATA%)",
        "• pe_icon.rs: Win32 UpdateResourceW (PE Icon + RT_RCDATA #7777)"
    ], "#E7B15B", "#22211C", "#E7B15B"),
    node_box(486, 214, 410, 82, "crates/fp-bundle-verify (PE Root of Trust & Verifier)", [
        "• Đọc khóa công khai gốc từ PE RT_RCDATA #7777 của tiến trình",
        "• Xác thực chữ ký RFC 8032 Ed25519 (ed25519-dalek VerifyingKey)",
        "• Chống Path Traversal (validate_safe_relative_path)"
    ], "#3BC38D", "#142424", "#3BC38D"),
    node_box(926, 214, 438, 82, "crates/fp-bundle-contract (Zero-Crypto Data Schemas)", [
        "• GameProjectManifest, MiniToolUiSchema, ReleaseBundleManifest",
        "• Mô hình Module 5 Trạng thái (ModuleRuntimeStatus)",
        "• Kiểm toán phụ thuộc & NTFS Read-Only Fallback (%LOCALAPPDATA%)"
    ], "#38BDF8", "#152232", "#38BDF8"),

    zone(18, 322, 1364, 180, "LAYER 3: SHARED AUTOMATION CORE CRATES (Dùng Chung Cho Mọi Tựa Game — Không Cần Sửa Code)", "#B291F7", "#121824", "#B291F7"),
    node_box(36, 346, 250, 66, "crates/fp-workflow-runtime", ["State Machine Executor, Retry,", "Checkpoints, WinRT OCR, Safe Tap"], "#2A3647", "#16202F", "#B291F7"),
    node_box(304, 346, 250, 66, "crates/fp-workflow-schema", ["Versioned Graph JSON Validator,", "Start/End & Edge Integrity"], "#2A3647", "#16202F", "#B291F7"),
    node_box(572, 346, 250, 66, "crates/fp-scheduler", ["Multi-Device Exclusive Leases,", "Priority Queue, Fencing Tokens"], "#2A3647", "#16202F", "#38BDF8"),
    node_box(840, 346, 250, 66, "crates/fp-device", ["DeviceActor Mailbox, Geometry,", "Rotation Transform, Fencing"], "#2A3647", "#16202F", "#38BDF8"),
    node_box(1108, 346, 256, 66, "crates/fp-adb", ["Tokio Command Runner (No Shell),", "USB/TCP Discovery, Screencap"], "#2A3647", "#16202F", "#38BDF8"),

    node_box(36, 424, 250, 66, "crates/fp-vision", ["Multiscale SSD Template Match,", "Bounding Box Normalization"], "#2A3647", "#16202F", "#3BC38D"),
    node_box(304, 424, 250, 66, "crates/fp-actions", ["Safe Random Tap Policy (18%),", "Exclusion Masks, Swipe Gen"], "#2A3647", "#16202F", "#3BC38D"),
    node_box(572, 424, 250, 66, "crates/fp-storage", ["SQLite WAL Mode, Migrations,", "Jobs, StepEvents, Checkpoints"], "#2A3647", "#16202F", "#E7B15B"),
    node_box(840, 424, 250, 66, "crates/fp-security", ["CRED_MGR:// Vault Abstraction,", "Secret Redaction, Guards"], "#2A3647", "#16202F", "#F27883"),
    node_box(1108, 424, 256, 66, "crates/fp-domain & fp-testkit", ["Core IDs, Rect/Point, Traits,", "Deterministic Fake Driver"], "#2A3647", "#16202F", "#8FA2BC"),

    arrow(340, 158, 340, 214, "#E7B15B", "arrow-amber", "Uses Builder"),
    arrow(560, 158, 420, 214, "#E7B15B", "arrow-amber", "CLI Build"),
    arrow(1020, 158, 760, 214, "#3BC38D", "arrow-green", "Verify Only"),
    arrow(1210, 158, 1210, 214, "#38BDF8", "arrow-blue", "UI Contract"),
    arrow(926, 145, 456, 225, "#F27883", "arrow-red", "FORBIDDEN (0 Dep)", dashed=True),
    arrow(456, 255, 486, 255, "#3BC38D", "arrow-green", ""),
    arrow(896, 255, 926, 255, "#38BDF8", "arrow-blue", ""),
])

# -------------------------------------------------------------------------
# 4. IV.2: FARMERPRO STUDIO ARCHITECTURE
# -------------------------------------------------------------------------
iv02_body = "\n".join([
    zone(18, 72, 544, 430, "FRONTEND: TAURI V2 WEBVIEW (React 19 + TypeScript + Vite + Tailwind v4)", "#38BDF8", "#111927", "#38BDF8"),
    node_box(34, 98, 512, 52, "Global Context Header & Navigation Rail", [
        "Project Switcher (Active GameProject) | Lab Device Selector | 6 Workspace Tabs | System Drawer"
    ], "#1878F5", "#172538", "#38BDF8"),
    node_box(34, 160, 250, 82, "1. Game Projects & Assets UI", ["Project Cards, Manifest Form,", "Template PNG Gallery, Health"], "#2A3647", "#16202F", "#1878F5"),
    node_box(296, 160, 250, 82, "2. Workflow Studio (React Flow)", ["Collapsible Palette, Graph Canvas,", "Node Inspector, Execution Dock"], "#2A3647", "#16202F", "#B291F7"),
    node_box(34, 252, 250, 82, "3. Tasks & Modules Config UI", ["Task Library, Priority Queue,", "Vault, SOCKS5 Proxy, Watchdog"], "#2A3647", "#16202F", "#3BC38D"),
    node_box(296, 252, 250, 82, "4. Device Lab & ROI Cropper", ["ADB Fleet, LDPlayer Control,", "Interactive PNG Cropper & OCR"], "#2A3647", "#16202F", "#38BDF8"),
    node_box(34, 344, 250, 82, "5. Test & Debug Inspector", ["Live Step Trace, Before/After", "Safe Tap Overlay, Checkpoints"], "#2A3647", "#16202F", "#E7B15B"),
    node_box(296, 344, 250, 82, "6. Mini Tool Builder & Release", ["Form Layout Designer, Preview,", "Pre-Flight Audit, 8-Step Build"], "#E7B15B", "#21221E", "#E7B15B"),
    node_box(34, 436, 512, 54, "Zustand Reactive Studio Store (useStudioStore.ts)", [
        "Đồng bộ trạng thái activeProjectId, activeDeviceId, workflows, tasks, uiSchema, buildReports"
    ], "#38BDF8", "#142030", "#38BDF8"),

    zone(596, 72, 224, 430, "TAURI V2 TYPED IPC BRIDGE", "#B291F7", "#151828", "#B291F7"),
    node_box(610, 104, 196, 380, "11 Studio IPC Commands", [
        "code:studio_list_projects",
        "code:studio_create_project",
        "code:studio_update_project",
        "code:studio_list_project_tasks",
        "code:studio_upsert_project_task",
        "code:studio_save_ui_schema",
        "code:studio_crop_save_template",
        "code:studio_test_template_match",
        "code:studio_preflight_check",
        "code:studio_build_mini_release",
        "code:studio_smoke_test_release",
        "-------------------------",
        "+ Existing ADB, Device,",
        "  Workflow, Vault & Proxy",
        "  Tauri Commands",
        "-------------------------",
        "Zero Raw Shell Strings;",
        "Typed Serde JSON Payloads"
    ], "#B291F7", "#1A1F33", "#B291F7"),

    zone(856, 72, 526, 430, "BACKEND: RUST CORE ENGINE (apps/desktop/src-tauri + 14 Crates)", "#3BC38D", "#101E24", "#3BC38D"),
    node_box(872, 98, 494, 86, "Multi-Project Persistence Service (fp-storage)", [
        "• SQLite WAL Mode (farmerpro.db v2 Schema: projects, project_modules,",
        "  vision_templates, task_definitions, mini_tool_schemas, project_releases)",
        "• Online Backup API + PRAGMA integrity_check + Atomic Transactions"
    ], "#3BC38D", "#152628", "#3BC38D"),
    node_box(872, 196, 494, 92, "Device Lab, Vision & Workflow Runtime Service", [
        "• fp-adb + fp-device: Quản lý thiết bị, chụp screencap PNG theo yêu cầu",
        "• fp-vision + fp-actions: Cắt mẫu ROI, khớp mẫu SSD đa tỉ lệ, WinRT OCR",
        "• fp-workflow-runtime + fp-scheduler: Chạy kiểm thử bước, ghi StepEvent thật"
    ], "#38BDF8", "#152232", "#38BDF8"),
    node_box(872, 300, 494, 92, "Mini Tool Packager & Security Service (fp-builder + fp-security)", [
        "• ProducerKeyManager: Quản lý khóa ký Ed25519 tại %LOCALAPPDATA%",
        "• MiniToolPackager: Đóng gói 8 bước, nhúng PE Icon + RT_RCDATA #7777",
        "• Smoke Test Runner: Gọi tiến trình con --verify-bundle & --test-tamper-suite"
    ], "#E7B15B", "#22211C", "#E7B15B"),
    node_box(872, 404, 494, 84, "Filesystem Project & Release Artifact Tree", [
        "• projects/<slug>/{project.fpproj.json, ui-schema.json, workflows/, templates/}",
        "• releases/<slug>/Auto_<Game>.exe + bundle/ + bin/ + data/ + build-report.json"
    ], "#2A3647", "#16202F", "#8FA2BC"),

    arrow(546, 290, 610, 290, "#B291F7", "arrow-purple", "invoke"),
    arrow(806, 150, 872, 150, "#3BC38D", "arrow-green", "SQLite"),
    arrow(806, 245, 872, 245, "#38BDF8", "arrow-blue", "Vision"),
    arrow(806, 345, 872, 345, "#E7B15B", "arrow-amber", "Build"),
])

# -------------------------------------------------------------------------
# 5. IV.3: GAMEPROJECT DATA MODEL (SQLITE MIGRATION V1 -> V2)
# -------------------------------------------------------------------------
iv03_body = "\n".join([
    zone(18, 72, 1364, 430, "SQLITE WAL MULTI-PROJECT SCHEMA V2 (Tương Thích Ngược 100% — Không Mất Dữ Liệu v1)", "#38BDF8", "#101824", "#38BDF8"),
    node_box(490, 94, 400, 150, "TABLE: projects (Root Entity - Mới ở P2)", [
        "code:id TEXT PRIMARY KEY       -- 'proj-jx2-mobile-001'",
        "code:slug TEXT UNIQUE NOT NULL -- 'jx2-mobile'",
        "code:name TEXT NOT NULL        -- 'Võ Hiệp Truyền Kỳ 2'",
        "code:game_package_id TEXT      -- 'com.blackhole.jx2mobile'",
        "code:version TEXT, exe_name TEXT, window_title TEXT",
        "code:icon_rel_path TEXT, target_width_px INT, target_height_px INT",
        "code:bundle_adb INT, project_dir_path TEXT, updated_at TEXT"
    ], "#1878F5", "#16253A", "#38BDF8"),

    node_box(36, 94, 390, 118, "TABLE: project_modules (5-State Config - Mới)", [
        "code:project_id TEXT REFERENCES projects(id)",
        "code:module_id TEXT -- 'mod.adb_core', 'mod.account_vault'...",
        "code:packaged_in_release INT NOT NULL DEFAULT 1",
        "code:enabled_by_default INT NOT NULL DEFAULT 1",
        "code:config_json TEXT, PRIMARY KEY (project_id, module_id)"
    ], "#3BC38D", "#142426", "#3BC38D"),

    node_box(36, 230, 390, 128, "TABLE: vision_templates (Project PNGs - Mới)", [
        "code:id TEXT PRIMARY KEY",
        "code:project_id TEXT REFERENCES projects(id)",
        "code:name TEXT, rel_path TEXT -- 'templates/sys_apps.png'",
        "code:width_px INT, height_px INT, source_crop_x/y INT",
        "code:default_confidence REAL, sha256 TEXT NOT NULL"
    ], "#B291F7", "#1A1D30", "#B291F7"),

    node_box(36, 376, 390, 112, "TABLE: mini_tool_schemas (UI Schema - Mới)", [
        "code:project_id TEXT PRIMARY KEY REFERENCES projects(id)",
        "code:schema_version INT NOT NULL DEFAULT 1",
        "code:ui_schema_json TEXT NOT NULL -- MiniToolUiSchema",
        "code:updated_at TEXT NOT NULL"
    ], "#E7B15B", "#22211C", "#E7B15B"),

    node_box(954, 94, 410, 126, "TABLE: task_definitions (Task Library - Mới)", [
        "code:id TEXT PRIMARY KEY",
        "code:project_id TEXT REFERENCES projects(id)",
        "code:workflow_id TEXT REFERENCES workflows(id)",
        "code:category_id, category_short_tag, category_color_hex",
        "code:title TEXT, priority INT, estimate_sec INT",
        "code:settings_schema_json TEXT -- Dynamic task fields"
    ], "#3BC38D", "#142426", "#3BC38D"),

    node_box(954, 240, 410, 126, "TABLE: project_releases (Build History - Mới)", [
        "code:id TEXT PRIMARY KEY, project_id TEXT REFERENCES projects(id)",
        "code:version TEXT, output_dir_path TEXT, exe_path TEXT",
        "code:exe_size_bytes INT, exe_sha256 TEXT, key_id TEXT",
        "code:publisher_public_key_hex TEXT, pe_rcdata_pubkey_id INT (7777)",
        "code:signature_ed25519_hex TEXT, build_report_json TEXT"
    ], "#E7B15B", "#22211C", "#E7B15B"),

    node_box(490, 268, 400, 102, "EXISTING TABLES: workflows & workflow_versions", [
        "code:id TEXT PRIMARY KEY, name TEXT, current_version INT",
        "code:+ ALTER ADD project_id TEXT REFERENCES projects(id)",
        "code:+ ALTER ADD category TEXT NOT NULL DEFAULT 'Chung'",
        "Auto-migrates all existing JX2 workflows to 'proj-jx2-mobile-001'"
    ], "#2A3647", "#16202F", "#38BDF8"),

    node_box(490, 394, 400, 94, "EXISTING TABLES: accounts, jobs, step_events, checkpoints", [
        "code:accounts: + ALTER ADD project_id TEXT REFERENCES projects(id)",
        "code:jobs:     + ALTER ADD project_id TEXT REFERENCES projects(id)",
        "step_events & checkpoints link via job_id -> jobs(id)"
    ], "#2A3647", "#16202F", "#F27883"),

    arrow(490, 145, 426, 145, "#3BC38D", "arrow-green", "1 : N"),
    arrow(490, 200, 426, 285, "#B291F7", "arrow-purple", "1 : N"),
    arrow(490, 232, 426, 425, "#E7B15B", "arrow-amber", "1 : 1"),
    arrow(890, 145, 954, 145, "#3BC38D", "arrow-green", "1 : N"),
    arrow(890, 215, 954, 290, "#E7B15B", "arrow-amber", "1 : N"),
    arrow(690, 244, 690, 268, "#38BDF8", "arrow-blue", "1:N"),
    arrow(890, 315, 954, 190, "#38BDF8", "arrow-blue", "wf_id"),
    arrow(690, 370, 690, 394, "#F27883", "arrow-red", "proj_id"),
])

# -------------------------------------------------------------------------
# 6. IV.4: WORKFLOW RUNTIME EXECUTION ARCHITECTURE
# -------------------------------------------------------------------------
iv04_body = "\n".join([
    zone(18, 72, 1364, 240, "STATE MACHINE EXECUTOR PIPELINE (crates/fp-workflow-runtime)", "#B291F7", "#121825", "#B291F7"),
    node_box(34, 112, 170, 135, "1. flow.start", [
        "Khởi tạo JobContext",
        "Kiểm tra Device Lease",
        "Ghi StepEvent:",
        "code:command_sent",
        "code:device_ack"
    ], "#2A3647", "#16202F", "#38BDF8"),
    node_box(248, 112, 185, 135, "2. vision.capture", [
        "Gọi AdbRunner:",
        "exec-out screencap -p",
        "Giải mã PNG (1280x720)",
        "Tính SHA-256 Frame",
        "code:Zero Video Stream"
    ], "#2A3647", "#16202F", "#B291F7"),
    node_box(478, 112, 198, 135, "3. vision.find_image", [
        "Multiscale SSD Matcher",
        "So khớp mẫu PNG",
        "Kiểm tra score >= minConf",
        "Trả về BoundingBox:",
        "code:[x:74, y:135, 88x88]"
    ], "#B291F7", "#1B1E32", "#B291F7"),
    node_box(726, 112, 198, 135, "4. action.random_safe_tap", [
        "Thu hẹp biên an toàn 18%",
        "Trừ vùng Exclusion Mask",
        "Lấy mẫu tọa độ ngẫu nhiên",
        "Gửi lệnh ADB:",
        "code:input tap 111 170"
    ], "#3BC38D", "#142624", "#3BC38D"),
    node_box(974, 112, 178, 135, "5. vision.ocr", [
        "Chụp khung hình mới",
        "Gọi WinRT OcrEngine",
        "Trích xuất văn bản",
        "Cập nhật biến Runtime:",
        "code:ocr_text / char_gold"
    ], "#38BDF8", "#152334", "#38BDF8"),
    node_box(1196, 112, 170, 135, "6. state.checkpoint", [
        "Lưu toàn bộ biến &",
        "node_id vào SQLite WAL",
        "Cho phép Resume khi",
        "mất kết nối đột ngột",
        "code:checkpoint_saved"
    ], "#E7B15B", "#22211C", "#E7B15B"),

    arrow(204, 178, 248, 178, "#38BDF8", "arrow-blue", "init"),
    arrow(433, 178, 478, 178, "#B291F7", "arrow-purple", "PNG"),
    arrow(676, 178, 726, 178, "#3BC38D", "arrow-green", "1.00"),
    arrow(924, 178, 974, 178, "#38BDF8", "arrow-blue", "1.2s"),
    arrow(1152, 178, 1196, 178, "#E7B15B", "arrow-amber", "vars"),

    zone(18, 330, 1364, 172, "RETRY, BACKOFF, FENCING & AUDIT LOGGING GUARANTEES", "#38BDF8", "#101722", "#38BDF8"),
    node_box(34, 358, 425, 126, "Cơ chế Thử lại & Chống kẹt (Retry & Backoff Engine)", [
        "• Nếu vision.find_image có score < minConfidence (VD: < 0.90):",
        "• Kiểm tra attempt_no <= max_retries (mặc định 3 lần, backoff 800ms)",
        "• Tự động quay lại chụp khung hình mới trước khi đánh dấu step_failed",
        "code:Đã kiểm chứng: test_executor_retry_logic (PASS)"
    ], "#E7B15B", "#1F201D", "#E7B15B"),
    node_box(482, 358, 435, 126, "Chính sách Chạm Ngẫu Nhiên An Toàn (Safe Random Tap Policy)", [
        "• Tuyệt đối KHÔNG hardcode tọa độ cố định (AGENTS.md Rule #5)",
        "• Hộp khớp [74, 135, 88, 88] + marginRatio 0.18 -> Vùng an toàn [90..146, 151..207]",
        "• Điểm chạm thực tế trên LDPlayer: (111, 170) nằm gọn trong tâm an toàn",
        "code:Đã kiểm chứng: test_safe_margin_shrink & Live ADB (PASS)"
    ], "#3BC38D", "#142423", "#3BC38D"),
    node_box(940, 358, 426, 126, "Nhật ký Sự kiện Bước Toàn diện (SQLite WAL StepEvents)", [
        "• Mỗi Node phát sinh chuỗi sự kiện: command_sent -> device_ack ->",
        "  verification_passed / step_failed -> checkpoint_saved",
        "• Phiên chạy thật trên emulator-5554 ghi nhận đủ 19/19 sự kiện chuẩn xác",
        "code:Bằng chứng: 09_live_adb_workflow_execution.json"
    ], "#38BDF8", "#152231", "#38BDF8"),
])

# -------------------------------------------------------------------------
# 7. IV.5: ADB / EMULATOR ADAPTER ARCHITECTURE
# -------------------------------------------------------------------------
iv05_body = "\n".join([
    zone(18, 72, 864, 430, "RUST HOST ADB & EMULATOR DRIVER STACK (crates/fp-adb + crates/fp-device)", "#38BDF8", "#111927", "#38BDF8"),
    node_box(36, 100, 402, 118, "1. DeviceActor & Generation Fencing (fp-device)", [
        "• Mỗi thiết bị Android có 1 Actor riêng xử lý tuần tự lệnh",
        "• Cấp phát `generation: u64` tăng dần mỗi lần acquire lease",
        "• Mọi lệnh mang generation cũ bị từ chối ngay lập tức (Stale Command Guard)",
        "code:test_generation_fencing & test_device_actor_operations (PASS)"
    ], "#38BDF8", "#152334", "#38BDF8"),
    node_box(462, 100, 402, 118, "2. Geometry & Orientation Normalizer (fp-device + fp-actions)", [
        "• Truy vấn `wm size` (1280x720) và chiều xoay màn hình (0/90/180/270 độ)",
        "• Chuyển đổi hệ tọa độ chuẩn hóa giữa Portrait và Landscape",
        "• Đảm bảo tọa độ mẫu Vision khớp chính xác với tọa độ `input tap`",
        "code:test_point_rotation_and_landscape_transform (PASS)"
    ], "#B291F7", "#1B1E32", "#B291F7"),
    node_box(36, 236, 828, 120, "3. Secure AdbRunner — Zero Raw Shell Interpolation (crates/fp-adb — AGENTS.md Rule #3)", [
        "• Thực thi hoàn toàn qua `tokio::process::Command::new(&adb_path).args(&[\"-s\", serial, ...])`",
        "• Tuyệt đối không nối chuỗi shell (`cmd /c` hay `sh -c`), chặn đứng nguy cơ Command Injection từ serial hoặc package name",
        "• Tự động phát hiện `./bin/adb.exe` đóng gói kèm Mini Tool trước khi tìm trong PATH hệ thống",
        "code:Bằng chứng: test_discover_adb_path & test_parse_adb_devices_output (PASS)"
    ], "#3BC38D", "#132524", "#3BC38D"),
    node_box(36, 374, 402, 112, "4. Unified USB / TCP & LAN IP Scanner", [
        "• Quét cổng giả lập cục bộ (127.0.0.1:5555..5585)",
        "• Quét dải IP LAN (`scanRange`) kết nối máy thật qua Wi-Fi ADB",
        "code:adb connect <ip>:<port> với timeout kiểm soát chặt"
    ], "#2A3647", "#16202F", "#1878F5"),
    node_box(462, 374, 402, 112, "5. Emulator Instance Adapter (LDPlayer / Nox / BlueStacks)", [
        "• Điều khiển vòng đời giả lập: Launch / Quit / Reboot instance",
        "• Quản lý ứng dụng trên thiết bị: `monkey`/`am start`, `am force-stop`",
        "code:Tích hợp tập trung tại khu vực 4. Device Lab"
    ], "#E7B15B", "#22211C", "#E7B15B"),

    zone(918, 72, 464, 430, "PHYSICAL & EMULATOR ANDROID TARGETS", "#3BC38D", "#101E22", "#3BC38D"),
    node_box(936, 104, 428, 180, "Verified Live Target: LDPlayer Android 14 (emulator-5554)", [
        "• Model: Samsung SM_X710N | Android 14 (API 34)",
        "• Resolution: 720x1280 / 1280x720 | State: device (ONLINE)",
        "• Screencap PNG: 08_adb_before_screen.png (SHA: 685df...)",
        "• Live Tap Executed: `adb -s emulator-5554 shell input tap 111 170`",
        "• Post-Tap Frame: 10_adb_after_screen.png (Hộp thoại System Apps mở)",
        "code:Bằng chứng vật lý thật 100% trong artifacts/p1_1_evidence/"
    ], "#3BC38D", "#142725", "#3BC38D"),
    node_box(936, 304, 428, 180, "Multi-Instance & Physical Device Fleet", [
        "• Instance #2: emulator-5556 (Secondary LDPlayer / SLG Farm)",
        "• Physical USB/TCP Phone: 192.168.1.x:5555",
        "• Cách ly hoàn toàn luồng lệnh theo `-s <serial>`",
        "• Mỗi thiết bị được bảo vệ bởi Device Lease & Generation Token riêng",
        "code:test_multi_device_concurrent_isolation_and_fencing (PASS)"
    ], "#38BDF8", "#152333", "#38BDF8"),

    arrow(864, 290, 936, 195, "#3BC38D", "arrow-green", "ADB Argv"),
    arrow(864, 420, 936, 385, "#38BDF8", "arrow-blue", "Multi-ID"),
])

# -------------------------------------------------------------------------
# 8. IV.6: TASK SCHEDULER & MULTI-DEVICE EXECUTION
# -------------------------------------------------------------------------
iv06_body = "\n".join([
    zone(18, 72, 390, 430, "1. PROJECT TASK QUEUE & PRIORITIES", "#38BDF8", "#111927", "#38BDF8"),
    node_box(34, 102, 358, 86, "Task #1 [NV] Chính Tuyến & Dã Tẩu (Priority 1)", [
        "Workflow: wf-jx2-main-quest | Est: 180s",
        "Params: auto_skip_dialog=true, max_quest_loops=10",
        "code:Hạng ưu tiên cao nhất trong BinaryHeap"
    ], "#1878F5", "#162438", "#38BDF8"),
    node_box(34, 202, 358, 86, "Task #2 [NV] Kiểm tra ADB Thật + OCR (Priority 2)", [
        "Workflow: wf-jx2-live-adb-probe | Est: 15s",
        "Nodes: capture -> find_image -> safe_tap -> ocr -> cp",
        "code:Đã kiểm chứng thực tế trên emulator-5554"
    ], "#3BC38D", "#142624", "#3BC38D"),
    node_box(34, 302, 358, 86, "Task #3 [PB] Sơn Tặc Mật Quật (Priority 3)", [
        "Workflow: wf-jx2-dungeon | Est: 300s",
        "Params: difficulty='Anh Hùng', auto_heal_pct=45",
        "code:Chờ Lease thiết bị giải phóng"
    ], "#B291F7", "#1B1E32", "#B291F7"),
    node_box(34, 402, 358, 84, "Task #4 [Nông Nghiệp] Thu Hoạch SLG (Project #2)", [
        "Workflow: wf-slg-harvest-and-patrol | Est: 90s",
        "code:Chạy độc lập trên tiến trình Auto_Kingdom_SLG.exe"
    ], "#E7B15B", "#22211C", "#E7B15B"),

    zone(436, 72, 446, 430, "2. SCHEDULER, LEASE MANAGER & FENCING (crates/fp-scheduler)", "#E7B15B", "#161B24", "#E7B15B"),
    node_box(454, 102, 410, 118, "Priority Dispatcher (BinaryHeap<ScheduledJob>)", [
        "• Sắp xếp nhiệm vụ theo `priority ASC`, sau đó theo thời điểm enqueue",
        "• Hỗ trợ hủy nhiệm vụ tức thì (`CancellationToken` / `stop_active_job`)",
        "• Không bao giờ cấp phát 2 job cùng lúc lên 1 serial thiết bị",
        "code:test_priority_queue_ordering (PASS)"
    ], "#E7B15B", "#22211C", "#E7B15B"),
    node_box(454, 236, 410, 130, "Exclusive Device Lease & Generation Token Table", [
        "• Bảng khóa thiết bị: `HashMap<DeviceId, LeaseRecord>`",
        "• Khi cấp Lease mới cho `emulator-5554`: tăng `generation_token` (VD: #14 -> #15)",
        "• Nếu luồng cũ (Gen #14) bị treo mạng và cố gửi lệnh tap trễ, `DeviceActor`",
        "  đối chiếu `cmd.gen (14) != current_gen (15)` -> TỪ CHỐI NGAY LẬP TỨC!",
        "code:test_device_lease_exclusion_and_fencing (PASS)"
    ], "#F27883", "#251922", "#F27883"),
    node_box(454, 382, 410, 104, "SQLite WAL Lease & Checkpoint Persistence", [
        "• Lưu trạng thái Lease và Checkpoint vào bảng `device_leases` & `checkpoints`",
        "• Tự động thu hồi Lease mồ côi khi khởi động lại ứng dụng"
    ], "#38BDF8", "#152333", "#38BDF8"),

    zone(916, 72, 466, 430, "3. ISOLATED MULTI-DEVICE TOKIO WORKERS", "#3BC38D", "#101E22", "#3BC38D"),
    node_box(934, 102, 430, 180, "Worker Thread A -> Target: emulator-5554", [
        "• Holder: Job `mini-job-cf950745` | Lease Generation: #15",
        "• State: RUNNING -> SUCCEEDED (19 StepEvents)",
        "• Tuyệt đối không bị can thiệp bởi các job đang chờ trong hàng đợi",
        "• Giải phóng Lease ngay khi kết thúc hoặc khi bấm `Dừng khẩn`",
        "code:Cách ly tài nguyên 100% theo DeviceId"
    ], "#3BC38D", "#142725", "#3BC38D"),
    node_box(934, 304, 430, 182, "Worker Thread B -> Target: emulator-5556", [
        "• Holder: Job `slg-job-8821a` | Lease Generation: #08",
        "• Thực thi song song trên thiết bị giả lập thứ 2",
        "• Có bộ biến Runtime (`ocr_text`, `food_stock`) và Checkpoint riêng biệt",
        "• Không chia sẻ trạng thái mutable với Worker A",
        "code:test_multi_device_concurrent_isolation_and_fencing (PASS)"
    ], "#38BDF8", "#152333", "#38BDF8"),

    arrow(392, 150, 454, 150, "#38BDF8", "arrow-blue", "Enqueue"),
    arrow(392, 245, 454, 180, "#3BC38D", "arrow-green", "Enqueue"),
    arrow(864, 280, 934, 190, "#3BC38D", "arrow-green", "Gen #15"),
    arrow(864, 320, 934, 380, "#38BDF8", "arrow-blue", "Gen #08"),
])

# -------------------------------------------------------------------------
# 9. IV.7: MINI TOOL UI SCHEMA RENDERER
# -------------------------------------------------------------------------
iv07_body = "\n".join([
    zone(18, 72, 406, 430, "1. SIGNED BUNDLE INPUTS (bundle/)", "#E7B15B", "#181A22", "#E7B15B"),
    node_box(34, 102, 374, 186, "bundle/ui-schema.json (MiniToolUiSchema v1)", [
        "code:{",
        "code:  \"schemaVersion\": 1,",
        "code:  \"layoutPreset\": \"compact_2_col\",",
        "code:  \"branding\": { \"accentColor\": \"#F59E0B\", ... },",
        "code:  \"deviceColumns\": [",
        "code:    {\"key\":\"char_level\",\"label\":\"Cấp\",\"fallback\":\"Chưa OCR (-)\"}",
        "code:  ],",
        "code:  \"taskCategories\": [{\"id\":\"quest\",\"shortTag\":\"NV\"}],",
        "code:  \"tasks\": [{\"id\":\"task-jx2-main\",\"settingsSchema\":[...]}]",
        "code:}"
    ], "#E7B15B", "#22211C", "#E7B15B"),
    node_box(34, 306, 374, 180, "bundle/manifest.fpbundle.json & signature.fpbundle.json", [
        "• Đã xác thực chữ ký Ed25519 khớp khóa PE RT_RCDATA #7777",
        "• Danh sách `packagedModules`: quyết định module nào được phép",
        "  hiển thị và khởi tạo trong Mini Tool",
        "• Tuyệt đối không tải bất kỳ giao diện nào trước khi Verify PASS",
        "code:Zero Hardcoded Game Strings in fp-mini-runtime.exe"
    ], "#3BC38D", "#142524", "#3BC38D"),

    zone(456, 72, 424, 430, "2. DYNAMIC SCHEMA RENDERER ENGINE (apps/mini-runtime)", "#38BDF8", "#111927", "#38BDF8"),
    node_box(474, 102, 388, 86, "A. Theme & Branding Injector", [
        "• Áp dụng CSS Variables động: `--accent`, `--header-bg`, `--badge`",
        "• Cập nhật tiêu đề cửa sổ Windows & thông tin Game Package ID",
        "code:JX2 = Amber (#F59E0B) | Kingdom SLG = Emerald (#10B981)"
    ], "#38BDF8", "#152333", "#38BDF8"),
    node_box(474, 200, 388, 92, "B. Truthful Dynamic OCR Device Table Renderer", [
        "• Sinh cột bảng thiết bị từ mảng `deviceColumns` trong schema",
        "• Khi chưa chạy bước `vision.ocr` thật, bắt buộc hiển thị `Chưa OCR (-)`",
        "• Chỉ hiển thị giá trị khi `updatedRuntimeVars` có dữ liệu OCR thật",
        "code:Chống tuyệt đối việc hiển thị chỉ số nhân vật giả lập"
    ], "#3BC38D", "#142624", "#3BC38D"),
    node_box(474, 304, 388, 92, "C. Category Tabs, Task Queue & Dynamic Settings Form", [
        "• Tự động sinh thanh lọc danh mục (`Tất cả`, `[NV]`, `[PB]`, `[HĐ]`...)",
        "• Tự động render form cấu hình theo từng Task (`boolean`, `number`, `select`)",
        "code:Không cần viết code React/HTML riêng cho từng game"
    ], "#B291F7", "#1B1E32", "#B291F7"),
    node_box(474, 408, 388, 80, "D. 5-State Module & Dependency Audit Panel", [
        "• Chỉ hiển thị các module có `packagedInRelease && permittedToInitialize`",
        "• Hiển thị ghi chú kỹ thuật trung thực (VD: giới hạn UDP của SOCKS5 Proxy)"
    ], "#E7B15B", "#22211C", "#E7B15B"),

    zone(916, 72, 466, 430, "3. MULTI-GAME STANDALONE UI OUTPUTS", "#3BC38D", "#101E22", "#3BC38D"),
    node_box(934, 102, 430, 182, "Output UI #1: Auto Võ Hiệp 2 Mobile v1.0.0", [
        "• Theme: Võ Hiệp Amber (#F59E0B) | Layout: compact_2_col",
        "• Cột thiết bị: Thiết bị | Trạng thái | Cấp | Vàng | Bạc",
        "• Nhóm Task: [NV] Nhiệm vụ | [PB] Phó bản | [HĐ] Hoạt động (5 Tasks)",
        "• Modules Active (5/6): ADB, Vision/OCR, Vault, SOCKS5 Proxy, Watchdog",
        "code:Ảnh chụp thực tế: screenshots/p1_poc/p1_1_auto_jx2_truthful.png"
    ], "#E7B15B", "#22201A", "#E7B15B"),
    node_box(934, 304, 430, 182, "Output UI #2: Auto Kingdom Rise SLG v1.0.0", [
        "• Theme: Chiến Thuật Emerald (#10B981) | Layout: compact_2_col",
        "• Cột thiết bị: Thiết bị | Thành Chính | Lương Thực | Gỗ | Quân Lực",
        "• Nhóm Task: [Nông Nghiệp] | [Quân Sự] | [Liên Minh] (4 Tasks)",
        "• Modules Active (4/6): ADB, Vision/OCR, Vault, Watchdog (Proxy ẩn)",
        "code:Ảnh chụp thực tế: screenshots/p1_poc/p1_1_auto_slg_truthful.png"
    ], "#3BC38D", "#142724", "#3BC38D"),

    arrow(408, 195, 474, 150, "#E7B15B", "arrow-amber", "Schema"),
    arrow(408, 380, 474, 440, "#3BC38D", "arrow-green", "5-State"),
    arrow(862, 160, 934, 160, "#E7B15B", "arrow-amber", "Game 1"),
    arrow(862, 350, 934, 350, "#3BC38D", "arrow-green", "Game 2"),
])

# -------------------------------------------------------------------------
# 10. IV.8: MINI TOOL BUILD & RELEASE PIPELINE (8 STEPS)
# -------------------------------------------------------------------------
iv08_body = "\n".join([
    zone(18, 72, 1364, 430, "8-STEP DETERMINISTIC MINI TOOL BUILD & RELEASE PIPELINE (crates/fp-builder)", "#E7B15B", "#131924", "#E7B15B"),
    node_box(34, 104, 315, 165, "STEP 1: Validate Project, Schema & Graphs", [
        "• Kiểm tra `GameProjectManifest` & slug",
        "• Kiểm tra `MiniToolUiSchema` (tasks, cột OCR)",
        "• Chạy `WorkflowValidator` trên toàn bộ",
        "  workflows được liên kết (start/end/edges)",
        "• Thời gian thực tế: < 1 ms",
        "code:Status: PASS (2 workflows JX2 / 1 SLG)"
    ], "#38BDF8", "#152333", "#38BDF8"),
    node_box(372, 104, 315, 165, "STEP 2: Asset Integrity & Path Guard", [
        "• Kiểm tra mọi `templateRef` trong Node Vision",
        "  tồn tại thật trên đĩa (`templates/*.png`)",
        "• Chặn Path Traversal (`../` hoặc đường dẫn",
        "  tuyệt đối) bằng `validate_safe_relative_path`",
        "• Kiểm tra header file `branding/icon.ico`",
        "code:Status: PASS (4 PNGs JX2 / 3 PNGs SLG)"
    ], "#38BDF8", "#152333", "#38BDF8"),
    node_box(710, 104, 315, 165, "STEP 3: Resolve 5-State Module Matrix", [
        "• Đối chiếu 6 module hệ thống với cấu hình",
        "  `packaged_modules` của Game Project",
        "• Xác thực đồ thị phụ thuộc module (VD:",
        "  `mod.vision_ocr` yêu cầu `mod.adb_core`)",
        "• Khóa danh sách quyền (`requiredCapabilities`)",
        "code:Status: PASS (5 modules JX2 / 4 SLG)"
    ], "#B291F7", "#1B1E32", "#B291F7"),
    node_box(1048, 104, 316, 165, "STEP 4: Copy Dedicated Runtime Binary", [
        "• Sao chép `target/release/fp-mini-runtime.exe`",
        "  sang `releases/<slug>/Auto_<Game>.exe`",
        "• Binary này hoàn toàn KHÔNG chứa mã nguồn",
        "  của Studio hay `fp-builder`",
        "• Kích thước: 12,515,840 bytes (~12.5 MB)",
        "code:Status: PASS (30 ms)"
    ], "#B291F7", "#1B1E32", "#B291F7"),

    node_box(34, 312, 315, 172, "STEP 5: PE Icon & RT_RCDATA #7777 Injection", [
        "• Gọi Win32 API `BeginUpdateResourceW` /",
        "  `UpdateResourceW` / `EndUpdateResourceW`",
        "• Nhúng 7 lớp biểu tượng `.ico` vào `RT_ICON`",
        "• Nhúng cứng 32-byte Ed25519 Public Key vào",
        "  tài nguyên PE `RT_RCDATA (#10) ID #7777`",
        "• Mở lại PE kiểm tra readback khớp 100%",
        "code:Status: PASS (1,018 ms)"
    ], "#E7B15B", "#23211A", "#E7B15B"),
    node_box(372, 312, 315, 172, "STEP 6: Package Bundle Assets & ADB", [
        "• Xuất `bundle/manifest.fpbundle.json`,",
        "  `ui-schema.json`, `workflows.fpbundle.json`",
        "• Sao chép `bundle/templates/*.png` & `.ico`",
        "• Đóng gói kèm `./bin/adb.exe`,",
        "  `AdbWinApi.dll`, `AdbWinUsbApi.dll`",
        "• Khởi tạo thư mục `./data/` cho SQLite WAL",
        "code:Status: PASS (24 ms)"
    ], "#E7B15B", "#23211A", "#E7B15B"),
    node_box(710, 312, 315, 172, "STEP 7: SHA-256 Table & Ed25519 Signing", [
        "• Băm SHA-256 từng file trong `bundle/`",
        "• Tạo chuỗi Canonical Manifest Payload",
        "• Đọc khóa bí mật từ kho khóa ngoài",
        "  `%LOCALAPPDATA%\\FarmerPro\\ProducerKeys`",
        "• Ký chữ ký số RFC 8032 Ed25519 (64 bytes)",
        "• Xuất `bundle/signature.fpbundle.json`",
        "code:KeyID: fp-ed25519-acb43ebd9ab4 (9 ms)"
    ], "#3BC38D", "#142624", "#3BC38D"),
    node_box(1048, 312, 316, 172, "STEP 8: PE Root-of-Trust Verify & Audit", [
        "• Đọc lại khóa công khai từ PE `RT_RCDATA #7777`",
        "  của file `Auto_<Game>.exe` vừa xuất",
        "• Xác thực chữ ký Ed25519 & toàn bộ bảng SHA-256",
        "• Kiểm toán phụ thuộc: ADB DLLs, WebView2",
        "  Evergreen, WinRT OCR, quyền ghi SQLite",
        "• Xuất `build-report.json` kèm SHA-256 EXE",
        "code:Status: PASS (Verify: 13ms | PoC Total: 1097–1127ms)"
    ], "#3BC38D", "#142624", "#3BC38D"),

    arrow(349, 186, 372, 186, "#38BDF8", "arrow-blue", ""),
    arrow(687, 186, 710, 186, "#B291F7", "arrow-purple", ""),
    arrow(1025, 186, 1048, 186, "#B291F7", "arrow-purple", ""),
    arrow(1206, 269, 190, 312, "#E7B15B", "arrow-amber", "Step 4 -> Step 5"),
    arrow(349, 398, 372, 398, "#E7B15B", "arrow-amber", ""),
    arrow(687, 398, 710, 398, "#3BC38D", "arrow-green", ""),
    arrow(1025, 398, 1048, 398, "#3BC38D", "arrow-green", ""),
])

# -------------------------------------------------------------------------
# 11. IV.9: SECURITY ARCHITECTURE — ED25519 ROOT OF TRUST & DEFENSE-IN-DEPTH
# -------------------------------------------------------------------------
iv09_body = "\n".join([
    zone(18, 72, 436, 430, "1. PRODUCER SIGNING ENVIRONMENT (Máy Nhà Sản Xuất)", "#E7B15B", "#171A22", "#E7B15B"),
    node_box(34, 102, 404, 118, "External Producer Keystore (%LOCALAPPDATA%)", [
        "• Đường dẫn: `%LOCALAPPDATA%\\FarmerPro\\ProducerKeys\\`",
        "  `active_producer_ed25519.json`",
        "• Chứa `SigningKey` (32-byte Secret Seed) & `VerifyingKey`",
        "• Khóa bí mật KHÔNG BAO GIỜ nằm trong mã nguồn, git hay bundle/",
        "code:Key ID: fp-ed25519-acb43ebd9ab4"
    ], "#F27883", "#251920", "#F27883"),
    node_box(34, 236, 404, 122, "PE Resource Root-of-Trust Injector (Step 5)", [
        "• Nhúng khóa công khai `acb43ebd9ab499f0...` (64 hex bytes) vào",
        "  phân vùng `.rsrc` của `Auto_<Game>.exe` tại:",
        "  `RT_RCDATA (lpType = #10), Resource ID = #7777`",
        "• Ký bảng SHA-256 của `bundle/` bằng khóa bí mật ở Step 7",
        "code:crates/fp-builder/src/pe_icon.rs"
    ], "#E7B15B", "#23211A", "#E7B15B"),
    node_box(34, 374, 404, 112, "P5 Release Hardening: Windows Authenticode (signtool.exe)", [
        "• Ký số toàn bộ file `Auto_<Game>.exe` bằng chứng thư EV/OV",
        "• Bảo vệ cả mã máy lẫn vùng tài nguyên `.rsrc (RT_RCDATA #7777)`",
        "  trước hành vi dùng công cụ hex/PE editor sửa trực tiếp file `.exe`",
        "code:Hoàn thiện chuỗi tin cậy OS -> Binary -> PE Key -> Bundle"
    ], "#38BDF8", "#152333", "#38BDF8"),

    zone(486, 72, 436, 430, "2. RUNTIME VERIFICATION GATE (Khi Mở Auto_<Game>.exe)", "#3BC38D", "#101E22", "#3BC38D"),
    node_box(502, 102, 404, 118, "Gate 1: Read Trusted Public Key from Self PE Memory", [
        "• Lấy đường dẫn tiến trình đang chạy: `std::env::current_exe()`",
        "• Gọi `LoadLibraryExW(..., LOAD_LIBRARY_AS_DATAFILE)`",
        "• Gọi `FindResourceW(hMod, #7777, RT_RCDATA)` -> Đọc 64 byte hex",
        "• Đây là nguồn tin cậy gốc (Root of Trust) để đối chiếu!",
        "code:read_pe_embedded_pubkey_from_current_process()"
    ], "#3BC38D", "#142624", "#3BC38D"),
    node_box(502, 236, 404, 122, "Gate 2: Cross-Check Disk Public Key & Envelope Header", [
        "• Đọc `bundle/trusted_publisher.pub` và trường",
        "  `publisherPublicKeyHex` trong `bundle/signature.fpbundle.json`",
        "• Nếu bất kỳ giá trị nào khác với khóa trong PE `RT_RCDATA #7777`:",
        "  -> Chặn đứng ngay với lỗi `PE_ROOT_OF_TRUST_MISMATCH`!",
        "code:Chặn hoàn toàn tấn công thay khóa công khai trên đĩa"
    ], "#3BC38D", "#142624", "#3BC38D"),
    node_box(502, 374, 404, 112, "Gate 3: RFC 8032 Ed25519 Verify & Per-File SHA-256 Check", [
        "• Kiểm tra phương trình chữ ký đường cong Elliptic Curve25519",
        "• Băm lại SHA-256 của toàn bộ file trong `bundle/` (manifest,",
        "  ui-schema, workflows, templates/*.png) đối chiếu bảng đã ký",
        "code:Chỉ khởi tạo Tauri UI & Engine khi cả 3 Gate đều PASS"
    ], "#3BC38D", "#142624", "#3BC38D"),

    zone(954, 72, 428, 430, "3. 5 TAMPER ATTACK CASES & BOUNDARY DISCLOSURE", "#F27883", "#1A141B", "#F27883"),
    node_box(970, 102, 396, 232, "5 Kịch bản Tấn công Giả mạo Đã Bị Chặn (5/5 REJECTED)", [
        "✅ Case 1: Sửa `ui-schema.json` + tính lại SHA-256 trên đĩa",
        "   -> Bị chặn: `ED25519_SIGNATURE_INVALID`",
        "✅ Case 2: Ghi đè `bundle/trusted_publisher.pub` bằng khóa lạ",
        "   -> Bị chặn: `PE_ROOT_OF_TRUST_MISMATCH (RT_RCDATA #7777)`",
        "✅ Case 3: Đảo bit chữ ký trong `signature.fpbundle.json`",
        "   -> Bị chặn: `ED25519_SIGNATURE_INVALID`",
        "✅ Case 4: Xóa tệp bắt buộc `manifest.fpbundle.json`",
        "   -> Bị chặn: `INTEGRITY_MISSING_MANIFEST`",
        "✅ Case 5: Sinh cặp khóa Ed25519 giả mạo mới, thay cả",
        "   `trusted_publisher.pub` lẫn ký lại `signature.fpbundle.json`",
        "   -> Bị chặn: `ED25519_PUBKEY_UNTRUSTED` (Khác khóa PE #7777)!"
    ], "#3BC38D", "#152522", "#3BC38D"),
    node_box(970, 350, 396, 136, "Phân tích Trung thực Giới hạn Bảo mật (Security Disclosure)", [
        "• Nhúng Public Key vào PE `RT_RCDATA #7777` chống triệt để việc",
        "  kẻ tấn công chỉ sửa/thay khóa và bundle trên đĩa.",
        "• Cơ chế này KHÔNG tự bảo vệ nếu kẻ tấn công thay thế cả file",
        "  `.exe` hoặc dùng PE Editor vá lại tài nguyên `#7777` trong `.exe`.",
        "• Do đó ở P5 cần kết hợp chữ ký Windows Authenticode lên file",
        "  `.exe` để hệ điều hành xác thực tính toàn vẹn nhị phân & nhà phát hành."
    ], "#E7B15B", "#241F19", "#E7B15B"),

    arrow(438, 290, 502, 160, "#E7B15B", "arrow-amber", "PE#7777"),
    arrow(704, 220, 704, 236, "#3BC38D", "arrow-green", ""),
    arrow(704, 358, 704, 374, "#3BC38D", "arrow-green", ""),
    arrow(906, 290, 970, 220, "#3BC38D", "arrow-green", "5/5 Block"),
])

# -------------------------------------------------------------------------
# 12. IV.10: STUDIO -> MULTI-GAME MINI TOOL PRODUCTION LIFECYCLE
# -------------------------------------------------------------------------
iv10_body = "\n".join([
    zone(18, 72, 376, 430, "1. UNCHANGED SHARED RUST CORE (14 CRATES)", "#B291F7", "#131826", "#B291F7"),
    node_box(34, 102, 344, 382, "Zero Core Code Modification Across Games", [
        "• fp-domain (Core Types & Driver Traits)",
        "• fp-adb (Argv Command & Discovery)",
        "• fp-device (DeviceActor & Fencing)",
        "• fp-vision (Multiscale SSD & WinRT OCR)",
        "• fp-actions (Safe Random Tap Policy)",
        "• fp-workflow-schema (JSON Graph Validator)",
        "• fp-workflow-runtime (State Machine Executor)",
        "• fp-scheduler (Leases & Priority Queue)",
        "• fp-storage (SQLite WAL Persistence)",
        "• fp-security (Vault & Secret Redaction)",
        "• fp-bundle-contract (5-State Modules & UI Schema)",
        "• fp-bundle-verify (PE #7777 Ed25519 Verifier)",
        "-----------------------------------------",
        "Biên dịch 1 lần duy nhất thành:",
        "code:target/release/fp-mini-runtime.exe",
        "Tái sử dụng cho N tựa game khác nhau!"
    ], "#B291F7", "#181E30", "#B291F7"),

    zone(426, 72, 466, 430, "2. FARMERPRO STUDIO MULTI-GAME PROJECTS", "#38BDF8", "#111927", "#38BDF8"),
    node_box(442, 102, 434, 118, "Project A: Võ Hiệp Truyền Kỳ 2 Mobile (jx2-mobile)", [
        "• Package: `com.blackhole.jx2mobile` | Icon: `jx2/branding/icon.ico`",
        "• Workflows: `wf-jx2-main-quest`, `wf-jx2-live-adb-probe` (4 PNGs)",
        "• UI Schema: Amber `#F59E0B`, 3 cột OCR (`Cấp`, `Vàng`, `Bạc`), 5 Tasks",
        "• Modules (5): ADB, Vision/OCR, Account Vault, SOCKS5 Proxy, Watchdog"
    ], "#E7B15B", "#22201A", "#E7B15B"),
    node_box(442, 234, 434, 118, "Project B: Kingdom Rise: Tactics SLG (kingdom-slg)", [
        "• Package: `com.kingdom.rise.slg` | Icon: `slg/branding/icon.ico`",
        "• Workflows: `wf-slg-harvest-and-patrol` (3 PNGs)",
        "• UI Schema: Emerald `#10B981`, 4 cột OCR (`Thành`, `Lương`, `Gỗ`, `Quân`), 4 Tasks",
        "• Modules (4): ADB, Vision/OCR, Account Vault, Watchdog (Proxy OFF)"
    ], "#3BC38D", "#142624", "#3BC38D"),
    node_box(442, 366, 434, 118, "Project N: Bất kỳ Tựa Game Android Mới (new-game-slug)", [
        "• Nhà phát triển chỉ cần tạo Project trên Studio UI:",
        "  1. Cắt ảnh mẫu PNG từ Device Lab  2. Kéo thả Node trong Workflow Studio",
        "  3. Cấu hình Task & Cột OCR        4. Bấm `Build & Xuất Mini Tool (.EXE)`",
        "code:Không cần viết Rust, không sửa JSON tay, không chạy Cargo!"
    ], "#38BDF8", "#152333", "#38BDF8"),

    zone(924, 72, 458, 430, "3. INDEPENDENT COMMERCIAL MINI TOOLS", "#3BC38D", "#101E22", "#3BC38D"),
    node_box(940, 102, 426, 118, "Release #1: Auto_JX2_Mobile.exe (Độc lập 100%)", [
        "• PE Icon riêng + PE RT_RCDATA #7777 PubKey + Signed Bundle #1",
        "• Database riêng: `data/runtime_jx2-mobile.db`",
        "• Đã kiểm chứng chạy độc lập khi FarmerPro Studio đóng hoàn toàn",
        "code:SHA256: 55c051cba31c269c7d307301ac2f32e451f18773..."
    ], "#E7B15B", "#22201A", "#E7B15B"),
    node_box(940, 234, 426, 118, "Release #2: Auto_Kingdom_SLG.exe (Độc lập 100%)", [
        "• PE Icon riêng + PE RT_RCDATA #7777 PubKey + Signed Bundle #2",
        "• Database riêng: `data/runtime_kingdom-slg.db`",
        "• Đã kiểm chứng chạy độc lập & NTFS Read-Only Fallback PASS",
        "code:SHA256: 95732232077c0747fe611def3afba858327df6f4..."
    ], "#3BC38D", "#142624", "#3BC38D"),
    node_box(940, 366, 426, 118, "Release #N: Auto_<NewGame>.exe (Sẵn sàng mở rộng)", [
        "• Đóng gói bằng `MiniToolPackager` & đo thời gian từng bước",
        "• Mỗi sản phẩm có nhận diện thương hiệu, bộ nhiệm vụ, cột OCR",
        "  và phạm vi module hoàn toàn riêng biệt",
        "code:Kiến trúc Nhà máy Phần mềm (Software Factory) Đích thực"
    ], "#38BDF8", "#152333", "#38BDF8"),

    arrow(378, 160, 442, 160, "#B291F7", "arrow-purple", "Shared"),
    arrow(378, 290, 442, 290, "#B291F7", "arrow-purple", "Shared"),
    arrow(378, 420, 442, 420, "#B291F7", "arrow-purple", "Shared"),
    arrow(876, 160, 940, 160, "#E7B15B", "arrow-amber", "Build #1"),
    arrow(876, 290, 940, 290, "#3BC38D", "arrow-green", "Build #2"),
    arrow(876, 420, 940, 420, "#38BDF8", "arrow-blue", "Build #N"),
])

# -------------------------------------------------------------------------
# 13. PART 6: 9-STEP MULTI-GAME MINI TOOL PRODUCTION WORKFLOW DIAGRAM
# -------------------------------------------------------------------------
part6_body = "\n".join([
    zone(18, 72, 1364, 430, "QUY TRÌNH 9 BƯỚC SẢN XUẤT MINI TOOL ĐA DỰ ÁN GAME TRÊN FARMERPRO STUDIO", "#38BDF8", "#101824", "#38BDF8"),
    node_box(34, 102, 285, 115, "BƯỚC 1: Tạo Game Project", [
        "• Khu vực: `1. Game Projects`",
        "• Project A: `jx2-mobile` (com.blackhole.jx2)",
        "• Project B: `kingdom-slg` (com.kingdom.slg)",
        "• Chọn độ phân giải chuẩn & `icon.ico`",
        "code:Khởi tạo projects/<slug>/"
    ], "#1878F5", "#162438", "#38BDF8"),
    node_box(344, 102, 285, 115, "BƯỚC 2: Kết nối Thiết bị Android", [
        "• Khu vực: `4. Device Lab`",
        "• Kết nối LDPlayer `emulator-5554` (SM_X710N)",
        "• Chụp ảnh màn hình PNG theo yêu cầu",
        "• Kéo khung ROI cắt ảnh mẫu lưu vào Project",
        "code:4 PNGs (JX2) | 3 PNGs (SLG)"
    ], "#38BDF8", "#152333", "#38BDF8"),
    node_box(654, 102, 285, 115, "BƯỚC 3: Thiết kế Workflow", [
        "• Khu vực: `2. Workflow Studio`",
        "• Kéo thả Node: `start -> capture ->",
        "  find_image -> random_safe_tap -> ocr -> cp`",
        "• Đặt Confidence `0.90` & Safe Margin `18%`",
        "code:Kiểm tra đồ thị WorkflowValidator"
    ], "#B291F7", "#1B1E32", "#B291F7"),
    node_box(964, 102, 398, 115, "BƯỚC 4: Tạo Task & Cấu hình Modules", [
        "• Khu vực: `3. Tasks & Modules`",
        "• Ánh xạ Workflow thành Nhiệm vụ người dùng:",
        "  - JX2: 5 Tasks (`[NV]`, `[PB]`, `[HĐ]`)",
        "  - SLG: 4 Tasks (`[Nông Nghiệp]`, `[Quân Sự]`, `[Liên Minh]`)",
        "• Bật/tắt Module: Vault, SOCKS5 Proxy, Watchdog"
    ], "#3BC38D", "#142624", "#3BC38D"),

    node_box(34, 250, 285, 115, "BƯỚC 5: Chạy Test & Debug", [
        "• Khu vực: `5. Test & Debug`",
        "• Chạy luồng thật trên `emulator-5554`",
        "• Đối chiếu 19 StepEvents, ảnh Trước/Sau,",
        "  tọa độ Safe Tap `(111, 170)` & kết quả OCR",
        "code:Xác nhận trạng thái VERIFIED"
    ], "#E7B15B", "#22211C", "#E7B15B"),
    node_box(344, 250, 285, 115, "BƯỚC 6: Thiết kế UI Mini Tool", [
        "• Khu vực: `6. Mini Tool Builder`",
        "• Chọn Layout `compact_2_col` & Màu thương hiệu",
        "  (`#F59E0B` cho JX2 / `#10B981` cho SLG)",
        "• Định nghĩa cột chỉ số OCR động",
        "• Kiểm tra trên khung Live Preview"
    ], "#E7B15B", "#22211C", "#E7B15B"),
    node_box(654, 250, 285, 115, "BƯỚC 7: Build EXE (8 Bước)", [
        "• Bấm `Build & Xuất Mini Tool (.EXE)`",
        "• Chạy Pre-Flight Audit 6 điểm",
        "• Copy `fp-mini-runtime.exe`, nhúng PE Icon",
        "  + khóa công khai vào `PE RT_RCDATA #7777`",
        "• Ký số RFC 8032 Ed25519 (đo telemetry từng bước)"
    ], "#F27883", "#251920", "#F27883"),
    node_box(964, 250, 398, 115, "BƯỚC 8: Kiểm tra & Xuất Release", [
        "• Khu vực: `6. Release Management`",
        "• Chạy Smoke Test: `--verify-bundle` & `--test-tamper-suite`",
        "• Kiểm tra SHA-256 binary, chữ ký Ed25519 & ADB DLLs",
        "• Xuất thư mục `releases/poc/Auto_JX2_Mobile/` và",
        "  `releases/poc/Auto_Kingdom_SLG/`"
    ], "#3BC38D", "#142624", "#3BC38D"),

    node_box(34, 392, 1328, 94, "BƯỚC 9: ĐÓNG HOÀN TOÀN FARMERPRO STUDIO & CHẠY ĐỘC LẬP 2 MINI TOOLS (BẰNG CHỨNG ĐA GAME)", [
        "• Sau khi đóng `FarmerPro.exe`, người chơi khởi chạy trực tiếp `Auto_JX2_Mobile.exe` và `Auto_Kingdom_SLG.exe` từ thư mục phát hành.",
        "• Mỗi Mini Tool tự đọc khóa công khai từ `PE RT_RCDATA #7777` của chính nó, xác thực `bundle/signature.fpbundle.json` & kết nối `./bin/adb.exe`.",
        "code:Lưu dữ liệu vào SQLite WAL riêng biệt (`data/runtime_jx2-mobile.db` & `data/runtime_kingdom-slg.db`) — 100% không phụ thuộc Studio!"
    ], "#3BC38D", "#132824", "#3BC38D"),

    arrow(319, 158, 344, 158, "#38BDF8", "arrow-blue", ""),
    arrow(629, 158, 654, 158, "#B291F7", "arrow-purple", ""),
    arrow(939, 158, 964, 158, "#3BC38D", "arrow-green", ""),
    arrow(1160, 217, 180, 250, "#E7B15B", "arrow-amber", "Bước 4 -> Bước 5"),
    arrow(319, 308, 344, 308, "#E7B15B", "arrow-amber", ""),
    arrow(629, 308, 654, 308, "#F27883", "arrow-red", ""),
    arrow(939, 308, 964, 308, "#3BC38D", "arrow-green", ""),
    arrow(1160, 365, 700, 392, "#3BC38D", "arrow-green", "Bước 8 -> Bước 9"),
])

# -------------------------------------------------------------------------
# 14. PART 8: P2 -> P6 IMPLEMENTATION ROADMAP TIMELINE
# -------------------------------------------------------------------------
part8_body = "\n".join([
    zone(18, 72, 1364, 430, "LỘ TRÌNH TRIỂN KHAI KỸ THUẬT P2 → P6 (CHỈ MỞ KHÓA SAU KHI NGHIỆM THU BÁO CÁO P1.2)", "#38BDF8", "#101824", "#38BDF8"),
    node_box(34, 106, 194, 376, "GATE HIỆN TẠI: P1.2", [
        "Trạng thái: ĐANG TRÌNH DUYỆT",
        "-------------------------",
        "✅ 34/34 Unit Tests PASS",
        "✅ PE RT_RCDATA #7777",
        "   Root of Trust Verified",
        "✅ 5/5 Tamper Cases Blocked",
        "✅ Live ADB LDPlayer Proof",
        "✅ Blueprint & PDF Report",
        "-------------------------",
        "🔒 CHỐT CHẶN (GATE):",
        "Không sửa UI/DB trước khi",
        "Người Phê Duyệt xác nhận",
        "7 Quyết định Kiến trúc!"
    ], "#E7B15B", "#232018", "#E7B15B"),

    node_box(274, 106, 202, 376, "GIAI ĐOẠN P2", [
        "GameProject, Storage,",
        "Migration v2 & Security",
        "-------------------------",
        "• Phạm vi: fp-storage,",
        "  fp-security, fp-domain",
        "• Online Backup SQLite +",
        "  PRAGMA integrity_check",
        "• Migration v1 -> v2",
        "  (6 bảng mới + 3 FK)",
        "• Migrate 10 quy trình",
        "  JX2 vào project mặc định",
        "• ProducerKeyManager IPC",
        "-------------------------",
        "Test: Migration + Rollback",
        "& Backup Restore Tests"
    ], "#38BDF8", "#152334", "#38BDF8"),

    node_box(495, 106, 202, 376, "GIAI ĐOẠN P3", [
        "Workflow Engine, Device",
        "Lab & Scheduler IPC",
        "-------------------------",
        "• Phạm vi: fp-adb, fp-vision",
        "  fp-workflow-runtime,",
        "  src-tauri IPC commands",
        "• ROI Template Cropper",
        "  lưu thẳng vào Project",
        "• Live SSD Match Sandbox",
        "  & WinRT OCR Preview",
        "• Xóa bỏ hoàn toàn dữ liệu",
        "  giả lập job-demo-8492",
        "-------------------------",
        "Test: Live ADB Crop, SSD",
        "Match & Fencing Tests"
    ], "#B291F7", "#1B1E32", "#B291F7"),

    node_box(716, 106, 202, 376, "GIAI ĐOẠN P4", [
        "Studio UX Redesign (6",
        "Khu Vực) & Mini Builder",
        "-------------------------",
        "• Phạm vi: apps/desktop/src",
        "• Global Context Header",
        "  (Project + Lab Device)",
        "• Refactor 10 tab cũ sang",
        "  6 Khu vực + Settings",
        "• Tích hợp Mini Tool",
        "  Builder & Live Preview",
        "• Đáp ứng 3 độ phân giải",
        "  1280x720 / 1600 / 1920",
        "-------------------------",
        "Test: E2E UI & Zero-Clip",
        "3-Resolution Verification"
    ], "#1878F5", "#162438", "#1878F5"),

    node_box(937, 106, 202, 376, "GIAI ĐOẠN P5", [
        "Build/Release Pipeline,",
        "Logo, Packaging & Signing",
        "-------------------------",
        "• Phạm vi: fp-builder,",
        "  Release Management UI",
        "• Pre-Flight Audit 6 điểm",
        "• Xuất EXE + PE Icon +",
        "  PE RT_RCDATA #7777",
        "• Tích hợp tùy chọn ký",
        "  Windows Authenticode",
        "• Smoke Test Runner ngay",
        "  trong giao diện Studio",
        "-------------------------",
        "Test: Build Report, ZIP",
        "& 5-Case Tamper Suite"
    ], "#3BC38D", "#142624", "#3BC38D"),

    node_box(1158, 106, 206, 376, "GIAI ĐOẠN P6", [
        "End-to-End Multi-Game",
        "Acceptance Verification",
        "-------------------------",
        "• Phạm vi: Toàn hệ thống",
        "  Studio + 2 Mini Tools",
        "• Kiểm thử quy trình 9",
        "  bước không cần code",
        "• Xuất 2 Mini Tools cho",
        "  2 game từ UI Studio",
        "• Đóng Studio, chạy độc",
        "  lập cả 2 Mini Tools",
        "  trên thiết bị ADB thật",
        "-------------------------",
        "Test: Full Acceptance",
        "Gate & Final Sign-off"
    ], "#3BC38D", "#132924", "#3BC38D"),

    arrow(228, 290, 274, 290, "#E7B15B", "arrow-amber", "Duyệt"),
    arrow(476, 290, 495, 290, "#38BDF8", "arrow-blue", ""),
    arrow(697, 290, 716, 290, "#B291F7", "arrow-purple", ""),
    arrow(918, 290, 937, 290, "#1878F5", "arrow-blue", ""),
    arrow(1139, 290, 1158, 290, "#3BC38D", "arrow-green", ""),
])

SVG_SPECS = [
    ("00_part1_overall_product_architecture.svg", "SƠ ĐỒ 1.1 — KIẾN TRÚC TỔNG QUAN FARMERPRO (NỀN TẢNG SẢN XUẤT ĐA AUTO TOOLS)", "Cấu trúc tổng thể hệ sinh thái FarmerPro: Studio IDE, Shared Rust Core Crates và các gói Mini Auto-Tools độc lập", part1_body),
    ("00_part1b_studio_engine_builder_runtime_relationship.svg", "SƠ ĐỒ 1.2 — SƠ ĐỒ QUAN HỆ GIỮA STUDIO, WORKFLOW ENGINE, BUILDER VÀ MINI RUNTIME", "Luồng chuyển giao đặc tả, liên kết biên dịch, cô lập khóa bí mật và thực thi độc lập giữa 4 phân hệ cốt lõi", part1b_body),
    ("00_part2_before_vs_proposed_architecture.svg", "SƠ ĐỒ 2.1 — SO SÁNH KIẾN TRÚC CŨ (10 TAB PHẲNG) VÀ KIẾN TRÚC ĐỀ XUẤT P1.2 (6 KHU VỰC + HỆ THỐNG)", "Ánh xạ 10 màn hình hiện tại sang Studio IDE đa dự án — Phân tách Thiết kế UI vs Trạng thái Backend", part2_body),
    ("IV_01_cargo_workspace_dependency_graph.svg", "SƠ ĐỒ IV.1 — RUST CARGO WORKSPACE DEPENDENCY GRAPH (14 CRATES & 3 APPS)", "Ranh giới bảo mật thời điểm biên dịch: fp-mini-runtime hoàn toàn không phụ thuộc vào fp-builder", iv01_body),
    ("IV_02_farmerpro_studio_architecture.svg", "SƠ ĐỒ IV.2 — FARMERPRO STUDIO IDE INTERNAL ARCHITECTURE", "Luồng dữ liệu hai chiều giữa React 19 + React Flow + Zustand qua Tauri v2 Typed IPC xuống Rust Core Services", iv02_body),
    ("IV_03_gameproject_data_model.svg", "SƠ ĐỒ IV.3 — GAMEPROJECT DATA MODEL & SQLITE MIGRATION V1 → V2 ER DIAGRAM", "Mô hình dữ liệu đa dự án trong SQLite WAL: 6 bảng mới + mở rộng khóa ngoại project_id bảo toàn dữ liệu cũ", iv03_body),
    ("IV_04_workflow_runtime_execution_architecture.svg", "SƠ ĐỒ IV.4 — WORKFLOW RUNTIME EXECUTION ARCHITECTURE", "Máy trạng thái thực thi quy trình tự động hóa: Chụp PNG -> Khớp mẫu SSD -> Safe Random Tap 18% -> WinRT OCR -> Checkpoint", iv04_body),
    ("IV_05_adb_emulator_adapter_architecture.svg", "SƠ ĐỒ IV.5 — ADB & EMULATOR ADAPTER ARCHITECTURE", "Kiến trúc điều khiển thiết bị Android không nội suy chuỗi shell, tích hợp Generation Fencing và chuẩn hóa tọa độ xoay", iv05_body),
    ("IV_06_task_scheduler_multidevice_execution.svg", "SƠ ĐỒ IV.6 — TASK SCHEDULER & MULTI-DEVICE EXECUTION ARCHITECTURE", "Hàng đợi ưu tiên, cấp phát Lease độc quyền từng thiết bị và cơ chế Generation Token Fencing chống xung đột", iv06_body),
    ("IV_07_mini_tool_ui_schema_renderer.svg", "SƠ ĐỒ IV.7 — MINI TOOL UI SCHEMA RENDERER ARCHITECTURE", "Cơ chế dựng giao diện Mini Tool động 100% từ bundle/ui-schema.json và kiểm soát Module 5 trạng thái", iv07_body),
    ("IV_08_mini_tool_build_release_pipeline.svg", "SƠ ĐỒ IV.8 — MINI TOOL BUILD & RELEASE PIPELINE (8-STEP PACKAGER)", "Dây chuyền đóng gói 8 bước của fp-builder: Kiểm tra đồ thị -> Nhúng PE Icon & RT_RCDATA #7777 -> Ký Ed25519 -> Kiểm toán", iv08_body),
    ("IV_09_security_architecture_ed25519_root_of_trust.svg", "SƠ ĐỒ IV.9 — SECURITY ARCHITECTURE: ED25519 PE RT_RCDATA #7777 ROOT OF TRUST", "Kiến trúc xác thực chữ ký số RFC 8032 Ed25519 với khóa công khai nhúng cứng trong PE Resource #7777 & vai trò của Authenticode", iv09_body),
    ("IV_10_studio_to_mini_tool_production_lifecycle.svg", "SƠ ĐỒ IV.10 — STUDIO → MULTI-GAME MINI TOOL PRODUCTION LIFECYCLE", "Một nền tảng FarmerPro Studio + 14 Rust Core Crates dùng chung sản xuất nhiều Mini Auto-Tools cho các game khác nhau", iv10_body),
    ("VI_01_9step_production_workflow.svg", "SƠ ĐỒ VI.1 — QUY TRÌNH 9 BƯỚC SẢN XUẤT MINI TOOL ĐA DỰ ÁN TRÊN FARMERPRO STUDIO", "Hành trình khép kín từ tạo Game Project, cắt mẫu ADB, vẽ Workflow, đóng gói EXE đến chạy độc lập không cần Studio", part6_body),
    ("VIII_01_p2_to_p6_roadmap_timeline.svg", "SƠ ĐỒ VIII.1 — LỘ TRÌNH TRIỂN KHAI KỸ THUẬT P2 → P6 SAU KHI PHÊ DUYỆT P1.2", "Chuỗi giai đoạn phát triển có kiểm soát cổng nghiệm thu (Stage-Gate) từ P2 (Storage/Schema) đến P6 (E2E Acceptance)", part8_body),
]

for fname, title, sub, body in SVG_SPECS:
    svg_text = svg_wrap(1400, 520, title, sub, body)
    with open(os.path.join(DIAGRAM_DIR, fname), "w", encoding="utf-8") as f:
        f.write(svg_text)

print(f"Generated {len(SVG_SPECS)} standalone editable SVG diagrams in {DIAGRAM_DIR}")
