import React, { useEffect, useState } from 'react';
import {
  AlertCircle,
  ArrowRightLeft,
  CheckCircle2,
  Play,
  RefreshCw,
  Wifi,
} from 'lucide-react';
import { useProjectStore } from '../../stores/projectStore';
import { useDeviceStore } from '../../stores/deviceStore';
import { api } from '../../lib/api';
import type {
  AdbPortMappingsOverview,
  ProxyProbeReport,
  ProxyProtocol,
} from '../../types';

export const NetworkView: React.FC = () => {
  const { activeProjectId, activeProjectDetail } = useProjectStore();
  const { devices, selectedDeviceId } = useDeviceStore();

  const [testHost, setTestHost] = useState('127.0.0.1');
  const [testPort, setTestPort] = useState('10808');
  const [testProtocol, setTestProtocol] = useState<ProxyProtocol>('socks5');
  const [targetProbeHost, setTargetProbeHost] = useState('');
  const [targetProbePort, setTargetProbePort] = useState('');
  const [isTesting, setIsTesting] = useState(false);
  const [probeReport, setProbeReport] = useState<ProxyProbeReport | null>(null);
  const [probeError, setProbeError] = useState<string | null>(null);

  const [portOverview, setPortOverview] = useState<AdbPortMappingsOverview | null>(null);
  const [portError, setPortError] = useState<string | null>(null);

  const selectedDevice = devices.find((d) => d.id === selectedDeviceId) || devices[0] || null;

  const refreshPorts = async () => {
    if (!selectedDevice?.adb_serial) {
      setPortOverview(null);
      return;
    }
    setPortError(null);
    try {
      const overview = await api.studioListDevicePorts(selectedDevice.adb_serial);
      setPortOverview(overview);
    } catch (err: any) {
      setPortError(err?.message || String(err));
    }
  };

  useEffect(() => {
    void refreshPorts();
  }, [selectedDevice?.adb_serial]);

  const handleProbeProxy = async () => {
    if (!activeProjectId) {
      setProbeError('Cần chọn Game Project ở Header trước khi kiểm tra Proxy.');
      return;
    }
    const parsedPort = Number(testPort);
    if (!Number.isFinite(parsedPort) || parsedPort <= 0 || parsedPort > 65535) {
      setProbeError('Port phải nằm trong khoảng 1..65535.');
      return;
    }

    setIsTesting(true);
    setProbeReport(null);
    setProbeError(null);
    try {
      const report = await api.studioProbeProjectProxy(activeProjectId, {
        host: testHost.trim(),
        port: parsedPort,
        protocol: testProtocol,
        timeoutMs: 2500,
        targetProbeHost: targetProbeHost.trim() || null,
        targetProbePort: targetProbePort.trim() ? Number(targetProbePort) : null,
      });
      setProbeReport(report);
    } catch (err: any) {
      setProbeError(err?.message || String(err));
    } finally {
      setIsTesting(false);
    }
  };

  return (
    <div className="flex-1 overflow-y-auto p-5 space-y-5 bg-[#101722] text-[#EDF3FC]">
      <div className="fp-panel p-4 flex items-center justify-between gap-3">
        <div>
          <div className="flex items-center gap-2">
            <span className="px-2 py-0.5 rounded text-[10px] bg-[#173355] text-[#38BDF8] font-mono font-bold border border-[#2A3647]">
              NETWORK &amp; PROXY DIAGNOSTICS
            </span>
            {activeProjectDetail && (
              <span className="text-xs font-mono text-[#A9B8CC]">
                {activeProjectDetail.project.slug}
              </span>
            )}
          </div>
          <h1 className="text-lg font-bold text-[#EDF3FC] mt-1">
            Kiểm tra Proxy (Rust TCP/SOCKS5/HTTP) &amp; ADB Port Map
          </h1>
          <p className="text-xs text-[#74869D]">
            Mọi kết quả Proxy Probe và danh sách cổng ADB đều được đo trực tiếp từ Rust backend,
            không dùng số liệu ping giả lập.
          </p>
        </div>
      </div>

      {/* Warning banner */}
      <div className="p-3.5 rounded-xl bg-[#3B2D17]/40 border border-[#E7B15B]/40 flex items-start gap-3 text-xs text-[#E7B15B]">
        <AlertCircle size={17} className="shrink-0 mt-0.5" />
        <div>
          <div className="font-bold text-[#EDF3FC]">
            Phân tách Proxy Máy trạm vs Proxy trong Game Android:
          </div>
          <p className="mt-0.5 text-[#A9B8CC] leading-relaxed">
            Proxy Probe chỉ xác minh kết nối TCP/handshake từ máy Windows tới proxy server. Android
            có thể bỏ qua system proxy nếu app sử dụng socket trực tiếp hoặc không có VPN TUN profile.
          </p>
        </div>
      </div>

      <div className="grid grid-cols-12 gap-5">
        {/* Left: Real ADB Forward/Reverse Inspection */}
        <div className="col-span-12 lg:col-span-6 fp-panel p-4 space-y-3">
          <div className="flex items-center justify-between border-b border-[#2A3647] pb-2.5">
            <div className="flex items-center gap-2">
              <ArrowRightLeft size={15} className="text-[#38BDF8]" />
              <h3 className="text-xs font-bold text-[#EDF3FC]">
                Bảng ADB Forward / Reverse Thực tế
              </h3>
            </div>
            <button
              onClick={() => refreshPorts()}
              disabled={!selectedDevice}
              className="px-2.5 py-1 rounded bg-[#161E29] border border-[#2A3647] text-[11px] text-[#A9B8CC] flex items-center gap-1 cursor-pointer disabled:opacity-50"
            >
              <RefreshCw size={11} />
              <span>Quét lại</span>
            </button>
          </div>

          {!selectedDevice ? (
            <div className="p-4 rounded bg-[#101722] border border-dashed border-[#2A3647] text-xs text-[#74869D]">
              0 thiết bị ADB đang kết nối (OFFLINE). Kết nối thiết bị thật/giả lập hoặc quản lý chi
              tiết trong Device Lab &rarr; Port Map.
            </div>
          ) : portError ? (
            <div className="p-3 rounded bg-[#381E24] border border-[#F27883]/40 text-xs text-[#F27883]">
              {portError}
            </div>
          ) : (
            <div className="space-y-2 text-xs">
              <div className="text-[11px] text-[#A9B8CC]">
                Thiết bị: <span className="font-mono text-[#EDF3FC]">{selectedDevice.adb_serial}</span>
              </div>
              {portOverview &&
              portOverview.forwards.length === 0 &&
              portOverview.reverses.length === 0 ? (
                <p className="text-[11px] text-[#74869D]">Chưa có rule forward/reverse nào.</p>
              ) : (
                <>
                  {portOverview?.forwards.map((r, i) => (
                    <div
                      key={`f-${i}`}
                      className="p-2.5 rounded bg-[#101722] border border-[#2A3647] font-mono text-[11px]"
                    >
                      FORWARD {r.local} &rarr; {r.remote}
                    </div>
                  ))}
                  {portOverview?.reverses.map((r, i) => (
                    <div
                      key={`r-${i}`}
                      className="p-2.5 rounded bg-[#101722] border border-[#2A3647] font-mono text-[11px]"
                    >
                      REVERSE {r.remote} &rarr; {r.local}
                    </div>
                  ))}
                </>
              )}
            </div>
          )}
        </div>

        {/* Right: Real Rust Proxy Probe */}
        <div className="col-span-12 lg:col-span-6 fp-panel p-4 space-y-3 text-xs">
          <div className="flex items-center gap-2 border-b border-[#2A3647] pb-2.5">
            <Wifi size={15} className="text-[#4B9CFF]" />
            <h3 className="text-xs font-bold text-[#EDF3FC]">
              Kiểm tra Proxy Thực tế (studio_probe_project_proxy)
            </h3>
          </div>

          <div className="grid grid-cols-3 gap-2">
            {(['socks5', 'http_connect', 'tcp_only'] as ProxyProtocol[]).map((proto) => (
              <button
                key={proto}
                onClick={() => setTestProtocol(proto)}
                className={`py-1.5 rounded font-semibold border cursor-pointer ${
                  testProtocol === proto
                    ? 'bg-[#1878F5] text-white border-[#1878F5]'
                    : 'bg-[#101722] text-[#74869D] border-[#2A3647]'
                }`}
              >
                {proto.toUpperCase()}
              </button>
            ))}
          </div>

          <div className="grid grid-cols-3 gap-2">
            <div className="col-span-2">
              <label className="block text-[#A9B8CC] mb-1 text-[11px]">Proxy Host</label>
              <input
                type="text"
                value={testHost}
                onChange={(e) => setTestHost(e.target.value)}
                className="w-full bg-[#101722] border border-[#2A3647] rounded px-3 py-1.5 text-[#EDF3FC] font-mono outline-none"
              />
            </div>
            <div>
              <label className="block text-[#A9B8CC] mb-1 text-[11px]">Port</label>
              <input
                type="text"
                value={testPort}
                onChange={(e) => setTestPort(e.target.value)}
                className="w-full bg-[#101722] border border-[#2A3647] rounded px-3 py-1.5 text-[#EDF3FC] font-mono outline-none"
              />
            </div>
          </div>

          <div className="grid grid-cols-3 gap-2">
            <div className="col-span-2">
              <label className="block text-[#A9B8CC] mb-1 text-[11px]">
                Target Probe Host (tuỳ chọn)
              </label>
              <input
                type="text"
                value={targetProbeHost}
                onChange={(e) => setTargetProbeHost(e.target.value)}
                placeholder="example.com"
                className="w-full bg-[#101722] border border-[#2A3647] rounded px-3 py-1.5 text-[#EDF3FC] font-mono outline-none"
              />
            </div>
            <div>
              <label className="block text-[#A9B8CC] mb-1 text-[11px]">Target Port</label>
              <input
                type="text"
                value={targetProbePort}
                onChange={(e) => setTargetProbePort(e.target.value)}
                placeholder="443"
                className="w-full bg-[#101722] border border-[#2A3647] rounded px-3 py-1.5 text-[#EDF3FC] font-mono outline-none"
              />
            </div>
          </div>

          <button
            onClick={handleProbeProxy}
            disabled={isTesting}
            className="w-full py-2 bg-[#1878F5] hover:bg-[#2F8CFF] disabled:opacity-50 text-white rounded font-semibold flex items-center justify-center gap-2 cursor-pointer"
          >
            <Play size={12} />
            <span>{isTesting ? 'Đang đo qua Rust TCP/SOCKS5...' : 'Chạy Proxy Probe Thật'}</span>
          </button>

          {probeError && (
            <div className="p-3 rounded bg-[#381E24] border border-[#F27883]/40 text-[#F27883]">
              {probeError}
            </div>
          )}

          {probeReport && (
            <div
              className={`p-3 rounded-lg border space-y-1.5 ${
                probeReport.reachable
                  ? 'bg-[#133528] border-[#3BC38D]/40 text-[#3BC38D]'
                  : 'bg-[#101722] border-[#E7B15B]/50 text-[#E7B15B]'
              }`}
            >
              <div className="flex items-center justify-between font-bold">
                <span className="flex items-center gap-1.5">
                  <CheckCircle2 size={14} />
                  <span>
                    {probeReport.reachable ? 'REACHABLE' : 'UNREACHABLE'} ({probeReport.protocol})
                  </span>
                </span>
                <span className="font-mono">{probeReport.totalLatencyMs} ms</span>
              </div>
              <div className="text-[11px] font-mono text-[#A9B8CC]">
                Handshake: {String(probeReport.handshakeOk)} · Auth: {probeReport.authStatus}
              </div>
              {probeReport.error && (
                <div className="text-[11px] font-mono text-[#F27883]">
                  Error: {probeReport.error}
                </div>
              )}
              <div className="text-[10px] text-[#74869D] pt-1 border-t border-[#2A3647]">
                {probeReport.disclaimer}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
