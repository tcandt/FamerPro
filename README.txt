================================================================================
FARMERPRO — ANDROID WORKFLOW AUTOMATION STUDIO
Phiên bản: v0.1.0-portable (Windows x64)
Bản quyền: (c) 2026 FarmerPro Development Team
================================================================================

1. GIỚI THIỆU
-------------
FarmerPro là bộ công cụ desktop tự động hóa quy trình Android chuyên nghiệp,
xây dựng trên nền tảng Rust Automation Core kết hợp Tauri v2 và React Studio Canvas.
Hệ thống tuân thủ nghiêm ngặt chính sách Safe Random Tap Policy (chống tọa độ cố định,
co biên an toàn vào trong bounding box) và Local-First Privacy.

2. HƯỚNG DẪN KHỞI CHẠY (ZERO-INSTALL PORTABLE)
---------------------------------------------
- Bước 1: Giải nén tệp zip "FarmerPro-v0.1.0-win-x64-portable.zip" ra bất kỳ thư mục nào
          (ví dụ: D:\FarmerPro hoặc C:\FarmerPro).
- Bước 2: Nhấp đúp chuột vào file "FarmerPro.exe" để mở ứng dụng ngay lập tức.
- KHÔNG yêu cầu cài đặt trình cài đặt (NSIS/MSI).
- KHÔNG yêu cầu quyền Administrator để sử dụng thông thường.
- KHÔNG yêu cầu cài đặt trước Node.js, Python, Rust hay Android SDK trên máy tính.

3. YÊU CẦU HỆ THỐNG
-------------------
- Hệ điều hành: Windows 10 hoặc Windows 11 (64-bit x64).
- Runtime hiển thị: Microsoft Edge WebView2 Runtime (đã có sẵn mặc định trên Windows 10/11).
- Kết nối thiết bị: Cáp USB hoặc mạng WiFi (ADB over TCP/IP). Điện thoại Android cần bật
  chế độ "Tùy chọn cho nhà phát triển" và "Gỡ lỗi USB" (USB Debugging).

4. CẤU TRÚC THƯ MỤC PORTABLE
----------------------------
FarmerPro/
├── FarmerPro.exe          -> Ứng dụng desktop chính
├── bin/                   -> Công cụ thực thi nội bộ đóng gói sẵn
│   ├── adb.exe            -> Android Debug Bridge v1.0.41 nội bộ (ưu tiên tự động)
│   ├── AdbWinApi.dll      -> Thư viện giao tiếp USB Windows
│   └── AdbWinUsbApi.dll   -> Thư viện giao tiếp WinUSB
├── assets/                -> Tài nguyên nhận dạng thị giác máy
│   ├── templates/         -> Ảnh mẫu SSD template matching (quest_marker.png, dialog_confirm.png)
│   └── models/            -> Thư mục dự phòng cho mô hình tương lai
├── workflows/             -> Thư viện quy trình tự động hóa JSON
│   └── beginner_quest.json -> Quy trình nhiệm vụ tân thủ chuẩn mực mẫu
├── data/                  -> Cơ sở dữ liệu SQLite cục bộ (farmerpro.db + WAL)
├── logs/                  -> Thư mục lưu vết nhật ký gỡ lỗi
└── README.txt             -> Hướng dẫn sử dụng và tài liệu phát hành

5. CƠ CHẾ BẢO MẬT & DỮ LIỆU CỤC BỘ (LOCAL-FIRST)
-----------------------------------------------
- Mọi dữ liệu cấu hình, thiết bị và đồ thị quy trình được lưu cục bộ trong thư mục `data/`
  bằng SQLite ở chế độ Write-Ahead Logging (WAL) hiệu năng cao.
- Thông tin đăng nhập và secret token được ủy quyền cho Windows Credential Manager
  thông qua Windows DPAPI của hệ điều hành, tuyệt đối không lưu trữ plaintext trong JSON
  hoặc cơ sở dữ liệu.
- Hoàn toàn không gửi dữ liệu màn hình hay thông tin tài khoản lên bất kỳ máy chủ đám mây nào.

6. HƯỚNG DẪN NÂNG CẤP PHIÊN BẢN (UPDATE)
----------------------------------------
- Khi có bản phát hành mới, người dùng chỉ cần sao chép đè file `FarmerPro.exe` và
  thư mục `bin/` mới.
- Toàn bộ cơ sở dữ liệu trong `data/`, kịch bản trong `workflows/` và ảnh mẫu trong
  `assets/` của người dùng được giữ nguyên vẹn, không bị mất mát.

Hỗ trợ kỹ thuật & Góp ý: FarmerPro Engineering Team
Website: https://github.com/farmerpro/studio
================================================================================
