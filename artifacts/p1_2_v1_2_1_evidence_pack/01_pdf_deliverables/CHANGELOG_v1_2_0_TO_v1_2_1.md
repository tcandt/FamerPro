# CHANGELOG — FARMERPRO P1.2 ACCEPTANCE REPORT (v1.2.0 -> v1.2.1-REVIEW)

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
