import React, { useEffect } from 'react';
import { Database, Smartphone, ShieldCheck, Terminal, FolderGit2, Radio } from 'lucide-react';
import { useDeviceStore } from '../../stores/deviceStore';
import { useWorkflowStore } from '../../stores/workflowStore';
import { useProjectStore } from '../../stores/projectStore';
import { isTauri } from '../../lib/api';

export const StatusBar: React.FC = () => {
  const { devices, appStatus, fetchStatus } = useDeviceStore();
  const { isRunning } = useWorkflowStore();
  const { projects, activeProjectId, activeProjectDetail } = useProjectStore();

  const readyCount = devices.filter((d) => d.connection_state === 'ready' && !d.is_mock).length;
  const tauriLive = isTauri();
  const activeProj =
    activeProjectDetail?.project || projects.find((p) => p.project.id === activeProjectId)?.project;

  useEffect(() => {
    fetchStatus();
  }, [fetchStatus]);

  const isPortable = appStatus?.is_portable ?? true;

  return (
    <footer className="h-[26px] bg-[#141C28] border-t border-[#2A3647] flex items-center justify-between px-3 text-[11px] text-[#74869D] select-none z-50 shrink-0">
      <div className="flex items-center gap-3.5 min-w-0">
        <div
          className={`flex items-center gap-1 px-1.5 py-0.2 rounded font-semibold text-[10px] ${
            tauriLive ? 'fp-badge-live' : 'fp-badge-testkit'
          }`}
          title={
            tauriLive
              ? 'Đang kết nối thật với FarmerPro Rust/SQLite Tauri Core'
              : 'Đang mở bằng trình duyệt web (không có Rust/SQLite Tauri IPC)'
          }
        >
          <Radio size={10} />
          <span>{tauriLive ? 'TAURI LIVE IPC' : 'WEB PREVIEW (NO TAURI)'}</span>
        </div>

        <div className="flex items-center gap-1.5 min-w-0" title={activeProj?.projectDirPath || ''}>
          <FolderGit2 size={12} className="text-[#4B9CFF] shrink-0" />
          <span className="truncate max-w-[200px]">
            {activeProj ? `${activeProj.slug} · v${activeProj.version}` : 'Chưa chọn Project'}
          </span>
        </div>

        <div
          className="hidden sm:flex items-center gap-1.5"
          title={appStatus?.adb_path || 'bin/adb.exe'}
        >
          <Terminal size={12} className={appStatus?.adb_available ? 'text-[#3BC38D]' : 'text-[#74869D]'} />
          <span>
            ADB: {appStatus?.adb_available ? appStatus.adb_version : 'Chưa kiểm tra'}
          </span>
        </div>

        <div className="flex items-center gap-1.5">
          <Smartphone size={12} className={readyCount > 0 ? 'text-[#3BC38D]' : 'text-[#74869D]'} />
          <span>
            {readyCount} thiết bị LIVE ({devices.length} phát hiện)
          </span>
        </div>

        <div
          className="hidden lg:flex items-center gap-1.5"
          title={appStatus?.db_path || 'data/farmerpro.db'}
        >
          <Database size={12} className="text-[#4B9CFF]" />
          <span>SQLite WAL {isPortable ? '(Portable data/)' : '(Local)'}</span>
        </div>
      </div>

      <div className="flex items-center gap-3 shrink-0">
        {isRunning && (
          <div className="flex items-center gap-1.5 text-[#E7B15B] font-medium">
            <span className="w-1.5 h-1.5 rounded-full bg-[#E7B15B] animate-pulse" />
            <span>Runtime đang chạy...</span>
          </div>
        )}
        <div className="flex items-center gap-1 text-[#3BC38D]">
          <ShieldCheck size={12} />
          <span>Local-First Studio</span>
        </div>
      </div>
    </footer>
  );
};
