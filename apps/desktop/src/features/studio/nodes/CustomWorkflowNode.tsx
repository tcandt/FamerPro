import React, { memo } from 'react';
import { Handle, Position, NodeProps } from '@xyflow/react';
import {
  Play,
  Rocket,
  Camera,
  HandMetal,
  ShieldCheck,
  Bookmark,
  GitBranch,
  CheckCircle2,
  Sparkles,
} from 'lucide-react';
import { WorkflowNodeData } from '../../../types';

const categoryConfig: Record<
  string,
  {
    icon: React.ElementType;
    color: string;
    bg: string;
    badgeBg: string;
    badgeText: string;
    accentBorder: string;
  }
> = {
  flow: {
    icon: Play,
    color: '#7BA6D6',
    bg: '#17273D',
    badgeBg: '#1E3352',
    badgeText: '#A9C7EB',
    accentBorder: '#2B4A75',
  },
  app: {
    icon: Rocket,
    color: '#1878F5',
    bg: '#122543',
    badgeBg: '#173663',
    badgeText: '#6BB0FF',
    accentBorder: '#1F529E',
  },
  vision: {
    icon: Camera,
    color: '#B291F7',
    bg: '#251C3B',
    badgeBg: '#352554',
    badgeText: '#CDB7FA',
    accentBorder: '#4F387A',
  },
  action: {
    icon: HandMetal,
    color: '#38BDF8',
    bg: '#0F2C3F',
    badgeBg: '#163E59',
    badgeText: '#7DD3FC',
    accentBorder: '#1A5880',
  },
  verify: {
    icon: ShieldCheck,
    color: '#3BC38D',
    bg: '#133528',
    badgeBg: '#1A4D39',
    badgeText: '#6EE7B7',
    accentBorder: '#23694E',
  },
  state: {
    icon: Bookmark,
    color: '#E7B15B',
    bg: '#3B2D17',
    badgeBg: '#543F1E',
    badgeText: '#FDE047',
    accentBorder: '#755829',
  },
  logic: {
    icon: GitBranch,
    color: '#A78BFA',
    bg: '#251D38',
    badgeBg: '#362952',
    badgeText: '#DDD6FE',
    accentBorder: '#503C78',
  },
};

export const CustomWorkflowNode = memo((props: NodeProps) => {
  const nodeData = props.data as unknown as WorkflowNodeData;
  const cat = categoryConfig[nodeData?.category || 'flow'] || categoryConfig.flow;
  const Icon = cat.icon;

  const minConfidence = nodeData?.config?.minConfidence ?? 0.9;
  const marginRatio = nodeData?.config?.marginRatio ?? 0.18;

  return (
    <div
      className={`w-[285px] rounded-xl bg-[#17202E] border transition-all select-none shadow-lg group relative ${
        props.selected
          ? 'border-[#38BDF8] ring-2 ring-[#38BDF8]/40 shadow-[#38BDF8]/10'
          : 'border-[#2A3647] hover:border-[#3D4F67]'
      }`}
    >
      {/* Input Port Handles (Multi-directional for clean routing) */}
      {nodeData?.nodeType !== 'flow.start' && (
        <>
          <Handle
            id="in-left"
            type="target"
            position={Position.Left}
            className="!w-3 !h-3 !-left-1.5 !bg-[#38BDF8] !border-2 !border-[#17202E] !rounded-full transition-transform hover:scale-125 hover:!bg-[#60A5FA]"
            title="Đầu vào (Trái)"
          />
          <Handle
            id="in-top"
            type="target"
            position={Position.Top}
            className="!w-3 !h-3 !-top-1.5 !bg-[#38BDF8] !border-2 !border-[#17202E] !rounded-full transition-transform hover:scale-125 hover:!bg-[#60A5FA]"
            title="Đầu vào (Trên)"
          />
          <Handle
            id="in-right"
            type="target"
            position={Position.Right}
            className="!w-3 !h-3 !-right-1.5 !bg-[#38BDF8] !border-2 !border-[#17202E] !rounded-full transition-transform hover:scale-125 hover:!bg-[#60A5FA]"
            title="Đầu vào (Phải)"
          />
        </>
      )}

      {/* Node Header */}
      <div className="flex items-center justify-between p-3.5 border-b border-[#2A3647]/80">
        <div className="flex items-center gap-2.5 min-w-0">
          <div
            className="w-8 h-8 rounded-lg flex items-center justify-center shrink-0 shadow-inner"
            style={{ backgroundColor: cat.bg, color: cat.color }}
          >
            <Icon size={17} />
          </div>
          <div className="min-w-0">
            <h4 className="font-bold text-[#EDF3FC] text-sm leading-tight truncate">
              {nodeData?.label || 'Node'}
            </h4>
            <span className="text-xs font-mono text-[#8EA2BC] block truncate mt-0.5">
              {nodeData?.nodeType || 'step'}
            </span>
          </div>
        </div>

        <span
          className="text-[10px] font-mono px-2 py-0.5 rounded font-bold shrink-0 uppercase tracking-wider"
          style={{ backgroundColor: cat.badgeBg, color: cat.badgeText }}
        >
          {nodeData?.category || 'flow'}
        </span>
      </div>

      {/* Node Body / Visual Parameters */}
      <div className="p-3.5 space-y-2.5 bg-[#121A26]/50">
        {/* App launch info */}
        {nodeData?.nodeType === 'app.launch' && (
          <div className="flex items-center justify-between p-2 rounded bg-[#101722] border border-[#2A3647] text-xs">
            <span className="text-[#8EA2BC]">Gói:</span>
            <span className="font-mono text-[#6BB0FF] font-semibold truncate max-w-[170px]">
              {nodeData.config?.packageId || 'com.example.game'}
            </span>
          </div>
        )}

        {/* Vision find image */}
        {nodeData?.nodeType === 'vision.find_image' && (
          <div className="space-y-2">
            <div className="flex items-center justify-between p-2 rounded bg-[#101722] border border-[#2A3647] text-xs">
              <span className="text-[#8EA2BC]">Mẫu ảnh:</span>
              <span className="font-mono text-[#CDB7FA] font-medium truncate max-w-[160px]">
                {nodeData.config?.templateRef || 'marker.png'}
              </span>
            </div>
            <div className="space-y-1">
              <div className="flex justify-between text-[11px]">
                <span className="text-[#8EA2BC]">Độ tin cậy tối thiểu</span>
                <span className="text-[#3BC38D] font-mono font-bold">
                  {(minConfidence * 100).toFixed(0)}%
                </span>
              </div>
              <div className="w-full bg-[#101722] h-2 rounded-full overflow-hidden border border-[#2A3647]">
                <div
                  className="bg-[#3BC38D] h-full rounded-full transition-all"
                  style={{ width: `${minConfidence * 100}%` }}
                />
              </div>
            </div>
          </div>
        )}

        {/* Action safe tap */}
        {nodeData?.nodeType === 'action.random_safe_tap' && (
          <div className="space-y-2">
            <div className="grid grid-cols-2 gap-2 text-xs">
              <div className="p-2 rounded bg-[#101722] border border-[#2A3647]">
                <div className="text-[11px] text-[#8EA2BC]">Co biên (Margin)</div>
                <div className="font-mono text-[#38BDF8] font-bold text-xs mt-0.5">
                  {(marginRatio * 100).toFixed(0)}%
                </div>
              </div>
              <div className="p-2 rounded bg-[#101722] border border-[#2A3647]">
                <div className="text-[11px] text-[#8EA2BC]">Tránh click cũ</div>
                <div className="font-mono text-[#EDF3FC] font-bold text-xs mt-0.5">
                  {nodeData.config?.avoidRecentPoints ?? 5} điểm
                </div>
              </div>
            </div>
            <div className="text-[11px] text-[#38BDF8] flex items-center gap-1.5 font-medium">
              <Sparkles size={12} />
              <span>Chống phát hiện tọa độ cố định</span>
            </div>
          </div>
        )}

        {/* Verification */}
        {nodeData?.nodeType === 'verify.wait_for_image' && (
          <div className="space-y-2">
            <div className="flex items-center justify-between p-2 rounded bg-[#101722] border border-[#2A3647] text-xs">
              <span className="text-[#8EA2BC]">Ảnh kỳ vọng:</span>
              <span className="font-mono text-[#6EE7B7] font-medium truncate max-w-[150px]">
                {nodeData.config?.templateRef || 'dialog.png'}
              </span>
            </div>
            <div className="flex items-center gap-1.5 text-xs text-[#3BC38D] font-medium">
              <CheckCircle2 size={13} />
              <span>Xác nhận giao diện sau thao tác</span>
            </div>
          </div>
        )}

        {/* Checkpoint */}
        {nodeData?.nodeType === 'state.checkpoint' && (
          <div className="flex items-center justify-between p-2 rounded bg-[#101722] border border-[#2A3647] text-xs">
            <span className="text-[#8EA2BC]">Mốc lưu:</span>
            <span className="font-mono text-[#FDE047] font-bold truncate max-w-[160px]">
              {nodeData.config?.name || 'quest_checkpoint'}
            </span>
          </div>
        )}

        {/* Flow / Generic */}
        {(nodeData?.nodeType === 'flow.start' ||
          nodeData?.nodeType === 'flow.end' ||
          nodeData?.nodeType === 'vision.capture') && (
          <div className="flex items-center justify-between text-xs text-[#8EA2BC] py-1">
            <span>
              {nodeData?.nodeType === 'flow.start'
                ? 'Điểm bắt đầu thực thi'
                : nodeData?.nodeType === 'flow.end'
                ? 'Điểm kết thúc quy trình'
                : 'Chụp khung hình PNG'}
            </span>
            <span className="text-xs font-mono font-bold text-[#3BC38D]">SẴN SÀNG</span>
          </div>
        )}
      </div>

      {/* Output Port Handles (Multi-directional for clean routing) */}
      {nodeData?.nodeType !== 'flow.end' && (
        <>
          <Handle
            id="out-right"
            type="source"
            position={Position.Right}
            className="!w-3 !h-3 !-right-1.5 !bg-[#1878F5] !border-2 !border-[#17202E] !rounded-full transition-transform hover:scale-125 hover:!bg-[#3B82F6]"
            title="Đầu ra (Phải)"
          />
          <Handle
            id="out-bottom"
            type="source"
            position={Position.Bottom}
            className="!w-3 !h-3 !-bottom-1.5 !bg-[#1878F5] !border-2 !border-[#17202E] !rounded-full transition-transform hover:scale-125 hover:!bg-[#3B82F6]"
            title="Đầu ra (Dưới)"
          />
          <Handle
            id="out-left"
            type="source"
            position={Position.Left}
            className="!w-3 !h-3 !-left-1.5 !bg-[#1878F5] !border-2 !border-[#17202E] !rounded-full transition-transform hover:scale-125 hover:!bg-[#3B82F6]"
            title="Đầu ra (Trái)"
          />
        </>
      )}
    </div>
  );
});
