import React from 'react';
import { AlertCircle } from 'lucide-react';

export const NetworkView: React.FC = () => {
  return (
    <div className="flex-1 overflow-y-auto p-6 space-y-6 bg-[#101722] text-[#EDF3FC]">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold text-[#EDF3FC]">Cấu hình Mạng & Proxy</h1>
          <p className="text-xs text-[#74869D] mt-0.5">
            Quản lý profile mạng và kiểm tra khả năng định tuyến.
          </p>
        </div>
      </div>

      <div className="p-4 rounded-[9px] bg-[#3B2D17]/40 border border-[#E7B15B]/40 flex items-start gap-3 text-xs text-[#E7B15B]">
        <AlertCircle size={18} className="shrink-0 mt-0.5" />
        <div>
          <div className="font-semibold">Lưu ý quan trọng về Proxy trên Android:</div>
          <p className="mt-1 leading-relaxed text-[#A9B8CC]">
            Đổi proxy trên máy tính không tự động thay đổi kết nối của game trên điện thoại. Lệnh cài đặt Android System Proxy qua ADB có thể bị nhiều game bỏ qua nếu không sử dụng VPN profile hợp lệ. FarmerPro không cam kết vượt rào cản mạng nếu ứng dụng đích không hỗ trợ.
          </p>
        </div>
      </div>

      <div className="grid grid-cols-2 gap-4 text-xs">
        <div className="p-4 bg-[#192230] border border-[#2A3647] rounded-[9px] space-y-2">
          <div className="flex items-center justify-between">
            <span className="font-semibold text-[#EDF3FC]">Profile Mạng Trực tiếp (Direct LAN)</span>
            <span className="px-2 py-0.5 rounded text-[10px] bg-[#133528] text-[#3BC38D] border border-[#3BC38D]/30">
              Đang hoạt động
            </span>
          </div>
          <p className="text-[#74869D] text-[11px]">Kết nối trực tiếp qua mạng nội bộ gia đình / văn phòng. Ping: 12ms.</p>
        </div>

        <div className="p-4 bg-[#192230] border border-[#2A3647] rounded-[9px] space-y-2 opacity-60">
          <div className="flex items-center justify-between">
            <span className="font-semibold text-[#EDF3FC]">SOCKS5 Gateway Cục bộ</span>
            <span className="px-2 py-0.5 rounded text-[10px] bg-[#202B3B] text-[#74869D] border border-[#2A3647]">
              Chưa bật
            </span>
          </div>
          <p className="text-[#74869D] text-[11px]">Định tuyến qua proxy cục bộ đã được ủy quyền.</p>
        </div>
      </div>
    </div>
  );
};
