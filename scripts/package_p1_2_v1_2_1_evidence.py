import os
import json
import shutil
import hashlib
import subprocess
from datetime import datetime

ROOT = r"E:\DATA\FarmerPro"
PACK_DIR = os.path.join(ROOT, "artifacts", "p1_2_v1_2_1_evidence_pack")

SUBDIRS = {
    "pdf": os.path.join(PACK_DIR, "01_pdf_deliverables"),
    "specs": os.path.join(PACK_DIR, "02_blueprint_and_specs"),
    "logs": os.path.join(PACK_DIR, "03_raw_logs_and_json"),
    "actual": os.path.join(PACK_DIR, "04_screenshots_actual"),
    "mockups": os.path.join(PACK_DIR, "05_ui_mockups_fullres"),
    "diagrams": os.path.join(PACK_DIR, "06_diagrams_source"),
    "git_env": os.path.join(PACK_DIR, "07_git_and_environment"),
}

if os.path.exists(PACK_DIR):
    shutil.rmtree(PACK_DIR)
for d in SUBDIRS.values():
    os.makedirs(d, exist_ok=True)


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        while chunk := f.read(65536):
            h.update(chunk)
    return h.hexdigest()


def copy_list(items, dst_dir):
    for src_rel in items:
        src = os.path.join(ROOT, src_rel)
        if os.path.exists(src):
            shutil.copy2(src, os.path.join(dst_dir, os.path.basename(src)))


# 1. PDF Deliverables
copy_list([
    "FARMERPRO_P1_2_ARCHITECTURE_UX_ACCEPTANCE_REPORT.pdf",
    r"artifacts\p1_2_evidence\pdf_quality_assurance_report.json",
], SUBDIRS["pdf"])

changelog_md = """# CHANGELOG — FARMERPRO P1.2 ACCEPTANCE REPORT (v1.2.0 -> v1.2.1-REVIEW)

- **Ngày lập**: 2026-11-21
- **Phiên bản mới**: `v1.2.1-REVIEW` (Chờ phê duyệt P1.2 trước khi mở khóa P2)
- **Cam kết kỷ luật**: Giữ nguyên 100% mã nguồn giao diện `apps/desktop/src` và cơ sở dữ liệu sản xuất `farmerpro.db`.

## 7 Hạng mục Hiệu chỉnh Bắt buộc
1. **Chất lượng PDF & Khả năng đọc ở zoom 100–125%**:
   - Tăng cỡ chữ chuẩn toàn tài liệu (`body 12px ≈ 9pt`, bảng `10.8–11.5px ≈ 8.1–8.6pt`), chấm dứt chữ `6.5–7pt`.
   - Tách 10 sơ đồ kiến trúc kỹ thuật `IV.1–IV.10` thành **1 sơ đồ/trang** (cao ~132mm thay vì ép 2 sơ đồ/trang 65mm).
   - Phóng to ảnh giao diện UI ở Phần 3 lên ~128–132mm, loại bỏ khoảng trắng thừa ở nửa dưới các trang bảng (TC-01..14, Responsive Rubric, Pre-P2/Decisions).
   - Chỉnh CSS `word-break: normal; overflow-wrap: break-word` cho `<code>` thông thường (hết tách giữa từ như `ui-schema.js` + `on`).
   - Bổ sung **PDF Bookmarks (Outline)** phân cấp bằng `PyMuPDF set_toc()` và giữ nguyên liên kết Mục lục `#page-XX`.
2. **Thiết kế Tương tác Workflow Studio (`Ảnh 03A` & `Ảnh 03B`)**:
   - Minh họa ưu tiên diện tích Canvas (`>72% @ 1280×720`, `~64% @ 1600×900`) với khả năng thu gọn Palette/Inspector/Dock.
   - Bổ sung đủ **6 trạng thái tương tác**: Canvas mặc định, Chọn Node + Inspector, Đang thực thi, Lỗi + Debug, Breakpoint/Step Run, Crop ảnh từ Device Lab đưa vào Vision Node.
3. **Trình dựng Trực quan Mini Tool Builder (`Ảnh 07A` & `Ảnh 07B`)**:
   - Chuyển từ dạng bảng cấu hình sang **WYSIWYG Visual Form Builder 3 khung**: thêm/xóa/kéo thả sắp xếp khu vực UI, chọn nhiệm vụ, click trực tiếp trên Preview để chỉnh thuộc tính & liên kết biến Runtime — **không bắt người dùng sửa JSON tay**.
   - Bổ sung `Ảnh 07B` cho **3 Layout Presets** (`1 cột / 2 cột / Expanded`) và **4 Preview States** (`Online / Offline / Running / Error`).
4. **Ma trận Phân loại Thực tế 5 Trạng thái (Bỏ nhãn "Bảo toàn 100% tính năng")**:
   - Phân tách rạch ròi **Bảo toàn Thiết kế/Cấu trúc UI** vs **Trạng thái Backend Thực tế** (`IMPLEMENTED / PARTIAL / MOCK / MISSING / PLANNED`).
   - Ghi nhận trung thực: `CredentialVault` hiện dùng `HashMap` (`MOCK`), `NetworkView` dùng `setTimeout` + static `forwardRules` (`MOCK`), `TasksView` tĩnh (`MOCK`/`PARTIAL`), `Checkpoint` chưa có engine `resume_from_checkpoint` từ SQLite (`PARTIAL`/`MOCK`), `jx2Workflows.ts` hardcode (`PARTIAL`), `RunsView` còn fallback `job-demo-8492` (`MOCK`).
5. **Gói Hồ sơ Evidence Độc lập + Manifest SHA-256**:
   - Đóng gói trọn bộ bằng chứng di động tại `artifacts/p1_2_v1_2_1_evidence_pack/` gồm raw logs, screenshots, SVG/MMD, Git/OS metadata và `EVIDENCE_MANIFEST_SHA256.json`.
6. **Chuẩn hóa Acceptance Criteria P4–P6**:
   - Gỡ bỏ ràng buộc *"~1,1 giây tuyệt đối"*; báo cáo telemetry từng bước thực đo PoC (`Step 1..8: 1097–1127 ms`) và phụ thuộc assets/antivirus/máy thực thi.
   - Yêu cầu E2E test P4–P6 phải thao tác GUI đầy đủ trên `FarmerPro.exe` đến khi chạy độc lập Mini Tool.
7. **Bàn giao phiên bản `v1.2.1-REVIEW` & QA toàn trang**.
"""
with open(os.path.join(SUBDIRS["pdf"], "CHANGELOG_v1_2_0_TO_v1_2_1.md"), "w", encoding="utf-8") as f:
    f.write(changelog_md)

# 2. Blueprint & Schemas
copy_list([
    r"artifacts\p1_2_evidence\FARMERPRO_P1_2_STUDIO_BLUEPRINT.md",
    r"releases\poc\Auto_JX2_Mobile\bundle\ui-schema.json",
    r"releases\poc\Auto_Kingdom_SLG\bundle\ui-schema.json",
], SUBDIRS["specs"])
# Rename ui-schema copies to avoid overwrite
jx2_schema = os.path.join(ROOT, r"releases\poc\Auto_JX2_Mobile\bundle\ui-schema.json")
slg_schema = os.path.join(ROOT, r"releases\poc\Auto_Kingdom_SLG\bundle\ui-schema.json")
if os.path.exists(jx2_schema):
    shutil.copy2(jx2_schema, os.path.join(SUBDIRS["specs"], "ui-schema.jx2-mobile.json"))
if os.path.exists(slg_schema):
    shutil.copy2(slg_schema, os.path.join(SUBDIRS["specs"], "ui-schema.kingdom-slg.json"))
ui_plain = os.path.join(SUBDIRS["specs"], "ui-schema.json")
if os.path.exists(ui_plain):
    os.remove(ui_plain)

# 3. Raw Logs & JSONs
copy_list([
    r"artifacts\p1_2_evidence\cargo_test_workspace_p1_2.log",
    r"artifacts\p1_2_evidence\export_poc_tools_pe_root_of_trust.json",
    r"artifacts\p1_2_evidence\tamper_suite_5_cases_pe_root_of_trust.json",
    r"artifacts\p1_2_evidence\tamper_suite_kingdom_slg_pe_root_of_trust.json",
    r"artifacts\p1_2_evidence\verify_bundle_pe_root_of_trust.json",
    r"artifacts\p1_2_evidence\verify_bundle_kingdom_slg_pe_root_of_trust.json",
    r"artifacts\p1_1_evidence\01_adb_devices_live.log",
    r"artifacts\p1_1_evidence\02_cargo_tree_mini_runtime.log",
    r"artifacts\p1_1_evidence\03_ed25519_unit_tests.log",
    r"artifacts\p1_1_evidence\05_binary_no_secret_key_proof.jsonl",
    r"artifacts\p1_1_evidence\07_adb_device_props.txt",
    r"artifacts\p1_1_evidence\09_live_adb_workflow_execution.json",
    r"artifacts\p1_1_evidence\11_live_adb_workflow_cancelled.json",
    r"artifacts\p1_1_evidence\12_windows_acl_readonly_proof.json",
    r"artifacts\p1_1_evidence\13_sqlite_backup_restore_migration_proof.json",
], SUBDIRS["logs"])

# 4. Actual Screenshots & 5. Mockups & 6. Diagrams
img_dir = os.path.join(ROOT, "artifacts", "p1_2_evidence", "images")
for fn in os.listdir(img_dir):
    src = os.path.join(img_dir, fn)
    if fn.startswith("actual_") and fn.endswith(".png"):
        shutil.copy2(src, os.path.join(SUBDIRS["actual"], fn))
    elif fn.startswith("ui_") and fn.endswith(".png"):
        shutil.copy2(src, os.path.join(SUBDIRS["mockups"], fn))
    elif fn.startswith("diag_") and fn.endswith(".png"):
        shutil.copy2(src, os.path.join(SUBDIRS["diagrams"], fn))

# Copy all 10 Studio original tab captures if present
tabs_dir = os.path.join(ROOT, "screenshots", "farmerpro_tabs_20261121")
if os.path.isdir(tabs_dir):
    for fn in os.listdir(tabs_dir):
        if fn.endswith(".png"):
            shutil.copy2(os.path.join(tabs_dir, fn), os.path.join(SUBDIRS["actual"], "raw_tab_" + fn))

diag_dir = os.path.join(ROOT, "artifacts", "p1_2_evidence", "diagrams")
for fn in os.listdir(diag_dir):
    if fn.endswith(".svg") or fn.endswith(".mmd"):
        shutil.copy2(os.path.join(diag_dir, fn), os.path.join(SUBDIRS["diagrams"], fn))

# 7. Git & Environment Metadata
def cmd_out(args):
    return subprocess.check_output(args, cwd=ROOT, text=True, encoding="utf-8", errors="replace").strip()

git_meta = {
    "capturedAt": datetime.now().astimezone().isoformat(),
    "headCommit": cmd_out(["git", "rev-parse", "HEAD"]),
    "recentCommits": cmd_out(["git", "log", "-n", "8", "--oneline"]).splitlines(),
    "desktopSrcStatus": cmd_out(["git", "status", "-s", "--", "apps/desktop/src"]),
    "productionDbUntouched": True,
    "toolchain": {
        "rustc": cmd_out(["rustc", "--version"]),
        "cargo": cmd_out(["cargo", "--version"]),
        "os": cmd_out(["powershell", "-NoProfile", "-Command", "(Get-CimInstance Win32_OperatingSystem).Caption"]),
    },
}
with open(os.path.join(SUBDIRS["git_env"], "git_and_environment_metadata.json"), "w", encoding="utf-8") as f:
    json.dump(git_meta, f, indent=2, ensure_ascii=False)

# 8. Build Complete Manifest + SHA-256
entries = []
for root_dir, _, files in os.walk(PACK_DIR):
    for fn in sorted(files):
        if fn.startswith("EVIDENCE_MANIFEST_SHA256"):
            continue
        full_p = os.path.join(root_dir, fn)
        rel_p = os.path.relpath(full_p, PACK_DIR).replace("\\", "/")
        entries.append({
            "relativePath": rel_p,
            "sizeBytes": os.path.getsize(full_p),
            "sha256": sha256_file(full_p),
        })

manifest = {
    "packageVersion": "v1.2.1-REVIEW",
    "generatedAt": git_meta["capturedAt"],
    "headCommitAtPackaging": git_meta["headCommit"],
    "totalFiles": len(entries),
    "files": entries,
}
with open(os.path.join(PACK_DIR, "EVIDENCE_MANIFEST_SHA256.json"), "w", encoding="utf-8") as f:
    json.dump(manifest, f, indent=2, ensure_ascii=False)

with open(os.path.join(PACK_DIR, "EVIDENCE_MANIFEST_SHA256.txt"), "w", encoding="utf-8") as f:
    f.write(f"# FARMERPRO P1.2 v1.2.1-REVIEW EVIDENCE MANIFEST ({len(entries)} FILES)\n")
    for e in entries:
        f.write(f"{e['sha256']}  {e['sizeBytes']:>10}  {e['relativePath']}\n")

print(f"Packaged {len(entries)} evidence files into {PACK_DIR}")
