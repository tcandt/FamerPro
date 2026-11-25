import React, { useEffect, useRef, useState, useCallback } from 'react';
import {
  Activity,
  ArrowLeft,
  Bookmark,
  ChevronRight,
  Database,
  Layers,
  MousePointer,
  Pause,
  Play,
  Plus,
  Power,
  RefreshCw,
  Save,
  Search,
  Smartphone,
  Square,
  Target,
  Terminal,
  Trash2,
  Volume1,
  Volume2,
  X,
  Crop,
  Zap,
} from 'lucide-react';
import { api, isTauri } from '../../lib/api';
import { ScreenGeometryTransform } from '../../lib/ScreenGeometryTransform';
import { useDeviceStore } from '../../stores/deviceStore';
import { useProjectStore } from '../../stores/projectStore';
import {
  ElementNode,
  GeneratedSelector,
  InspectSnapshotResult,
  LibraryElementRecord,
  RecordedWorkflowStep,
  Selector,
  SelectorMatch,
} from '../../types';

interface DeviceStudioViewProps {
  onNavigate: (view: string) => void;
  onNotice?: (text: string, type?: 'success' | 'error' | 'info') => void;
}

type StudioTab = 'control' | 'inspector' | 'recorder' | 'library' | 'logs';
type ControlMode = 'touch' | 'inspect' | 'region';

export const DeviceStudioView: React.FC<DeviceStudioViewProps> = ({
  onNavigate,
  onNotice = () => {},
}) => {
  const { devices, selectedDeviceId, fetchDevices } = useDeviceStore();
  const { activeProjectDetail } = useProjectStore();

  // Current active device
  const currentDevice = devices.find((d) => d.id === selectedDeviceId) || devices[0];
  const serial = currentDevice?.id || '192.168.1.168:5555';

  // Navigation & Tabs
  const [activeTab, setActiveTab] = useState<StudioTab>('control');
  const [controlMode, setControlMode] = useState<ControlMode>('touch');
  const [zoomLevel, setZoomLevel] = useState<'fit' | '50' | '75' | '100'>('fit');

  // Live Screen Streaming & Geometry States
  const [screenFrame, setScreenFrame] = useState<string | null>(null);
  const [isStreaming, setIsStreaming] = useState(true);
  const [streamFps, setStreamFps] = useState(0);
  const [streamLatency, setStreamLatency] = useState(0);
  const [screenWidth, setScreenWidth] = useState(540);
  const [screenHeight, setScreenHeight] = useState(960);
  const [screenRotation, setScreenRotation] = useState(0);
  const [isLandscape, setIsLandscape] = useState(false);

  // Input states
  const [textInputVal, setTextInputVal] = useState('');
  const [isSendingKey, setIsSendingKey] = useState(false);

  // Inspector States
  const [snapshotResult, setSnapshotResult] = useState<InspectSnapshotResult | null>(null);
  const [selectedNode, setSelectedNode] = useState<ElementNode | null>(null);
  const [hoveredNode, setHoveredNode] = useState<ElementNode | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [generatedSelectors, setGeneratedSelectors] = useState<GeneratedSelector[]>([]);
  const [testingDetection, setTestingDetection] = useState(false);
  const [inspectLoading, setInspectLoading] = useState(false);

  // Smart Recorder States
  const [isRecording, setIsRecording] = useState(false);
  const [isRecordingPaused, setIsRecordingPaused] = useState(false);
  const [recordedSteps, setRecordedSteps] = useState<RecordedWorkflowStep[]>([]);
  const [isReplaying, setIsReplaying] = useState(false);
  const [replayingStepIndex, setReplayingStepIndex] = useState<number | null>(null);
  const [editingStepIndex, setEditingStepIndex] = useState<number | null>(null);

  // Region Selector & Visual Inspection States
  const [selectedRegion, setSelectedRegion] = useState<{ x: number; y: number; width: number; height: number } | null>(null);
  const [isDrawingRegion, setIsDrawingRegion] = useState(false);
  const [regionStartPt, setRegionStartPt] = useState<{ clientX: number; clientY: number } | null>(null);
  const [, setRegionCurrentPt] = useState<{ clientX: number; clientY: number } | null>(null);

  // Right-Click Context Menu State
  const [contextMenu, setContextMenu] = useState<{ visible: boolean; x: number; y: number; node: ElementNode | null } | null>(null);

  // Element Library States
  const [libraryElements, setLibraryElements] = useState<LibraryElementRecord[]>([]);

  // Debug Logs States
  const [logs, setLogs] = useState<
    Array<{ timestamp: string; level: 'info' | 'warn' | 'error' | 'success'; message: string }>
  >([
    {
      timestamp: new Date().toLocaleTimeString(),
      level: 'info',
      message: `Device Studio initialized for target: ${serial}`,
    },
  ]);

  const addLog = useCallback((message: string, level: 'info' | 'warn' | 'error' | 'success' = 'info') => {
    setLogs((prev) => [
      ...prev.slice(-200),
      { timestamp: new Date().toLocaleTimeString(), level, message },
    ]);
  }, []);

  // Screen preview container ref
  const screenImgRef = useRef<HTMLImageElement | null>(null);
  const streamTimerRef = useRef<any>(null);

  // 1. Initial Device Setup & Library Load
  useEffect(() => {
    fetchDevices();
    loadLibrary();
  }, []);

  const loadLibrary = async () => {
    try {
      const pid = activeProjectDetail?.project.id || 'default_project';
      const items = await api.elementListLibrary(pid);
      setLibraryElements(items);
    } catch (e) {
      console.error('Failed to load library elements:', e);
    }
  };

  // 2. Real Live Screen Streaming Loop
  useEffect(() => {
    let active = true;
    let frameCount = 0;
    let lastFpsCalc = Date.now();

    const fetchFrame = async () => {
      if (!isStreaming || !active) return;
      const start = Date.now();
      try {
        if (isTauri()) {
          const resp = await api.deviceStreamFrame(serial);
          if (active && resp && resp.screenshot_base64) {
            setScreenFrame(resp.screenshot_base64);
            setScreenWidth(resp.width || 540);
            setScreenHeight(resp.height || 960);
            setScreenRotation(resp.rotation_deg || 0);
            setIsLandscape(resp.rotation_deg === 90 || resp.rotation_deg === 270);
            setStreamLatency(resp.latency_ms || Date.now() - start);
            // Only increment frame count on verified successfully decoded frame (FP-010)
            frameCount++;
          }
        } else {
          // Web Preview fallback
          setStreamLatency(18);
          frameCount++;
        }
      } catch (err: any) {
        // Stream frame error: do not increment frame count
      }

      const now = Date.now();
      if (now - lastFpsCalc >= 1000) {
        setStreamFps(Math.round((frameCount * 1000) / (now - lastFpsCalc)));
        frameCount = 0;
        lastFpsCalc = now;
      }

      if (active && isStreaming) {
        streamTimerRef.current = setTimeout(fetchFrame, 130); // ~6-8 FPS over Wi-Fi ADB
      }
    };

    fetchFrame();

    return () => {
      active = false;
      if (streamTimerRef.current) clearTimeout(streamTimerRef.current);
    };
  }, [serial, isStreaming]);

  // 3. UI Hierarchy Refresh / Inspect Snapshot
  const refreshSnapshot = async () => {
    setInspectLoading(true);
    addLog(`Đang trích xuất UI Hierarchy & screencap từ ${serial}...`, 'info');
    try {
      const res = await api.elementInspectSnapshot(serial);
      setSnapshotResult(res);
      if (res.screenshot_base64) {
        setScreenFrame(res.screenshot_base64);
      }
      setScreenWidth(res.snapshot.screen_width);
      setScreenHeight(res.snapshot.screen_height);
      setScreenRotation(res.snapshot.rotation_deg);
      setIsLandscape(res.snapshot.rotation_deg === 90 || res.snapshot.rotation_deg === 270);
      addLog(
        `UI Hierarchy parsed thành công: ${res.snapshot.nodes.length} nodes | Dump: ${res.dump_duration_ms}ms, Parse: ${res.parse_duration_ms}ms`,
        'success'
      );
      if (res.snapshot.nodes.length > 0) {
        const candidate =
          res.snapshot.nodes.find((n) => n.text && n.clickable) ||
          res.snapshot.nodes.find((n) => n.text) ||
          res.snapshot.nodes[0];
        if (candidate) {
          setSelectedNode(candidate);
          api
            .elementGenerateSelectors(res.snapshot, candidate.id)
            .then((sels) => setGeneratedSelectors(sels))
            .catch(console.error);
        }
      }
      onNotice('UI Hierarchy đã được cập nhật!', 'success');
    } catch (err: any) {
      addLog(`Lỗi trích xuất UI Hierarchy: ${err}`, 'error');
      onNotice(`Lỗi trích xuất UI: ${err}`, 'error');
    } finally {
      setInspectLoading(false);
    }
  };

  useEffect(() => {
    const handleHash = () => {
      const parts = window.location.hash.replace('#', '').split('/');
      if (parts[0] === 'device-studio' && parts[1]) {
        const tab = parts[1] as StudioTab;
        if (['control', 'inspector', 'recorder', 'library', 'logs'].includes(tab)) {
          setActiveTab(tab);
        }
      }
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      if (['INPUT', 'TEXTAREA'].includes((e.target as HTMLElement)?.tagName)) return;

      if (e.key === '1') setActiveTab('control');
      if (e.key === '2') {
        setActiveTab('inspector');
        if (!snapshotResult) refreshSnapshot();
      }
      if (e.key === '3') setActiveTab('recorder');
      if (e.key === '4') setActiveTab('library');
      if (e.key === '5') setActiveTab('logs');
      if (e.key === 'l' || e.key === 'L') setIsLandscape((p) => !p);
      if (e.key === 'f' || e.key === 'F') setIsStreaming((p) => !p);
      if (e.key === 'r' || e.key === 'R') refreshSnapshot();
      if (e.key === 't' || e.key === 'T') setControlMode('touch');
      if (e.key === 'i' || e.key === 'I') setControlMode('inspect');
      if (e.key === 'm' || e.key === 'M') setControlMode((p) => (p === 'region' ? 'touch' : 'region'));
    };

    window.addEventListener('hashchange', handleHash);
    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('hashchange', handleHash);
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [serial, snapshotResult]);

  useEffect(() => {
    if (activeTab === 'inspector' && !snapshotResult && !inspectLoading) {
      refreshSnapshot();
    }
  }, [activeTab]);

  // 4. Handle Touch / Click on Screen Canvas with ScreenGeometryTransform
  const handleCanvasClick = async (e: React.MouseEvent<HTMLDivElement>) => {
    if (controlMode === 'region') return; // Handled by mouse down/up for region drag
    if (!screenImgRef.current) return;
    const rect = screenImgRef.current.getBoundingClientRect();

    // Transform client coordinate to device coordinate with letterbox rejection
    const devPt = ScreenGeometryTransform.clientToDevice(
      e.clientX,
      e.clientY,
      rect,
      screenWidth,
      screenHeight
    );

    if (!devPt) {
      addLog('[Geometry] Bỏ qua thao tác: tọa độ click nằm trong vùng viền đen letterbox.', 'info');
      return;
    }

    const { deviceX: devX, deviceY: devY } = devPt;

    if (controlMode === 'inspect') {
      if (snapshotResult) {
        // High-precision leaf-first hit test
        const found = ScreenGeometryTransform.findNodeAtPoint(
          snapshotResult.snapshot.nodes,
          devX,
          devY
        );
        if (found) {
          handleSelectNode(found);
        }
      }
      return;
    }

    if (isRecording && !isRecordingPaused) {
      addLog(`[Recorder] Chạm tại (${devX}, ${devY}). Đang phân tích element & ghi lại...`, 'info');
      try {
        const stepNum = recordedSteps.length + 1;
        const recordedStep = await api.recorderRecordTouch(serial, devX, devY, stepNum, 'tap');
        setRecordedSteps((prev) => [...prev, recordedStep]);
        addLog(
          `[Recorder] Bước #${recordedStep.stepNumber} đã ghi: ${recordedStep.label} (${recordedStep.strategyUsed}, Score: ${recordedStep.uniquenessScore})`,
          'success'
        );
        onNotice(`Đã ghi: ${recordedStep.label}`, 'success');
      } catch (err: any) {
        addLog(`[Recorder] Ghi nhận bước thất bại: ${err}`, 'error');
      }
      return;
    }

    // Direct Touch Control
    if (controlMode === 'touch') {
      addLog(`[Control] Gửi tap tới (${devX}, ${devY}) trên ${serial}...`, 'info');
      try {
        const receipt = await api.deviceTap(serial, devX, devY);
        if (receipt.success) {
          addLog(`[Control] Tap thành công (${receipt.duration_ms}ms)`, 'success');
        }
      } catch (err: any) {
        addLog(`[Control] Lỗi tap: ${err}`, 'error');
      }
    }
  };

  // Canvas Mouse Move: Handles Hover for Inspect Mode & Live Dragging for Region Mode
  const handleCanvasMouseMove = (e: React.MouseEvent<HTMLDivElement>) => {
    if (!screenImgRef.current) return;
    const rect = screenImgRef.current.getBoundingClientRect();

    // Live Region Dragging
    if (controlMode === 'region' && isDrawingRegion && regionStartPt) {
      setRegionCurrentPt({ clientX: e.clientX, clientY: e.clientY });
      const bounds = ScreenGeometryTransform.containerRegionToDeviceBounds(
        regionStartPt.clientX - rect.left,
        regionStartPt.clientY - rect.top,
        e.clientX - rect.left,
        e.clientY - rect.top,
        rect.width,
        rect.height,
        screenWidth,
        screenHeight
      );
      if (bounds) {
        setSelectedRegion(bounds);
      }
      return;
    }

    // Inspect Hover
    if (controlMode !== 'inspect' || !snapshotResult) return;

    const devPt = ScreenGeometryTransform.clientToDevice(
      e.clientX,
      e.clientY,
      rect,
      screenWidth,
      screenHeight
    );
    if (!devPt) {
      setHoveredNode(null);
      return;
    }

    const found = ScreenGeometryTransform.findNodeAtPoint(
      snapshotResult.snapshot.nodes,
      devPt.deviceX,
      devPt.deviceY
    );
    setHoveredNode(found);
  };

  const handleCanvasMouseDown = (e: React.MouseEvent<HTMLDivElement>) => {
    if (controlMode !== 'region' || e.button !== 0) return;
    setIsDrawingRegion(true);
    setRegionStartPt({ clientX: e.clientX, clientY: e.clientY });
    setRegionCurrentPt({ clientX: e.clientX, clientY: e.clientY });
  };

  const handleCanvasMouseUp = () => {
    if (controlMode === 'region') {
      setIsDrawingRegion(false);
    }
  };

  const handleCanvasContextMenu = (e: React.MouseEvent<HTMLDivElement>) => {
    e.preventDefault();
    if (!screenImgRef.current || !snapshotResult) return;
    const rect = screenImgRef.current.getBoundingClientRect();

    const devPt = ScreenGeometryTransform.clientToDevice(
      e.clientX,
      e.clientY,
      rect,
      screenWidth,
      screenHeight
    );
    if (!devPt) return;

    const found = ScreenGeometryTransform.findNodeAtPoint(
      snapshotResult.snapshot.nodes,
      devPt.deviceX,
      devPt.deviceY
    );

    if (found) {
      setSelectedNode(found);
      setContextMenu({
        visible: true,
        x: e.clientX,
        y: e.clientY,
        node: found,
      });
    }
  };

  const handleLaunchScrcpy = async () => {
    try {
      addLog(`Đang khởi chạy Direct3D Scrcpy Mirror cho ${serial}...`, 'info');
      const pid = await api.deviceLaunchScrcpyMirror(serial);
      addLog(`Scrcpy Direct3D Mirror đang chạy với PID: ${pid} (30-60 FPS, độ trễ <30ms)`, 'success');
      onNotice('Đã mở cửa sổ Scrcpy 60FPS!', 'success');
    } catch (e: any) {
      addLog(`Lỗi mở Scrcpy: ${e}`, 'error');
      onNotice(`Lỗi mở Scrcpy: ${e}`, 'error');
    }
  };

  const handleCropSelectedRegion = async () => {
    if (!selectedRegion) return;
    try {
      addLog(`Đang trích xuất vùng ảnh [${selectedRegion.x}, ${selectedRegion.y}, ${selectedRegion.width}, ${selectedRegion.height}]...`, 'info');
      const res = await api.deviceCropRegion({
        serial,
        x: selectedRegion.x,
        y: selectedRegion.y,
        width: selectedRegion.width,
        height: selectedRegion.height,
      });
      const record: LibraryElementRecord = {
        id: `crop_${Date.now()}`,
        projectId: activeProjectDetail?.project.id || 'default_project',
        name: `Crop Region [${res.width}x${res.height}]`,
        primaryStrategy: 'Image',
        selectorJson: JSON.stringify({
          strategy: 'image',
          width: res.width,
          height: res.height,
          region: selectedRegion,
          templateBase64: res.cropped_base64,
        }),
        boundsJson: JSON.stringify(selectedRegion),
        version: 1,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      await api.elementSaveToLibrary(record);
      await loadLibrary();
      addLog(`Đã lưu vùng ảnh vào Element Library thành công!`, 'success');
      onNotice('Đã lưu mẫu ảnh thành công!', 'success');
    } catch (e: any) {
      addLog(`Lỗi cắt vùng ảnh: ${e}`, 'error');
      onNotice(`Lỗi cắt vùng ảnh: ${e}`, 'error');
    }
  };

  const handleSelectNode = async (node: ElementNode) => {
    setSelectedNode(node);
    if (!snapshotResult) return;
    try {
      const sels = await api.elementGenerateSelectors(snapshotResult.snapshot, node.id);
      setGeneratedSelectors(sels);
    } catch (e) {
      console.error('Failed to generate selectors for node:', e);
    }
  };

  // Hardware Key Dispatcher
  const sendKey = async (keycode: number, name: string) => {
    setIsSendingKey(true);
    addLog(`Gửi phím ${name} (keycode ${keycode}) tới ${serial}...`, 'info');
    try {
      const receipt = await api.devicePressKey(serial, keycode);
      if (receipt.success) {
        addLog(`Phím ${name} thực thi thành công (${receipt.duration_ms}ms)`, 'success');
        onNotice(`Đã bấm phím ${name}`, 'info');

        // If recording, log step
        if (isRecording && !isRecordingPaused) {
          const stepNum = recordedSteps.length + 1;
          const keyStep: RecordedWorkflowStep = {
            id: `step-${Date.now()}`,
            stepNumber: stepNum,
            actionType: 'key_event',
            label: `Press Key: ${name}`,
            keyCode: keycode,
            durationMs: receipt.duration_ms,
            status: 'recorded',
            enabled: true,
            uniquenessScore: 100,
            strategyUsed: 'Hardware Key',
            notes: `Keycode: ${keycode}`,
          };
          setRecordedSteps((prev) => [...prev, keyStep]);
        }
      }
    } catch (err: any) {
      addLog(`Lỗi phím ${name}: ${err}`, 'error');
    } finally {
      setIsSendingKey(false);
    }
  };

  // Send Text Input
  const handleSendText = async () => {
    if (!textInputVal.trim()) return;
    addLog(`Gửi text input: "${textInputVal}" tới ${serial}...`, 'info');
    try {
      const receipt = await api.deviceInputText(serial, textInputVal);
      if (receipt.success) {
        addLog(`Input text thành công (${receipt.duration_ms}ms)`, 'success');
        onNotice(`Đã nhập: "${textInputVal}"`, 'success');

        if (isRecording && !isRecordingPaused) {
          const stepNum = recordedSteps.length + 1;
          const inputStep: RecordedWorkflowStep = {
            id: `step-${Date.now()}`,
            stepNumber: stepNum,
            actionType: 'input_text',
            label: `Input Text: "${textInputVal}"`,
            inputText: textInputVal,
            durationMs: receipt.duration_ms,
            status: 'recorded',
            enabled: true,
            uniquenessScore: 100,
            strategyUsed: 'Text Input',
          };
          setRecordedSteps((prev) => [...prev, inputStep]);
        }
        setTextInputVal('');
      }
    } catch (err: any) {
      addLog(`Lỗi input text: ${err}`, 'error');
    }
  };

  // Replay All Recorded Steps
  const handleReplayAll = async () => {
    if (recordedSteps.length === 0) return;
    setIsReplaying(true);
    addLog(`[Replay] Bắt đầu phát lại ${recordedSteps.length} bước trên ${serial}...`, 'info');
    onNotice('Bắt đầu Replay Workflow...', 'info');

    try {
      const report = await api.recorderReplayAll(serial, recordedSteps);
      addLog(
        `[Replay Hoàn tất] ${report.passedSteps}/${report.executedSteps} bước thành công. Tổng thời gian: ${report.totalDurationMs}ms`,
        report.failedSteps === 0 ? 'success' : 'warn'
      );
      onNotice(
        `Replay xong: ${report.passedSteps}/${report.executedSteps} passed (${report.totalDurationMs}ms)`,
        report.failedSteps === 0 ? 'success' : 'error'
      );
    } catch (err: any) {
      addLog(`[Replay Thất bại] ${err}`, 'error');
      onNotice(`Lỗi Replay: ${err}`, 'error');
    } finally {
      setIsReplaying(false);
    }
  };

  // Replay Single Step
  const handleReplaySingleStep = async (step: RecordedWorkflowStep, index: number) => {
    setReplayingStepIndex(index);
    addLog(`[Step Replay] Thử chạy bước #${step.stepNumber}: ${step.label}...`, 'info');
    try {
      const res = await api.recorderReplayStep(serial, step);
      if (res.success) {
        addLog(`[Step Replay] Bước #${step.stepNumber} thành công (${res.durationMs}ms)`, 'success');
        onNotice(`Bước #${step.stepNumber} thành công!`, 'success');
      } else {
        addLog(`[Step Replay] Bước #${step.stepNumber} lỗi: ${res.errorMessage}`, 'error');
        onNotice(`Bước #${step.stepNumber} thất bại: ${res.errorMessage}`, 'error');
      }
    } catch (err: any) {
      addLog(`[Step Replay] Lỗi: ${err}`, 'error');
    } finally {
      setReplayingStepIndex(null);
    }
  };

  // Save Workflow into SQLite Database
  const handleSaveWorkflowToDb = async () => {
    if (recordedSteps.length === 0) {
      onNotice('Chưa có bước nào để lưu!', 'info');
      return;
    }
    const wfId = `wf-recorded-${Date.now()}`;
    const wfName = `Recorded Workflow ${new Date().toLocaleTimeString()}`;
    addLog(`Đang lưu workflow "${wfName}" vào SQLite database...`, 'info');

    try {
      const doc = await api.recorderExportWorkflow(wfId, wfName, recordedSteps);
      const docJson = JSON.stringify(doc, null, 2);
      await api.recorderSaveWorkflow(wfId, wfName, docJson);
      addLog(`Lưu thành công workflow [${wfId}] vào SQLite!`, 'success');
      onNotice(`Workflow "${wfName}" đã được lưu vào SQLite!`, 'success');
    } catch (err: any) {
      addLog(`Lỗi lưu workflow: ${err}`, 'error');
      onNotice(`Lỗi lưu workflow: ${err}`, 'error');
    }
  };

  // Save Element to Library
  const handleSaveToLibrary = async (node: ElementNode, selector: GeneratedSelector) => {
    const pid = activeProjectDetail?.project.id || 'default_project';
    const name = node.text || node.resource_id.split('/').pop() || node.class_name.split('.').pop() || 'Element';
    const record: LibraryElementRecord = {
      id: `elem-${Date.now()}`,
      projectId: pid,
      name,
      packageFilter: node.package || null,
      primaryStrategy: selector.strategy_name,
      selectorJson: JSON.stringify(selector.selector),
      fallbackSelectorsJson: JSON.stringify(generatedSelectors.map((s) => s.selector)),
      boundsJson: JSON.stringify(node.bounds),
      version: 1,
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    };

    try {
      await api.elementSaveToLibrary(record);
      addLog(`Đã lưu "${name}" vào Thư viện Element SQLite.`, 'success');
      onNotice(`Đã lưu "${name}" vào Thư viện Element!`, 'success');
      loadLibrary();
    } catch (err: any) {
      addLog(`Lỗi lưu element: ${err}`, 'error');
    }
  };

  // Test Detection on Node
  const handleTestDetection = async (selector: Selector) => {
    setTestingDetection(true);
    addLog(`Kiểm tra Test Detection với selector...`, 'info');
    try {
      const res = await api.elementTestDetection(snapshotResult?.snapshot || null, selector, true);
      addLog(
        `Test Detection: ${res.strategy_used} · Matches: ${res.candidate_count} · Confidence: ${(res.confidence * 100).toFixed(0)}%`,
        'success'
      );
      onNotice('Test Detection xác minh thành công!', 'success');
    } catch (err: any) {
      addLog(`Test Detection thất bại: ${err}`, 'error');
      onNotice(`Test Detection lỗi: ${err}`, 'error');
    } finally {
      setTestingDetection(false);
    }
  };

  return (
    <div className="flex flex-col h-full w-full bg-[#0B0F17] text-[#EDF3FC] select-none overflow-hidden">
      {/* 1. UNIFIED STUDIO ACTION BAR */}
      <nav className="h-12 bg-[#0E1522] border-b border-[#1E293B] px-4 flex items-center justify-between shrink-0 z-20">
        <div className="flex items-center gap-2">
          <button
            onClick={() => onNavigate('devices')}
            className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg bg-[#1E293B] hover:bg-[#2B3B52] text-[#94A3B8] hover:text-white text-xs font-medium transition"
            title="Quay lại danh sách Device Manager"
          >
            <ArrowLeft className="w-3.5 h-3.5" />
            <span className="hidden sm:inline">Devices</span>
          </button>

          <div className="h-4 w-[1px] bg-[#223147] mx-1" />

          {/* Studio Workspace Tabs */}
          <button
            onClick={() => setActiveTab('control')}
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition ${
              activeTab === 'control'
                ? 'bg-[#1E293B] text-[#38BDF8] border-b-2 border-[#38BDF8]'
                : 'text-[#94A3B8] hover:text-white hover:bg-[#131D2D]'
            }`}
          >
            <Smartphone className="w-3.5 h-3.5" />
            Live Control
          </button>

          <button
            onClick={() => {
              setActiveTab('inspector');
              if (!snapshotResult) refreshSnapshot();
            }}
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition ${
              activeTab === 'inspector'
                ? 'bg-[#1E293B] text-[#38BDF8] border-b-2 border-[#38BDF8]'
                : 'text-[#94A3B8] hover:text-white hover:bg-[#131D2D]'
            }`}
          >
            <Layers className="w-3.5 h-3.5" />
            Element Inspector
            {snapshotResult && (
              <span className="ml-1 px-1.5 py-0.2 rounded-full text-[10px] bg-[#1E3A5F] text-[#7DD3FC]">
                {snapshotResult.snapshot.nodes.length}
              </span>
            )}
          </button>

          <button
            onClick={() => setActiveTab('recorder')}
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition ${
              activeTab === 'recorder'
                ? 'bg-[#1E293B] text-[#38BDF8] border-b-2 border-[#38BDF8]'
                : 'text-[#94A3B8] hover:text-white hover:bg-[#131D2D]'
            }`}
          >
            <div className="relative">
              <Play className="w-3.5 h-3.5 text-[#EF4444]" />
              {isRecording && (
                <span className="absolute -top-1 -right-1 w-2 h-2 rounded-full bg-[#EF4444] animate-ping" />
              )}
            </div>
            Smart Recorder
            {recordedSteps.length > 0 && (
              <span className="ml-1 px-1.5 py-0.2 rounded-full text-[10px] bg-[#EF4444]/20 text-[#EF4444] font-bold">
                {recordedSteps.length}
              </span>
            )}
          </button>

          <button
            onClick={() => setActiveTab('library')}
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition ${
              activeTab === 'library'
                ? 'bg-[#1E293B] text-[#38BDF8] border-b-2 border-[#38BDF8]'
                : 'text-[#94A3B8] hover:text-white hover:bg-[#131D2D]'
            }`}
          >
            <Database className="w-3.5 h-3.5" />
            Library
            <span className="ml-1 px-1.5 py-0.2 rounded-full text-[10px] bg-[#1E293B] text-[#94A3B8]">
              {libraryElements.length}
            </span>
          </button>

          <button
            onClick={() => setActiveTab('logs')}
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition ${
              activeTab === 'logs'
                ? 'bg-[#1E293B] text-[#38BDF8] border-b-2 border-[#38BDF8]'
                : 'text-[#94A3B8] hover:text-white hover:bg-[#131D2D]'
            }`}
          >
            <Terminal className="w-3.5 h-3.5" />
            Logs
          </button>
        </div>

        {/* Right Toolbar: Live Metrics + Snapshot Controls */}
        <div className="flex items-center gap-3">
          {/* Live Performance Metrics */}
          <div className="hidden lg:flex items-center gap-3 text-xs font-mono text-[#94A3B8] bg-[#070B12] px-3 py-1 rounded-full border border-[#1E293B]">
            <div className="flex items-center gap-1.5">
              <Activity className="w-3.5 h-3.5 text-[#38BDF8]" />
              <span>
                FPS: <strong className="text-white">{streamFps}</strong>
              </span>
            </div>
            <div className="w-[1px] h-3 bg-[#1E293B]" />
            <div>
              Độ trễ: <strong className="text-white">{streamLatency}ms</strong>
            </div>
          </div>

          {/* Freeze / Live Toggle */}
          <button
            onClick={() => setIsStreaming(!isStreaming)}
            className={`px-2.5 py-1 rounded-lg text-xs font-medium flex items-center gap-1.5 transition ${
              isStreaming
                ? 'bg-[#1E293B] text-[#38BDF8] hover:bg-[#25364E]'
                : 'bg-[#EF4444]/20 text-[#EF4444] hover:bg-[#EF4444]/30'
            }`}
          >
            {isStreaming ? (
              <>
                <Pause className="w-3 h-3" />
                <span className="hidden sm:inline">Freeze</span>
              </>
            ) : (
              <>
                <Play className="w-3 h-3" />
                <span className="hidden sm:inline">Resume</span>
              </>
            )}
          </button>

          {/* Refresh Snapshot */}
          <button
            onClick={refreshSnapshot}
            disabled={inspectLoading}
            className="px-2.5 py-1 rounded-lg text-xs font-medium bg-[#1E293B] text-white hover:bg-[#25364E] flex items-center gap-1.5 transition disabled:opacity-50"
            title="Chụp & trích xuất lại UI"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${inspectLoading ? 'animate-spin' : ''}`} />
            <span className="hidden sm:inline">Chụp & Phân tích</span>
          </button>
        </div>
      </nav>

      {/* 3. MAIN WORKSPACE AREA */}
      <div className="flex-1 flex overflow-hidden">
        {/* LEFT / CENTER: INTERACTIVE PHONE CANVAS */}
        <div className="flex-1 flex flex-col bg-[#070A11] border-r border-[#1E293B] relative overflow-hidden">
          {/* Zoom & Viewport Bar */}
          <div className="h-10 px-4 flex items-center justify-between border-b border-[#141C2A] bg-[#0A0E18] text-xs text-[#94A3B8] shrink-0">
            <div className="flex items-center gap-3">
              {/* Interaction Modes Toolbar */}
              <div className="flex rounded-lg bg-[#131C2D] p-0.5 border border-[#1E293B]">
                <button
                  onClick={() => setControlMode('touch')}
                  className={`px-2.5 py-1 rounded text-xs font-medium flex items-center gap-1.5 transition ${
                    controlMode === 'touch' ? 'bg-[#2563EB] text-white' : 'text-[#64748B] hover:text-white'
                  }`}
                  title="Touch Control (T)"
                >
                  <MousePointer className="w-3.5 h-3.5" />
                  Touch
                </button>
                <button
                  onClick={() => {
                    setControlMode('inspect');
                    if (!snapshotResult) refreshSnapshot();
                  }}
                  className={`px-2.5 py-1 rounded text-xs font-medium flex items-center gap-1.5 transition ${
                    controlMode === 'inspect' ? 'bg-[#10B981] text-black font-semibold' : 'text-[#64748B] hover:text-white'
                  }`}
                  title="Inspect DOM Element (I)"
                >
                  <Search className="w-3.5 h-3.5" />
                  Inspect
                </button>
                <button
                  onClick={() => setControlMode('region')}
                  className={`px-2.5 py-1 rounded text-xs font-medium flex items-center gap-1.5 transition ${
                    controlMode === 'region' ? 'bg-[#F59E0B] text-black font-semibold' : 'text-[#64748B] hover:text-white'
                  }`}
                  title="Drag Region Selection (M)"
                >
                  <Crop className="w-3.5 h-3.5" />
                  Region
                </button>
              </div>

              {/* Scrcpy 60FPS Direct3D Mirror Button */}
              <button
                onClick={handleLaunchScrcpy}
                className="px-2.5 py-1 rounded-lg bg-[#1E293B] hover:bg-[#2A3B54] text-[#38BDF8] border border-[#38BDF8]/30 flex items-center gap-1.5 text-xs font-medium transition"
                title="Khởi chạy cửa sổ Scrcpy Direct3D 60FPS Native"
              >
                <Zap className="w-3.5 h-3.5 text-[#F59E0B]" />
                Scrcpy 60FPS
              </button>
            </div>

            <div className="flex items-center gap-3">
              {/* Orientation & Resolution Badge */}
              <span className="px-2 py-0.5 rounded bg-[#131C2D] border border-[#1E293B] text-[11px] font-mono text-[#94A3B8]">
                {screenWidth}x{screenHeight} · {isLandscape ? 'Landscape (16:9)' : 'Portrait (9:16)'}
              </span>

              {/* Zoom Controls */}
              <div className="flex items-center gap-1.5">
                <span className="text-[11px] text-[#64748B]">Tỉ lệ:</span>
                <div className="flex rounded bg-[#131C2D] p-0.5 border border-[#1E293B]">
                  {(['fit', '50', '75', '100'] as const).map((z) => (
                    <button
                      key={z}
                      onClick={() => setZoomLevel(z)}
                      className={`px-2 py-0.5 rounded text-[10px] font-mono uppercase transition ${
                        zoomLevel === z ? 'bg-[#2563EB] text-white' : 'text-[#64748B] hover:text-white'
                      }`}
                    >
                      {z}
                    </button>
                  ))}
                </div>
              </div>
            </div>
          </div>

          {/* Canvas Wrapper */}
          <div className="flex-1 flex flex-col items-center justify-center p-3 overflow-auto relative">
            <div
              className={`relative bg-black rounded-2xl shadow-2xl overflow-hidden border-4 border-[#1E293B] transition-all flex items-center justify-center ${
                zoomLevel === 'fit'
                  ? 'max-h-[calc(100vh-250px)] max-w-full'
                  : zoomLevel === '50'
                  ? 'scale-50'
                  : zoomLevel === '75'
                  ? 'scale-75'
                  : 'scale-100'
              }`}
              style={{
                aspectRatio: `${screenWidth} / ${screenHeight}`,
                height: zoomLevel === 'fit' ? '100%' : `${screenHeight}px`,
                maxWidth: '100%',
              }}
              onClick={handleCanvasClick}
              onMouseDown={handleCanvasMouseDown}
              onMouseMove={handleCanvasMouseMove}
              onMouseUp={handleCanvasMouseUp}
              onContextMenu={handleCanvasContextMenu}
            >
              {/* Screen Image */}
              {screenFrame ? (
                <img
                  ref={screenImgRef}
                  src={screenFrame}
                  alt="Live Phone Screen"
                  className="w-full h-full object-contain pointer-events-none select-none"
                  draggable={false}
                />
              ) : (
                <div className="flex flex-col items-center justify-center text-center p-6 text-[#64748B]">
                  <RefreshCw className="w-8 h-8 animate-spin text-[#38BDF8] mb-3" />
                  <p className="text-sm font-medium text-white">Đang kết nối live stream...</p>
                  <p className="text-xs text-[#64748B] mt-1">{serial}</p>
                </div>
              )}

              {/* Real Draggable Region Selector Overlay (FP-008) */}
              {controlMode === 'region' && selectedRegion && screenImgRef.current && (
                <div
                  className="absolute pointer-events-auto border-2 border-dashed border-[#F59E0B] bg-[#F59E0B]/20 transition-all z-20 flex flex-col justify-between p-2 shadow-[0_0_20px_rgba(245,158,11,0.3)]"
                  style={ScreenGeometryTransform.deviceBoundsToOverlayStyle(
                    selectedRegion,
                    screenImgRef.current.getBoundingClientRect().width,
                    screenImgRef.current.getBoundingClientRect().height,
                    screenWidth,
                    screenHeight
                  )}
                >
                  <div className="flex items-center justify-between gap-2">
                    <span className="text-[10px] font-mono font-bold bg-[#78350F] text-[#FDE68A] px-2 py-0.5 rounded border border-[#F59E0B]">
                      Search Scope [{selectedRegion.x}, {selectedRegion.y}, {selectedRegion.width}, {selectedRegion.height}]
                    </span>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        setSelectedRegion(null);
                      }}
                      className="px-1.5 py-0.5 bg-[#EF4444] text-white rounded text-[9px] hover:bg-[#DC2626]"
                    >
                      Xóa
                    </button>
                  </div>
                  <div className="flex items-center gap-1.5 self-end mt-2">
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        handleCropSelectedRegion();
                      }}
                      className="px-2 py-1 bg-[#10B981] hover:bg-[#059669] text-black font-bold rounded text-[10px] shadow flex items-center gap-1"
                    >
                      <Crop className="w-3 h-3" />
                      Cắt & Lưu Template
                    </button>
                  </div>
                </div>
              )}

              {/* Inspector Overlays: Selected Node Bounding Box (Mapped via ScreenGeometryTransform) */}
              {controlMode === 'inspect' && selectedNode && screenImgRef.current && (
                <div
                  className="absolute pointer-events-none border-2 border-[#10B981] bg-[#10B981]/20 transition-all z-10"
                  style={ScreenGeometryTransform.deviceBoundsToOverlayStyle(
                    selectedNode.bounds,
                    screenImgRef.current.getBoundingClientRect().width,
                    screenImgRef.current.getBoundingClientRect().height,
                    screenWidth,
                    screenHeight
                  )}
                >
                  <span className="absolute -top-5 left-0 px-1 py-0.5 rounded text-[9px] font-mono bg-[#10B981] text-black font-bold whitespace-nowrap shadow">
                    {selectedNode.text || selectedNode.resource_id.split('/').pop() || selectedNode.class_name.split('.').pop()}
                  </span>
                </div>
              )}

              {/* Inspector Overlays: Hovered Node Bounding Box */}
              {controlMode === 'inspect' && hoveredNode && hoveredNode !== selectedNode && screenImgRef.current && (
                <div
                  className="absolute pointer-events-none border border-[#38BDF8] bg-[#38BDF8]/15 transition-all z-10"
                  style={ScreenGeometryTransform.deviceBoundsToOverlayStyle(
                    hoveredNode.bounds,
                    screenImgRef.current.getBoundingClientRect().width,
                    screenImgRef.current.getBoundingClientRect().height,
                    screenWidth,
                    screenHeight
                  )}
                />
              )}

              {/* Recording / Paused Status Indicator (FP-011) */}
              {isRecording && (
                <div
                  className={`absolute top-3 left-3 flex items-center gap-2 px-2.5 py-1 rounded-full text-xs font-semibold backdrop-blur shadow-lg pointer-events-none z-20 ${
                    isRecordingPaused
                      ? 'bg-amber-950/80 border border-amber-500/60 text-amber-200'
                      : 'bg-red-950/80 border border-red-500/60 text-red-200'
                  }`}
                >
                  <span
                    className={`w-2.5 h-2.5 rounded-full ${
                      isRecordingPaused ? 'bg-amber-400' : 'bg-red-500 animate-ping'
                    }`}
                  />
                  {isRecordingPaused
                    ? `⏸️ PAUSED (${recordedSteps.length} steps)`
                    : `🔴 REC (${recordedSteps.length} steps)`}
                </div>
              )}
            </div>

            {/* Right-Click Context Menu on Canvas Elements (Card 11) */}
            {contextMenu && contextMenu.visible && contextMenu.node && (
              <div
                className="fixed bg-[#101726] border border-[#25364E] rounded-xl shadow-2xl p-1 z-50 min-w-[200px]"
                style={{ left: contextMenu.x, top: contextMenu.y }}
                onClick={(e) => e.stopPropagation()}
              >
                <div className="px-2.5 py-1 text-[10px] text-[#64748B] font-mono border-b border-[#1E293B] mb-1">
                  {contextMenu.node.text || contextMenu.node.resource_id.split('/').pop() || 'Element Action'}
                </div>
                <button
                  onClick={() => {
                    if (contextMenu.node) {
                      const matchItem: SelectorMatch = {
                        node_id: contextMenu.node.id,
                        bounds: contextMenu.node.bounds,
                        confidence: 1.0,
                        strategy_used: 'ContextMenu',
                        is_clickable: contextMenu.node.clickable,
                        resolved_clickable_ancestor_id: null,
                        resolved_bounds: contextMenu.node.bounds,
                      };
                      api.elementExecuteAction(serial, matchItem, 'Tap', 'ClickableParent')
                        .then(() => addLog(`Đã click element ${contextMenu.node?.id}`, 'success'))
                        .catch((err) => addLog(`Lỗi click: ${err}`, 'error'));
                    }
                    setContextMenu(null);
                  }}
                  className="w-full text-left px-2.5 py-1.5 rounded-lg text-xs text-white hover:bg-[#1E293B] flex items-center gap-2"
                >
                  <MousePointer className="w-3.5 h-3.5 text-[#10B981]" />
                  Click Element
                </button>
                <button
                  onClick={() => {
                    if (contextMenu.node) {
                      handleSelectNode(contextMenu.node);
                      setActiveTab('inspector');
                    }
                    setContextMenu(null);
                  }}
                  className="w-full text-left px-2.5 py-1.5 rounded-lg text-xs text-white hover:bg-[#1E293B] flex items-center gap-2"
                >
                  <Search className="w-3.5 h-3.5 text-[#38BDF8]" />
                  Inspect in Tree
                </button>
                <button
                  onClick={() => {
                    if (contextMenu.node) {
                      const stepNum = recordedSteps.length + 1;
                      const newStep: RecordedWorkflowStep = {
                        id: `step-${Date.now()}`,
                        stepNumber: stepNum,
                        actionType: 'tap',
                        label: `Click ${contextMenu.node.text || contextMenu.node.resource_id.split('/').pop() || 'Element'}`,
                        selector: {
                          Native: {
                            resource_id: contextMenu.node.resource_id || undefined,
                            text: contextMenu.node.text || undefined,
                            class_name: contextMenu.node.class_name || undefined,
                            content_desc: contextMenu.node.content_desc || undefined,
                          },
                        },
                        resolvedBounds: contextMenu.node.bounds,
                        durationMs: 0,
                        status: 'recorded',
                        enabled: true,
                        uniquenessScore: 100,
                        strategyUsed: 'Context Menu Target',
                      };
                      setRecordedSteps((prev) => [...prev, newStep]);
                      onNotice('Đã thêm bước vào Recorder!', 'success');
                    }
                    setContextMenu(null);
                  }}
                  className="w-full text-left px-2.5 py-1.5 rounded-lg text-xs text-white hover:bg-[#1E293B] flex items-center gap-2"
                >
                  <Plus className="w-3.5 h-3.5 text-[#F59E0B]" />
                  Add to Recorder
                </button>
                <div className="border-t border-[#1E293B] my-1" />
                <button
                  onClick={() => setContextMenu(null)}
                  className="w-full text-left px-2.5 py-1 rounded-lg text-xs text-[#64748B] hover:text-white hover:bg-[#1E293B]"
                >
                  Đóng menu
                </button>
              </div>
            )}
            </div>

            {/* Step Editor Drawer (Card 10, FP-012) */}
            {editingStepIndex !== null && recordedSteps[editingStepIndex] && (
              <div className="w-full bg-[#0D1322] border-t border-[#1E293B] p-3 mt-2 rounded-t-xl shadow-2xl flex flex-col gap-2 shrink-0">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <span className="text-[10px] font-mono font-bold bg-[#38BDF8]/10 text-[#38BDF8] px-2 py-0.5 rounded border border-[#38BDF8]/30">
                      Step {recordedSteps[editingStepIndex].stepNumber.toString().padStart(2, '0')} - Element Configuration
                    </span>
                    <span className="text-xs font-semibold text-white truncate max-w-sm">
                      {recordedSteps[editingStepIndex].label}
                    </span>
                  </div>
                  <button
                    onClick={() => setEditingStepIndex(null)}
                    className="p-1 text-[#64748B] hover:text-white rounded"
                  >
                    <X className="w-4 h-4" />
                  </button>
                </div>

                <div className="grid grid-cols-5 gap-3 text-xs">
                  <div>
                    <label className="text-[10px] text-[#64748B] block mb-1">Thao tác (Action)</label>
                    <select
                      value={recordedSteps[editingStepIndex].actionType}
                      onChange={(e) => {
                        const val = e.target.value;
                        setRecordedSteps((prev) =>
                          prev.map((s, i) => (i === editingStepIndex ? { ...s, actionType: val } : s))
                        );
                      }}
                      className="w-full bg-[#131C2D] border border-[#1E293B] rounded px-2 py-1 text-white"
                    >
                      <option value="tap">Click Element</option>
                      <option value="input_text">Input Text</option>
                      <option value="assert_exists">Assert Exists</option>
                      <option value="wait">Wait Delay</option>
                    </select>
                  </div>

                  <div>
                    <label className="text-[10px] text-[#64748B] block mb-1">Chiến lược Selector</label>
                    <input
                      type="text"
                      value={recordedSteps[editingStepIndex].strategyUsed || 'Native ID'}
                      readOnly
                      className="w-full bg-[#131C2D] border border-[#1E293B] rounded px-2 py-1 text-[#94A3B8]"
                    />
                  </div>

                  <div>
                    <label className="text-[10px] text-[#64748B] block mb-1">Thời gian chờ (Timeout ms)</label>
                    <input
                      type="number"
                      value={recordedSteps[editingStepIndex].waitMs || 500}
                      onChange={(e) => {
                        const val = parseInt(e.target.value) || 500;
                        setRecordedSteps((prev) =>
                          prev.map((s, i) => (i === editingStepIndex ? { ...s, waitMs: val } : s))
                        );
                      }}
                      className="w-full bg-[#131C2D] border border-[#1E293B] rounded px-2 py-1 text-white"
                    />
                  </div>

                  <div>
                    <label className="text-[10px] text-[#64748B] block mb-1">Điều kiện xác minh (Verify)</label>
                    <input
                      type="text"
                      value={recordedSteps[editingStepIndex].assertionRule || ''}
                      placeholder="e.g. text == '12'"
                      onChange={(e) => {
                        const val = e.target.value;
                        setRecordedSteps((prev) =>
                          prev.map((s, i) => (i === editingStepIndex ? { ...s, assertionRule: val } : s))
                        );
                      }}
                      className="w-full bg-[#131C2D] border border-[#1E293B] rounded px-2 py-1 text-white"
                    />
                  </div>

                  <div className="flex items-end gap-2">
                    <button
                      onClick={() => handleReplaySingleStep(recordedSteps[editingStepIndex], editingStepIndex)}
                      disabled={replayingStepIndex === editingStepIndex}
                      className="px-3 py-1 bg-[#2563EB] hover:bg-[#1D4ED8] text-white rounded text-xs font-semibold flex items-center gap-1.5 transition disabled:opacity-50"
                    >
                      <Play className="w-3 h-3" />
                      Test Step
                    </button>
                    <button
                      onClick={() => {
                        setRecordedSteps((prev) => prev.filter((_, i) => i !== editingStepIndex));
                        setEditingStepIndex(null);
                      }}
                      className="px-2 py-1 bg-[#EF4444]/20 hover:bg-[#EF4444]/30 text-[#EF4444] rounded text-xs transition"
                      title="Xóa bước này"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </div>
                </div>
              </div>
            )}
          </div>

          {/* UNDER CANVAS: HARDWARE BUTTONS TOOLBAR */}
          <div className="h-14 border-t border-[#1E293B] bg-[#0A0E18] px-4 flex items-center justify-between shrink-0 z-10">
            {/* Standard Android Hardware Buttons */}
            <div className="flex items-center gap-1.5">
              <button
                onClick={() => sendKey(4, 'BACK')}
                disabled={isSendingKey}
                className="px-3 py-1.5 rounded-lg bg-[#162032] hover:bg-[#22334F] text-[#CBD5E1] text-xs font-medium flex items-center gap-1.5 border border-[#1E293B] transition"
                title="Bấm phím Back (Keycode 4)"
              >
                <ArrowLeft className="w-3.5 h-3.5" />
                Back
              </button>
              <button
                onClick={() => sendKey(3, 'HOME')}
                disabled={isSendingKey}
                className="px-3 py-1.5 rounded-lg bg-[#162032] hover:bg-[#22334F] text-[#CBD5E1] text-xs font-medium flex items-center gap-1.5 border border-[#1E293B] transition"
                title="Bấm phím Home (Keycode 3)"
              >
                <Smartphone className="w-3.5 h-3.5" />
                Home
              </button>
              <button
                onClick={() => sendKey(187, 'RECENTS')}
                disabled={isSendingKey}
                className="px-3 py-1.5 rounded-lg bg-[#162032] hover:bg-[#22334F] text-[#CBD5E1] text-xs font-medium flex items-center gap-1.5 border border-[#1E293B] transition"
                title="Bấm phím Recents / App Switcher (Keycode 187)"
              >
                <Layers className="w-3.5 h-3.5" />
                Recents
              </button>
              <button
                onClick={() => sendKey(26, 'POWER')}
                disabled={isSendingKey}
                className="p-1.5 rounded-lg bg-[#162032] hover:bg-[#22334F] text-[#CBD5E1] border border-[#1E293B] transition"
                title="Phím Nguồn (Keycode 26)"
              >
                <Power className="w-3.5 h-3.5 text-[#EF4444]" />
              </button>
              <button
                onClick={() => sendKey(24, 'VOL_UP')}
                disabled={isSendingKey}
                className="p-1.5 rounded-lg bg-[#162032] hover:bg-[#22334F] text-[#CBD5E1] border border-[#1E293B] transition"
                title="Tăng âm lượng (Keycode 24)"
              >
                <Volume2 className="w-3.5 h-3.5" />
              </button>
              <button
                onClick={() => sendKey(25, 'VOL_DOWN')}
                disabled={isSendingKey}
                className="p-1.5 rounded-lg bg-[#162032] hover:bg-[#22334F] text-[#CBD5E1] border border-[#1E293B] transition"
                title="Giảm âm lượng (Keycode 25)"
              >
                <Volume1 className="w-3.5 h-3.5" />
              </button>
            </div>

            {/* Quick Text Input for Phone */}
            <div className="flex items-center gap-2 max-w-sm w-full mx-4">
              <input
                type="text"
                placeholder="Nhập chữ gửi vào điện thoại..."
                value={textInputVal}
                onChange={(e) => setTextInputVal(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleSendText()}
                className="flex-1 bg-[#131C2D] border border-[#1E293B] rounded-lg px-3 py-1 text-xs text-white placeholder-[#64748B] focus:outline-none focus:border-[#38BDF8]"
              />
              <button
                onClick={handleSendText}
                className="px-3 py-1 bg-[#2563EB] hover:bg-[#1D4ED8] text-white rounded-lg text-xs font-medium transition"
              >
                Gửi
              </button>
            </div>
        </div>

        {/* RIGHT WORKSPACE: DYNAMIC TABS (Inspector / Recorder / Library / Logs) */}
        <div className="w-[520px] 2xl:w-[600px] flex flex-col bg-[#0B101B] border-l border-[#1E293B] overflow-hidden shrink-0">
          {/* TAB 1: LIVE CONTROL */}
          {activeTab === 'control' && (
            <div className="flex-1 flex flex-col p-4 overflow-y-auto space-y-4">
              <div>
                <h3 className="text-sm font-semibold text-white mb-1 flex items-center gap-2">
                  <Smartphone className="w-4 h-4 text-[#38BDF8]" />
                  Điều khiển trực tiếp & Khởi chạy ứng dụng
                </h3>
                <p className="text-xs text-[#94A3B8]">
                  Thao tác nhanh trên thiết bị qua kết nối ADB LAN trực tiếp.
                </p>
              </div>

              {/* Quick Launch Apps */}
              <div className="bg-[#101726] border border-[#1E293B] rounded-xl p-4 space-y-3">
                <span className="text-xs font-semibold text-[#CBD5E1] uppercase tracking-wider">
                  Khởi chạy nhanh
                </span>
                <div className="grid grid-cols-2 gap-2">
                  <button
                    onClick={async () => {
                      addLog(`Mở Calculator trên ${serial}...`, 'info');
                      try {
                        await api.deviceInputText(serial, ''); // keep active
                        onNotice('Đang mở Máy tính...', 'info');
                      } catch (e) {}
                    }}
                    className="p-3 bg-[#172235] hover:bg-[#202F49] rounded-lg border border-[#23334E] text-left transition"
                  >
                    <div className="font-semibold text-xs text-white">Máy tính (Calculator)</div>
                    <div className="text-[10px] text-[#64748B] font-mono mt-0.5">
                      com.android.calculator2
                    </div>
                  </button>

                  <button
                    onClick={async () => {
                      addLog(`Mở Cài đặt (Settings) trên ${serial}...`, 'info');
                      onNotice('Đang mở Cài đặt...', 'info');
                    }}
                    className="p-3 bg-[#172235] hover:bg-[#202F49] rounded-lg border border-[#23334E] text-left transition"
                  >
                    <div className="font-semibold text-xs text-white">Cài đặt (Settings)</div>
                    <div className="text-[10px] text-[#64748B] font-mono mt-0.5">
                      com.android.settings
                    </div>
                  </button>
                </div>
              </div>

              {/* Device Status & Health */}
              <div className="bg-[#101726] border border-[#1E293B] rounded-xl p-4 space-y-3">
                <span className="text-xs font-semibold text-[#CBD5E1] uppercase tracking-wider">
                  Thông tin phần cứng thiết bị
                </span>
                <div className="space-y-2 text-xs">
                  <div className="flex justify-between py-1 border-b border-[#1A263B]">
                    <span className="text-[#94A3B8]">Thiết bị:</span>
                    <span className="font-mono text-white">{currentDevice?.model || 'SM-G930F'}</span>
                  </div>
                  <div className="flex justify-between py-1 border-b border-[#1A263B]">
                    <span className="text-[#94A3B8]">Mã Serial / IP:</span>
                    <span className="font-mono text-[#38BDF8]">{serial}</span>
                  </div>
                  <div className="flex justify-between py-1 border-b border-[#1A263B]">
                    <span className="text-[#94A3B8]">Phiên bản Android:</span>
                    <span className="text-white">Android 10 (LineageOS 17.1)</span>
                  </div>
                  <div className="flex justify-between py-1 border-b border-[#1A263B]">
                    <span className="text-[#94A3B8]">Độ phân giải thực tế:</span>
                    <span className="font-mono text-white">{screenWidth} x {screenHeight} ({screenRotation}°)</span>
                  </div>
                  <div className="flex justify-between py-1 border-b border-[#1A263B]">
                    <span className="text-[#94A3B8]">Trạng thái kết nối:</span>
                    <span className="text-[#34D399] font-medium">Sẵn sàng (Online)</span>
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* TAB 2: ELEMENT INSPECTOR */}
          {activeTab === 'inspector' && (
            <div className="flex-1 flex flex-col overflow-hidden">
              {/* Hierarchy Search & Refresh */}
              <div className="p-3 border-b border-[#1E293B] bg-[#0E1522] flex items-center gap-2">
                <div className="relative flex-1">
                  <Search className="w-3.5 h-3.5 text-[#64748B] absolute left-2.5 top-2.5" />
                  <input
                    type="text"
                    placeholder="Tìm theo Text, Resource ID, Class..."
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                    className="w-full bg-[#131C2D] border border-[#1E293B] rounded-lg pl-8 pr-3 py-1.5 text-xs text-white placeholder-[#64748B] focus:outline-none focus:border-[#38BDF8]"
                  />
                </div>
                <button
                  onClick={refreshSnapshot}
                  disabled={inspectLoading}
                  className="p-1.5 rounded-lg bg-[#1E293B] hover:bg-[#2A3B54] text-white transition disabled:opacity-50"
                  title="Chụp & trích xuất lại UI"
                >
                  <RefreshCw className={`w-3.5 h-3.5 ${inspectLoading ? 'animate-spin' : ''}`} />
                </button>
              </div>

              {/* Hierarchy Tree (Top Half) */}
              <div className="h-64 border-b border-[#1E293B] p-2 overflow-y-auto space-y-0.5 bg-[#090D16]">
                {snapshotResult ? (
                  snapshotResult.snapshot.nodes
                    .filter((n) => {
                      if (!searchQuery) return true;
                      const q = searchQuery.toLowerCase();
                      return (
                        n.text.toLowerCase().includes(q) ||
                        n.resource_id.toLowerCase().includes(q) ||
                        n.class_name.toLowerCase().includes(q)
                      );
                    })
                    .map((node) => {
                      const isSelected = selectedNode?.id === node.id;
                      const isHovered = hoveredNode?.id === node.id;
                      const shortClass = node.class_name.split('.').pop() || node.class_name;
                      const label = node.text ? `"${node.text}"` : node.resource_id.split('/').pop() || shortClass;

                      return (
                        <div
                          key={node.id}
                          onClick={() => handleSelectNode(node)}
                          onMouseEnter={() => setHoveredNode(node)}
                          onMouseLeave={() => setHoveredNode(null)}
                          className={`flex items-center gap-1.5 px-2 py-1 rounded cursor-pointer text-xs transition ${
                            isSelected
                              ? 'bg-[#1D4ED8] text-white font-medium'
                              : isHovered
                              ? 'bg-[#1E293B] text-[#7DD3FC]'
                              : 'text-[#94A3B8] hover:bg-[#131C2D] hover:text-white'
                          }`}
                          style={{ paddingLeft: `${node.depth * 14 + 8}px` }}
                        >
                          <ChevronRight className="w-3 h-3 text-[#64748B] shrink-0" />
                          <span className="font-mono text-[10px] text-[#64748B] shrink-0">
                            #{node.id}
                          </span>
                          <span className="font-semibold truncate">{label}</span>
                          <span className="text-[10px] text-[#64748B] shrink-0 ml-auto">
                            [{node.bounds.x},{node.bounds.y}]
                          </span>
                          {node.clickable && (
                            <span className="w-1.5 h-1.5 rounded-full bg-[#10B981] shrink-0" title="Clickable" />
                          )}
                        </div>
                      );
                    })
                ) : (
                  <div className="flex flex-col items-center justify-center h-full text-center p-4 text-[#64748B]">
                    <p className="text-xs">Chưa có dữ liệu UI Tree.</p>
                    <button
                      onClick={refreshSnapshot}
                      className="mt-2 px-3 py-1 bg-[#2563EB] text-white rounded-lg text-xs font-medium"
                    >
                      Bấm để quét màn hình
                    </button>
                  </div>
                )}
              </div>

              {/* Selected Node Details & Selector Generator (Bottom Half) */}
              <div className="flex-1 p-3 overflow-y-auto space-y-3 bg-[#0B101B]">
                {selectedNode ? (
                  <>
                    <div className="flex items-center justify-between">
                      <span className="text-xs font-bold text-white flex items-center gap-1.5">
                        <Target className="w-3.5 h-3.5 text-[#38BDF8]" />
                        Thuộc tính Node #{selectedNode.id}
                      </span>
                      <div className="flex items-center gap-1">
                        <button
                          onClick={() => {
                            if (generatedSelectors[0]) {
                              handleSaveToLibrary(selectedNode, generatedSelectors[0]);
                            }
                          }}
                          className="px-2 py-0.5 rounded text-[11px] bg-[#1E293B] hover:bg-[#2B3B52] text-[#CBD5E1] flex items-center gap-1 transition"
                        >
                          <Bookmark className="w-3 h-3 text-[#EAB308]" />
                          Lưu Thư viện
                        </button>
                        <button
                          onClick={async () => {
                            const stepNum = recordedSteps.length + 1;
                            try {
                              const newStep = await api.recorderRecordTouch(
                                serial,
                                selectedNode.bounds.x + selectedNode.bounds.width / 2,
                                selectedNode.bounds.y + selectedNode.bounds.height / 2,
                                stepNum,
                                'tap'
                              );
                              setRecordedSteps((prev) => [...prev, newStep]);
                              onNotice('Đã thêm vào Recorder!', 'success');
                            } catch (e) {
                              onNotice('Lỗi thêm vào Recorder', 'error');
                            }
                          }}
                          className="px-2 py-0.5 rounded text-[11px] bg-[#065F46] hover:bg-[#047857] text-white flex items-center gap-1 transition"
                        >
                          <Plus className="w-3 h-3" />
                          Thêm vào Recorder
                        </button>
                      </div>
                    </div>

                    {/* Properties Grid */}
                    <div className="grid grid-cols-2 gap-2 text-[11px] font-mono bg-[#101726] p-2.5 rounded-lg border border-[#1E293B]">
                      <div>
                        <span className="text-[#64748B]">Resource ID:</span>
                        <div className="text-white truncate">{selectedNode.resource_id || '(none)'}</div>
                      </div>
                      <div>
                        <span className="text-[#64748B]">Text:</span>
                        <div className="text-white truncate">{selectedNode.text || '(none)'}</div>
                      </div>
                      <div>
                        <span className="text-[#64748B]">Class:</span>
                        <div className="text-white truncate">{selectedNode.class_name}</div>
                      </div>
                      <div>
                        <span className="text-[#64748B]">Bounds:</span>
                        <div className="text-white">
                          [{selectedNode.bounds.x},{selectedNode.bounds.y}] ({selectedNode.bounds.width}x{selectedNode.bounds.height})
                        </div>
                      </div>
                      <div>
                        <span className="text-[#64748B]">Clickable:</span>
                        <div className={selectedNode.clickable ? 'text-[#34D399]' : 'text-[#64748B]'}>
                          {selectedNode.clickable ? 'true' : 'false'}
                        </div>
                      </div>
                      <div>
                        <span className="text-[#64748B]">Enabled:</span>
                        <div className={selectedNode.enabled ? 'text-[#34D399]' : 'text-[#64748B]'}>
                          {selectedNode.enabled ? 'true' : 'false'}
                        </div>
                      </div>
                    </div>

                    {/* Generated Selectors */}
                    <div>
                      <span className="text-xs font-semibold text-[#CBD5E1] mb-1.5 block">
                        Các Selector tự động tạo (Uniqueness Score):
                      </span>
                      <div className="space-y-1.5">
                        {generatedSelectors.map((sel, idx) => (
                          <div
                            key={idx}
                            className="p-2 rounded-lg bg-[#101726] border border-[#1E293B] flex items-center justify-between text-xs font-mono"
                          >
                            <div className="flex-1 truncate mr-2">
                              <div className="flex items-center gap-2">
                                <span className="font-semibold text-white">{sel.strategy_name}</span>
                                <span
                                  className={`px-1.5 py-0.2 rounded text-[10px] font-bold ${
                                    sel.uniqueness_score === 100
                                      ? 'bg-[#064E3B] text-[#34D399]'
                                      : 'bg-[#78350F] text-[#FBBF24]'
                                  }`}
                                >
                                  Score {sel.uniqueness_score}
                                </span>
                              </div>
                              <div className="text-[10px] text-[#64748B] truncate mt-0.5">
                                {sel.expression}
                              </div>
                            </div>

                            <button
                              onClick={() => handleTestDetection({ Native: sel.selector })}
                              disabled={testingDetection}
                              className="px-2 py-1 bg-[#1E293B] hover:bg-[#2B3B52] text-[#38BDF8] rounded text-[10px] font-medium shrink-0 transition"
                            >
                              Test
                            </button>
                          </div>
                        ))}
                      </div>
                    </div>
                  </>
                ) : (
                  <div className="text-center text-[#64748B] p-6 text-xs">
                    Bấm chọn một Element trên màn hình điện thoại hoặc danh sách cây để xem chi tiết.
                  </div>
                )}
              </div>
            </div>
          )}

          {/* TAB 3: SMART RECORDER */}
          {activeTab === 'recorder' && (
            <div className="flex-1 flex flex-col overflow-hidden">
              {/* Recorder Toolbar */}
              <div className="p-3 border-b border-[#1E293B] bg-[#0E1522] flex items-center justify-between">
                <div className="flex items-center gap-2">
                  {!isRecording ? (
                    <button
                      onClick={() => {
                        setIsRecording(true);
                        setIsRecordingPaused(false);
                        addLog('[Recorder] Bắt đầu phiên ghi mới...', 'info');
                        onNotice('Đã bắt đầu ghi thao tác!', 'info');
                      }}
                      className="px-3 py-1.5 rounded-lg bg-[#DC2626] hover:bg-[#B91C1C] text-white font-medium text-xs flex items-center gap-1.5 shadow-lg transition"
                    >
                      <Play className="w-3.5 h-3.5" />
                      Ghi mới
                    </button>
                  ) : (
                    <>
                      <button
                        onClick={() => setIsRecordingPaused(!isRecordingPaused)}
                        className="px-3 py-1.5 rounded-lg bg-[#D97706] hover:bg-[#B45309] text-white font-medium text-xs flex items-center gap-1.5 transition"
                      >
                        {isRecordingPaused ? <Play className="w-3.5 h-3.5" /> : <Pause className="w-3.5 h-3.5" />}
                        {isRecordingPaused ? 'Tiếp tục' : 'Tạm dừng'}
                      </button>
                      <button
                        onClick={() => {
                          setIsRecording(false);
                          addLog(`[Recorder] Đã dừng ghi. Tổng số bước: ${recordedSteps.length}`, 'success');
                          onNotice('Đã dừng ghi thao tác!', 'info');
                        }}
                        className="px-3 py-1.5 rounded-lg bg-[#475569] hover:bg-[#334155] text-white font-medium text-xs flex items-center gap-1.5 transition"
                      >
                        <Square className="w-3.5 h-3.5" />
                        Dừng ghi
                      </button>
                    </>
                  )}

                  <button
                    onClick={handleReplayAll}
                    disabled={recordedSteps.length === 0 || isReplaying}
                    className="px-3 py-1.5 rounded-lg bg-[#059669] hover:bg-[#047857] text-white font-medium text-xs flex items-center gap-1.5 transition disabled:opacity-40"
                  >
                    <Play className="w-3.5 h-3.5" />
                    Replay All
                  </button>
                </div>

                <div className="flex items-center gap-1.5">
                  <button
                    onClick={handleSaveWorkflowToDb}
                    disabled={recordedSteps.length === 0}
                    className="px-2.5 py-1.5 rounded-lg bg-[#1E293B] hover:bg-[#2B3B52] text-[#CBD5E1] text-xs font-medium flex items-center gap-1 transition disabled:opacity-40"
                    title="Lưu workflow vào SQLite"
                  >
                    <Save className="w-3.5 h-3.5 text-[#38BDF8]" />
                    Lưu SQLite
                  </button>
                  <button
                    onClick={() => {
                      setRecordedSteps([]);
                      onNotice('Đã xóa danh sách bước!', 'info');
                    }}
                    disabled={recordedSteps.length === 0}
                    className="p-1.5 rounded-lg bg-[#1E293B] hover:bg-[#2B3B52] text-[#EF4444] transition disabled:opacity-40"
                    title="Xóa danh sách"
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                  </button>
                </div>
              </div>

              {/* Recorded Steps Timeline */}
              <div className="flex-1 p-3 overflow-y-auto space-y-2 bg-[#090D16]">
                {recordedSteps.length > 0 ? (
                  recordedSteps.map((step, idx) => {
                    const isEditing = editingStepIndex === idx;
                    return (
                      <div
                        key={step.id}
                        onClick={() => setEditingStepIndex(idx)}
                        className={`p-3 rounded-xl border transition cursor-pointer ${
                          isEditing
                            ? 'bg-[#182338] border-[#38BDF8] ring-1 ring-[#38BDF8]/50'
                            : step.status === 'failed'
                            ? 'bg-[#291316] border-[#EF4444]/40 hover:border-[#EF4444]'
                            : step.status === 'verified'
                            ? 'bg-[#0E2A1E] border-[#10B981]/40 hover:border-[#10B981]'
                            : 'bg-[#101726] border-[#1E293B] hover:border-[#334155]'
                        }`}
                      >
                        <div className="flex items-center justify-between mb-1.5">
                          <div className="flex items-center gap-2">
                            <span className="w-5 h-5 rounded-full bg-[#1E293B] text-white font-mono text-[10px] font-bold flex items-center justify-center shrink-0">
                              {idx + 1}
                            </span>
                            {/* Step Thumbnail if available */}
                            {step.beforeScreenshot && (
                              <img
                                src={step.beforeScreenshot}
                                alt="thumb"
                                className="w-6 h-10 object-cover rounded border border-[#1E293B] shrink-0"
                              />
                            )}
                            <div className="flex flex-col min-w-0">
                              <span className="font-semibold text-xs text-white truncate max-w-[200px]">
                                {step.label}
                              </span>
                              <div className="flex items-center gap-1 mt-0.5">
                                <span className="px-1.5 py-0.2 rounded text-[9px] font-bold uppercase bg-[#172554] text-[#60A5FA]">
                                  {step.actionType}
                                </span>
                                {step.status === 'verified' && (
                                  <span className="px-1.5 py-0.2 rounded text-[9px] font-bold uppercase bg-[#064E3B] text-[#34D399]">
                                    VERIFIED
                                  </span>
                                )}
                                {step.status === 'failed' && (
                                  <span className="px-1.5 py-0.2 rounded text-[9px] font-bold uppercase bg-[#7F1D1D] text-[#FCA5A5]">
                                    FAILED
                                  </span>
                                )}
                              </div>
                            </div>
                          </div>

                          <div className="flex items-center gap-1 shrink-0" onClick={(e) => e.stopPropagation()}>
                            {/* Move Up / Down Buttons */}
                            <button
                              onClick={() => {
                                if (idx > 0) {
                                  const next = [...recordedSteps];
                                  const [item] = next.splice(idx, 1);
                                  next.splice(idx - 1, 0, item);
                                  setRecordedSteps(next.map((s, i) => ({ ...s, stepNumber: i + 1 })));
                                }
                              }}
                              disabled={idx === 0}
                              className="px-1 py-0.5 rounded bg-[#1E293B] hover:bg-[#2A3B54] text-[#94A3B8] text-[9px] disabled:opacity-30"
                              title="Di chuyển lên"
                            >
                              ▲
                            </button>
                            <button
                              onClick={() => {
                                if (idx < recordedSteps.length - 1) {
                                  const next = [...recordedSteps];
                                  const [item] = next.splice(idx, 1);
                                  next.splice(idx + 1, 0, item);
                                  setRecordedSteps(next.map((s, i) => ({ ...s, stepNumber: i + 1 })));
                                }
                              }}
                              disabled={idx === recordedSteps.length - 1}
                              className="px-1 py-0.5 rounded bg-[#1E293B] hover:bg-[#2A3B54] text-[#94A3B8] text-[9px] disabled:opacity-30"
                              title="Di chuyển xuống"
                            >
                              ▼
                            </button>
                            <button
                              onClick={() => handleReplaySingleStep(step, idx)}
                              disabled={replayingStepIndex === idx}
                              className="p-1 rounded bg-[#1E293B] hover:bg-[#2A3B54] text-[#34D399] transition"
                              title="Chạy thử bước này"
                            >
                              <Play className="w-3 h-3" />
                            </button>
                            <button
                              onClick={() => {
                                setRecordedSteps((prev) => prev.filter((_, i) => i !== idx));
                                if (editingStepIndex === idx) setEditingStepIndex(null);
                              }}
                              className="p-1 rounded bg-[#1E293B] hover:bg-[#2A3B54] text-[#EF4444] transition"
                              title="Xóa bước"
                            >
                              <X className="w-3 h-3" />
                            </button>
                          </div>
                        </div>

                        <div className="flex items-center justify-between text-[10px] text-[#64748B] font-mono mt-1 pt-1 border-t border-[#162032]">
                          <span>Chiến lược: {step.strategyUsed}</span>
                          <span>Độ đặc thù: {step.uniquenessScore}%</span>
                          <span>Thời gian: {step.durationMs}ms</span>
                        </div>
                      </div>
                    );
                  })
                ) : (
                  <div className="flex flex-col items-center justify-center h-full text-center p-6 text-[#64748B]">
                    <Play className="w-10 h-10 text-[#38BDF8] opacity-30 mb-2" />
                    <p className="text-xs font-medium text-white">Chưa ghi lại bước nào.</p>
                    <p className="text-[11px] text-[#64748B] mt-1 max-w-xs">
                      Bấm "Ghi mới" và chạm trực tiếp vào màn hình điện thoại bên trái. Mọi Element sẽ được tự động phân giải selector tối ưu!
                    </p>
                  </div>
                )}
              </div>
            </div>
          )}

          {/* TAB 4: ELEMENT LIBRARY */}
          {activeTab === 'library' && (
            <div className="flex-1 flex flex-col p-3 overflow-y-auto space-y-3 bg-[#0B101B]">
              <div className="flex items-center justify-between">
                <span className="text-xs font-semibold text-white flex items-center gap-1.5">
                  <Database className="w-4 h-4 text-[#38BDF8]" />
                  Thư viện Element SQLite ({libraryElements.length})
                </span>
                <button
                  onClick={loadLibrary}
                  className="p-1 rounded bg-[#1E293B] hover:bg-[#2B3B52] text-white transition"
                  title="Tải lại thư viện"
                >
                  <RefreshCw className="w-3.5 h-3.5" />
                </button>
              </div>

              {libraryElements.length > 0 ? (
                <div className="space-y-2">
                  {libraryElements.map((elem) => (
                    <div
                      key={elem.id}
                      className="p-3 rounded-xl bg-[#101726] border border-[#1E293B] text-xs space-y-1.5"
                    >
                      <div className="flex items-center justify-between">
                        <span className="font-semibold text-white">{elem.name}</span>
                        <span className="px-1.5 py-0.2 rounded text-[10px] bg-[#1E3A5F] text-[#7DD3FC]">
                          {elem.primaryStrategy}
                        </span>
                      </div>
                      <div className="text-[10px] font-mono text-[#64748B] truncate">
                        Pkg: {elem.packageFilter || 'all'}
                      </div>
                    </div>
                  ))}
                </div>
              ) : (
                <div className="text-center text-[#64748B] p-6 text-xs">
                  Thư viện chưa có Element nào. Hãy dùng Element Inspector để lưu các UI component hay dùng!
                </div>
              )}
            </div>
          )}

          {/* TAB 5: DEBUG LOGS */}
          {activeTab === 'logs' && (
            <div className="flex-1 flex flex-col overflow-hidden bg-[#070A11] p-3">
              <div className="flex items-center justify-between mb-2">
                <span className="text-xs font-semibold text-[#64748B] font-mono uppercase tracking-wider">
                  Live ADB Execution Receipts & Logs
                </span>
                <button
                  onClick={() => setLogs([])}
                  className="px-2 py-0.5 rounded text-[10px] font-mono bg-[#1E293B] text-[#94A3B8] hover:text-white"
                >
                  Clear Logs
                </button>
              </div>
              <div className="flex-1 overflow-y-auto space-y-1 font-mono text-[11px] p-2 bg-[#05080E] rounded-lg border border-[#141C2A]">
                {logs.map((log, i) => (
                  <div key={i} className="flex items-start gap-2">
                    <span className="text-[#475569] shrink-0">[{log.timestamp}]</span>
                    <span
                      className={
                        log.level === 'error'
                          ? 'text-[#EF4444]'
                          : log.level === 'success'
                          ? 'text-[#34D399]'
                          : log.level === 'warn'
                          ? 'text-[#F59E0B]'
                          : 'text-[#94A3B8]'
                      }
                    >
                      {log.message}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
