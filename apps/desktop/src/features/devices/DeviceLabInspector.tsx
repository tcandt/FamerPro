import React, { useEffect, useState } from 'react';
import {
  Activity,
  Battery,
  Camera,
  CheckCircle2,
  Crop,
  Crosshair,
  Layers,
  Monitor,
  Network,
  Play,
  Plus,
  RefreshCw,
  ShieldAlert,
  Square as StopSquare,
  Trash2,
} from 'lucide-react';
import { ElementInspectorView } from './ElementInspectorView';
import { api, isTauri } from '../../lib/api';
import { useDeviceStore } from '../../stores/deviceStore';
import { useProjectStore } from '../../stores/projectStore';
import {
  AdbPortMappingsOverview,
  DeviceHealthReport,
  DeviceSummary,
  formatAndroidVersion,
  TemplateMatchPreviewReport,
} from '../../types';

interface DeviceLabInspectorProps {
  currentDevice?: DeviceSummary;
  onNotice: (text: string, type?: 'success' | 'error' | 'info') => void;
  onOpenInStudio: (deviceId: string) => void;
}

function generateTestkitFrameBase64(): { dataUrl: string; rawB64: string } {
  const canvas = document.createElement('canvas');
  canvas.width = 540;
  canvas.height = 960;
  const ctx = canvas.getContext('2d');
  if (ctx) {
    ctx.fillStyle = '#101722';
    ctx.fillRect(0, 0, 540, 960);
    ctx.strokeStyle = '#2A3647';
    ctx.lineWidth = 2;
    ctx.strokeRect(20, 20, 500, 920);
    ctx.fillStyle = '#1878F5';
    ctx.fillRect(100, 200, 120, 60);
    ctx.fillStyle = '#EDF3FC';
    ctx.font = 'bold 14px sans-serif';
    ctx.fillText('TESTKIT SAMPLE', 105, 235);
    ctx.fillStyle = '#74869D';
    ctx.font = '12px monospace';
    ctx.fillText('SYNTHETIC 540x960 · NOT LIVE ADB', 140, 890);
  }
  const dataUrl = canvas.toDataURL('image/png');
  return { dataUrl, rawB64: dataUrl.split(',')[1] || '' };
}

export const DeviceLabInspector: React.FC<DeviceLabInspectorProps> = ({
  currentDevice,
  onNotice,
  onOpenInStudio,
}) => {
  const {
    selectedDeviceDetail,
    packages,
    selectedPackageName,
    isLoadingPackages,
    capturedScreenshot,
    setCapturedScreenshot,
    setSelectedPackageName,
    launchApp,
    stopApp,
    loadPackages,
    captureCurrentDevice,
  } = useDeviceStore();
  const { activeProjectId, activeProjectDetail, refreshActiveProject } = useProjectStore();

  const [tab, setTab] = useState<'inspector' | 'health' | 'vision' | 'ports'>('inspector');
  const [showAllSystemPackages, setShowAllSystemPackages] = useState(false);

  // Health & Reconnect state
  const [probeSerial, setProbeSerial] = useState('127.0.0.1:5555');
  const [healthReport, setHealthReport] = useState<DeviceHealthReport | null>(null);
  const [probing, setProbing] = useState(false);

  // Snapshot / ROI / Template state
  const [isCapturing, setIsCapturing] = useState(false);
  const [frameSourceKind, setFrameSourceKind] = useState<'LIVE' | 'TESTKIT'>('LIVE');
  const [frameDims, setFrameDims] = useState<{ w: number; h: number }>({ w: 1080, h: 1920 });
  const [tplName, setTplName] = useState('Quest Button');
  const [tplRelPath, setTplRelPath] = useState('templates/quest_btn.png');
  const [roiX, setRoiX] = useState(100);
  const [roiY, setRoiY] = useState(200);
  const [roiW, setRoiW] = useState(120);
  const [roiH, setRoiH] = useState(60);
  const [savingTpl, setSavingTpl] = useState(false);
  const [matchPreview, setMatchPreview] = useState<TemplateMatchPreviewReport | null>(null);

  // Port mapping state
  const [portsOverview, setPortsOverview] = useState<AdbPortMappingsOverview | null>(null);
  const [fwdLocal, setFwdLocal] = useState('tcp:18800');
  const [fwdRemote, setFwdRemote] = useState('tcp:18800');
  const [revRemote, setRevRemote] = useState('tcp:19900');
  const [revLocal, setRevLocal] = useState('tcp:19900');
  const [loadingPorts, setLoadingPorts] = useState(false);

  useEffect(() => {
    if (currentDevice) {
      setProbeSerial(currentDevice.adb_serial);
      setFrameDims({ w: currentDevice.width_px, h: currentDevice.height_px });
    }
  }, [currentDevice?.id]);

  const targetSerial = currentDevice?.adb_serial || probeSerial.trim();

  const handleProbeHealth = async () => {
    if (!targetSerial) return;
    setProbing(true);
    try {
      const rep = await api.studioProbeDeviceHealth(targetSerial);
      setHealthReport(rep);
      onNotice(
        rep.reachable
          ? `Health OK (${rep.latencyMs}ms): ${rep.widthPx}×${rep.heightPx}`
          : `Device ${rep.connectionState.toUpperCase()}: ${rep.error || 'Offline'}`,
        rep.reachable ? 'success' : 'error'
      );
    } catch (err: any) {
      onNotice(`Probe Health thất bại: ${err?.message || err}`, 'error');
    } finally {
      setProbing(false);
    }
  };

  const handleReconnect = async () => {
    if (!targetSerial) return;
    setProbing(true);
    try {
      const rep = await api.studioReconnectDevice(targetSerial);
      setHealthReport(rep);
      onNotice(
        rep.reachable ? `Đã kết nối lại ${rep.serial}` : `Reconnect chưa thấy thiết bị: ${rep.error}`,
        rep.reachable ? 'success' : 'info'
      );
    } catch (err: any) {
      onNotice(`Reconnect lỗi: ${err?.message || err}`, 'error');
    } finally {
      setProbing(false);
    }
  };

  const handleCaptureLive = async () => {
    if (!currentDevice) return;
    setIsCapturing(true);
    try {
      await captureCurrentDevice();
      setFrameSourceKind(currentDevice.is_mock ? 'TESTKIT' : 'LIVE');
      setFrameDims({ w: currentDevice.width_px, h: currentDevice.height_px });
      setMatchPreview(null);
      onNotice('Đã chụp khung hình từ ADB.', 'success');
    } catch (err: any) {
      onNotice(`Lỗi chụp màn hình: ${err?.message || err}`, 'error');
    } finally {
      setIsCapturing(false);
    }
  };

  const handleLoadTestkitSample = () => {
    const { dataUrl } = generateTestkitFrameBase64();
    setCapturedScreenshot(dataUrl);
    setFrameSourceKind('TESTKIT');
    setFrameDims({ w: 540, h: 960 });
    setRoiX(100);
    setRoiY(200);
    setRoiW(120);
    setRoiH(60);
    setMatchPreview(null);
    onNotice('Đã nạp khung hình mẫu TESTKIT (540×960) — không phải Live ADB.', 'info');
  };

  const handleCropAndSaveTemplate = async () => {
    if (!activeProjectId) {
      onNotice('Hãy chọn Active Project trên Header trước khi lưu Template.', 'error');
      return;
    }
    if (!capturedScreenshot) {
      onNotice('Chưa có ảnh khung hình để cắt ROI.', 'error');
      return;
    }
    const rawB64 = capturedScreenshot.includes(',')
      ? capturedScreenshot.split(',')[1]
      : capturedScreenshot;
    setSavingTpl(true);
    try {
      const rec = await api.studioCropAndSaveTemplate({
        projectId: activeProjectId,
        name: tplName.trim() || 'ROI Template',
        relPath: tplRelPath.trim(),
        framePngBase64: rawB64,
        roi: { x: roiX, y: roiY, width: roiW, height: roiH },
        defaultConfidence: 0.88,
      });
      await refreshActiveProject();
      onNotice(`Đã lưu ROI ${rec.widthPx}×${rec.heightPx} vào Project (${rec.relPath}).`, 'success');
    } catch (err: any) {
      onNotice(`Lỗi cắt/lưu ROI: ${err?.message || err}`, 'error');
    } finally {
      setSavingTpl(false);
    }
  };

  const handleTestMatch = async (templateId: string) => {
    if (!activeProjectId || !capturedScreenshot) {
      onNotice('Cần có Active Project và khung hình hiện tại để Test Match.', 'error');
      return;
    }
    const rawB64 = capturedScreenshot.includes(',')
      ? capturedScreenshot.split(',')[1]
      : capturedScreenshot;
    try {
      const rep = await api.studioTestProjectTemplateMatch({
        projectId: activeProjectId,
        templateId,
        framePngBase64: rawB64,
        frameWidth: frameDims.w,
        frameHeight: frameDims.h,
        minConfidence: 0.85,
      });
      setMatchPreview(rep);
      onNotice(
        rep.matched
          ? `Khớp mẫu conf=${(rep.confidence ?? 0).toFixed(2)}`
          : `Không khớp mẫu: ${rep.error || 'confidence thấp'}`,
        rep.matched ? 'success' : 'error'
      );
    } catch (err: any) {
      onNotice(`Test Match lỗi: ${err?.message || err}`, 'error');
    }
  };

  const handleRefreshPorts = async () => {
    if (!targetSerial) return;
    setLoadingPorts(true);
    try {
      const res = await api.studioListDevicePorts(targetSerial);
      setPortsOverview(res);
    } catch (err: any) {
      setPortsOverview(null);
      onNotice(`ADB port list chưa khả dụng: ${err?.message || err}`, 'error');
    } finally {
      setLoadingPorts(false);
    }
  };

  const handleAddForward = async () => {
    if (!targetSerial) return;
    try {
      await api.studioAddDeviceForward(targetSerial, fwdLocal.trim(), fwdRemote.trim());
      await handleRefreshPorts();
      onNotice(`Đã tạo adb forward ${fwdLocal} -> ${fwdRemote}`, 'success');
    } catch (err: any) {
      onNotice(`Lỗi adb forward: ${err?.message || err}`, 'error');
    }
  };

  const handleAddReverse = async () => {
    if (!targetSerial) return;
    try {
      await api.studioAddDeviceReverse(targetSerial, revRemote.trim(), revLocal.trim());
      await handleRefreshPorts();
      onNotice(`Đã tạo adb reverse ${revRemote} -> ${revLocal}`, 'success');
    } catch (err: any) {
      onNotice(`Lỗi adb reverse: ${err?.message || err}`, 'error');
    }
  };

  const sourceBadge = !isTauri()
    ? { label: 'WEB PREVIEW', cls: 'fp-badge-testkit' }
    : !currentDevice
    ? { label: 'OFFLINE', cls: 'fp-badge-offline' }
    : currentDevice.is_mock
    ? { label: 'TESTKIT', cls: 'fp-badge-testkit' }
    : currentDevice.connection_state === 'ready'
    ? { label: 'LIVE', cls: 'fp-badge-live' }
    : currentDevice.connection_state === 'unauthorized' || currentDevice.connection_state === 'blocked'
    ? { label: 'ERROR', cls: 'fp-badge-error' }
    : { label: 'OFFLINE', cls: 'fp-badge-offline' };

  const projectTemplates = activeProjectDetail?.templates || [];

  return (
    <div className="w-[420px] bg-[#141C28] flex flex-col overflow-hidden shrink-0 border-l border-[#2A3647]">
      {/* Inspector Header & Truthful Source Badge */}
      <div className="p-3.5 border-b border-[#2A3647] space-y-2">
        <div className="flex items-center justify-between gap-2">
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <span className="text-[11px] font-mono text-[#74869D] truncate">
                {currentDevice ? currentDevice.adb_serial : probeSerial || 'no-device'}
              </span>
              <span className={`text-[10px] font-bold px-1.5 py-0.2 rounded ${sourceBadge.cls}`}>
                {sourceBadge.label}
              </span>
            </div>
            <h3 className="text-sm font-bold text-[#EDF3FC] truncate mt-0.5">
              {currentDevice ? currentDevice.alias : 'Chưa chọn thiết bị ADB'}
            </h3>
            {currentDevice && (
              <p className="text-[11px] text-[#74869D]">
                {formatAndroidVersion(currentDevice.android_api)} · {currentDevice.width_px}×
                {currentDevice.height_px}px
              </p>
            )}
          </div>
          {currentDevice && (
            <button
              onClick={() => onOpenInStudio(currentDevice.id)}
              className="px-2.5 py-1.5 rounded bg-[#192434] hover:bg-[#202E42] border border-[#2A3647] text-[11px] font-semibold text-[#38BDF8] cursor-pointer shrink-0"
            >
              Mở Studio
            </button>
          )}
        </div>

        {/* Inspector Sub-tabs */}
        <div className="grid grid-cols-4 gap-1 bg-[#101722] p-1 rounded-md border border-[#2A3647] text-[11px]">
          <button
            onClick={() => setTab('inspector')}
            className={`py-1 rounded font-medium flex items-center justify-center gap-1 cursor-pointer transition-colors ${
              tab === 'inspector' ? 'bg-[#1878F5] text-white shadow' : 'text-[#A9B8CC] hover:text-white'
            }`}
          >
            <Crosshair size={12} />
            <span>Element Inspector</span>
          </button>
          <button
            onClick={() => setTab('health')}
            className={`py-1 rounded font-medium flex items-center justify-center gap-1 cursor-pointer transition-colors ${
              tab === 'health' ? 'bg-[#173355] text-white' : 'text-[#A9B8CC] hover:text-white'
            }`}
          >
            <Activity size={12} />
            <span>Health & App</span>
          </button>
          <button
            onClick={() => setTab('vision')}
            className={`py-1 rounded font-medium flex items-center justify-center gap-1 cursor-pointer transition-colors ${
              tab === 'vision' ? 'bg-[#173355] text-white' : 'text-[#A9B8CC] hover:text-white'
            }`}
          >
            <Crop size={12} />
            <span>ROI &amp; Vision</span>
          </button>
          <button
            onClick={() => setTab('ports')}
            className={`py-1 rounded font-medium flex items-center justify-center gap-1 cursor-pointer transition-colors ${
              tab === 'ports' ? 'bg-[#173355] text-white' : 'text-[#A9B8CC] hover:text-white'
            }`}
          >
            <Network size={12} />
            <span>Port Map</span>
          </button>
        </div>
      </div>

      {/* Tab Body */}
      <div className="flex-1 overflow-y-auto p-3.5 space-y-3.5 text-xs">
        {tab === 'inspector' && (
          <ElementInspectorView
            serial={currentDevice?.adb_serial || probeSerial}
            onNotice={onNotice}
            onAddToWorkflow={(_sel, name) => {
              onNotice(`Selector for "${name}" sent to Workflow Studio`, 'success');
              if (currentDevice) {
                onOpenInStudio(currentDevice.adb_serial);
              }
            }}
          />
        )}

        {tab === 'health' && (
          <>
            {/* Live Health Probe & Reconnect (P3 IPC) */}
            <div className="p-3 bg-[#17202E] border border-[#2A3647] rounded-lg space-y-2.5">
              <div className="flex items-center justify-between">
                <span className="font-bold text-[#EDF3FC]">Kiểm tra Sức khỏe ADB (P3 IPC)</span>
                <span className="text-[10px] font-mono text-[#74869D]">probe_health</span>
              </div>
              <div className="flex gap-1.5">
                <input
                  value={probeSerial}
                  onChange={(e) => setProbeSerial(e.target.value)}
                  placeholder="Serial hoặc 127.0.0.1:5555"
                  className="flex-1 px-2.5 py-1.5 rounded bg-[#101722] border border-[#2A3647] font-mono text-[11px] text-[#EDF3FC] outline-none"
                />
                <button
                  onClick={handleProbeHealth}
                  disabled={probing}
                  className="px-2.5 py-1.5 rounded bg-[#1878F5] hover:bg-[#2F8CFF] text-white font-semibold cursor-pointer disabled:opacity-50"
                >
                  Probe
                </button>
                <button
                  onClick={handleReconnect}
                  disabled={probing}
                  title="Reconnect endpoint"
                  className="p-1.5 rounded bg-[#202B3B] hover:bg-[#2A3647] border border-[#2A3647] text-[#3BC38D] cursor-pointer"
                >
                  <RefreshCw size={13} className={probing ? 'animate-spin' : ''} />
                </button>
              </div>
              {healthReport && (
                <div
                  className={`p-2.5 rounded border text-[11px] space-y-1 ${
                    healthReport.reachable ? 'fp-badge-live' : 'fp-badge-error'
                  }`}
                >
                  <div className="flex items-center justify-between font-semibold">
                    <span>
                      {healthReport.serial} · {healthReport.connectionState.toUpperCase()}
                    </span>
                    <span>{healthReport.latencyMs} ms</span>
                  </div>
                  {healthReport.reachable ? (
                    <div>
                      Kích thước: {healthReport.widthPx}×{healthReport.heightPx}px · Góc{' '}
                      {healthReport.rotationDeg}°
                    </div>
                  ) : (
                    <div className="font-mono break-all">
                      {healthReport.error || 'Không thể kết nối thiết bị'}
                    </div>
                  )}
                </div>
              )}
            </div>

            {currentDevice && (
              <>
                <div className="grid grid-cols-2 gap-2">
                  <div className="p-2.5 bg-[#17202E] border border-[#2A3647] rounded-lg flex items-center gap-2">
                    <Battery size={16} className="text-[#3BC38D] shrink-0" />
                    <div>
                      <div className="text-[10px] text-[#74869D]">Pin &amp; Nguồn</div>
                      <div className="font-semibold text-[#EDF3FC]">
                        {selectedDeviceDetail?.battery_level != null
                          ? `${selectedDeviceDetail.battery_level}%`
                          : 'USB / Emulator'}
                      </div>
                    </div>
                  </div>
                  <div className="p-2.5 bg-[#17202E] border border-[#2A3647] rounded-lg flex items-center gap-2">
                    <Monitor size={16} className="text-[#38BDF8] shrink-0" />
                    <div>
                      <div className="text-[10px] text-[#74869D]">Màn hình</div>
                      <div className="font-semibold text-[#EDF3FC]">
                        {selectedDeviceDetail?.is_screen_on ? 'Đang bật' : 'Chưa xác định'}
                      </div>
                    </div>
                  </div>
                </div>

                {/* Package Launcher */}
                <div className="p-3 bg-[#17202E] border border-[#2A3647] rounded-lg space-y-2.5">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-1.5 font-bold text-[#EDF3FC]">
                      <Layers size={13} className="text-[#38BDF8]" />
                      <span>Quản lý &amp; Mở App ({packages.length})</span>
                    </div>
                    <button
                      onClick={() => {
                        const next = !showAllSystemPackages;
                        setShowAllSystemPackages(next);
                        loadPackages(currentDevice.id, !next);
                      }}
                      className="text-[10px] text-[#74869D] hover:text-[#38BDF8] cursor-pointer"
                    >
                      {showAllSystemPackages ? 'Chỉ app cài thêm' : 'Hiện hệ thống'}
                    </button>
                  </div>

                  {isLoadingPackages ? (
                    <div className="text-[#74869D] italic">Đang quét packages...</div>
                  ) : packages.length === 0 ? (
                    <div className="text-[#74869D]">Không có package nào từ thiết bị.</div>
                  ) : (
                    <>
                      <select
                        value={selectedPackageName || ''}
                        onChange={(e) => setSelectedPackageName(e.target.value)}
                        className="w-full bg-[#101722] border border-[#2A3647] rounded p-2 text-xs text-[#EDF3FC]"
                      >
                        {packages.map((pkg) => (
                          <option key={pkg.package_name} value={pkg.package_name}>
                            {pkg.is_game ? '🎮 ' : '📱 '}
                            {pkg.display_name}
                          </option>
                        ))}
                      </select>
                      <div className="grid grid-cols-2 gap-2">
                        <button
                          onClick={async () => {
                            try {
                              onNotice(await launchApp(currentDevice.id, selectedPackageName || undefined), 'success');
                            } catch (e: any) {
                              onNotice(e?.message || String(e), 'error');
                            }
                          }}
                          className="py-1.5 bg-[#1878F5] hover:bg-[#2F8CFF] text-white rounded font-semibold flex items-center justify-center gap-1 cursor-pointer"
                        >
                          <Play size={12} fill="currentColor" />
                          <span>Mở App</span>
                        </button>
                        <button
                          onClick={async () => {
                            try {
                              onNotice(await stopApp(currentDevice.id, selectedPackageName || undefined), 'info');
                            } catch (e: any) {
                              onNotice(e?.message || String(e), 'error');
                            }
                          }}
                          className="py-1.5 bg-[#3B1717] hover:bg-[#F27883] text-[#F27883] hover:text-white rounded font-semibold flex items-center justify-center gap-1 border border-[#F27883]/30 cursor-pointer"
                        >
                          <StopSquare size={12} fill="currentColor" />
                          <span>Tắt App</span>
                        </button>
                      </div>
                    </>
                  )}
                </div>
              </>
            )}
          </>
        )}

        {tab === 'vision' && (
          <>
            <div className="p-3 bg-[#17202E] border border-[#2A3647] rounded-lg space-y-2.5">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-1.5 font-bold text-[#EDF3FC]">
                  <Camera size={14} className="text-[#4B9CFF]" />
                  <span>Khung hình &amp; Cắt ROI</span>
                </div>
                <span
                  className={`text-[10px] font-bold px-1.5 py-0.2 rounded ${
                    frameSourceKind === 'LIVE' ? 'fp-badge-live' : 'fp-badge-testkit'
                  }`}
                >
                  {frameSourceKind === 'LIVE' ? 'LIVE CAPTURE' : 'TESTKIT SAMPLE'}
                </span>
              </div>

              <div className="w-full h-48 bg-[#0C121B] border border-[#2A3647] rounded-lg flex items-center justify-center overflow-hidden relative">
                {matchPreview?.annotatedPngBase64 ? (
                  <img
                    src={`data:image/png;base64,${matchPreview.annotatedPngBase64}`}
                    alt="Annotated ROI Match"
                    className="max-h-full max-w-full object-contain"
                  />
                ) : capturedScreenshot ? (
                  <img
                    src={capturedScreenshot}
                    alt="Device Snapshot"
                    className="max-h-full max-w-full object-contain"
                  />
                ) : (
                  <div className="text-center text-[#74869D] px-4">
                    Chưa có ảnh. Chụp từ thiết bị ADB LIVE hoặc nạp khung hình mẫu TestKit để test ROI.
                  </div>
                )}
              </div>

              <div className="grid grid-cols-2 gap-2">
                <button
                  onClick={handleCaptureLive}
                  disabled={!currentDevice || isCapturing}
                  className="py-1.5 px-2 bg-[#1878F5] hover:bg-[#2F8CFF] text-white rounded font-semibold flex items-center justify-center gap-1 cursor-pointer disabled:opacity-40"
                >
                  <Camera size={12} />
                  <span>{isCapturing ? 'Đang chụp...' : 'Chụp ADB thật'}</span>
                </button>
                <button
                  onClick={handleLoadTestkitSample}
                  className="py-1.5 px-2 bg-[#202B3B] hover:bg-[#2A3647] border border-[#2A3647] text-[#B291F7] rounded font-medium cursor-pointer"
                >
                  Nạp mẫu TestKit
                </button>
              </div>
            </div>

            {/* Save ROI to Active Project */}
            <div className="p-3 bg-[#17202E] border border-[#2A3647] rounded-lg space-y-2.5">
              <div className="flex items-center justify-between">
                <span className="font-bold text-[#EDF3FC]">Lưu Template ROI vào Project</span>
                <span className="text-[10px] font-mono text-[#74869D]">
                  {activeProjectDetail?.project.slug || 'No Project'}
                </span>
              </div>

              <div className="grid grid-cols-2 gap-2">
                <input
                  value={tplName}
                  onChange={(e) => setTplName(e.target.value)}
                  placeholder="Tên template"
                  className="px-2 py-1.5 rounded bg-[#101722] border border-[#2A3647] text-[#EDF3FC]"
                />
                <input
                  value={tplRelPath}
                  onChange={(e) => setTplRelPath(e.target.value)}
                  placeholder="templates/btn.png"
                  className="px-2 py-1.5 rounded bg-[#101722] border border-[#2A3647] font-mono text-[11px] text-[#EDF3FC]"
                />
              </div>

              <div className="grid grid-cols-4 gap-1.5">
                {[
                  { label: 'X', val: roiX, set: setRoiX },
                  { label: 'Y', val: roiY, set: setRoiY },
                  { label: 'W', val: roiW, set: setRoiW },
                  { label: 'H', val: roiH, set: setRoiH },
                ].map((f) => (
                  <label key={f.label} className="space-y-0.5 block">
                    <span className="text-[10px] text-[#74869D]">{f.label}</span>
                    <input
                      type="number"
                      value={f.val}
                      onChange={(e) => f.set(Number(e.target.value))}
                      className="w-full px-2 py-1 rounded bg-[#101722] border border-[#2A3647] font-mono text-[#EDF3FC]"
                    />
                  </label>
                ))}
              </div>

              <button
                onClick={handleCropAndSaveTemplate}
                disabled={savingTpl || !capturedScreenshot || !activeProjectId}
                className="w-full py-1.5 rounded bg-[#3BC38D] hover:bg-[#32ad7c] text-[#0B1220] font-bold flex items-center justify-center gap-1.5 cursor-pointer disabled:opacity-40"
              >
                <Crop size={13} />
                <span>{savingTpl ? 'Đang cắt & lưu...' : 'Cắt ROI & Lưu vào SQLite Project'}</span>
              </button>
            </div>

            {/* Project-scoped Template Catalog */}
            <div className="p-3 bg-[#17202E] border border-[#2A3647] rounded-lg space-y-2">
              <div className="font-bold text-[#EDF3FC]">
                Templates của Active Project ({projectTemplates.length})
              </div>
              {projectTemplates.length === 0 ? (
                <div className="text-[#74869D]">Chưa có template nào trong Project này.</div>
              ) : (
                <div className="space-y-1.5">
                  {projectTemplates.map((t) => (
                    <div
                      key={t.id}
                      className="p-2 rounded bg-[#101722] border border-[#2A3647] flex items-center justify-between gap-2"
                    >
                      <div className="min-w-0">
                        <div className="font-medium text-[#EDF3FC] truncate">{t.name}</div>
                        <div className="font-mono text-[10px] text-[#74869D] truncate">
                          {t.relPath} · {t.widthPx}×{t.heightPx}
                        </div>
                      </div>
                      <button
                        onClick={() => handleTestMatch(t.id)}
                        disabled={!capturedScreenshot}
                        className="px-2 py-1 rounded bg-[#173355] hover:bg-[#1f4470] text-[#4B9CFF] text-[10.5px] font-semibold cursor-pointer shrink-0 disabled:opacity-40"
                      >
                        Test Match
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </>
        )}

        {tab === 'ports' && (
          <div className="space-y-3">
            <div className="p-2.5 rounded bg-[#17202E] border border-[#E7B15B]/35 text-[11px] text-[#E7B15B] flex items-start gap-2">
              <ShieldAlert size={14} className="shrink-0 mt-0.5" />
              <span>
                ADB forward/reverse quản lý port mapping cục bộ host ↔ Android. Không phải Proxy
                outbound định tuyến traffic game.
              </span>
            </div>

            <div className="p-3 bg-[#17202E] border border-[#2A3647] rounded-lg space-y-2.5">
              <div className="flex items-center justify-between">
                <span className="font-bold text-[#EDF3FC]">Port Mapping ({targetSerial})</span>
                <button
                  onClick={handleRefreshPorts}
                  disabled={loadingPorts}
                  className="px-2 py-1 rounded bg-[#202B3B] hover:bg-[#2A3647] text-[#4B9CFF] text-[11px] flex items-center gap-1 cursor-pointer"
                >
                  <RefreshCw size={11} className={loadingPorts ? 'animate-spin' : ''} />
                  <span>Đọc danh sách</span>
                </button>
              </div>

              <div className="space-y-1.5">
                <div className="text-[11px] text-[#A9B8CC] font-semibold">Thêm adb forward</div>
                <div className="flex gap-1.5">
                  <input
                    value={fwdLocal}
                    onChange={(e) => setFwdLocal(e.target.value)}
                    placeholder="local (tcp:18800)"
                    className="w-1/2 px-2 py-1 rounded bg-[#101722] border border-[#2A3647] font-mono text-[11px] text-[#EDF3FC]"
                  />
                  <input
                    value={fwdRemote}
                    onChange={(e) => setFwdRemote(e.target.value)}
                    placeholder="remote (tcp:18800)"
                    className="w-1/2 px-2 py-1 rounded bg-[#101722] border border-[#2A3647] font-mono text-[11px] text-[#EDF3FC]"
                  />
                  <button
                    onClick={handleAddForward}
                    className="px-2.5 py-1 rounded bg-[#1878F5] text-white font-semibold cursor-pointer"
                    title="Thêm forward"
                  >
                    <Plus size={12} />
                  </button>
                </div>
              </div>

              <div className="space-y-1.5">
                <div className="text-[11px] text-[#A9B8CC] font-semibold">Thêm adb reverse</div>
                <div className="flex gap-1.5">
                  <input
                    value={revRemote}
                    onChange={(e) => setRevRemote(e.target.value)}
                    placeholder="remote (tcp:19900)"
                    className="w-1/2 px-2 py-1 rounded bg-[#101722] border border-[#2A3647] font-mono text-[11px] text-[#EDF3FC]"
                  />
                  <input
                    value={revLocal}
                    onChange={(e) => setRevLocal(e.target.value)}
                    placeholder="local (tcp:19900)"
                    className="w-1/2 px-2 py-1 rounded bg-[#101722] border border-[#2A3647] font-mono text-[11px] text-[#EDF3FC]"
                  />
                  <button
                    onClick={handleAddReverse}
                    className="px-2.5 py-1 rounded bg-[#3BC38D] text-[#0B1220] font-semibold cursor-pointer"
                    title="Thêm reverse"
                  >
                    <Plus size={12} />
                  </button>
                </div>
              </div>

              {portsOverview && (
                <div className="space-y-2 pt-2 border-t border-[#2A3647]">
                  <div className="text-[11px] text-[#74869D]">
                    Forward ({portsOverview.forwards.length}) · Reverse ({portsOverview.reverses.length})
                  </div>
                  {portsOverview.forwards.map((f, idx) => (
                    <div
                      key={`f-${idx}`}
                      className="flex items-center justify-between p-1.5 rounded bg-[#101722] font-mono text-[11px]"
                    >
                      <span>
                        FWD {f.local} → {f.remote}
                      </span>
                      <button
                        onClick={() => api.studioRemoveDeviceForward(f.serial, f.local).then(handleRefreshPorts)}
                        className="text-[#F27883] cursor-pointer"
                      >
                        <Trash2 size={12} />
                      </button>
                    </div>
                  ))}
                  {portsOverview.reverses.map((r, idx) => (
                    <div
                      key={`r-${idx}`}
                      className="flex items-center justify-between p-1.5 rounded bg-[#101722] font-mono text-[11px]"
                    >
                      <span>
                        REV {r.remote} → {r.local}
                      </span>
                      <button
                        onClick={() => api.studioRemoveDeviceReverse(r.serial, r.remote).then(handleRefreshPorts)}
                        className="text-[#F27883] cursor-pointer"
                      >
                        <Trash2 size={12} />
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </div>

            <div className="p-2.5 rounded bg-[#101722] border border-[#2A3647] text-[11px] text-[#74869D] flex items-center gap-1.5">
              <CheckCircle2 size={13} className="text-[#4B9CFF] shrink-0" />
              <span>Tất cả endpoint serial/spec đều đi qua bộ kiểm tra sanitize an toàn.</span>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
