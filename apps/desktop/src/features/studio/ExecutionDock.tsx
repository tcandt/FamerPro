import React, { useState, useRef } from 'react';
import {
  Clock,
  Terminal,
  Image as ImageIcon,
  Variable,
  ChevronDown,
  ChevronUp,
  CheckCircle2,
  Copy,
  GripHorizontal,
  Search,
} from 'lucide-react';
import { useWorkflowStore } from '../../stores/workflowStore';
import { useDeviceStore } from '../../stores/deviceStore';

export const ExecutionDock: React.FC = () => {
  const {
    activeDockTab,
    setActiveDockTab,
    isDockCollapsed,
    toggleDock,
    dockHeight,
    setDockHeight,
    runEvents,
    isRunning,
  } = useWorkflowStore();
  const selectedDeviceId = useDeviceStore((state) => state.selectedDeviceId);

  const [logFilter, setLogFilter] = useState('ALL');
  const [logSearch, setLogSearch] = useState('');
  const isDraggingRef = useRef(false);

  const handleMouseDown = (e: React.MouseEvent) => {
    e.preventDefault();
    isDraggingRef.current = true;
    const startY = e.clientY;
    const startHeight = dockHeight;

    const onMouseMove = (moveEvent: MouseEvent) => {
      if (!isDraggingRef.current) return;
      const delta = startY - moveEvent.clientY;
      const newHeight = Math.min(Math.max(startHeight + delta, 120), 480);
      setDockHeight(newHeight);
    };

    const onMouseUp = () => {
      isDraggingRef.current = false;
      window.removeEventListener('mousemove', onMouseMove);
      window.removeEventListener('mouseup', onMouseUp);
    };

    window.addEventListener('mousemove', onMouseMove);
    window.addEventListener('mouseup', onMouseUp);
  };

  const logs = [
    { level: 'INFO', time: '10:14:02.100', text: 'Core device lease acquired (fencing generation: 1, serial: emulator-5554)' },
    { level: 'TRACE', time: '10:14:02.250', text: 'Snapshot PNG captured (1080x2400@0deg, latency 32ms)' },
    { level: 'VISION', time: '10:14:02.410', text: 'Normalized SSD match: score=0.965 at bounds (200, 400, 150, 60)' },
    { level: 'ACTION', time: '10:14:02.580', text: 'Safe point sampled (x=242.0, y=428.0), avoiding recent 5 click history' },
    { level: 'TRACE', time: '10:14:02.720', text: 'ADB input tap dispatched: exit code 0' },
    { level: 'VERIFY', time: '10:14:03.110', text: 'Post-condition dialog_confirm confirmed on fresh capture frame (confidence 94%)' },
    { level: 'INFO', time: '10:14:03.200', text: 'Checkpoint quest_active saved to SQLite WAL storage' },
  ];

  const filteredLogs = logs.filter((l) => {
    const matchesLevel = logFilter === 'ALL' || l.level === logFilter;
    const matchesSearch =
      logSearch === '' || l.text.toLowerCase().includes(logSearch.toLowerCase());
    return matchesLevel && matchesSearch;
  });

  return (
    <div
      style={{ height: isDockCollapsed ? 36 : dockHeight }}
      className="border-t border-[#2A3647] bg-[#141C28] flex flex-col z-20 transition-all select-none relative shrink-0 shadow-2xl"
    >
      {/* Drag handle on top border to resize */}
      {!isDockCollapsed && (
        <div
          onMouseDown={handleMouseDown}
          className="h-1.5 w-full bg-[#16202F] hover:bg-[#38BDF8] cursor-row-resize flex items-center justify-center transition-colors group"
          title="Kéo chuột để thay đổi chiều cao thanh Debugger"
        >
          <GripHorizontal size={14} className="text-[#5A6D85] group-hover:text-white" />
        </div>
      )}

      {/* Dock Header & Tab Navigation */}
      <div className="h-9 px-3 bg-[#16202F] border-b border-[#2A3647] flex items-center justify-between">
        <div className="flex items-center space-x-1">
          <button
            onClick={() => setActiveDockTab('results')}
            className={`px-2.5 py-1 text-xs font-semibold rounded flex items-center gap-1.5 transition-colors cursor-pointer ${
              activeDockTab === 'results' && !isDockCollapsed
                ? 'bg-[#1878F5] text-white shadow-sm'
                : 'text-[#A9B8CC] hover:text-white hover:bg-[#202B3B]'
            }`}
          >
            <CheckCircle2 size={13} />
            <span>Kết quả</span>
          </button>

          <button
            onClick={() => setActiveDockTab('timeline')}
            className={`px-2.5 py-1 text-xs font-semibold rounded flex items-center gap-1.5 transition-colors cursor-pointer ${
              activeDockTab === 'timeline' && !isDockCollapsed
                ? 'bg-[#1878F5] text-white shadow-sm'
                : 'text-[#A9B8CC] hover:text-white hover:bg-[#202B3B]'
            }`}
          >
            <Clock size={13} />
            <span>Tiến trình ({runEvents.length})</span>
          </button>

          <button
            onClick={() => setActiveDockTab('logs')}
            className={`px-2.5 py-1 text-xs font-semibold rounded flex items-center gap-1.5 transition-colors cursor-pointer ${
              activeDockTab === 'logs' && !isDockCollapsed
                ? 'bg-[#1878F5] text-white shadow-sm'
                : 'text-[#A9B8CC] hover:text-white hover:bg-[#202B3B]'
            }`}
          >
            <Terminal size={13} />
            <span>Nhật ký Debug</span>
          </button>

          <button
            onClick={() => setActiveDockTab('verification')}
            className={`px-2.5 py-1 text-xs font-semibold rounded flex items-center gap-1.5 transition-colors cursor-pointer ${
              activeDockTab === 'verification' && !isDockCollapsed
                ? 'bg-[#1878F5] text-white shadow-sm'
                : 'text-[#A9B8CC] hover:text-white hover:bg-[#202B3B]'
            }`}
          >
            <ImageIcon size={13} />
            <span>Ảnh xác minh</span>
          </button>

          <button
            onClick={() => setActiveDockTab('variables')}
            className={`px-2.5 py-1 text-xs font-semibold rounded flex items-center gap-1.5 transition-colors cursor-pointer ${
              activeDockTab === 'variables' && !isDockCollapsed
                ? 'bg-[#1878F5] text-white shadow-sm'
                : 'text-[#A9B8CC] hover:text-white hover:bg-[#202B3B]'
            }`}
          >
            <Variable size={13} />
            <span>Biến</span>
          </button>
        </div>

        <div className="flex items-center gap-2">
          {!isDockCollapsed && (
            <span className="text-[10px] text-[#5A6D85] font-mono">
              Chiều cao: {dockHeight}px
            </span>
          )}
          <button
            onClick={toggleDock}
            className="p-1 text-[#74869D] hover:text-[#EDF3FC] hover:bg-[#202B3B] rounded transition-colors"
            title={isDockCollapsed ? 'Mở rộng Debugger' : 'Thu gọn Debugger'}
          >
            {isDockCollapsed ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
          </button>
        </div>
      </div>

      {/* Dock Content Body */}
      {!isDockCollapsed && (
        <div className="flex-1 overflow-y-auto p-3 text-xs bg-[#101722]">
          {/* Results Tab */}
          {activeDockTab === 'results' && (
            <div className="grid grid-cols-4 gap-3 text-xs">
              <div className="p-3 bg-[#16202F] border border-[#2A3647] rounded-lg">
                <span className="text-[#74869D] text-[11px]">Trạng thái thực thi:</span>
                <div className="font-bold text-[#3BC38D] text-sm mt-0.5">
                  {isRunning ? 'Đang gửi lệnh ADB...' : 'Hoàn tất & Đã xác minh (Verified)'}
                </div>
                <div className="mt-1 text-[11px] text-[#A9B8CC]">
                  Tổng thời gian: 1.18s · 8/8 nodes hoàn tất
                </div>
              </div>

              <div className="p-3 bg-[#16202F] border border-[#2A3647] rounded-lg">
                <span className="text-[#74869D] text-[11px]">Mức độ an toàn tọa độ:</span>
                <div className="font-bold text-[#38BDF8] text-sm mt-0.5">100% Tuân thủ Policy</div>
                <div className="mt-1 text-[11px] text-[#A9B8CC]">
                  Biên 18% co vào trong, tránh 5 điểm cũ
                </div>
              </div>

              <div className="p-3 bg-[#16202F] border border-[#2A3647] rounded-lg">
                <span className="text-[#74869D] text-[11px]">Độ tin cậy nhận diện:</span>
                <div className="font-bold text-[#EDF3FC] text-sm mt-0.5">96.5% (Ngưỡng: 92%)</div>
                <div className="mt-1 text-[11px] text-[#3BC38D]">
                  Vượt ngưỡng tối thiểu +4.5%
                </div>
              </div>

              <div className="p-3 bg-[#16202F] border border-[#2A3647] rounded-lg">
                <span className="text-[#74869D] text-[11px]">Checkpoint lưu trữ:</span>
                <div className="font-bold text-[#FDE047] text-sm mt-0.5 font-mono">quest_active</div>
                <div className="mt-1 text-[11px] text-[#A9B8CC]">
                  Lưu vào SQLite WAL an toàn
                </div>
              </div>
            </div>
          )}

          {/* Timeline Tab */}
          {activeDockTab === 'timeline' && (
            <div className="space-y-1.5 font-mono text-[11px]">
              {runEvents.length === 0 ? (
                <div className="text-[#74869D] italic py-4 text-center">
                  Chưa có sự kiện thực thi nào trong phiên này. Nhấn "Chạy luồng" ở thanh tiêu đề để bắt đầu.
                </div>
              ) : (
                runEvents.map((ev) => (
                  <div
                    key={ev.id}
                    className="flex items-center justify-between p-2 rounded-md bg-[#16202F] border border-[#2A3647]"
                  >
                    <div className="flex items-center gap-2.5">
                      <span className="px-1.5 py-0.5 rounded text-[10px] bg-[#173355] text-[#38BDF8] font-bold">
                        {ev.node_id}
                      </span>
                      <span className="text-[#EDF3FC] font-semibold">{ev.event_type}</span>
                    </div>
                    <div className="flex items-center gap-3 text-[#74869D]">
                      <span className="truncate max-w-[240px] text-[#A9B8CC]">
                        {ev.payload_json}
                      </span>
                      <span>{new Date(ev.at).toLocaleTimeString()}</span>
                    </div>
                  </div>
                ))
              )}
            </div>
          )}

          {/* Logs Tab */}
          {activeDockTab === 'logs' && (
            <div className="space-y-2 font-mono text-[11px]">
              <div className="flex items-center justify-between pb-2 border-b border-[#2A3647]">
                <div className="flex items-center gap-2">
                  <span className="text-[#74869D] text-xs">Lọc cấp độ:</span>
                  {['ALL', 'INFO', 'VISION', 'ACTION', 'VERIFY'].map((lvl) => (
                    <button
                      key={lvl}
                      onClick={() => setLogFilter(lvl)}
                      className={`px-2 py-0.5 rounded text-[10px] font-semibold transition-colors ${
                        logFilter === lvl
                          ? 'bg-[#1878F5] text-white'
                          : 'bg-[#16202F] text-[#74869D] hover:text-white'
                      }`}
                    >
                      {lvl}
                    </button>
                  ))}
                </div>

                <div className="flex items-center gap-2">
                  <div className="flex items-center gap-1 bg-[#16202F] border border-[#2A3647] rounded px-2 py-0.5 text-xs text-[#74869D]">
                    <Search size={11} />
                    <input
                      type="text"
                      placeholder="Tìm nhật ký..."
                      value={logSearch}
                      onChange={(e) => setLogSearch(e.target.value)}
                      className="bg-transparent text-[#EDF3FC] outline-none text-[11px] w-28"
                    />
                  </div>
                  <button
                    onClick={() =>
                      navigator.clipboard?.writeText(JSON.stringify(logs, null, 2))
                    }
                    className="flex items-center gap-1 px-2 py-1 bg-[#16202F] hover:bg-[#202B3B] text-[#EDF3FC] rounded text-xs border border-[#2A3647]"
                  >
                    <Copy size={11} /> Copy
                  </button>
                </div>
              </div>

              <div className="space-y-1">
                {filteredLogs.map((l, idx) => (
                  <div key={idx} className="flex items-center gap-2 leading-relaxed">
                    <span className="text-[#5A6D85]">{l.time}</span>
                    <span
                      className={`px-1 rounded text-[9px] font-bold ${
                        l.level === 'INFO'
                          ? 'bg-[#133528] text-[#3BC38D]'
                          : l.level === 'VISION'
                          ? 'bg-[#251C3B] text-[#B291F7]'
                          : l.level === 'ACTION'
                          ? 'bg-[#0F2C3F] text-[#38BDF8]'
                          : l.level === 'VERIFY'
                          ? 'bg-[#1A4D39] text-[#6EE7B7]'
                          : 'bg-[#202B3B] text-[#74869D]'
                      }`}
                    >
                      {l.level}
                    </span>
                    <span className="text-[#EDF3FC]">{l.text}</span>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Verification Tab */}
          {activeDockTab === 'verification' && (
            <div className="flex items-center gap-4">
              <div className="p-2.5 bg-[#16202F] border border-[#2A3647] rounded-lg text-center">
                <div className="w-36 h-36 bg-[#101722] border border-[#2A3647] rounded-md flex flex-col items-center justify-center text-[#74869D] text-[10px] mb-2 relative overflow-hidden">
                  <div className="absolute inset-x-4 top-10 h-10 border-2 border-[#1878F5] bg-[#1878F5]/20 flex items-center justify-center text-white text-[10px] font-bold rounded">
                    quest_marker.png
                  </div>
                  <div className="absolute bottom-2 left-2 text-[9px] text-[#5A6D85] font-mono">
                    Frame 01 (1080×2400)
                  </div>
                </div>
                <div className="font-semibold text-[#3BC38D] text-xs">Khớp mẫu (96.5%)</div>
                <div className="text-[10px] text-[#74869D] font-mono">Tọa độ: (200, 400, 150, 60)</div>
              </div>

              <div className="p-2.5 bg-[#16202F] border border-[#2A3647] rounded-lg text-center">
                <div className="w-36 h-36 bg-[#101722] border border-[#2A3647] rounded-md flex flex-col items-center justify-center text-[#74869D] text-[10px] mb-2 relative overflow-hidden">
                  <div className="absolute inset-x-3 top-8 bottom-8 border-2 border-[#3BC38D] bg-[#3BC38D]/20 flex items-center justify-center text-white text-[10px] font-bold rounded">
                    dialog_confirm.png
                  </div>
                  <div className="absolute bottom-2 left-2 text-[9px] text-[#5A6D85] font-mono">
                    Frame 02 (1080×2400)
                  </div>
                </div>
                <div className="font-semibold text-[#3BC38D] text-xs">Xác nhận mở hộp thoại</div>
                <div className="text-[10px] text-[#74869D] font-mono">Độ trễ phản hồi: 320ms</div>
              </div>
            </div>
          )}

          {/* Variables Tab */}
          {activeDockTab === 'variables' && (
            <div className="grid grid-cols-3 gap-2.5">
              <div className="p-2.5 bg-[#16202F] border border-[#2A3647] rounded-md">
                <span className="text-[#74869D] text-[10px] block">Tên gói ứng dụng</span>
                <span className="font-mono text-[#6BB0FF] font-semibold">com.example.game</span>
              </div>
              <div className="p-2.5 bg-[#16202F] border border-[#2A3647] rounded-md">
                <span className="text-[#74869D] text-[10px] block">Trạng thái mốc an toàn</span>
                <span className="font-mono text-[#FDE047] font-semibold">quest_active = true</span>
              </div>
              <div className="p-2.5 bg-[#16202F] border border-[#2A3647] rounded-md">
                <span className="text-[#74869D] text-[10px] block">Thiết bị gán hiện tại</span>
                <span className="font-mono text-[#3BC38D] font-semibold">{selectedDeviceId || 'emulator-5554'}</span>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
};
