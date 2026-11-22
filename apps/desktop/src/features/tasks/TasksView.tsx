import React, { useMemo, useState } from 'react';
import {
  CheckSquare,
  Cpu,
  Edit3,
  Layers,
  Play,
  Plus,
  RefreshCw,
  ShieldCheck,
  Trash2,
  Workflow,
  X,
} from 'lucide-react';
import { useProjectStore } from '../../stores/projectStore';
import { useDeviceStore } from '../../stores/deviceStore';
import { useWorkflowStore } from '../../stores/workflowStore';
import { api } from '../../lib/api';
import type {
  MiniTaskDef,
  ProjectModuleRecord,
  ProjectTaskRecord,
  RunWorkflowExecutionSummary,
  TaskCategoryDef,
} from '../../types';

interface StandardModuleMeta {
  moduleId: string;
  label: string;
  description: string;
  required?: boolean;
}

const STANDARD_MODULES: StandardModuleMeta[] = [
  {
    moduleId: 'mod.adb_core',
    label: 'ADB Core Runner',
    description: 'Điều phối adb.exe argument-vector, tap/swipe an toàn và chụp ảnh màn hình.',
    required: true,
  },
  {
    moduleId: 'mod.vision_ocr',
    label: 'Vision & ROI Matcher',
    description: 'So khớp template PNG theo tỷ lệ màn hình, chuẩn hóa bounding box & OCR.',
  },
  {
    moduleId: 'mod.account_vault',
    label: 'Account Vault & DPAPI',
    description: 'Quản lý danh tính tài khoản theo dự án và tham chiếu bí mật Windows DPAPI.',
  },
  {
    moduleId: 'mod.proxy_network',
    label: 'Proxy Probe & Gateway',
    description: 'Kiểm tra kết nối TCP / SOCKS5 / HTTP CONNECT (không giả lập VPN toàn hệ thống).',
  },
  {
    moduleId: 'mod.adb_port_forward',
    label: 'ADB Socket Forwarding',
    description: 'Quản lý quy tắc adb forward / adb reverse riêng biệt cho thiết bị mục tiêu.',
  },
  {
    moduleId: 'mod.emulator_optimizer',
    label: 'Emulator & DPI Helper',
    description: 'Chuẩn hóa độ phân giải mục tiêu và cấu hình giả lập khi chạy Mini Tool.',
  },
];

interface TaskFormState {
  id: string;
  title: string;
  workflowId: string;
  categoryId: string;
  categoryShortTag: string;
  categoryName: string;
  categoryColorHex: string;
  priority: number;
  estimateSec: number;
  defaultEnabled: boolean;
  settingsSchemaJson: string;
}

const emptyTaskForm = (defaultWorkflowId: string, nextPriority: number): TaskFormState => ({
  id: `task-${Date.now().toString(36)}`,
  title: '',
  workflowId: defaultWorkflowId,
  categoryId: 'cat.general',
  categoryShortTag: 'NV',
  categoryName: 'Nhiệm vụ',
  categoryColorHex: '#38BDF8',
  priority: nextPriority,
  estimateSec: 45,
  defaultEnabled: true,
  settingsSchemaJson: '[]',
});

export const TasksView: React.FC = () => {
  const {
    activeProjectId,
    activeProjectDetail,
    refreshActiveProject,
  } = useProjectStore();
  const { devices, selectedDeviceId } = useDeviceStore();
  const { setRunEvents, setActiveJobState, setIsRunning } = useWorkflowStore();

  const [editingTask, setEditingTask] = useState<TaskFormState | null>(null);
  const [isNewTask, setIsNewTask] = useState(false);
  const [busyKey, setBusyKey] = useState<string | null>(null);
  const [feedback, setFeedback] = useState<{ kind: 'ok' | 'err'; text: string } | null>(null);
  const [lastTaskRun, setLastTaskRun] = useState<RunWorkflowExecutionSummary | null>(null);

  const project = activeProjectDetail?.project ?? null;
  const tasks: ProjectTaskRecord[] = useMemo(
    () =>
      [...(activeProjectDetail?.tasks || [])].sort(
        (a, b) => a.priority - b.priority || a.title.localeCompare(b.title)
      ),
    [activeProjectDetail?.tasks]
  );
  const workflows = activeProjectDetail?.workflows || [];
  const modules = activeProjectDetail?.modules || [];

  const mergedModules = useMemo(() => {
    const byId = new Map<string, ProjectModuleRecord>();
    for (const m of modules) {
      byId.set(m.moduleId, m);
    }
    const list: Array<StandardModuleMeta & { record?: ProjectModuleRecord }> = STANDARD_MODULES.map(
      (meta) => ({
        ...meta,
        record: byId.get(meta.moduleId),
      })
    );
    for (const m of modules) {
      if (!STANDARD_MODULES.some((s) => s.moduleId === m.moduleId)) {
        list.push({
          moduleId: m.moduleId,
          label: m.moduleId,
          description: 'Module tùy chỉnh đã lưu trong SQLite dự án.',
          record: m,
        });
      }
    }
    return list;
  }, [modules]);

  const workflowNameById = useMemo(() => {
    const map = new Map<string, string>();
    for (const wf of workflows) {
      map.set(wf.id, wf.name);
    }
    return map;
  }, [workflows]);

  const openCreateTask = () => {
    const defaultWf = workflows[0]?.id || '';
    setEditingTask(emptyTaskForm(defaultWf, tasks.length + 1));
    setIsNewTask(true);
    setFeedback(null);
  };

  const openEditTask = (t: ProjectTaskRecord) => {
    setEditingTask({
      id: t.id,
      title: t.title,
      workflowId: t.workflowId,
      categoryId: t.categoryId,
      categoryShortTag: t.categoryShortTag,
      categoryName: t.categoryName,
      categoryColorHex: t.categoryColorHex,
      priority: t.priority,
      estimateSec: t.estimateSec,
      defaultEnabled: t.defaultEnabled,
      settingsSchemaJson: t.settingsSchemaJson || '[]',
    });
    setIsNewTask(false);
    setFeedback(null);
  };

  const handleSaveTask = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!activeProjectId || !editingTask) return;
    if (!editingTask.title.trim()) {
      setFeedback({ kind: 'err', text: 'Tên Task không được để trống.' });
      return;
    }
    if (!editingTask.workflowId) {
      setFeedback({
        kind: 'err',
        text: 'Cần chọn một Workflow thuộc Project trước khi lưu Task.',
      });
      return;
    }

    let parsedSettings: any[] = [];
    try {
      const candidate = JSON.parse(editingTask.settingsSchemaJson || '[]');
      if (!Array.isArray(candidate)) {
        throw new Error('settingsSchemaJson phải là mảng JSON');
      }
      parsedSettings = candidate;
    } catch (err: any) {
      setFeedback({ kind: 'err', text: `Settings JSON không hợp lệ: ${err?.message || err}` });
      return;
    }

    setBusyKey(`save-task:${editingTask.id}`);
    setFeedback(null);
    try {
      const taskDef: MiniTaskDef = {
        id: editingTask.id.trim(),
        title: editingTask.title.trim(),
        categoryId: editingTask.categoryId.trim() || 'cat.general',
        workflowId: editingTask.workflowId,
        defaultEnabled: editingTask.defaultEnabled,
        priority: Math.max(1, Number(editingTask.priority) || 1),
        estimateSec: Math.max(1, Number(editingTask.estimateSec) || 30),
        settingsSchema: parsedSettings,
      };
      const categoryMeta: TaskCategoryDef = {
        id: taskDef.categoryId,
        shortTag: (editingTask.categoryShortTag.trim() || 'NV').slice(0, 6).toUpperCase(),
        displayName: editingTask.categoryName.trim() || 'Nhiệm vụ',
        colorHex: editingTask.categoryColorHex || '#38BDF8',
      };
      await api.studioUpsertProjectTask(activeProjectId, taskDef, categoryMeta);
      await refreshActiveProject();
      setEditingTask(null);
      setFeedback({
        kind: 'ok',
        text: `Đã lưu Task '${taskDef.title}' gắn với Workflow '${
          workflowNameById.get(taskDef.workflowId) || taskDef.workflowId
        }' vào SQLite.`,
      });
    } catch (err: any) {
      setFeedback({ kind: 'err', text: `Lỗi lưu Task: ${err?.message || String(err)}` });
    } finally {
      setBusyKey(null);
    }
  };

  const handleDeleteTask = async (task: ProjectTaskRecord) => {
    if (!activeProjectId) return;
    setBusyKey(`del-task:${task.id}`);
    setFeedback(null);
    try {
      await api.studioDeleteProjectTask(activeProjectId, task.id);
      await refreshActiveProject();
      setFeedback({ kind: 'ok', text: `Đã xóa Task '${task.title}' khỏi Project.` });
    } catch (err: any) {
      setFeedback({ kind: 'err', text: `Lỗi xóa Task: ${err?.message || String(err)}` });
    } finally {
      setBusyKey(null);
    }
  };

  const handleToggleModule = async (
    moduleId: string,
    nextPackaged: boolean,
    nextEnabled: boolean,
    existingConfigJson?: string
  ) => {
    if (!activeProjectId) return;
    setBusyKey(`mod:${moduleId}`);
    setFeedback(null);
    try {
      await api.studioUpsertProjectModule(
        activeProjectId,
        moduleId,
        nextPackaged,
        nextEnabled,
        existingConfigJson || '{}'
      );
      await refreshActiveProject();
      setFeedback({
        kind: 'ok',
        text: `Đã cập nhật Module '${moduleId}' (Packaged=${nextPackaged}, Default=${nextEnabled}).`,
      });
    } catch (err: any) {
      setFeedback({ kind: 'err', text: `Lỗi cập nhật Module: ${err?.message || String(err)}` });
    } finally {
      setBusyKey(null);
    }
  };

  const handleRunTaskTestKit = async (task: ProjectTaskRecord) => {
    if (!activeProjectId) return;
    const onlineDevice = devices.find((d) => d.id === selectedDeviceId) || devices[0];
    const targetDeviceId = onlineDevice?.id || 'testkit-emulator';
    setBusyKey(`run-task:${task.id}`);
    setIsRunning(true);
    setFeedback(null);
    try {
      const summary = await api.studioRunProjectWorkflow({
        projectId: activeProjectId,
        workflowId: task.workflowId,
        taskId: task.id,
        deviceId: targetDeviceId,
        priority: task.priority,
        leaseTtlSecs: 120,
        workflowTimeoutMs: 60000,
      });
      setLastTaskRun(summary);
      const detail = await api.studioGetJobResumeDetail(summary.jobId);
      setActiveJobState(
        summary.jobId,
        detail,
        summary,
        `Chạy thử Task '${task.title}' qua ${summary.adapterKind} (${summary.state})`
      );
      if (detail?.stepEvents) {
        setRunEvents(detail.stepEvents);
      }
      await refreshActiveProject();
      setFeedback({
        kind: 'ok',
        text: `Đã chạy thử Task '${task.title}' qua ${summary.adapterKind} (Job ${summary.jobId} · ${summary.state}).`,
      });
    } catch (err: any) {
      setFeedback({ kind: 'err', text: `Chạy thử Task thất bại: ${err?.message || String(err)}` });
    } finally {
      setIsRunning(false);
      setBusyKey(null);
    }
  };

  if (!activeProjectId || !project) {
    return (
      <div className="flex-1 flex items-center justify-center bg-[#101722] p-6">
        <div className="fp-panel p-6 max-w-md text-center space-y-2">
          <Layers size={28} className="mx-auto text-[#4B9CFF]" />
          <h2 className="text-base font-bold text-[#EDF3FC]">Chưa chọn Game Project</h2>
          <p className="text-xs text-[#74869D]">
            Chọn hoặc tạo một Game Project trên Header để cấu hình danh mục Task và ma trận Module
            đóng gói cho Mini Tool.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex-1 flex flex-col p-5 space-y-4 overflow-y-auto bg-[#101722] text-[#EDF3FC]">
      {/* Header */}
      <div className="fp-panel p-4 flex flex-col lg:flex-row lg:items-center lg:justify-between gap-3">
        <div className="space-y-1">
          <div className="flex items-center gap-2 flex-wrap">
            <span className="px-2 py-0.5 rounded text-[10px] font-mono font-bold bg-[#173355] text-[#38BDF8] border border-[#2A3647]">
              PROJECT TASKS &amp; MODULE MATRIX
            </span>
            <span className="text-xs font-mono text-[#A9B8CC]">{project.slug}</span>
            <span className="text-[11px] text-[#74869D]">
              · {tasks.length} tasks · {workflows.length} workflows ·{' '}
              {modules.filter((m) => m.packagedInRelease).length} modules đóng gói
            </span>
          </div>
          <h1 className="text-lg font-bold text-[#EDF3FC]">
            Quản lý Task &amp; Module cho {project.name}
          </h1>
          <p className="text-xs text-[#74869D]">
            Mỗi Task trong Mini Tool ánh xạ trực tiếp tới một Workflow của dự án trong SQLite. Module
            Matrix quyết định thành phần nào được đóng gói vào bản phát hành.
          </p>
        </div>

        <div className="flex items-center gap-2 shrink-0">
          <button
            onClick={() => refreshActiveProject()}
            className="px-3 py-1.5 rounded-md bg-[#161E29] hover:bg-[#202B3B] border border-[#2A3647] text-xs text-[#A9B8CC] flex items-center gap-1.5 cursor-pointer"
          >
            <RefreshCw size={13} />
            <span>Đồng bộ SQLite</span>
          </button>
          <button
            onClick={openCreateTask}
            disabled={workflows.length === 0}
            className="px-3.5 py-1.5 rounded-md bg-[#1878F5] hover:bg-[#2F8CFF] disabled:opacity-50 text-white text-xs font-semibold flex items-center gap-1.5 cursor-pointer shadow-sm"
          >
            <Plus size={14} />
            <span>Thêm Task mới</span>
          </button>
        </div>
      </div>

      {feedback && (
        <div
          className={`px-3.5 py-2.5 rounded-lg border text-xs flex items-center justify-between ${
            feedback.kind === 'ok'
              ? 'bg-[#133528] border-[#3BC38D]/40 text-[#3BC38D]'
              : 'bg-[#381E24] border-[#F27883]/40 text-[#F27883]'
          }`}
        >
          <span>{feedback.text}</span>
          <button
            onClick={() => setFeedback(null)}
            className="opacity-70 hover:opacity-100 cursor-pointer"
          >
            <X size={13} />
          </button>
        </div>
      )}

      {lastTaskRun && (
        <div className="p-3 rounded-lg bg-[#16202F] border border-[#2A3647] flex flex-wrap items-center justify-between gap-2 text-xs">
          <div className="flex items-center gap-2">
            <span
              className={
                lastTaskRun.adapterKind === 'synthetic' ||
                lastTaskRun.adapterKind.toLowerCase().includes('testkit') ||
                lastTaskRun.deviceId.toLowerCase().includes('testkit')
                  ? 'fp-badge-testkit'
                  : 'fp-badge-live'
              }
            >
              {lastTaskRun.adapterKind === 'synthetic' ||
              lastTaskRun.adapterKind.toLowerCase().includes('testkit') ||
              lastTaskRun.deviceId.toLowerCase().includes('testkit')
                ? 'TESTKIT'
                : 'LIVE'}
            </span>
            <span className="font-mono text-[#EDF3FC] font-semibold">Job {lastTaskRun.jobId}</span>
            <span className="text-[#A9B8CC]">
              State: <strong className="text-[#38BDF8]">{lastTaskRun.state}</strong> ·{' '}
              {lastTaskRun.eventsRecorded} events · Fencing gen #{lastTaskRun.fencingGeneration}
            </span>
          </div>
          {lastTaskRun.blockReason && (
            <span className="font-mono text-[11px] text-[#E7B15B]">
              Block: {lastTaskRun.blockReason}
            </span>
          )}
        </div>
      )}

      {/* Main 2-column layout */}
      <div className="grid grid-cols-12 gap-5 items-start">
        {/* Left 7 cols: Project Tasks */}
        <div className="col-span-12 xl:col-span-7 fp-panel p-4 space-y-3">
          <div className="flex items-center justify-between pb-2.5 border-b border-[#2A3647]">
            <div className="flex items-center gap-2">
              <CheckSquare size={15} className="text-[#4B9CFF]" />
              <h2 className="text-sm font-bold text-[#EDF3FC]">
                Danh sách Task thuộc Project ({tasks.length})
              </h2>
            </div>
            {workflows.length === 0 && (
              <span className="text-[11px] text-[#E7B15B]">
                Cần tạo ít nhất 1 Workflow trong Workflow Studio trước khi thêm Task.
              </span>
            )}
          </div>

          {tasks.length === 0 ? (
            <div className="p-6 rounded-lg bg-[#101722] border border-dashed border-[#2A3647] text-center space-y-2">
              <Workflow size={24} className="mx-auto text-[#74869D]" />
              <div className="text-xs font-semibold text-[#EDF3FC]">
                Project chưa có Task nào trong bảng <code>project_tasks</code>
              </div>
              <p className="text-[11px] text-[#74869D] max-w-md mx-auto">
                Nhấn &quot;Thêm Task mới&quot; để gắn một Workflow trong dự án thành tác vụ hiển thị
                trên Mini Tool xuất ra.
              </p>
            </div>
          ) : (
            <div className="space-y-2">
              {tasks.map((task) => {
                const wfName = workflowNameById.get(task.workflowId) || task.workflowId;
                return (
                  <div
                    key={task.id}
                    className="p-3 rounded-lg bg-[#101722] border border-[#2A3647] hover:border-[#3D4F67] transition-colors flex flex-col sm:flex-row sm:items-center justify-between gap-3"
                  >
                    <div className="min-w-0 space-y-1">
                      <div className="flex items-center gap-2 flex-wrap">
                        <span className="font-mono text-[11px] px-1.5 py-0.5 rounded bg-[#16202F] border border-[#2A3647] text-[#A9B8CC]">
                          P{task.priority}
                        </span>
                        <span
                          className="px-1.5 py-0.5 rounded text-[10px] font-bold border"
                          style={{
                            color: task.categoryColorHex || '#38BDF8',
                            borderColor: `${task.categoryColorHex || '#38BDF8'}55`,
                            backgroundColor: `${task.categoryColorHex || '#38BDF8'}18`,
                          }}
                        >
                          [{task.categoryShortTag}] {task.categoryName}
                        </span>
                        <span className="text-xs font-bold text-[#EDF3FC] truncate">
                          {task.title}
                        </span>
                        {task.defaultEnabled ? (
                          <span className="text-[10px] px-1.5 py-0.5 rounded bg-[#133528] text-[#3BC38D] font-semibold">
                            Bật mặc định
                          </span>
                        ) : (
                          <span className="text-[10px] px-1.5 py-0.5 rounded bg-[#202B3B] text-[#74869D]">
                            Tắt mặc định
                          </span>
                        )}
                      </div>
                      <div className="text-[11px] text-[#74869D] flex items-center gap-3 flex-wrap font-mono">
                        <span>Workflow: {wfName}</span>
                        <span>· Ước tính: {task.estimateSec}s</span>
                        <span>· ID: {task.id}</span>
                      </div>
                    </div>

                    <div className="flex items-center gap-1.5 shrink-0">
                      <button
                        onClick={() => handleRunTaskTestKit(task)}
                        disabled={busyKey === `run-task:${task.id}`}
                        title="Chạy thử Workflow của Task qua TestKit"
                        className="px-2.5 py-1.5 rounded bg-[#133528] hover:bg-[#194735] border border-[#3BC38D]/40 text-[#3BC38D] text-[11px] font-semibold flex items-center gap-1 cursor-pointer"
                      >
                        <Play size={11} />
                        <span>Chạy thử</span>
                      </button>
                      <button
                        onClick={() => openEditTask(task)}
                        title="Chỉnh sửa Task"
                        className="p-1.5 rounded bg-[#16202F] hover:bg-[#202B3B] border border-[#2A3647] text-[#A9B8CC] hover:text-[#EDF3FC] cursor-pointer"
                      >
                        <Edit3 size={13} />
                      </button>
                      <button
                        onClick={() => handleDeleteTask(task)}
                        disabled={busyKey === `del-task:${task.id}`}
                        title="Xóa Task khỏi Project"
                        className="p-1.5 rounded bg-[#16202F] hover:bg-[#381E24] border border-[#2A3647] text-[#74869D] hover:text-[#F27883] cursor-pointer"
                      >
                        <Trash2 size={13} />
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>

        {/* Right 5 cols: Module Matrix */}
        <div className="col-span-12 xl:col-span-5 fp-panel p-4 space-y-3">
          <div className="flex items-center justify-between pb-2.5 border-b border-[#2A3647]">
            <div className="flex items-center gap-2">
              <Cpu size={15} className="text-[#B291F7]" />
              <h2 className="text-sm font-bold text-[#EDF3FC]">
                Ma trận Module Đóng gói ({mergedModules.length})
              </h2>
            </div>
            <span className="text-[10px] font-mono text-[#74869D]">project_modules</span>
          </div>

          <div className="space-y-2">
            {mergedModules.map((mod) => {
              const packaged = mod.record ? mod.record.packagedInRelease : Boolean(mod.required);
              const enabledDefault = mod.record
                ? mod.record.enabledByDefault
                : Boolean(mod.required);
              const isBusy = busyKey === `mod:${mod.moduleId}`;

              return (
                <div
                  key={mod.moduleId}
                  className="p-3 rounded-lg bg-[#101722] border border-[#2A3647] space-y-2"
                >
                  <div className="flex items-start justify-between gap-2">
                    <div className="min-w-0">
                      <div className="flex items-center gap-2 flex-wrap">
                        <span className="text-xs font-bold text-[#EDF3FC]">{mod.label}</span>
                        <span className="font-mono text-[10px] text-[#38BDF8]">{mod.moduleId}</span>
                        {mod.required && (
                          <span className="text-[10px] px-1.5 py-0.5 rounded bg-[#173355] text-[#4B9CFF] font-semibold">
                            Cốt lõi
                          </span>
                        )}
                      </div>
                      <p className="text-[11px] text-[#74869D] mt-0.5">{mod.description}</p>
                    </div>
                    {packaged && <ShieldCheck size={15} className="text-[#3BC38D] shrink-0 mt-0.5" />}
                  </div>

                  <div className="flex items-center gap-4 pt-1 border-t border-[#1D2838] text-[11px]">
                    <label className="flex items-center gap-1.5 cursor-pointer text-[#A9B8CC]">
                      <input
                        type="checkbox"
                        checked={packaged}
                        disabled={isBusy}
                        onChange={(e) =>
                          handleToggleModule(
                            mod.moduleId,
                            e.target.checked,
                            e.target.checked ? enabledDefault : false,
                            mod.record?.configJson
                          )
                        }
                        className="accent-[#1878F5]"
                      />
                      <span>Đóng gói vào Release</span>
                    </label>

                    <label className="flex items-center gap-1.5 cursor-pointer text-[#A9B8CC]">
                      <input
                        type="checkbox"
                        checked={enabledDefault}
                        disabled={isBusy || !packaged}
                        onChange={(e) =>
                          handleToggleModule(
                            mod.moduleId,
                            packaged,
                            e.target.checked,
                            mod.record?.configJson
                          )
                        }
                        className="accent-[#3BC38D]"
                      />
                      <span>Bật mặc định</span>
                    </label>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      </div>

      {/* Create / Edit Task Modal */}
      {editingTask && (
        <div className="fixed inset-0 z-50 bg-black/70 backdrop-blur-xs flex items-center justify-center p-4">
          <form
            onSubmit={handleSaveTask}
            className="w-full max-w-lg rounded-xl bg-[#16202F] border border-[#2A3647] shadow-2xl p-5 space-y-4 text-xs"
          >
            <div className="flex items-center justify-between border-b border-[#2A3647] pb-3">
              <h3 className="text-sm font-bold text-[#EDF3FC]">
                {isNewTask ? 'Thêm Task mới cho Project' : `Chỉnh sửa Task (${editingTask.id})`}
              </h3>
              <button
                type="button"
                onClick={() => setEditingTask(null)}
                className="text-[#74869D] hover:text-[#EDF3FC] cursor-pointer"
              >
                <X size={16} />
              </button>
            </div>

            <div className="grid grid-cols-2 gap-3">
              <div className="col-span-2">
                <label className="block text-[#A9B8CC] mb-1 font-semibold">Tên Task</label>
                <input
                  type="text"
                  required
                  value={editingTask.title}
                  onChange={(e) => setEditingTask({ ...editingTask, title: e.target.value })}
                  placeholder="Ví dụ: Vận tiêu hàng ngày / Thu hoạch tài nguyên"
                  className="w-full bg-[#101722] border border-[#2A3647] rounded px-3 py-1.5 text-[#EDF3FC] outline-none"
                />
              </div>

              <div>
                <label className="block text-[#A9B8CC] mb-1 font-semibold">Task ID</label>
                <input
                  type="text"
                  required
                  disabled={!isNewTask}
                  value={editingTask.id}
                  onChange={(e) => setEditingTask({ ...editingTask, id: e.target.value })}
                  className="w-full bg-[#101722] border border-[#2A3647] rounded px-3 py-1.5 text-[#EDF3FC] font-mono outline-none disabled:opacity-60"
                />
              </div>

              <div>
                <label className="block text-[#A9B8CC] mb-1 font-semibold">
                  Ánh xạ Workflow (Bắt buộc)
                </label>
                <select
                  value={editingTask.workflowId}
                  onChange={(e) => setEditingTask({ ...editingTask, workflowId: e.target.value })}
                  className="w-full bg-[#101722] border border-[#2A3647] rounded px-2.5 py-1.5 text-[#EDF3FC] outline-none"
                >
                  {workflows.map((wf) => (
                    <option key={wf.id} value={wf.id}>
                      {wf.name} ({wf.id})
                    </option>
                  ))}
                </select>
              </div>

              <div>
                <label className="block text-[#A9B8CC] mb-1 font-semibold">Tag Nhóm (2-4 ký tự)</label>
                <input
                  type="text"
                  value={editingTask.categoryShortTag}
                  onChange={(e) =>
                    setEditingTask({ ...editingTask, categoryShortTag: e.target.value })
                  }
                  className="w-full bg-[#101722] border border-[#2A3647] rounded px-3 py-1.5 text-[#EDF3FC] font-mono uppercase outline-none"
                />
              </div>

              <div>
                <label className="block text-[#A9B8CC] mb-1 font-semibold">Tên Nhóm</label>
                <input
                  type="text"
                  value={editingTask.categoryName}
                  onChange={(e) => setEditingTask({ ...editingTask, categoryName: e.target.value })}
                  className="w-full bg-[#101722] border border-[#2A3647] rounded px-3 py-1.5 text-[#EDF3FC] outline-none"
                />
              </div>

              <div>
                <label className="block text-[#A9B8CC] mb-1 font-semibold">Độ ưu tiên</label>
                <input
                  type="number"
                  min={1}
                  value={editingTask.priority}
                  onChange={(e) =>
                    setEditingTask({ ...editingTask, priority: Number(e.target.value) || 1 })
                  }
                  className="w-full bg-[#101722] border border-[#2A3647] rounded px-3 py-1.5 text-[#EDF3FC] font-mono outline-none"
                />
              </div>

              <div>
                <label className="block text-[#A9B8CC] mb-1 font-semibold">Ước tính (giây)</label>
                <input
                  type="number"
                  min={1}
                  value={editingTask.estimateSec}
                  onChange={(e) =>
                    setEditingTask({ ...editingTask, estimateSec: Number(e.target.value) || 30 })
                  }
                  className="w-full bg-[#101722] border border-[#2A3647] rounded px-3 py-1.5 text-[#EDF3FC] font-mono outline-none"
                />
              </div>

              <div className="col-span-2 flex items-center justify-between pt-1">
                <label className="flex items-center gap-2 cursor-pointer text-[#EDF3FC]">
                  <input
                    type="checkbox"
                    checked={editingTask.defaultEnabled}
                    onChange={(e) =>
                      setEditingTask({ ...editingTask, defaultEnabled: e.target.checked })
                    }
                    className="accent-[#3BC38D]"
                  />
                  <span>Bật mặc định khi xuất Mini Tool</span>
                </label>

                <div className="flex items-center gap-2">
                  <span className="text-[#74869D]">Màu Tag:</span>
                  <input
                    type="color"
                    value={editingTask.categoryColorHex}
                    onChange={(e) =>
                      setEditingTask({ ...editingTask, categoryColorHex: e.target.value })
                    }
                    className="w-7 h-6 rounded bg-transparent border border-[#2A3647] cursor-pointer"
                  />
                </div>
              </div>
            </div>

            <div className="flex items-center justify-end gap-2 pt-3 border-t border-[#2A3647]">
              <button
                type="button"
                onClick={() => setEditingTask(null)}
                className="px-3 py-1.5 rounded bg-[#101722] border border-[#2A3647] text-[#A9B8CC] cursor-pointer"
              >
                Hủy
              </button>
              <button
                type="submit"
                disabled={busyKey?.startsWith('save-task:')}
                className="px-4 py-1.5 rounded bg-[#1878F5] hover:bg-[#2F8CFF] text-white font-semibold cursor-pointer"
              >
                Lưu Task vào SQLite
              </button>
            </div>
          </form>
        </div>
      )}
    </div>
  );
};
