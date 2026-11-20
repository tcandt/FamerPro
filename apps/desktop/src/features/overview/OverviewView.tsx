import React from 'react';
import { Smartphone, Activity, AlertTriangle, CheckCircle, ArrowRight, Play } from 'lucide-react';
import { useDeviceStore } from '../../stores/deviceStore';
import { useWorkflowStore } from '../../stores/workflowStore';

export const OverviewView: React.FC<{ onNavigate: (view: string) => void }> = ({ onNavigate }) => {
  const { devices } = useDeviceStore();
  const { isRunning } = useWorkflowStore();

  const readyDevices = devices.filter((d) => d.connection_state === 'ready');
  const attentionDevices = devices.filter((d) => d.connection_state !== 'ready');

  return (
    <div className="flex-1 overflow-y-auto p-6 space-y-6 bg-[#101722] text-[#EDF3FC]">
      {/* Top Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold text-[#EDF3FC]">Trung tâm Tổng quan</h1>
          <p className="text-xs text-[#74869D] mt-0.5">
            Theo dõi trạng thái thiết bị thực, các phiên tự động hóa đang chạy và hàng đợi cảnh báo.
          </p>
        </div>
        <button
          onClick={() => onNavigate('studio')}
          className="px-4 py-2 bg-[#1878F5] hover:bg-[#2F8CFF] text-white text-xs font-semibold rounded flex items-center gap-2 cursor-pointer shadow-sm transition-colors"
        >
          <Play size={13} fill="currentColor" />
          <span>Mở Workflow Studio</span>
        </button>
      </div>

      {/* 4 KPI Cards */}
      <div className="grid grid-cols-4 gap-4">
        <div className="p-4 rounded-[9px] bg-[#192230] border border-[#2A3647]">
          <div className="flex items-center justify-between text-[#74869D] mb-2">
            <span className="text-xs font-medium">Thiết bị kết nối</span>
            <Smartphone size={16} className="text-[#1878F5]" />
          </div>
          <div className="text-2xl font-bold font-mono text-[#EDF3FC]">{devices.length}</div>
          <div className="text-[11px] text-[#3BC38D] mt-1">{readyDevices.length} thiết bị sẵn sàng</div>
        </div>

        <div className="p-4 rounded-[9px] bg-[#192230] border border-[#2A3647]">
          <div className="flex items-center justify-between text-[#74869D] mb-2">
            <span className="text-xs font-medium">Tác vụ đang chạy</span>
            <Activity size={16} className="text-[#3BC38D]" />
          </div>
          <div className="text-2xl font-bold font-mono text-[#EDF3FC]">{isRunning ? '1' : '0'}</div>
          <div className="text-[11px] text-[#74869D] mt-1">Đơn luồng trên mỗi thiết bị</div>
        </div>

        <div className="p-4 rounded-[9px] bg-[#192230] border border-[#2A3647]">
          <div className="flex items-center justify-between text-[#74869D] mb-2">
            <span className="text-xs font-medium">Cần chú ý</span>
            <AlertTriangle size={16} className="text-[#E7B15B]" />
          </div>
          <div className="text-2xl font-bold font-mono text-[#E7B15B]">{attentionDevices.length}</div>
          <div className="text-[11px] text-[#74869D] mt-1">Chưa cấp quyền hoặc mất kết nối</div>
        </div>

        <div className="p-4 rounded-[9px] bg-[#192230] border border-[#2A3647]">
          <div className="flex items-center justify-between text-[#74869D] mb-2">
            <span className="text-xs font-medium">Tỷ lệ xác minh (KPI)</span>
            <CheckCircle size={16} className="text-[#4B9CFF]" />
          </div>
          <div className="text-2xl font-bold font-mono text-[#EDF3FC]">99.2%</div>
          <div className="text-[11px] text-[#3BC38D] mt-1">Thử nghiệm testkit đạt chuẩn</div>
        </div>
      </div>

      {/* 2-Column Body: Running Now vs Needs Attention */}
      <div className="grid grid-cols-12 gap-6">
        {/* Left Column (7 cols): Active Devices */}
        <div className="col-span-7 rounded-[9px] bg-[#192230] border border-[#2A3647] p-4 flex flex-col space-y-3">
          <div className="flex items-center justify-between">
            <h3 className="text-xs font-semibold text-[#EDF3FC]">Danh sách thiết bị sẵn sàng</h3>
            <button
              onClick={() => onNavigate('devices')}
              className="text-xs text-[#1878F5] hover:underline flex items-center gap-1"
            >
              <span>Xem chi tiết</span>
              <ArrowRight size={12} />
            </button>
          </div>

          <div className="space-y-2">
            {readyDevices.map((dev) => (
              <div
                key={dev.id}
                className="p-3 rounded bg-[#161E29] border border-[#2A3647] flex items-center justify-between text-xs"
              >
                <div>
                  <div className="font-semibold text-[#EDF3FC]">{dev.alias}</div>
                  <div className="text-[11px] text-[#74869D] font-mono">{dev.adb_serial}</div>
                </div>
                <div className="flex items-center gap-3">
                  <span className="px-2 py-0.5 rounded text-[10px] bg-[#133528] text-[#3BC38D] border border-[#3BC38D]/30 font-medium">
                    Sẵn sàng
                  </span>
                  <span className="text-[11px] text-[#74869D]">Android {dev.android_api || 33}</span>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Right Column (5 cols): Attention Queue */}
        <div className="col-span-5 rounded-[9px] bg-[#192230] border border-[#2A3647] p-4 flex flex-col space-y-3">
          <h3 className="text-xs font-semibold text-[#EDF3FC]">Hàng đợi cần xử lý (Attention Queue)</h3>
          {attentionDevices.length === 0 ? (
            <div className="p-6 text-center text-[#74869D] text-xs">
              Mọi thiết bị đều đang hoạt động bình thường!
            </div>
          ) : (
            <div className="space-y-2">
              {attentionDevices.map((dev) => (
                <div
                  key={dev.id}
                  className="p-3 rounded bg-[#161E29] border border-[#E7B15B]/30 flex flex-col space-y-1 text-xs"
                >
                  <div className="flex items-center justify-between">
                    <span className="font-semibold text-[#EDF3FC]">{dev.alias}</span>
                    <span className="px-2 py-0.5 rounded text-[10px] bg-[#3B2D17] text-[#E7B15B] font-medium border border-[#E7B15B]/30">
                      Chưa ủy quyền ADB
                    </span>
                  </div>
                  <p className="text-[11px] text-[#74869D]">
                    Vui lòng bấm "Cho phép gỡ lỗi USB" trên màn hình điện thoại.
                  </p>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
