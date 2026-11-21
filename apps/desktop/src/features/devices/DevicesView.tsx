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
  Wifi,
  Plus,
  Power,
  RotateCw,
  Trash2,
  Square as StopSquare,
  Radio,
  Layers,
  Sparkles,
  CheckCircle2,
  AlertCircle,
  Cpu,
} from 'lucide-react';
import { useDeviceStore } from '../../stores/deviceStore';
import { formatAndroidVersion, ConnectionState } from '../../types';

interface DevicesViewProps {
  onNavigate?: (view: string) => void;
}

export const DevicesView: React.FC<DevicesViewProps> = ({ onNavigate }) => {
  const {
    devices,
    selectedDeviceId,
    selectedDeviceDetail,
    packages,
    selectedPackageName,
    ldInstances,
    ldPath,
    isLoadingDevices,
    isLoadingPackages,
    isLoadingLd,
    capturedScreenshot,
    selectDevice,
    setSelectedPackageName,
    fetchDevices,
    fetchLdInstances,
    ldAction,
    connectEndpoint,
    disconnectEndpoint,
    scanRange,
    autoDiscover,
    launchApp,
    stopApp,
    captureCurrentDevice,
  } = useDeviceStore();

  const [searchQuery, setSearchQuery] = useState('');
  const [filterState, setFilterState] = useState<'all' | 'ready' | 'unauthorized'>('all');
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [isCapturing, setIsCapturing] = useState(false);

  // Direct connect form
  const [connectIpPort, setConnectIpPort] = useState('127.0.0.1:5555');
  const [isConnecting, setIsConnecting] = useState(false);

  // Scan Range form
  const [showScanModal, setShowScanModal] = useState(false);
  const [scanSubnet, setScanSubnet] = useState('192.168.1');
  const [scanPort, setScanPort] = useState('5555');
  const [scanEmulators, setScanEmulators] = useState(true);
  const [isScanning, setIsScanning] = useState(false);
  const [scanResultNotice, setScanResultNotice] = useState<string | null>(null);

  // Action status message
  const [actionMessage, setActionMessage] = useState<{ text: string; type: 'success' | 'error' | 'info' } | null>(null);

  // Package filter: user apps vs all apps
  const [showAllSystemPackages, setShowAllSystemPackages] = useState(false);

  useEffect(() => {
    fetchDevices();
    fetchLdInstances();
  }, [fetchDevices, fetchLdInstances]);

  const showNotice = (text: string, type: 'success' | 'error' | 'info' = 'info') => {
    setActionMessage({ text, type });
    setTimeout(() => setActionMessage(null), 4500);
  };

  const currentDevice = devices.find((d) => d.id === selectedDeviceId) || devices[0];

  const handleConnectDirect = async (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    if (!connectIpPort.trim()) return;
    setIsConnecting(true);
    try {
      const res = await connectEndpoint(connectIpPort.trim());
      showNotice(res, 'success');
    } catch (err: any) {
      showNotice(`Lỗi kết nối: ${err.message || err}`, 'error');
    } finally {
      setIsConnecting(false);
    }
  };

  const handleDisconnect = async (serial: string) => {
    try {
      const res = await disconnectEndpoint(serial);
      showNotice(res, 'info');
    } catch (err: any) {
      showNotice(`Lỗi ngắt kết nối: ${err.message || err}`, 'error');
    }
  };

  const handleScanRange = async () => {
    setIsScanning(true);
    setScanResultNotice(null);
    try {
      const res = await scanRange(
        scanSubnet.trim() || undefined,
        parseInt(scanPort) || 5555,
        scanEmulators
      );
      setScanResultNotice(
        `Đã quét ${res.scanned_count} cổng. Tìm thấy & kết nối thành công: ${res.connected_endpoints.length} thiết bị (${res.connected_endpoints.join(', ') || 'không có thiết bị mới'})`
      );
      showNotice(`Quét hoàn tất: ${res.connected_endpoints.length} thiết bị mở kết nối`, 'success');
    } catch (err: any) {
      setScanResultNotice(`Lỗi quét dải IP: ${err.message || err}`);
      showNotice(`Lỗi quét: ${err.message || err}`, 'error');
    } finally {
      setIsScanning(false);
    }
  };

  const handleAutoDiscover = async () => {
    setIsScanning(true);
    try {
      await autoDiscover();
      showNotice('Tự động nhận diện thiết bị hoàn tất!', 'success');
    } catch (err: any) {
      showNotice(`Lỗi nhận diện: ${err.message || err}`, 'error');
    } finally {
      setIsScanning(false);
    }
  };

  const handleLaunchSelectedApp = async () => {
    if (!currentDevice) {
      showNotice('Vui lòng chọn thiết bị trước', 'error');
      return;
    }
    if (!selectedPackageName) {
      showNotice('Vui lòng chọn ứng dụng trước', 'error');
      return;
    }
    try {
      const res = await launchApp(currentDevice.id, selectedPackageName);
      showNotice(res, 'success');
    } catch (err: any) {
      showNotice(`Lỗi khởi chạy: ${err.message || err}`, 'error');
    }
  };

  const handleStopSelectedApp = async () => {
    if (!currentDevice) {
      showNotice('Vui lòng chọn thiết bị trước', 'error');
      return;
    }
    if (!selectedPackageName) {
      showNotice('Vui lòng chọn ứng dụng trước', 'error');
      return;
    }
    try {
      const res = await stopApp(currentDevice.id, selectedPackageName);
      showNotice(res, 'info');
    } catch (err: any) {
      showNotice(`Lỗi tắt app: ${err.message || err}`, 'error');
    }
  };

  const handleCaptureSnapshot = async () => {
    if (!currentDevice) return;
    setIsCapturing(true);
    try {
      await captureCurrentDevice();
      showNotice('Đã chụp khung hình thực tế từ thiết bị', 'success');
    } catch (err: any) {
      showNotice(`Lỗi chụp màn hình: ${err.message || err}`, 'error');
    } finally {
      setIsCapturing(false);
    }
  };

  const handleLdAction = async (action: string, idx?: number) => {
    try {
      const res = await ldAction(action, idx);
      showNotice(res, 'success');
    } catch (err: any) {
      showNotice(`Lỗi thao tác LDPlayer: ${err.message || err}`, 'error');
    }
  };

  const filteredDevices = devices.filter((d) => {
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
      {/* Left: Device Table, Connection Bar, LDPlayer Manager */}
      <div className="flex-1 flex flex-col border-r border-[#2A3647] overflow-hidden">
        {/* Top Control Bar */}
        <div className="p-4 border-b border-[#2A3647] space-y-3 bg-[#141C28]">
          <div className="flex items-center justify-between">
            <div>
              <div className="flex items-center gap-2">
                <h2 className="text-sm font-bold text-[#EDF3FC] tracking-wide">
                  Quản lý Thiết bị Android & Giả lập LDPlayer
                </h2>
                <span className="px-2 py-0.5 rounded text-[10px] bg-[#173355] text-[#38BDF8] font-mono font-semibold border border-[#2A3647]">
                  {devices.length} thiết bị thực ({devices.filter((d) => d.connection_state === 'ready').length} online)
                </span>
              </div>
              <p className="text-xs text-[#74869D] mt-0.5">
                Kết nối USB trực tiếp, TCP/IP LAN, tự động nhận diện giả lập và quản trị tiến trình LDPlayer nhanh.
              </p>
            </div>

            <div className="flex items-center gap-2">
              <button
                onClick={handleAutoDiscover}
                disabled={isScanning}
                className="px-2.5 py-1.5 bg-[#16202F] hover:bg-[#202B3B] text-[#38BDF8] text-xs font-semibold rounded-md border border-[#2A3647] flex items-center gap-1.5 transition-colors cursor-pointer disabled:opacity-50"
                title="Tự động quét các cổng giả lập và kết nối ADB"
              >
                <Sparkles size={13} className={isScanning ? 'animate-spin' : ''} />
                <span>Tự động nhận diện</span>
              </button>

              <button
                onClick={() => setShowScanModal(true)}
                className="px-2.5 py-1.5 bg-[#16202F] hover:bg-[#202B3B] text-[#EDF3FC] text-xs font-semibold rounded-md border border-[#2A3647] flex items-center gap-1.5 transition-colors cursor-pointer"
                title="Quét dải IP mạng cục bộ"
              >
                <Wifi size={13} className="text-[#3BC38D]" />
                <span>Quét dải IP</span>
              </button>

              <button
                onClick={() => {
                  fetchDevices();
                  fetchLdInstances();
                }}
                disabled={isLoadingDevices}
                className="px-3 py-1.5 bg-[#1878F5] hover:bg-[#2F8CFF] text-white text-xs font-semibold rounded-md shadow-sm flex items-center gap-1.5 transition-colors cursor-pointer disabled:opacity-50"
              >
                <RefreshCw size={13} className={isLoadingDevices ? 'animate-spin' : ''} />
                <span>{isLoadingDevices ? 'Đang quét...' : 'Quét ADB'}</span>
              </button>
            </div>
          </div>

          {/* Direct IP Connect Bar */}
          <div className="flex items-center gap-2 pt-1 border-t border-[#2A3647]/50">
            <span className="text-xs text-[#74869D] font-medium shrink-0 flex items-center gap-1">
              <Plus size={13} className="text-[#38BDF8]" />
              <span>Kết nối trực tiếp IP:Port</span>
            </span>
            <form onSubmit={handleConnectDirect} className="flex items-center gap-2 flex-1 max-w-md">
              <input
                type="text"
                placeholder="127.0.0.1:5555 hoặc 192.168.1.x:5555"
                value={connectIpPort}
                onChange={(e) => setConnectIpPort(e.target.value)}
                className="bg-[#101722] border border-[#2A3647] rounded-md px-2.5 py-1 text-xs text-[#EDF3FC] placeholder-[#74869D] outline-none flex-1 font-mono"
              />
              <button
                type="submit"
                disabled={isConnecting}
                className="px-3 py-1 bg-[#192434] hover:bg-[#1878F5] text-[#38BDF8] hover:text-white rounded-md text-xs font-semibold border border-[#2A3647] transition-colors cursor-pointer disabled:opacity-50 shrink-0"
              >
                {isConnecting ? 'Đang kết nối...' : 'Kết nối ADB'}
              </button>
            </form>
          </div>

          {/* Action Notification Toast */}
          {actionMessage && (
            <div
              className={`p-2 rounded-md text-xs flex items-center gap-2 animate-fadeIn ${
                actionMessage.type === 'success'
                  ? 'bg-[#133528] text-[#3BC38D] border border-[#3BC38D]/40'
                  : actionMessage.type === 'error'
                  ? 'bg-[#3B1717] text-[#F27883] border border-[#F27883]/40'
                  : 'bg-[#173355] text-[#38BDF8] border border-[#38BDF8]/40'
              }`}
            >
              {actionMessage.type === 'success' && <CheckCircle2 size={13} className="shrink-0" />}
              {actionMessage.type === 'error' && <AlertCircle size={13} className="shrink-0" />}
              <span className="font-medium">{actionMessage.text}</span>
            </div>
          )}

          {/* LDPlayer Quick Management Section */}
          <div className="p-2.5 rounded-lg bg-[#101722] border border-[#2A3647] space-y-2">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <Cpu size={14} className="text-[#38BDF8]" />
                <span className="text-xs font-bold text-[#EDF3FC]">Trình giả lập LDPlayer</span>
                {ldPath ? (
                  <span className="text-[10px] text-[#74869D] font-mono truncate max-w-[280px]">
                    ({ldPath})
                  </span>
                ) : (
                  <span className="text-[10px] text-[#E7B15B] font-mono">
                    (Chưa tìm thấy ldconsole.exe)
                  </span>
                )}
              </div>

              <div className="flex items-center gap-1.5">
                <button
                  onClick={() => handleLdAction('kill_all')}
                  className="px-2 py-0.5 bg-[#3B1717] hover:bg-[#F27883] text-[#F27883] hover:text-white rounded text-[11px] font-bold border border-[#F27883]/30 flex items-center gap-1 transition-colors cursor-pointer"
                  title="Tắt toàn bộ tiến trình dnplayer, ld9boxheadless"
                >
                  <Trash2 size={11} />
                  <span>Kill All LDPlayer</span>
                </button>
              </div>
            </div>

            {isLoadingLd ? (
              <div className="text-[11px] text-[#74869D] italic py-1">Đang kiểm tra LDPlayer...</div>
            ) : ldInstances.length === 0 ? (
              <div className="text-[11px] text-[#74869D] italic py-1">
                Không có máy ảo LDPlayer nào được cấu hình hoặc LDPlayer chưa được cài đặt.
              </div>
            ) : (
              <div className="grid grid-cols-2 gap-2">
                {ldInstances.map((inst) => (
                  <div
                    key={inst.index}
                    className="p-2 rounded bg-[#16202F] border border-[#2A3647] flex items-center justify-between text-xs"
                  >
                    <div className="truncate mr-2">
                      <div className="font-bold text-[#EDF3FC] flex items-center gap-1.5">
                        <span>{inst.name} (#{inst.index})</span>
                        {inst.is_running ? (
                          <span className="w-1.5 h-1.5 rounded-full bg-[#3BC38D]" title="Đang chạy" />
                        ) : (
                          <span className="w-1.5 h-1.5 rounded-full bg-[#74869D]" title="Đã tắt" />
                        )}
                      </div>
                      <div className="text-[10px] text-[#74869D] font-mono">
                        {inst.width}×{inst.height} ({inst.dpi} dpi) · {inst.is_running ? `PID: ${inst.pid}` : 'Đã tắt'}
                      </div>
                    </div>

                    <div className="flex items-center gap-1 shrink-0">
                      <button
                        onClick={() => handleLdAction('launch', inst.index)}
                        className="px-2 py-1 bg-[#192434] hover:bg-[#1878F5] text-[#38BDF8] hover:text-white rounded text-[11px] font-semibold border border-[#2A3647] flex items-center gap-0.5 transition-colors cursor-pointer"
                        title="Mở hoặc kích hoạt cửa sổ LDPlayer lên trên"
                      >
                        <Power size={11} />
                        <span>Mở/Active</span>
                      </button>
                      <button
                        onClick={() => handleLdAction('reboot', inst.index)}
                        className="p-1 bg-[#192434] hover:bg-[#202E42] text-[#A9B8CC] hover:text-white rounded border border-[#2A3647] transition-colors cursor-pointer"
                        title="Khởi động lại LDPlayer này"
                      >
                        <RotateCw size={11} />
                      </button>
                      <button
                        onClick={() => handleLdAction('quit', inst.index)}
                        className="p-1 bg-[#192434] hover:bg-[#3B1717] text-[#F27883] rounded border border-[#2A3647] transition-colors cursor-pointer"
                        title="Tắt nhanh LDPlayer này"
                      >
                        <StopSquare size={11} />
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* Search & Filter Tabs */}
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

        {/* Batch selection indicator bar */}
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
                  <td colSpan={7} className="p-8 text-center text-[#74869D] italic">
                    Chưa phát hiện thiết bị nào kết nối. Bật LDPlayer hoặc cắm điện thoại (bật USB Debugging), sau đó bấm "Tự động nhận diện" hoặc "Quét ADB".
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
                          Model: {dev.model || 'Android Device'}
                        </div>
                      </td>

                      <td className="p-3 font-mono text-[#38BDF8] font-medium">
                        <div className="flex items-center gap-1.5">
                          <span>{dev.adb_serial}</span>
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              navigator.clipboard?.writeText(dev.adb_serial);
                              showNotice(`Đã sao chép: ${dev.adb_serial}`, 'info');
                            }}
                            className="text-[#5A6D85] hover:text-white p-0.5 rounded cursor-pointer"
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
                          {dev.adb_serial.includes(':') && (
                            <button
                              onClick={() => handleDisconnect(dev.adb_serial)}
                              className="px-2 py-1 bg-[#192434] hover:bg-[#3B1717] text-[#F27883] rounded border border-[#2A3647] font-medium transition-colors"
                              title="Ngắt kết nối mạng TCP/IP"
                            >
                              Ngắt
                            </button>
                          )}
                          <button
                            onClick={() => handleOpenInStudio(dev.id)}
                            className="px-2.5 py-1 bg-[#192434] hover:bg-[#1F334E] text-[#38BDF8] rounded border border-[#2A3647] font-medium flex items-center gap-1 transition-colors"
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

      {/* Right: Selected Device Details, App Launch/Stop & Live Snapshot */}
      <div className="w-[390px] bg-[#141C28] flex flex-col overflow-y-auto p-4 space-y-4 shrink-0">
        {currentDevice ? (
          <>
            <div className="border-b border-[#2A3647] pb-3">
              <div className="flex items-center justify-between">
                <span className="text-[10px] text-[#74869D] uppercase font-mono">
                  {currentDevice.adb_serial}
                </span>
                <span className="text-[10px] px-1.5 rounded bg-[#133528] text-[#3BC38D] font-bold">
                  THIẾT BỊ THỰC
                </span>
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
                  <div className="text-[10px] text-[#74869D]">Pin & Nguồn</div>
                  <div className="font-bold text-[#EDF3FC]">
                    {selectedDeviceDetail?.battery_level !== undefined && selectedDeviceDetail?.battery_level !== null
                      ? `${selectedDeviceDetail.battery_level}%`
                      : 'Đang cấp nguồn USB'}
                  </div>
                </div>
              </div>

              <div className="p-2.5 bg-[#17202E] border border-[#2A3647] rounded-lg flex items-center gap-2.5">
                <Monitor size={18} className="text-[#38BDF8] shrink-0" />
                <div>
                  <div className="text-[10px] text-[#74869D]">Màn hình</div>
                  <div className="font-bold text-[#EDF3FC]">
                    {selectedDeviceDetail?.is_screen_on ? 'Đang bật' : 'Chế độ chờ'}
                  </div>
                </div>
              </div>
            </div>

            {/* Application Launcher & Manager */}
            <div className="p-3 bg-[#17202E] border border-[#2A3647] rounded-lg space-y-3">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-1.5 font-bold text-xs text-[#EDF3FC]">
                  <Layers size={14} className="text-[#38BDF8]" />
                  <span>Quản lý & Khởi chạy Ứng dụng</span>
                </div>
                <button
                  onClick={() => setShowAllSystemPackages(!showAllSystemPackages)}
                  className="text-[10px] text-[#74869D] hover:text-[#38BDF8] cursor-pointer"
                >
                  {showAllSystemPackages ? 'Chỉ app cài thêm' : 'Hiện app hệ thống'}
                </button>
              </div>

              {isLoadingPackages ? (
                <div className="text-xs text-[#74869D] italic py-2">
                  Đang tải danh sách packages từ thiết bị...
                </div>
              ) : packages.length === 0 ? (
                <div className="text-xs text-[#74869D] italic py-2">
                  Chưa phát hiện ứng dụng nào. Nhấn kiểm tra thiết bị.
                </div>
              ) : (
                <div className="space-y-2">
                  <label className="text-[11px] text-[#74869D] block">Chọn ứng dụng để điều khiển:</label>
                  <select
                    value={selectedPackageName || ''}
                    onChange={(e) => setSelectedPackageName(e.target.value)}
                    className="w-full bg-[#101722] border border-[#2A3647] rounded-md p-2 text-xs text-[#EDF3FC] outline-none font-medium cursor-pointer"
                  >
                    {packages.map((pkg) => (
                      <option key={pkg.package_name} value={pkg.package_name}>
                        {pkg.is_game ? '🎮 ' : '📱 '}
                        {pkg.display_name}
                      </option>
                    ))}
                  </select>

                  <div className="grid grid-cols-2 gap-2 pt-1">
                    <button
                      onClick={handleLaunchSelectedApp}
                      className="py-1.5 bg-[#1878F5] hover:bg-[#2F8CFF] text-white rounded text-xs font-semibold flex items-center justify-center gap-1.5 transition-colors cursor-pointer shadow-sm"
                    >
                      <Play size={12} fill="currentColor" />
                      <span>Mở App</span>
                    </button>
                    <button
                      onClick={handleStopSelectedApp}
                      className="py-1.5 bg-[#3B1717] hover:bg-[#F27883] text-[#F27883] hover:text-white rounded text-xs font-semibold flex items-center justify-center gap-1.5 border border-[#F27883]/30 transition-colors cursor-pointer"
                    >
                      <StopSquare size={12} fill="currentColor" />
                      <span>Đóng App (Kill)</span>
                    </button>
                  </div>
                </div>
              )}
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
                {capturedScreenshot ? (
                  <div className="w-full h-full p-2 flex flex-col items-center justify-center bg-[#0C121B] relative">
                    <img
                      src={capturedScreenshot}
                      alt={`Snapshot ${currentDevice.alias}`}
                      className="max-h-46 max-w-full rounded border border-[#2A3647] object-contain shadow-md"
                    />
                    <div className="absolute bottom-2.5 right-3 px-2 py-0.5 rounded bg-[#101722]/90 border border-[#2A3647] text-[9px] font-mono text-[#3BC38D] shadow">
                      {currentDevice.width_px}×{currentDevice.height_px} PNG
                    </div>
                  </div>
                ) : (
                  <>
                    <Camera size={28} className="mb-2 text-[#5A6D85]" />
                    <span className="text-xs">Chưa có ảnh chụp khung hình</span>
                    <span className="text-[10px] text-[#5A6D85] mt-0.5">
                      Nhấn nút bên dưới để lấy snapshot thực tế từ ADB
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
                <span>{isCapturing ? 'Đang trích xuất qua ADB...' : 'Chụp khung hình thực tế'}</span>
              </button>
            </div>

            {/* Device Diagnostics & Actions */}
            <div className="p-3 bg-[#17202E] border border-[#2A3647] rounded-lg space-y-2.5 text-xs">
              <span className="font-bold text-[#EDF3FC] text-xs block">
                Thông số Kỹ thuật & Bảo vệ
              </span>

              <div className="space-y-1.5 text-[11px]">
                <div className="flex justify-between py-1 border-b border-[#2A3647]/50">
                  <span className="text-[#74869D]">IP Thiết bị:</span>
                  <span className="font-mono text-[#EDF3FC]">
                    {selectedDeviceDetail?.ip_address || '127.0.0.1'}
                  </span>
                </div>
                <div className="flex justify-between py-1 border-b border-[#2A3647]/50">
                  <span className="text-[#74869D]">Độ phân giải thực:</span>
                  <span className="font-mono text-[#EDF3FC]">
                    {currentDevice.width_px}×{currentDevice.height_px}px
                  </span>
                </div>
                <div className="flex justify-between py-1 border-b border-[#2A3647]/50">
                  <span className="text-[#74869D]">Gói ứng dụng đã quét:</span>
                  <span className="font-mono text-[#38BDF8]">
                    {packages.length} packages
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
            Chưa có thiết bị nào được chọn. Hãy kết nối hoặc mở LDPlayer.
          </div>
        )}
      </div>

      {/* Scan Range Modal */}
      {showScanModal && (
        <div className="fixed inset-0 bg-black/60 backdrop-blur-xs flex items-center justify-center z-50 p-4">
          <div className="w-[460px] bg-[#141C28] border border-[#2A3647] rounded-xl shadow-2xl p-5 space-y-4">
            <div className="flex items-center justify-between border-b border-[#2A3647] pb-3">
              <div className="flex items-center gap-2">
                <Wifi size={16} className="text-[#3BC38D]" />
                <h3 className="font-bold text-sm text-[#EDF3FC]">Quét Thiết bị Mạng & Giả lập</h3>
              </div>
              <button
                onClick={() => setShowScanModal(false)}
                className="text-[#74869D] hover:text-white text-xs cursor-pointer"
              >
                Đóng
              </button>
            </div>

            <div className="space-y-3 text-xs">
              <div>
                <label className="text-[#74869D] block mb-1">Dải mạng Subnet LAN (1..254):</label>
                <input
                  type="text"
                  value={scanSubnet}
                  onChange={(e) => setScanSubnet(e.target.value)}
                  placeholder="192.168.1"
                  className="w-full bg-[#101722] border border-[#2A3647] rounded-md p-2 text-[#EDF3FC] font-mono outline-none"
                />
              </div>

              <div>
                <label className="text-[#74869D] block mb-1">Cổng ADB mục tiêu:</label>
                <input
                  type="number"
                  value={scanPort}
                  onChange={(e) => setScanPort(e.target.value)}
                  placeholder="5555"
                  className="w-full bg-[#101722] border border-[#2A3647] rounded-md p-2 text-[#EDF3FC] font-mono outline-none"
                />
              </div>

              <div className="flex items-center gap-2 pt-1">
                <input
                  type="checkbox"
                  id="chkScanEmu"
                  checked={scanEmulators}
                  onChange={(e) => setScanEmulators(e.target.checked)}
                  className="accent-[#1878F5] cursor-pointer"
                />
                <label htmlFor="chkScanEmu" className="text-[#EDF3FC] cursor-pointer select-none">
                  Quét các cổng giả lập localhost (5555-5585, Nox 62001, Memu 21503, MuMu 7555)
                </label>
              </div>

              {scanResultNotice && (
                <div className="p-2.5 rounded bg-[#101722] border border-[#2A3647] text-[11px] text-[#38BDF8] font-mono leading-relaxed">
                  {scanResultNotice}
                </div>
              )}
            </div>

            <div className="flex items-center justify-end gap-2 pt-2 border-t border-[#2A3647]">
              <button
                onClick={() => setShowScanModal(false)}
                className="px-3 py-1.5 bg-[#16202F] text-[#A9B8CC] hover:text-white rounded text-xs border border-[#2A3647] cursor-pointer"
              >
                Hủy
              </button>
              <button
                onClick={handleScanRange}
                disabled={isScanning}
                className="px-4 py-1.5 bg-[#1878F5] hover:bg-[#2F8CFF] text-white rounded text-xs font-semibold flex items-center gap-1.5 cursor-pointer disabled:opacity-50"
              >
                <Radio size={13} className={isScanning ? 'animate-pulse' : ''} />
                <span>{isScanning ? 'Đang quét song song...' : 'Bắt đầu quét & Tự kết nối'}</span>
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
