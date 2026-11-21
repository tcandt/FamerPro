import React from 'react';
import {
  Bot,
  Search,
  Minus,
  Square,
  X,
  Play,
  Square as StopSquare,
  CheckCircle2,
  AlertCircle,
  Smartphone,
  RefreshCw,
} from 'lucide-react';
import { useWorkflowStore } from '../../stores/workflowStore';
import { useDeviceStore } from '../../stores/deviceStore';
import { api } from '../../lib/api';

interface AppHeaderProps {
  currentView: string;
  onValidate: () => void;
  onRun: () => void;
  onStop: () => void;
  onRefreshDevices?: () => void;
}

const viewNames: Record<string, string> = {
  overview: 'Trung tâm Tổng quan',
  devices: 'Quản lý Thiết bị Android',
  studio: 'Workflow Studio',
  tasks: 'Bộ Nhiệm vụ (Planner)',
  runs: 'Nhật ký Phiên chạy (Monitor)',
  accounts: 'Quản lý Tài khoản (Vault)',
  network: 'Mạng & Proxy Gateway',
  gallery: 'Design Tokens & Rubric',
  settings: 'Cài đặt & Chẩn đoán',
};

export const AppHeader: React.FC<AppHeaderProps> = ({
  currentView,
  onValidate,
  onRun,
  onStop,
  onRefreshDevices,
}) => {
  const { workflowName, isDirty, isRunning, validationReport } = useWorkflowStore();
  const { devices, selectedDeviceId, selectDevice } = useDeviceStore();

  const handleMinimize = () => {
    api.windowMinimize();
  };

  const handleToggleMaximize = () => {
    api.windowToggleMaximize();
  };

  const handleClose = () => {
    api.windowClose();
  };

  return (
    <header
      data-tauri-drag-region
      onDoubleClick={handleToggleMaximize}
      className="h-[44px] bg-[#141C28] border-b border-[#2A3647] flex items-center justify-between px-3 select-none z-50 text-xs shrink-0 cursor-default"
    >
      {/* Left: Branding & Breadcrumbs */}
      <div className="flex items-center gap-2.5 min-w-0" data-tauri-drag-region>
        <div className="flex items-center gap-2">
          <div className="w-5.5 h-5.5 rounded bg-[#1878F5] flex items-center justify-center text-white font-bold shadow-sm">
            <Bot size={13} />
          </div>
          <span className="font-bold text-[#EDF3FC] tracking-wide text-[13px]">FarmerPro</span>
          <span className="px-1.5 py-0.5 rounded text-[10px] bg-[#173355] text-[#4B9CFF] font-semibold border border-[#2A3647]">
            v0.1.0-portable
          </span>
        </div>

        <div className="h-3.5 w-px bg-[#2A3647] mx-1" />

        {/* Breadcrumb path */}
        <div className="flex items-center gap-1.5 min-w-0 text-xs text-[#74869D]">
          <span>Phân hệ</span>
          <span>/</span>
          <span className="text-[#A9B8CC] font-medium">{viewNames[currentView] || currentView}</span>
          {currentView === 'studio' && (
            <>
              <span>/</span>
              <span className="text-[#EDF3FC] font-semibold truncate max-w-[180px]">
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

      {/* Center: Quick Switcher / Global Command search */}
      <div className="flex items-center gap-2" data-tauri-drag-region>
        {currentView === 'studio' && (
          <div className="flex items-center gap-1.5 bg-[#101722] border border-[#2A3647] rounded-md px-2 py-1 text-xs text-[#EDF3FC]">
            <Smartphone size={13} className="text-[#1878F5] shrink-0" />
            <span className="text-[11px] text-[#74869D]">Thiết bị:</span>
            <select
              value={selectedDeviceId || ''}
              onChange={(e) => selectDevice(e.target.value)}
              className="bg-transparent text-[#EDF3FC] outline-none cursor-pointer text-xs font-medium max-w-[170px] truncate"
            >
              {devices.map((d) => (
                <option key={d.id} value={d.id} className="bg-[#161E29] text-[#EDF3FC]">
                  {d.alias} {d.is_mock ? '(Demo)' : '(Live)'}
                </option>
              ))}
            </select>
          </div>
        )}

        <button
          className="flex items-center gap-2 bg-[#101722] hover:bg-[#192230] border border-[#2A3647] rounded-md px-2.5 py-1 text-xs text-[#74869D] hover:text-[#A9B8CC] transition-colors cursor-pointer"
          title="Mở bảng lệnh nhanh"
        >
          <Search size={12} />
          <span className="text-[11px]">Lệnh nhanh (Ctrl+K)</span>
        </button>
      </div>

      {/* Right: View-specific Primary Actions & Window Controls */}
      <div className="flex items-center gap-2">
        {/* Studio Action Controls */}
        {currentView === 'studio' && (
          <div className="flex items-center gap-1.5 mr-1">
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
              title="Kiểm tra cấu trúc và tính liên tục của luồng"
            >
              <CheckCircle2 size={13} className="text-[#4B9CFF]" />
              <span>Kiểm tra</span>
            </button>

            {!isRunning ? (
              <button
                onClick={onRun}
                className="px-3 py-1 bg-[#1878F5] hover:bg-[#2F8CFF] text-white text-xs font-semibold rounded-md flex items-center gap-1.5 shadow-sm transition-colors cursor-pointer"
                title="Chạy luồng trên thiết bị đang chọn"
              >
                <Play size={12} fill="currentColor" />
                <span>Chạy luồng</span>
              </button>
            ) : (
              <button
                onClick={onStop}
                className="px-3 py-1 bg-[#F27883] hover:bg-[#ff8f99] text-white text-xs font-semibold rounded-md flex items-center gap-1.5 shadow-sm transition-colors cursor-pointer"
                title="Dừng khẩn cấp phiên chạy"
              >
                <StopSquare size={12} fill="currentColor" />
                <span>Dừng luồng</span>
              </button>
            )}
          </div>
        )}

        {/* Devices Action Controls */}
        {currentView === 'devices' && onRefreshDevices && (
          <button
            onClick={onRefreshDevices}
            className="px-2.5 py-1 bg-[#192230] hover:bg-[#202B3B] text-[#EDF3FC] text-xs font-medium rounded-md border border-[#2A3647] flex items-center gap-1.5 transition-colors cursor-pointer mr-1"
          >
            <RefreshCw size={12} className="text-[#3BC38D]" />
            <span>Quét lại ADB</span>
          </button>
        )}

        <div className="h-4 w-px bg-[#2A3647] mx-0.5" />

        {/* Window Control Buttons */}
        <div className="flex items-center space-x-0.5">
          <button
            onClick={handleMinimize}
            className="w-8 h-7 flex items-center justify-center hover:bg-[#202B3B] text-[#A9B8CC] hover:text-white rounded transition-colors"
            title="Thu nhỏ xuống Taskbar"
            aria-label="Minimize"
          >
            <Minus size={13} />
          </button>
          <button
            onClick={handleToggleMaximize}
            className="w-8 h-7 flex items-center justify-center hover:bg-[#202B3B] text-[#A9B8CC] hover:text-white rounded transition-colors"
            title="Phóng to / Thu nhỏ cửa sổ"
            aria-label="Maximize"
          >
            <Square size={11} />
          </button>
          <button
            onClick={handleClose}
            className="w-8 h-7 flex items-center justify-center hover:bg-[#F27883] text-[#A9B8CC] hover:text-white rounded transition-colors"
            title="Đóng ứng dụng"
            aria-label="Close"
          >
            <X size={13} />
          </button>
        </div>
      </div>
    </header>
  );
};
