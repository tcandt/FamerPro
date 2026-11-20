import React, { useState } from 'react';
import { GripVertical, Play, ArrowUp, ArrowDown, CheckCircle2, Clock } from 'lucide-react';

export const TasksView: React.FC = () => {
  const [tasks, setTasks] = useState([
    { id: 't1', title: 'Nhiệm vụ tân thủ chính tuyến', priority: 1, ready: true, estimateMs: 45000 },
    { id: 't2', title: 'Nhận quà điểm danh 7 ngày', priority: 2, ready: true, estimateMs: 12000 },
    { id: 't3', title: 'Vượt phó bản trang bị cấp 1', priority: 3, ready: false, reason: 'Chờ đạt cấp độ 15', estimateMs: 180000 },
  ]);

  const moveUp = (idx: number) => {
    if (idx === 0) return;
    const next = [...tasks];
    const temp = next[idx - 1];
    next[idx - 1] = next[idx];
    next[idx] = temp;
    setTasks(next);
  };

  const moveDown = (idx: number) => {
    if (idx === tasks.length - 1) return;
    const next = [...tasks];
    const temp = next[idx + 1];
    next[idx + 1] = next[idx];
    next[idx] = temp;
    setTasks(next);
  };

  return (
    <div className="flex-1 overflow-y-auto p-6 space-y-6 bg-[#101722] text-[#EDF3FC]">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold text-[#EDF3FC]">Bộ Nhiệm vụ (Task Planner)</h1>
          <p className="text-xs text-[#74869D] mt-0.5">
            Sắp xếp thứ tự ưu tiên và điều kiện thực thi nhiệm vụ theo hàng đợi đơn luồng trên từng thiết bị.
          </p>
        </div>
        <button className="px-4 py-2 bg-[#1878F5] hover:bg-[#2F8CFF] text-white text-xs font-semibold rounded flex items-center gap-1.5 shadow-sm">
          <Play size={13} fill="currentColor" />
          <span>Bắt đầu chạy kế hoạch</span>
        </button>
      </div>

      <div className="grid grid-cols-12 gap-6">
        <div className="col-span-8 rounded-[9px] bg-[#192230] border border-[#2A3647] p-4 space-y-3">
          <h3 className="text-xs font-semibold text-[#EDF3FC]">Tập nhiệm vụ đã chọn (Selected Task Set)</h3>
          <div className="space-y-2">
            {tasks.map((task, idx) => (
              <div
                key={task.id}
                className="p-3 bg-[#161E29] border border-[#2A3647] rounded flex items-center justify-between text-xs"
              >
                <div className="flex items-center gap-3">
                  <GripVertical size={16} className="text-[#74869D] cursor-grab" />
                  <span className="w-5 h-5 rounded bg-[#202B3B] text-[#7BA6D6] flex items-center justify-center font-bold text-[10px]">
                    {idx + 1}
                  </span>
                  <div>
                    <div className="font-semibold text-[#EDF3FC]">{task.title}</div>
                    <div className="text-[11px] text-[#74869D]">
                      Ước tính: {task.estimateMs / 1000}s · Độ ưu tiên {task.priority}
                    </div>
                  </div>
                </div>

                <div className="flex items-center gap-2">
                  <span
                    className={`px-2 py-0.5 rounded text-[10px] font-medium border ${
                      task.ready
                        ? 'bg-[#133528] text-[#3BC38D] border-[#3BC38D]/30'
                        : 'bg-[#3B2D17] text-[#E7B15B] border-[#E7B15B]/30'
                    }`}
                  >
                    {task.ready ? 'Đủ điều kiện' : task.reason}
                  </span>
                  <button
                    onClick={() => moveUp(idx)}
                    disabled={idx === 0}
                    className="p-1 hover:bg-[#202B3B] text-[#74869D] hover:text-white rounded disabled:opacity-30"
                  >
                    <ArrowUp size={13} />
                  </button>
                  <button
                    onClick={() => moveDown(idx)}
                    disabled={idx === tasks.length - 1}
                    className="p-1 hover:bg-[#202B3B] text-[#74869D] hover:text-white rounded disabled:opacity-30"
                  >
                    <ArrowDown size={13} />
                  </button>
                </div>
              </div>
            ))}
          </div>
        </div>

        <div className="col-span-4 rounded-[9px] bg-[#192230] border border-[#2A3647] p-4 space-y-3">
          <h3 className="text-xs font-semibold text-[#EDF3FC]">Kế hoạch Điều phối (Execution Plan)</h3>
          <p className="text-[11px] text-[#74869D] leading-relaxed">
            Bộ điều phối Rust (fp-scheduler) đánh giá điều kiện trước: các nhiệm vụ đủ điều kiện sẽ được xếp lịch trước, chạy tuần tự trên thiết bị.
          </p>

          <div className="p-3 bg-[#101722] rounded border border-[#2A3647] space-y-2 text-xs">
            <div className="flex items-center gap-2 text-[#3BC38D]">
              <CheckCircle2 size={13} />
              <span>1. Chạy 'Nhiệm vụ tân thủ'</span>
            </div>
            <div className="flex items-center gap-2 text-[#3BC38D]">
              <CheckCircle2 size={13} />
              <span>2. Chạy 'Nhận quà điểm danh'</span>
            </div>
            <div className="flex items-center gap-2 text-[#E7B15B]">
              <Clock size={13} />
              <span>3. Bỏ qua 'Vượt phó bản' (Chờ điều kiện)</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
