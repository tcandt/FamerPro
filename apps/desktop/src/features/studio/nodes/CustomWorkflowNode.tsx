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
} from 'lucide-react';
import { WorkflowNodeData } from '../../../types';

const categoryConfig: Record<string, { icon: React.ElementType; color: string; bg: string }> = {
  flow: { icon: Play, color: '#7BA6D6', bg: '#173355' },
  app: { icon: Rocket, color: '#1878F5', bg: '#173355' },
  vision: { icon: Camera, color: '#B291F7', bg: '#2A1F45' },
  action: { icon: HandMetal, color: '#4B9CFF', bg: '#173355' },
  verify: { icon: ShieldCheck, color: '#3BC38D', bg: '#133528' },
  state: { icon: Bookmark, color: '#E7B15B', bg: '#3B2D17' },
  logic: { icon: GitBranch, color: '#7BA6D6', bg: '#173355' },
};

export const CustomWorkflowNode = memo((props: NodeProps) => {
  const nodeData = props.data as unknown as WorkflowNodeData;
  const cat = categoryConfig[nodeData?.category || 'flow'] || categoryConfig.flow;
  const Icon = cat.icon;

  return (
    <div
      className={`min-w-[200px] max-w-[240px] rounded-[10px] bg-[#192230] border transition-all text-xs select-none shadow-md ${
        props.selected
          ? 'border-[#4B9CFF] ring-2 ring-[#4B9CFF]/30'
          : 'border-[#2A3647] hover:border-[#3B4B61]'
      }`}
    >
      {/* Input Port */}
      {nodeData?.nodeType !== 'flow.start' && (
        <Handle
          type="target"
          position={Position.Left}
          className="!w-2.5 !h-2.5 !bg-[#7BA6D6] !border-2 !border-[#192230]"
        />
      )}

      {/* Header */}
      <div className="flex items-center justify-between p-2.5 border-b border-[#2A3647]">
        <div className="flex items-center gap-2">
          <div
            className="w-5 h-5 rounded flex items-center justify-center shrink-0"
            style={{ backgroundColor: cat.bg, color: cat.color }}
          >
            <Icon size={12} />
          </div>
          <span className="font-semibold text-[#EDF3FC] truncate">{nodeData?.label || 'Node'}</span>
        </div>
        <span className="text-[10px] font-mono text-[#74869D] uppercase">
          {nodeData?.nodeType ? nodeData.nodeType.split('.')[1] || nodeData.nodeType : ''}
        </span>
      </div>

      {/* Node Config Summary */}
      <div className="p-2.5 space-y-1">
        {nodeData?.config?.packageId && (
          <div className="text-[11px] text-[#A9B8CC] truncate">
            <span className="text-[#74869D]">Gói: </span>
            {nodeData.config.packageId}
          </div>
        )}
        {nodeData?.config?.templateRef && (
          <div className="text-[11px] text-[#B291F7] truncate">
            <span className="text-[#74869D]">Ảnh: </span>
            {nodeData.config.templateRef}
          </div>
        )}
        {nodeData?.config?.minConfidence && (
          <div className="text-[11px] text-[#3BC38D]">
            <span className="text-[#74869D]">Độ tin cậy: </span>
            ≥ {nodeData.config.minConfidence * 100}%
          </div>
        )}
        {nodeData?.config?.marginRatio && (
          <div className="text-[11px] text-[#4B9CFF]">
            <span className="text-[#74869D]">Biên an toàn: </span>
            {nodeData.config.marginRatio * 100}%
          </div>
        )}
        {Object.keys(nodeData?.config || {}).length === 0 && (
          <span className="text-[11px] text-[#74869D] italic">Mặc định</span>
        )}
      </div>

      {/* Output Port */}
      {nodeData?.nodeType !== 'flow.end' && (
        <Handle
          type="source"
          position={Position.Right}
          className="!w-2.5 !h-2.5 !bg-[#1878F5] !border-2 !border-[#192230]"
        />
      )}
    </div>
  );
});
