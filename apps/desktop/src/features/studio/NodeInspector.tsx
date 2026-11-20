import React, { useState } from 'react';
import { Sliders, ArrowLeftRight, RefreshCw, FileText, Info, Trash2 } from 'lucide-react';
import { useWorkflowStore } from '../../stores/workflowStore';

export const NodeInspector: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'config' | 'ports' | 'retry' | 'notes'>('config');
  const { nodes, selectedNodeId, updateNodeConfig, setNodes, selectNode } = useWorkflowStore();

  const selectedNode = nodes.find((n) => n.id === selectedNodeId);

  const handleDeleteNode = () => {
    if (!selectedNodeId) return;
    setNodes(nodes.filter((n) => n.id !== selectedNodeId));
    selectNode(null);
  };

  if (!selectedNode) {
    return (
      <div className="w-[312px] bg-[#161E29] border-l border-[#2A3647] flex flex-col items-center justify-center p-6 text-center select-none shrink-0 z-20">
        <div className="w-10 h-10 rounded-full bg-[#192230] border border-[#2A3647] flex items-center justify-center text-[#74869D] mb-3">
          <Info size={18} />
        </div>
        <h4 className="text-xs font-semibold text-[#EDF3FC] mb-1">Chưa chọn Node</h4>
        <p className="text-[11px] text-[#74869D] leading-relaxed">
          Nhấp vào bất kỳ node nào trên đồ thị để xem và chỉnh sửa thuộc tính, chính sách retry và tham số kiểm tra.
        </p>
      </div>
    );
  }

  const { data } = selectedNode;
  const config = data.config || {};

  return (
    <div className="w-[312px] bg-[#161E29] border-l border-[#2A3647] flex flex-col h-full select-none shrink-0 z-20">
      {/* Header */}
      <div className="p-3 border-b border-[#2A3647] flex items-center justify-between">
        <div>
          <span className="text-[10px] text-[#74869D] uppercase font-mono">{selectedNode.id}</span>
          <h3 className="text-xs font-semibold text-[#EDF3FC]">{data.label}</h3>
        </div>
        <button
          onClick={handleDeleteNode}
          className="p-1.5 text-[#74869D] hover:text-[#F27883] hover:bg-[#202B3B] rounded transition-colors"
          title="Xóa node"
        >
          <Trash2 size={14} />
        </button>
      </div>

      {/* Tabs */}
      <div className="flex border-b border-[#2A3647] bg-[#121A26]">
        <button
          onClick={() => setActiveTab('config')}
          className={`flex-1 py-2 text-[11px] font-medium flex items-center justify-center gap-1 border-b-2 ${
            activeTab === 'config'
              ? 'text-[#1878F5] border-[#1878F5] bg-[#161E29]'
              : 'text-[#74869D] border-transparent hover:text-[#A9B8CC]'
          }`}
        >
          <Sliders size={12} />
          <span>Thiết lập</span>
        </button>
        <button
          onClick={() => setActiveTab('ports')}
          className={`flex-1 py-2 text-[11px] font-medium flex items-center justify-center gap-1 border-b-2 ${
            activeTab === 'ports'
              ? 'text-[#1878F5] border-[#1878F5] bg-[#161E29]'
              : 'text-[#74869D] border-transparent hover:text-[#A9B8CC]'
          }`}
        >
          <ArrowLeftRight size={12} />
          <span>Cổng IO</span>
        </button>
        <button
          onClick={() => setActiveTab('retry')}
          className={`flex-1 py-2 text-[11px] font-medium flex items-center justify-center gap-1 border-b-2 ${
            activeTab === 'retry'
              ? 'text-[#1878F5] border-[#1878F5] bg-[#161E29]'
              : 'text-[#74869D] border-transparent hover:text-[#A9B8CC]'
          }`}
        >
          <RefreshCw size={12} />
          <span>Retry</span>
        </button>
        <button
          onClick={() => setActiveTab('notes')}
          className={`flex-1 py-2 text-[11px] font-medium flex items-center justify-center gap-1 border-b-2 ${
            activeTab === 'notes'
              ? 'text-[#1878F5] border-[#1878F5] bg-[#161E29]'
              : 'text-[#74869D] border-transparent hover:text-[#A9B8CC]'
          }`}
        >
          <FileText size={12} />
          <span>Ghi chú</span>
        </button>
      </div>

      {/* Form Content */}
      <div className="flex-1 overflow-y-auto p-4 space-y-4 text-xs">
        {activeTab === 'config' && (
          <div className="space-y-3">
            {/* Package Id (for app.launch) */}
            {data.nodeType === 'app.launch' && (
              <div>
                <label className="block text-[#A9B8CC] mb-1 font-medium">Tên gói (Package Name)</label>
                <input
                  type="text"
                  value={config.packageId || ''}
                  onChange={(e) => updateNodeConfig(selectedNode.id, { packageId: e.target.value })}
                  placeholder="com.example.game"
                  className="w-full bg-[#101722] border border-[#2A3647] rounded px-2.5 py-1.5 text-[#EDF3FC] focus:border-[#4B9CFF] outline-none"
                />
              </div>
            )}

            {/* Template Ref (for vision.find_image / verify) */}
            {(data.nodeType === 'vision.find_image' || data.nodeType === 'verify.wait_for_image') && (
              <>
                <div>
                  <label className="block text-[#A9B8CC] mb-1 font-medium">Tệp ảnh mẫu (Template)</label>
                  <input
                    type="text"
                    value={config.templateRef || ''}
                    onChange={(e) => updateNodeConfig(selectedNode.id, { templateRef: e.target.value })}
                    placeholder="assets/templates/marker.png"
                    className="w-full bg-[#101722] border border-[#2A3647] rounded px-2.5 py-1.5 text-[#EDF3FC] focus:border-[#4B9CFF] outline-none"
                  />
                </div>
                <div>
                  <label className="block text-[#A9B8CC] mb-1 font-medium">
                    Độ tin cậy tối thiểu ({(config.minConfidence ?? 0.9) * 100}%)
                  </label>
                  <input
                    type="range"
                    min="0.5"
                    max="1.0"
                    step="0.01"
                    value={config.minConfidence ?? 0.9}
                    onChange={(e) =>
                      updateNodeConfig(selectedNode.id, { minConfidence: parseFloat(e.target.value) })
                    }
                    className="w-full accent-[#1878F5]"
                  />
                </div>
              </>
            )}

            {/* Safe Random Tap configuration */}
            {data.nodeType === 'action.random_safe_tap' && (
              <>
                <div>
                  <label className="block text-[#A9B8CC] mb-1 font-medium">
                    Tỷ lệ co biên an toàn (Margin: {(config.marginRatio ?? 0.18) * 100}%)
                  </label>
                  <input
                    type="range"
                    min="0.05"
                    max="0.4"
                    step="0.01"
                    value={config.marginRatio ?? 0.18}
                    onChange={(e) =>
                      updateNodeConfig(selectedNode.id, { marginRatio: parseFloat(e.target.value) })
                    }
                    className="w-full accent-[#1878F5]"
                  />
                  <p className="text-[10px] text-[#74869D] mt-1">
                    Co khung nhận diện vào trong để điểm click không nằm sát mép nút.
                  </p>
                </div>
                <div>
                  <label className="block text-[#A9B8CC] mb-1 font-medium">Tránh N điểm bấm gần nhất</label>
                  <input
                    type="number"
                    min="1"
                    max="20"
                    value={config.avoidRecentPoints ?? 5}
                    onChange={(e) =>
                      updateNodeConfig(selectedNode.id, { avoidRecentPoints: parseInt(e.target.value) || 1 })
                    }
                    className="w-full bg-[#101722] border border-[#2A3647] rounded px-2.5 py-1.5 text-[#EDF3FC] outline-none"
                  />
                </div>
              </>
            )}

            {/* Checkpoint Name */}
            {data.nodeType === 'state.checkpoint' && (
              <div>
                <label className="block text-[#A9B8CC] mb-1 font-medium">Tên mốc (Checkpoint Name)</label>
                <input
                  type="text"
                  value={config.name || ''}
                  onChange={(e) => updateNodeConfig(selectedNode.id, { name: e.target.value })}
                  placeholder="quest_active"
                  className="w-full bg-[#101722] border border-[#2A3647] rounded px-2.5 py-1.5 text-[#EDF3FC] outline-none"
                />
              </div>
            )}
          </div>
        )}

        {activeTab === 'ports' && (
          <div className="space-y-3">
            <div>
              <span className="text-[#74869D] font-medium">Cổng vào (Input Port):</span>
              <div className="p-2 bg-[#101722] rounded border border-[#2A3647] mt-1 text-[#EDF3FC]">
                flow: Execution Context
              </div>
            </div>
            <div>
              <span className="text-[#74869D] font-medium">Cổng ra (Output Port):</span>
              <div className="p-2 bg-[#101722] rounded border border-[#2A3647] mt-1 text-[#EDF3FC]">
                flow: Next Node
              </div>
            </div>
          </div>
        )}

        {activeTab === 'retry' && (
          <div className="space-y-3">
            <div>
              <label className="block text-[#A9B8CC] mb-1 font-medium">Số lần thử lại tối đa (Max Attempts)</label>
              <input
                type="number"
                min="1"
                max="10"
                defaultValue={3}
                className="w-full bg-[#101722] border border-[#2A3647] rounded px-2.5 py-1.5 text-[#EDF3FC] outline-none"
              />
            </div>
            <div>
              <label className="block text-[#A9B8CC] mb-1 font-medium">Thời gian chờ giãn cách (Backoff ms)</label>
              <input
                type="number"
                step="100"
                defaultValue={1200}
                className="w-full bg-[#101722] border border-[#2A3647] rounded px-2.5 py-1.5 text-[#EDF3FC] outline-none"
              />
            </div>
            <div>
              <label className="block text-[#A9B8CC] mb-1 font-medium">Hành vi khi lỗi liên tục</label>
              <select className="w-full bg-[#101722] border border-[#2A3647] rounded px-2.5 py-1.5 text-[#EDF3FC] outline-none">
                <option value="pause">Tạm dừng để người dùng duyệt (Pause for review)</option>
                <option value="fail">Đánh dấu thất bại (Fail closed)</option>
              </select>
            </div>
          </div>
        )}

        {activeTab === 'notes' && (
          <div>
            <label className="block text-[#A9B8CC] mb-1 font-medium">Ghi chú hướng dẫn</label>
            <textarea
              rows={4}
              placeholder="Nhập hướng dẫn dành cho người vận hành..."
              className="w-full bg-[#101722] border border-[#2A3647] rounded p-2 text-[#EDF3FC] outline-none resize-none"
            />
          </div>
        )}
      </div>
    </div>
  );
};
