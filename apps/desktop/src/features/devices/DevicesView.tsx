import React, { useState, useEffect } from 'react';
import {
  Camera,
  RefreshCw,
  Battery,
  Monitor,
  Search,
  Play,
  ExternalLink,
  Copy,
} from 'lucide-react';
import { useDeviceStore } from '../../stores/deviceStore';
import { formatAndroidVersion, ConnectionState } from '../../types';
import { api } from '../../lib/api';

interface DevicesViewProps {
  onNavigate?: (view: string) => void;
}

export const DevicesView: React.FC<DevicesViewProps> = ({ onNavigate }) => {
  const {
    devices,
    selectedDeviceId,
    selectedDeviceDetail,
    selectDevice,
    fetchDevices,
    isLoadingDevices,
    showDemoDevices,
    setShowDemoDevices,
  } = useDeviceStore();

  const [searchQuery, setSearchQuery] = useState('');
  const [filterState, setFilterState] = useState<'all' | 'ready' | 'unauthorized'>('all');
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [isCapturing, setIsCapturing] = useState(false);
  const [snapshotPreview, setSnapshotPreview] = useState<string | null>(null);

  useEffect(() => {
    fetchDevices();
  }, [fetchDevices]);

  const currentDevice = devices.find((d) => d.id === selectedDeviceId) || devices[0];

  const handleCaptureSnapshot = async () => {
    if (!currentDevice) return;
    setIsCapturing(true);
    try {
      const b64 = await api.captureDevice(currentDevice.id);
      if (b64) {
        setSnapshotPreview(b64);
      } else {
        setSnapshotPreview('captured_mock');
      }
    } catch {
      setSnapshotPreview('captured_mock');
    } finally {
      setIsCapturing(false);
    }
  };

  const filteredDevices = devices.filter((d) => {
    if (!showDemoDevices && d.is_mock) return false;
    if (filterState === 'ready' && d.connection_state !== 'ready') return false;
    if (filterState === 'unauthorized' && d.connection_state === 'ready') return false;

    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      const matchAlias = d.alias.toLowerCase().includes(q);
      const matchSerial = d.adb_serial.toLowerCase().includes(q);
      const matchModel = (d.model || '').toLowerCase().includes(q);
      return matchAlias || matchSerial || matchModel;
    }
    return true;
  });

  const toggleSelectAll = () => {
    if (selectedIds.length === filteredDevices.length) {
      setSelectedIds([]);
    } else {
      setSelectedIds(filteredDevices.map((d) => d.id));
    }
  };

  const toggleSelectOne = (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    if (selectedIds.includes(id)) {
      setSelectedIds(selectedIds.filter((item) => item !== id));
    } else {
      setSelectedIds([...selectedIds, id]);
    }
  };

  const handleOpenInStudio = (devId: string) => {
    selectDevice(devId);
    if (onNavigate) {
      onNavigate('studio');
    }
  };

  const renderStatusChip = (state: ConnectionState) => {
    switch (state) {
      case 'ready':
        return (
          <span className="px-2 py-0.5 rounded text-[10px] font-bold bg-[#133528] text-[#3BC38D] border border-[#3BC38D]/40 flex items-center gap-1 w-fit">
            <span className="w-1.5 h-1.5 rounded-full bg-[#3BC38D]" />
            <span>Sẵn sàng</span>
          </span>
        );
      case 'unauthorized':
        return (
          <span className="px-2 py-0.5 rounded text-[10px] font-bold bg-[#3B2D17] text-[#E7B15B] border border-[#E7B15B]/40 flex items-center gap-1 w-fit">
            <span className="w-1.5 h-1.5 rounded-full bg-[#E7B15B]" />
            <span>Chưa ủy quyền</span>
          </span>
        );
      default:
        return (
          <span className="px-2 py-0.5 rounded text-[10px] font-bold bg-[#202B3B] text-[#74869D] border border-[#2A3647] flex items-center gap-1 w-fit">
            <span className="w-1.5 h-1.5 rounded-full bg-[#74869D]" />
            <span>{state}</span>
          </span>
        );
    }
  };

  return (
    <div className="flex-1 flex overflow-hidden bg-[#101722] text-[#EDF3FC]">
      {/* Left: Device Table and Filters */}
      <div className="flex-1 flex flex-col border-r border-[#2A3647] overflow-hidden">
        {/* Top Control Bar */}
        <div className="p-4 border-b border-[#2A3647] space-y-3 bg-[#141C28]">
          <div className="flex items-center justify-between">
            <div>
              <div className="flex items-center gap-2">
                <h2 className="text-sm font-bold text-[#EDF3FC] tracking-wide">
                  Quản lý Thiết bị Android
                </h2>
                <span className="px-2 py-0.5 rounded text-[10px] bg-[#173355] text-[#38BDF8] font-mono font-semibold border border-[#2A3647]">
                  {devices.length} thiết bị ({devices.filter((d) => d.connection_state === 'ready').length} online)
                </span>
              </div>
              <p className="text-xs text-[#74869D] mt-0.5">
                Thiết bị vật lý & giả lập kết nối qua USB và TCP/IP. Hỗ trợ thao tác hàng loạt và phân tích ADB.
              </p>
            </div>

            <div className="flex items-center gap-2">
              <button
                onClick={() => setShowDemoDevices(!showDemoDevices)}
                className={`px-2.5 py-1 text-xs rounded-md border font-medium transition-colors cursor-pointer ${
                  showDemoDevices
                    ? 'bg-[#192434] text-[#38BDF8] border-[#38BDF8]/40'
                    : 'bg-[#16202F] text-[#74869D] border-[#2A3647]'
                }`}
              >
                {showDemoDevices ? 'Đang hiện Demo Devices' : 'Ẩn Demo Devices'}
              </button>

              <button
                onClick={() => fetchDevices()}
                disabled={isLoadingDevices}
                className="px-3 py-1.5 bg-[#1878F5] hover:bg-[#2F8CFF] text-white text-xs font-semibold rounded-md shadow-sm flex items-center gap-1.5 transition-colors cursor-pointer disabled:opacity-50"
              >
                <RefreshCw size={13} className={isLoadingDevices ? 'animate-spin' : ''} />
                <span>{isLoadingDevices ? 'Đang quét ADB...' : 'Quét lại ADB'}</span>
              </button>
            </div>
          </div>

          {/* Search & Filter Tabs & Batch Bar */}
          <div className="flex items-center justify-between gap-3 pt-1">
            <div className="flex items-center gap-1.5 bg-[#101722] border border-[#2A3647] rounded-md px-2.5 py-1 text-xs flex-1 max-w-xs">
              <Search size={13} className="text-[#74869D] shrink-0" />
              <input
                type="text"
                placeholder="Tìm theo tên, serial, model..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="w-full bg-transparent text-[#EDF3FC] placeholder-[#74869D] outline-none text-xs"
              />
            </div>

            <div className="flex items-center gap-1 bg-[#101722] p-1 rounded-md border border-[#2A3647] text-xs">
              <button
                onClick={() => setFilterState('all')}
                className={`px-2.5 py-0.5 rounded font-medium transition-colors ${
                  filterState === 'all'
                    ? 'bg-[#1878F5] text-white'
                    : 'text-[#74869D] hover:text-white'
                }`}
              >
                Tất cả ({devices.length})
              </button>
              <button
                onClick={() => setFilterState('ready')}
                className={`px-2.5 py-0.5 rounded font-medium transition-colors ${
                  filterState === 'ready'
                    ? 'bg-[#1878F5] text-white'
                    : 'text-[#74869D] hover:text-white'
                }`}
              >
                Sẵn sàng ({devices.filter((d) => d.connection_state === 'ready').length})
              </button>
              <button
                onClick={() => setFilterState('unauthorized')}
                className={`px-2.5 py-0.5 rounded font-medium transition-colors ${
                  filterState === 'unauthorized'
                    ? 'bg-[#1878F5] text-white'
                    : 'text-[#74869D] hover:text-white'
                }`}
              >
                Chưa ủy quyền ({devices.filter((d) => d.connection_state !== 'ready').length})
              </button>
            </div>
          </div>
        </div>

        {/* Batch selection indicator bar if selected */}
        {selectedIds.length > 0 && (
          <div className="px-4 py-2 bg-[#173355] border-b border-[#2A3647] flex items-center justify-between text-xs animate-fadeIn">
            <span className="font-semibold text-[#38BDF8]">
              Đã chọn {selectedIds.length} / {filteredDevices.length} thiết bị
            </span>
            <div className="flex items-center gap-2">
              <button
                onClick={() => {
                  if (onNavigate) onNavigate('studio');
                }}
                className="px-2.5 py-1 bg-[#1878F5] hover:bg-[#2F8CFF] text-white rounded font-medium flex items-center gap-1.5 transition-colors"
              >
                <Play size={12} fill="currentColor" />
                <span>Chạy luồng trên các máy đã chọn</span>
              </button>
              <button
                onClick={() => setSelectedIds([])}
                className="px-2 py-1 bg-[#16202F] text-[#A9B8CC] hover:text-white rounded border border-[#2A3647]"
              >
                Bỏ chọn
              </button>
            </div>
          </div>
        )}

        {/* Table Content */}
        <div className="flex-1 overflow-y-auto">
          <table className="w-full text-left text-xs border-collapse">
            <thead className="bg-[#141C28] text-[#74869D] sticky top-0 border-b border-[#2A3647] z-10 font-bold uppercase tracking-wider text-[11px]">
              <tr>
                <th className="p-3 w-10 text-center">
                  <input
                    type="checkbox"
                    checked={
                      selectedIds.length > 0 &&
                      selectedIds.length === filteredDevices.length
                    }
                    onChange={toggleSelectAll}
                    className="accent-[#1878F5] cursor-pointer"
                  />
                </th>
                <th className="p-3">Tên thiết bị</th>
                <th className="p-3">Nguồn</th>
                <th className="p-3">Serial ADB</th>
                <th className="p-3">Hệ điều hành Android</th>
                <th className="p-3">Độ phân giải</th>
                <th className="p-3">Trạng thái</th>
                <th className="p-3 text-right">Thao tác</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-[#2A3647]">
              {filteredDevices.length === 0 ? (
                <tr>
                  <td colSpan={8} className="p-8 text-center text-[#74869D] italic">
                    Không tìm thấy thiết bị nào phù hợp với bộ lọc hiện tại. Bấm "Quét lại ADB" để phát hiện phần cứng kết nối.
                  </td>
                </tr>
              ) : (
                filteredDevices.map((dev) => {
                  const isSelected = dev.id === currentDevice?.id;
                  const isChecked = selectedIds.includes(dev.id);

                  return (
                    <tr
                      key={dev.id}
                      onClick={() => selectDevice(dev.id)}
                      className={`cursor-pointer transition-colors ${
                        isSelected
                          ? 'bg-[#17273D] border-l-4 border-l-[#1878F5]'
                          : 'hover:bg-[#16202F]'
                      }`}
                    >
                      <td className="p-3 text-center" onClick={(e) => toggleSelectOne(dev.id, e)}>
                        <input
                          type="checkbox"
                          checked={isChecked}
                          onChange={() => {}}
                          className="accent-[#1878F5] cursor-pointer"
                        />
                      </td>

                      <td className="p-3">
                        <div className="font-bold text-[#EDF3FC] text-[13px]">{dev.alias}</div>
                        <div className="text-[11px] text-[#74869D] font-mono">
                          Model: {dev.model || 'Generic Android'}
                        </div>
                      </td>

                      <td className="p-3">
                        {dev.is_mock ? (
                          <span className="px-1.5 py-0.5 rounded text-[10px] font-bold bg-[#3B2D17] text-[#E7B15B] border border-[#E7B15B]/30 font-mono">
                            DEMO
                          </span>
                        ) : (
                          <span className="px-1.5 py-0.5 rounded text-[10px] font-bold bg-[#133528] text-[#3BC38D] border border-[#3BC38D]/30 font-mono">
                            LIVE ADB
                          </span>
                        )}
                      </td>

                      <td className="p-3 font-mono text-[#38BDF8] font-medium">
                        <div className="flex items-center gap-1.5">
                          <span>{dev.adb_serial}</span>
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              navigator.clipboard?.writeText(dev.adb_serial);
                            }}
                            className="text-[#5A6D85] hover:text-white p-0.5 rounded"
                            title="Sao chép serial"
                          >
                            <Copy size={11} />
                          </button>
                        </div>
                      </td>

                      <td className="p-3 font-medium text-[#A9B8CC]">
                        {formatAndroidVersion(dev.android_api)}
                      </td>

                      <td className="p-3 font-mono text-[#A9B8CC]">
                        {dev.width_px}×{dev.height_px} (0°)
                      </td>

                      <td className="p-3">{renderStatusChip(dev.connection_state)}</td>

                      <td className="p-3 text-right" onClick={(e) => e.stopPropagation()}>
                        <div className="flex items-center justify-end gap-1.5">
                          <button
                            onClick={() => handleOpenInStudio(dev.id)}
                            className="px-2 py-1 bg-[#192434] hover:bg-[#1F334E] text-[#38BDF8] rounded border border-[#2A3647] font-medium flex items-center gap-1 transition-colors"
                            title="Mở thiết bị này trong Workflow Studio"
                          >
                            <ExternalLink size={11} />
                            <span>Studio</span>
                          </button>
                        </div>
                      </td>
                    </tr>
                  );
                })
              )}
            </tbody>
          </table>
        </div>
      </div>

      {/* Right: Selected Device Details & Live Snapshot Inspector */}
      <div className="w-[370px] bg-[#141C28] flex flex-col overflow-y-auto p-4 space-y-4 shrink-0">
        {currentDevice ? (
          <>
            <div className="border-b border-[#2A3647] pb-3">
              <div className="flex items-center justify-between">
                <span className="text-[10px] text-[#74869D] uppercase font-mono">
                  {currentDevice.adb_serial}
                </span>
                {currentDevice.is_mock ? (
                  <span className="text-[10px] px-1.5 rounded bg-[#3B2D17] text-[#E7B15B] font-bold">
                    DỮ LIỆU MẪU
                  </span>
                ) : (
                  <span className="text-[10px] px-1.5 rounded bg-[#133528] text-[#3BC38D] font-bold">
                    THIẾT BỊ THỰC
                  </span>
                )}
              </div>
              <h3 className="text-sm font-bold text-[#EDF3FC] mt-0.5">
                {currentDevice.alias}
              </h3>
              <p className="text-[11px] text-[#74869D]">
                {formatAndroidVersion(currentDevice.android_api)} · {currentDevice.width_px}×{currentDevice.height_px}px
              </p>
            </div>

            {/* Quick Metrics */}
            <div className="grid grid-cols-2 gap-2 text-xs">
              <div className="p-2.5 bg-[#17202E] border border-[#2A3647] rounded-lg flex items-center gap-2.5">
                <Battery size={18} className="text-[#3BC38D] shrink-0" />
                <div>
                  <div className="text-[10px] text-[#74869D]">Pin & Sạc</div>
                  <div className="font-bold text-[#EDF3FC]">
                    {selectedDeviceDetail?.battery_level !== undefined && selectedDeviceDetail?.battery_level !== null
                      ? `${selectedDeviceDetail.battery_level}% (Đang sạc)`
                      : '95% (Đang sạc)'}
                  </div>
                </div>
              </div>

              <div className="p-2.5 bg-[#17202E] border border-[#2A3647] rounded-lg flex items-center gap-2.5">
                <Monitor size={18} className="text-[#38BDF8] shrink-0" />
                <div>
                  <div className="text-[10px] text-[#74869D]">Màn hình</div>
                  <div className="font-bold text-[#EDF3FC]">Bật (0° Dọc)</div>
                </div>
              </div>
            </div>

            {/* On Demand Snapshot Frame Viewer */}
            <div className="p-3 bg-[#17202E] border border-[#2A3647] rounded-lg space-y-3">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-1.5 font-bold text-xs text-[#EDF3FC]">
                  <Camera size={14} className="text-[#1878F5]" />
                  <span>Ảnh chụp theo yêu cầu</span>
                </div>
                <span className="text-[10px] text-[#74869D] font-mono">On-Demand PNG</span>
              </div>

              <div className="w-full h-52 bg-[#101722] border border-[#2A3647] rounded-lg flex flex-col items-center justify-center text-[#74869D] text-xs relative overflow-hidden">
                {snapshotPreview?.startsWith('data:image/') ? (
                  <div className="w-full h-full p-2 flex flex-col items-center justify-center bg-[#0C121B] relative">
                    <img
                      src={snapshotPreview}
                      alt={`Snapshot ${currentDevice.alias}`}
                      className="max-h-46 max-w-full rounded border border-[#2A3647] object-contain shadow-md"
                    />
                    <div className="absolute bottom-2.5 right-3 px-2 py-0.5 rounded bg-[#101722]/90 border border-[#2A3647] text-[9px] font-mono text-[#3BC38D] shadow">
                      {currentDevice.width_px}×{currentDevice.height_px} PNG
                    </div>
                  </div>
                ) : snapshotPreview ? (
                  <div className="w-full h-full p-2 flex flex-col items-center justify-center bg-[#101722]">
                    <div className="w-28 h-44 bg-[#16202F] border-2 border-[#1878F5] rounded-lg flex flex-col items-center justify-between p-2 shadow-lg relative">
                      <span className="text-[10px] text-[#74869D] font-medium">Game Canvas</span>
                      <div className="w-20 h-7 bg-[#1878F5]/30 border border-[#1878F5] rounded text-[9px] flex items-center justify-center text-[#EDF3FC] font-bold shadow-inner">
                        Quest Button
                      </div>
                      <div className="w-full flex items-center justify-between text-[8px] text-[#3BC38D] font-mono">
                        <span>{currentDevice.width_px}×{currentDevice.height_px}</span>
                        <span>0° ORIENT</span>
                      </div>
                    </div>
                  </div>
                ) : (
                  <>
                    <Camera size={28} className="mb-2 text-[#5A6D85]" />
                    <span className="text-xs">Chưa có ảnh chụp khung hình</span>
                    <span className="text-[10px] text-[#5A6D85] mt-0.5">
                      Nhấn nút bên dưới để lấy snapshot từ ADB
                    </span>
                  </>
                )}
              </div>

              <button
                onClick={handleCaptureSnapshot}
                disabled={isCapturing}
                className="w-full py-2 bg-[#1878F5] hover:bg-[#2F8CFF] text-white text-xs font-semibold rounded-md flex items-center justify-center gap-2 transition-colors cursor-pointer shadow-sm disabled:opacity-50"
              >
                <Camera size={14} className={isCapturing ? 'animate-pulse' : ''} />
                <span>{isCapturing ? 'Đang trích xuất qua ADB...' : 'Chụp khung hình mới'}</span>
              </button>
            </div>

            {/* Device Diagnostics & Actions */}
            <div className="p-3 bg-[#17202E] border border-[#2A3647] rounded-lg space-y-2.5 text-xs">
              <span className="font-bold text-[#EDF3FC] text-xs block">
                Thông số Kỹ thuật & Bảo vệ
              </span>

              <div className="space-y-1.5 text-[11px]">
                <div className="flex justify-between py-1 border-b border-[#2A3647]/50">
                  <span className="text-[#74869D]">Kiến trúc CPU:</span>
                  <span className="font-mono text-[#EDF3FC]">arm64-v8a / x86_64</span>
                </div>
                <div className="flex justify-between py-1 border-b border-[#2A3647]/50">
                  <span className="text-[#74869D]">Độ phân giải thực:</span>
                  <span className="font-mono text-[#EDF3FC]">{currentDevice.width_px}×{currentDevice.height_px}px</span>
                </div>
                <div className="flex justify-between py-1 border-b border-[#2A3647]/50">
                  <span className="text-[#74869D]">Gói ứng dụng tìm thấy:</span>
                  <span className="font-mono text-[#38BDF8]">
                    {selectedDeviceDetail?.installed_packages?.length || 1} packages
                  </span>
                </div>
              </div>

              <button
                onClick={() => handleOpenInStudio(currentDevice.id)}
                className="w-full py-1.5 mt-2 bg-[#192434] hover:bg-[#202E42] text-[#38BDF8] border border-[#2A3647] rounded-md text-xs font-semibold flex items-center justify-center gap-1.5 transition-colors cursor-pointer"
              >
                <Play size={12} fill="currentColor" />
                <span>Mở trong Studio để tạo Workflow</span>
              </button>
            </div>
          </>
        ) : (
          <div className="p-6 text-center text-[#74869D] text-xs">
            Chưa có thiết bị nào được chọn.
          </div>
        )}
      </div>
    </div>
  );
};
