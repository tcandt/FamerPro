import React, { useState } from 'react';
import { Camera, RefreshCw, Battery, Monitor } from 'lucide-react';
import { useDeviceStore } from '../../stores/deviceStore';

export const DevicesView: React.FC = () => {
  const { devices, selectedDeviceId, selectDevice } = useDeviceStore();
  const [isCapturing, setIsCapturing] = useState(false);
  const [snapshotPreview, setSnapshotPreview] = useState<string | null>(null);

  const currentDevice = devices.find((d) => d.id === selectedDeviceId) || devices[0];

  const handleCaptureSnapshot = () => {
    setIsCapturing(true);
    setTimeout(() => {
      setIsCapturing(false);
      setSnapshotPreview('captured');
    }, 600);
  };

  return (
    <div className="flex-1 flex overflow-hidden bg-[#101722] text-[#EDF3FC]">
      <div className="flex-1 flex flex-col border-r border-[#2A3647] overflow-hidden">
        <div className="p-4 border-b border-[#2A3647] flex items-center justify-between">
          <div>
            <h2 className="text-sm font-semibold text-[#EDF3FC]">Quản lý Thiết bị Android</h2>
            <p className="text-xs text-[#74869D]">
              Danh sách điện thoại kết nối qua USB và TCP/IP. Nhấp vào dòng để kiểm tra chi tiết.
            </p>
          </div>
          <button className="px-3 py-1.5 bg-[#202B3B] hover:bg-[#2A3647] text-[#EDF3FC] text-xs font-medium rounded border border-[#2A3647] flex items-center gap-1.5">
            <RefreshCw size={13} />
            <span>Quét lại ADB</span>
          </button>
        </div>

        <div className="flex-1 overflow-y-auto">
          <table className="w-full text-left text-xs border-collapse">
            <thead className="bg-[#161E29] text-[#74869D] sticky top-0 border-b border-[#2A3647]">
              <tr>
                <th className="p-3 font-medium">Tên định danh</th>
                <th className="p-3 font-medium">Serial ADB</th>
                <th className="p-3 font-medium">Hệ điều hành</th>
                <th className="p-3 font-medium">Độ phân giải</th>
                <th className="p-3 font-medium">Trạng thái</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-[#2A3647]">
              {devices.map((dev) => {
                const isSelected = dev.id === currentDevice?.id;
                return (
                  <tr
                    key={dev.id}
                    onClick={() => selectDevice(dev.id)}
                    className={`cursor-pointer transition-colors ${
                      isSelected ? 'bg-[#173355]' : 'hover:bg-[#192230]'
                    }`}
                  >
                    <td className="p-3 font-medium text-[#EDF3FC]">{dev.alias}</td>
                    <td className="p-3 font-mono text-[#A9B8CC]">{dev.adb_serial}</td>
                    <td className="p-3 text-[#A9B8CC]">Android {dev.android_api || 33}</td>
                    <td className="p-3 font-mono text-[#A9B8CC]">
                      {dev.width_px}×{dev.height_px}
                    </td>
                    <td className="p-3">
                      <span
                        className={`px-2 py-0.5 rounded text-[10px] font-medium border ${
                          dev.connection_state === 'ready'
                            ? 'bg-[#133528] text-[#3BC38D] border-[#3BC38D]/30'
                            : 'bg-[#3B2D17] text-[#E7B15B] border-[#E7B15B]/30'
                        }`}
                      >
                        {dev.connection_state === 'ready' ? 'Sẵn sàng' : 'Chưa ủy quyền'}
                      </span>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </div>

      <div className="w-[360px] bg-[#161E29] flex flex-col overflow-y-auto p-4 space-y-4">
        {currentDevice ? (
          <>
            <div>
              <span className="text-[10px] text-[#74869D] uppercase font-mono">
                {currentDevice.adb_serial}
              </span>
              <h3 className="text-sm font-semibold text-[#EDF3FC]">{currentDevice.alias}</h3>
            </div>

            <div className="grid grid-cols-2 gap-2 text-xs">
              <div className="p-2.5 bg-[#192230] border border-[#2A3647] rounded flex items-center gap-2">
                <Battery size={16} className="text-[#3BC38D]" />
                <div>
                  <div className="text-[10px] text-[#74869D]">Pin</div>
                  <div className="font-semibold text-[#EDF3FC]">95% (Đang sạc)</div>
                </div>
              </div>

              <div className="p-2.5 bg-[#192230] border border-[#2A3647] rounded flex items-center gap-2">
                <Monitor size={16} className="text-[#4B9CFF]" />
                <div>
                  <div className="text-[10px] text-[#74869D]">Màn hình</div>
                  <div className="font-semibold text-[#EDF3FC]">Bật (0° Dọc)</div>
                </div>
              </div>
            </div>

            <div className="p-3 bg-[#192230] border border-[#2A3647] rounded space-y-3">
              <div className="flex items-center justify-between">
                <span className="text-xs font-semibold text-[#EDF3FC]">Ảnh chụp theo yêu cầu</span>
                <span className="text-[10px] text-[#74869D]">PNG tĩnh</span>
              </div>

              <div className="w-full h-48 bg-[#101722] border border-[#2A3647] rounded flex flex-col items-center justify-center text-[#74869D] text-xs relative overflow-hidden">
                {snapshotPreview ? (
                  <div className="w-full h-full p-2 flex flex-col items-center justify-center bg-[#101722]">
                    <div className="w-24 h-40 bg-[#161E29] border-2 border-[#1878F5] rounded flex flex-col items-center justify-between p-2">
                      <span className="text-[9px] text-[#74869D]">Game Home</span>
                      <div className="w-16 h-6 bg-[#1878F5]/30 border border-[#1878F5] rounded text-[8px] flex items-center justify-center text-[#EDF3FC]">
                        Nhiệm vụ
                      </div>
                      <span className="text-[8px] text-[#3BC38D]">1080×2400</span>
                    </div>
                  </div>
                ) : (
                  <>
                    <Camera size={24} className="mb-2 text-[#74869D]" />
                    <span>Chưa có ảnh chụp</span>
                  </>
                )}
              </div>

              <button
                onClick={handleCaptureSnapshot}
                disabled={isCapturing}
                className="w-full py-2 bg-[#1878F5] hover:bg-[#2F8CFF] text-white text-xs font-semibold rounded flex items-center justify-center gap-2 transition-colors cursor-pointer"
              >
                <Camera size={14} />
                <span>{isCapturing ? 'Đang chụp từ ADB...' : 'Chụp khung hình mới'}</span>
              </button>
            </div>
          </>
        ) : (
          <div className="p-6 text-center text-[#74869D] text-xs">Chưa có thiết bị nào được chọn.</div>
        )}
      </div>
    </div>
  );
};
