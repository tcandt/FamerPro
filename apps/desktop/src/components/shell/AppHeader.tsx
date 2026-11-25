import React from 'react';
import {
  Bot,
  Minus,
  Square,
  X,
  Play,
  Square as StopSquare,
  CheckCircle2,
  AlertCircle,
  Smartphone,
  RefreshCw,
  FolderGit2,
} from 'lucide-react';
import { useWorkflowStore } from '../../stores/workflowStore';
import { useDeviceStore } from '../../stores/deviceStore';
import { useProjectStore } from '../../stores/projectStore';
import { api, isTauri } from '../../lib/api';
import type { RuntimeSourceStatus } from '../../types';

interface AppHeaderProps {
  currentView: string;
  onSelectView: (view: string) => void;
  onValidate: () => void;
  onRun: () => void;
  onStop: () => void;
  onRefreshDevices?: () => void;
}

const viewNames: Record<string, string> = {
  projects: 'Projects & Dashboard',
  overview: 'Projects & Dashboard',
  studio: 'Workflow Studio',
  tasks: 'Tasks & Modules',
  devices: 'Device Lab',
  runs: 'Test & Debug',
  builder: 'Mini Tool Builder',
  workflows: 'Workflow Repository',
  accounts: 'Tài khoản (Vault)',
  network: 'Proxy & Port Mapping',
  settings: 'System Settings',
  gallery: 'Design Tokens',
};

export function resolveDeviceRuntimeBadge(
  devices: ReturnType<typeof useDeviceStore.getState>['devices'],
  selectedDeviceId: string | null
): { status: RuntimeSourceStatus; label: string; className: string } {
  if (!isTauri()) {
    return { status: 'TESTKIT', label: 'WEB PREVIEW', className: 'fp-badge-testkit' };
  }
  const dev = devices.find((d) => d.id === selectedDeviceId) || devices[0];
  if (!dev) {
    return { status: 'OFFLINE', label: 'OFFLINE', className: 'fp-badge-offline' };
  }
  if (dev.is_mock) {
    return { status: 'TESTKIT', label: 'TESTKIT', className: 'fp-badge-testkit' };
  }
  if (dev.connection_state === 'ready' || dev.connection_state === 'busy') {
    return { status: 'LIVE', label: 'LIVE', className: 'fp-badge-live' };
  }
  if (dev.connection_state === 'unauthorized' || dev.connection_state === 'blocked') {
    return { status: 'ERROR', label: 'ERROR', className: 'fp-badge-error' };
  }
  return { status: 'OFFLINE', label: 'OFFLINE', className: 'fp-badge-offline' };
}

export const AppHeader: React.FC<AppHeaderProps> = ({
  currentView,
  onSelectView,
  onValidate,
  onRun,
  onStop,
  onRefreshDevices,
}) => {
  const { workflowName, isDirty, isRunning, validationReport } = useWorkflowStore();
  const { devices, selectedDeviceId, selectDevice } = useDeviceStore();
  const {
    projects,
    activeProjectId,
    activeProjectDetail,
    selectProject,
    fetchProjects,
    isLoadingProjects,
  } = useProjectStore();

  const deviceBadge = resolveDeviceRuntimeBadge(devices, selectedDeviceId);
  const activeProj =
    activeProjectDetail?.project || projects.find((p) => p.project.id === activeProjectId)?.project;

  return (
    <header
      data-tauri-drag-region
      onDoubleClick={() => api.windowToggleMaximize()}
      onMouseDown={(e) => {
        if (e.button === 0 && !(e.target as HTMLElement).closest('button, input, select, textarea, [data-no-drag]')) {
          api.windowStartDragging();
        }
      }}
      className="h-[44px] bg-[#141C28] border-b border-[#2A3647] flex items-center justify-between px-3 select-none z-50 text-xs shrink-0 cursor-default gap-2"
    >
      {/* Left: Studio Brand + Breadcrumb */}
      <div className="flex items-center gap-2 min-w-0" data-tauri-drag-region>
        <button
          onClick={() => onSelectView('projects')}
          className="flex items-center gap-2 cursor-pointer hover:opacity-90 shrink-0"
          title="Về Dashboard Dự án"
        >
          <div className="w-6 h-6 rounded bg-[#1878F5] flex items-center justify-center text-white font-bold shadow-sm">
            <Bot size={14} />
          </div>
          <span className="font-bold text-[#EDF3FC] tracking-wide text-[13px] whitespace-nowrap">
            FarmerPro Studio
          </span>
        </button>

        <span className="hidden xl:inline-flex px-1.5 py-0.5 rounded text-[10px] bg-[#173355] text-[#4B9CFF] font-semibold border border-[#2A3647] whitespace-nowrap">
          IDE v0.1
        </span>

        <div className="h-3.5 w-px bg-[#2A3647] mx-0.5 shrink-0" />

        <div className="flex items-center gap-1.5 min-w-0 text-[12px] text-[#74869D]">
          <span className="text-[#A9B8CC] font-medium truncate max-w-[170px]">
            {viewNames[currentView] || currentView}
          </span>
          {currentView === 'studio' && (
            <>
              <span>/</span>
              <span className="text-[#EDF3FC] font-semibold truncate max-w-[160px]">
                {workflowName}
              </span>
              {isDirty && (
                <span
                  className="w-2 h-2 rounded-full bg-[#E7B15B] shrink-0"
                  title="Có thay đổi chưa lưu"
                />
              )}
            </>
          )}
        </div>
      </div>

      {/* Center: Global Project Context + Global Device Context */}
      <div className="flex items-center gap-2 min-w-0">
        {/* Global Project Context Selector */}
        <div
          className="flex items-center gap-1.5 bg-[#101722] border border-[#2A3647] rounded-md px-2 py-1 text-xs text-[#EDF3FC] min-w-0"
          title={
            activeProj
              ? `${activeProj.name} (${activeProj.gamePackageId}) · ${activeProj.targetWidthPx}×${activeProj.targetHeightPx}`
              : 'Chọn Game Project làm việc'
          }
        >
          <FolderGit2 size={13} className="text-[#4B9CFF] shrink-0" />
          <span className="text-[11px] text-[#74869D] hidden lg:inline shrink-0">Project:</span>
          <select
            value={activeProjectId || ''}
            onChange={(e) => selectProject(e.target.value || null)}
            className="bg-transparent text-[#EDF3FC] outline-none cursor-pointer text-xs font-medium max-w-[165px] truncate"
            aria-label="Active Game Project"
          >
            {projects.length === 0 ? (
              <option value="" className="bg-[#161E29] text-[#74869D]">
                {isTauri() ? 'Chưa có Project' : 'Cần Tauri SQLite'}
              </option>
            ) : (
              projects.map((p) => (
                <option key={p.project.id} value={p.project.id} className="bg-[#161E29] text-[#EDF3FC]">
                  {p.project.name} ({p.project.version})
                </option>
              ))
            )}
          </select>
          {activeProj && (
            <span className="hidden 2xl:inline text-[10px] font-mono px-1.5 py-0.2 rounded bg-[#192230] text-[#A9B8CC] border border-[#2A3647]">
              {activeProj.targetWidthPx}×{activeProj.targetHeightPx}
            </span>
          )}
        </div>

        {/* Global Device Context Selector + Honest Runtime Badge */}
        <div
          className="flex items-center gap-1.5 bg-[#101722] border border-[#2A3647] rounded-md px-2 py-1 text-xs text-[#EDF3FC] min-w-0"
          title="Ngữ cảnh Thiết bị Toàn cục"
        >
          <Smartphone size={13} className="text-[#3BC38D] shrink-0" />
          <select
            value={selectedDeviceId || ''}
            onChange={(e) => selectDevice(e.target.value || null)}
            className="bg-transparent text-[#EDF3FC] outline-none cursor-pointer text-xs font-medium max-w-[155px] truncate"
            aria-label="Active Target Device"
          >
            {devices.length === 0 ? (
              <option value="" className="bg-[#161E29] text-[#74869D]">
                Không có thiết bị ADB
              </option>
            ) : (
              devices.map((d) => (
                <option key={d.id} value={d.id} className="bg-[#161E29] text-[#EDF3FC]">
                  {d.alias} ({d.adb_serial})
                </option>
              ))
            )}
          </select>
          <span
            className={`text-[10px] font-bold px-1.5 py-0.2 rounded tracking-wide shrink-0 ${deviceBadge.className}`}
          >
            {deviceBadge.label}
          </span>
        </div>
      </div>

      {/* Right: Contextual View Controls + Window Controls */}
      <div className="flex items-center gap-1.5 shrink-0">
        {currentView === 'projects' && (
          <button
            onClick={() => fetchProjects(true)}
            disabled={isLoadingProjects}
            className="px-2.5 py-1 bg-[#192230] hover:bg-[#202B3B] text-[#EDF3FC] text-xs font-medium rounded-md border border-[#2A3647] flex items-center gap-1.5 transition-colors cursor-pointer disabled:opacity-50"
          >
            <RefreshCw size={12} className={`text-[#4B9CFF] ${isLoadingProjects ? 'animate-spin' : ''}`} />
            <span className="hidden md:inline">Đồng bộ Projects</span>
          </button>
        )}

        {currentView === 'devices' && onRefreshDevices && (
          <button
            onClick={onRefreshDevices}
            className="px-2.5 py-1 bg-[#192230] hover:bg-[#202B3B] text-[#EDF3FC] text-xs font-medium rounded-md border border-[#2A3647] flex items-center gap-1.5 transition-colors cursor-pointer"
          >
            <RefreshCw size={12} className="text-[#3BC38D]" />
            <span>Quét ADB</span>
          </button>
        )}

        {currentView === 'studio' && (
          <div className="flex items-center gap-1.5">
            {validationReport && (
              <div
                className={`flex items-center gap-1 px-2 py-0.5 rounded text-[11px] border font-medium ${
                  validationReport.is_valid
                    ? 'bg-[#133528] text-[#3BC38D] border-[#3BC38D]/40'
                    : 'bg-[#F27883]/10 text-[#F27883] border-[#F27883]/40'
                }`}
              >
                {validationReport.is_valid ? <CheckCircle2 size={12} /> : <AlertCircle size={12} />}
                <span>
                  {validationReport.is_valid ? 'Hợp lệ' : `${validationReport.errors.length} lỗi`}
                </span>
              </div>
            )}

            <button
              onClick={onValidate}
              className="px-2.5 py-1 bg-[#192230] hover:bg-[#202B3B] text-[#EDF3FC] text-xs font-medium rounded-md border border-[#2A3647] flex items-center gap-1.5 transition-colors cursor-pointer"
              title="Kiểm tra cấu trúc luồng"
            >
              <CheckCircle2 size={13} className="text-[#4B9CFF]" />
              <span className="hidden lg:inline">Kiểm tra</span>
            </button>

            {!isRunning ? (
              <button
                onClick={onRun}
                className="px-3 py-1 bg-[#1878F5] hover:bg-[#2F8CFF] text-white text-xs font-semibold rounded-md flex items-center gap-1.5 shadow-sm transition-colors cursor-pointer"
                title="Chạy thử luồng"
              >
                <Play size={12} fill="currentColor" />
                <span>Chạy luồng</span>
              </button>
            ) : (
              <button
                onClick={onStop}
                className="px-3 py-1 bg-[#F27883] hover:bg-[#ff8f99] text-white text-xs font-semibold rounded-md flex items-center gap-1.5 shadow-sm transition-colors cursor-pointer"
                title="Dừng luồng"
              >
                <StopSquare size={12} fill="currentColor" />
                <span>Dừng</span>
              </button>
            )}
          </div>
        )}

        <div className="h-4 w-px bg-[#2A3647] mx-0.5" />

        <div className="flex items-center space-x-0.5">
          <button
            onClick={() => api.windowMinimize()}
            className="w-7 h-7 flex items-center justify-center hover:bg-[#202B3B] text-[#A9B8CC] hover:text-white rounded transition-colors cursor-pointer"
            title="Thu nhỏ"
            aria-label="Minimize"
          >
            <Minus size={13} />
          </button>
          <button
            onClick={() => api.windowToggleMaximize()}
            className="w-7 h-7 flex items-center justify-center hover:bg-[#202B3B] text-[#A9B8CC] hover:text-white rounded transition-colors cursor-pointer"
            title="Phóng to / Thu nhỏ"
            aria-label="Maximize"
          >
            <Square size={11} />
          </button>
          <button
            onClick={() => api.windowClose()}
            className="w-7 h-7 flex items-center justify-center hover:bg-[#F27883] text-[#A9B8CC] hover:text-white rounded transition-colors cursor-pointer"
            title="Đóng"
            aria-label="Close"
          >
            <X size={13} />
          </button>
        </div>
      </div>
    </header>
  );
};
