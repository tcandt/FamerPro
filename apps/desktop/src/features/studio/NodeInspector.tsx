import React, { useState } from 'react';
import {
  Sliders,
  ArrowLeftRight,
  RefreshCw,
  Info,
  Trash2,
  Play,
  CheckCircle2,
  AlertCircle,
  ShieldCheck,
  ChevronRight,
} from 'lucide-react';
import { useWorkflowStore } from '../../stores/workflowStore';
import { useDeviceStore } from '../../stores/deviceStore';
import { useProjectStore } from '../../stores/projectStore';
import { api } from '../../lib/api';

export const NodeInspector: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'config' | 'retry' | 'ports'>('config');
  const [testResult, setTestResult] = useState<{
    success: boolean;
    message: string;
    pointTapped?: { x: number; y: number } | null;
  } | null>(null);
  const [isTesting, setIsTesting] = useState(false);

  const { selectedDeviceId } = useDeviceStore();
  const activeProjectTemplates = useProjectStore(
    (state) => state.activeProjectDetail?.templates || []
  );
  const {
    nodes,
    selectedNodeId,
    breakpointNodeIds,
    updateNodeConfig,
    toggleBreakpoint,
    setNodes,
    selectNode,
    isInspectorCollapsed,
    toggleInspector,
  } = useWorkflowStore();

  if (isInspectorCollapsed) {
    return null;
  }

  const selectedNode = nodes.find((n) => n.id === selectedNodeId);

  const handleDeleteNode = () => {
    if (!selectedNodeId) return;
    setNodes(nodes.filter((n) => n.id !== selectedNodeId));
    selectNode(null);
  };

  const handleTestNode = async () => {
    if (!selectedNode) return;
    setIsTesting(true);
    setTestResult(null);
    try {
      const res = await api.executeTestNode(
        selectedDeviceId || 'emulator-5554',
        selectedNode.data.nodeType,
        JSON.stringify(selectedNode.data.config || {})
      );
      setTestResult({
        success: res.success,
        message: res.message,
        pointTapped: res.point_tapped,
      });
    } catch (err: any) {
      setTestResult({
        success: false,
        message: `Lỗi: ${err?.message || String(err)}`,
        pointTapped: null,
      });
    } finally {
      setIsTesting(false);
    }
  };

  if (!selectedNode) {
    return (
      <div className="w-[280px] xl:w-[320px] bg-[#141C28] border-l border-[#2A3647] flex flex-col items-center justify-center p-6 text-center select-none shrink-0 z-20">
        <div className="w-12 h-12 rounded-xl bg-[#192230] border border-[#2A3647] flex items-center justify-center text-[#74869D] mb-3 shadow-inner">
          <Info size={22} className="text-[#38BDF8]" />
        </div>
        <h4 className="text-xs font-bold text-[#EDF3FC] mb-1">Chưa chọn Node nào</h4>
        <p className="text-[11px] text-[#74869D] leading-relaxed max-w-[240px]">
          Nhấp vào bất kỳ node nào trên đồ thị Canvas để cấu hình tham số, điều chỉnh vùng an toàn hoặc thử nghiệm độc lập.
        </p>
      </div>
    );
  }

  const { data } = selectedNode;
  const config = data.config || {};
  const marginRatio = config.marginRatio ?? 0.18;
  const minConfidence = config.minConfidence ?? 0.9;

  return (
    <div className="w-[280px] xl:w-[320px] bg-[#141C28] border-l border-[#2A3647] flex flex-col h-full select-none shrink-0 z-20">
      {/* Header */}
      <div className="p-3 border-b border-[#2A3647] flex items-center justify-between bg-[#16202F]">
        <div className="min-w-0">
          <div className="flex items-center gap-1.5">
            <span className="text-[10px] text-[#74869D] uppercase font-mono">{selectedNode.id}</span>
            <span className="text-[9px] px-1 rounded bg-[#202B3B] text-[#7BA6D6] font-mono">
              {data.nodeType}
            </span>
          </div>
          <h3 className="text-xs font-bold text-[#EDF3FC] truncate">{data.label}</h3>
        </div>
        <div className="flex items-center gap-1">
          <button
            onClick={handleDeleteNode}
            className="p-1.5 text-[#74869D] hover:text-[#F27883] hover:bg-[#202B3B] rounded transition-colors"
            title="Xóa node này khỏi đồ thị"
          >
            <Trash2 size={14} />
          </button>
          <button
            onClick={toggleInspector}
            className="p-1.5 text-[#74869D] hover:text-[#EDF3FC] hover:bg-[#202B3B] rounded transition-colors"
            title="Thu gọn bảng Inspector"
          >
            <ChevronRight size={14} />
          </button>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex border-b border-[#2A3647] bg-[#101722]">
        <button
          onClick={() => setActiveTab('config')}
          className={`flex-1 py-2 text-[11px] font-semibold flex items-center justify-center gap-1.5 border-b-2 transition-colors ${
            activeTab === 'config'
              ? 'text-[#38BDF8] border-[#38BDF8] bg-[#141C28]'
              : 'text-[#74869D] border-transparent hover:text-[#A9B8CC]'
          }`}
        >
          <Sliders size={12} />
          <span>Thuộc tính</span>
        </button>
        <button
          onClick={() => setActiveTab('retry')}
          className={`flex-1 py-2 text-[11px] font-semibold flex items-center justify-center gap-1.5 border-b-2 transition-colors ${
            activeTab === 'retry'
              ? 'text-[#38BDF8] border-[#38BDF8] bg-[#141C28]'
              : 'text-[#74869D] border-transparent hover:text-[#A9B8CC]'
          }`}
        >
          <RefreshCw size={12} />
          <span>Chính sách Retry</span>
        </button>
        <button
          onClick={() => setActiveTab('ports')}
          className={`flex-1 py-2 text-[11px] font-semibold flex items-center justify-center gap-1.5 border-b-2 transition-colors ${
            activeTab === 'ports'
              ? 'text-[#38BDF8] border-[#38BDF8] bg-[#141C28]'
              : 'text-[#74869D] border-transparent hover:text-[#A9B8CC]'
          }`}
        >
          <ArrowLeftRight size={12} />
          <span>Cổng IO</span>
        </button>
      </div>

      {/* Form Body */}
      <div className="flex-1 overflow-y-auto p-3.5 space-y-4 text-xs">
        {activeTab === 'config' && (
          <div className="space-y-3.5">
            {/* Label rename & Breakpoint toggle */}
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <label className="block text-[#A9B8CC] font-semibold text-[11px]">
                  Tên hiển thị Node
                </label>
                <button
                  onClick={() => toggleBreakpoint(selectedNode.id)}
                  className={`px-2 py-0.5 rounded text-[10px] font-semibold cursor-pointer ${
                    breakpointNodeIds.includes(selectedNode.id)
                      ? 'fp-badge-error'
                      : 'bg-[#202B3B] text-[#74869D] hover:text-white'
                  }`}
                >
                  {breakpointNodeIds.includes(selectedNode.id) ? '● Breakpoint ON' : '○ Breakpoint'}
                </button>
              </div>
              <input
                type="text"
                value={data.label}
                onChange={(e) => {
                  const newLabel = e.target.value;
                  setNodes(
                    nodes.map((n) =>
                      n.id === selectedNode.id
                        ? { ...n, data: { ...n.data, label: newLabel } }
                        : n
                    )
                  );
                }}
                className="w-full bg-[#101722] border border-[#2A3647] rounded-md px-2.5 py-1.5 text-[#EDF3FC] focus:border-[#38BDF8] outline-none text-xs"
              />
            </div>

            {/* Package Id (for app.launch) */}
            {data.nodeType === 'app.launch' && (
              <div>
                <label className="block text-[#A9B8CC] mb-1 font-semibold text-[11px]">
                  Tên gói (Package Name)
                </label>
                <input
                  type="text"
                  value={config.packageId || ''}
                  onChange={(e) =>
                    updateNodeConfig(selectedNode.id, { packageId: e.target.value })
                  }
                  placeholder="com.example.game"
                  className="w-full bg-[#101722] border border-[#2A3647] rounded-md px-2.5 py-1.5 text-[#EDF3FC] focus:border-[#38BDF8] outline-none font-mono text-xs"
                />
              </div>
            )}

            {/* Template Ref & Confidence */}
            {(data.nodeType === 'vision.find_image' ||
              data.nodeType === 'verify.wait_for_image') && (
              <div className="space-y-3 p-3 rounded-lg bg-[#101722] border border-[#2A3647]">
                <div className="space-y-1.5">
                  <label className="block text-[#A9B8CC] font-semibold text-[11px]">
                    Chọn Template thuộc Active Project
                  </label>
                  {activeProjectTemplates.length > 0 && (
                    <select
                      value={
                        activeProjectTemplates.some((t) => t.relPath === config.templateRef)
                          ? config.templateRef
                          : ''
                      }
                      onChange={(e) =>
                        updateNodeConfig(selectedNode.id, { templateRef: e.target.value })
                      }
                      className="w-full bg-[#17202E] border border-[#2A3647] rounded px-2 py-1.5 text-[#EDF3FC] text-xs"
                    >
                      <option value="">-- Chọn từ Project ROI --</option>
                      {activeProjectTemplates.map((tpl) => (
                        <option key={tpl.id} value={tpl.relPath}>
                          {tpl.name} ({tpl.relPath} · {tpl.widthPx}×{tpl.heightPx})
                        </option>
                      ))}
                    </select>
                  )}
                  <input
                    type="text"
                    value={config.templateRef || ''}
                    onChange={(e) =>
                      updateNodeConfig(selectedNode.id, { templateRef: e.target.value })
                    }
                    placeholder="templates/marker.png"
                    className="w-full bg-[#17202E] border border-[#2A3647] rounded px-2.5 py-1.5 text-[#EDF3FC] focus:border-[#38BDF8] outline-none font-mono text-xs"
                  />
                </div>
                <div>
                  <div className="flex justify-between mb-1">
                    <label className="text-[#A9B8CC] font-semibold text-[11px]">
                      Độ tin cậy tối thiểu
                    </label>
                    <span className="font-mono text-[#3BC38D] font-bold">
                      {(minConfidence * 100).toFixed(0)}%
                    </span>
                  </div>
                  <input
                    type="range"
                    min="0.5"
                    max="1.0"
                    step="0.01"
                    value={minConfidence}
                    onChange={(e) =>
                      updateNodeConfig(selectedNode.id, {
                        minConfidence: parseFloat(e.target.value),
                      })
                    }
                    className="w-full accent-[#38BDF8]"
                  />
                  <p className="text-[10px] text-[#74869D] mt-1">
                    Ngưỡng SSD chuẩn hóa: &gt;90% đảm bảo không nhận diện nhầm chi tiết nền.
                  </p>
                </div>
              </div>
            )}

            {/* Safe Random Tap configuration + Interactive Visualizer */}
            {data.nodeType === 'action.random_safe_tap' && (
              <div className="space-y-3 p-3 rounded-lg bg-[#101722] border border-[#2A3647]">
                <div className="flex items-center gap-1.5 text-[#38BDF8] font-bold text-[11px]">
                  <ShieldCheck size={14} />
                  <span>Chính sách Tap Ngẫu Nhiên An Toàn</span>
                </div>

                {/* Interactive SVG Diagram representing safe inner margin */}
                <div className="w-full h-28 bg-[#17202E] border border-[#2A3647] rounded-md flex flex-col items-center justify-center p-2 relative overflow-hidden">
                  <div className="text-[9px] text-[#74869D] absolute top-1 left-2 font-mono">
                    Khung phát hiện: 120×60px
                  </div>
                  <div className="w-48 h-18 border border-dashed border-[#F27883]/60 relative flex items-center justify-center rounded">
                    <div
                      className="border-2 border-[#38BDF8] bg-[#38BDF8]/20 flex items-center justify-center text-[10px] text-[#38BDF8] font-bold transition-all rounded"
                      style={{
                        width: `${(1 - marginRatio * 2) * 100}%`,
                        height: `${(1 - marginRatio * 2) * 100}%`,
                      }}
                    >
                      <span>Vùng Safe Tap</span>
                    </div>
                  </div>
                  <div className="text-[9px] text-[#A9B8CC] mt-1">
                    Biên loại trừ: {(marginRatio * 100).toFixed(0)}% mỗi phía
                  </div>
                </div>

                <div>
                  <div className="flex justify-between mb-1">
                    <label className="text-[#A9B8CC] font-semibold text-[11px]">
                      Tỷ lệ co biên an toàn (Margin)
                    </label>
                    <span className="font-mono text-[#38BDF8] font-bold">
                      {(marginRatio * 100).toFixed(0)}%
                    </span>
                  </div>
                  <input
                    type="range"
                    min="0.05"
                    max="0.35"
                    step="0.01"
                    value={marginRatio}
                    onChange={(e) =>
                      updateNodeConfig(selectedNode.id, {
                        marginRatio: parseFloat(e.target.value),
                      })
                    }
                    className="w-full accent-[#38BDF8]"
                  />
                </div>

                <div>
                  <label className="block text-[#A9B8CC] mb-1 font-semibold text-[11px]">
                    Tránh N điểm bấm gần nhất (Anti-pattern)
                  </label>
                  <input
                    type="number"
                    min="1"
                    max="20"
                    value={config.avoidRecentPoints ?? 5}
                    onChange={(e) =>
                      updateNodeConfig(selectedNode.id, {
                        avoidRecentPoints: parseInt(e.target.value) || 1,
                      })
                    }
                    className="w-full bg-[#17202E] border border-[#2A3647] rounded px-2.5 py-1.5 text-[#EDF3FC] outline-none text-xs font-mono"
                  />
                </div>
              </div>
            )}

            {/* Checkpoint Name */}
            {data.nodeType === 'state.checkpoint' && (
              <div>
                <label className="block text-[#A9B8CC] mb-1 font-semibold text-[11px]">
                  Tên mốc kiểm tra (Checkpoint Identifier)
                </label>
                <input
                  type="text"
                  value={config.name || ''}
                  onChange={(e) =>
                    updateNodeConfig(selectedNode.id, { name: e.target.value })
                  }
                  placeholder="quest_active"
                  className="w-full bg-[#101722] border border-[#2A3647] rounded px-2.5 py-1.5 text-[#EDF3FC] outline-none font-mono text-xs"
                />
              </div>
            )}

            {/* Test Node Button */}
            <div className="pt-2 border-t border-[#2A3647]">
              <button
                onClick={handleTestNode}
                disabled={isTesting}
                className="w-full py-1.5 bg-[#202B3B] hover:bg-[#2A3647] text-[#38BDF8] text-xs font-semibold rounded-md border border-[#2A3647] flex items-center justify-center gap-1.5 transition-colors cursor-pointer"
              >
                <Play size={12} fill="currentColor" />
                <span>{isTesting ? 'Đang chạy kiểm tra...' : 'Thử nghiệm node này'}</span>
              </button>

              {testResult && (
                <div
                  className={`mt-2 p-2.5 rounded border text-[11px] space-y-1 ${
                    testResult.success
                      ? 'bg-[#133528] border-[#3BC38D]/40 text-[#3BC38D]'
                      : 'bg-[#F27883]/10 border-[#F27883]/40 text-[#F27883]'
                  }`}
                >
                  <div className="flex items-center gap-1.5 font-semibold">
                    {testResult.success ? (
                      <CheckCircle2 size={13} className="shrink-0" />
                    ) : (
                      <AlertCircle size={13} className="shrink-0" />
                    )}
                    <span>{testResult.message}</span>
                  </div>
                  {testResult.pointTapped && (
                    <div className="text-[10px] text-[#A9B8CC] font-mono pl-4">
                      Tọa độ an toàn: ({testResult.pointTapped.x.toFixed(1)},{' '}
                      {testResult.pointTapped.y.toFixed(1)}) · Co biên an toàn
                    </div>
                  )}
                </div>
              )}
            </div>
          </div>
        )}

        {activeTab === 'retry' && (
          <div className="space-y-3">
            <div>
              <label className="block text-[#A9B8CC] mb-1 font-semibold text-[11px]">
                Số lần thử lại tối đa (Max Retries)
              </label>
              <input
                type="number"
                min="1"
                max="10"
                defaultValue={3}
                className="w-full bg-[#101722] border border-[#2A3647] rounded px-2.5 py-1.5 text-[#EDF3FC] outline-none font-mono text-xs"
              />
            </div>
            <div>
              <label className="block text-[#A9B8CC] mb-1 font-semibold text-[11px]">
                Thời gian chờ giãn cách (Backoff ms)
              </label>
              <input
                type="number"
                step="100"
                defaultValue={1200}
                className="w-full bg-[#101722] border border-[#2A3647] rounded px-2.5 py-1.5 text-[#EDF3FC] outline-none font-mono text-xs"
              />
            </div>
            <div>
              <label className="block text-[#A9B8CC] mb-1 font-semibold text-[11px]">
                Hành vi khi thử lại thất bại
              </label>
              <select className="w-full bg-[#101722] border border-[#2A3647] rounded px-2.5 py-1.5 text-[#EDF3FC] outline-none text-xs">
                <option value="pause">Tạm dừng để người dùng xử lý (Pause for review)</option>
                <option value="fail">Đóng khẩn cấp an toàn (Fail closed)</option>
              </select>
            </div>
          </div>
        )}

        {activeTab === 'ports' && (
          <div className="space-y-3">
            <div>
              <span className="text-[#74869D] font-semibold text-[11px]">Cổng vào (Input Port):</span>
              <div className="p-2.5 bg-[#101722] rounded border border-[#2A3647] mt-1 text-[#EDF3FC] font-mono text-[11px]">
                flow: Execution Context (Frame + Target)
              </div>
            </div>
            <div>
              <span className="text-[#74869D] font-semibold text-[11px]">Cổng ra (Output Port):</span>
              <div className="p-2.5 bg-[#101722] rounded border border-[#2A3647] mt-1 text-[#EDF3FC] font-mono text-[11px]">
                flow: Next Step Signal
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
