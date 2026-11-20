import React from 'react';
import { Plus, Upload, CheckCircle2, ArrowRight } from 'lucide-react';

export const WorkflowsView: React.FC<{ onOpenStudio: () => void }> = ({ onOpenStudio }) => {
  const workflows = [
    {
      id: 'example-beginner-quest',
      name: 'Nhiệm vụ tân thủ — DEMO',
      version: 'v1.0.0',
      nodesCount: 8,
      lastEdited: 'Vừa xong',
      status: 'Hợp lệ',
      category: 'Tân thủ',
    },
    {
      id: 'auto-combat-daily',
      name: 'Tự động phó bản hằng ngày',
      version: 'v0.9.2',
      nodesCount: 14,
      lastEdited: 'Hôm qua',
      status: 'Bản nháp',
      category: 'Chiến đấu',
    },
    {
      id: 'claim-rewards-idle',
      name: 'Thu thập quà online & điểm danh',
      version: 'v1.1.0',
      nodesCount: 6,
      lastEdited: '3 ngày trước',
      status: 'Hợp lệ',
      category: 'Nhận thưởng',
    },
  ];

  return (
    <div className="flex-1 overflow-y-auto p-6 space-y-6 bg-[#101722] text-[#EDF3FC]">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold text-[#EDF3FC]">Kho Quy trình (Workflow Library)</h1>
          <p className="text-xs text-[#74869D] mt-0.5">
            Các đồ thị tự động hóa đã lưu. Nhấp để chỉnh sửa trên canvas hoặc xuất bản phiên bản mới.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button className="px-3 py-1.5 bg-[#202B3B] hover:bg-[#2A3647] text-[#EDF3FC] text-xs font-medium rounded border border-[#2A3647] flex items-center gap-1.5">
            <Upload size={13} />
            <span>Nhập JSON</span>
          </button>
          <button
            onClick={onOpenStudio}
            className="px-3.5 py-1.5 bg-[#1878F5] hover:bg-[#2F8CFF] text-white text-xs font-semibold rounded flex items-center gap-1.5 shadow-sm"
          >
            <Plus size={14} />
            <span>Tạo quy trình mới</span>
          </button>
        </div>
      </div>

      <div className="grid grid-cols-3 gap-4">
        {workflows.map((wf) => (
          <div
            key={wf.id}
            onClick={onOpenStudio}
            className="p-4 rounded-[9px] bg-[#192230] border border-[#2A3647] hover:border-[#1878F5] cursor-pointer transition-all flex flex-col justify-between h-44 group"
          >
            <div>
              <div className="flex items-center justify-between text-xs mb-2">
                <span className="px-2 py-0.5 rounded text-[10px] bg-[#173355] text-[#4B9CFF] font-medium border border-[#2A3647]">
                  {wf.category}
                </span>
                <span className="text-[11px] text-[#74869D] font-mono">{wf.version}</span>
              </div>
              <h3 className="text-sm font-semibold text-[#EDF3FC] group-hover:text-[#4B9CFF] transition-colors">
                {wf.name}
              </h3>
              <p className="text-[11px] text-[#74869D] mt-1">{wf.nodesCount} nodes · Sửa {wf.lastEdited}</p>
            </div>

            <div className="flex items-center justify-between pt-3 border-t border-[#2A3647] text-xs">
              <span className="flex items-center gap-1 text-[#3BC38D] text-[11px]">
                <CheckCircle2 size={12} />
                <span>{wf.status}</span>
              </span>
              <span className="text-xs text-[#1878F5] group-hover:translate-x-1 transition-transform flex items-center gap-1">
                <span>Chỉnh sửa</span>
                <ArrowRight size={13} />
              </span>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};
