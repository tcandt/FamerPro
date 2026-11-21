import React, { useState } from 'react';
import {
  GripVertical,
  Play,
  ArrowUp,
  ArrowDown,
  Clock,
  Plus,
  Trash2,
  Smartphone,
} from 'lucide-react';
import { useDeviceStore } from '../../stores/deviceStore';
import { useWorkflowStore } from '../../stores/workflowStore';

interface TaskItem {
  id: string;
  title: string;
  category: string;
  priority: number;
  ready: boolean;
  reason?: string;
  estimateSec: number;
  enabled: boolean;
  status: 'idle' | 'running' | 'success' | 'failed' | 'waiting';
}

interface TaskTemplate {
  id: string;
  title: string;
  category: string;
  estimateSec: number;
  condition: string;
  isReadyByDefault: boolean;
}

export const TasksView: React.FC = () => {
  const { devices, selectedDeviceId, selectDevice } = useDeviceStore();
  const { setIsRunning } = useWorkflowStore();

  const taskCatalog: TaskTemplate[] = [
    {
      id: 'cat-01',
      title: 'Nhiệm vụ tân thủ chính tuyến',
      category: 'Chính Tuyến',
      estimateSec: 45,
      condition: 'Cần cấp độ 1-10',
      isReadyByDefault: true,
    },
    {
      id: 'cat-02',
      title: 'Nhận quà điểm danh 7 ngày',
      category: 'Phúc Lợi',
      estimateSec: 15,
      condition: 'Đã hoàn thành đăng nhập hôm nay',
      isReadyByDefault: true,
    },
    {
      id: 'cat-03',
      title: 'Thu hoạch mỏ tài nguyên bang hội',
      category: 'Phúc Lợi',
      estimateSec: 30,
      condition: 'Đã gia nhập bang hội',
      isReadyByDefault: true,
    },
    {
      id: 'cat-04',
      title: 'Vượt phó bản trang bị Cấp 1',
      category: 'Phó Bản',
      estimateSec: 120,
      condition: 'Chờ đạt cấp độ 15 & Thể lực > 20',
      isReadyByDefault: false,
    },
    {
      id: 'cat-05',
      title: 'Đấu trường PVP 5 lượt tự do',
      category: 'Đấu Trường',
      estimateSec: 90,
      condition: 'Vé khiêu chiến > 0',
      isReadyByDefault: true,
    },
    {
      id: 'cat-06',
      title: 'Dọn dẹp hòm đồ & bán trang bị trắng',
      category: 'Quản Trị',
      estimateSec: 25,
      condition: 'Sức chứa kho > 90%',
      isReadyByDefault: true,
    },
  ];

  const [selectedCatalogIds, setSelectedCatalogIds] = useState<string[]>([]);

  const [plannedTasks, setPlannedTasks] = useState<TaskItem[]>([
    {
      id: 'plan-1',
      title: 'Nhiệm vụ tân thủ chính tuyến',
      category: 'Chính Tuyến',
      priority: 1,
      ready: true,
      estimateSec: 45,
      enabled: true,
      status: 'idle',
    },
    {
      id: 'plan-2',
      title: 'Nhận quà điểm danh 7 ngày',
      category: 'Phúc Lợi',
      priority: 2,
      ready: true,
      estimateSec: 15,
      enabled: true,
      status: 'idle',
    },
    {
      id: 'plan-3',
      title: 'Thu hoạch mỏ tài nguyên bang hội',
      category: 'Phúc Lợi',
      priority: 3,
      ready: true,
      estimateSec: 30,
      enabled: true,
      status: 'idle',
    },
    {
      id: 'plan-4',
      title: 'Vượt phó bản trang bị Cấp 1',
      category: 'Phó Bản',
      priority: 4,
      ready: false,
      reason: 'Chờ đạt cấp độ 15',
      estimateSec: 120,
      enabled: true,
      status: 'waiting',
    },
  ]);

  const toggleCatalogSelect = (id: string) => {
    if (selectedCatalogIds.includes(id)) {
      setSelectedCatalogIds(selectedCatalogIds.filter((item) => item !== id));
    } else {
      setSelectedCatalogIds([...selectedCatalogIds, id]);
    }
  };

  const handleAddSelectedToPlan = () => {
    const newItems: TaskItem[] = selectedCatalogIds
      .map((id) => taskCatalog.find((c) => c.id === id))
      .filter(Boolean)
      .map((tpl, idx) => ({
        id: `plan-${Date.now()}-${idx}`,
        title: tpl!.title,
        category: tpl!.category,
        priority: plannedTasks.length + idx + 1,
        ready: tpl!.isReadyByDefault,
        reason: tpl!.isReadyByDefault ? undefined : tpl!.condition,
        estimateSec: tpl!.estimateSec,
        enabled: true,
        status: tpl!.isReadyByDefault ? 'idle' : 'waiting',
      }));

    setPlannedTasks([...plannedTasks, ...newItems]);
    setSelectedCatalogIds([]);
  };

  const handleAddSingleToPlan = (tpl: TaskTemplate) => {
    const newItem: TaskItem = {
      id: `plan-${Date.now()}`,
      title: tpl.title,
      category: tpl.category,
      priority: plannedTasks.length + 1,
      ready: tpl.isReadyByDefault,
      reason: tpl.isReadyByDefault ? undefined : tpl.condition,
      estimateSec: tpl.estimateSec,
      enabled: true,
      status: tpl.isReadyByDefault ? 'idle' : 'waiting',
    };
    setPlannedTasks([...plannedTasks, newItem]);
  };

  const moveUp = (idx: number) => {
    if (idx === 0) return;
    const next = [...plannedTasks];
    const temp = next[idx - 1];
    next[idx - 1] = next[idx];
    next[idx] = temp;
    // Re-index priorities
    next.forEach((item, i) => (item.priority = i + 1));
    setPlannedTasks(next);
  };

  const moveDown = (idx: number) => {
    if (idx === plannedTasks.length - 1) return;
    const next = [...plannedTasks];
    const temp = next[idx + 1];
    next[idx + 1] = next[idx];
    next[idx] = temp;
    // Re-index priorities
    next.forEach((item, i) => (item.priority = i + 1));
    setPlannedTasks(next);
  };

  const removeTask = (id: string) => {
    const next = plannedTasks.filter((t) => t.id !== id);
    next.forEach((item, i) => (item.priority = i + 1));
    setPlannedTasks(next);
  };

  const toggleTaskEnabled = (id: string) => {
    setPlannedTasks(
      plannedTasks.map((t) => (t.id === id ? { ...t, enabled: !t.enabled } : t))
    );
  };

  const enabledTasks = plannedTasks.filter((t) => t.enabled);
  const totalEstimateSec = enabledTasks.reduce((sum, t) => sum + t.estimateSec, 0);

  const [isExecuting, setIsExecuting] = useState(false);

  const handleExecutePlan = () => {
    setIsExecuting(true);
    setIsRunning(true);
    setTimeout(() => {
      setIsExecuting(false);
      setIsRunning(false);
    }, 2500);
  };

  return (
    <div className="flex-1 flex flex-col p-6 space-y-5 overflow-y-auto bg-[#101722] text-[#EDF3FC]">
      {/* Top Header */}
      <div className="flex items-center justify-between">
        <div>
          <div className="flex items-center gap-2">
            <h1 className="text-xl font-bold text-[#EDF3FC] tracking-tight">
              Bộ Nhiệm vụ (Task Library & Planner)
            </h1>
            <span className="px-2 py-0.5 rounded text-[10px] bg-[#173355] text-[#38BDF8] font-mono font-semibold border border-[#2A3647]">
              fp-scheduler v1.0
            </span>
          </div>
          <p className="text-xs text-[#74869D] mt-1">
            Chọn nhiệm vụ từ thư viện, thẩm định điều kiện trước và lập kế hoạch thực thi tuần tự trên từng thiết bị.
          </p>
        </div>

        <div className="flex items-center gap-3">
          <div className="flex items-center gap-1.5 bg-[#16202F] border border-[#2A3647] rounded-md px-2.5 py-1 text-xs text-[#EDF3FC]">
            <Smartphone size={13} className="text-[#1878F5]" />
            <span className="text-[#74869D] text-[11px]">Thiết bị đích:</span>
            <select
              value={selectedDeviceId || ''}
              onChange={(e) => selectDevice(e.target.value)}
              className="bg-transparent text-[#EDF3FC] outline-none cursor-pointer text-xs font-semibold"
            >
              {devices.map((d) => (
                <option key={d.id} value={d.id} className="bg-[#161E29] text-[#EDF3FC]">
                  {d.alias} ({d.adb_serial})
                </option>
              ))}
            </select>
          </div>

          <button
            onClick={handleExecutePlan}
            disabled={isExecuting || enabledTasks.length === 0}
            className="px-4 py-1.5 bg-[#1878F5] hover:bg-[#2F8CFF] text-white text-xs font-semibold rounded-md flex items-center gap-1.5 shadow-sm transition-colors cursor-pointer disabled:opacity-50"
          >
            <Play size={13} fill="currentColor" />
            <span>{isExecuting ? 'Đang thực thi kế hoạch...' : 'Bắt đầu chạy kế hoạch'}</span>
          </button>
        </div>
      </div>

      {/* 2-Column Split: Left = Catalog (Library), Right = Plan Queue */}
      <div className="grid grid-cols-12 gap-6 flex-1 min-h-0">
        {/* Left Column (5 cols): Task Catalog */}
        <div className="col-span-5 rounded-xl bg-[#16202F] border border-[#2A3647] p-4 flex flex-col shadow-sm">
          <div className="flex items-center justify-between pb-3 border-b border-[#2A3647]">
            <div>
              <h3 className="text-xs font-bold text-[#EDF3FC]">Thư viện Nhiệm vụ Mẫu</h3>
              <span className="text-[11px] text-[#74869D]">
                Danh mục có thể gán vào kế hoạch
              </span>
            </div>
            {selectedCatalogIds.length > 0 && (
              <button
                onClick={handleAddSelectedToPlan}
                className="px-2.5 py-1 bg-[#1878F5] hover:bg-[#2F8CFF] text-white rounded text-xs font-semibold flex items-center gap-1 transition-colors cursor-pointer"
              >
                <Plus size={12} />
                <span>Thêm ({selectedCatalogIds.length})</span>
              </button>
            )}
          </div>

          <div className="flex-1 overflow-y-auto space-y-2 mt-3 pr-1">
            {taskCatalog.map((tpl) => {
              const isSelected = selectedCatalogIds.includes(tpl.id);
              return (
                <div
                  key={tpl.id}
                  onClick={() => toggleCatalogSelect(tpl.id)}
                  className={`p-3 rounded-lg border transition-all cursor-pointer text-xs flex items-start justify-between ${
                    isSelected
                      ? 'bg-[#172E49] border-[#38BDF8]'
                      : 'bg-[#101722] border-[#2A3647] hover:border-[#3D4F67]'
                  }`}
                >
                  <div className="flex items-start gap-2.5 min-w-0">
                    <input
                      type="checkbox"
                      checked={isSelected}
                      onChange={() => {}}
                      className="accent-[#1878F5] mt-0.5 cursor-pointer"
                    />
                    <div className="min-w-0">
                      <div className="font-bold text-[#EDF3FC] text-[12px] truncate">
                        {tpl.title}
                      </div>
                      <div className="text-[10px] text-[#74869D] mt-0.5 flex items-center gap-2">
                        <span className="px-1.5 py-0.2 rounded bg-[#16202F] text-[#7BA6D6] font-medium border border-[#2A3647]">
                          {tpl.category}
                        </span>
                        <span>Ước tính: {tpl.estimateSec}s</span>
                      </div>
                      <div className="text-[10px] text-[#A9B8CC] mt-1 flex items-center gap-1">
                        <span className="text-[#74869D]">Điều kiện:</span>
                        <span>{tpl.condition}</span>
                      </div>
                    </div>
                  </div>

                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      handleAddSingleToPlan(tpl);
                    }}
                    className="p-1 text-[#74869D] hover:text-[#38BDF8] hover:bg-[#16202F] rounded transition-colors shrink-0 ml-1"
                    title="Thêm vào kế hoạch"
                  >
                    <Plus size={14} />
                  </button>
                </div>
              );
            })}
          </div>
        </div>

        {/* Right Column (7 cols): Execution Plan Queue */}
        <div className="col-span-7 rounded-xl bg-[#16202F] border border-[#2A3647] p-4 flex flex-col shadow-sm">
          <div className="flex items-center justify-between pb-3 border-b border-[#2A3647]">
            <div>
              <h3 className="text-xs font-bold text-[#EDF3FC]">
                Kế hoạch Thực thi Tuần tự (Hàng đợi {plannedTasks.length} tác vụ)
              </h3>
              <span className="text-[11px] text-[#74869D]">
                fp-scheduler ưu tiên tác vụ đủ điều kiện, chạy đơn luồng trên thiết bị
              </span>
            </div>

            <div className="flex items-center gap-2 text-xs">
              <div className="px-2.5 py-1 rounded bg-[#101722] border border-[#2A3647] text-[#3BC38D] font-mono font-semibold flex items-center gap-1">
                <Clock size={12} />
                <span>
                  Tổng: {Math.floor(totalEstimateSec / 60)}m {totalEstimateSec % 60}s
                </span>
              </div>
            </div>
          </div>

          <div className="flex-1 overflow-y-auto space-y-2 mt-3 pr-1">
            {plannedTasks.length === 0 ? (
              <div className="p-8 text-center text-[#74869D] text-xs italic bg-[#101722] rounded-lg border border-[#2A3647]">
                Kế hoạch đang trống. Hãy chọn nhiệm vụ từ Thư viện bên trái để bắt đầu lập lịch.
              </div>
            ) : (
              plannedTasks.map((task, idx) => (
                <div
                  key={task.id}
                  className={`p-3 rounded-lg border flex items-center justify-between text-xs transition-all ${
                    task.enabled
                      ? 'bg-[#101722] border-[#2A3647]'
                      : 'bg-[#101722]/40 border-[#2A3647]/50 opacity-50'
                  }`}
                >
                  <div className="flex items-center gap-3 min-w-0">
                    <GripVertical size={16} className="text-[#5A6D85] cursor-grab shrink-0" />

                    <span className="w-6 h-6 rounded-md bg-[#16202F] text-[#38BDF8] flex items-center justify-center font-bold font-mono text-[11px] shrink-0 border border-[#2A3647]">
                      #{idx + 1}
                    </span>

                    <input
                      type="checkbox"
                      checked={task.enabled}
                      onChange={() => toggleTaskEnabled(task.id)}
                      className="accent-[#1878F5] cursor-pointer"
                      title={task.enabled ? 'Tắt nhiệm vụ này' : 'Bật nhiệm vụ này'}
                    />

                    <div className="min-w-0">
                      <div className="font-bold text-[#EDF3FC] text-[13px] truncate">
                        {task.title}
                      </div>
                      <div className="text-[10px] text-[#74869D] mt-0.5 flex items-center gap-2">
                        <span className="font-mono text-[#7BA6D6]">
                          {task.estimateSec}s
                        </span>
                        <span>·</span>
                        <span className="text-[#A9B8CC]">{task.category}</span>
                      </div>
                    </div>
                  </div>

                  <div className="flex items-center gap-2 shrink-0">
                    <span
                      className={`px-2 py-0.5 rounded text-[10px] font-bold border ${
                        task.ready
                          ? 'bg-[#133528] text-[#3BC38D] border-[#3BC38D]/40'
                          : 'bg-[#3B2D17] text-[#E7B15B] border-[#E7B15B]/40'
                      }`}
                    >
                      {task.ready ? 'Đủ điều kiện' : task.reason || 'Chờ điều kiện'}
                    </span>

                    <div className="flex items-center gap-0.5 border-l border-[#2A3647] pl-1.5 ml-1">
                      <button
                        onClick={() => moveUp(idx)}
                        disabled={idx === 0}
                        className="p-1 hover:bg-[#202B3B] text-[#74869D] hover:text-white rounded disabled:opacity-20 transition-colors"
                        title="Tăng thứ tự ưu tiên"
                      >
                        <ArrowUp size={13} />
                      </button>
                      <button
                        onClick={() => moveDown(idx)}
                        disabled={idx === plannedTasks.length - 1}
                        className="p-1 hover:bg-[#202B3B] text-[#74869D] hover:text-white rounded disabled:opacity-20 transition-colors"
                        title="Giảm thứ tự ưu tiên"
                      >
                        <ArrowDown size={13} />
                      </button>
                      <button
                        onClick={() => removeTask(task.id)}
                        className="p-1 hover:bg-[#202B3B] text-[#74869D] hover:text-[#F27883] rounded transition-colors ml-1"
                        title="Xóa khỏi hàng đợi"
                      >
                        <Trash2 size={13} />
                      </button>
                    </div>
                  </div>
                </div>
              ))
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
