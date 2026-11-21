import React, { useState } from 'react';
import { Monitor, ShieldCheck, Palette } from 'lucide-react';

export const ComponentGalleryView: React.FC = () => {
  const [selectedViewport, setSelectedViewport] = useState<'1600' | '1440' | '1280'>('1600');

  const tokens = [
    { name: 'App Background', token: '--bg-app', hex: '#101722', role: 'Nền sâu nhất của cửa sổ ứng dụng' },
    { name: 'Sidebar Background', token: '--bg-sidebar', hex: '#141C28', role: 'Thanh điều hướng bên trái và tiêu đề' },
    { name: 'Surface Background', token: '--bg-surface', hex: '#16202F', role: 'Thẻ card, bảng panel và thanh công cụ' },
    { name: 'Elevated Surface', token: '--bg-elevated', hex: '#17273D', role: 'Bảng inspector, hàng được chọn, dialog' },
    { name: 'Canvas Background', token: '--bg-canvas', hex: '#101722', role: 'Vùng canvas React Flow' },
    { name: 'Border Subtle', token: '--border-subtle', hex: '#2A3647', role: 'Đường viền ngăn cách mặc định' },
    { name: 'Border Focus', token: '--border-focus', hex: '#38BDF8', role: 'Viền focus keyboard, node đang chọn' },
    { name: 'Accent Primary', token: '--accent-primary', hex: '#1878F5', role: 'Nút hành động chính (Primary action)' },
    { name: 'Success / Verified', token: '--color-success', hex: '#3BC38D', role: 'Xác minh thành công, thiết bị ready' },
    { name: 'Warning / Retry', token: '--color-warning', hex: '#E7B15B', role: 'Chờ đợi, thử lại (retry/wait), checkpoint' },
    { name: 'Danger / Stop', token: '--color-danger', hex: '#F27883', role: 'Lỗi, dừng khẩn cấp (destructive action)' },
    { name: 'Vision Target', token: '--color-vision', hex: '#B291F7', role: 'Node nhận dạng mẫu ảnh SSD và OCR' },
  ];

  const acceptanceCriteria = [
    {
      criterion: 'Visual Hierarchy & Canvas Focus',
      specification: 'Canvas trung tâm chiếm >55% diện tích làm việc, node kích thước ≥260px đọc rõ chữ',
      status: 'Đã hoàn thiện',
    },
    {
      criterion: 'Unified App Shell & Custom Titlebar',
      specification: 'Loại bỏ titlebar trắng Windows, header gọn 44px, thanh bên 230px / 64px mượt mà',
      status: 'Đã hoàn thiện',
    },
    {
      criterion: 'Devices Management & Android API',
      specification: 'Hiển thị chính xác Android 13 (API 33), 12 (API 31), 10 (API 29); phân biệt rõ Demo vs Live',
      status: 'Đã hoàn thiện',
    },
    {
      criterion: 'Safe Random Tap Policy',
      specification: 'Co biên 18% vào trong bounding box, tránh N điểm click gần nhất, không hardcode tọa độ',
      status: 'Đã hoàn thiện',
    },
    {
      criterion: 'Task Library & Execution Planner',
      specification: 'Thư viện danh mục nhiệm vụ, chọn nhiều mục, sắp xếp thứ tự ưu tiên, dự toán thời gian',
      status: 'Đã hoàn thiện',
    },
    {
      criterion: 'Runtime Debugger & Checkpoints',
      specification: 'Log traces theo cấp độ, chi tiết tọa độ/ảnh xác minh, khôi phục từ checkpoint',
      status: 'Đã hoàn thiện',
    },
    {
      criterion: 'Security & Credential Protection',
      specification: 'Sử dụng Windows Credential Manager tham chiếu secret, không lưu plaintext mật khẩu',
      status: 'Đã hoàn thiện',
    },
    {
      criterion: 'Portable ZIP Release Packaging',
      specification: 'Bỏ NSIS/MSI, đóng gói ZIP portable với internal adb.exe, data/, logs/, README.txt',
      status: 'Đã hoàn thiện',
    },
  ];

  return (
    <div className="flex-1 overflow-y-auto p-6 space-y-6 bg-[#101722] text-[#EDF3FC]">
      {/* Top Header */}
      <div className="flex items-center justify-between">
        <div>
          <div className="flex items-center gap-2">
            <h1 className="text-xl font-bold text-[#EDF3FC] tracking-tight">
              Hệ thống Thiết kế & Tiêu chuẩn Nghiệm thu (Tokens Specification)
            </h1>
            <span className="px-2 py-0.5 rounded text-[10px] bg-[#173355] text-[#38BDF8] font-mono font-semibold border border-[#2A3647]">
              FARMERPRO_DESIGN_TOKENS.json
            </span>
          </div>
          <p className="text-xs text-[#74869D] mt-1">
            Bảng mã màu chuẩn Dark Precision tokens, kiểm tra co giãn đa độ phân giải và checklist nghiệm thu kỹ thuật.
          </p>
        </div>
      </div>

      {/* Viewport Scaling Simulator */}
      <div className="p-4 bg-[#16202F] border border-[#2A3647] rounded-xl space-y-3 shadow-sm">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2 text-xs font-bold text-[#EDF3FC]">
            <Monitor size={16} className="text-[#38BDF8]" />
            <span>Mô phỏng 3 Độ Phân giải Cửa sổ Mục tiêu</span>
          </div>
          <div className="flex items-center gap-1.5 bg-[#101722] p-1 rounded-md border border-[#2A3647] text-xs">
            <button
              onClick={() => setSelectedViewport('1600')}
              className={`px-3 py-1 rounded font-medium transition-colors ${
                selectedViewport === '1600'
                  ? 'bg-[#1878F5] text-white shadow-sm'
                  : 'text-[#74869D] hover:text-white'
              }`}
            >
              1600×900 (Mục tiêu chuẩn)
            </button>
            <button
              onClick={() => setSelectedViewport('1440')}
              className={`px-3 py-1 rounded font-medium transition-colors ${
                selectedViewport === '1440'
                  ? 'bg-[#1878F5] text-white shadow-sm'
                  : 'text-[#74869D] hover:text-white'
              }`}
            >
              1440×900 (Laptop)
            </button>
            <button
              onClick={() => setSelectedViewport('1280')}
              className={`px-3 py-1 rounded font-medium transition-colors ${
                selectedViewport === '1280'
                  ? 'bg-[#1878F5] text-white shadow-sm'
                  : 'text-[#74869D] hover:text-white'
              }`}
            >
              1280×720 (Màn hình nhỏ)
            </button>
          </div>
        </div>

        <div className="p-3.5 bg-[#101722] rounded-lg border border-[#2A3647] text-xs text-[#A9B8CC] leading-relaxed">
          {selectedViewport === '1600' && (
            <span>
              <strong>1600×900:</strong> Thanh bên mở rộng 230px, Node Palette 250px, Inspector 320px. Canvas chiếm ~800px (~55% chiều ngang). Toàn bộ node, cổng kết nối và tham số co biên đều hiển thị rõ ràng mà không cần zoom thủ công.
            </span>
          )}
          {selectedViewport === '1440' && (
            <span>
              <strong>1440×900:</strong> Thanh bên thu gọn thành Rail 64px hoặc mở 230px, Canvas giữ vững ~820px diện tích làm việc, Inspector thu gọn thành một cú click.
            </span>
          )}
          {selectedViewport === '1280' && (
            <span>
              <strong>1280×720:</strong> Thanh bên Rail 64px, Inspector có thể thu gọn vào lề, Canvas tự động mở rộng &gt;60% chiều ngang, không xảy ra hiện tượng tràn chữ hay clipping nút bấm.
            </span>
          )}
        </div>
      </div>

      {/* Design Tokens Palette */}
      <div className="rounded-xl bg-[#16202F] border border-[#2A3647] p-4 space-y-3 shadow-sm">
        <div className="flex items-center gap-2">
          <Palette size={15} className="text-[#1878F5]" />
          <h3 className="text-xs font-bold text-[#EDF3FC]">
            Bảng Mã Màu Design Tokens (Precision Dark Tokens)
          </h3>
        </div>
        <div className="grid grid-cols-4 gap-3">
          {tokens.map((t) => (
            <div
              key={t.token}
              className="p-3 rounded-lg bg-[#101722] border border-[#2A3647] flex items-center gap-3"
            >
              <div
                className="w-9 h-9 rounded-md border border-[#2A3647] shrink-0 shadow-inner"
                style={{ backgroundColor: t.hex }}
              />
              <div className="text-xs truncate min-w-0">
                <div className="font-bold text-[#EDF3FC] truncate">{t.name}</div>
                <div className="font-mono text-[10px] text-[#38BDF8]">{t.hex}</div>
                <div className="text-[10px] text-[#74869D] truncate">{t.role}</div>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Component States Showcase */}
      <div className="rounded-xl bg-[#16202F] border border-[#2A3647] p-4 space-y-3 shadow-sm">
        <h3 className="text-xs font-bold text-[#EDF3FC]">Showcase Trạng thái Component</h3>
        <div className="flex flex-wrap items-center gap-3 text-xs">
          <button className="px-3 py-1.5 bg-[#1878F5] hover:bg-[#2F8CFF] text-white rounded font-medium shadow-sm transition-colors cursor-pointer">
            Primary Default
          </button>
          <button className="px-3 py-1.5 bg-[#16202F] text-[#EDF3FC] rounded border border-[#2A3647] font-medium hover:bg-[#202B3B] transition-colors cursor-pointer">
            Secondary Button
          </button>
          <button className="px-3 py-1.5 bg-[#101722] text-[#5A6D85] rounded border border-[#2A3647] cursor-not-allowed">
            Disabled State
          </button>
          <button className="px-3 py-1.5 bg-[#F27883] hover:bg-[#ff8f99] text-white rounded font-medium shadow-sm transition-colors cursor-pointer">
            Destructive Action
          </button>
          <span className="px-2.5 py-1 rounded text-[11px] bg-[#133528] text-[#3BC38D] border border-[#3BC38D]/40 font-bold">
            Verified Success
          </span>
          <span className="px-2.5 py-1 rounded text-[11px] bg-[#3B2D17] text-[#E7B15B] border border-[#E7B15B]/40 font-bold">
            Retry / Waiting
          </span>
          <span className="px-2.5 py-1 rounded text-[11px] bg-[#251C3B] text-[#B291F7] border border-[#B291F7]/40 font-bold">
            Vision Target SSD
          </span>
          <span className="px-2.5 py-1 rounded text-[11px] bg-[#173355] text-[#38BDF8] border border-[#38BDF8]/40 font-bold">
            Safe Random Tap
          </span>
        </div>
      </div>

      {/* Independent Acceptance Checklist */}
      <div className="rounded-xl bg-[#16202F] border border-[#2A3647] p-4 space-y-3 shadow-sm">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <ShieldCheck size={16} className="text-[#3BC38D]" />
            <h3 className="text-xs font-bold text-[#EDF3FC]">
              Bảng Tiêu chuẩn Nghiệm thu Kỹ thuật & Giao diện (Acceptance Matrix)
            </h3>
          </div>
          <span className="text-[11px] text-[#74869D]">Theo FarmerPro Master Blueprint V1.0</span>
        </div>

        <table className="w-full text-left text-xs border-collapse">
          <thead className="bg-[#101722] text-[#74869D] border-b border-[#2A3647] font-bold text-[11px] uppercase tracking-wider">
            <tr>
              <th className="p-3">Hạng mục kiểm nghiệm</th>
              <th className="p-3">Quy chuẩn kỹ thuật</th>
              <th className="p-3 text-right">Trạng thái</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-[#2A3647]">
            {acceptanceCriteria.map((row) => (
              <tr key={row.criterion} className="hover:bg-[#17273D] transition-colors">
                <td className="p-3 font-bold text-[#EDF3FC]">{row.criterion}</td>
                <td className="p-3 text-[#A9B8CC] text-[11px]">{row.specification}</td>
                <td className="p-3 text-right">
                  <span className="px-2 py-0.5 rounded text-[10px] bg-[#133528] text-[#3BC38D] border border-[#3BC38D]/40 font-bold">
                    {row.status}
                  </span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
};
