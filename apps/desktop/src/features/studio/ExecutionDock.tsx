import React from 'react';
import {
  Clock,
  Terminal,
  Image as ImageIcon,
  Variable,
  ChevronDown,
  ChevronUp,
  CheckCircle2,
  Copy,
} from 'lucide-react';
import { useWorkflowStore } from '../../stores/workflowStore';

export const ExecutionDock: React.FC = () => {
  const {
    activeDockTab,
    setActiveDockTab,
    isDockCollapsed,
    toggleDock,
    runEvents,
    isRunning,
  } = useWorkflowStore();

  return (
    <div
      className={`border-t border-[#2A3647] bg-[#161E29] flex flex-col z-20 transition-all select-none ${
        isDockCollapsed ? 'h-9' : 'h-[220px]'
      }`}
    >
      <div className="h-9 px-3 bg-[#192230] border-b border-[#2A3647] flex items-center justify-between">
        <div className="flex items-center space-x-1">
          <button
            onClick={() => setActiveTabSafe('results')}
            className={`px-3 py-1 text-xs font-medium rounded flex items-center gap-1.5 transition-colors ${
              activeDockTab === 'results' && !isDockCollapsed
                ? 'bg-[#1878F5] text-white'
                : 'text-[#A9B8CC] hover:text-white hover:bg-[#202B3B]'
            }`}
          >
            <CheckCircle2 size={13} />
            <span>Kết quả</span>
          </button>

          <button
            onClick={() => setActiveTabSafe('timeline')}
            className={`px-3 py-1 text-xs font-medium rounded flex items-center gap-1.5 transition-colors ${
              activeDockTab === 'timeline' && !isDockCollapsed
                ? 'bg-[#1878F5] text-white'
                : 'text-[#A9B8CC] hover:text-white hover:bg-[#202B3B]'
            }`}
          >
            <Clock size={13} />
            <span>Tiến trình ({runEvents.length})</span>
          </button>

          <button
            onClick={() => setActiveTabSafe('logs')}
            className={`px-3 py-1 text-xs font-medium rounded flex items-center gap-1.5 transition-colors ${
              activeDockTab === 'logs' && !isDockCollapsed
                ? 'bg-[#1878F5] text-white'
                : 'text-[#A9B8CC] hover:text-white hover:bg-[#202B3B]'
            }`}
          >
            <Terminal size={13} />
            <span>Nhật ký (Logs)</span>
          </button>

          <button
            onClick={() => setActiveTabSafe('verification')}
            className={`px-3 py-1 text-xs font-medium rounded flex items-center gap-1.5 transition-colors ${
              activeDockTab === 'verification' && !isDockCollapsed
                ? 'bg-[#1878F5] text-white'
                : 'text-[#A9B8CC] hover:text-white hover:bg-[#202B3B]'
            }`}
          >
            <ImageIcon size={13} />
            <span>Ảnh xác minh</span>
          </button>

          <button
            onClick={() => setActiveTabSafe('variables')}
            className={`px-3 py-1 text-xs font-medium rounded flex items-center gap-1.5 transition-colors ${
              activeDockTab === 'variables' && !isDockCollapsed
                ? 'bg-[#1878F5] text-white'
                : 'text-[#A9B8CC] hover:text-white hover:bg-[#202B3B]'
            }`}
          >
            <Variable size={13} />
            <span>Biến</span>
          </button>
        </div>

        <button
          onClick={toggleDock}
          className="p-1 text-[#74869D] hover:text-[#EDF3FC] hover:bg-[#202B3B] rounded"
          title={isDockCollapsed ? 'Mở rộng Execution Dock' : 'Thu gọn Execution Dock'}
        >
          {isDockCollapsed ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
        </button>
      </div>

      {!isDockCollapsed && (
        <div className="flex-1 overflow-y-auto p-3 text-xs bg-[#101722]">
          {activeDockTab === 'results' && (
            <div className="flex items-center gap-4 text-xs">
              <div className="p-3 bg-[#192230] border border-[#2A3647] rounded w-64">
                <span className="text-[#74869D]">Trạng thái: </span>
                <span className="font-semibold text-[#3BC38D]">
                  {isRunning ? 'Đang thực thi...' : 'Hoàn tất thành công (Verified)'}
                </span>
                <div className="mt-2 text-[11px] text-[#A9B8CC]">
                  Tổng thời gian: 1.48s · 8/8 nodes đã xác minh
                </div>
              </div>
            </div>
          )}

          {activeDockTab === 'timeline' && (
            <div className="space-y-1.5 font-mono text-[11px]">
              {runEvents.length === 0 ? (
                <div className="text-[#74869D] italic py-4 text-center">
                  Chưa có sự kiện thực thi nào. Bấm "Chạy luồng" ở trên để bắt đầu quan sát tiến trình.
                </div>
              ) : (
                runEvents.map((ev) => (
                  <div
                    key={ev.id}
                    className="flex items-center justify-between p-1.5 rounded bg-[#192230] border border-[#2A3647]"
                  >
                    <div className="flex items-center gap-2">
                      <span className="px-1.5 py-0.5 rounded text-[10px] bg-[#173355] text-[#4B9CFF]">
                        {ev.node_id}
                      </span>
                      <span className="text-[#EDF3FC]">{ev.event_type}</span>
                    </div>
                    <div className="flex items-center gap-3 text-[#74869D]">
                      <span className="truncate max-w-[200px]">{ev.payload_json}</span>
                      <span>{new Date(ev.at).toLocaleTimeString()}</span>
                    </div>
                  </div>
                ))
              )}
            </div>
          )}

          {activeDockTab === 'logs' && (
            <div className="space-y-1 font-mono text-[11px]">
              <div className="flex items-center justify-between pb-1 text-[#74869D] border-b border-[#2A3647]">
                <span>Log Traces (Tự động ẩn thông tin nhạy cảm)</span>
                <button
                  onClick={() => navigator.clipboard?.writeText(JSON.stringify(runEvents, null, 2))}
                  className="flex items-center gap-1 hover:text-white"
                >
                  <Copy size={11} /> Copy
                </button>
              </div>
              <div className="text-[#3BC38D]">[INFO] Core device lease acquired (fencing generation: 1)</div>
              <div className="text-[#A9B8CC]">[TRACE] Snapshot PNG captured (1080x1920@0deg, 1.2MB)</div>
              <div className="text-[#B291F7]">[VISION] Normalized SSD match: score=0.985 at (200, 400, 150, 60)</div>
              <div className="text-[#4B9CFF]">[ACTION] Safe point sampled (x=242.0, y=428.0) avoiding recent 5 clicks</div>
              <div className="text-[#3BC38D]">[VERIFY] Post-condition confirmed on fresh capture frame</div>
            </div>
          )}

          {activeDockTab === 'verification' && (
            <div className="flex items-center gap-3">
              <div className="p-2 bg-[#192230] border border-[#2A3647] rounded text-center">
                <div className="w-28 h-28 bg-[#121A26] border border-[#2A3647] rounded flex items-center justify-center text-[#74869D] text-[10px] mb-1.5 relative overflow-hidden">
                  <div className="absolute inset-x-4 top-10 h-8 border-2 border-[#1878F5] bg-[#1878F5]/20 flex items-center justify-center text-white text-[9px] font-bold">
                    Target
                  </div>
                  <span>Frame 01</span>
                </div>
                <span className="text-[10px] text-[#3BC38D]">Tìm thấy (98.5%)</span>
              </div>

              <div className="p-2 bg-[#192230] border border-[#2A3647] rounded text-center">
                <div className="w-28 h-28 bg-[#121A26] border border-[#2A3647] rounded flex items-center justify-center text-[#74869D] text-[10px] mb-1.5 relative overflow-hidden">
                  <div className="absolute inset-x-2 top-6 bottom-6 border-2 border-[#3BC38D] bg-[#3BC38D]/20 flex items-center justify-center text-white text-[9px] font-bold">
                    Dialog
                  </div>
                  <span>Frame 02</span>
                </div>
                <span className="text-[10px] text-[#3BC38D]">Đã mở hội thoại</span>
              </div>
            </div>
          )}

          {activeDockTab === 'variables' && (
            <div className="grid grid-cols-2 gap-2">
              <div className="p-2 bg-[#192230] border border-[#2A3647] rounded">
                <span className="text-[#74869D]">packageId: </span>
                <span className="font-mono text-[#EDF3FC]">com.example.game</span>
              </div>
              <div className="p-2 bg-[#192230] border border-[#2A3647] rounded">
                <span className="text-[#74869D]">checkpoint_quest_active: </span>
                <span className="font-mono text-[#EDF3FC]">true</span>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );

  function setActiveTabSafe(tab: any) {
    setActiveDockTab(tab);
  }
};
