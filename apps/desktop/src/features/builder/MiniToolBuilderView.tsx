import React, { useEffect, useMemo, useState } from 'react';
import {
  AlertTriangle,
  ArrowDown,
  ArrowUp,
  CheckCircle2,
  Eye,
  EyeOff,
  FolderGit2,
  Hammer,
  Image as ImageIcon,
  Layout,
  Monitor,
  PackageCheck,
  Palette,
  Play,
  Plus,
  RefreshCw,
  Save,
  ShieldCheck,
  Sparkles,
  Trash2,
} from 'lucide-react';
import { useProjectStore } from '../../stores/projectStore';
import { api } from '../../lib/api';
import type {
  BuildPreflightReport,
  BuildReport,
  MiniLayoutPreset,
  MiniToolUiSchema,
  RuntimeColumnBinding,
  UtilityToggleDef,
} from '../../types';

interface MiniToolBuilderViewProps {
  onNavigate?: (view: string) => void;
}

type PreviewRuntimeState = 'IDLE' | 'RUNNING' | 'PAUSED' | 'OFFLINE';

const ALL_SECTIONS: Array<{ id: string; label: string }> = [
  { id: 'header_bar', label: 'Header & Controls' },
  { id: 'device_table', label: 'Device Status Table' },
  { id: 'task_and_log_split', label: 'Tasks, Utilities & Log' },
  { id: 'task_checklist', label: 'Task Checklist & Settings' },
  { id: 'utilities_panel', label: 'Utility Switches' },
  { id: 'quick_log', label: 'Runtime Event Log' },
  { id: 'footer_controls', label: 'Footer Controls & Action Bar' },
  { id: 'status_footer', label: 'Status & Signature Footer' },
];

const buildDefaultSchema = (
  slug: string,
  title: string,
  version: string,
  detail: ReturnType<typeof useProjectStore.getState>['activeProjectDetail']
): MiniToolUiSchema => {
  const tasks = (detail?.tasks || []).map((t) => ({
    id: t.id,
    title: t.title,
    categoryId: t.categoryId,
    workflowId: t.workflowId,
    defaultEnabled: t.defaultEnabled,
    priority: t.priority,
    estimateSec: t.estimateSec,
    settingsSchema: (() => {
      try {
        return JSON.parse(t.settingsSchemaJson || '[]');
      } catch {
        return [];
      }
    })(),
  }));

  const categoriesMap = new Map<string, { id: string; shortTag: string; displayName: string; colorHex: string }>();
  for (const t of detail?.tasks || []) {
    if (!categoriesMap.has(t.categoryId)) {
      categoriesMap.set(t.categoryId, {
        id: t.categoryId,
        shortTag: t.categoryShortTag,
        displayName: t.categoryName,
        colorHex: t.categoryColorHex,
      });
    }
  }

  return {
    schemaVersion: 1,
    projectSlug: slug,
    windowTitle: title,
    versionBadge: `v${version}`,
    themeAccentHex: '#1878F5',
    headerBgHex: '#121E2A',
    layoutPreset: 'compact2_col',
    defaultWindowSize: [960, 620],
    sectionsOrder: ALL_SECTIONS.map((s) => s.id),
    hiddenComponents: [],
    deviceColumns: [
      {
        id: 'col-account',
        label: 'Account / Profile',
        runtimeVarKey: 'account_alias',
        fallbackDisplay: '—',
        minWindowWidth: 640,
        badgeColor: '#38BDF8',
      },
      {
        id: 'col-state',
        label: 'Active Task',
        runtimeVarKey: 'current_task',
        fallbackDisplay: 'Standby',
        minWindowWidth: 780,
        badgeColor: '#3BC38D',
      },
    ],
    taskCategories: Array.from(categoriesMap.values()),
    tasks,
    utilities: [
      {
        id: 'util.auto_reconnect',
        label: 'Tự động kết nối lại ADB khi mất tín hiệu',
        defaultEnabled: true,
        requiredModule: 'mod.adb_core',
        settingsSchema: [],
      },
      {
        id: 'util.save_checkpoint',
        label: 'Lưu Checkpoint SQLite WAL sau mỗi vòng',
        defaultEnabled: true,
        requiredModule: null,
        settingsSchema: [],
      },
    ],
    initialRuntimeVars: {},
  };
};

export const MiniToolBuilderView: React.FC<MiniToolBuilderViewProps> = ({ onNavigate }) => {
  const { activeProjectId, activeProjectDetail, refreshActiveProject } = useProjectStore();
  const project = activeProjectDetail?.project ?? null;
  const releases = activeProjectDetail?.releases || [];
  const tasks = activeProjectDetail?.tasks || [];

  const [schema, setSchema] = useState<MiniToolUiSchema | null>(null);
  const [exeName, setExeName] = useState('');
  const [version, setVersion] = useState('');
  const [bundleAdb, setBundleAdb] = useState(false);
  const [customOutputDir, setCustomOutputDir] = useState('');

  const [previewState, setPreviewState] = useState<PreviewRuntimeState>('IDLE');
  const [preflight, setPreflight] = useState<BuildPreflightReport | null>(null);
  const [lastBuildReport, setLastBuildReport] = useState<BuildReport | null>(null);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [notice, setNotice] = useState<{ kind: 'ok' | 'err'; text: string } | null>(null);

  useEffect(() => {
    if (!project) {
      setSchema(null);
      return;
    }
    setExeName(project.exeName);
    setVersion(project.version);
    setBundleAdb(project.bundleAdb);
    setPreflight(null);
    setLastBuildReport(null);

    if (activeProjectDetail?.uiSchema) {
      setSchema(activeProjectDetail.uiSchema);
    } else {
      setSchema(
        buildDefaultSchema(project.slug, project.windowTitle, project.version, activeProjectDetail)
      );
    }
  }, [activeProjectId, activeProjectDetail?.project.updatedAt]);

  const syncedSchema = useMemo(() => {
    if (!schema || !project) return null;
    const catMap = new Map<
      string,
      { id: string; shortTag: string; displayName: string; colorHex: string }
    >();
    const syncedTasks = tasks.map((t) => {
      if (!catMap.has(t.categoryId)) {
        catMap.set(t.categoryId, {
          id: t.categoryId,
          shortTag: t.categoryShortTag,
          displayName: t.categoryName,
          colorHex: t.categoryColorHex,
        });
      }
      return {
        id: t.id,
        title: t.title,
        categoryId: t.categoryId,
        workflowId: t.workflowId,
        defaultEnabled: t.defaultEnabled,
        priority: t.priority,
        estimateSec: t.estimateSec,
        settingsSchema: (() => {
          try {
            return JSON.parse(t.settingsSchemaJson || '[]');
          } catch {
            return [];
          }
        })(),
      };
    });

    return {
      ...schema,
      projectSlug: project.slug,
      tasks: syncedTasks.length > 0 ? syncedTasks : schema.tasks,
      taskCategories:
        catMap.size > 0 ? Array.from(catMap.values()) : schema.taskCategories,
    };
  }, [schema, project, tasks]);

  const saveProjectAndSchema = async (): Promise<MiniToolUiSchema | null> => {
    if (!activeProjectId || !project || !syncedSchema) return null;
    const updatedSchema: MiniToolUiSchema = {
      ...syncedSchema,
      versionBadge: syncedSchema.versionBadge || `v${version || project.version}`,
    };
    await api.studioUpdateProject({
      id: activeProjectId,
      exeName: exeName.trim() || project.exeName,
      windowTitle: updatedSchema.windowTitle.trim() || project.windowTitle,
      version: version.trim() || project.version,
      bundleAdb,
    });
    await api.studioSaveUiSchema(activeProjectId, updatedSchema);
    setSchema(updatedSchema);
    await refreshActiveProject();
    return updatedSchema;
  };

  const handleSaveUiSchema = async () => {
    setBusyAction('save');
    setNotice(null);
    try {
      const saved = await saveProjectAndSchema();
      setNotice({
        kind: 'ok',
        text: `Đã lưu UI Schema (${saved?.layoutPreset}) & Metadata vào SQLite.`,
      });
    } catch (err: any) {
      setNotice({ kind: 'err', text: `Lưu UI Schema thất bại: ${err?.message || String(err)}` });
    } finally {
      setBusyAction(null);
    }
  };

  const handleEnsureDefaultIcon = async () => {
    if (!activeProjectId || !schema) return;
    setBusyAction('icon');
    setNotice(null);
    try {
      const iconPath = await api.studioEnsureProjectIcon({
        projectId: activeProjectId,
        accentHex: schema.themeAccentHex,
        overwrite: true,
      });
      setNotice({ kind: 'ok', text: `Đã tạo Windows Icon (.ico) hợp lệ tại: ${iconPath}` });
      const report = await api.studioBuilderPreflight(activeProjectId);
      setPreflight(report);
    } catch (err: any) {
      setNotice({ kind: 'err', text: `Tạo Icon thất bại: ${err?.message || String(err)}` });
    } finally {
      setBusyAction(null);
    }
  };

  const handleUploadIcoFile = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file || !activeProjectId) return;
    setBusyAction('icon');
    setNotice(null);
    try {
      const buf = await file.arrayBuffer();
      const bytes = new Uint8Array(buf);
      let binary = '';
      for (let i = 0; i < bytes.length; i++) {
        binary += String.fromCharCode(bytes[i]);
      }
      const b64 = btoa(binary);
      const iconPath = await api.studioEnsureProjectIcon({
        projectId: activeProjectId,
        iconBase64: b64,
        overwrite: true,
      });
      setNotice({ kind: 'ok', text: `Đã nạp file .ico (${file.name}) vào ${iconPath}` });
      const report = await api.studioBuilderPreflight(activeProjectId);
      setPreflight(report);
    } catch (err: any) {
      setNotice({ kind: 'err', text: `Upload .ico thất bại: ${err?.message || String(err)}` });
    } finally {
      setBusyAction(null);
    }
  };

  const handleRunPreflight = async () => {
    if (!activeProjectId) return;
    setBusyAction('preflight');
    setNotice(null);
    try {
      await saveProjectAndSchema();
      const report = await api.studioBuilderPreflight(activeProjectId);
      setPreflight(report);
      setNotice({
        kind: report.ready ? 'ok' : 'err',
        text: report.ready
          ? `Preflight PASS — Sẵn sàng đóng gói ${report.exeName} (${report.workflowCount} workflows).`
          : `Preflight chưa đạt (${report.issues.length} lỗi): ${report.issues.join(' · ')}`,
      });
    } catch (err: any) {
      setNotice({ kind: 'err', text: `Preflight lỗi: ${err?.message || String(err)}` });
    } finally {
      setBusyAction(null);
    }
  };

  const handleBuildExportRelease = async () => {
    if (!activeProjectId || !schema) return;
    setBusyAction('export');
    setNotice(null);
    try {
      await saveProjectAndSchema();
      await api.studioEnsureProjectIcon({
        projectId: activeProjectId,
        accentHex: schema.themeAccentHex,
        overwrite: false,
      });
      const report = await api.studioBuilderPreflight(activeProjectId);
      setPreflight(report);
      if (!report.ready) {
        setNotice({
          kind: 'err',
          text: `Không thể xuất Release vì Preflight lỗi: ${report.issues.join(' · ')}`,
        });
        return;
      }
      const res = await api.studioBuilderExportRelease(
        activeProjectId,
        customOutputDir.trim() || undefined
      );
      setLastBuildReport(res.buildReport);
      await refreshActiveProject();
      setNotice({
        kind: 'ok',
        text: `Đã xuất & ký Ed25519 thành công: ${res.release.exePath} (Key ${res.release.keyId})`,
      });
    } catch (err: any) {
      setNotice({ kind: 'err', text: `Build EXE thất bại: ${err?.message || String(err)}` });
    } finally {
      setBusyAction(null);
    }
  };

  const moveSection = (index: number, delta: -1 | 1) => {
    if (!schema) return;
    const next = [...schema.sectionsOrder];
    const target = index + delta;
    if (target < 0 || target >= next.length) return;
    const tmp = next[target];
    next[target] = next[index];
    next[index] = tmp;
    setSchema({ ...schema, sectionsOrder: next });
  };

  const toggleSectionHidden = (id: string) => {
    if (!schema) return;
    const hidden = schema.hiddenComponents.includes(id)
      ? schema.hiddenComponents.filter((s) => s !== id)
      : [...schema.hiddenComponents, id];
    setSchema({ ...schema, hiddenComponents: hidden });
  };

  const addDeviceColumn = () => {
    if (!schema) return;
    const nextCol: RuntimeColumnBinding = {
      id: `col-${Date.now().toString(36)}`,
      label: `Custom Col ${schema.deviceColumns.length + 1}`,
      runtimeVarKey: `var_${schema.deviceColumns.length + 1}`,
      fallbackDisplay: '0',
      minWindowWidth: 720,
      badgeColor: schema.themeAccentHex,
    };
    setSchema({ ...schema, deviceColumns: [...schema.deviceColumns, nextCol] });
  };

  const addUtilityToggle = () => {
    if (!schema) return;
    const nextUtil: UtilityToggleDef = {
      id: `util.custom_${Date.now().toString(36)}`,
      label: `Tiện ích mới #${schema.utilities.length + 1}`,
      defaultEnabled: true,
      requiredModule: null,
      settingsSchema: [],
    };
    setSchema({ ...schema, utilities: [...schema.utilities, nextUtil] });
  };

  if (!project || !syncedSchema) {
    return (
      <div className="flex-1 flex items-center justify-center bg-[#101722] p-6">
        <div className="fp-panel p-6 max-w-md text-center space-y-3">
          <PackageCheck size={28} className="mx-auto text-[#B291F7]" />
          <h2 className="text-base font-bold text-[#EDF3FC]">Chưa chọn Game Project</h2>
          <p className="text-xs text-[#74869D]">
            Chọn một Game Project từ Header để thiết kế giao diện Mini Tool WYSIWYG và xuất file
            EXE độc lập ký Ed25519.
          </p>
          {onNavigate && (
            <button
              onClick={() => onNavigate('projects')}
              className="px-3.5 py-1.5 rounded bg-[#1878F5] text-white text-xs font-semibold cursor-pointer inline-flex items-center gap-1.5"
            >
              <FolderGit2 size={13} />
              <span>Mở Projects &amp; Dashboard</span>
            </button>
          )}
        </div>
      </div>
    );
  }

  const visibleSections = syncedSchema.sectionsOrder.filter(
    (s) => !syncedSchema.hiddenComponents.includes(s)
  );
  const showHeader = !syncedSchema.hiddenComponents.includes('header_bar');
  const showDeviceTable = visibleSections.includes('device_table');
  const showTasks =
    visibleSections.includes('task_checklist') || visibleSections.includes('task_and_log_split');
  const showUtilities =
    visibleSections.includes('utilities_panel') || visibleSections.includes('task_and_log_split');
  const showLog =
    visibleSections.includes('quick_log') || visibleSections.includes('task_and_log_split');
  const showFooter =
    visibleSections.includes('status_footer') || visibleSections.includes('footer_controls');
  const previewCompact2Col = syncedSchema.layoutPreset === 'compact2_col';
  const previewExpanded = syncedSchema.layoutPreset === 'expanded';

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden bg-[#101722] text-[#EDF3FC]">
      {/* Top Toolbar */}
      <div className="px-5 py-3 border-b border-[#2A3647] bg-[#141C28] flex flex-wrap items-center justify-between gap-3 shrink-0">
        <div>
          <div className="flex items-center gap-2">
            <span className="px-2 py-0.5 rounded text-[10px] font-mono font-bold bg-[#173355] text-[#B291F7] border border-[#2A3647]">
              WYSIWYG MINI TOOL BUILDER
            </span>
            <span className="text-xs font-mono text-[#A9B8CC]">
              {project.slug} · {syncedSchema.layoutPreset} · {syncedSchema.tasks.length} tasks
            </span>
          </div>
          <h1 className="text-base font-bold text-[#EDF3FC] mt-0.5">
            Thiết kế Giao diện &amp; Đóng gói {exeName || project.exeName}
          </h1>
        </div>

        <div className="flex items-center gap-2 flex-wrap">
          <button
            onClick={handleSaveUiSchema}
            disabled={busyAction !== null}
            className="px-3 py-1.5 rounded-md bg-[#16202F] hover:bg-[#202B3B] border border-[#2A3647] text-xs text-[#EDF3FC] font-semibold flex items-center gap-1.5 cursor-pointer"
          >
            <Save size={13} className="text-[#4B9CFF]" />
            <span>Lưu Schema</span>
          </button>

          <button
            onClick={handleRunPreflight}
            disabled={busyAction !== null}
            className="px-3 py-1.5 rounded-md bg-[#16202F] hover:bg-[#202B3B] border border-[#2A3647] text-xs text-[#38BDF8] font-semibold flex items-center gap-1.5 cursor-pointer"
          >
            <RefreshCw size={13} />
            <span>Kiểm tra Preflight</span>
          </button>

          <button
            onClick={handleBuildExportRelease}
            disabled={busyAction !== null}
            className="px-4 py-1.5 rounded-md bg-[#1878F5] hover:bg-[#2F8CFF] disabled:opacity-50 text-white text-xs font-bold flex items-center gap-1.5 cursor-pointer shadow-sm"
          >
            <Hammer size={13} />
            <span>{busyAction === 'export' ? 'Đang đóng gói...' : 'Build & Xuất EXE'}</span>
          </button>
        </div>
      </div>

      {notice && (
        <div
          className={`mx-5 mt-3 px-3.5 py-2 rounded-lg border text-xs flex items-center gap-2 shrink-0 ${
            notice.kind === 'ok'
              ? 'bg-[#133528] border-[#3BC38D]/40 text-[#3BC38D]'
              : 'bg-[#381E24] border-[#F27883]/40 text-[#F27883]'
          }`}
        >
          {notice.kind === 'ok' ? <CheckCircle2 size={14} /> : <AlertTriangle size={14} />}
          <span className="truncate">{notice.text}</span>
        </div>
      )}

      {/* 3-Area Body: Left Palette | Center Live Preview | Right Preflight & Releases */}
      <div className="flex-1 grid grid-cols-12 gap-4 p-5 overflow-hidden min-h-0">
        {/* Left Column (4 cols): Palette & Schema Controls */}
        <div className="col-span-12 xl:col-span-4 fp-panel p-4 overflow-y-auto space-y-4 text-xs">
          {/* Branding & Packaging Identity */}
          <div className="space-y-2.5">
            <div className="flex items-center gap-1.5 font-bold text-[#EDF3FC] border-b border-[#2A3647] pb-2">
              <Palette size={14} className="text-[#B291F7]" />
              <span>Branding &amp; Packaging EXE</span>
            </div>

            <div className="grid grid-cols-2 gap-2">
              <div className="col-span-2">
                <label className="block text-[#A9B8CC] mb-1 text-[11px]">Tiêu đề cửa sổ</label>
                <input
                  type="text"
                  value={syncedSchema.windowTitle}
                  onChange={(e) => setSchema({ ...syncedSchema, windowTitle: e.target.value })}
                  className="w-full bg-[#101722] border border-[#2A3647] rounded px-2.5 py-1.5 text-[#EDF3FC] outline-none"
                />
              </div>
              <div>
                <label className="block text-[#A9B8CC] mb-1 text-[11px]">Tên file EXE</label>
                <input
                  type="text"
                  value={exeName}
                  onChange={(e) => setExeName(e.target.value)}
                  className="w-full bg-[#101722] border border-[#2A3647] rounded px-2.5 py-1.5 text-[#EDF3FC] font-mono outline-none"
                />
              </div>
              <div>
                <label className="block text-[#A9B8CC] mb-1 text-[11px]">Phiên bản</label>
                <input
                  type="text"
                  value={version}
                  onChange={(e) => {
                    setVersion(e.target.value);
                    setSchema({ ...syncedSchema, versionBadge: `v${e.target.value}` });
                  }}
                  className="w-full bg-[#101722] border border-[#2A3647] rounded px-2.5 py-1.5 text-[#EDF3FC] font-mono outline-none"
                />
              </div>
              <div className="min-w-0">
                <label className="block text-[#A9B8CC] mb-1 text-[11px]">Màu chủ đạo</label>
                <div className="flex items-center gap-1.5 min-w-0">
                  <input
                    type="color"
                    value={syncedSchema.themeAccentHex}
                    onChange={(e) => setSchema({ ...syncedSchema, themeAccentHex: e.target.value })}
                    className="w-7 h-7 rounded bg-transparent border border-[#2A3647] cursor-pointer shrink-0"
                  />
                  <input
                    type="text"
                    value={syncedSchema.themeAccentHex}
                    onChange={(e) => setSchema({ ...syncedSchema, themeAccentHex: e.target.value })}
                    className="w-full min-w-0 bg-[#101722] border border-[#2A3647] rounded px-2 py-1 text-[#EDF3FC] font-mono outline-none"
                  />
                </div>
              </div>
              <div className="min-w-0">
                <label className="block text-[#A9B8CC] mb-1 text-[11px]">Màu Header</label>
                <div className="flex items-center gap-1.5 min-w-0">
                  <input
                    type="color"
                    value={syncedSchema.headerBgHex}
                    onChange={(e) => setSchema({ ...syncedSchema, headerBgHex: e.target.value })}
                    className="w-7 h-7 rounded bg-transparent border border-[#2A3647] cursor-pointer shrink-0"
                  />
                  <input
                    type="text"
                    value={syncedSchema.headerBgHex}
                    onChange={(e) => setSchema({ ...syncedSchema, headerBgHex: e.target.value })}
                    className="w-full min-w-0 bg-[#101722] border border-[#2A3647] rounded px-2 py-1 text-[#EDF3FC] font-mono outline-none"
                  />
                </div>
              </div>
            </div>

            <div className="flex items-center gap-2 pt-1 flex-wrap">
              <button
                type="button"
                onClick={handleEnsureDefaultIcon}
                disabled={busyAction !== null}
                className="px-2.5 py-1.5 rounded bg-[#101722] hover:bg-[#192434] border border-[#2A3647] text-[11px] text-[#38BDF8] flex items-center gap-1 cursor-pointer"
              >
                <Sparkles size={12} />
                <span>Tạo Icon .ico theo Accent</span>
              </button>
              <label className="px-2.5 py-1.5 rounded bg-[#101722] hover:bg-[#192434] border border-[#2A3647] text-[11px] text-[#A9B8CC] flex items-center gap-1 cursor-pointer">
                <ImageIcon size={12} />
                <span>Nạp .ico riêng</span>
                <input type="file" accept=".ico" className="hidden" onChange={handleUploadIcoFile} />
              </label>
            </div>

            <label className="flex items-center gap-2 text-[11px] text-[#A9B8CC] cursor-pointer">
              <input
                type="checkbox"
                checked={bundleAdb}
                onChange={(e) => setBundleAdb(e.target.checked)}
                className="accent-[#1878F5]"
              />
              <span>Đóng gói kèm bin/adb.exe vào thư mục phát hành</span>
            </label>
          </div>

          {/* Layout & Sections Order */}
          <div className="space-y-2.5 pt-2 border-t border-[#2A3647]">
            <div className="flex items-center gap-1.5 font-bold text-[#EDF3FC]">
              <Layout size={14} className="text-[#4B9CFF]" />
              <span>Bố cục &amp; Thứ tự Thành phần</span>
            </div>

            <div className="grid grid-cols-3 gap-1.5">
              {(['compact1_col', 'compact2_col', 'expanded'] as MiniLayoutPreset[]).map((p) => (
                <button
                  key={p}
                  type="button"
                  onClick={() => setSchema({ ...syncedSchema, layoutPreset: p })}
                  className={`py-1.5 px-2 rounded text-[11px] font-mono border cursor-pointer ${
                    syncedSchema.layoutPreset === p
                      ? 'bg-[#1878F5] text-white border-[#1878F5]'
                      : 'bg-[#101722] text-[#A9B8CC] border-[#2A3647]'
                  }`}
                >
                  {p}
                </button>
              ))}
            </div>

            <div className="space-y-1.5">
              {syncedSchema.sectionsOrder.map((secId, idx) => {
                const label = ALL_SECTIONS.find((s) => s.id === secId)?.label || secId;
                const hidden = syncedSchema.hiddenComponents.includes(secId);
                return (
                  <div
                    key={secId}
                    className="px-2.5 py-1.5 rounded bg-[#101722] border border-[#2A3647] flex items-center justify-between"
                  >
                    <span className={hidden ? 'text-[#5A6D85] line-through' : 'text-[#EDF3FC]'}>
                      {label}
                    </span>
                    <div className="flex items-center gap-1">
                      <button
                        type="button"
                        onClick={() => moveSection(idx, -1)}
                        className="p-1 text-[#74869D] hover:text-[#EDF3FC] cursor-pointer"
                      >
                        <ArrowUp size={12} />
                      </button>
                      <button
                        type="button"
                        onClick={() => moveSection(idx, 1)}
                        className="p-1 text-[#74869D] hover:text-[#EDF3FC] cursor-pointer"
                      >
                        <ArrowDown size={12} />
                      </button>
                      <button
                        type="button"
                        onClick={() => toggleSectionHidden(secId)}
                        className="p-1 text-[#A9B8CC] hover:text-[#38BDF8] cursor-pointer"
                      >
                        {hidden ? <EyeOff size={12} /> : <Eye size={12} />}
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>

          {/* Device Table Columns */}
          <div className="space-y-2 pt-2 border-t border-[#2A3647]">
            <div className="flex items-center justify-between">
              <span className="font-bold text-[#EDF3FC]">
                Cột Bảng Thiết bị ({syncedSchema.deviceColumns.length})
              </span>
              <button
                type="button"
                onClick={addDeviceColumn}
                className="text-[11px] text-[#38BDF8] flex items-center gap-1 cursor-pointer"
              >
                <Plus size={12} />
                <span>Thêm cột</span>
              </button>
            </div>
            {syncedSchema.deviceColumns.map((col, idx) => (
              <div
                key={col.id}
                className="p-2 rounded bg-[#101722] border border-[#2A3647] flex items-center gap-2"
              >
                <input
                  type="text"
                  value={col.label}
                  onChange={(e) => {
                    const next = [...syncedSchema.deviceColumns];
                    next[idx] = { ...col, label: e.target.value };
                    setSchema({ ...syncedSchema, deviceColumns: next });
                  }}
                  className="flex-1 bg-transparent border-b border-[#2A3647] px-1 py-0.5 text-[#EDF3FC] outline-none"
                />
                <input
                  type="text"
                  value={col.runtimeVarKey}
                  onChange={(e) => {
                    const next = [...syncedSchema.deviceColumns];
                    next[idx] = { ...col, runtimeVarKey: e.target.value };
                    setSchema({ ...syncedSchema, deviceColumns: next });
                  }}
                  className="w-24 bg-transparent border-b border-[#2A3647] px-1 py-0.5 text-[#38BDF8] font-mono outline-none"
                />
                <button
                  type="button"
                  onClick={() =>
                    setSchema({
                      ...syncedSchema,
                      deviceColumns: syncedSchema.deviceColumns.filter((c) => c.id !== col.id),
                    })
                  }
                  className="text-[#74869D] hover:text-[#F27883] cursor-pointer"
                >
                  <Trash2 size={12} />
                </button>
              </div>
            ))}
          </div>

          {/* Utility Switches */}
          <div className="space-y-2 pt-2 border-t border-[#2A3647]">
            <div className="flex items-center justify-between">
              <span className="font-bold text-[#EDF3FC]">
                Tiện ích Bật/Tắt ({syncedSchema.utilities.length})
              </span>
              <button
                type="button"
                onClick={addUtilityToggle}
                className="text-[11px] text-[#38BDF8] flex items-center gap-1 cursor-pointer"
              >
                <Plus size={12} />
                <span>Thêm tiện ích</span>
              </button>
            </div>
            {syncedSchema.utilities.map((u, idx) => (
              <div
                key={u.id}
                className="p-2 rounded bg-[#101722] border border-[#2A3647] flex items-center gap-2"
              >
                <input
                  type="checkbox"
                  checked={u.defaultEnabled}
                  onChange={(e) => {
                    const next = [...syncedSchema.utilities];
                    next[idx] = { ...u, defaultEnabled: e.target.checked };
                    setSchema({ ...syncedSchema, utilities: next });
                  }}
                  className="accent-[#3BC38D]"
                />
                <input
                  type="text"
                  value={u.label}
                  onChange={(e) => {
                    const next = [...syncedSchema.utilities];
                    next[idx] = { ...u, label: e.target.value };
                    setSchema({ ...syncedSchema, utilities: next });
                  }}
                  className="flex-1 bg-transparent text-[#EDF3FC] outline-none"
                />
                <button
                  type="button"
                  onClick={() =>
                    setSchema({
                      ...syncedSchema,
                      utilities: syncedSchema.utilities.filter((x) => x.id !== u.id),
                    })
                  }
                  className="text-[#74869D] hover:text-[#F27883] cursor-pointer"
                >
                  <Trash2 size={12} />
                </button>
              </div>
            ))}
          </div>
        </div>

        {/* Center Column (5 cols): WYSIWYG Live Preview Frame */}
        <div className="col-span-12 xl:col-span-5 fp-panel p-4 flex flex-col overflow-hidden">
          <div className="flex items-center justify-between pb-3 border-b border-[#2A3647] shrink-0">
            <div className="flex items-center gap-2">
              <Monitor size={14} className="text-[#38BDF8]" />
              <span className="text-xs font-bold text-[#EDF3FC]">
                WYSIWYG Mini Tool Live Preview
              </span>
            </div>
            <div className="flex items-center gap-1 bg-[#101722] p-0.5 rounded border border-[#2A3647]">
              {(['IDLE', 'RUNNING', 'PAUSED', 'OFFLINE'] as PreviewRuntimeState[]).map((st) => (
                <button
                  key={st}
                  type="button"
                  onClick={() => setPreviewState(st)}
                  className={`px-2 py-0.5 rounded text-[10px] font-mono font-bold cursor-pointer ${
                    previewState === st
                      ? 'bg-[#1878F5] text-white'
                      : 'text-[#74869D] hover:text-[#EDF3FC]'
                  }`}
                >
                  {st}
                </button>
              ))}
            </div>
          </div>

          {/* Simulated Standalone Mini Tool Window */}
          <div className="flex-1 mt-3 rounded-xl border border-[#2A3647] bg-[#0D131D] overflow-y-auto flex flex-col shadow-inner">
            {/* Window Header */}
            {showHeader && (
              <div
                className="px-4 py-3 border-b border-[#2A3647] flex items-center justify-between gap-2"
                style={{ backgroundColor: syncedSchema.headerBgHex }}
              >
                <div className="flex items-center gap-2 min-w-0">
                  <div
                    className="w-3 h-3 rounded-sm shrink-0"
                    style={{ backgroundColor: syncedSchema.themeAccentHex }}
                  />
                  <span className="text-xs font-bold text-white truncate">
                    {syncedSchema.windowTitle}
                  </span>
                  <span className="px-1.5 py-0.5 rounded text-[10px] font-mono bg-black/30 text-[#CBD5E1]">
                    {syncedSchema.versionBadge}
                  </span>
                </div>

                <div className="flex items-center gap-1.5 shrink-0">
                  <span className="px-2 py-0.5 rounded text-[10px] font-mono bg-black/40 text-[#94A3B8]">
                    STATE: {previewState}
                  </span>
                  <span
                    className="px-2.5 py-1 rounded text-[10px] font-bold text-white inline-flex items-center gap-1"
                    style={{ backgroundColor: syncedSchema.themeAccentHex }}
                  >
                    <Play size={10} />
                    <span>{previewState === 'RUNNING' ? 'Pause' : 'Start Tool'}</span>
                  </span>
                </div>
              </div>
            )}

            <div className="p-3.5 space-y-3 flex-1">
              {/* Top Device Table Preview */}
              {showDeviceTable && (
                <div className="rounded-lg border border-[#2A3647] bg-[#141D2B] overflow-hidden text-[11px]">
                  <div className="px-3 py-1.5 bg-[#192434] border-b border-[#2A3647] font-bold text-[#A9B8CC]">
                    Thiết bị Mục tiêu (Preview {previewState})
                  </div>
                  <table className="w-full text-left">
                    <thead className="text-[10px] text-[#74869D] border-b border-[#2A3647]">
                      <tr>
                        <th className="p-2">Device Serial</th>
                        {syncedSchema.deviceColumns.map((c) => (
                          <th key={c.id} className="p-2">
                            {c.label}
                          </th>
                        ))}
                      </tr>
                    </thead>
                    <tbody>
                      <tr className="text-[#EDF3FC]">
                        <td className="p-2 font-mono">
                          {previewState === 'OFFLINE' ? '0 devices' : 'preview-device-01'}
                        </td>
                        {syncedSchema.deviceColumns.map((c) => (
                          <td key={c.id} className="p-2 font-mono text-[#A9B8CC]">
                            {previewState === 'RUNNING'
                              ? `running:${c.runtimeVarKey}`
                              : c.fallbackDisplay}
                          </td>
                        ))}
                      </tr>
                    </tbody>
                  </table>
                </div>
              )}

              {/* Layout Grid for Tasks + Utilities */}
              <div
                className={`grid gap-3 ${
                  previewCompact2Col || previewExpanded ? 'grid-cols-2' : 'grid-cols-1'
                }`}
              >
                {showTasks && (
                  <div className="rounded-lg border border-[#2A3647] bg-[#141D2B] p-3 space-y-2">
                    <div className="text-[11px] font-bold text-[#EDF3FC]">
                      Nhiệm vụ ({syncedSchema.tasks.length})
                    </div>
                    {syncedSchema.tasks.length === 0 ? (
                      <div className="text-[10px] text-[#74869D]">
                        Chưa có Task — thêm Task tại mục Tasks &amp; Modules.
                      </div>
                    ) : (
                      <div className="space-y-1.5">
                        {syncedSchema.tasks.map((t) => (
                          <div
                            key={t.id}
                            className="flex items-center justify-between text-[11px] p-1.5 rounded bg-[#0D131D] border border-[#2A3647]"
                          >
                            <span className="truncate text-[#EDF3FC]">{t.title}</span>
                            <span
                              className="px-1.5 py-0.2 rounded text-[9px] font-mono shrink-0"
                              style={{
                                backgroundColor: `${syncedSchema.themeAccentHex}25`,
                                color: syncedSchema.themeAccentHex,
                              }}
                            >
                              {t.defaultEnabled ? 'ON' : 'OFF'}
                            </span>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                )}

                {showUtilities && (
                  <div className="rounded-lg border border-[#2A3647] bg-[#141D2B] p-3 space-y-2">
                    <div className="text-[11px] font-bold text-[#EDF3FC]">
                      Tiện ích ({syncedSchema.utilities.length})
                    </div>
                    <div className="space-y-1.5">
                      {syncedSchema.utilities.length === 0 ? (
                        <div className="text-[10px] text-[#74869D]">Chưa có tiện ích.</div>
                      ) : (
                        syncedSchema.utilities.map((u) => (
                          <div
                            key={u.id}
                            className="flex items-center justify-between text-[11px] text-[#A9B8CC]"
                          >
                            <span className="truncate">{u.label}</span>
                            <span className="font-mono text-[10px] text-[#3BC38D]">
                              {u.defaultEnabled ? 'ON' : 'OFF'}
                            </span>
                          </div>
                        ))
                      )}
                    </div>
                  </div>
                )}
              </div>

              {showLog && (
                <div className="rounded-lg border border-[#2A3647] bg-[#141D2B] p-2.5 text-[10px] font-mono text-[#74869D]">
                  [PREVIEW {previewState}] Runtime logger ready · No Studio process required
                </div>
              )}
            </div>

            {showFooter && (
              <div className="px-4 py-2 border-t border-[#2A3647] bg-[#141D2B] text-[10px] font-mono text-[#74869D] flex items-center justify-between">
                <span>Signed Bundle Ready · SQLite Isolated</span>
                <span>Preset: {syncedSchema.layoutPreset}</span>
              </div>
            )}
          </div>
        </div>

        {/* Right Column (3 cols): Preflight, Build Output & Releases */}
        <div className="col-span-12 xl:col-span-3 fp-panel p-4 overflow-y-auto space-y-4 text-xs">
          <div className="space-y-2">
            <div className="font-bold text-[#EDF3FC] border-b border-[#2A3647] pb-2">
              Thư mục Xuất Release (Tuỳ chọn)
            </div>
            <input
              type="text"
              value={customOutputDir}
              onChange={(e) => setCustomOutputDir(e.target.value)}
              placeholder={`Mặc định: projects/${project.slug}/releases/v${version || project.version}`}
              className="w-full bg-[#101722] border border-[#2A3647] rounded px-2.5 py-1.5 text-[#EDF3FC] font-mono text-[11px] outline-none"
            />
          </div>

          {/* Preflight Check Result */}
          <div className="space-y-2 pt-2 border-t border-[#2A3647]">
            <div className="flex items-center justify-between font-bold text-[#EDF3FC]">
              <span>Preflight Checks</span>
              {preflight && (
                <span className={preflight.ready ? 'fp-badge-live' : 'fp-badge-error'}>
                  {preflight.ready ? 'READY' : 'BLOCKED'}
                </span>
              )}
            </div>
            {!preflight ? (
              <p className="text-[11px] text-[#74869D]">
                Nhấn &quot;Kiểm tra Preflight&quot; hoặc &quot;Build &amp; Xuất EXE&quot; để kiểm
                tra manifest, workflow, icon và producer keystore.
              </p>
            ) : (
              <div className="space-y-1.5">
                {preflight.checks.map((c) => (
                  <div
                    key={c.stepNo}
                    className="p-2 rounded bg-[#101722] border border-[#2A3647] text-[11px]"
                  >
                    <div className="flex items-center justify-between font-semibold">
                      <span className="text-[#EDF3FC]">
                        #{c.stepNo} {c.name}
                      </span>
                      <span
                        className={
                          c.status === 'PASS' ? 'text-[#3BC38D] font-mono' : 'text-[#F27883] font-mono'
                        }
                      >
                        {c.status}
                      </span>
                    </div>
                    <div className="text-[10px] text-[#74869D] mt-0.5 break-all">{c.detail}</div>
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* Last Build Report */}
          {lastBuildReport && (
            <div className="space-y-1.5 pt-2 border-t border-[#2A3647] text-[11px]">
              <div className="font-bold text-[#3BC38D]">Build Report Mới nhất</div>
              <div className="p-2.5 rounded bg-[#101722] border border-[#3BC38D]/40 font-mono space-y-1 text-[10px]">
                <div className="text-[#EDF3FC] break-all">{lastBuildReport.toolExePath}</div>
                <div className="text-[#A9B8CC]">
                  SHA: {lastBuildReport.toolExeSha256.slice(0, 16)}...
                </div>
                <div className="text-[#A9B8CC]">
                  Key: {lastBuildReport.keyId} · PubKey PE Verified:{' '}
                  {String(lastBuildReport.peEmbeddedPubkeyVerified)}
                </div>
                <div className="text-[#74869D]">
                  Total: {lastBuildReport.totalDurationMs} ms · Icon Layers:{' '}
                  {lastBuildReport.peIconLayersInjected}
                </div>
              </div>
            </div>
          )}

          {/* Signed Releases History */}
          <div className="space-y-2 pt-2 border-t border-[#2A3647]">
            <div className="flex items-center justify-between">
              <span className="font-bold text-[#EDF3FC]">
                Lịch sử Phát hành ({releases.length})
              </span>
              <ShieldCheck size={14} className="text-[#3BC38D]" />
            </div>
            {releases.length === 0 ? (
              <p className="text-[11px] text-[#74869D]">
                Chưa có release nào trong <code>project_releases</code>.
              </p>
            ) : (
              <div className="space-y-1.5">
                {releases.map((rel) => (
                  <div
                    key={rel.id}
                    className="p-2.5 rounded bg-[#101722] border border-[#2A3647] space-y-1 text-[11px]"
                  >
                    <div className="font-bold text-[#EDF3FC] truncate">
                      v{rel.version} · {rel.exePath.split(/[\\/]/).pop()}
                    </div>
                    <div className="font-mono text-[10px] text-[#74869D] break-all">
                      {rel.exePath}
                    </div>
                    <div className="font-mono text-[10px] text-[#3BC38D]">
                      Key {rel.keyId} · {rel.exeSizeBytes.toLocaleString()} bytes
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
