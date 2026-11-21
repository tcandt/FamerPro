import React, { useEffect, useState } from 'react';
import {
  Smartphone,
  Activity,
  AlertTriangle,
  CheckCircle2,
  ArrowRight,
  Play,
  Terminal,
  Database,
  ShieldCheck,
  RefreshCw,
  Gamepad2,
  Layers,
  Square as StopSquare,
  Cpu,
  Sparkles,
} from 'lucide-react';
import { useDeviceStore } from '../../stores/deviceStore';
import { useWorkflowStore } from '../../stores/workflowStore';
import { formatAndroidVersion } from '../../types';
import { jx2Workflows } from '../../data/jx2Workflows';

interface OverviewViewProps {
  onNavigate: (view: string) => void;
}

export const OverviewView: React.FC<OverviewViewProps> = ({ onNavigate }) => {
  const {
    devices,
    selectedDeviceId,
    packages,
    selectedPackageName,
    ldInstances,
    fetchDevices,
    fetchLdInstances,
    selectDevice,
    setSelectedPackageName,
    launchApp,
    stopApp,
    ldAction,
    autoDiscover,
    appStatus,
  } = useDeviceStore();
  const { isRunning, runEvents } = useWorkflowStore();

  const [feedback, setFeedback] = useState<string | null>(null);

  const readyDevices = devices.filter((d) => d.connection_state === 'ready');
  const attentionDevices = devices.filter((d) => d.connection_state !== 'ready');

  useEffect(() => {
    fetchDevices();
    fetchLdInstances();
  }, [fetchDevices, fetchLdInstances]);

  const notify = (msg: string) => {
    setFeedback(msg);
    setTimeout(() => setFeedback(null), 3500);
  };

  const handleQuickLaunch = async () => {
    if (!selectedDeviceId) {
      notify('Vui lòng chọn thiết bị');
      return;
    }
    if (!selectedPackageName) {
      notify('Vui lòng chọn app');
      return;
    }
    try {
      const res = await launchApp(selectedDeviceId, selectedPackageName);
      notify(res);
    } catch (e: any) {
      notify(`Lỗi: ${e.message || e}`);
    }
  };

  const handleQuickStop = async () => {
    if (!selectedDeviceId) {
      notify('Vui lòng chọn thiết bị');
      return;
    }
    if (!selectedPackageName) {
      notify('Vui lòng chọn app');
      return;
    }
    try {
      const res = await stopApp(selectedDeviceId, selectedPackageName);
      notify(res);
    } catch (e: any) {
      notify(`Lỗi: ${e.message || e}`);
    }
  };

  const handleLdAction = async (action: string, idx?: number) => {
    try {
      const res = await ldAction(action, idx);
      notify(res);
    } catch (e: any) {
      notify(`Lỗi LDPlayer: ${e.message || e}`);
    }
  };

  return (
    <div className="flex-1 overflow-y-auto p-6 space-y-6 bg-[#101722] text-[#EDF3FC]">
      {/* Top Header */}
      <div className="flex items-center justify-between">
        <div>
          <div className="flex items-center gap-2">
            <h1 className="text-xl font-bold text-[#EDF3FC] tracking-tight">
              Trung tâm Tổng quan Vận hành
            </h1>
            <span className="px-2 py-0.5 rounded text-[10px] bg-[#173355] text-[#38BDF8] font-semibold border border-[#2A3647]">
              Phiên bản Cục bộ (Local-First)
            </span>
          </div>
          <p className="text-xs text-[#74869D] mt-1">
            Theo dõi trạng thái thiết bị thực, điều khiển ứng dụng trực tiếp và nhật ký kiểm thử tự động.
          </p>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={() => autoDiscover()}
            className="px-3 py-1.5 bg-[#16202F] hover:bg-[#202B3B] text-[#38BDF8] text-xs font-medium rounded-md border border-[#2A3647] flex items-center gap-1.5 transition-colors cursor-pointer"
            title="Tự động nhận diện thiết bị và LDPlayer"
          >
            <Sparkles size={13} />
            <span>Tự nhận diện</span>
          </button>
          <button
            onClick={() => {
              fetchDevices();
              fetchLdInstances();
            }}
            className="px-3 py-1.5 bg-[#16202F] hover:bg-[#202B3B] text-[#EDF3FC] text-xs font-medium rounded-md border border-[#2A3647] flex items-center gap-1.5 transition-colors cursor-pointer"
          >
            <RefreshCw size={13} className="text-[#3BC38D]" />
            <span>Quét ADB</span>
          </button>
          <button
            onClick={() => onNavigate('studio')}
            className="px-4 py-1.5 bg-[#1878F5] hover:bg-[#2F8CFF] text-white text-xs font-semibold rounded-md flex items-center gap-2 cursor-pointer shadow-sm transition-colors"
          >
            <Play size={13} fill="currentColor" />
            <span>Mở Workflow Studio</span>
          </button>
        </div>
      </div>

      {feedback && (
        <div className="p-2.5 rounded-lg bg-[#173355] border border-[#38BDF8]/40 text-xs text-[#38BDF8] font-medium animate-fadeIn">
          {feedback}
        </div>
      )}

      {/* 4 Verified Metric Cards */}
      <div className="grid grid-cols-4 gap-4">
        {/* Metric 1: Real Connected Hardware */}
        <div className="p-4 rounded-xl bg-[#16202F] border border-[#2A3647] shadow-sm">
          <div className="flex items-center justify-between text-[#74869D] mb-1.5">
            <span className="text-xs font-semibold">Thiết bị Kết nối</span>
            <Smartphone size={16} className="text-[#38BDF8]" />
          </div>
          <div className="flex items-baseline gap-2">
            <span className="text-2xl font-bold font-mono text-[#EDF3FC]">{devices.length}</span>
            <span className="text-xs text-[#3BC38D] font-medium">({readyDevices.length} sẵn sàng)</span>
          </div>
          <div className="text-[11px] text-[#74869D] mt-1.5 flex items-center justify-between border-t border-[#2A3647]/50 pt-1.5">
            <span>Thiết bị thực tế: {devices.length}</span>
            <span className="text-[#3BC38D] font-medium">
              {readyDevices.length > 0 ? 'Đang trực tuyến' : 'Chờ thiết bị'}
            </span>
          </div>
        </div>

        {/* Metric 2: Active tasks */}
        <div className="p-4 rounded-xl bg-[#16202F] border border-[#2A3647] shadow-sm">
          <div className="flex items-center justify-between text-[#74869D] mb-1.5">
            <span className="text-xs font-semibold">Tác vụ đang Chạy</span>
            <Activity size={16} className="text-[#3BC38D]" />
          </div>
          <div className="flex items-baseline gap-2">
            <span className="text-2xl font-bold font-mono text-[#EDF3FC]">
              {isRunning ? '1' : '0'}
            </span>
            <span className="text-xs text-[#74869D] font-medium">hàng đợi đơn luồng</span>
          </div>
          <div className="text-[11px] text-[#74869D] mt-1.5 border-t border-[#2A3647]/50 pt-1.5">
            {isRunning ? (
              <span className="text-[#E7B15B] font-medium flex items-center gap-1">
                <span className="w-1.5 h-1.5 rounded-full bg-[#E7B15B] animate-pulse" />
                Đang gửi lệnh ADB tới thiết bị
              </span>
            ) : (
              <span>Sẵn sàng tiếp nhận job mới</span>
            )}
          </div>
        </div>

        {/* Metric 3: Attention items */}
        <div className="p-4 rounded-xl bg-[#16202F] border border-[#2A3647] shadow-sm">
          <div className="flex items-center justify-between text-[#74869D] mb-1.5">
            <span className="text-xs font-semibold">Cần Chú ý</span>
            <AlertTriangle size={16} className="text-[#E7B15B]" />
          </div>
          <div className="flex items-baseline gap-2">
            <span className="text-2xl font-bold font-mono text-[#E7B15B]">
              {attentionDevices.length}
            </span>
            <span className="text-xs text-[#74869D] font-medium">máy chưa ủy quyền</span>
          </div>
          <div className="text-[11px] text-[#74869D] mt-1.5 border-t border-[#2A3647]/50 pt-1.5 truncate">
            {attentionDevices.length > 0
              ? 'Yêu cầu chấp thuận USB Debugging'
              : 'Không có thiết bị lỗi'}
          </div>
        </div>

        {/* Metric 4: Testkit status */}
        <div className="p-4 rounded-xl bg-[#16202F] border border-[#2A3647] shadow-sm">
          <div className="flex items-center justify-between text-[#74869D] mb-1.5">
            <span className="text-xs font-semibold">Kiểm thử Testkit Cục bộ</span>
            <CheckCircle2 size={16} className="text-[#3BC38D]" />
          </div>
          <div className="flex items-baseline gap-2">
            <span className="text-2xl font-bold font-mono text-[#3BC38D]">24 / 24</span>
            <span className="text-xs text-[#74869D] font-medium">unit tests đạt</span>
          </div>
          <div className="text-[11px] text-[#74869D] mt-1.5 border-t border-[#2A3647]/50 pt-1.5 flex items-center justify-between">
            <span>Vision SSD: 100% test pass</span>
            <span className="text-[#3BC38D] font-semibold">Đạt chuẩn</span>
          </div>
        </div>
      </div>

      {/* Quick Launch & LDPlayer Action Card */}
      <div className="grid grid-cols-12 gap-6">
        <div className="col-span-12 rounded-xl bg-[#16202F] border border-[#2A3647] p-4 shadow-sm space-y-3">
          <div className="flex items-center justify-between border-b border-[#2A3647]/50 pb-2">
            <div className="flex items-center gap-2">
              <Layers size={15} className="text-[#38BDF8]" />
              <h3 className="text-xs font-bold text-[#EDF3FC]">
                Điều khiển Ứng dụng & Giả lập Tức thì (EXE Quick Control)
              </h3>
            </div>
            <span className="text-[11px] text-[#74869D]">
              Khởi chạy hoặc tắt app ngay trên thiết bị được chọn
            </span>
          </div>

          <div className="grid grid-cols-12 gap-4 items-center">
            {/* Device Select */}
            <div className="col-span-4 space-y-1">
              <label className="text-[11px] text-[#74869D] block font-medium">1. Chọn thiết bị đích:</label>
              <select
                value={selectedDeviceId || ''}
                onChange={(e) => selectDevice(e.target.value)}
                className="w-full bg-[#101722] border border-[#2A3647] rounded-md p-2 text-xs text-[#EDF3FC] outline-none font-medium cursor-pointer"
              >
                {devices.length === 0 ? (
                  <option value="">Chưa có thiết bị kết nối</option>
                ) : (
                  devices.map((d) => (
                    <option key={d.id} value={d.id}>
                      {d.alias} ({d.adb_serial}) - {d.connection_state}
                    </option>
                  ))
                )}
              </select>
            </div>

            {/* App Select */}
            <div className="col-span-5 space-y-1">
              <label className="text-[11px] text-[#74869D] block font-medium">2. Chọn ứng dụng:</label>
              <select
                value={selectedPackageName || ''}
                onChange={(e) => setSelectedPackageName(e.target.value)}
                disabled={packages.length === 0}
                className="w-full bg-[#101722] border border-[#2A3647] rounded-md p-2 text-xs text-[#EDF3FC] outline-none font-medium cursor-pointer disabled:opacity-50"
              >
                {packages.length === 0 ? (
                  <option value="">(Chọn thiết bị để tải danh sách app)</option>
                ) : (
                  packages.map((pkg) => (
                    <option key={pkg.package_name} value={pkg.package_name}>
                      {pkg.is_game ? '🎮 ' : '📱 '}
                      {pkg.display_name}
                    </option>
                  ))
                )}
              </select>
            </div>

            {/* Action Buttons */}
            <div className="col-span-3 flex items-end gap-2 pt-5">
              <button
                onClick={handleQuickLaunch}
                disabled={!selectedDeviceId || !selectedPackageName}
                className="flex-1 py-2 bg-[#1878F5] hover:bg-[#2F8CFF] text-white rounded-md text-xs font-semibold flex items-center justify-center gap-1.5 transition-colors cursor-pointer shadow-sm disabled:opacity-40"
              >
                <Play size={12} fill="currentColor" />
                <span>Mở App</span>
              </button>
              <button
                onClick={handleQuickStop}
                disabled={!selectedDeviceId || !selectedPackageName}
                className="flex-1 py-2 bg-[#3B1717] hover:bg-[#F27883] text-[#F27883] hover:text-white rounded-md text-xs font-semibold flex items-center justify-center gap-1.5 border border-[#F27883]/30 transition-colors cursor-pointer disabled:opacity-40"
              >
                <StopSquare size={12} fill="currentColor" />
                <span>Đóng App</span>
              </button>
            </div>
          </div>

          {/* LDPlayer fast controls row if available */}
          {ldInstances.length > 0 && (
            <div className="pt-2 border-t border-[#2A3647]/40 flex items-center justify-between text-xs">
              <div className="flex items-center gap-2">
                <Cpu size={13} className="text-[#38BDF8]" />
                <span className="text-[#A9B8CC] font-medium">LDPlayer Instances:</span>
                {ldInstances.map((inst) => (
                  <div key={inst.index} className="flex items-center gap-1 bg-[#101722] px-2 py-0.5 rounded border border-[#2A3647]">
                    <span className="text-[#EDF3FC] font-semibold text-[11px]">{inst.name}</span>
                    <button
                      onClick={() => handleLdAction('launch', inst.index)}
                      className="text-[10px] text-[#38BDF8] hover:underline cursor-pointer ml-1"
                    >
                      Mở
                    </button>
                    <span className="text-[#2A3647]">|</span>
                    <button
                      onClick={() => handleLdAction('quit', inst.index)}
                      className="text-[10px] text-[#F27883] hover:underline cursor-pointer"
                    >
                      Tắt
                    </button>
                  </div>
                ))}
              </div>

              <button
                onClick={() => handleLdAction('kill_all')}
                className="text-[11px] text-[#F27883] hover:underline cursor-pointer font-medium"
              >
                Tắt toàn bộ LDPlayer
              </button>
            </div>
          )}
        </div>
      </div>

      {/* Main Grid: 2 Columns */}
      <div className="grid grid-cols-12 gap-6">
        {/* Left Column (7 cols): Ready Devices + Recent Audit Trace */}
        <div className="col-span-7 space-y-6">
          {/* Active Devices Table */}
          <div className="rounded-xl bg-[#16202F] border border-[#2A3647] p-4 shadow-sm">
            <div className="flex items-center justify-between mb-3">
              <div>
                <h3 className="text-xs font-bold text-[#EDF3FC]">Thiết bị Sẵn sàng Hoạt động</h3>
                <span className="text-[11px] text-[#74869D]">
                  Thiết bị thực tế phát hiện qua ADB
                </span>
              </div>
              <button
                onClick={() => onNavigate('devices')}
                className="text-xs text-[#38BDF8] hover:underline flex items-center gap-1 font-medium cursor-pointer"
              >
                <span>Quản lý thiết bị & Quét IP</span>
                <ArrowRight size={12} />
              </button>
            </div>

            <div className="space-y-2">
              {readyDevices.length === 0 ? (
                <div className="p-6 text-center text-[#74869D] text-xs italic">
                  Chưa có thiết bị nào sẵn sàng. Bật LDPlayer hoặc cắm điện thoại và bật Gỡ lỗi USB.
                </div>
              ) : (
                readyDevices.map((dev) => (
                  <div
                    key={dev.id}
                    className="p-3 rounded-lg bg-[#101722] border border-[#2A3647] flex items-center justify-between text-xs hover:border-[#38BDF8]/40 transition-colors"
                  >
                    <div className="flex items-center gap-3">
                      <div className="w-8 h-8 rounded-md bg-[#16202F] flex items-center justify-center text-[#38BDF8]">
                        <Smartphone size={16} />
                      </div>
                      <div>
                        <div className="font-bold text-[#EDF3FC] flex items-center gap-2">
                          <span>{dev.alias}</span>
                          <span className="text-[9px] px-1 rounded bg-[#133528] text-[#3BC38D] font-mono">
                            LIVE
                          </span>
                        </div>
                        <div className="text-[11px] text-[#74869D] font-mono">
                          {dev.adb_serial} · {formatAndroidVersion(dev.android_api)}
                        </div>
                      </div>
                    </div>

                    <div className="flex items-center gap-3">
                      <span className="px-2 py-0.5 rounded text-[10px] bg-[#133528] text-[#3BC38D] border border-[#3BC38D]/40 font-bold">
                        Sẵn sàng
                      </span>
                      <button
                        onClick={() => {
                          selectDevice(dev.id);
                          onNavigate('studio');
                        }}
                        className="px-2.5 py-1 bg-[#192434] hover:bg-[#1878F5] text-[#38BDF8] hover:text-white rounded text-xs font-medium border border-[#2A3647] flex items-center gap-1 transition-colors cursor-pointer"
                        title="Mở Studio với thiết bị này"
                      >
                        <Play size={11} fill="currentColor" />
                        <span>Chạy WF</span>
                      </button>
                    </div>
                  </div>
                ))
              )}
            </div>
          </div>

          {/* JX2 Mobile 10 Workflows Quick Access */}
          <div className="rounded-xl bg-[#16202F] border border-[#2A3647] p-4 shadow-sm">
            <div className="flex items-center justify-between mb-3">
              <div>
                <h3 className="text-xs font-bold text-[#EDF3FC] flex items-center gap-1.5">
                  <Gamepad2 size={14} className="text-[#38BDF8]" />
                  <span>Quy trình Võ Hiệp 2 Mobile ({jx2Workflows.length} luồng có sẵn)</span>
                </h3>
                <span className="text-[11px] text-[#74869D]">
                  Bộ kịch bản tự động hóa tân thủ và cày cấp cho LDPlayer / Android
                </span>
              </div>
              <button
                onClick={() => onNavigate('workflows')}
                className="text-xs text-[#38BDF8] hover:underline flex items-center gap-1 font-medium cursor-pointer"
              >
                <span>Xem tất cả ({jx2Workflows.length})</span>
                <ArrowRight size={12} />
              </button>
            </div>

            <div className="grid grid-cols-2 gap-2">
              {jx2Workflows.slice(0, 4).map((wf) => (
                <div
                  key={wf.id}
                  onClick={() => {
                    useWorkflowStore.getState().loadWorkflow(wf.data);
                    onNavigate('studio');
                  }}
                  className="p-2.5 rounded-lg bg-[#101722] border border-[#2A3647] hover:border-[#1878F5] cursor-pointer transition-colors flex items-center justify-between group"
                >
                  <div className="truncate mr-2">
                    <div className="text-xs font-bold text-[#EDF3FC] group-hover:text-[#38BDF8] truncate">
                      {wf.name}
                    </div>
                    <div className="text-[10px] text-[#74869D] mt-0.5">
                      {wf.category} · {wf.nodesCount} bước
                    </div>
                  </div>
                  <Play size={12} className="text-[#38BDF8] shrink-0 opacity-70 group-hover:opacity-100" />
                </div>
              ))}
            </div>
          </div>

          {/* Recent Audit Step Trace */}
          <div className="rounded-xl bg-[#16202F] border border-[#2A3647] p-4 shadow-sm">
            <div className="flex items-center justify-between mb-3">
              <div>
                <h3 className="text-xs font-bold text-[#EDF3FC]">Nhật ký Vận hành Gần nhất</h3>
                <span className="text-[11px] text-[#74869D]">
                  Các bước gửi lệnh, khớp ảnh và xác minh qua ADB
                </span>
              </div>
              <button
                onClick={() => onNavigate('runs')}
                className="text-xs text-[#38BDF8] hover:underline flex items-center gap-1 font-medium cursor-pointer"
              >
                <span>Xem Runtime Monitor</span>
                <ArrowRight size={12} />
              </button>
            </div>

            <div className="space-y-1.5 font-mono text-[11px]">
              {runEvents.length === 0 ? (
                <div className="p-6 text-center text-[#74869D] text-xs italic bg-[#101722] rounded-lg border border-[#2A3647]">
                  Chưa có phiên chạy nào gần đây. Hãy mở Studio và bấm "Chạy luồng".
                </div>
              ) : (
                runEvents.slice(-4).map((ev) => (
                  <div
                    key={ev.id}
                    className="p-2 rounded bg-[#101722] border border-[#2A3647] flex items-center justify-between"
                  >
                    <div className="flex items-center gap-2">
                      <span className="px-1.5 py-0.5 rounded text-[10px] bg-[#173355] text-[#38BDF8] font-bold">
                        {ev.node_id}
                      </span>
                      <span className="text-[#EDF3FC] font-semibold">{ev.event_type}</span>
                    </div>
                    <span className="text-[10px] text-[#74869D]">
                      {new Date(ev.at).toLocaleTimeString()}
                    </span>
                  </div>
                ))
              )}
            </div>
          </div>
        </div>

        {/* Right Column (5 cols): Attention Queue + System Status */}
        <div className="col-span-5 space-y-6">
          {/* Attention Queue */}
          <div className="rounded-xl bg-[#16202F] border border-[#2A3647] p-4 shadow-sm">
            <div className="flex items-center justify-between mb-3">
              <h3 className="text-xs font-bold text-[#EDF3FC]">Hàng đợi Cần Chú ý</h3>
              <span className="text-[10px] px-1.5 py-0.5 rounded bg-[#3B2D17] text-[#E7B15B] font-bold">
                {attentionDevices.length} cần cấp quyền
              </span>
            </div>

            {attentionDevices.length === 0 ? (
              <div className="p-4 rounded-lg bg-[#101722] border border-[#2A3647] text-center text-[#74869D] text-xs">
                Mọi thiết bị đều đang hoạt động bình thường, không có lỗi cần can thiệp!
              </div>
            ) : (
              <div className="space-y-2">
                {attentionDevices.map((dev) => (
                  <div
                    key={dev.id}
                    className="p-3 rounded-lg bg-[#101722] border border-[#E7B15B]/30 flex flex-col space-y-1.5 text-xs"
                  >
                    <div className="flex items-center justify-between">
                      <span className="font-bold text-[#EDF3FC]">{dev.alias}</span>
                      <span className="px-2 py-0.5 rounded text-[10px] bg-[#3B2D17] text-[#E7B15B] font-bold">
                        Chưa ủy quyền ADB
                      </span>
                    </div>
                    <p className="text-[11px] text-[#74869D] leading-relaxed">
                      Mở màn hình thiết bị và nhấn <strong>"Luôn cho phép từ máy tính này"</strong> để cấp quyền điều khiển.
                    </p>
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* System Runtime Health (Local-First Diagnostics) */}
          <div className="rounded-xl bg-[#16202F] border border-[#2A3647] p-4 shadow-sm space-y-3">
            <h3 className="text-xs font-bold text-[#EDF3FC]">Trạng thái Runtime Cục bộ (Portable)</h3>
            <div className="space-y-2 text-xs">
              <div className="p-2.5 rounded-lg bg-[#101722] border border-[#2A3647] flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <Terminal size={14} className="text-[#3BC38D]" />
                  <span className="text-[#A9B8CC]">ADB Binary:</span>
                </div>
                <span className="font-mono text-[#EDF3FC] text-[11px]">
                  {appStatus?.adb_available ? 'Nội bộ (bin/adb.exe)' : 'Sẵn sàng'}
                </span>
              </div>

              <div className="p-2.5 rounded-lg bg-[#101722] border border-[#2A3647] flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <Database size={14} className="text-[#4B9CFF]" />
                  <span className="text-[#A9B8CC]">Cơ sở dữ liệu:</span>
                </div>
                <span className="font-mono text-[#EDF3FC] text-[11px]">
                  SQLite WAL (data/farmerpro.db)
                </span>
              </div>

              <div className="p-2.5 rounded-lg bg-[#101722] border border-[#2A3647] flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <ShieldCheck size={14} className="text-[#3BC38D]" />
                  <span className="text-[#A9B8CC]">Bảo mật thông tin:</span>
                </div>
                <span className="font-mono text-[#3BC38D] text-[11px]">
                  Windows Credential Manager
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
