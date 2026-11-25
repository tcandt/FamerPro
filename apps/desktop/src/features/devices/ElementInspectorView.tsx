import React, { useEffect, useRef, useState } from 'react';
import {
  AlertTriangle,
  ArrowRight,
  Check,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Clock,
  Code,
  Copy,
  Crosshair,
  Eye,
  Maximize2,
  Minimize2,
  MousePointer,
  Play,
  RefreshCw,
  Search,
  Sparkles,
  Zap,
} from 'lucide-react';
import { api } from '../../lib/api';
import {
  ActionExecutionMode,
  ElementActionKind,
  ElementActionResult,
  ElementNode,
  GeneratedSelector,
  InspectSnapshotResult,
  NativeElementSelector,
  SelectorMatch,
} from '../../types';

interface ElementInspectorViewProps {
  serial: string;
  onNotice: (text: string, type?: 'success' | 'error' | 'info') => void;
  onAddToWorkflow?: (selector: NativeElementSelector, name: string) => void;
}

const getShortClass = (className: string): string => {
  return className.split('.').pop() || className;
};

const getNodeLabel = (node: ElementNode): string => {
  if (node.text) {
    return `"${node.text}" (${getShortClass(node.class_name)})`;
  }
  if (node.content_desc) {
    return `[${node.content_desc}] (${getShortClass(node.class_name)})`;
  }
  if (node.resource_id) {
    const shortId = node.resource_id.split('/').pop() || node.resource_id;
    return `${shortId} (${getShortClass(node.class_name)})`;
  }
  return getShortClass(node.class_name);
};

export const ElementInspectorView: React.FC<ElementInspectorViewProps> = ({
  serial,
  onNotice,
  onAddToWorkflow,
}) => {
  const [loading, setLoading] = useState(false);
  const [snapshotResult, setSnapshotResult] = useState<InspectSnapshotResult | null>(null);
  const [selectedNode, setSelectedNode] = useState<ElementNode | null>(null);
  const [hoveredNode, setHoveredNode] = useState<ElementNode | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [inspectMode, setInspectMode] = useState<'inspect' | 'control'>('inspect');
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [generatedSelectors, setGeneratedSelectors] = useState<GeneratedSelector[]>([]);
  const [copiedIndex, setCopiedIndex] = useState<number | null>(null);
  const [actionKind, setActionKind] = useState<'Tap' | 'DoubleTap' | 'LongPress' | 'SetText'>('Tap');
  const [actionText, setActionText] = useState('Hello');
  const [execMode, setExecMode] = useState<ActionExecutionMode>('ClickableParent');
  const [executingAction, setExecutingAction] = useState(false);
  const [lastActionResult, setLastActionResult] = useState<ElementActionResult | null>(null);
  const [collapsedNodes, setCollapsedNodes] = useState<Set<number>>(new Set());

  const previewContainerRef = useRef<HTMLDivElement>(null);

  // Fetch initial snapshot on mount or serial change
  useEffect(() => {
    if (serial) {
      handleRefreshSnapshot();
    }
  }, [serial]);

  const handleRefreshSnapshot = async () => {
    if (!serial) return;
    setLoading(true);
    try {
      const res = await api.elementInspectSnapshot(serial);
      setSnapshotResult(res);
      setSelectedNode(null);
      setHoveredNode(null);
      setGeneratedSelectors([]);
      onNotice(
        `Snapshot updated in ${res.total_duration_ms}ms (Dump: ${res.dump_duration_ms}ms, Parse: ${res.parse_duration_ms}ms)`,
        'success'
      );
    } catch (err: any) {
      onNotice(`Failed to capture element snapshot: ${err}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  // Generate selectors when selected node changes
  useEffect(() => {
    if (selectedNode && snapshotResult?.snapshot) {
      try {
        api.elementGenerateSelectors(snapshotResult.snapshot, selectedNode.id).then((selectors) => {
          setGeneratedSelectors(selectors);
        });
      } catch (e) {
        console.error('Error generating selectors', e);
      }
    } else {
      setGeneratedSelectors([]);
    }
  }, [selectedNode, snapshotResult]);

  const toggleCollapse = (nodeId: number) => {
    setCollapsedNodes((prev) => {
      const next = new Set(prev);
      if (next.has(nodeId)) {
        next.delete(nodeId);
      } else {
        next.add(nodeId);
      }
      return next;
    });
  };

  const handleCopy = (text: string, index: number) => {
    navigator.clipboard.writeText(text);
    setCopiedIndex(index);
    setTimeout(() => setCopiedIndex(null), 2000);
    onNotice('Selector copied to clipboard', 'info');
  };

  const handleExecuteAction = async () => {
    if (!selectedNode || !snapshotResult) return;
    setExecutingAction(true);
    try {
      let kind: ElementActionKind = 'Tap';
      if (actionKind === 'DoubleTap') kind = 'DoubleTap';
      else if (actionKind === 'LongPress') kind = { LongPress: { duration_ms: 1000 } };
      else if (actionKind === 'SetText') kind = { SetText: { text: actionText, clear_first: true } };

      const matchItem: SelectorMatch = {
        node_id: selectedNode.id,
        bounds: selectedNode.bounds,
        confidence: 1.0,
        strategy_used: 'inspector_manual',
        is_clickable: selectedNode.clickable,
        resolved_clickable_ancestor_id: selectedNode.parent_id,
        resolved_bounds: selectedNode.clickable
          ? selectedNode.bounds
          : selectedNode.parent_id !== null && snapshotResult.snapshot.nodes[selectedNode.parent_id]
          ? snapshotResult.snapshot.nodes[selectedNode.parent_id].bounds
          : selectedNode.bounds,
      };

      const result = await api.elementExecuteAction(serial, matchItem, kind, execMode);
      setLastActionResult(result);
      if (result.new_screenshot_base64) {
        setSnapshotResult((prev) =>
          prev ? { ...prev, screenshot_base64: result.new_screenshot_base64! } : null
        );
      }
      onNotice(
        `Action executed in ${result.execution_ms}ms at (${Math.round(result.plan.safe_point.x)}, ${Math.round(result.plan.safe_point.y)})`,
        'success'
      );
    } catch (err: any) {
      onNotice(`Action execution failed: ${err}`, 'error');
    } finally {
      setExecutingAction(false);
    }
  };

  // Canvas click handler
  const handlePreviewClick = async (e: React.MouseEvent<HTMLDivElement>) => {
    if (!snapshotResult) return;
    const rect = e.currentTarget.getBoundingClientRect();
    const clickX = e.clientX - rect.left;
    const clickY = e.clientY - rect.top;

    // Convert display coords to device coords
    const scaleX = snapshotResult.snapshot.screen_width / rect.width;
    const scaleY = snapshotResult.snapshot.screen_height / rect.height;
    const devX = clickX * scaleX;
    const devY = clickY * scaleY;

    if (inspectMode === 'control') {
      // Direct tap
      try {
        const dummyMatch: SelectorMatch = {
          node_id: null,
          bounds: { x: devX - 5, y: devY - 5, width: 10, height: 10 },
          confidence: 1.0,
          strategy_used: 'direct_touch',
          is_clickable: true,
          resolved_clickable_ancestor_id: null,
          resolved_bounds: { x: devX - 5, y: devY - 5, width: 10, height: 10 },
        };
        await api.elementExecuteAction(serial, dummyMatch, 'Tap', 'DynamicBoundsTap');
        handleRefreshSnapshot();
      } catch (err: any) {
        onNotice(`Direct touch failed: ${err}`, 'error');
      }
    } else {
      // Find node under point
      const node = await api.elementFindAtPoint(snapshotResult.snapshot, devX, devY);
      if (node) {
        setSelectedNode(node);
      }
    }
  };

  const handlePreviewMouseMove = (e: React.MouseEvent<HTMLDivElement>) => {
    if (!snapshotResult || inspectMode === 'control') return;
    const rect = e.currentTarget.getBoundingClientRect();
    const mouseX = e.clientX - rect.left;
    const mouseY = e.clientY - rect.top;

    const scaleX = snapshotResult.snapshot.screen_width / rect.width;
    const scaleY = snapshotResult.snapshot.screen_height / rect.height;
    const devX = mouseX * scaleX;
    const devY = mouseY * scaleY;

    // Find node in local nodes array
    const nodes = snapshotResult.snapshot.nodes;
    let found: ElementNode | null = null;
    let minArea = Infinity;

    for (const n of nodes) {
      if (
        n.is_visible &&
        devX >= n.bounds.x &&
        devX <= n.bounds.x + n.bounds.width &&
        devY >= n.bounds.y &&
        devY <= n.bounds.y + n.bounds.height
      ) {
        const area = n.bounds.width * n.bounds.height;
        if (area < minArea) {
          minArea = area;
          found = n;
        }
      }
    }
    setHoveredNode(found);
  };

  const renderTreeNode = (node: ElementNode, allNodes: ElementNode[]) => {
    const isSelected = selectedNode?.id === node.id;
    const isHovered = hoveredNode?.id === node.id;
    const isCollapsed = collapsedNodes.has(node.id);
    const hasChildren = node.children_ids.length > 0;

    // Filter check
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      const matchSelf =
        node.text.toLowerCase().includes(q) ||
        node.resource_id.toLowerCase().includes(q) ||
        node.class_name.toLowerCase().includes(q) ||
        node.content_desc.toLowerCase().includes(q);
      if (!matchSelf) {
        return null;
      }
    }

    return (
      <div key={node.id} className="flex flex-col">
        <div
          onClick={() => setSelectedNode(node)}
          onMouseEnter={() => setHoveredNode(node)}
          className={`group flex items-center gap-1.5 px-2 py-1 text-xs cursor-pointer select-none rounded transition-colors ${
            isSelected
              ? 'bg-[#1878F5] text-white font-medium'
              : isHovered
              ? 'bg-[#1E293B] text-[#93C5FD]'
              : 'text-[#94A3B8] hover:bg-[#1E293B]/60'
          }`}
          style={{ paddingLeft: `${node.depth * 14 + 6}px` }}
        >
          {hasChildren ? (
            <button
              onClick={(e) => {
                e.stopPropagation();
                toggleCollapse(node.id);
              }}
              className="p-0.5 hover:bg-black/20 rounded text-[#64748B]"
            >
              {isCollapsed ? <ChevronRight size={12} /> : <ChevronDown size={12} />}
            </button>
          ) : (
            <span className="w-3" />
          )}

          <span className="font-mono text-[11px] opacity-80">
            &lt;{getShortClass(node.class_name)}&gt;
          </span>

          {node.text && (
            <span
              className={`truncate max-w-[140px] px-1 rounded ${
                isSelected ? 'bg-blue-600 text-white' : 'bg-emerald-950/60 text-emerald-400'
              }`}
            >
              "{node.text}"
            </span>
          )}

          {node.resource_id && (
            <span
              className={`truncate max-w-[120px] font-mono text-[10px] ${
                isSelected ? 'text-blue-100' : 'text-[#64748B]'
              }`}
            >
              #{node.resource_id.split('/').pop()}
            </span>
          )}

          {node.clickable && (
            <span className="ml-auto text-[9px] px-1 py-0.2 bg-amber-500/20 text-amber-300 rounded font-mono">
              click
            </span>
          )}
        </div>

        {!isCollapsed &&
          hasChildren &&
          node.children_ids.map((childId) => {
            const childNode = allNodes[childId];
            return childNode ? renderTreeNode(childNode, allNodes) : null;
          })}
      </div>
    );
  };

  return (
    <div
      className={`flex flex-col bg-[#0B111A] border border-[#1E293B] rounded-xl overflow-hidden shadow-2xl transition-all ${
        isFullscreen ? 'fixed inset-4 z-50 rounded-2xl border-[#3B82F6]' : 'h-[740px]'
      }`}
    >
      {/* 1. Header Toolbar */}
      <div className="flex items-center justify-between px-4 py-2.5 bg-[#0F172A] border-b border-[#1E293B]">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2">
            <span className="w-2.5 h-2.5 rounded-full bg-emerald-500 animate-pulse" />
            <span className="font-bold text-sm text-[#F1F5F9]">Element Inspector</span>
            <span className="text-xs font-mono text-[#64748B]">({serial})</span>
          </div>

          {snapshotResult && (
            <div className="flex items-center gap-2 text-[11px] text-[#94A3B8] bg-[#1E293B]/70 px-2.5 py-1 rounded-md">
              <Clock size={12} className="text-blue-400" />
              <span>{snapshotResult.snapshot.nodes.length} Elements</span>
              <span>·</span>
              <span>Dump: {snapshotResult.dump_duration_ms}ms</span>
              <span>·</span>
              <span>Parse: {snapshotResult.parse_duration_ms}ms</span>
            </div>
          )}
        </div>

        <div className="flex items-center gap-2">
          {/* Mode Switcher */}
          <div className="flex items-center bg-[#1E293B] p-0.5 rounded-lg border border-[#334155]">
            <button
              onClick={() => setInspectMode('inspect')}
              className={`flex items-center gap-1.5 px-3 py-1 rounded text-xs font-medium transition-all ${
                inspectMode === 'inspect'
                  ? 'bg-[#1878F5] text-white shadow'
                  : 'text-[#94A3B8] hover:text-white'
              }`}
            >
              <Crosshair size={13} />
              Inspect
            </button>
            <button
              onClick={() => setInspectMode('control')}
              className={`flex items-center gap-1.5 px-3 py-1 rounded text-xs font-medium transition-all ${
                inspectMode === 'control'
                  ? 'bg-amber-600 text-white shadow'
                  : 'text-[#94A3B8] hover:text-white'
              }`}
            >
              <MousePointer size={13} />
              Control
            </button>
          </div>

          {/* Refresh Snapshot */}
          <button
            onClick={handleRefreshSnapshot}
            disabled={loading}
            className="flex items-center gap-1.5 px-3 py-1.5 bg-[#1E293B] hover:bg-[#334155] text-xs text-[#E2E8F0] font-medium rounded-lg border border-[#334155] transition-all disabled:opacity-50"
          >
            <RefreshCw size={13} className={loading ? 'animate-spin text-blue-400' : ''} />
            {loading ? 'Dumping...' : 'Refresh Snapshot'}
          </button>

          {/* Fullscreen Toggle */}
          <button
            onClick={() => setIsFullscreen(!isFullscreen)}
            className="p-1.5 text-[#94A3B8] hover:text-white hover:bg-[#1E293B] rounded-lg transition-colors border border-[#334155]"
            title={isFullscreen ? 'Exit Fullscreen' : 'Fullscreen Inspector'}
          >
            {isFullscreen ? <Minimize2 size={15} /> : <Maximize2 size={15} />}
          </button>
        </div>
      </div>

      {/* 2. Main 4-Panel Body */}
      <div className="flex-1 grid grid-cols-12 overflow-hidden">
        {/* Panel 1: Device Preview Canvas (Col 1-4) */}
        <div className="col-span-4 flex flex-col bg-[#050911] border-r border-[#1E293B] relative overflow-hidden">
          <div className="flex items-center justify-between px-3 py-1.5 bg-[#0D1524] border-b border-[#1E293B] text-[11px] text-[#94A3B8]">
            <span className="flex items-center gap-1.5">
              <Eye size={12} className="text-blue-400" />
              Device Preview{' '}
              {snapshotResult?.snapshot &&
                `(${snapshotResult.snapshot.screen_width}x${snapshotResult.snapshot.screen_height})`}
            </span>
            <span className="font-mono text-[10px] text-[#64748B]">
              {hoveredNode
                ? `[${Math.round(hoveredNode.bounds.x)}, ${Math.round(hoveredNode.bounds.y)}]`
                : 'Hover to inspect'}
            </span>
          </div>

          <div
            ref={previewContainerRef}
            className="flex-1 flex items-center justify-center p-3 overflow-auto relative select-none"
          >
            {snapshotResult ? (
              <div
                className="relative cursor-crosshair border border-[#1E293B] shadow-2xl rounded"
                style={{
                  width: `${300}px`,
                  height: `${(300 * snapshotResult.snapshot.screen_height) / snapshotResult.snapshot.screen_width}px`,
                }}
                onClick={handlePreviewClick}
                onMouseMove={handlePreviewMouseMove}
                onMouseLeave={() => setHoveredNode(null)}
              >
                <img
                  src={snapshotResult.screenshot_base64}
                  alt="Device Screen"
                  className="w-full h-full object-contain pointer-events-none rounded"
                />

                {/* Selected Node Bounding Box Overlay */}
                {selectedNode && (
                  <div
                    className="absolute border-2 border-blue-500 bg-blue-500/20 pointer-events-none transition-all z-20"
                    style={{
                      left: `${(selectedNode.bounds.x / snapshotResult.snapshot.screen_width) * 100}%`,
                      top: `${(selectedNode.bounds.y / snapshotResult.snapshot.screen_height) * 100}%`,
                      width: `${(selectedNode.bounds.width / snapshotResult.snapshot.screen_width) * 100}%`,
                      height: `${(selectedNode.bounds.height / snapshotResult.snapshot.screen_height) * 100}%`,
                    }}
                  >
                    <span className="absolute -top-5 left-0 px-1 py-0.2 bg-blue-600 text-white text-[9px] font-mono rounded whitespace-nowrap shadow">
                      {getNodeLabel(selectedNode)}
                    </span>
                  </div>
                )}

                {/* Hovered Node Bounding Box Overlay */}
                {hoveredNode && hoveredNode.id !== selectedNode?.id && (
                  <div
                    className="absolute border border-cyan-400 bg-cyan-400/10 pointer-events-none z-10"
                    style={{
                      left: `${(hoveredNode.bounds.x / snapshotResult.snapshot.screen_width) * 100}%`,
                      top: `${(hoveredNode.bounds.y / snapshotResult.snapshot.screen_height) * 100}%`,
                      width: `${(hoveredNode.bounds.width / snapshotResult.snapshot.screen_width) * 100}%`,
                      height: `${(hoveredNode.bounds.height / snapshotResult.snapshot.screen_height) * 100}%`,
                    }}
                  />
                )}
              </div>
            ) : (
              <div className="flex flex-col items-center justify-center text-center p-6 text-[#64748B]">
                <RefreshCw size={24} className="mb-2 animate-spin text-blue-500" />
                <p className="text-xs">Connecting to device and extracting UI Hierarchy...</p>
              </div>
            )}
          </div>
        </div>

        {/* Panel 2: UI Hierarchy Tree (Col 5-7) */}
        <div className="col-span-3 flex flex-col bg-[#090E17] border-r border-[#1E293B] overflow-hidden">
          <div className="p-2 border-b border-[#1E293B] bg-[#0F172A]">
            <div className="relative">
              <Search size={13} className="absolute left-2.5 top-2.5 text-[#64748B]" />
              <input
                type="text"
                placeholder="Filter element text, ID, class..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="w-full bg-[#1E293B] text-xs text-[#F1F5F9] pl-8 pr-3 py-1.5 rounded-lg border border-[#334155] focus:outline-none focus:border-blue-500"
              />
            </div>
          </div>

          <div className="flex-1 overflow-y-auto p-1.5 space-y-0.5 font-mono">
            {snapshotResult?.snapshot.root_ids.map((rootId) => {
              const rootNode = snapshotResult.snapshot.nodes[rootId];
              return rootNode ? renderTreeNode(rootNode, snapshotResult.snapshot.nodes) : null;
            })}
          </div>
        </div>

        {/* Panel 3: Element Properties (Col 8-9) */}
        <div className="col-span-3 flex flex-col bg-[#0B111A] border-r border-[#1E293B] overflow-y-auto p-3 space-y-3">
          <div className="flex items-center justify-between pb-2 border-b border-[#1E293B]">
            <span className="font-semibold text-xs text-[#F1F5F9] flex items-center gap-1.5">
              <Code size={13} className="text-blue-400" />
              Element Properties
            </span>
            {selectedNode && (
              <span className="text-[10px] font-mono text-[#64748B]">ID #{selectedNode.id}</span>
            )}
          </div>

          {selectedNode ? (
            <div className="space-y-3 text-xs">
              {/* Clickable Ancestor Alert */}
              {!selectedNode.clickable && selectedNode.parent_id !== null && (
                <div className="p-2.5 bg-amber-950/30 border border-amber-800/50 rounded-lg flex flex-col gap-1.5">
                  <div className="flex items-center gap-1.5 text-amber-400 text-[11px] font-medium">
                    <AlertTriangle size={13} />
                    <span>Element is not clickable</span>
                  </div>
                  <p className="text-[10px] text-[#CBD5E1]">
                    Click will resolve to parent:{' '}
                    <span className="font-mono text-amber-200">
                      {getShortClass(
                        snapshotResult?.snapshot.nodes[selectedNode.parent_id]?.class_name || ''
                      )}
                    </span>
                  </p>
                  <button
                    onClick={() => {
                      if (snapshotResult?.snapshot.nodes[selectedNode.parent_id!]) {
                        setSelectedNode(snapshotResult.snapshot.nodes[selectedNode.parent_id!]);
                      }
                    }}
                    className="self-start text-[10px] text-amber-300 underline hover:text-amber-200"
                  >
                    Select Clickable Parent
                  </button>
                </div>
              )}

              {/* Attributes Table */}
              <div className="bg-[#111827] rounded-lg border border-[#1F2937] overflow-hidden">
                <table className="w-full text-[11px]">
                  <tbody>
                    <tr className="border-b border-[#1F2937]">
                      <td className="px-2.5 py-1.5 text-[#64748B] font-mono w-24">Class</td>
                      <td className="px-2.5 py-1.5 text-[#E2E8F0] font-mono break-all">
                        {selectedNode.class_name}
                      </td>
                    </tr>
                    <tr className="border-b border-[#1F2937]">
                      <td className="px-2.5 py-1.5 text-[#64748B] font-mono">Resource ID</td>
                      <td className="px-2.5 py-1.5 text-blue-400 font-mono break-all">
                        {selectedNode.resource_id || '<none>'}
                      </td>
                    </tr>
                    <tr className="border-b border-[#1F2937]">
                      <td className="px-2.5 py-1.5 text-[#64748B] font-mono">Text</td>
                      <td className="px-2.5 py-1.5 text-emerald-400 font-sans font-medium break-all">
                        {selectedNode.text ? `"${selectedNode.text}"` : '<none>'}
                      </td>
                    </tr>
                    <tr className="border-b border-[#1F2937]">
                      <td className="px-2.5 py-1.5 text-[#64748B] font-mono">Content Desc</td>
                      <td className="px-2.5 py-1.5 text-[#CBD5E1] break-all">
                        {selectedNode.content_desc || '<none>'}
                      </td>
                    </tr>
                    <tr className="border-b border-[#1F2937]">
                      <td className="px-2.5 py-1.5 text-[#64748B] font-mono">Bounds</td>
                      <td className="px-2.5 py-1.5 text-amber-300 font-mono">
                        [{selectedNode.bounds.x}, {selectedNode.bounds.y}] ({selectedNode.bounds.width}
                        x{selectedNode.bounds.height})
                      </td>
                    </tr>
                    <tr>
                      <td className="px-2.5 py-1.5 text-[#64748B] font-mono">Package</td>
                      <td className="px-2.5 py-1.5 text-[#94A3B8] font-mono break-all">
                        {selectedNode.package}
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>

              {/* Generated Selectors Card */}
              <div className="space-y-1.5">
                <span className="font-semibold text-[11px] text-[#CBD5E1] flex items-center gap-1">
                  <Sparkles size={12} className="text-amber-400" />
                  Generated Selectors ({generatedSelectors.length})
                </span>

                <div className="space-y-1.5 max-h-56 overflow-y-auto">
                  {generatedSelectors.map((sel, idx) => (
                    <div
                      key={idx}
                      className="p-2 bg-[#131D2E] border border-[#1E293B] rounded-lg flex flex-col gap-1"
                    >
                      <div className="flex items-center justify-between">
                        <span className="text-[10px] font-semibold text-[#93C5FD]">
                          {sel.strategy_name}
                        </span>
                        <div className="flex items-center gap-1.5">
                          {sel.is_recommended && (
                            <span className="px-1.5 py-0.2 bg-emerald-500/20 text-emerald-400 text-[9px] rounded font-semibold">
                              Recommended
                            </span>
                          )}
                          <span
                            className={`text-[9px] font-mono px-1 rounded ${
                              sel.uniqueness_score === 100
                                ? 'bg-blue-500/20 text-blue-300'
                                : 'bg-amber-500/20 text-amber-300'
                            }`}
                          >
                            {sel.uniqueness_score}% Unique ({sel.match_count})
                          </span>
                        </div>
                      </div>

                      <div className="flex items-center justify-between gap-1 bg-[#090E17] px-2 py-1 rounded font-mono text-[10px] text-[#E2E8F0] break-all">
                        <span>{sel.expression}</span>
                        <button
                          onClick={() => handleCopy(sel.expression, idx)}
                          className="p-1 text-[#64748B] hover:text-white rounded transition-colors"
                          title="Copy expression"
                        >
                          {copiedIndex === idx ? (
                            <Check size={11} className="text-emerald-400" />
                          ) : (
                            <Copy size={11} />
                          )}
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          ) : (
            <div className="flex-1 flex flex-col items-center justify-center text-center p-6 text-[#64748B]">
              <Crosshair size={28} className="mb-2 text-[#334155]" />
              <p className="text-xs">
                Click any element on the preview screen or tree to inspect properties.
              </p>
            </div>
          )}
        </div>

        {/* Panel 4: Action Builder & Library (Col 10-12) */}
        <div className="col-span-2 flex flex-col bg-[#0E1626] p-3 space-y-3 overflow-y-auto">
          <div className="flex items-center justify-between pb-2 border-b border-[#1E293B]">
            <span className="font-semibold text-xs text-[#F1F5F9] flex items-center gap-1.5">
              <Zap size={13} className="text-amber-400" />
              Action Builder
            </span>
          </div>

          {selectedNode ? (
            <div className="space-y-3 text-xs">
              {/* Action Type */}
              <div className="space-y-1">
                <label className="text-[10px] font-medium text-[#64748B] uppercase">Action Type</label>
                <select
                  value={actionKind}
                  onChange={(e) => setActionKind(e.target.value as any)}
                  className="w-full bg-[#1E293B] text-xs text-[#E2E8F0] px-2.5 py-1.5 rounded-lg border border-[#334155] focus:outline-none focus:border-blue-500"
                >
                  <option value="Tap">Safe Tap</option>
                  <option value="DoubleTap">Double Tap</option>
                  <option value="LongPress">Long Press (1s)</option>
                  <option value="SetText">Input Text</option>
                </select>
              </div>

              {actionKind === 'SetText' && (
                <div className="space-y-1">
                  <label className="text-[10px] font-medium text-[#64748B] uppercase">
                    Input Content
                  </label>
                  <input
                    type="text"
                    value={actionText}
                    onChange={(e) => setActionText(e.target.value)}
                    className="w-full bg-[#1E293B] text-xs text-[#E2E8F0] px-2.5 py-1.5 rounded-lg border border-[#334155] focus:outline-none focus:border-blue-500"
                  />
                </div>
              )}

              {/* Execution Mode */}
              <div className="space-y-1">
                <label className="text-[10px] font-medium text-[#64748B] uppercase">
                  Execution Mode
                </label>
                <select
                  value={execMode}
                  onChange={(e) => setExecMode(e.target.value as any)}
                  className="w-full bg-[#1E293B] text-xs text-[#E2E8F0] px-2.5 py-1.5 rounded-lg border border-[#334155] focus:outline-none focus:border-blue-500"
                >
                  <option value="ClickableParent">Clickable Parent (Auto)</option>
                  <option value="DynamicBoundsTap">Dynamic Bounds Tap (Rule 5)</option>
                  <option value="SemanticClick">Semantic Click (Accessibility)</option>
                </select>
              </div>

              {/* Safe Margin Tag */}
              <div className="p-2 bg-[#131D2E] rounded border border-[#1E293B] text-[10px] text-[#94A3B8]">
                <span>Safe Margin Policy: </span>
                <span className="font-mono text-emerald-400">15% inner padding</span>
              </div>

              {/* Test Action Button */}
              <button
                onClick={handleExecuteAction}
                disabled={executingAction}
                className="w-full py-2 bg-[#1878F5] hover:bg-blue-600 active:bg-blue-700 text-white font-medium text-xs rounded-lg shadow flex items-center justify-center gap-1.5 transition-all disabled:opacity-50"
              >
                <Play size={13} className={executingAction ? 'animate-spin' : ''} />
                {executingAction ? 'Executing on LAN...' : 'Test Action on Phone'}
              </button>

              {/* Add to Workflow Button */}
              {onAddToWorkflow && generatedSelectors.length > 0 && (
                <button
                  onClick={() => {
                    const best = generatedSelectors[0];
                    onAddToWorkflow(best.selector, getNodeLabel(selectedNode));
                    onNotice(`Added node for "${getNodeLabel(selectedNode)}" to Workflow`, 'success');
                  }}
                  className="w-full py-2 bg-[#1E293B] hover:bg-[#334155] text-[#E2E8F0] font-medium text-xs rounded-lg border border-[#334155] flex items-center justify-center gap-1.5 transition-all"
                >
                  <ArrowRight size={13} className="text-blue-400" />
                  Add to Workflow
                </button>
              )}

              {/* Action Receipt Log */}
              {lastActionResult && (
                <div className="p-2 bg-emerald-950/30 border border-emerald-800/40 rounded-lg text-[10px] space-y-1">
                  <div className="flex items-center gap-1 text-emerald-400 font-semibold">
                    <CheckCircle2 size={11} />
                    <span>Action Succeeded ({lastActionResult.execution_ms}ms)</span>
                  </div>
                  <div className="font-mono text-[#94A3B8]">
                    Target: ({Math.round(lastActionResult.plan.safe_point.x)},{' '}
                    {Math.round(lastActionResult.plan.safe_point.y)})
                  </div>
                </div>
              )}
            </div>
          ) : (
            <div className="text-center p-4 text-[#64748B] text-xs">
              Select an element to test actions.
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
