import React, { useState } from 'react';
import {
  AlertCircle,
  Wifi,
  Radio,
  ArrowRightLeft,
  CheckCircle2,
  Play,
  Plus,
  Shield,
} from 'lucide-react';

export const NetworkView: React.FC = () => {

  const [testHost, setTestHost] = useState('127.0.0.1');
  const [testPort, setTestPort] = useState('10808');
  const [testProtocol, setTestProtocol] = useState<'socks5' | 'http'>('socks5');
  const [isTesting, setIsTesting] = useState(false);
  const [testResult, setTestResult] = useState<{
    success: boolean;
    latencyMs: number;
    ip: string;
    message: string;
  } | null>(null);

  const handleTestProxy = () => {
    setIsTesting(true);
    setTestResult(null);
    setTimeout(() => {
      setIsTesting(false);
      setTestResult({
        success: true,
        latencyMs: 38,
        ip: '103.142.26.88 (VN Viettel IDC)',
        message: 'Kết nối SOCKS5 thành công — Ping 38ms',
      });
    }, 600);
  };

  const forwardRules = [
    {
      id: 'rule-1',
      deviceSerial: 'emulator-5554',
      type: 'reverse',
      localPort: 'tcp:8080',
      remotePort: 'tcp:8080',
      desc: 'API mock server callback',
    },
    {
      id: 'rule-2',
      deviceSerial: '192.168.1.105:5555',
      type: 'forward',
      localPort: 'tcp:5555',
      remotePort: 'tcp:5555',
      desc: 'ADB wireless bridge tunnel',
    },
  ];

  return (
    <div className="flex-1 overflow-y-auto p-6 space-y-6 bg-[#101722] text-[#EDF3FC]">
      {/* Top Header */}
      <div className="flex items-center justify-between">
        <div>
          <div className="flex items-center gap-2">
            <h1 className="text-xl font-bold text-[#EDF3FC] tracking-tight">
              Cấu hình Mạng & Proxy Gateway
            </h1>
            <span className="px-2 py-0.5 rounded text-[10px] bg-[#173355] text-[#38BDF8] font-mono font-semibold border border-[#2A3647]">
              Mạng cục bộ (Direct LAN)
            </span>
          </div>
          <p className="text-xs text-[#74869D] mt-1">
            Quản lý tuyến mạng, kiểm tra độ trễ proxy và giám sát bảng chuyển tiếp cổng ADB (Socket Forwarding).
          </p>
        </div>
      </div>

      {/* Critical Warning Banner on Android System Proxy Limits */}
      <div className="p-4 rounded-xl bg-[#3B2D17]/40 border border-[#E7B15B]/40 flex items-start gap-3 text-xs text-[#E7B15B] shadow-sm">
        <AlertCircle size={18} className="shrink-0 mt-0.5 text-[#E7B15B]" />
        <div>
          <div className="font-bold text-[#EDF3FC] text-[13px]">
            Lưu ý kỹ thuật quan trọng về Proxy trên Android:
          </div>
          <p className="mt-1 leading-relaxed text-[#A9B8CC]">
            Đổi proxy trên máy tính không tự động thay đổi kết nối của game trên điện thoại. Lệnh cài đặt Android System Proxy qua ADB có thể bị nhiều game bỏ qua nếu không sử dụng VPN profile hợp lệ. FarmerPro không cam kết vượt rào cản mạng nếu ứng dụng đích không hỗ trợ.
          </p>
        </div>
      </div>

      {/* 2-Column Grid */}
      <div className="grid grid-cols-12 gap-6">
        {/* Left Column (7 cols): Network Profiles + ADB Port Forwarding */}
        <div className="col-span-7 space-y-6">
          {/* Network Profiles */}
          <div className="rounded-xl bg-[#16202F] border border-[#2A3647] p-4 shadow-sm space-y-3">
            <div className="flex items-center justify-between">
              <div>
                <h3 className="text-xs font-bold text-[#EDF3FC]">Hồ sơ Mạng Khả dụng</h3>
                <span className="text-[11px] text-[#74869D]">
                  Các cấu hình định tuyến cho máy trạm và thiết bị
                </span>
              </div>
            </div>

            <div className="space-y-2 text-xs">
              {/* Direct LAN Profile */}
              <div className="p-3 bg-[#101722] border border-[#3BC38D]/40 rounded-lg flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <div className="w-8 h-8 rounded-md bg-[#133528] flex items-center justify-center text-[#3BC38D]">
                    <Wifi size={16} />
                  </div>
                  <div>
                    <div className="font-bold text-[#EDF3FC]">Direct LAN (Mạng Nội bộ)</div>
                    <div className="text-[11px] text-[#74869D]">
                      Kết nối trực tiếp qua switch mạng gia đình / văn phòng. Ping: 12ms.
                    </div>
                  </div>
                </div>
                <span className="px-2 py-0.5 rounded text-[10px] bg-[#133528] text-[#3BC38D] border border-[#3BC38D]/30 font-bold">
                  Đang hoạt động
                </span>
              </div>

              {/* SOCKS5 Profile */}
              <div className="p-3 bg-[#101722] border border-[#2A3647] rounded-lg flex items-center justify-between opacity-80 hover:opacity-100 transition-opacity">
                <div className="flex items-center gap-3">
                  <div className="w-8 h-8 rounded-md bg-[#16202F] flex items-center justify-center text-[#38BDF8]">
                    <Radio size={16} />
                  </div>
                  <div>
                    <div className="font-bold text-[#EDF3FC]">Local SOCKS5 Gateway</div>
                    <div className="text-[11px] text-[#74869D]">
                      Định tuyến 127.0.0.1:10808 (Đã cấu hình xác thực)
                    </div>
                  </div>
                </div>
                <span className="px-2 py-0.5 rounded text-[10px] bg-[#202B3B] text-[#74869D] border border-[#2A3647] font-medium">
                  Sẵn sàng
                </span>
              </div>

              {/* Tailscale / WireGuard Profile */}
              <div className="p-3 bg-[#101722] border border-[#2A3647] rounded-lg flex items-center justify-between opacity-80 hover:opacity-100 transition-opacity">
                <div className="flex items-center gap-3">
                  <div className="w-8 h-8 rounded-md bg-[#16202F] flex items-center justify-center text-[#B291F7]">
                    <Shield size={16} />
                  </div>
                  <div>
                    <div className="font-bold text-[#EDF3FC]">WireGuard / Tailscale Mesh</div>
                    <div className="text-[11px] text-[#74869D]">
                      Mạng riêng ảo P2P an toàn giữa máy chủ điều khiển và phone farm
                    </div>
                  </div>
                </div>
                <span className="px-2 py-0.5 rounded text-[10px] bg-[#202B3B] text-[#74869D] border border-[#2A3647] font-medium">
                  Chưa kích hoạt
                </span>
              </div>
            </div>
          </div>

          {/* ADB Socket Reverse & Forwarding Rules */}
          <div className="rounded-xl bg-[#16202F] border border-[#2A3647] p-4 shadow-sm space-y-3">
            <div className="flex items-center justify-between">
              <div>
                <h3 className="text-xs font-bold text-[#EDF3FC]">
                  Bảng Chuyển tiếp Cổng ADB (Socket Forwarding)
                </h3>
                <span className="text-[11px] text-[#74869D]">
                  Các lệnh `adb reverse` và `adb forward` đang hoạt động
                </span>
              </div>
              <button className="px-2.5 py-1 bg-[#192434] hover:bg-[#202E42] text-[#38BDF8] border border-[#2A3647] rounded-md text-xs font-semibold flex items-center gap-1 transition-colors">
                <Plus size={12} />
                <span>Thêm quy tắc</span>
              </button>
            </div>

            <div className="space-y-2">
              {forwardRules.map((rule) => (
                <div
                  key={rule.id}
                  className="p-3 rounded-lg bg-[#101722] border border-[#2A3647] flex items-center justify-between text-xs"
                >
                  <div className="flex items-center gap-3">
                    <ArrowRightLeft size={16} className="text-[#38BDF8]" />
                    <div>
                      <div className="font-mono text-[#EDF3FC] font-semibold flex items-center gap-2">
                        <span className="text-[#74869D] uppercase text-[10px]">{rule.type}:</span>
                        <span>{rule.localPort}</span>
                        <span>⇄</span>
                        <span>{rule.remotePort}</span>
                      </div>
                      <div className="text-[10px] text-[#74869D] mt-0.5">
                        Thiết bị: {rule.deviceSerial} · {rule.desc}
                      </div>
                    </div>
                  </div>
                  <span className="px-2 py-0.5 rounded text-[10px] bg-[#133528] text-[#3BC38D] font-mono font-bold">
                    ACTIVE
                  </span>
                </div>
              ))}
            </div>
          </div>
        </div>

        {/* Right Column (5 cols): Interactive Proxy Tester */}
        <div className="col-span-5 rounded-xl bg-[#16202F] border border-[#2A3647] p-4 shadow-sm space-y-4">
          <div>
            <h3 className="text-xs font-bold text-[#EDF3FC]">Công cụ Đo Độ trễ Proxy</h3>
            <span className="text-[11px] text-[#74869D]">
              Kiểm tra IP đầu ra và độ trễ phản hồi trước khi gán cho thiết bị
            </span>
          </div>

          <div className="space-y-3 text-xs">
            <div>
              <label className="block text-[#A9B8CC] mb-1 font-semibold text-[11px]">Giao thức</label>
              <div className="grid grid-cols-2 gap-2">
                <button
                  onClick={() => setTestProtocol('socks5')}
                  className={`py-1.5 rounded font-semibold text-xs border transition-colors ${
                    testProtocol === 'socks5'
                      ? 'bg-[#1878F5] text-white border-[#1878F5]'
                      : 'bg-[#101722] text-[#74869D] border-[#2A3647]'
                  }`}
                >
                  SOCKS5
                </button>
                <button
                  onClick={() => setTestProtocol('http')}
                  className={`py-1.5 rounded font-semibold text-xs border transition-colors ${
                    testProtocol === 'http'
                      ? 'bg-[#1878F5] text-white border-[#1878F5]'
                      : 'bg-[#101722] text-[#74869D] border-[#2A3647]'
                  }`}
                >
                  HTTP / HTTPS
                </button>
              </div>
            </div>

            <div>
              <label className="block text-[#A9B8CC] mb-1 font-semibold text-[11px]">
                Địa chỉ máy chủ (Host / IP)
              </label>
              <input
                type="text"
                value={testHost}
                onChange={(e) => setTestHost(e.target.value)}
                placeholder="127.0.0.1"
                className="w-full bg-[#101722] border border-[#2A3647] rounded px-3 py-1.5 text-[#EDF3FC] font-mono outline-none"
              />
            </div>

            <div>
              <label className="block text-[#A9B8CC] mb-1 font-semibold text-[11px]">Cổng (Port)</label>
              <input
                type="text"
                value={testPort}
                onChange={(e) => setTestPort(e.target.value)}
                placeholder="10808"
                className="w-full bg-[#101722] border border-[#2A3647] rounded px-3 py-1.5 text-[#EDF3FC] font-mono outline-none"
              />
            </div>

            <button
              onClick={handleTestProxy}
              disabled={isTesting}
              className="w-full py-2 bg-[#1878F5] hover:bg-[#2F8CFF] text-white rounded text-xs font-semibold flex items-center justify-center gap-2 transition-colors cursor-pointer shadow-sm disabled:opacity-50"
            >
              <Play size={12} fill="currentColor" />
              <span>{isTesting ? 'Đang gửi gói tin kiểm tra...' : 'Kiểm tra Kết nối & Ping'}</span>
            </button>

            {/* Test Result Box */}
            {testResult && (
              <div className="p-3 rounded-lg bg-[#101722] border border-[#3BC38D]/40 space-y-2 text-xs animate-fadeIn">
                <div className="flex items-center gap-1.5 text-[#3BC38D] font-bold">
                  <CheckCircle2 size={14} />
                  <span>{testResult.message}</span>
                </div>
                <div className="text-[11px] text-[#A9B8CC] space-y-1">
                  <div>
                    <span className="text-[#74869D]">IP Thoát: </span>
                    <span className="font-mono text-[#EDF3FC]">{testResult.ip}</span>
                  </div>
                  <div>
                    <span className="text-[#74869D]">Độ trễ Round-Trip: </span>
                    <span className="font-mono text-[#38BDF8] font-bold">
                      {testResult.latencyMs} ms
                    </span>
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
