import React, { useCallback, useEffect, useMemo, useState } from 'react';
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
import {
  LayoutGrid,
  MapPin,
  SlidersHorizontal,
  Maximize2,
  Save,
  Plus,
  Play,
  Pause,
  RotateCcw,
  BookmarkCheck,
  Square as StopSquare,
  FolderGit2,
} from 'lucide-react';

import { useWorkflowStore } from '../../stores/workflowStore';
import { useProjectStore } from '../../stores/projectStore';
import { useDeviceStore } from '../../stores/deviceStore';
import { api, isTauri } from '../../lib/api';
import { CustomWorkflowNode } from './nodes/CustomWorkflowNode';
import { NodePalette } from './NodePalette';
import { NodeInspector } from './NodeInspector';
import { ExecutionDock } from './ExecutionDock';

const ProjectWorkflowHeaderBar: React.FC = () => {
  const {
    workflowId,
    workflowName,
    isDirty,
    isRunning,
    activeJobId,
    breakpointNodeIds,
    setWorkflowName,
    setIsRunning,
    setActiveJobState,
    setRunEvents,
    setLastExecutionNotice,
    serializeWorkflowJson,
    newBlankWorkflow,
    loadWorkflow,
    markSaved,
  } = useWorkflowStore();
  const { activeProjectId, activeProjectDetail, refreshActiveProject } = useProjectStore();
  const { selectedDeviceId } = useDeviceStore();

  const [saving, setSaving] = useState(false);
  const [newWfModal, setNewWfModal] = useState(false);
  const [newWfName, setNewWfName] = useState('');

  const projectWorkflows = activeProjectDetail?.workflows || [];

  // Auto-load first workflow from Active Project when switching projects cleanly
  useEffect(() => {
    if (!activeProjectDetail) return;
    const matchesCurrent = projectWorkflows.some((w) => w.id === workflowId);
    if (!matchesCurrent && projectWorkflows.length > 0) {
      try {
        const parsed = JSON.parse(projectWorkflows[0].draftJson);
        loadWorkflow(parsed);
      } catch (_) {}
    }
  }, [activeProjectDetail?.project.id]);

  const handleSelectProjectWorkflow = (wfId: string) => {
    const found = projectWorkflows.find((w) => w.id === wfId);
    if (!found) return;
    try {
      const parsed = JSON.parse(found.draftJson);
      loadWorkflow(parsed);
      setLastExecutionNotice(`Đã nạp workflow '${found.name}' từ SQLite.`);
    } catch (e: any) {
      setLastExecutionNotice(`Lỗi đọc draftJson: ${e?.message || e}`);
    }
  };

  const handleSaveToProject = async () => {
    if (!activeProjectId) {
      setLastExecutionNotice('Chưa chọn Active Project trên Header.');
      return;
    }
    setSaving(true);
    try {
      const jsonStr = serializeWorkflowJson('published');
      await api.studioSaveProjectWorkflow(
        activeProjectId,
        workflowId,
        workflowName,
        'Studio',
        jsonStr
      );
      markSaved();
      await refreshActiveProject();
      setLastExecutionNotice(`Đã lưu '${workflowName}' vào Project SQLite.`);
    } catch (err: any) {
      setLastExecutionNotice(`Lỗi lưu workflow: ${err?.message || String(err)}`);
    } finally {
      setSaving(false);
    }
  };

  const handleCreateWorkflow = async (e: React.FormEvent) => {
    e.preventDefault();
    const cleanName = newWfName.trim() || 'Workflow Mới';
    const newId = `wf-${Date.now().toString(36)}`;
    newBlankWorkflow(newId, cleanName, activeProjectDetail?.project.gamePackageId);
    setNewWfModal(false);
    setNewWfName('');
    if (activeProjectId && isTauri()) {
      setTimeout(async () => {
        try {
          const jsonStr = useWorkflowStore.getState().serializeWorkflowJson('published');
          await api.studioSaveProjectWorkflow(activeProjectId, newId, cleanName, 'Studio', jsonStr);
          useWorkflowStore.getState().markSaved();
          await refreshActiveProject();
          setLastExecutionNotice(`Đã tạo và lưu workflow '${cleanName}' vào Project.`);
        } catch (err: any) {
          setLastExecutionNotice(`Lỗi tạo workflow: ${err?.message || err}`);
        }
      }, 20);
    }
  };

  const handleStartCoordinator = async () => {
    if (!isTauri() || !activeProjectId) {
      setLastExecutionNotice('Cần mở trong FarmerPro Desktop và chọn Active Project.');
      return;
    }
    if (!selectedDeviceId) {
      setLastExecutionNotice(
        'LIVE HARDWARE BLOCKED: Chưa có thiết bị ADB gắn sẵn. Kết nối thiết bị trước khi chạy ADB thật.'
      );
      return;
    }
    setIsRunning(true);
    try {
      const jsonStr = serializeWorkflowJson('published');
      await api.studioSaveProjectWorkflow(activeProjectId, workflowId, workflowName, 'Studio', jsonStr);
      markSaved();
      const started = await api.studioStartProjectWorkflow({
        projectId: activeProjectId,
        workflowId,
        deviceId: selectedDeviceId,
        priority: 5,
        leaseTtlSecs: 20,
        workflowTimeoutMs: 25000,
      });
      setActiveJobState(
        started.jobId,
        null,
        null,
        `Đã khởi chạy Coordinator job ${started.jobId} trên ${started.deviceId}`
      );
      const detail = await api.studioGetJobResumeDetail(started.jobId);
      if (detail) {
        setActiveJobState(started.jobId, detail, null);
        setRunEvents(detail.stepEvents);
      }
    } catch (err: any) {
      setLastExecutionNotice(`Coordinator báo lỗi: ${err?.message || String(err)}`);
    } finally {
      setIsRunning(false);
    }
  };

  return (
    <div className="h-10 px-3 bg-[#161E29] border-b border-[#2A3647] flex items-center justify-between gap-2 text-xs shrink-0">
      {/* Left: Active Project Workflows Selector & Rename */}
      <div className="flex items-center gap-2 min-w-0">
        <div className="flex items-center gap-1.5 text-[#A9B8CC] shrink-0">
          <FolderGit2 size={13} className="text-[#4B9CFF]" />
          <span className="font-mono text-[11px] text-[#74869D]">
            {activeProjectDetail?.project.slug || 'no-project'}
          </span>
        </div>

        <select
          value={projectWorkflows.some((w) => w.id === workflowId) ? workflowId : ''}
          onChange={(e) => handleSelectProjectWorkflow(e.target.value)}
          className="px-2 py-1 rounded bg-[#101722] border border-[#2A3647] text-[#EDF3FC] text-xs max-w-[200px]"
          aria-label="Project Workflow Selector"
        >
          <option value="" disabled>
            {projectWorkflows.length === 0 ? 'Chưa có WF đã lưu' : 'Chọn workflow...'}
          </option>
          {projectWorkflows.map((w) => (
            <option key={w.id} value={w.id}>
              {w.name} ({w.id})
            </option>
          ))}
        </select>

        <input
          value={workflowName}
          onChange={(e) => setWorkflowName(e.target.value)}
          className="px-2 py-1 rounded bg-[#101722] border border-[#2A3647] focus:border-[#4B9CFF] outline-none text-[#EDF3FC] text-xs w-44"
          title="Tên workflow"
        />

        <button
          onClick={() => setNewWfModal(true)}
          className="px-2 py-1 rounded bg-[#202B3B] hover:bg-[#2A3647] text-[#3BC38D] flex items-center gap-1 cursor-pointer shrink-0"
          title="Tạo workflow mới trong Project"
        >
          <Plus size={12} />
          <span>Tạo luồng</span>
        </button>

        <button
          onClick={handleSaveToProject}
          disabled={saving || !activeProjectId}
          className="px-2.5 py-1 rounded bg-[#1878F5] hover:bg-[#2F8CFF] text-white font-semibold flex items-center gap-1 cursor-pointer disabled:opacity-40 shrink-0"
        >
          <Save size={12} />
          <span>{saving ? 'Đang lưu...' : isDirty ? 'Lưu Project *' : 'Đã lưu'}</span>
        </button>
      </div>

      {/* Right: RuntimeCoordinator Controls + Breakpoint Count */}
      <div className="flex items-center gap-1.5 shrink-0">
        {breakpointNodeIds.length > 0 && (
          <span className="px-2 py-0.5 rounded fp-badge-testkit text-[10.5px] font-mono">
            BP: {breakpointNodeIds.length}
          </span>
        )}

        <button
          onClick={handleStartCoordinator}
          disabled={isRunning}
          className="px-2.5 py-1 rounded bg-[#133528] hover:bg-[#194735] border border-[#3BC38D]/40 text-[#3BC38D] font-semibold flex items-center gap-1 cursor-pointer disabled:opacity-40"
          title="Chạy trực tiếp qua RuntimeCoordinator P3"
        >
          <Play size={11} fill="currentColor" />
          <span>Coordinator</span>
        </button>

        {activeJobId && (
          <>
            <button
              onClick={() => api.studioPauseJob(activeJobId).catch((e) => setLastExecutionNotice(String(e)))}
              className="p-1.5 rounded bg-[#202B3B] hover:bg-[#2A3647] text-[#E7B15B] cursor-pointer"
              title="Pause Job"
            >
              <Pause size={12} />
            </button>
            <button
              onClick={() => api.studioResumeJob(activeJobId).catch((e) => setLastExecutionNotice(String(e)))}
              className="p-1.5 rounded bg-[#202B3B] hover:bg-[#2A3647] text-[#3BC38D] cursor-pointer"
              title="Resume Job"
            >
              <RotateCcw size={12} />
            </button>
            <button
              onClick={() =>
                api
                  .studioSuspendJobToCheckpoint(activeJobId)
                  .catch((e) => setLastExecutionNotice(String(e)))
              }
              className="p-1.5 rounded bg-[#202B3B] hover:bg-[#2A3647] text-[#4B9CFF] cursor-pointer"
              title="Suspend to SQLite Checkpoint"
            >
              <BookmarkCheck size={12} />
            </button>
            <button
              onClick={() => api.studioCancelJob(activeJobId).catch((e) => setLastExecutionNotice(String(e)))}
              className="p-1.5 rounded bg-[#3B1717] hover:bg-[#531e1e] text-[#F27883] cursor-pointer"
              title="Cancel Job"
            >
              <StopSquare size={12} />
            </button>
          </>
        )}
      </div>

      {newWfModal && (
        <div className="fixed inset-0 z-50 bg-black/60 flex items-center justify-center p-4">
          <form
            onSubmit={handleCreateWorkflow}
            className="w-80 bg-[#192230] border border-[#2A3647] rounded-lg p-4 space-y-3 shadow-2xl"
          >
            <div className="font-semibold text-sm text-[#EDF3FC]">Tạo Workflow Mới</div>
            <input
              value={newWfName}
              onChange={(e) => setNewWfName(e.target.value)}
              placeholder="VD: Thu thập tài nguyên"
              autoFocus
              className="w-full px-3 py-2 rounded bg-[#101722] border border-[#2A3647] text-[#EDF3FC] text-xs"
            />
            <div className="flex justify-end gap-2">
              <button
                type="button"
                onClick={() => setNewWfModal(false)}
                className="px-3 py-1 rounded bg-[#202B3B] text-[#A9B8CC] cursor-pointer"
              >
                Hủy
              </button>
              <button
                type="submit"
                className="px-3 py-1 rounded bg-[#1878F5] text-white font-semibold cursor-pointer"
              >
                Tạo
              </button>
            </div>
          </form>
        </div>
      )}
    </div>
  );
};

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
        className="flex items-center gap-1.5 px-2.5 py-1 text-[#A9B8CC] hover:text-white hover:bg-[#202B3B] rounded transition-colors cursor-pointer"
        title="Tự động căn chỉnh node"
      >
        <LayoutGrid size={13} className="text-[#38BDF8]" />
        <span>Sắp xếp tự động</span>
      </button>

      <button
        onClick={() => fitView({ duration: 300, padding: 0.1 })}
        className="flex items-center gap-1.5 px-2.5 py-1 text-[#A9B8CC] hover:text-white hover:bg-[#202B3B] rounded transition-colors cursor-pointer"
        title="Vừa khung hình (Fit View)"
      >
        <Maximize2 size={13} className="text-[#3BC38D]" />
        <span>Vừa khung hình</span>
      </button>

      <div className="h-3.5 w-px bg-[#2A3647] mx-0.5" />

      <button
        onClick={toggleMiniMap}
        className={`p-1.5 rounded transition-colors cursor-pointer ${
          showMiniMap ? 'bg-[#1878F5]/20 text-[#38BDF8]' : 'text-[#74869D] hover:text-white hover:bg-[#202B3B]'
        }`}
        title={showMiniMap ? 'Ẩn MiniMap' : 'Hiện MiniMap'}
      >
        <MapPin size={14} />
      </button>

      <button
        onClick={toggleInspector}
        className={`p-1.5 rounded transition-colors cursor-pointer ${
          !isInspectorCollapsed
            ? 'bg-[#1878F5]/20 text-[#38BDF8]'
            : 'text-[#74869D] hover:text-white hover:bg-[#202B3B]'
        }`}
        title={isInspectorCollapsed ? 'Hiện Inspector' : 'Ẩn Inspector'}
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
    <div className="flex flex-col flex-1 h-full w-full overflow-hidden select-none">
      <ProjectWorkflowHeaderBar />
      <div className="flex flex-1 overflow-hidden">
        <NodePalette />

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
              fitViewOptions={{ padding: 0.12, minZoom: 0.2, maxZoom: 1.15 }}
              minZoom={0.2}
              maxZoom={2.0}
              className="w-full h-full"
            >
              <Background variant={BackgroundVariant.Dots} gap={20} size={1.2} color="#253244" />
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

          <ExecutionDock />
        </div>

        <NodeInspector />
      </div>
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
