import React, { useEffect } from 'react';
import { Database, Smartphone, ShieldCheck, Terminal } from 'lucide-react';
import { useDeviceStore } from '../../stores/deviceStore';
import { useWorkflowStore } from '../../stores/workflowStore';

export const StatusBar: React.FC = () => {
  const { devices, appStatus, fetchStatus } = useDeviceStore();
  const { isRunning } = useWorkflowStore();
  const onlineCount = devices.filter((d) => d.connection_state === 'ready').length;

  useEffect(() => {
    fetchStatus();
  }, [fetchStatus]);

  const isPortable = appStatus?.is_portable ?? true;

  return (
    <footer className="h-6.5 bg-[#141C28] border-t border-[#2A3647] flex items-center justify-between px-3 text-[11px] text-[#74869D] select-none z-50 shrink-0">
      <div className="flex items-center gap-4">
        <div className="flex items-center gap-1.5" title={appStatus?.adb_path || 'bin/adb.exe'}>
          <Terminal size={12} className="text-[#3BC38D]" />
          <span>ADB: {appStatus?.adb_version || '1.0.41'} (Sẵn sàng)</span>
        </div>

        <div className="flex items-center gap-1.5">
          <Smartphone size={12} className={onlineCount > 0 ? 'text-[#3BC38D]' : 'text-[#74869D]'} />
          <span>
            {onlineCount} thiết bị sẵn sàng ({devices.length} kết nối)
          </span>
        </div>

        <div className="flex items-center gap-1.5" title={appStatus?.db_path || 'data/farmerpro.db'}>
          <Database size={12} className="text-[#4B9CFF]" />
          <span>SQLite WAL: Cục bộ {isPortable ? '(Thư mục data/)' : ''}</span>
        </div>
      </div>

      <div className="flex items-center gap-3">
        {isRunning && (
          <div className="flex items-center gap-1.5 text-[#E7B15B] font-medium">
            <span className="w-1.5 h-1.5 rounded-full bg-[#E7B15B] animate-pulse" />
            <span>Workflow đang thực thi...</span>
          </div>
        )}
        <div className="flex items-center gap-1 text-[#3BC38D]">
          <ShieldCheck size={12} />
          <span>Local First (Bảo mật 100% Cục bộ)</span>
        </div>
      </div>
    </footer>
  );
};
