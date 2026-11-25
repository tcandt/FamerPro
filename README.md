# FarmerPro — Android Workflow Automation Studio

FarmerPro is a professional desktop automation studio for Android (supporting Android 9–13) built with a high-performance **Rust 2024** core engine and a **Tauri v2 + React 19 + TypeScript + React Flow** visual graph interface.

## Highlights
- **Visual Node Canvas**: Drag-and-drop workflow editor inspired by n8n, tailored for mobile automation.
- **Universal Element Inspector & Smart Action Engine**: Inspect native UI hierarchy, OCR text, and visual elements dynamically. No hardcoded coordinates.
- **Dynamic Safe Tap Policy**: Targets are dynamically located inside observed bounding boxes, shrunk by inner safe margins (15–18%), subtracted by exclusion zones, and sampled with recent-point avoidance.
- **Verification-First Execution**: Commands verify post-conditions with fresh screen captures and semantic hierarchy diffs before advancing checkpoints.
- **Robust Multi-Device Scheduling**: Single active serialized action queue per device, Win32 `LockFileEx` cross-process hardware leases, and fencing generations.
- **Local-First & Secure**: SQLite WAL persistence, secrets stored in Windows Credential Manager / DPAPI, redacted log traces, no cloud lock-in.

---

## 🚀 Mới Nhất: Universal Element Inspector (PoC Verified)
Đã hoàn thành kiểm chứng thực tế Phase 1 trên phần cứng điện thoại thật qua mạng LAN (**Samsung Galaxy S7 Android 10 LineageOS**):
- 📄 **Báo cáo kỹ thuật chi tiết**: [docs/FARMERPRO_ELEMENT_INSPECTOR_POC_REPORT.md](docs/FARMERPRO_ELEMENT_INSPECTOR_POC_REPORT.md)
- 🧪 **Script kiểm thử tự động**: [scripts/poc_test_element.py](scripts/poc_test_element.py)
- 📸 **Bằng chứng thực thi (Before/After & XML Tree)**: [artifacts/element_inspector_poc/](artifacts/element_inspector_poc/)

---

## Architecture Overview
```text
FarmerPro/
├── apps/
│   ├── desktop/            # Tauri v2 + React 19 + Vite + Tailwind CSS v4 Studio UI
│   ├── cli/                # Standalone CLI diagnostic runner & headless worker
│   └── mini-runtime/       # Ultra-compact runtime for standalone exported mini-tools
├── crates/
│   ├── fp-domain/          # Core identifiers, Bounds, ScreenGeometry, ActionReceipt
│   ├── fp-adb/             # Tokio-based ADB runner, USB/LAN discovery, screencap, input
│   ├── fp-device/          # Device actor abstraction, geometry metadata, orientation
│   ├── fp-vision/          # Template matching, bounding box normalization, ROI
│   ├── fp-actions/         # Safe random tap policy, exclusion masks, swipe generator
│   ├── fp-workflow-schema/ # Versioned workflow JSON graph validation
│   ├── fp-workflow-runtime/# State machine executor, coordinator, retry, checkpoints
│   ├── fp-scheduler/       # Multi-device leases (Win32 LockFileEx), priority queue
│   ├── fp-storage/         # SQLite v6 WAL migrations, CRUD for workflows/jobs
│   ├── fp-security/        # Windows DPAPI, WinCred secrets redaction
│   ├── fp-builder/         # Mini-tool compiler, Ed25519 bundle signer, Authenticode
│   └── fp-updater/         # Standalone external updater with anti-rollback ledger
├── docs/                   # Architecture decision records & design specifications
└── artifacts/              # Acceptance test evidence, verification packs, and PDF reports
```

---

## 🛠️ Hướng Dẫn Dành Cho Team Remote (Developer Runbook)

### 1. Yêu cầu môi trường
- **Rust**: 1.96+ (x86_64-pc-windows-msvc)
- **Node.js**: v20+ hoặc v24+
- **pnpm**: v10+ hoặc v11+
- **Android Platform Tools (ADB)**: Đã thêm vào `PATH`
- **Python**: 3.10+ (phục vụ chạy các script kiểm thử và kiểm tra report)

### 2. Cài đặt và kiểm tra toàn bộ workspace
```powershell
# Clone repository
git clone https://github.com/tcandt/FamerPro.git
cd FamerPro

# Cài đặt dependencies frontend
cd apps/desktop
pnpm install
cd ../..

# Chạy toàn bộ 99+ Unit & Integration Tests của Rust core
cargo test --workspace
```

### 3. Chạy Desktop Studio ở chế độ Dev
```powershell
cd apps/desktop
pnpm tauri dev
```

### 4. Chạy PoC Element Inspector trên điện thoại thật (LAN / USB)
```powershell
# Đảm bảo thiết bị đã kết nối adb (ví dụ adb devices)
python scripts/poc_test_element.py
```

### 5. Xem các báo cáo nghiệm thu chính (Master Acceptance Reports)
- `FARMERPRO_MASTER_BLUEPRINT_V1.0.md`: Bản thiết kế tổng thể kiến trúc v1.0.
- `FARMERPRO_P6_ACCEPTANCE_REPORT.pdf`: Báo cáo nghiệm thu giai đoạn P6 (Bảo mật, phân quyền 6 thành phần, rollback).
- `FARMERPRO_GATE1_SANDBOX_REPORT.pdf`: Báo cáo nghiệm thu môi trường Clean Windows Sandbox.
- `FARMERPRO_RC4_ADDENDUM_REPORT.pdf`: Báo cáo nghiệm thu khóa phần cứng cross-process `LockFileEx` & SQLite v6.
- `docs/FARMERPRO_ELEMENT_INSPECTOR_POC_REPORT.md`: Báo cáo thiết kế & kiểm chứng Universal Element Inspector.
