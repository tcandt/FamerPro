import React from 'react';
import { Copy } from 'lucide-react';
import { useWorkflowStore } from '../../stores/workflowStore';

export const RunsView: React.FC = () => {
  const { runEvents, isRunning } = useWorkflowStore();

  return (
    <div className="flex-1 overflow-y-auto p-6 space-y-6 bg-[#101722] text-[#EDF3FC]">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold text-[#EDF3FC]">Nhật ký Phiên chạy (Runtime Monitor)</h1>
          <p className="text-xs text-[#74869D] mt-0.5">
            Chi tiết từng bước gửi lệnh, xác nhận từ thiết bị và kết quả xác minh qua ảnh chụp.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <span
            className={`px-3 py-1 rounded text-xs font-semibold border ${
              isRunning
                ? 'bg-[#3B2D17] text-[#E7B15B] border-[#E7B15B]/30'
                : 'bg-[#133528] text-[#3BC38D] border-[#3BC38D]/30'
            }`}
          >
            {isRunning ? 'Đang thực thi' : 'Sẵn sàng'}
          </span>
        </div>
      </div>

      <div className="rounded-[9px] bg-[#192230] border border-[#2A3647] p-4 space-y-3">
        <div className="flex items-center justify-between border-b border-[#2A3647] pb-3 text-xs">
          <span className="font-semibold text-[#EDF3FC]">Dòng sự kiện (Step Trace Table)</span>
          <button
            onClick={() => navigator.clipboard?.writeText(JSON.stringify(runEvents, null, 2))}
            className="flex items-center gap-1.5 px-2.5 py-1 bg-[#202B3B] hover:bg-[#2A3647] text-[#EDF3FC] rounded text-xs"
          >
            <Copy size={12} />
            <span>Sao chép toàn bộ Trace</span>
          </button>
        </div>

        <div className="space-y-2 font-mono text-xs">
          {runEvents.length === 0 ? (
            <div className="text-center py-8 text-[#74869D] italic">
              Chưa có phiên chạy nào gần đây. Hãy khởi chạy từ Studio hoặc Bộ nhiệm vụ.
            </div>
          ) : (
            runEvents.map((ev) => (
              <div
                key={ev.id}
                className="p-3 bg-[#161E29] border border-[#2A3647] rounded flex items-center justify-between"
              >
                <div className="flex items-center gap-3">
                  <span className="px-2 py-0.5 bg-[#173355] text-[#4B9CFF] rounded text-[11px] font-bold">
                    {ev.node_id}
                  </span>
                  <span className="text-[#EDF3FC] font-semibold">{ev.event_type}</span>
                  <span className="text-[#74869D] text-[11px] truncate max-w-[300px]">
                    {ev.payload_json}
                  </span>
                </div>
                <div className="text-[#74869D] text-[11px]">
                  {new Date(ev.at).toLocaleTimeString()} (lần thử {ev.attempt_no})
                </div>
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
};
