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
  RotateCcw,
} from 'lucide-react';
import { useWorkflowStore } from '../../stores/workflowStore';
import { useDeviceStore } from '../../stores/deviceStore';
import { api } from '../../lib/api';

export const ExecutionDock: React.FC = () => {
  const {
    workflowId,
    activeDockTab,
    setActiveDockTab,
    isDockCollapsed,
    toggleDock,
    dockHeight,
    setDockHeight,
    runEvents,
    isRunning,
    activeJobId,
    activeJobDetail,
    executionSummary,
    lastExecutionNotice,
    setActiveJobState,
    setRunEvents,
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

  const filteredEvents = runEvents.filter((ev) => {
    const matchesLevel =
      logFilter === 'ALL' || ev.event_type.toUpperCase() === logFilter.toUpperCase();
    const matchesSearch =
      logSearch === '' ||
      ev.node_id.toLowerCase().includes(logSearch.toLowerCase()) ||
      ev.payload_json.toLowerCase().includes(logSearch.toLowerCase());
    return matchesLevel && matchesSearch;
  });

  const handleResumeCheckpoint = async () => {
    if (!activeJobId) return;
    try {
      const sum = await api.studioResumeJobFromCheckpoint(activeJobId, 20, 25000);
      const detail = await api.studioGetJobResumeDetail(activeJobId);
      setActiveJobState(
        activeJobId,
        detail,
        sum,
        `Resume checkpoint -> ${sum.state} (gen=${sum.fencingGeneration})`
      );
      if (detail) setRunEvents(detail.stepEvents);
    } catch (err: any) {
      setActiveJobState(
        activeJobId,
        activeJobDetail,
        executionSummary,
        `Resume Checkpoint lỗi: ${err?.message || String(err)}`
      );
    }
  };

  const latestCp = activeJobDetail?.resumeContext.latestCheckpoint;
  const varsObj = executionSummary?.variables || {};

  return (
    <div
      style={{ height: isDockCollapsed ? 36 : dockHeight }}
      className="border-t border-[#2A3647] bg-[#141C28] flex flex-col z-20 transition-all select-none relative shrink-0 shadow-2xl"
    >
      {!isDockCollapsed && (
        <div
          onMouseDown={handleMouseDown}
          className="h-1.5 w-full bg-[#16202F] hover:bg-[#38BDF8] cursor-row-resize flex items-center justify-center transition-colors group"
          title="Kéo để thay đổi chiều cao Execution Dock"
        >
          <GripHorizontal size={14} className="text-[#5A6D85] group-hover:text-white" />
        </div>
      )}

      {/* Dock Header */}
      <div className="h-9 px-3 bg-[#16202F] border-b border-[#2A3647] flex items-center justify-between gap-2">
        <div className="flex items-center space-x-1">
          {[
            { id: 'results', label: 'Kết quả', icon: CheckCircle2 },
            { id: 'timeline', label: `Tiến trình (${runEvents.length})`, icon: Clock },
            { id: 'logs', label: 'Nhật ký Debug', icon: Terminal },
            { id: 'verification', label: 'Checkpoint & Evidence', icon: ImageIcon },
            { id: 'variables', label: 'Biến Runtime', icon: Variable },
          ].map((t) => {
            const Icon = t.icon;
            const active = activeDockTab === t.id && !isDockCollapsed;
            return (
              <button
                key={t.id}
                onClick={() => setActiveDockTab(t.id as any)}
                className={`px-2.5 py-1 text-xs font-semibold rounded flex items-center gap-1.5 transition-colors cursor-pointer ${
                  active
                    ? 'bg-[#1878F5] text-white shadow-sm'
                    : 'text-[#A9B8CC] hover:text-white hover:bg-[#202B3B]'
                }`}
              >
                <Icon size={13} />
                <span>{t.label}</span>
              </button>
            );
          })}
        </div>

        <div className="flex items-center gap-2 min-w-0">
          {lastExecutionNotice && (
            <span className="text-[11px] text-[#E7B15B] truncate max-w-[300px]">
              {lastExecutionNotice}
            </span>
          )}
          <button
            onClick={toggleDock}
            className="p-1 text-[#74869D] hover:text-[#EDF3FC] hover:bg-[#202B3B] rounded transition-colors cursor-pointer"
            title={isDockCollapsed ? 'Mở Debugger' : 'Thu gọn Debugger'}
          >
            {isDockCollapsed ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
          </button>
        </div>
      </div>

      {/* Dock Content Body */}
      {!isDockCollapsed && (
        <div className="flex-1 overflow-y-auto p-3 text-xs bg-[#101722]">
          {activeDockTab === 'results' && (
            <>
              {!executionSummary && !activeJobDetail ? (
                <div className="text-[#74869D] py-6 text-center">
                  Chưa có kết quả thực thi thật cho workflow <code>{workflowId}</code>. Nhấn{' '}
                  <strong>Coordinator</strong> hoặc <strong>Chạy luồng</strong> để thực thi.
                </div>
              ) : (
                <div className="grid grid-cols-2 lg:grid-cols-4 gap-3">
                  <div className="p-3 bg-[#16202F] border border-[#2A3647] rounded-lg">
                    <span className="text-[#74869D] text-[11px]">Trạng thái Job:</span>
                    <div className="font-bold text-[#EDF3FC] text-sm mt-0.5 uppercase">
                      {isRunning
                        ? 'RUNNING'
                        : executionSummary?.state || activeJobDetail?.resumeContext.job.state || 'N/A'}
                    </div>
                    <div className="mt-1 text-[11px] font-mono text-[#A9B8CC] truncate">
                      {activeJobId || executionSummary?.jobId}
                    </div>
                  </div>

                  <div className="p-3 bg-[#16202F] border border-[#2A3647] rounded-lg">
                    <span className="text-[#74869D] text-[11px]">Lease Fencing Gen:</span>
                    <div className="font-bold text-[#38BDF8] text-sm mt-0.5 font-mono">
                      gen=
                      {executionSummary?.fencingGeneration ??
                        activeJobDetail?.resumeContext.job.fencingGeneration ??
                        0}
                    </div>
                    <div className="mt-1 text-[11px] text-[#A9B8CC]">
                      Adapter: {executionSummary?.adapterKind || 'adb'}
                    </div>
                  </div>

                  <div className="p-3 bg-[#16202F] border border-[#2A3647] rounded-lg">
                    <span className="text-[#74869D] text-[11px]">Step Events (SQLite):</span>
                    <div className="font-bold text-[#3BC38D] text-sm mt-0.5">
                      {runEvents.length} events
                    </div>
                    <div className="mt-1 text-[11px] text-[#A9B8CC] truncate">
                      Node: {executionSummary?.currentNodeId || activeJobDetail?.resumeContext.job.currentNodeId || '-'}
                    </div>
                  </div>

                  <div className="p-3 bg-[#16202F] border border-[#2A3647] rounded-lg">
                    <span className="text-[#74869D] text-[11px]">Lý do Blocked / Guard:</span>
                    <div className="font-semibold text-[#E7B15B] text-[11px] mt-0.5 break-words">
                      {executionSummary?.blockReason || 'Không bị chặn'}
                    </div>
                  </div>
                </div>
              )}
            </>
          )}

          {activeDockTab === 'timeline' && (
            <div className="space-y-1.5 font-mono text-[11px]">
              {runEvents.length === 0 ? (
                <div className="text-[#74869D] italic py-5 text-center">
                  Chưa có StepEvent nào được ghi nhận từ backend Rust.
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
                      <span className="text-[#74869D]">#{ev.attempt_no}</span>
                    </div>
                    <div className="flex items-center gap-3 text-[#74869D]">
                      <span className="truncate max-w-[300px] text-[#A9B8CC]">{ev.payload_json}</span>
                      <span>{new Date(ev.at).toLocaleTimeString()}</span>
                    </div>
                  </div>
                ))
              )}
            </div>
          )}

          {activeDockTab === 'logs' && (
            <div className="space-y-2 font-mono text-[11px]">
              <div className="flex items-center justify-between pb-2 border-b border-[#2A3647]">
                <div className="flex items-center gap-1.5 flex-wrap">
                  {['ALL', 'COMMAND_SENT', 'DEVICE_ACK', 'VERIFICATION_FAILED', 'CHECKPOINT_SAVED'].map(
                    (lvl) => (
                      <button
                        key={lvl}
                        onClick={() => setLogFilter(lvl)}
                        className={`px-2 py-0.5 rounded text-[10px] font-semibold cursor-pointer ${
                          logFilter === lvl
                            ? 'bg-[#1878F5] text-white'
                            : 'bg-[#16202F] text-[#74869D] hover:text-white'
                        }`}
                      >
                        {lvl}
                      </button>
                    )
                  )}
                </div>

                <div className="flex items-center gap-2">
                  <div className="flex items-center gap-1 bg-[#16202F] border border-[#2A3647] rounded px-2 py-0.5 text-xs text-[#74869D]">
                    <Search size={11} />
                    <input
                      type="text"
                      placeholder="Lọc node/payload..."
                      value={logSearch}
                      onChange={(e) => setLogSearch(e.target.value)}
                      className="bg-transparent text-[#EDF3FC] outline-none text-[11px] w-32"
                    />
                  </div>
                  <button
                    onClick={() => navigator.clipboard?.writeText(JSON.stringify(filteredEvents, null, 2))}
                    className="flex items-center gap-1 px-2 py-1 bg-[#16202F] hover:bg-[#202B3B] text-[#EDF3FC] rounded text-xs border border-[#2A3647] cursor-pointer"
                  >
                    <Copy size={11} /> Copy
                  </button>
                </div>
              </div>

              {filteredEvents.length === 0 ? (
                <div className="text-[#74869D] py-4 text-center">Không có bản ghi log nào.</div>
              ) : (
                <div className="space-y-1">
                  {filteredEvents.map((ev) => (
                    <div key={ev.id} className="flex items-center gap-2 leading-relaxed">
                      <span className="text-[#5A6D85]">{new Date(ev.at).toLocaleTimeString()}</span>
                      <span className="px-1.5 rounded text-[9.5px] bg-[#173355] text-[#4B9CFF] font-bold">
                        {ev.event_type}
                      </span>
                      <span className="text-[#3BC38D]">[{ev.node_id}]</span>
                      <span className="text-[#EDF3FC] truncate">{ev.payload_json}</span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}

          {activeDockTab === 'verification' && (
            <div className="space-y-2.5">
              {!latestCp ? (
                <div className="text-[#74869D] py-5 text-center">
                  Chưa có checkpoint nào trong SQLite cho job hiện hành.
                </div>
              ) : (
                <div className="p-3 rounded-lg bg-[#16202F] border border-[#2A3647] flex items-center justify-between gap-3">
                  <div className="space-y-1 min-w-0">
                    <div className="font-semibold text-[#EDF3FC]">
                      Checkpoint <code>{latestCp.id}</code> (node <code>{latestCp.nodeId}</code>)
                    </div>
                    <div className="font-mono text-[11px] text-[#A9B8CC] truncate">
                      {latestCp.stateJson}
                    </div>
                  </div>
                  <button
                    onClick={handleResumeCheckpoint}
                    className="px-3 py-1.5 rounded bg-[#1878F5] hover:bg-[#2F8CFF] text-white text-xs font-semibold flex items-center gap-1 cursor-pointer shrink-0"
                  >
                    <RotateCcw size={12} />
                    <span>Resume Từ Checkpoint</span>
                  </button>
                </div>
              )}
            </div>
          )}

          {activeDockTab === 'variables' && (
            <div className="grid grid-cols-1 sm:grid-cols-3 gap-2.5">
              <div className="p-2.5 bg-[#16202F] border border-[#2A3647] rounded-md">
                <span className="text-[#74869D] text-[10px] block">Workflow ID</span>
                <span className="font-mono text-[#6BB0FF] font-semibold">{workflowId}</span>
              </div>
              <div className="p-2.5 bg-[#16202F] border border-[#2A3647] rounded-md">
                <span className="text-[#74869D] text-[10px] block">Active Device Context</span>
                <span className="font-mono text-[#3BC38D] font-semibold">
                  {selectedDeviceId || 'Chưa chọn thiết bị'}
                </span>
              </div>
              <div className="p-2.5 bg-[#16202F] border border-[#2A3647] rounded-md">
                <span className="text-[#74869D] text-[10px] block">Biến thực thi thu được</span>
                <span className="font-mono text-[#A9B8CC] text-[11px] break-all">
                  {Object.keys(varsObj).length > 0 ? JSON.stringify(varsObj) : 'Chưa có biến runtime'}
                </span>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
};
