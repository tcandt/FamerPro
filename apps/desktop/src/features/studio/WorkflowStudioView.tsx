import React, { useCallback, useMemo } from 'react';
import {
  ReactFlow,
  Controls,
  Background,
  MiniMap,
  BackgroundVariant,
  Node,
  NodeTypes,
  useReactFlow,
  ReactFlowProvider,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { LayoutGrid, MapPin, SlidersHorizontal, Maximize2 } from 'lucide-react';

import { useWorkflowStore } from '../../stores/workflowStore';
import { CustomWorkflowNode } from './nodes/CustomWorkflowNode';
import { NodePalette } from './NodePalette';
import { NodeInspector } from './NodeInspector';
import { ExecutionDock } from './ExecutionDock';

const CanvasToolbar: React.FC = () => {
  const { fitView } = useReactFlow();
  const { autoLayout, showMiniMap, toggleMiniMap, isInspectorCollapsed, toggleInspector } =
    useWorkflowStore();

  const handleAutoLayout = () => {
    autoLayout();
    setTimeout(() => {
      fitView({ duration: 400, padding: 0.25 });
    }, 50);
  };

  return (
    <div className="absolute top-3 right-3 z-10 flex items-center gap-1.5 bg-[#161E29]/90 backdrop-blur-sm border border-[#2A3647] rounded-lg p-1 shadow-md text-xs">
      <button
        onClick={handleAutoLayout}
        className="flex items-center gap-1.5 px-2.5 py-1 text-[#A9B8CC] hover:text-white hover:bg-[#202B3B] rounded transition-colors"
        title="Tự động căn chỉnh node theo luồng cân đối"
      >
        <LayoutGrid size={13} className="text-[#38BDF8]" />
        <span>Sắp xếp tự động</span>
      </button>

      <button
        onClick={() => fitView({ duration: 300, padding: 0.2 })}
        className="flex items-center gap-1.5 px-2.5 py-1 text-[#A9B8CC] hover:text-white hover:bg-[#202B3B] rounded transition-colors"
        title="Thu phóng vừa vặn toàn bộ đồ thị (Fit View)"
      >
        <Maximize2 size={13} className="text-[#3BC38D]" />
        <span>Vừa khung hình</span>
      </button>

      <div className="h-3.5 w-px bg-[#2A3647] mx-0.5" />

      <button
        onClick={toggleMiniMap}
        className={`p-1.5 rounded transition-colors ${
          showMiniMap ? 'bg-[#1878F5]/20 text-[#38BDF8]' : 'text-[#74869D] hover:text-white hover:bg-[#202B3B]'
        }`}
        title={showMiniMap ? 'Ẩn bản đồ thu nhỏ (MiniMap)' : 'Hiện bản đồ thu nhỏ'}
      >
        <MapPin size={14} />
      </button>

      <button
        onClick={toggleInspector}
        className={`p-1.5 rounded transition-colors ${
          !isInspectorCollapsed
            ? 'bg-[#1878F5]/20 text-[#38BDF8]'
            : 'text-[#74869D] hover:text-white hover:bg-[#202B3B]'
        }`}
        title={isInspectorCollapsed ? 'Hiện bảng thuộc tính Node' : 'Ẩn bảng thuộc tính (Toàn màn hình canvas)'}
      >
        <SlidersHorizontal size={14} />
      </button>
    </div>
  );
};

export const WorkflowStudioContent: React.FC = () => {
  const {
    nodes,
    edges,
    onNodesChange,
    onEdgesChange,
    onConnect,
    selectNode,
    showMiniMap,
  } = useWorkflowStore();

  const nodeTypes: NodeTypes = useMemo(
    () => ({
      custom: CustomWorkflowNode as any,
    }),
    []
  );

  const handleNodeClick = useCallback(
    (_: React.MouseEvent, node: Node) => {
      selectNode(node.id);
    },
    [selectNode]
  );

  const handlePaneClick = useCallback(() => {
    selectNode(null);
  }, [selectNode]);

  return (
    <div className="flex flex-1 h-full w-full overflow-hidden select-none">
      {/* Left: Node Palette */}
      <NodePalette />

      {/* Center: Canvas Hero + Bottom Execution Dock */}
      <div className="flex-1 flex flex-col h-full overflow-hidden relative">
        <div className="flex-1 relative w-full h-full bg-[#101722]">
          <ReactFlow
            nodes={nodes as any}
            edges={edges}
            nodeTypes={nodeTypes}
            onNodesChange={onNodesChange as any}
            onEdgesChange={onEdgesChange}
            onConnect={onConnect}
            onNodeClick={handleNodeClick}
            onPaneClick={handlePaneClick}
            fitView
            fitViewOptions={{ padding: 0.18, minZoom: 0.45, maxZoom: 1.15 }}
            minZoom={0.35}
            maxZoom={2.0}
            className="w-full h-full"
          >
            <Background
              variant={BackgroundVariant.Dots}
              gap={20}
              size={1.2}
              color="#253244"
            />
            <Controls className="!bg-[#161E29] !border-[#2A3647] !text-[#EDF3FC] !rounded-lg !shadow-lg" />
            {showMiniMap && (
              <MiniMap
                nodeStrokeColor="#38BDF8"
                nodeColor="#192230"
                maskColor="rgba(16, 23, 34, 0.75)"
                className="!bg-[#141C28] !border-[#2A3647] !rounded-lg !shadow-lg"
              />
            )}
          </ReactFlow>

          <CanvasToolbar />
        </div>

        {/* Bottom Execution Dock */}
        <ExecutionDock />
      </div>

      {/* Right: Contextual Node Inspector */}
      <NodeInspector />
    </div>
  );
};

export const WorkflowStudioView: React.FC = () => {
  return (
    <ReactFlowProvider>
      <WorkflowStudioContent />
    </ReactFlowProvider>
  );
};
