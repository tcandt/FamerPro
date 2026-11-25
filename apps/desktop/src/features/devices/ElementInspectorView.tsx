import React, { useEffect, useRef, useState } from 'react';
import {
  AlertTriangle,
  ArrowRight,
  Bookmark,
  Check,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Clock,
  Code,
  Copy,
  Crosshair,
  Database,
  Eye,
  FolderPlus,
  Layers,
  Maximize2,
  Minimize2,
  MousePointer,
  Play,
  RefreshCw,
  Search,
  Sparkles,
  Target,
  Trash2,
  X,
  Zap,
} from 'lucide-react';
import { api } from '../../lib/api';
import {
  ActionExecutionMode,
  DetectionResult,
  ElementActionKind,
  ElementActionResult,
  ElementNode,
  GeneratedSelector,
  InspectSnapshotResult,
  LibraryElementRecord,
  NativeElementSelector,
  SelectorMatch,
  VirtualSelectorMode,
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

  // Virtual Selector Mode & Test Detection States
  const [selectorMode, setSelectorMode] = useState<VirtualSelectorMode>('native');
  const [ocrText, setOcrText] = useState('');
  const [imageTemplateId, setImageTemplateId] = useState('');
  const [pointCoords, setPointCoords] = useState<{ x: number; y: number }>({ x: 0.5, y: 0.5 });
  const [testingDetection, setTestingDetection] = useState(false);
  const [testDetectionResult, setTestDetectionResult] = useState<DetectionResult | null>(null);

  // Action Builder States
  const [actionKind, setActionKind] = useState<'Tap' | 'DoubleTap' | 'LongPress' | 'SetText'>('Tap');
  const [actionText, setActionText] = useState('Hello');
  const [execMode, setExecMode] = useState<ActionExecutionMode>('ClickableParent');
  const [executingAction, setExecutingAction] = useState(false);
  const [lastActionResult, setLastActionResult] = useState<ElementActionResult | null>(null);
  const [collapsedNodes, setCollapsedNodes] = useState<Set<number>>(new Set());

  // Element Library States
  const [showLibraryModal, setShowLibraryModal] = useState(false);
  const [libraryElements, setLibraryElements] = useState<LibraryElementRecord[]>([]);
  const [showLibraryDrawer, setShowLibraryDrawer] = useState(false);
  const [saveElementName, setSaveElementName] = useState('');
  const [saveProjectId, setSaveProjectId] = useState('default_project');
  const [savingLibrary, setSavingLibrary] = useState(false);

  const previewContainerRef = useRef<HTMLDivElement>(null);

  // Fetch initial snapshot on mount or serial change
  useEffect(() => {
    if (serial) {
      handleRefreshSnapshot();
      loadLibraryElements();
    }
  }, [serial]);

  const loadLibraryElements = async () => {
    try {
      const records = await api.elementListLibrary(saveProjectId);
      setLibraryElements(records);
    } catch (e) {
      console.error('Failed to list library elements:', e);
    }
  };

  const handleRefreshSnapshot = async () => {
    if (!serial) return;
    setLoading(true);
    try {
      const res = await api.elementInspectSnapshot(serial);
      setSnapshotResult(res);
      setSelectedNode(null);
      setHoveredNode(null);
      setGeneratedSelectors([]);
      setTestDetectionResult(null);
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

  // Test Detection across all 7 selector modes
  const handleTestDetection = async () => {
    setTestingDetection(true);
    setTestDetectionResult(null);
    try {
      let selectorPayload: any = null;

      if (selectorMode === 'native') {
        if (!selectedNode) {
          onNotice('Please select an element from hierarchy or preview first', 'error');
          setTestingDetection(false);
          return;
        }
        selectorPayload = {
          Native: {
            resource_id: selectedNode.resource_id || null,
            text: selectedNode.text || null,
            class_name: selectedNode.class_name || null,
            content_desc: selectedNode.content_desc || null,
            clickable: selectedNode.clickable,
          },
        };
      } else if (selectorMode === 'ocr') {
        if (!ocrText.trim()) {
          onNotice('Please enter text for OCR detection', 'error');
          setTestingDetection(false);
          return;
        }
        selectorPayload = {
          Ocr: {
            text: ocrText.trim(),
            expected_count: 1,
            min_confidence: 0.8,
          },
        };
      } else if (selectorMode === 'image') {
        selectorPayload = {
          Image: {
            template_name: imageTemplateId || 'template_default',
            threshold: 0.85,
          },
        };
      } else if (selectorMode === 'point') {
        selectorPayload = {
          Point: {
            x: pointCoords.x,
            y: pointCoords.y,
            normalized: true,
          },
        };
      } else if (selectorMode === 'region') {
        selectorPayload = {
          Region: {
            name: 'selected_region',
            bounds: selectedNode?.bounds || { x: 0, y: 0, width: 300, height: 300 },
          },
        };
      } else if (selectorMode === 'hybrid') {
        selectorPayload = {
          Hybrid: {
            primary: {
              Native: {
                resource_id: selectedNode?.resource_id || null,
                text: selectedNode?.text || null,
              },
            },
            fallbacks: [
              {
                Point: {
                  x: selectedNode ? selectedNode.bounds.x + selectedNode.bounds.width / 2 : 540,
                  y: selectedNode ? selectedNode.bounds.y + selectedNode.bounds.height / 2 : 960,
                  normalized: false,
                },
              },
            ],
          },
        };
      }

      const res = await api.elementTestDetection(
        snapshotResult?.snapshot || null,
        selectorPayload,
        true
      );
      setTestDetectionResult(res);
      onNotice(
        `Detection Verified: ${res.strategy_used} (Confidence: ${(res.confidence * 100).toFixed(0)}%, Matches: ${res.candidate_count})`,
        'success'
      );
    } catch (err: any) {
      onNotice(`Detection Test Failed: ${err}`, 'error');
    } finally {
      setTestingDetection(false);
    }
  };

  const handleExecuteAction = async () => {
    if (!snapshotResult) return;
    setExecutingAction(true);
    try {
      let kind: ElementActionKind = 'Tap';
      if (actionKind === 'DoubleTap') kind = 'DoubleTap';
      else if (actionKind === 'LongPress') kind = { LongPress: { duration_ms: 1000 } };
      else if (actionKind === 'SetText') kind = { SetText: { text: actionText, clear_first: true } };

      let matchItem: SelectorMatch;

      if (testDetectionResult) {
        matchItem = {
          node_id: testDetectionResult.matches[0]?.node_id ?? null,
          bounds: testDetectionResult.target_bounds,
          confidence: testDetectionResult.confidence,
          strategy_used: testDetectionResult.strategy_used,
          is_clickable: testDetectionResult.is_clickable,
          resolved_clickable_ancestor_id: testDetectionResult.resolved_clickable_ancestor_id ?? null,
          resolved_bounds: testDetectionResult.resolved_bounds,
        };
      } else if (selectedNode) {
        matchItem = {
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
      } else {
        onNotice('Select an element or run Test Detection before executing action', 'error');
        setExecutingAction(false);
        return;
      }

      const result = await api.elementExecuteAction(serial, matchItem, kind, execMode);
      setLastActionResult(result);
      if (result.new_screenshot_base64) {
        setSnapshotResult((prev) =>
          prev ? { ...prev, screenshot_base64: result.new_screenshot_base64! } : null
        );
      }
      onNotice(
        `Action executed in ${result.execution_ms}ms at (${Math.round(result.plan.safe_point.x)}, ${Math.round(result.plan.safe_point.y)}) [Mode: ${result.plan.mode}]`,
        'success'
      );
    } catch (err: any) {
      onNotice(`Action execution failed: ${err}`, 'error');
    } finally {
      setExecutingAction(false);
    }
  };

  const handleSaveToLibrary = async () => {
    if (!saveElementName.trim()) {
      onNotice('Please provide an element name', 'error');
      return;
    }
    setSavingLibrary(true);
    try {
      const record: LibraryElementRecord = {
        id: `elem_${Date.now()}_${Math.random().toString(36).substring(2, 7)}`,
        projectId: saveProjectId,
        name: saveElementName.trim(),
        packageFilter: snapshotResult?.package_name || null,
        primaryStrategy: selectorMode,
        selectorJson: JSON.stringify(
          selectedNode
            ? {
                resource_id: selectedNode.resource_id || null,
                text: selectedNode.text || null,
                class_name: selectedNode.class_name || null,
              }
            : { ocr: ocrText, point: pointCoords }
        ),
        fallbackSelectorsJson: JSON.stringify([]),
        boundsJson: selectedNode ? JSON.stringify(selectedNode.bounds) : null,
        version: 1,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      await api.elementSaveToLibrary(record);
      onNotice(`Element "${saveElementName}" saved to Library!`, 'success');
      setShowLibraryModal(false);
      setSaveElementName('');
      await loadLibraryElements();
    } catch (err: any) {
      onNotice(`Failed to save to library: ${err}`, 'error');
    } finally {
      setSavingLibrary(false);
    }
  };

  const handleDeleteLibraryItem = async (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      await api.elementDeleteFromLibrary(id);
      onNotice('Element removed from library', 'info');
      await loadLibraryElements();
    } catch (err: any) {
      onNotice(`Delete failed: ${err}`, 'error');
    }
  };

  // Canvas click handler
  const handlePreviewClick = async (e: React.MouseEvent<HTMLDivElement>) => {
    if (!snapshotResult) return;
    const rect = e.currentTarget.getBoundingClientRect();
    const clickX = e.clientX - rect.left;
    const clickY = e.clientY - rect.top;

    const scaleX = snapshotResult.snapshot.screen_width / rect.width;
    const scaleY = snapshotResult.snapshot.screen_height / rect.height;
    const devX = clickX * scaleX;
    const devY = clickY * scaleY;

    if (inspectMode === 'control') {
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
      const node = await api.elementFindAtPoint(snapshotResult.snapshot, devX, devY);
      if (node) {
        setSelectedNode(node);
        setTestDetectionResult(null);
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
          onClick={() => {
            setSelectedNode(node);
            setTestDetectionResult(null);
          }}
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
        isFullscreen ? 'fixed inset-4 z-50 rounded-2xl border-[#3B82F6]' : 'h-[760px]'
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
          {/* Virtual Selector Mode Tabs */}
          <div className="flex items-center bg-[#131D2E] p-0.5 rounded-lg border border-[#1E293B]">
            {(['native', 'ocr', 'image', 'point', 'hybrid'] as VirtualSelectorMode[]).map((mode) => (
              <button
                key={mode}
                onClick={() => {
                  setSelectorMode(mode);
                  setTestDetectionResult(null);
                }}
                className={`px-2.5 py-1 rounded text-xs font-medium uppercase tracking-wider transition-all ${
                  selectorMode === mode
                    ? 'bg-[#1878F5] text-white shadow'
                    : 'text-[#64748B] hover:text-[#94A3B8]'
                }`}
              >
                {mode}
              </button>
            ))}
          </div>

          {/* Inspect / Control Mode */}
          <div className="flex items-center bg-[#1E293B] p-0.5 rounded-lg border border-[#334155]">
            <button
              onClick={() => setInspectMode('inspect')}
              className={`flex items-center gap-1.5 px-2.5 py-1 rounded text-xs font-medium transition-all ${
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
              className={`flex items-center gap-1.5 px-2.5 py-1 rounded text-xs font-medium transition-all ${
                inspectMode === 'control'
                  ? 'bg-amber-600 text-white shadow'
                  : 'text-[#94A3B8] hover:text-white'
              }`}
            >
              <MousePointer size={13} />
              Control
            </button>
          </div>

          {/* Library Drawer Toggle */}
          <button
            onClick={() => setShowLibraryDrawer(!showLibraryDrawer)}
            className={`flex items-center gap-1.5 px-2.5 py-1.5 text-xs font-medium rounded-lg border transition-all ${
              showLibraryDrawer
                ? 'bg-purple-900/40 text-purple-300 border-purple-500'
                : 'bg-[#1E293B] hover:bg-[#334155] text-[#E2E8F0] border-[#334155]'
            }`}
          >
            <Database size={13} />
            Library ({libraryElements.length})
          </button>

          {/* Refresh Snapshot */}
          <button
            onClick={handleRefreshSnapshot}
            disabled={loading}
            className="flex items-center gap-1.5 px-3 py-1.5 bg-[#1E293B] hover:bg-[#334155] text-xs text-[#E2E8F0] font-medium rounded-lg border border-[#334155] transition-all disabled:opacity-50"
          >
            <RefreshCw size={13} className={loading ? 'animate-spin text-blue-400' : ''} />
            {loading ? 'Dumping...' : 'Refresh'}
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
      <div className="flex-1 grid grid-cols-12 overflow-hidden relative">
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

                {/* Test Detection Verified Bounding Box Overlay */}
                {testDetectionResult && (
                  <div
                    className="absolute border-2 border-dashed border-amber-400 bg-amber-400/25 pointer-events-none transition-all z-30 animate-pulse"
                    style={{
                      left: `${(testDetectionResult.target_bounds.x / snapshotResult.snapshot.screen_width) * 100}%`,
                      top: `${(testDetectionResult.target_bounds.y / snapshotResult.snapshot.screen_height) * 100}%`,
                      width: `${(testDetectionResult.target_bounds.width / snapshotResult.snapshot.screen_width) * 100}%`,
                      height: `${(testDetectionResult.target_bounds.height / snapshotResult.snapshot.screen_height) * 100}%`,
                    }}
                  >
                    <span className="absolute -bottom-5 left-0 px-1.5 py-0.5 bg-amber-600 text-white text-[9px] font-mono rounded shadow flex items-center gap-1">
                      <Target size={10} />
                      Detected: {(testDetectionResult.confidence * 100).toFixed(0)}% (
                      {testDetectionResult.strategy_used})
                    </span>
                  </div>
                )}

                {/* Resolved Clickable Parent Bounds Overlay */}
                {testDetectionResult && testDetectionResult.resolved_clickable_ancestor_id && (
                  <div
                    className="absolute border border-emerald-400 bg-emerald-400/15 pointer-events-none z-25"
                    style={{
                      left: `${(testDetectionResult.resolved_bounds.x / snapshotResult.snapshot.screen_width) * 100}%`,
                      top: `${(testDetectionResult.resolved_bounds.y / snapshotResult.snapshot.screen_height) * 100}%`,
                      width: `${(testDetectionResult.resolved_bounds.width / snapshotResult.snapshot.screen_width) * 100}%`,
                      height: `${(testDetectionResult.resolved_bounds.height / snapshotResult.snapshot.screen_height) * 100}%`,
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

        {/* Panel 3: Element Properties & Selector Lab (Col 8-9) */}
        <div className="col-span-3 flex flex-col bg-[#0B111A] border-r border-[#1E293B] overflow-y-auto p-3 space-y-3">
          <div className="flex items-center justify-between pb-2 border-b border-[#1E293B]">
            <span className="font-semibold text-xs text-[#F1F5F9] flex items-center gap-1.5">
              <Code size={13} className="text-blue-400" />
              Properties & Selectors
            </span>
            {selectedNode && (
              <span className="text-[10px] font-mono text-[#64748B]">Node #{selectedNode.id}</span>
            )}
          </div>

          {/* Virtual Selector Parameter Inputs */}
          {selectorMode === 'ocr' && (
            <div className="p-2.5 bg-[#131D2E] rounded-lg border border-[#1E293B] space-y-2">
              <label className="text-[10px] font-medium text-[#93C5FD] uppercase flex items-center gap-1">
                <Search size={11} /> OCR Target Text
              </label>
              <input
                type="text"
                placeholder="e.g. Nhận thưởng, 7, Login..."
                value={ocrText}
                onChange={(e) => setOcrText(e.target.value)}
                className="w-full bg-[#0A101D] text-xs text-white px-2 py-1.5 rounded border border-[#334155] focus:outline-none focus:border-blue-500"
              />
            </div>
          )}

          {selectorMode === 'image' && (
            <div className="p-2.5 bg-[#131D2E] rounded-lg border border-[#1E293B] space-y-2">
              <label className="text-[10px] font-medium text-[#93C5FD] uppercase flex items-center gap-1">
                <Layers size={11} /> Template Image ID
              </label>
              <input
                type="text"
                placeholder="e.g. reward_icon.png, button_confirm..."
                value={imageTemplateId}
                onChange={(e) => setImageTemplateId(e.target.value)}
                className="w-full bg-[#0A101D] text-xs text-white px-2 py-1.5 rounded border border-[#334155] focus:outline-none focus:border-blue-500"
              />
            </div>
          )}

          {selectorMode === 'point' && (
            <div className="p-2.5 bg-[#131D2E] rounded-lg border border-[#1E293B] space-y-2">
              <label className="text-[10px] font-medium text-[#93C5FD] uppercase flex items-center gap-1">
                <Crosshair size={11} /> Relative Point (0.0 - 1.0)
              </label>
              <div className="grid grid-cols-2 gap-2">
                <input
                  type="number"
                  step="0.05"
                  min="0"
                  max="1"
                  value={pointCoords.x}
                  onChange={(e) => setPointCoords({ ...pointCoords, x: parseFloat(e.target.value) || 0 })}
                  className="bg-[#0A101D] text-xs text-white px-2 py-1.5 rounded border border-[#334155]"
                />
                <input
                  type="number"
                  step="0.05"
                  min="0"
                  max="1"
                  value={pointCoords.y}
                  onChange={(e) => setPointCoords({ ...pointCoords, y: parseFloat(e.target.value) || 0 })}
                  className="bg-[#0A101D] text-xs text-white px-2 py-1.5 rounded border border-[#334155]"
                />
              </div>
            </div>
          )}

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

                <div className="space-y-1.5 max-h-48 overflow-y-auto">
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

        {/* Panel 4: Action Builder & Execution Engine (Col 10-12) */}
        <div className="col-span-2 flex flex-col bg-[#0E1626] p-3 space-y-3 overflow-y-auto">
          <div className="flex items-center justify-between pb-2 border-b border-[#1E293B]">
            <span className="font-semibold text-xs text-[#F1F5F9] flex items-center gap-1.5">
              <Zap size={13} className="text-amber-400" />
              Smart Action Engine
            </span>
          </div>

          <div className="space-y-3 text-xs">
            {/* Test Detection Button (Separate from Action Execution!) */}
            <button
              onClick={handleTestDetection}
              disabled={testingDetection}
              className="w-full py-2 bg-gradient-to-r from-amber-600 to-amber-700 hover:from-amber-500 hover:to-amber-600 active:scale-[0.98] text-white font-medium text-xs rounded-lg shadow flex items-center justify-center gap-1.5 transition-all disabled:opacity-50"
            >
              <Target size={13} className={testingDetection ? 'animate-spin' : ''} />
              {testingDetection ? 'Detecting Target...' : 'Test Detection (Verify Bounds)'}
            </button>

            {/* Save to Element Library */}
            <button
              onClick={() => {
                if (selectedNode) {
                  setSaveElementName(getNodeLabel(selectedNode));
                }
                setShowLibraryModal(true);
              }}
              className="w-full py-1.5 bg-[#1E293B] hover:bg-[#334155] text-purple-300 text-xs rounded-lg border border-purple-500/40 flex items-center justify-center gap-1.5 transition-all"
            >
              <Bookmark size={13} />
              Save to Library
            </button>

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

            {/* Safe Margin Policy Indicator */}
            <div className="p-2 bg-[#131D2E] rounded border border-[#1E293B] text-[10px] text-[#94A3B8]">
              <span>Safe Margin Policy: </span>
              <span className="font-mono text-emerald-400">15% inner padding</span>
            </div>

            {/* Test Action on Phone Button */}
            <button
              onClick={handleExecuteAction}
              disabled={executingAction}
              className="w-full py-2 bg-[#1878F5] hover:bg-blue-600 active:bg-blue-700 text-white font-medium text-xs rounded-lg shadow flex items-center justify-center gap-1.5 transition-all disabled:opacity-50"
            >
              <Play size={13} className={executingAction ? 'animate-spin' : ''} />
              {executingAction ? 'Executing on LAN...' : 'Test Action on Phone'}
            </button>

            {/* Add to Workflow Button */}
            {onAddToWorkflow && generatedSelectors.length > 0 && selectedNode && (
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
        </div>

        {/* 3. Element Library Drawer Overlay */}
        {showLibraryDrawer && (
          <div className="absolute right-0 top-0 bottom-0 w-80 bg-[#0F172A] border-l border-[#1E293B] shadow-2xl z-40 flex flex-col">
            <div className="flex items-center justify-between p-3 border-b border-[#1E293B]">
              <span className="font-semibold text-xs text-[#F1F5F9] flex items-center gap-1.5">
                <Database size={13} className="text-purple-400" />
                Element Library ({libraryElements.length})
              </span>
              <button
                onClick={() => setShowLibraryDrawer(false)}
                className="p-1 text-[#64748B] hover:text-white"
              >
                <X size={14} />
              </button>
            </div>

            <div className="flex-1 overflow-y-auto p-2 space-y-2">
              {libraryElements.length === 0 ? (
                <div className="text-center p-6 text-[#64748B] text-xs">
                  No elements saved yet in this project.
                </div>
              ) : (
                libraryElements.map((el) => (
                  <div
                    key={el.id}
                    onClick={() => {
                      onNotice(`Loaded element: ${el.name}`, 'info');
                    }}
                    className="p-2.5 bg-[#131D2E] hover:bg-[#1E293B] rounded-lg border border-[#1E293B] cursor-pointer group flex flex-col gap-1 transition-all"
                  >
                    <div className="flex items-center justify-between">
                      <span className="font-medium text-xs text-[#E2E8F0]">{el.name}</span>
                      <button
                        onClick={(e) => handleDeleteLibraryItem(el.id, e)}
                        className="opacity-0 group-hover:opacity-100 p-1 text-red-400 hover:text-red-300 transition-opacity"
                        title="Delete from library"
                      >
                        <Trash2 size={12} />
                      </button>
                    </div>
                    <div className="flex items-center gap-2 text-[10px] text-[#94A3B8]">
                      <span className="px-1.5 py-0.2 bg-purple-950/60 text-purple-300 rounded font-mono uppercase">
                        {el.primaryStrategy}
                      </span>
                      <span>v{el.version}</span>
                    </div>
                  </div>
                ))
              )}
            </div>
          </div>
        )}
      </div>

      {/* 4. Save to Library Modal */}
      {showLibraryModal && (
        <div className="fixed inset-0 bg-black/60 backdrop-blur-sm z-50 flex items-center justify-center p-4">
          <div className="bg-[#0F172A] border border-[#1E293B] rounded-xl p-4 w-96 shadow-2xl flex flex-col gap-3">
            <div className="flex items-center justify-between pb-2 border-b border-[#1E293B]">
              <span className="font-semibold text-sm text-[#F1F5F9] flex items-center gap-1.5">
                <Bookmark size={15} className="text-purple-400" />
                Save Element to Library
              </span>
              <button
                onClick={() => setShowLibraryModal(false)}
                className="p-1 text-[#64748B] hover:text-white"
              >
                <X size={15} />
              </button>
            </div>

            <div className="space-y-1 text-xs">
              <label className="text-[#94A3B8]">Element Name</label>
              <input
                type="text"
                value={saveElementName}
                onChange={(e) => setSaveElementName(e.target.value)}
                placeholder="e.g. btn_confirm_reward"
                className="w-full bg-[#1E293B] text-xs text-white px-2.5 py-1.5 rounded-lg border border-[#334155] focus:outline-none focus:border-purple-500"
              />
            </div>

            <div className="space-y-1 text-xs">
              <label className="text-[#94A3B8]">Project Scope</label>
              <input
                type="text"
                value={saveProjectId}
                onChange={(e) => setSaveProjectId(e.target.value)}
                className="w-full bg-[#1E293B] text-xs text-white px-2.5 py-1.5 rounded-lg border border-[#334155] focus:outline-none focus:border-purple-500"
              />
            </div>

            <div className="flex justify-end gap-2 pt-2 border-t border-[#1E293B]">
              <button
                onClick={() => setShowLibraryModal(false)}
                className="px-3 py-1.5 bg-[#1E293B] hover:bg-[#334155] text-xs text-[#CBD5E1] rounded-lg"
              >
                Cancel
              </button>
              <button
                onClick={handleSaveToLibrary}
                disabled={savingLibrary}
                className="px-4 py-1.5 bg-purple-600 hover:bg-purple-500 text-xs text-white font-medium rounded-lg disabled:opacity-50 flex items-center gap-1.5"
              >
                <FolderPlus size={13} />
                {savingLibrary ? 'Saving...' : 'Save Element'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
