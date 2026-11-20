import React, { useCallback, useMemo } from 'react';
import {
  ReactFlow,
  Controls,
  Background,
  MiniMap,
  BackgroundVariant,
  Node,
  NodeTypes,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';

import { useWorkflowStore } from '../../stores/workflowStore';
import { CustomWorkflowNode } from './nodes/CustomWorkflowNode';
import { NodePalette } from './NodePalette';
import { NodeInspector } from './NodeInspector';
import { ExecutionDock } from './ExecutionDock';

export const WorkflowStudioView: React.FC = () => {
  const {
    nodes,
    edges,
    onNodesChange,
    onEdgesChange,
    onConnect,
    selectNode,
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
        <div className="flex-1 relative w-full h-full bg-[#121A26]">
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
            defaultViewport={{ x: 50, y: 50, zoom: 0.85 }}
            minZoom={0.2}
            maxZoom={2.0}
            className="w-full h-full"
          >
            <Background
              variant={BackgroundVariant.Dots}
              gap={16}
              size={1}
              color="#2A3647"
            />
            <Controls className="!bg-[#192230] !border-[#2A3647]" />
            <MiniMap
              nodeStrokeColor="#4B9CFF"
              nodeColor="#192230"
              maskColor="rgba(16, 23, 34, 0.7)"
              className="!bg-[#161E29] !border-[#2A3647]"
            />
          </ReactFlow>
        </div>

        {/* Bottom Execution Dock */}
        <ExecutionDock />
      </div>

      {/* Right: Contextual Node Inspector */}
      <NodeInspector />
    </div>
  );
};
