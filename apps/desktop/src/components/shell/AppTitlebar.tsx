import React from 'react';
import { Minus, Square, X, Bot } from 'lucide-react';
import { api } from '../../lib/api';

export const AppTitlebar: React.FC = () => {
  return (
    <div
      data-tauri-drag-region
      className="h-[38px] bg-[#161E29] border-b border-[#2A3647] flex items-center justify-between px-3 select-none z-50 text-xs"
    >
      <div className="flex items-center gap-2">
        <div className="w-5 h-5 rounded bg-[#1878F5] flex items-center justify-center text-white font-bold text-[10px]">
          <Bot size={13} />
        </div>
        <span className="font-semibold text-[#EDF3FC] tracking-wide">FarmerPro</span>
        <span className="text-[#74869D]">·</span>
        <span className="text-[#A9B8CC]">Android Automation Studio</span>
        <span className="px-1.5 py-0.5 rounded text-[10px] bg-[#173355] text-[#4B9CFF] font-medium border border-[#2A3647]">
          v0.1.0-preview
        </span>
      </div>

      <div className="flex items-center space-x-1">
        <button
          onClick={() => api.windowMinimize()}
          className="w-8 h-6 flex items-center justify-center hover:bg-[#202B3B] text-[#A9B8CC] hover:text-white rounded cursor-pointer"
          aria-label="Minimize"
          title="Thu nhỏ cửa sổ"
        >
          <Minus size={13} />
        </button>
        <button
          onClick={() => api.windowToggleMaximize()}
          className="w-8 h-6 flex items-center justify-center hover:bg-[#202B3B] text-[#A9B8CC] hover:text-white rounded cursor-pointer"
          aria-label="Maximize"
          title="Phóng to / Khôi phục cửa sổ"
        >
          <Square size={11} />
        </button>
        <button
          onClick={() => api.windowClose()}
          className="w-8 h-6 flex items-center justify-center hover:bg-[#F27883] text-[#A9B8CC] hover:text-white rounded cursor-pointer"
          aria-label="Close"
          title="Đóng ứng dụng"
        >
          <X size={13} />
        </button>
      </div>
    </div>
  );
};
