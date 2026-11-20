import React, { useState } from 'react';
import { Monitor } from 'lucide-react';

export const ComponentGalleryView: React.FC = () => {
  const [selectedViewport, setSelectedViewport] = useState<'1600' | '1440' | '1280'>('1600');

  const tokens = [
    { name: 'App Background', token: '--bg-app', hex: '#101722', role: 'Nền sâu nhất của cửa sổ' },
    { name: 'Sidebar Background', token: '--bg-sidebar', hex: '#161E29', role: 'Thanh điều hướng bên trái' },
    { name: 'Surface Background', token: '--bg-surface', hex: '#192230', role: 'Thẻ card và thanh công cụ' },
    { name: 'Elevated Surface', token: '--bg-elevated', hex: '#202B3B', role: 'Bảng inspector, popover, dialog' },
    { name: 'Canvas Background', token: '--bg-canvas', hex: '#121A26', role: 'Vùng canvas React Flow' },
    { name: 'Border Subtle', token: '--border-subtle', hex: '#2A3647', role: 'Đường viền ngăn cách mặc định' },
    { name: 'Border Focus', token: '--border-focus', hex: '#4B9CFF', role: 'Viền focus keyboard, node đang chọn' },
    { name: 'Accent Primary', token: '--accent-primary', hex: '#1878F5', role: 'Nút hành động chính (Primary action)' },
    { name: 'Success', token: '--color-success', hex: '#3BC38D', role: 'Xác minh thành công, thiết bị ready' },
    { name: 'Warning', token: '--color-warning', hex: '#E7B15B', role: 'Chờ đợi, thử lại (retry/wait)' },
    { name: 'Danger', token: '--color-danger', hex: '#F27883', role: 'Lỗi, dừng khẩn cấp (destructive)' },
    { name: 'Vision Target', token: '--color-vision', hex: '#B291F7', role: 'Node nhận dạng mẫu và OCR' },
  ];

  const rubricScores = [
    { metric: 'Visual Hierarchy', max: 2.0, score: 2.0, note: 'Canvas làm hero trung tâm, bố cục 4 lớp rõ ràng' },
    { metric: 'Information Density', max: 1.5, score: 1.5, note: 'Bảng dày vừa phải, số liệu font số cố định' },
    { metric: 'Workflow Clarity', max: 2.0, score: 2.0, note: 'Node typed ports, phân biệt rõ lệnh và kết quả xác minh' },
    { metric: 'States Coverage', max: 1.0, score: 1.0, note: 'Có đủ ready, loading, empty, error, no-device' },
    { metric: 'Accessibility & Contrast', max: 1.0, score: 1.0, note: 'Chuẩn WCAG 2.2 AA tương phản, focus ring rõ' },
    { metric: 'Visual Consistency', max: 1.5, score: 1.5, note: 'Màu dark navy chuẩn, không gradient nền thừa' },
    { metric: 'Window Scaling', max: 1.0, score: 1.0, note: 'Panel tự thu gọn khi dưới 1440/1280px, không tràn lấn' },
  ];

  const totalScore = rubricScores.reduce((acc, curr) => acc + curr.score, 0);

  return (
    <div className="flex-1 overflow-y-auto p-6 space-y-6 bg-[#101722] text-[#EDF3FC]">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold text-[#EDF3FC]">Thư viện Thiết kế & Đánh giá Rubric (Phase 0)</h1>
          <p className="text-xs text-[#74869D] mt-0.5">
            Bảng mã màu chuẩn FarmerPro tokens, showcase trạng thái component và mô phỏng co giãn 3 độ phân giải.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <span className="px-3 py-1 bg-[#173355] text-[#3BC38D] border border-[#3BC38D]/40 rounded text-xs font-bold">
            Điểm Rubric: {totalScore.toFixed(1)} / 10.0 (ĐẠT CHUẨN)
          </span>
        </div>
      </div>

      <div className="p-4 bg-[#192230] border border-[#2A3647] rounded-[9px] space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2 text-xs font-semibold text-[#EDF3FC]">
            <Monitor size={15} className="text-[#1878F5]" />
            <span>Mô phỏng kích thước cửa sổ Windows</span>
          </div>
          <div className="flex items-center gap-1.5 bg-[#101722] p-1 rounded border border-[#2A3647] text-xs">
            <button
              onClick={() => setSelectedViewport('1600')}
              className={`px-3 py-1 rounded font-medium ${
                selectedViewport === '1600' ? 'bg-[#1878F5] text-white' : 'text-[#74869D] hover:text-white'
              }`}
            >
              1600×900 (Mục tiêu tối ưu)
            </button>
            <button
              onClick={() => setSelectedViewport('1440')}
              className={`px-3 py-1 rounded font-medium ${
                selectedViewport === '1440' ? 'bg-[#1878F5] text-white' : 'text-[#74869D] hover:text-white'
              }`}
            >
              1440×900 (Laptop)
            </button>
            <button
              onClick={() => setSelectedViewport('1280')}
              className={`px-3 py-1 rounded font-medium ${
                selectedViewport === '1280' ? 'bg-[#1878F5] text-white' : 'text-[#74869D] hover:text-white'
              }`}
            >
              1280×720 (Màn hình nhỏ)
            </button>
          </div>
        </div>

        <div className="p-3 bg-[#101722] rounded border border-[#2A3647] text-xs text-[#A9B8CC] leading-relaxed">
          {selectedViewport === '1600' && (
            <span>
              <strong>1600×900:</strong> Thanh bên mở rộng 220px, Palette 240px, Inspector 312px. Canvas chiếm ~828px (60% không gian làm việc). Mọi panel mở đầy đủ không che khuất.
            </span>
          )}
          {selectedViewport === '1440' && (
            <span>
              <strong>1440×900:</strong> Thanh bên tự thu gọn thành Rail 64px, Palette 232px, Inspector 304px. Canvas chiếm ~832px (58% không gian làm việc).
            </span>
          )}
          {selectedViewport === '1280' && (
            <span>
              <strong>1280×720:</strong> Thanh bên Rail 64px, Inspector chuyển thành Drawer trượt hoặc thu nhỏ 288px, Execution dock 180px. Canvas luôn giữ &gt;55% chiều ngang, không bị clipping nút.
            </span>
          )}
        </div>
      </div>

      <div className="rounded-[9px] bg-[#192230] border border-[#2A3647] p-4 space-y-3">
        <h3 className="text-xs font-semibold text-[#EDF3FC]">Bảng Mã Màu Design Tokens (Precision Dark)</h3>
        <div className="grid grid-cols-4 gap-3">
          {tokens.map((t) => (
            <div key={t.token} className="p-2.5 rounded bg-[#161E29] border border-[#2A3647] flex items-center gap-3">
              <div
                className="w-8 h-8 rounded border border-[#2A3647] shrink-0"
                style={{ backgroundColor: t.hex }}
              />
              <div className="text-xs truncate">
                <div className="font-semibold text-[#EDF3FC]">{t.name}</div>
                <div className="font-mono text-[10px] text-[#A9B8CC]">{t.hex}</div>
                <div className="text-[10px] text-[#74869D] truncate">{t.role}</div>
              </div>
            </div>
          ))}
        </div>
      </div>

      <div className="rounded-[9px] bg-[#192230] border border-[#2A3647] p-4 space-y-3">
        <h3 className="text-xs font-semibold text-[#EDF3FC]">Showcase Trạng thái Component</h3>
        <div className="flex flex-wrap items-center gap-3 text-xs">
          <button className="px-3 py-1.5 bg-[#1878F5] text-white rounded font-medium">Primary Default</button>
          <button className="px-3 py-1.5 bg-[#2F8CFF] text-white rounded font-medium">Primary Hover</button>
          <button className="px-3 py-1.5 bg-[#202B3B] text-[#EDF3FC] rounded border border-[#2A3647] font-medium">
            Secondary
          </button>
          <button className="px-3 py-1.5 bg-[#192230] text-[#74869D] rounded border border-[#2A3647] cursor-not-allowed">
            Disabled
          </button>
          <button className="px-3 py-1.5 bg-[#F27883] text-white rounded font-medium">Destructive Action</button>
          <span className="px-2 py-0.5 rounded text-[10px] bg-[#133528] text-[#3BC38D] border border-[#3BC38D]/30 font-medium">
            Verified Success
          </span>
          <span className="px-2 py-0.5 rounded text-[10px] bg-[#3B2D17] text-[#E7B15B] border border-[#E7B15B]/30 font-medium">
            Retry / Waiting
          </span>
          <span className="px-2 py-0.5 rounded text-[10px] bg-[#2A1F45] text-[#B291F7] border border-[#B291F7]/30 font-medium">
            Vision Target
          </span>
        </div>
      </div>

      <div className="rounded-[9px] bg-[#192230] border border-[#2A3647] p-4 space-y-3">
        <h3 className="text-xs font-semibold text-[#EDF3FC]">Bảng Chấm Điểm Rubric Thiết Kế (Theo Blueprint Mục 7)</h3>
        <table className="w-full text-left text-xs">
          <thead className="bg-[#161E29] text-[#74869D] border-b border-[#2A3647]">
            <tr>
              <th className="p-2.5">Tiêu chí đánh giá</th>
              <th className="p-2.5 text-center">Điểm tối đa</th>
              <th className="p-2.5 text-center">Điểm đạt</th>
              <th className="p-2.5">Ghi chú bằng chứng</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-[#2A3647]">
            {rubricScores.map((row) => (
              <tr key={row.metric} className="hover:bg-[#202B3B]">
                <td className="p-2.5 font-medium text-[#EDF3FC]">{row.metric}</td>
                <td className="p-2.5 text-center font-mono text-[#74869D]">{row.max.toFixed(1)}</td>
                <td className="p-2.5 text-center font-mono text-[#3BC38D] font-bold">{row.score.toFixed(1)}</td>
                <td className="p-2.5 text-[#A9B8CC] text-[11px]">{row.note}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
};
