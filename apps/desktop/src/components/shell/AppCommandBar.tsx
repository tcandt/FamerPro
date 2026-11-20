import React from 'react';
import { Play, Square, CheckCircle2, Smartphone, Undo2, Redo2, Search, AlertCircle } from 'lucide-react';
import { useWorkflowStore } from '../../stores/workflowStore';
import { useDeviceStore } from '../../stores/deviceStore';

interface AppCommandBarProps {
  currentView: string;
  onValidate: () => void;
  onRun: () => void;
  onStop: () => void;
}

export const AppCommandBar: React.FC<AppCommandBarProps> = ({
  currentView,
  onValidate,
  onRun,
  onStop,
}) => {
  const { workflowName, isDirty, isRunning, validationReport } = useWorkflowStore();
  const { devices, selectedDeviceId, selectDevice } = useDeviceStore();

  return (
    <div className="h-[52px] bg-[#192230] border-b border-[#2A3647] flex items-center justify-between px-4 z-40 select-none">
      {/* Left: Breadcrumbs & Workflow status */}
      <div className="flex items-center gap-2 min-w-0">
        <span className="text-xs text-[#74869D]">FarmerPro</span>
        <span className="text-xs text-[#74869D]">/</span>
        <span className="text-xs text-[#A9B8CC] capitalize">
          {currentView === 'studio' ? 'Quy trình' : currentView}
        </span>
        <span className="text-xs text-[#74869D]">/</span>
        <span className="text-xs font-medium text-[#EDF3FC] truncate max-w-[200px]">
          {workflowName}
        </span>
        {isDirty && (
          <span className="w-2 h-2 rounded-full bg-[#E7B15B]" title="Có thay đổi chưa lưu" />
        )}
        <span className="text-[11px] px-1.5 py-0.5 rounded bg-[#202B3B] text-[#7BA6D6] border border-[#2A3647]">
          Bản nháp
        </span>
      </div>

      {/* Middle: Device selector and Global commands */}
      <div className="flex items-center gap-3">
        {currentView === 'studio' && (
          <>
            <div className="flex items-center gap-1.5 bg-[#101722] border border-[#2A3647] rounded px-2.5 py-1 text-xs text-[#EDF3FC]">
              <Smartphone size={14} className="text-[#1878F5]" />
              <span className="text-[#74869D]">Thiết bị:</span>
              <select
                value={selectedDeviceId || ''}
                onChange={(e) => selectDevice(e.target.value)}
                className="bg-transparent text-[#EDF3FC] outline-none cursor-pointer text-xs"
              >
                {devices.map((d) => (
                  <option key={d.id} value={d.id} className="bg-[#161E29] text-[#EDF3FC]">
                    {d.alias} ({d.adb_serial})
                  </option>
                ))}
              </select>
            </div>

            <div className="flex items-center gap-1 border-r border-[#2A3647] pr-2">
              <button
                className="p-1.5 text-[#A9B8CC] hover:text-[#EDF3FC] hover:bg-[#202B3B] rounded"
                title="Hoàn tác (Ctrl+Z)"
              >
                <Undo2 size={14} />
              </button>
              <button
                className="p-1.5 text-[#A9B8CC] hover:text-[#EDF3FC] hover:bg-[#202B3B] rounded"
                title="Làm lại (Ctrl+Shift+Z)"
              >
                <Redo2 size={14} />
              </button>
            </div>
          </>
        )}

        <div className="flex items-center gap-1.5 bg-[#101722] border border-[#2A3647] rounded px-2.5 py-1 text-xs text-[#74869D]">
          <Search size={13} />
          <span>Lệnh (Ctrl+K)</span>
        </div>
      </div>

      {/* Right: Validation & Run Controls */}
      <div className="flex items-center gap-2">
        {validationReport && (
          <div
            className={`flex items-center gap-1 px-2 py-1 rounded text-[11px] border ${
              validationReport.is_valid
                ? 'bg-[#173355] text-[#3BC38D] border-[#3BC38D]/40'
                : 'bg-[#F27883]/10 text-[#F27883] border-[#F27883]/40'
            }`}
          >
            {validationReport.is_valid ? <CheckCircle2 size={13} /> : <AlertCircle size={13} />}
            <span>{validationReport.is_valid ? 'Hợp lệ' : `${validationReport.errors.length} lỗi`}</span>
          </div>
        )}

        <button
          onClick={onValidate}
          className="px-3 py-1.5 bg-[#202B3B] hover:bg-[#2A3647] text-[#EDF3FC] text-xs font-medium rounded border border-[#2A3647] flex items-center gap-1.5 transition-colors"
        >
          <CheckCircle2 size={14} className="text-[#4B9CFF]" />
          <span>Kiểm tra luồng</span>
        </button>

        {!isRunning ? (
          <button
            onClick={onRun}
            className="px-3.5 py-1.5 bg-[#1878F5] hover:bg-[#2F8CFF] text-white text-xs font-medium rounded flex items-center gap-1.5 shadow-sm transition-colors cursor-pointer"
          >
            <Play size={13} fill="currentColor" />
            <span>Chạy luồng</span>
          </button>
        ) : (
          <button
            onClick={onStop}
            className="px-3.5 py-1.5 bg-[#F27883] hover:bg-[#ff8f99] text-white text-xs font-medium rounded flex items-center gap-1.5 shadow-sm transition-colors cursor-pointer"
          >
            <Square size={13} fill="currentColor" />
            <span>Dừng luồng</span>
          </button>
        )}
      </div>
    </div>
  );
};
