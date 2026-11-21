import { create } from 'zustand';
import { Node, Edge, applyNodeChanges, applyEdgeChanges, NodeChange, EdgeChange, Connection, addEdge } from '@xyflow/react';
import { WorkflowNodeData, StepEvent, ValidationReport } from '../types';

interface WorkflowState {
  workflowId: string;
  workflowName: string;
  isDirty: boolean;
  nodes: Node<WorkflowNodeData>[];
  edges: Edge[];
  selectedNodeId: string | null;
  isRunning: boolean;
  runEvents: StepEvent[];
  activeDockTab: 'results' | 'timeline' | 'logs' | 'verification' | 'variables';
  isDockCollapsed: boolean;
  dockHeight: number;
  isInspectorCollapsed: boolean;
  showMiniMap: boolean;
  validationReport: ValidationReport | null;

  setNodes: (nodes: Node<WorkflowNodeData>[]) => void;
  setEdges: (edges: Edge[]) => void;
  onNodesChange: (changes: NodeChange<Node<WorkflowNodeData>>[]) => void;
  onEdgesChange: (changes: EdgeChange[]) => void;
  onConnect: (connection: Connection) => void;
  selectNode: (id: string | null) => void;
  updateNodeConfig: (id: string, config: Record<string, any>) => void;
  addNode: (nodeType: string, category: WorkflowNodeData['category'], label: string) => void;
  autoLayout: () => void;
  setWorkflowName: (name: string) => void;
  setIsRunning: (running: boolean) => void;
  addRunEvent: (event: StepEvent) => void;
  clearRunEvents: () => void;
  setActiveDockTab: (tab: WorkflowState['activeDockTab']) => void;
  toggleDock: () => void;
  setDockHeight: (height: number) => void;
  toggleInspector: () => void;
  toggleMiniMap: () => void;
  setValidationReport: (report: ValidationReport | null) => void;
  loadWorkflow: (data: any) => void;
}

const initialNodes: Node<WorkflowNodeData>[] = [
  {
    id: 'n-start',
    type: 'custom',
    position: { x: 60, y: 60 },
    data: { label: 'Bắt đầu luồng', nodeType: 'flow.start', category: 'flow', config: {} },
  },
  {
    id: 'n-launch',
    type: 'custom',
    position: { x: 400, y: 60 },
    data: { label: 'Mở ứng dụng game', nodeType: 'app.launch', category: 'app', config: { packageId: 'com.example.game' } },
  },
  {
    id: 'n-capture',
    type: 'custom',
    position: { x: 740, y: 60 },
    data: { label: 'Chụp khung hình', nodeType: 'vision.capture', category: 'vision', config: { freshnessMs: 500 } },
  },
  {
    id: 'n-find',
    type: 'custom',
    position: { x: 1080, y: 60 },
    data: { label: 'Tìm ảnh nút nhiệm vụ', nodeType: 'vision.find_image', category: 'vision', config: { templateRef: 'quest_marker.png', minConfidence: 0.92 } },
  },
  {
    id: 'n-tap',
    type: 'custom',
    position: { x: 60, y: 300 },
    data: { label: 'Safe Random Tap', nodeType: 'action.random_safe_tap', category: 'action', config: { marginRatio: 0.18, avoidRecentPoints: 5 } },
  },
  {
    id: 'n-verify',
    type: 'custom',
    position: { x: 400, y: 300 },
    data: { label: 'Xác minh hội thoại', nodeType: 'verify.wait_for_image', category: 'verify', config: { templateRef: 'dialog_confirm.png', minConfidence: 0.90 } },
  },
  {
    id: 'n-checkpoint',
    type: 'custom',
    position: { x: 740, y: 300 },
    data: { label: 'Lưu Checkpoint', nodeType: 'state.checkpoint', category: 'state', config: { name: 'quest_active' } },
  },
  {
    id: 'n-end',
    type: 'custom',
    position: { x: 1080, y: 300 },
    data: { label: 'Kết thúc hoàn tất', nodeType: 'flow.end', category: 'flow', config: {} },
  },
];

const initialEdges: Edge[] = [
  { id: 'e1', source: 'n-start', target: 'n-launch', animated: false },
  { id: 'e2', source: 'n-launch', target: 'n-capture', animated: false },
  { id: 'e3', source: 'n-capture', target: 'n-find', animated: false },
  { id: 'e4', source: 'n-find', target: 'n-tap', animated: false },
  { id: 'e5', source: 'n-tap', target: 'n-verify', animated: false },
  { id: 'e6', source: 'n-verify', target: 'n-checkpoint', animated: false },
  { id: 'e7', source: 'n-checkpoint', target: 'n-end', animated: false },
];

export const useWorkflowStore = create<WorkflowState>((set, get) => ({
  workflowId: 'wf-beginner-quest-01',
  workflowName: 'Nhiệm vụ tân thủ — AutoFarm',
  isDirty: false,
  nodes: initialNodes,
  edges: initialEdges,
  selectedNodeId: 'n-tap',
  isRunning: false,
  runEvents: [],
  activeDockTab: 'timeline',
  isDockCollapsed: false,
  dockHeight: 220,
  isInspectorCollapsed: false,
  showMiniMap: true,
  validationReport: null,

  setNodes: (nodes) => set({ nodes, isDirty: true }),
  setEdges: (edges) => set({ edges, isDirty: true }),

  onNodesChange: (changes) =>
    set({
      nodes: applyNodeChanges(changes, get().nodes),
      isDirty: true,
    }),

  onEdgesChange: (changes) =>
    set({
      edges: applyEdgeChanges(changes, get().edges),
      isDirty: true,
    }),

  onConnect: (connection) =>
    set({
      edges: addEdge(connection, get().edges),
      isDirty: true,
    }),

  selectNode: (id) => set({ selectedNodeId: id }),

  updateNodeConfig: (id, config) =>
    set({
      nodes: get().nodes.map((node) =>
        node.id === id ? { ...node, data: { ...node.data, config: { ...node.data.config, ...config } } } : node
      ),
      isDirty: true,
    }),

  addNode: (nodeType, category, label) => {
    const id = `n-${Date.now()}`;
    const newNode: Node<WorkflowNodeData> = {
      id,
      type: 'custom',
      position: { x: 300, y: 250 },
      data: { label, nodeType, category, config: {} },
    };
    set({
      nodes: [...get().nodes, newNode],
      selectedNodeId: id,
      isDirty: true,
    });
  },

  autoLayout: () => {
    const nodes = get().nodes;
    const perRow = 4;
    const nodeWidth = 340;
    const nodeHeight = 240;
    const startX = 60;
    const startY = 60;

    const newNodes = nodes.map((node, index) => {
      const row = Math.floor(index / perRow);
      const col = index % perRow;

      return {
        ...node,
        position: {
          x: startX + col * nodeWidth,
          y: startY + row * nodeHeight,
        },
      };
    });

    set({ nodes: newNodes, isDirty: true });
  },

  setWorkflowName: (name) => set({ workflowName: name, isDirty: true }),
  setIsRunning: (running) => set({ isRunning: running }),
  addRunEvent: (event) => set({ runEvents: [...get().runEvents, event] }),
  clearRunEvents: () => set({ runEvents: [] }),
  setActiveDockTab: (tab) => set({ activeDockTab: tab, isDockCollapsed: false }),
  toggleDock: () => set({ isDockCollapsed: !get().isDockCollapsed }),
  setDockHeight: (height) => set({ dockHeight: height }),
  toggleInspector: () => set({ isInspectorCollapsed: !get().isInspectorCollapsed }),
  toggleMiniMap: () => set({ showMiniMap: !get().showMiniMap }),
  setValidationReport: (report) => set({ validationReport: report }),

  loadWorkflow: (data) => {
    if (data.graph && Array.isArray(data.graph.nodes)) {
      const nodes: Node<WorkflowNodeData>[] = data.graph.nodes.map((n: any) => ({
        id: n.id,
        type: 'custom',
        position: n.position || { x: 100, y: 100 },
        data: {
          label: n.type,
          nodeType: n.type,
          category: (n.type.split('.')[0] || 'flow') as WorkflowNodeData['category'],
          config: n.config || {},
          timeoutMs: n.timeoutMs,
        },
      }));
      const edges: Edge[] = (data.graph.edges || []).map((e: any) => ({
        id: e.id,
        source: e.source,
        target: e.target,
      }));
      set({
        workflowId: data.workflowId || 'imported',
        workflowName: data.name || 'Imported Workflow',
        nodes,
        edges,
        isDirty: false,
      });
    }
  },
}));
