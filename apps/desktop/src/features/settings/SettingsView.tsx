import React from 'react';
import { Database, Terminal, Shield } from 'lucide-react';

export const SettingsView: React.FC = () => {
  return (
    <div className="flex-1 overflow-y-auto p-6 space-y-6 bg-[#101722] text-[#EDF3FC]">
      <div>
        <h1 className="text-xl font-bold text-[#EDF3FC]">Cài đặt & Chẩn đoán</h1>
        <p className="text-xs text-[#74869D] mt-0.5">
          Cấu hình môi trường ADB, đường dẫn cơ sở dữ liệu SQLite và chính sách bảo mật cục bộ.
        </p>
      </div>

      <div className="space-y-4 max-w-2xl text-xs">
        <div className="p-4 bg-[#192230] border border-[#2A3647] rounded-[9px] space-y-3">
          <div className="flex items-center gap-2 font-semibold text-[#EDF3FC]">
            <Terminal size={15} className="text-[#1878F5]" />
            <span>Đường dẫn ADB (Android Debug Bridge)</span>
          </div>
          <div>
            <label className="block text-[#74869D] mb-1">Thực thi ADB:</label>
            <div className="flex gap-2">
              <input
                type="text"
                readOnly
                value="C:\Users\TINH-NGUYEN\AppData\Local\Microsoft\WinGet\Packages\Genymobile.scrcpy_Microsoft.Winget.Source_8wekyb3d8bbwe\scrcpy-win64-v4.1\adb.exe"
                className="flex-1 bg-[#101722] border border-[#2A3647] rounded px-2.5 py-1.5 text-[#EDF3FC] font-mono text-[11px]"
              />
              <button className="px-3 py-1.5 bg-[#202B3B] hover:bg-[#2A3647] text-[#EDF3FC] rounded border border-[#2A3647]">
                Duyệt...
              </button>
            </div>
            <p className="text-[11px] text-[#3BC38D] mt-1">Đã phát hiện ADB phiên bản 1.0.41</p>
          </div>
        </div>

        <div className="p-4 bg-[#192230] border border-[#2A3647] rounded-[9px] space-y-3">
          <div className="flex items-center gap-2 font-semibold text-[#EDF3FC]">
            <Database size={15} className="text-[#3BC38D]" />
            <span>Lưu trữ Cục bộ (SQLite WAL)</span>
          </div>
          <div>
            <label className="block text-[#74869D] mb-1">Tệp cơ sở dữ liệu:</label>
            <input
              type="text"
              readOnly
              value="farmerpro.db (Chế độ WAL + Foreign Keys bật)"
              className="w-full bg-[#101722] border border-[#2A3647] rounded px-2.5 py-1.5 text-[#EDF3FC] font-mono text-[11px]"
            />
          </div>
        </div>

        <div className="p-4 bg-[#192230] border border-[#2A3647] rounded-[9px] space-y-3">
          <div className="flex items-center gap-2 font-semibold text-[#EDF3FC]">
            <Shield size={15} className="text-[#E7B15B]" />
            <span>Hàng rào an toàn (Destructive Action Guard)</span>
          </div>
          <p className="text-[11px] text-[#A9B8CC] leading-relaxed">
            Các thao tác như Xóa dữ liệu ứng dụng (Clear App Data) hoặc Khởi động lại thiết bị bắt buộc phải có xác nhận rõ ràng của người dùng trước khi lệnh được gửi xuống thiết bị.
          </p>
        </div>
      </div>
    </div>
  );
};
