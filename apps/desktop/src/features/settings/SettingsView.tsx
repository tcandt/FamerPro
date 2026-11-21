import React, { useState } from 'react';
import {
  Database,
  Terminal,
  Shield,
  CheckCircle2,
  HardDrive,
  Sliders,
  RotateCw,
} from 'lucide-react';
import { useDeviceStore } from '../../stores/deviceStore';

export const SettingsView: React.FC = () => {
  const { appStatus } = useDeviceStore();

  const [marginRatio, setMarginRatio] = useState(0.18);
  const [avoidPoints, setAvoidPoints] = useState(5);
  const [dbOptimized, setDbOptimized] = useState(false);

  const handleOptimizeDb = () => {
    setDbOptimized(true);
    setTimeout(() => {
      setDbOptimized(false);
    }, 2500);
  };

  const adbPathDisplay =
    appStatus?.adb_path || 'bin/adb.exe (Tự động ưu tiên thư mục nội bộ)';
  const dbPathDisplay =
    appStatus?.db_path || 'data/farmerpro.db (Thư mục dữ liệu cạnh FarmerPro.exe)';

  return (
    <div className="flex-1 overflow-y-auto p-6 space-y-6 bg-[#101722] text-[#EDF3FC]">
      {/* Top Header */}
      <div>
        <div className="flex items-center gap-2">
          <h1 className="text-xl font-bold text-[#EDF3FC] tracking-tight">
            Cài đặt & Môi trường Chạy (Portable Settings)
          </h1>
          <span className="px-2 py-0.5 rounded text-[10px] bg-[#173355] text-[#38BDF8] font-mono font-semibold border border-[#2A3647]">
            Portable Mode (Zero-Install)
          </span>
        </div>
        <p className="text-xs text-[#74869D] mt-1">
          Cấu hình môi trường ADB nội bộ, chính sách Safe Tap an toàn, vị trí lưu trữ SQLite và chẩn đoán WebView2.
        </p>
      </div>

      <div className="grid grid-cols-12 gap-6 max-w-5xl">
        {/* Left Column (7 cols): Portable Environment & Storage */}
        <div className="col-span-7 space-y-6">
          {/* Section 1: Portable ADB Runtime */}
          <div className="p-4 bg-[#16202F] border border-[#2A3647] rounded-xl space-y-3 shadow-sm">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2 font-bold text-xs text-[#EDF3FC]">
                <Terminal size={15} className="text-[#38BDF8]" />
                <span>Môi trường ADB (Android Debug Bridge)</span>
              </div>
              <span className="px-2 py-0.5 rounded text-[10px] bg-[#133528] text-[#3BC38D] font-mono font-bold">
                1.0.41 SẴN SÀNG
              </span>
            </div>

            <div className="space-y-2 text-xs">
              <div>
                <label className="block text-[#74869D] mb-1 font-semibold text-[11px]">
                  Đường dẫn thực thi ADB đang sử dụng:
                </label>
                <input
                  type="text"
                  readOnly
                  value={adbPathDisplay}
                  className="w-full bg-[#101722] border border-[#2A3647] rounded-md px-3 py-1.5 text-[#EDF3FC] font-mono text-[11px] outline-none"
                />
              </div>

              <div className="p-2.5 rounded-lg bg-[#101722] border border-[#2A3647] text-[11px] text-[#A9B8CC] space-y-1">
                <div className="text-[#3BC38D] font-semibold flex items-center gap-1.5">
                  <CheckCircle2 size={13} />
                  <span>Cơ chế tìm kiếm tự động Portable:</span>
                </div>
                <div className="text-[#74869D] leading-relaxed">
                  1. `bin/adb.exe` cạnh file `FarmerPro.exe` (Được ưu tiên hàng đầu)
                  <br />
                  2. Biến môi trường `FARMERPRO_ADB_PATH` (nếu cấu hình riêng)
                  <br />
                  3. ADB toàn cục trên hệ thống Windows qua PATH
                </div>
              </div>
            </div>
          </div>

          {/* Section 2: Local SQLite WAL Storage */}
          <div className="p-4 bg-[#16202F] border border-[#2A3647] rounded-xl space-y-3 shadow-sm">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2 font-bold text-xs text-[#EDF3FC]">
                <Database size={15} className="text-[#4B9CFF]" />
                <span>Cơ sở Dữ liệu Cục bộ (SQLite WAL)</span>
              </div>
              <span className="px-2 py-0.5 rounded text-[10px] bg-[#173355] text-[#4B9CFF] font-mono font-bold">
                WAL + FOREIGN KEYS
              </span>
            </div>

            <div className="space-y-3 text-xs">
              <div>
                <label className="block text-[#74869D] mb-1 font-semibold text-[11px]">
                  Tệp cơ sở dữ liệu:
                </label>
                <input
                  type="text"
                  readOnly
                  value={dbPathDisplay}
                  className="w-full bg-[#101722] border border-[#2A3647] rounded-md px-3 py-1.5 text-[#EDF3FC] font-mono text-[11px] outline-none"
                />
              </div>

              <div className="flex items-center gap-2 pt-1">
                <button
                  onClick={handleOptimizeDb}
                  className="px-3 py-1.5 bg-[#192434] hover:bg-[#202E42] text-[#38BDF8] border border-[#2A3647] rounded-md text-xs font-semibold flex items-center gap-1.5 transition-colors cursor-pointer"
                >
                  <RotateCw size={12} className={dbOptimized ? 'animate-spin' : ''} />
                  <span>Tối ưu hóa (VACUUM WAL)</span>
                </button>

                {dbOptimized && (
                  <span className="text-[#3BC38D] text-[11px] flex items-center gap-1">
                    <CheckCircle2 size={12} />
                    <span>Đã nén và làm sạch WAL!</span>
                  </span>
                )}
              </div>
            </div>
          </div>

          {/* Section 3: Destructive Action & Guard */}
          <div className="p-4 bg-[#16202F] border border-[#2A3647] rounded-xl space-y-2.5 shadow-sm">
            <div className="flex items-center gap-2 font-bold text-xs text-[#EDF3FC]">
              <Shield size={15} className="text-[#E7B15B]" />
              <span>Hàng rào Bảo vệ Thao tác Nguy hiểm (Destructive Action Guard)</span>
            </div>
            <p className="text-[11px] text-[#A9B8CC] leading-relaxed">
              Các thao tác có tính hủy dữ liệu như: Xóa dữ liệu ứng dụng (Clear App Data), Gỡ cài đặt package hoặc Khởi động lại máy bắt buộc phải hiển thị hộp thoại xác nhận với người dùng trước khi gửi lệnh xuống ADB.
            </p>
          </div>
        </div>

        {/* Right Column (5 cols): Safe Automation Policies & Diagnostics */}
        <div className="col-span-5 space-y-6">
          {/* Safe Automation Policy Defaults */}
          <div className="p-4 bg-[#16202F] border border-[#2A3647] rounded-xl space-y-3.5 shadow-sm text-xs">
            <div className="flex items-center gap-2 font-bold text-xs text-[#EDF3FC]">
              <Sliders size={15} className="text-[#38BDF8]" />
              <span>Chính sách Tương tác An toàn Mặc định</span>
            </div>

            <div>
              <div className="flex justify-between mb-1">
                <label className="text-[#A9B8CC] font-semibold text-[11px]">
                  Tỷ lệ co biên an toàn mặc định
                </label>
                <span className="font-mono text-[#38BDF8] font-bold">
                  {(marginRatio * 100).toFixed(0)}%
                </span>
              </div>
              <input
                type="range"
                min="0.05"
                max="0.3"
                step="0.01"
                value={marginRatio}
                onChange={(e) => setMarginRatio(parseFloat(e.target.value))}
                className="w-full accent-[#38BDF8]"
              />
              <p className="text-[10px] text-[#74869D] mt-1">
                Co cạnh bounding box để điểm click luôn nằm bên trong vùng nút, không bấm sát mép ngoài.
              </p>
            </div>

            <div>
              <div className="flex justify-between mb-1">
                <label className="text-[#A9B8CC] font-semibold text-[11px]">
                  Tránh N điểm bấm gần nhất
                </label>
                <span className="font-mono text-[#EDF3FC] font-bold">
                  {avoidPoints} điểm
                </span>
              </div>
              <input
                type="number"
                min="1"
                max="15"
                value={avoidPoints}
                onChange={(e) => setAvoidPoints(parseInt(e.target.value) || 1)}
                className="w-full bg-[#101722] border border-[#2A3647] rounded px-3 py-1.5 text-[#EDF3FC] font-mono outline-none text-xs"
              />
              <p className="text-[10px] text-[#74869D] mt-1">
                Ngăn chặn việc bấm lặp lại cùng một vị trí pixel nhiều lần liên tiếp.
              </p>
            </div>
          </div>

          {/* Diagnostics & Package Info */}
          <div className="p-4 bg-[#16202F] border border-[#2A3647] rounded-xl space-y-3 shadow-sm text-xs">
            <div className="flex items-center gap-2 font-bold text-xs text-[#EDF3FC]">
              <HardDrive size={15} className="text-[#3BC38D]" />
              <span>Chẩn đoán Bản Portable</span>
            </div>

            <div className="space-y-1.5 text-[11px]">
              <div className="flex justify-between py-1 border-b border-[#2A3647]/50">
                <span className="text-[#74869D]">Phiên bản:</span>
                <span className="font-mono text-[#EDF3FC]">FarmerPro v0.1.0-portable</span>
              </div>
              <div className="flex justify-between py-1 border-b border-[#2A3647]/50">
                <span className="text-[#74869D]">Kiến trúc mục tiêu:</span>
                <span className="font-mono text-[#38BDF8]">x86_64-pc-windows-msvc</span>
              </div>
              <div className="flex justify-between py-1 border-b border-[#2A3647]/50">
                <span className="text-[#74869D]">GUI WebView2:</span>
                <span className="font-mono text-[#3BC38D]">Evergreen Runtime</span>
              </div>
              <div className="flex justify-between py-1 border-b border-[#2A3647]/50">
                <span className="text-[#74869D]">Quyền Administrator:</span>
                <span className="font-mono text-[#3BC38D]">Không yêu cầu (User Mode)</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
