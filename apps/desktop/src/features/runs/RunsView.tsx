import React, { useState } from 'react';
import {
  Copy,
  CheckCircle2,
  RotateCcw,
  Smartphone,
  MousePointerClick,
  Bookmark,
} from 'lucide-react';
import { useWorkflowStore } from '../../stores/workflowStore';
import { useDeviceStore } from '../../stores/deviceStore';
import { StepEvent } from '../../types';

export const RunsView: React.FC = () => {
  const { runEvents, isRunning, clearRunEvents } = useWorkflowStore();
  const { devices, selectedDeviceId, selectDevice } = useDeviceStore();

  const [selectedEventId, setSelectedEventId] = useState<string | null>(null);
  const [checkpointRestored, setCheckpointRestored] = useState<string | null>(null);

  // Default synthetic events if none yet
  const displayEvents: StepEvent[] =
    runEvents.length > 0
      ? runEvents
      : [
          {
            id: 'evt-1',
            job_id: 'job-demo-8492',
            node_id: 'n-start',
            event_type: 'command_sent',
            attempt_no: 1,
            at: new Date(Date.now() - 4000).toISOString(),
            payload_json: '{"action":"initialize_pipeline","session_id":"sess-01"}',
          },
          {
            id: 'evt-2',
            job_id: 'job-demo-8492',
            node_id: 'n-launch',
            event_type: 'device_ack',
            attempt_no: 1,
            at: new Date(Date.now() - 3200).toISOString(),
            payload_json: '{"package":"com.example.game","status":"running","pid":4120}',
          },
          {
            id: 'evt-3',
            job_id: 'job-demo-8492',
            node_id: 'n-find',
            event_type: 'verification_passed',
            attempt_no: 1,
            at: new Date(Date.now() - 2100).toISOString(),
            payload_json: '{"template":"quest_marker.png","score":0.965,"bounds":[200,400,150,60]}',
          },
          {
            id: 'evt-4',
            job_id: 'job-demo-8492',
            node_id: 'n-tap',
            event_type: 'device_ack',
            attempt_no: 1,
            at: new Date(Date.now() - 1500).toISOString(),
            payload_json: '{"cmd":"input tap 242 428","sampled_point":[242.0,428.0],"margin":0.18}',
          },
          {
            id: 'evt-5',
            job_id: 'job-demo-8492',
            node_id: 'n-verify',
            event_type: 'verification_passed',
            attempt_no: 1,
            at: new Date(Date.now() - 800).toISOString(),
            payload_json: '{"dialog_confirmed":true,"confidence":0.94,"latency_ms":320}',
          },
          {
            id: 'evt-6',
            job_id: 'job-demo-8492',
            node_id: 'n-checkpoint',
            event_type: 'checkpoint_saved',
            attempt_no: 1,
            at: new Date(Date.now() - 200).toISOString(),
            payload_json: '{"name":"quest_active","saved_to":"sqlite_wal","step_index":5}',
          },
        ];

  const activeEvent =
    displayEvents.find((e) => e.id === selectedEventId) || displayEvents[displayEvents.length - 1];

  const handleRestoreCheckpoint = (name: string) => {
    setCheckpointRestored(name);
    setTimeout(() => {
      setCheckpointRestored(null);
    }, 2500);
  };

  return (
    <div className="flex-1 flex flex-col p-6 space-y-5 overflow-hidden bg-[#101722] text-[#EDF3FC]">
      {/* Top Header */}
      <div className="flex items-center justify-between">
        <div>
          <div className="flex items-center gap-2">
            <h1 className="text-xl font-bold text-[#EDF3FC] tracking-tight">
              Nhật ký Phiên chạy (Runtime Monitor & Debugger)
            </h1>
            <span
              className={`px-2.5 py-0.5 rounded text-[10px] font-bold border ${
                isRunning
                  ? 'bg-[#3B2D17] text-[#E7B15B] border-[#E7B15B]/40 animate-pulse'
                  : 'bg-[#133528] text-[#3BC38D] border-[#3BC38D]/40'
              }`}
            >
              {isRunning ? 'Đang gửi lệnh ADB' : 'Sẵn sàng'}
            </span>
          </div>
          <p className="text-xs text-[#74869D] mt-1">
            Gỡ lỗi từng bước tương tác: kiểm tra tọa độ an toàn, điểm SSD so khớp mẫu và phục hồi checkpoint.
          </p>
        </div>

        <div className="flex items-center gap-3">
          <div className="flex items-center gap-1.5 bg-[#16202F] border border-[#2A3647] rounded-md px-2.5 py-1 text-xs text-[#EDF3FC]">
            <Smartphone size={13} className="text-[#1878F5]" />
            <span className="text-[#74869D] text-[11px]">Thiết bị:</span>
            <select
              value={selectedDeviceId || ''}
              onChange={(e) => selectDevice(e.target.value)}
              className="bg-transparent text-[#EDF3FC] outline-none cursor-pointer text-xs font-semibold"
            >
              {devices.map((d) => (
                <option key={d.id} value={d.id} className="bg-[#161E29] text-[#EDF3FC]">
                  {d.alias}
                </option>
              ))}
            </select>
          </div>

          <button
            onClick={() => navigator.clipboard?.writeText(JSON.stringify(displayEvents, null, 2))}
            className="px-3 py-1.5 bg-[#16202F] hover:bg-[#202B3B] text-[#EDF3FC] rounded-md text-xs font-medium border border-[#2A3647] flex items-center gap-1.5 transition-colors cursor-pointer"
          >
            <Copy size={12} />
            <span>Sao chép JSON</span>
          </button>

          <button
            onClick={clearRunEvents}
            className="px-2.5 py-1.5 bg-[#16202F] hover:bg-[#202B3B] text-[#74869D] hover:text-[#F27883] rounded-md text-xs font-medium border border-[#2A3647] transition-colors"
          >
            Xóa nhật ký
          </button>
        </div>
      </div>

      {/* 2-Column Split: Left = Event Timeline + Checkpoints, Right = Step Inspector */}
      <div className="grid grid-cols-12 gap-6 flex-1 min-h-0">
        {/* Left Column (6 cols): Event Timeline */}
        <div className="col-span-6 flex flex-col space-y-4 overflow-hidden">
          {/* Timeline Table */}
          <div className="flex-1 rounded-xl bg-[#16202F] border border-[#2A3647] p-4 flex flex-col overflow-hidden shadow-sm">
            <div className="flex items-center justify-between pb-3 border-b border-[#2A3647]">
              <div>
                <h3 className="text-xs font-bold text-[#EDF3FC]">Dòng sự kiện Thực thi (Step Trace)</h3>
                <span className="text-[11px] text-[#74869D]">
                  Nhấp vào từng dòng để xem chi tiết tọa độ và ảnh xác minh
                </span>
              </div>
              <span className="font-mono text-xs text-[#38BDF8] font-semibold">
                {displayEvents.length} events
              </span>
            </div>

            <div className="flex-1 overflow-y-auto space-y-2 mt-3 pr-1">
              {displayEvents.map((ev, idx) => {
                const isSelected = ev.id === activeEvent?.id;
                return (
                  <div
                    key={ev.id}
                    onClick={() => setSelectedEventId(ev.id)}
                    className={`p-3 rounded-lg border text-xs cursor-pointer transition-all flex items-center justify-between ${
                      isSelected
                        ? 'bg-[#172E49] border-[#38BDF8]'
                        : 'bg-[#101722] border-[#2A3647] hover:border-[#3D4F67]'
                    }`}
                  >
                    <div className="flex items-center gap-2.5 min-w-0">
                      <span className="font-mono text-[10px] text-[#5A6D85]">#{idx + 1}</span>
                      <span className="px-1.5 py-0.5 rounded text-[10px] bg-[#173355] text-[#38BDF8] font-mono font-bold">
                        {ev.node_id}
                      </span>
                      <div className="min-w-0">
                        <span className="font-semibold text-[#EDF3FC] block truncate">
                          {ev.event_type}
                        </span>
                        <span className="text-[10px] text-[#74869D] font-mono block truncate max-w-[200px]">
                          {ev.payload_json}
                        </span>
                      </div>
                    </div>

                    <div className="text-right shrink-0">
                      <span className="text-[10px] text-[#5A6D85] block font-mono">
                        {new Date(ev.at).toLocaleTimeString()}
                      </span>
                      <span className="text-[9px] text-[#3BC38D] font-mono">Lần thử {ev.attempt_no}</span>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>

          {/* Checkpoint Recovery Drawer */}
          <div className="rounded-xl bg-[#16202F] border border-[#2A3647] p-4 shadow-sm space-y-3">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <Bookmark size={15} className="text-[#FDE047]" />
                <h3 className="text-xs font-bold text-[#EDF3FC]">
                  Điểm an toàn đã lưu (Checkpoints)
                </h3>
              </div>
              <span className="text-[10px] text-[#74869D]">Khôi phục khi gặp lỗi</span>
            </div>

            <div className="p-3 rounded-lg bg-[#101722] border border-[#2A3647] flex items-center justify-between text-xs">
              <div>
                <div className="font-mono text-[#FDE047] font-bold">quest_active</div>
                <div className="text-[10px] text-[#74869D]">
                  Trạng thái: Hoàn tất đối thoại NPC · Đã lưu vào SQLite WAL
                </div>
              </div>

              <button
                onClick={() => handleRestoreCheckpoint('quest_active')}
                className="px-3 py-1.5 bg-[#192434] hover:bg-[#1F334E] text-[#38BDF8] rounded-md font-semibold text-xs border border-[#2A3647] flex items-center gap-1.5 transition-colors cursor-pointer"
              >
                <RotateCcw size={12} />
                <span>Khôi phục phiên</span>
              </button>
            </div>

            {checkpointRestored && (
              <div className="p-2 rounded bg-[#133528] border border-[#3BC38D]/40 text-[#3BC38D] text-xs flex items-center gap-2 animate-fadeIn">
                <CheckCircle2 size={13} />
                <span>
                  Đã khôi phục trạng thái phiên chạy từ checkpoint "{checkpointRestored}"!
                </span>
              </div>
            )}
          </div>
        </div>

        {/* Right Column (6 cols): Step Inspector / Deep Debug View */}
        <div className="col-span-6 rounded-xl bg-[#16202F] border border-[#2A3647] p-4 flex flex-col shadow-sm overflow-y-auto space-y-4">
          <div className="border-b border-[#2A3647] pb-3">
            <div className="flex items-center justify-between">
              <span className="text-[10px] text-[#74869D] font-mono uppercase">
                CHI TIẾT BƯỚC: {activeEvent?.node_id}
              </span>
              <span className="text-[10px] px-1.5 rounded bg-[#133528] text-[#3BC38D] font-bold">
                VERIFIED
              </span>
            </div>
            <h3 className="text-sm font-bold text-[#EDF3FC] mt-0.5">
              Sự kiện: {activeEvent?.event_type}
            </h3>
            <span className="text-[11px] text-[#74869D]">
              Thời điểm: {new Date(activeEvent?.at || '').toLocaleTimeString()} · Job ID:{' '}
              {activeEvent?.job_id}
            </span>
          </div>

          {/* Safe Random Tap Inspector / Coordinates Breakdown */}
          <div className="p-3 rounded-lg bg-[#101722] border border-[#2A3647] space-y-2.5">
            <div className="flex items-center gap-2 text-xs font-bold text-[#38BDF8]">
              <MousePointerClick size={15} />
              <span>Phân tích Tọa độ Safe Random Tap Policy</span>
            </div>

            <div className="w-full h-32 bg-[#17202E] border border-[#2A3647] rounded flex items-center justify-center p-3 relative overflow-hidden">
              {/* Outer Bounding Box */}
              <div className="w-48 h-20 border border-dashed border-[#F27883] relative flex items-center justify-center rounded">
                <div className="absolute top-1 left-2 text-[8px] text-[#F27883] font-mono">
                  Bounds: [200, 400, 150, 60]
                </div>

                {/* Inner Safe Margin Rectangle */}
                <div className="w-36 h-12 border-2 border-[#38BDF8] bg-[#38BDF8]/15 rounded flex items-center justify-center relative">
                  <div className="w-2.5 h-2.5 rounded-full bg-[#FDE047] shadow-lg animate-ping absolute" />
                  <div className="w-2 h-2 rounded-full bg-[#FDE047] shadow-md" />
                  <span className="absolute bottom-1 right-2 text-[8px] text-[#38BDF8] font-mono">
                    (242.0, 428.0)
                  </span>
                </div>
              </div>
            </div>

            <div className="grid grid-cols-2 gap-2 text-[11px]">
              <div className="p-2 rounded bg-[#16202F] border border-[#2A3647]">
                <span className="text-[#74869D] block text-[10px]">Tọa độ điểm bấm:</span>
                <span className="font-mono text-[#38BDF8] font-bold">X: 242.0, Y: 428.0</span>
              </div>
              <div className="p-2 rounded bg-[#16202F] border border-[#2A3647]">
                <span className="text-[#74869D] block text-[10px]">Tỷ lệ co biên an toàn:</span>
                <span className="font-mono text-[#3BC38D] font-bold">18% (Tránh viền nút)</span>
              </div>
            </div>
          </div>

          {/* Raw Payload JSON Viewer */}
          <div className="space-y-1.5">
            <span className="text-xs font-bold text-[#EDF3FC] block">
              Dữ liệu JSON Payload chi tiết
            </span>
            <div className="p-3 rounded-lg bg-[#101722] border border-[#2A3647] font-mono text-[11px] text-[#A9B8CC] overflow-x-auto leading-relaxed">
              <pre>{JSON.stringify(JSON.parse(activeEvent?.payload_json || '{}'), null, 2)}</pre>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
