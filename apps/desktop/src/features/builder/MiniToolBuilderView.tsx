import React, { useEffect, useMemo, useState } from 'react';
import {
  AlertTriangle,
  ArrowDown,
  ArrowUp,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Eye,
  EyeOff,
  FolderGit2,
  Hammer,
  Image as ImageIcon,
  Layers,
  Layout,
  Monitor,
  PackageCheck,
  Play,
  Plus,
  RefreshCw,
  Save,
  ShieldCheck,
  Sliders,
  Sparkles,
  Trash2,
} from 'lucide-react';
import { useProjectStore } from '../../stores/projectStore';
import { api } from '../../lib/api';
import type {
  BuildPreflightReport,
  BuildReport,
  MiniLayoutPreset,
  MiniTaskDef,
  MiniToolUiSchema,
  ProjectTaskRecord,
  RuntimeColumnBinding,
  TaskCategoryDef,
  UtilityToggleDef,
} from '../../types';

interface MiniToolBuilderViewProps {
  onNavigate?: (view: string) => void;
}

type PreviewRuntimeState = 'IDLE' | 'RUNNING' | 'PAUSED' | 'OFFLINE' | 'ERROR';
type ComponentId =
  | 'header_bar'
  | 'device_table'
  | 'task_checklist'
  | 'utilities_panel'
  | 'quick_log'
  | 'footer_controls';

const COMPONENT_CATALOG: Array<{ id: ComponentId; label: string; desc: string }> = [
  { id: 'header_bar', label: 'Header & Branding Bar', desc: 'Tiêu đề, version badge, màu nền & nút Start' },
  { id: 'device_table', label: 'Device ADB Table', desc: 'Bảng thiết bị thật & các cột gắn biến Runtime' },
  { id: 'task_checklist', label: 'Task Checklist', desc: 'Danh sách nhiệm vụ theo thứ tự ưu tiên' },
  { id: 'utilities_panel', label: 'Utilities Switches', desc: 'Công tắc bật/tắt tiện ích bổ sung' },
  { id: 'quick_log', label: 'Runtime Event Log', desc: 'Khung log thực thi & chiều cao co giãn' },
  { id: 'footer_controls', label: 'Footer Controls', desc: 'Thanh nút chạy ADB thật & thông tin KeyID' },
];

const WINDOW_SIZE_PRESETS: Record<MiniLayoutPreset, [number, number]> = {
  compact1_col: [640, 460],
  compact2_col: [840, 500],
  expanded: [1020, 660],
};

const SECTION_LABELS: Record<string, string> = {
  header_bar: 'Header & Branding Bar',
  device_table: 'Device ADB Table',
  task_checklist: 'Task Checklist',
  utilities_panel: 'Utilities Switches',
  quick_log: 'Runtime Event Log',
  footer_controls: 'Footer Controls',
  task_and_log_split: 'Tasks, Utilities & Log (Split)',
  status_footer: 'Footer Status Bar',
};

const normalizeSections = (raw: string[]): ComponentId[] => {
  const out: ComponentId[] = [];
  const pushUnique = (id: ComponentId) => {
    if (!out.includes(id)) out.push(id);
  };
  for (const s of raw) {
    if (s === 'task_and_log_split') {
      pushUnique('task_checklist');
      pushUnique('utilities_panel');
      pushUnique('quick_log');
    } else if (s === 'status_footer' || s === 'footer_controls') {
      pushUnique('footer_controls');
    } else if (COMPONENT_CATALOG.some((c) => c.id === s)) {
      pushUnique(s as ComponentId);
    }
  }
  if (!out.includes('header_bar')) {
    out.unshift('header_bar');
  }
  return out;
};

const buildDefaultSchema = (
  slug: string,
  title: string,
  version: string,
  detail: ReturnType<typeof useProjectStore.getState>['activeProjectDetail']
): MiniToolUiSchema => {
  const tasks: MiniTaskDef[] = (detail?.tasks || [])
    .slice()
    .sort((a, b) => a.priority - b.priority)
    .map((t) => ({
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

  const categoriesMap = new Map<string, TaskCategoryDef>();
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
    defaultWindowSize: [840, 500],
    sectionsOrder: [
      'header_bar',
      'device_table',
      'task_checklist',
      'utilities_panel',
      'quick_log',
      'footer_controls',
    ],
    hiddenComponents: [],
    deviceColumns: [
      {
        id: 'col-account',
        label: 'Account / Profile',
        runtimeVarKey: 'account_alias',
        fallbackDisplay: '—',
        minWindowWidth: 520,
        badgeColor: '#38BDF8',
      },
      {
        id: 'col-state',
        label: 'Active Task',
        runtimeVarKey: 'current_task',
        fallbackDisplay: 'Standby',
        minWindowWidth: 680,
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
    initialRuntimeVars: {
      __uiCompactDensity: 'compact',
      __uiCollapsedSections: [],
      __uiLogMaxHeightPx: 170,
    },
  };
};

export const MiniToolBuilderView: React.FC<MiniToolBuilderViewProps> = ({ onNavigate }) => {
  const { activeProjectId, activeProjectDetail, refreshActiveProject } = useProjectStore();
  const project = activeProjectDetail?.project ?? null;
  const releases = activeProjectDetail?.releases || [];
  const projectTasks: ProjectTaskRecord[] = useMemo(
    () =>
      [...(activeProjectDetail?.tasks || [])].sort(
        (a, b) => a.priority - b.priority || a.title.localeCompare(b.title)
      ),
    [activeProjectDetail?.tasks]
  );

  const [schema, setSchema] = useState<MiniToolUiSchema | null>(null);
  const [exeName, setExeName] = useState('');
  const [version, setVersion] = useState('');
  const [bundleAdb, setBundleAdb] = useState(false);
  const [customOutputDir, setCustomOutputDir] = useState('');

  const [selectedComponent, setSelectedComponent] = useState<ComponentId>('header_bar');
  const [previewState, setPreviewState] = useState<PreviewRuntimeState>('IDLE');
  const [previewCollapsed, setPreviewCollapsed] = useState<Record<string, boolean>>({});
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
      const loaded = activeProjectDetail.uiSchema;
      const normSections = normalizeSections(loaded.sectionsOrder || []);
      const vars = loaded.initialRuntimeVars || {};
      const initCollapsed: Record<string, boolean> = {};
      if (Array.isArray(vars.__uiCollapsedSections)) {
        for (const c of vars.__uiCollapsedSections) {
          initCollapsed[c] = true;
        }
      }
      setPreviewCollapsed(initCollapsed);
      setSchema({
        ...loaded,
        sectionsOrder: normSections,
        initialRuntimeVars: {
          __uiCompactDensity: loaded.layoutPreset === 'expanded' ? 'comfortable' : 'compact',
          __uiCollapsedSections: [],
          __uiLogMaxHeightPx: 170,
          ...vars,
        },
      });
    } else {
      setPreviewCollapsed({});
      setSchema(
        buildDefaultSchema(project.slug, project.windowTitle, project.version, activeProjectDetail)
      );
    }
  }, [activeProjectId, activeProjectDetail?.project.updatedAt]);

  const syncedSchema = useMemo(() => {
    if (!schema || !project) return null;
    const catMap = new Map<string, TaskCategoryDef>();
    const existingTaskOrder = new Map<string, MiniTaskDef>();
    for (const st of schema.tasks) existingTaskOrder.set(st.id, st);

    const syncedTasks: MiniTaskDef[] = projectTasks.map((t) => {
      if (!catMap.has(t.categoryId)) {
        catMap.set(t.categoryId, {
          id: t.categoryId,
          shortTag: t.categoryShortTag,
          displayName: t.categoryName,
          colorHex: t.categoryColorHex,
        });
      }
      const override = existingTaskOrder.get(t.id);
      return {
        id: t.id,
        title: t.title,
        categoryId: t.categoryId,
        workflowId: t.workflowId,
        defaultEnabled: override ? override.defaultEnabled : t.defaultEnabled,
        priority: override ? override.priority : t.priority,
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

    syncedTasks.sort((a, b) => a.priority - b.priority);

    return {
      ...schema,
      projectSlug: project.slug,
      tasks: syncedTasks.length > 0 ? syncedTasks : schema.tasks,
      taskCategories: catMap.size > 0 ? Array.from(catMap.values()) : schema.taskCategories,
    };
  }, [schema, project, projectTasks]);

  const updateRuntimeVar = (key: string, value: any) => {
    if (!schema) return;
    setSchema({
      ...schema,
      initialRuntimeVars: {
        ...(schema.initialRuntimeVars || {}),
        [key]: value,
      },
    });
  };

  const compactDensity: 'compact' | 'comfortable' =
    (syncedSchema?.initialRuntimeVars?.__uiCompactDensity as 'compact' | 'comfortable') ||
    (syncedSchema?.layoutPreset === 'expanded' ? 'comfortable' : 'compact');
  const logMaxHeightPx: number = Number(syncedSchema?.initialRuntimeVars?.__uiLogMaxHeightPx || 170);
  const defaultCollapsedSections: string[] = Array.isArray(
    syncedSchema?.initialRuntimeVars?.__uiCollapsedSections
  )
    ? syncedSchema!.initialRuntimeVars.__uiCollapsedSections
    : [];

  const saveProjectAndSchema = async (): Promise<MiniToolUiSchema | null> => {
    if (!activeProjectId || !project || !syncedSchema) return null;

    // Persist any reordered task priorities back into SQLite project_tasks
    for (const st of syncedSchema.tasks) {
      const dbTask = projectTasks.find((t) => t.id === st.id);
      if (
        dbTask &&
        (dbTask.priority !== st.priority || dbTask.defaultEnabled !== st.defaultEnabled)
      ) {
        const catMeta: TaskCategoryDef = syncedSchema.taskCategories.find(
          (c) => c.id === dbTask.categoryId
        ) || {
          id: dbTask.categoryId,
          shortTag: dbTask.categoryShortTag,
          displayName: dbTask.categoryName,
          colorHex: dbTask.categoryColorHex,
        };
        await api.studioUpsertProjectTask(activeProjectId, st, catMeta);
      }
    }

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
        text: `Đã lưu UI Schema (${saved?.layoutPreset}, ${saved?.defaultWindowSize[0]}x${saved?.defaultWindowSize[1]}, ${saved?.sectionsOrder.length} components) vào SQLite.`,
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

  const addComponent = (id: ComponentId) => {
    if (!schema) return;
    if (schema.sectionsOrder.includes(id)) {
      setSchema({
        ...schema,
        hiddenComponents: schema.hiddenComponents.filter((h) => h !== id),
      });
    } else {
      setSchema({
        ...schema,
        sectionsOrder: [...schema.sectionsOrder, id],
        hiddenComponents: schema.hiddenComponents.filter((h) => h !== id),
      });
    }
    setSelectedComponent(id);
  };

  const removeComponent = (id: ComponentId) => {
    if (!schema) return;
    setSchema({
      ...schema,
      sectionsOrder: schema.sectionsOrder.filter((s) => s !== id),
      hiddenComponents: schema.hiddenComponents.filter((s) => s !== id),
    });
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

  const moveTaskPriority = (idx: number, delta: -1 | 1) => {
    if (!syncedSchema) return;
    const list = [...syncedSchema.tasks];
    const target = idx + delta;
    if (target < 0 || target >= list.length) return;
    const tmp = list[target];
    list[target] = list[idx];
    list[idx] = tmp;
    const reindexed = list.map((t, i) => ({ ...t, priority: i + 1 }));
    setSchema({ ...syncedSchema, tasks: reindexed });
  };

  const toggleDefaultCollapsed = (secId: string) => {
    const next = defaultCollapsedSections.includes(secId)
      ? defaultCollapsedSections.filter((s) => s !== secId)
      : [...defaultCollapsedSections, secId];
    updateRuntimeVar('__uiCollapsedSections', next);
    setPreviewCollapsed((prev) => ({ ...prev, [secId]: next.includes(secId) }));
  };

  const applyPreset = (preset: MiniLayoutPreset) => {
    if (!syncedSchema) return;
    const size = WINDOW_SIZE_PRESETS[preset];
    setSchema({
      ...syncedSchema,
      layoutPreset: preset,
      defaultWindowSize: size,
      initialRuntimeVars: {
        ...(syncedSchema.initialRuntimeVars || {}),
        __uiCompactDensity: preset === 'expanded' ? 'comfortable' : 'compact',
        __uiLogMaxHeightPx: preset === 'compact1_col' ? 120 : preset === 'compact2_col' ? 165 : 230,
      },
    });
  };

  const addDeviceColumn = () => {
    if (!syncedSchema) return;
    const nextCol: RuntimeColumnBinding = {
      id: `col-${Date.now().toString(36)}`,
      label: `Runtime Col ${syncedSchema.deviceColumns.length + 1}`,
      runtimeVarKey: `var_${syncedSchema.deviceColumns.length + 1}`,
      fallbackDisplay: '—',
      minWindowWidth: 500,
      badgeColor: syncedSchema.themeAccentHex,
    };
    setSchema({ ...syncedSchema, deviceColumns: [...syncedSchema.deviceColumns, nextCol] });
  };

  const addUtilityToggle = () => {
    if (!syncedSchema) return;
    const nextUtil: UtilityToggleDef = {
      id: `util.custom_${Date.now().toString(36)}`,
      label: `Tiện ích mới #${syncedSchema.utilities.length + 1}`,
      defaultEnabled: true,
      requiredModule: null,
      settingsSchema: [],
    };
    setSchema({ ...syncedSchema, utilities: [...syncedSchema.utilities, nextUtil] });
  };

  if (!project || !syncedSchema) {
    return (
      <div className="flex-1 flex items-center justify-center bg-[#101722] p-6">
        <div className="fp-panel p-6 max-w-md text-center space-y-3">
          <PackageCheck size={28} className="mx-auto text-[#B291F7]" />
          <h2 className="text-base font-bold text-[#EDF3FC]">Chưa chọn Game Project</h2>
          <p className="text-xs text-[#74869D]">
            Chọn một Game Project từ Header để mở WYSIWYG Mini Tool Builder và xuất file EXE độc lập.
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

  const activeSections = syncedSchema.sectionsOrder;
  const hiddenSet = new Set(syncedSchema.hiddenComponents);
  const missingCatalog = COMPONENT_CATALOG.filter((c) => !activeSections.includes(c.id));

  const showHeader = activeSections.includes('header_bar') && !hiddenSet.has('header_bar');
  const showDeviceTable = activeSections.includes('device_table') && !hiddenSet.has('device_table');
  const showTasks = activeSections.includes('task_checklist') && !hiddenSet.has('task_checklist');
  const showUtilities =
    activeSections.includes('utilities_panel') && !hiddenSet.has('utilities_panel');
  const showLog = activeSections.includes('quick_log') && !hiddenSet.has('quick_log');
  const showFooter =
    activeSections.includes('footer_controls') && !hiddenSet.has('footer_controls');

  const isSingleCol = syncedSchema.layoutPreset === 'compact1_col';
  const rowPadClass = compactDensity === 'compact' ? 'p-1.5 text-[11px]' : 'p-2.5 text-xs';

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden bg-[#101722] text-[#EDF3FC]">
      {/* Top Toolbar */}
      <div className="px-4 py-2.5 border-b border-[#2A3647] bg-[#141C28] flex flex-wrap items-center justify-between gap-3 shrink-0">
        <div>
          <div className="flex items-center gap-2 flex-wrap">
            <span className="px-2 py-0.5 rounded text-[10px] font-mono font-bold bg-[#173355] text-[#B291F7] border border-[#2A3647]">
              WYSIWYG MINI TOOL BUILDER
            </span>
            <span className="text-xs font-mono text-[#A9B8CC]">
              {project.slug} · {syncedSchema.layoutPreset} · {syncedSchema.defaultWindowSize[0]}×
              {syncedSchema.defaultWindowSize[1]} · {compactDensity}
            </span>
          </div>
          <h1 className="text-sm font-bold text-[#EDF3FC] mt-0.5">
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
            className="px-3.5 py-1.5 rounded-md bg-[#1878F5] hover:bg-[#2F8CFF] disabled:opacity-50 text-white text-xs font-bold flex items-center gap-1.5 cursor-pointer shadow-sm"
          >
            <Hammer size={13} />
            <span>{busyAction === 'export' ? 'Đang đóng gói...' : 'Build & Xuất EXE'}</span>
          </button>
        </div>
      </div>

      {notice && (
        <div
          className={`mx-4 mt-2.5 px-3 py-2 rounded-lg border text-xs flex items-center gap-2 shrink-0 ${
            notice.kind === 'ok'
              ? 'bg-[#133528] border-[#3BC38D]/40 text-[#3BC38D]'
              : 'bg-[#381E24] border-[#F27883]/40 text-[#F27883]'
          }`}
        >
          {notice.kind === 'ok' ? <CheckCircle2 size={14} /> : <AlertTriangle size={14} />}
          <span className="truncate">{notice.text}</span>
        </div>
      )}

      {/* 3-Column WYSIWYG Builder */}
      <div className="flex-1 grid grid-cols-12 gap-3 p-4 overflow-hidden min-h-0">
        {/* Left Pane (4 cols): Component Palette, Ordering & Contextual Inspector */}
        <div className="col-span-12 xl:col-span-4 fp-panel p-3.5 overflow-y-auto space-y-3.5 text-xs">
          {/* Presets & Window Size */}
          <div className="space-y-2">
            <div className="flex items-center justify-between font-bold text-[#EDF3FC]">
              <span className="flex items-center gap-1.5">
                <Layout size={13} className="text-[#4B9CFF]" />
                <span>Preset &amp; Kích thước Cửa sổ</span>
              </span>
              <span className="font-mono text-[10px] text-[#74869D]">
                {syncedSchema.defaultWindowSize[0]}×{syncedSchema.defaultWindowSize[1]}
              </span>
            </div>
            <div className="grid grid-cols-3 gap-1.5">
              {(['compact1_col', 'compact2_col', 'expanded'] as MiniLayoutPreset[]).map((p) => (
                <button
                  key={p}
                  type="button"
                  onClick={() => applyPreset(p)}
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
            <div className="grid grid-cols-3 gap-2">
              <div>
                <label className="block text-[10px] text-[#74869D] mb-0.5">Width (px)</label>
                <input
                  type="number"
                  value={syncedSchema.defaultWindowSize[0]}
                  onChange={(e) =>
                    setSchema({
                      ...syncedSchema,
                      defaultWindowSize: [
                        Math.max(380, Number(e.target.value) || 640),
                        syncedSchema.defaultWindowSize[1],
                      ],
                    })
                  }
                  className="w-full bg-[#101722] border border-[#2A3647] rounded px-2 py-1 text-[#EDF3FC] font-mono"
                />
              </div>
              <div>
                <label className="block text-[10px] text-[#74869D] mb-0.5">Height (px)</label>
                <input
                  type="number"
                  value={syncedSchema.defaultWindowSize[1]}
                  onChange={(e) =>
                    setSchema({
                      ...syncedSchema,
                      defaultWindowSize: [
                        syncedSchema.defaultWindowSize[0],
                        Math.max(360, Number(e.target.value) || 460),
                      ],
                    })
                  }
                  className="w-full bg-[#101722] border border-[#2A3647] rounded px-2 py-1 text-[#EDF3FC] font-mono"
                />
              </div>
              <div>
                <label className="block text-[10px] text-[#74869D] mb-0.5">Mật độ dòng</label>
                <select
                  value={compactDensity}
                  onChange={(e) => updateRuntimeVar('__uiCompactDensity', e.target.value)}
                  className="w-full bg-[#101722] border border-[#2A3647] rounded px-1.5 py-1 text-[#EDF3FC]"
                >
                  <option value="compact">Compact</option>
                  <option value="comfortable">Comfortable</option>
                </select>
              </div>
            </div>
          </div>

          {/* Component Palette & Active Hierarchy (Add / Remove / Hide / Reorder) */}
          <div className="space-y-2 pt-2.5 border-t border-[#2A3647]">
            <div className="flex items-center justify-between font-bold text-[#EDF3FC]">
              <span className="flex items-center gap-1.5">
                <Layers size={13} className="text-[#38BDF8]" />
                <span>Component Tree ({activeSections.length})</span>
              </span>
              <span className="text-[10px] text-[#74869D]">Nhấp để sửa thuộc tính</span>
            </div>

            {missingCatalog.length > 0 && (
              <div className="p-2 rounded bg-[#101722] border border-dashed border-[#2A3647] flex flex-wrap gap-1.5">
                {missingCatalog.map((c) => (
                  <button
                    key={c.id}
                    type="button"
                    onClick={() => addComponent(c.id)}
                    className="px-2 py-1 rounded bg-[#173355] hover:bg-[#1E4470] text-[#38BDF8] text-[10px] font-semibold flex items-center gap-1 cursor-pointer"
                  >
                    <Plus size={11} />
                    <span>{c.label}</span>
                  </button>
                ))}
              </div>
            )}

            <div className="space-y-1">
              {activeSections.map((secId, idx) => {
                const label = SECTION_LABELS[secId] || secId;
                const hidden = hiddenSet.has(secId);
                const selected = selectedComponent === secId;
                return (
                  <div
                    key={secId}
                    onClick={() => setSelectedComponent(secId as ComponentId)}
                    className={`px-2.5 py-1.5 rounded border flex items-center justify-between cursor-pointer ${
                      selected
                        ? 'bg-[#172E49] border-[#38BDF8]'
                        : 'bg-[#101722] border-[#2A3647] hover:border-[#3D4F67]'
                    }`}
                  >
                    <span className={hidden ? 'text-[#5A6D85] line-through' : 'text-[#EDF3FC]'}>
                      {label}
                    </span>
                    <div
                      className="flex items-center gap-1"
                      onClick={(e) => e.stopPropagation()}
                    >
                      <button
                        type="button"
                        onClick={() => moveSection(idx, -1)}
                        title="Di chuyển lên"
                        className="p-1 text-[#74869D] hover:text-[#EDF3FC] cursor-pointer"
                      >
                        <ArrowUp size={12} />
                      </button>
                      <button
                        type="button"
                        onClick={() => moveSection(idx, 1)}
                        title="Di chuyển xuống"
                        className="p-1 text-[#74869D] hover:text-[#EDF3FC] cursor-pointer"
                      >
                        <ArrowDown size={12} />
                      </button>
                      <button
                        type="button"
                        onClick={() => toggleSectionHidden(secId)}
                        title={hidden ? 'Hiện component' : 'Ẩn component'}
                        className="p-1 text-[#A9B8CC] hover:text-[#38BDF8] cursor-pointer"
                      >
                        {hidden ? <EyeOff size={12} /> : <Eye size={12} />}
                      </button>
                      <button
                        type="button"
                        onClick={() => removeComponent(secId as ComponentId)}
                        title="Xóa khỏi UI Schema"
                        className="p-1 text-[#74869D] hover:text-[#F27883] cursor-pointer"
                      >
                        <Trash2 size={12} />
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>

          {/* Contextual Component Inspector */}
          <div className="space-y-2.5 pt-2.5 border-t border-[#2A3647]">
            <div className="flex items-center justify-between font-bold text-[#EDF3FC]">
              <span className="flex items-center gap-1.5">
                <Sliders size={13} className="text-[#B291F7]" />
                <span>Thuộc tính: {SECTION_LABELS[selectedComponent]}</span>
              </span>
            </div>

            {selectedComponent === 'header_bar' && (
              <div className="space-y-2">
                <div>
                  <label className="block text-[#A9B8CC] mb-0.5 text-[11px]">Tiêu đề cửa sổ</label>
                  <input
                    type="text"
                    value={syncedSchema.windowTitle}
                    onChange={(e) => setSchema({ ...syncedSchema, windowTitle: e.target.value })}
                    className="w-full bg-[#101722] border border-[#2A3647] rounded px-2.5 py-1.5 text-[#EDF3FC] outline-none"
                  />
                </div>
                <div className="grid grid-cols-2 gap-2">
                  <div>
                    <label className="block text-[#A9B8CC] mb-0.5 text-[11px]">Tên file EXE</label>
                    <input
                      type="text"
                      value={exeName}
                      onChange={(e) => setExeName(e.target.value)}
                      className="w-full bg-[#101722] border border-[#2A3647] rounded px-2 py-1 text-[#EDF3FC] font-mono outline-none"
                    />
                  </div>
                  <div>
                    <label className="block text-[#A9B8CC] mb-0.5 text-[11px]">Phiên bản</label>
                    <input
                      type="text"
                      value={version}
                      onChange={(e) => {
                        setVersion(e.target.value);
                        setSchema({ ...syncedSchema, versionBadge: `v${e.target.value}` });
                      }}
                      className="w-full bg-[#101722] border border-[#2A3647] rounded px-2 py-1 text-[#EDF3FC] font-mono outline-none"
                    />
                  </div>
                  <div className="min-w-0">
                    <label className="block text-[#A9B8CC] mb-0.5 text-[11px]">Màu Accent</label>
                    <div className="flex items-center gap-1.5 min-w-0">
                      <input
                        type="color"
                        value={syncedSchema.themeAccentHex}
                        onChange={(e) =>
                          setSchema({ ...syncedSchema, themeAccentHex: e.target.value })
                        }
                        className="w-7 h-7 rounded bg-transparent border border-[#2A3647] shrink-0"
                      />
                      <input
                        type="text"
                        value={syncedSchema.themeAccentHex}
                        onChange={(e) =>
                          setSchema({ ...syncedSchema, themeAccentHex: e.target.value })
                        }
                        className="w-full min-w-0 bg-[#101722] border border-[#2A3647] rounded px-2 py-1 text-[#EDF3FC] font-mono"
                      />
                    </div>
                  </div>
                  <div className="min-w-0">
                    <label className="block text-[#A9B8CC] mb-0.5 text-[11px]">Màu Header</label>
                    <div className="flex items-center gap-1.5 min-w-0">
                      <input
                        type="color"
                        value={syncedSchema.headerBgHex}
                        onChange={(e) =>
                          setSchema({ ...syncedSchema, headerBgHex: e.target.value })
                        }
                        className="w-7 h-7 rounded bg-transparent border border-[#2A3647] shrink-0"
                      />
                      <input
                        type="text"
                        value={syncedSchema.headerBgHex}
                        onChange={(e) =>
                          setSchema({ ...syncedSchema, headerBgHex: e.target.value })
                        }
                        className="w-full min-w-0 bg-[#101722] border border-[#2A3647] rounded px-2 py-1 text-[#EDF3FC] font-mono"
                      />
                    </div>
                  </div>
                </div>
                <div className="flex items-center gap-2 pt-1 flex-wrap">
                  <button
                    type="button"
                    onClick={handleEnsureDefaultIcon}
                    disabled={busyAction !== null}
                    className="px-2.5 py-1 rounded bg-[#101722] hover:bg-[#192434] border border-[#2A3647] text-[11px] text-[#38BDF8] flex items-center gap-1 cursor-pointer"
                  >
                    <Sparkles size={11} />
                    <span>Sinh Icon .ico</span>
                  </button>
                  <label className="px-2.5 py-1 rounded bg-[#101722] hover:bg-[#192434] border border-[#2A3647] text-[11px] text-[#A9B8CC] flex items-center gap-1 cursor-pointer">
                    <ImageIcon size={11} />
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
                  <span>Đóng gói kèm bin/adb.exe vào Release</span>
                </label>
              </div>
            )}

            {selectedComponent === 'device_table' && (
              <div className="space-y-2">
                <label className="flex items-center gap-2 text-[11px] text-[#A9B8CC] cursor-pointer">
                  <input
                    type="checkbox"
                    checked={defaultCollapsedSections.includes('device_table')}
                    onChange={() => toggleDefaultCollapsed('device_table')}
                    className="accent-[#1878F5]"
                  />
                  <span>Thu gọn Bảng Thiết bị mặc định khi khởi động</span>
                </label>

                <div className="flex items-center justify-between pt-1">
                  <span className="font-semibold text-[#EDF3FC]">
                    Cột Gắn Biến Runtime ({syncedSchema.deviceColumns.length})
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
                    className="p-2 rounded bg-[#101722] border border-[#2A3647] space-y-1.5"
                  >
                    <div className="flex items-center gap-1.5">
                      <input
                        type="text"
                        value={col.label}
                        placeholder="Tên cột"
                        onChange={(e) => {
                          const next = [...syncedSchema.deviceColumns];
                          next[idx] = { ...col, label: e.target.value };
                          setSchema({ ...syncedSchema, deviceColumns: next });
                        }}
                        className="flex-1 bg-[#16202F] border border-[#2A3647] rounded px-2 py-1 text-[#EDF3FC]"
                      />
                      <input
                        type="text"
                        value={col.runtimeVarKey}
                        placeholder="runtime_var"
                        onChange={(e) => {
                          const next = [...syncedSchema.deviceColumns];
                          next[idx] = { ...col, runtimeVarKey: e.target.value };
                          setSchema({ ...syncedSchema, deviceColumns: next });
                        }}
                        className="w-28 bg-[#16202F] border border-[#2A3647] rounded px-2 py-1 text-[#38BDF8] font-mono"
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
                    <div className="grid grid-cols-2 gap-1.5 text-[10px]">
                      <input
                        type="text"
                        value={col.fallbackDisplay}
                        placeholder="Fallback text"
                        onChange={(e) => {
                          const next = [...syncedSchema.deviceColumns];
                          next[idx] = { ...col, fallbackDisplay: e.target.value };
                          setSchema({ ...syncedSchema, deviceColumns: next });
                        }}
                        className="bg-[#16202F] border border-[#2A3647] rounded px-2 py-0.5 text-[#A9B8CC]"
                      />
                      <input
                        type="number"
                        value={col.minWindowWidth}
                        placeholder="Min width px"
                        onChange={(e) => {
                          const next = [...syncedSchema.deviceColumns];
                          next[idx] = {
                            ...col,
                            minWindowWidth: Number(e.target.value) || 400,
                          };
                          setSchema({ ...syncedSchema, deviceColumns: next });
                        }}
                        className="bg-[#16202F] border border-[#2A3647] rounded px-2 py-0.5 text-[#A9B8CC] font-mono"
                      />
                    </div>
                  </div>
                ))}
              </div>
            )}

            {selectedComponent === 'task_checklist' && (
              <div className="space-y-1.5">
                <div className="text-[11px] text-[#74869D]">
                  Sắp xếp thứ tự Task &amp; bật/tắt mặc định trong Mini Tool:
                </div>
                {syncedSchema.tasks.length === 0 ? (
                  <div className="p-2 rounded bg-[#101722] text-[#74869D]">
                    Chưa có Task — thêm Task ở màn hình Tasks &amp; Modules.
                  </div>
                ) : (
                  syncedSchema.tasks.map((t, idx) => (
                    <div
                      key={t.id}
                      className="p-2 rounded bg-[#101722] border border-[#2A3647] flex items-center justify-between gap-2"
                    >
                      <label className="flex items-center gap-1.5 min-w-0 cursor-pointer">
                        <input
                          type="checkbox"
                          checked={t.defaultEnabled}
                          onChange={(e) => {
                            const next = syncedSchema.tasks.map((item) =>
                              item.id === t.id
                                ? { ...item, defaultEnabled: e.target.checked }
                                : item
                            );
                            setSchema({ ...syncedSchema, tasks: next });
                          }}
                          className="accent-[#3BC38D]"
                        />
                        <span className="truncate text-[#EDF3FC]">
                          P{t.priority}. {t.title}
                        </span>
                      </label>
                      <div className="flex items-center gap-1 shrink-0">
                        <button
                          type="button"
                          onClick={() => moveTaskPriority(idx, -1)}
                          className="p-1 text-[#74869D] hover:text-[#EDF3FC] cursor-pointer"
                        >
                          <ArrowUp size={11} />
                        </button>
                        <button
                          type="button"
                          onClick={() => moveTaskPriority(idx, 1)}
                          className="p-1 text-[#74869D] hover:text-[#EDF3FC] cursor-pointer"
                        >
                          <ArrowDown size={11} />
                        </button>
                      </div>
                    </div>
                  ))
                )}
              </div>
            )}

            {selectedComponent === 'utilities_panel' && (
              <div className="space-y-1.5">
                <div className="flex items-center justify-between">
                  <span className="font-semibold text-[#EDF3FC]">
                    Tiện ích ({syncedSchema.utilities.length})
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
            )}

            {selectedComponent === 'quick_log' && (
              <div className="space-y-2">
                <label className="flex items-center gap-2 text-[11px] text-[#A9B8CC] cursor-pointer">
                  <input
                    type="checkbox"
                    checked={defaultCollapsedSections.includes('quick_log')}
                    onChange={() => toggleDefaultCollapsed('quick_log')}
                    className="accent-[#1878F5]"
                  />
                  <span>Thu gọn Nhật ký mặc định</span>
                </label>
                <div>
                  <div className="flex justify-between text-[11px] mb-1">
                    <span className="text-[#A9B8CC]">Chiều cao tối đa khung Log</span>
                    <span className="font-mono text-[#38BDF8]">{logMaxHeightPx} px</span>
                  </div>
                  <input
                    type="range"
                    min={90}
                    max={360}
                    step={10}
                    value={logMaxHeightPx}
                    onChange={(e) => updateRuntimeVar('__uiLogMaxHeightPx', Number(e.target.value))}
                    className="w-full accent-[#1878F5]"
                  />
                </div>
              </div>
            )}

            {selectedComponent === 'footer_controls' && (
              <p className="text-[11px] text-[#74869D]">
                Hiển thị nút Chạy Nhiệm vụ (ADB Thật), Quét ADB và Footer KeyID / SQLite Namespace
                của Mini Tool.
              </p>
            )}
          </div>
        </div>

        {/* Center Column (5 cols): Interactive WYSIWYG Live Preview */}
        <div className="col-span-12 xl:col-span-5 fp-panel p-3.5 flex flex-col overflow-hidden">
          <div className="flex items-center justify-between pb-2.5 border-b border-[#2A3647] shrink-0 gap-2 flex-wrap">
            <div className="flex items-center gap-1.5">
              <Monitor size={14} className="text-[#38BDF8]" />
              <span className="text-xs font-bold text-[#EDF3FC]">
                WYSIWYG Live Preview (1:1 Runtime)
              </span>
            </div>
            <div className="flex items-center gap-1 bg-[#101722] p-0.5 rounded border border-[#2A3647]">
              {(['IDLE', 'RUNNING', 'PAUSED', 'OFFLINE', 'ERROR'] as PreviewRuntimeState[]).map(
                (st) => (
                  <button
                    key={st}
                    type="button"
                    onClick={() => setPreviewState(st)}
                    className={`px-1.5 py-0.5 rounded text-[10px] font-mono font-bold cursor-pointer ${
                      previewState === st
                        ? st === 'ERROR'
                          ? 'bg-[#DC2626] text-white'
                          : 'bg-[#1878F5] text-white'
                        : 'text-[#74869D] hover:text-[#EDF3FC]'
                    }`}
                  >
                    {st}
                  </button>
                )
              )}
            </div>
          </div>

          <div className="flex-1 mt-2.5 rounded-xl border border-[#2A3647] bg-[#0D131D] overflow-y-auto flex flex-col justify-between shadow-inner">
            <div className="flex flex-col">
              {/* Header Bar */}
              {showHeader && (
                <div
                  onClick={() => setSelectedComponent('header_bar')}
                  className={`px-3 py-2 border-b border-[#2A3647] flex items-center justify-between gap-2 cursor-pointer transition-all ${
                    selectedComponent === 'header_bar' ? 'ring-2 ring-inset ring-[#38BDF8]' : ''
                  }`}
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
                    <span className="px-1.5 py-0.5 rounded text-[10px] font-mono bg-black/30 text-[#CBD5E1] shrink-0">
                      {syncedSchema.versionBadge}
                    </span>
                  </div>

                  <div className="flex items-center gap-1.5 shrink-0">
                    <span
                      className={`px-2 py-0.5 rounded text-[10px] font-mono font-bold ${
                        previewState === 'ERROR'
                          ? 'bg-[#381E24] text-[#F27883] border border-[#F27883]/50'
                          : previewState === 'RUNNING'
                          ? 'bg-[#133528] text-[#3BC38D]'
                          : 'bg-black/40 text-[#94A3B8]'
                      }`}
                    >
                      {previewState === 'ERROR' ? 'ERROR: ADB_MISSING' : previewState}
                    </span>
                  </div>
                </div>
              )}

              <div className="p-2.5 space-y-2.5">
                {/* Device Table */}
                {showDeviceTable && (
                  <div
                    onClick={() => setSelectedComponent('device_table')}
                    className={`rounded-lg border bg-[#141D2B] overflow-hidden text-[11px] cursor-pointer ${
                      selectedComponent === 'device_table' ? 'border-[#38BDF8]' : 'border-[#2A3647]'
                    }`}
                  >
                    <div
                      onClick={(e) => {
                        e.stopPropagation();
                        setPreviewCollapsed((p) => ({ ...p, device_table: !p.device_table }));
                      }}
                      className="px-2.5 py-1 bg-[#192434] border-b border-[#2A3647] font-bold text-[#A9B8CC] flex items-center justify-between text-[10px] uppercase"
                    >
                      <span>Thiết bị ADB &amp; Cột Runtime</span>
                      {previewCollapsed.device_table ? (
                        <ChevronRight size={12} />
                      ) : (
                        <ChevronDown size={12} />
                      )}
                    </div>
                    {!previewCollapsed.device_table && (
                      <table className="w-full text-left">
                        <thead className="text-[10px] text-[#74869D] border-b border-[#2A3647]">
                          <tr>
                            <th className="p-1.5">Device</th>
                            {syncedSchema.deviceColumns.map((c) => (
                              <th key={c.id} className="p-1.5">
                                {c.label}
                              </th>
                            ))}
                          </tr>
                        </thead>
                        <tbody>
                          <tr className="text-[#EDF3FC]">
                            <td className="p-1.5 font-mono">
                              {previewState === 'OFFLINE' || previewState === 'ERROR'
                                ? '0 devices'
                                : 'ldplayer-5554'}
                            </td>
                            {syncedSchema.deviceColumns.map((c) => (
                              <td
                                key={c.id}
                                className="p-1.5 font-mono"
                                style={{
                                  color:
                                    previewState === 'RUNNING'
                                      ? c.badgeColor || '#38BDF8'
                                      : '#94A3B8',
                                }}
                              >
                                {previewState === 'RUNNING'
                                  ? `${c.runtimeVarKey}=active`
                                  : c.fallbackDisplay}
                              </td>
                            ))}
                          </tr>
                        </tbody>
                      </table>
                    )}
                  </div>
                )}

                {/* Tasks + Utilities + Log adaptive grid */}
                <div className={`grid gap-2.5 ${isSingleCol ? 'grid-cols-1' : 'grid-cols-2'}`}>
                  <div className="space-y-2.5">
                    {showTasks && (
                      <div
                        onClick={() => setSelectedComponent('task_checklist')}
                        className={`rounded-lg border bg-[#141D2B] p-2.5 space-y-1.5 cursor-pointer ${
                          selectedComponent === 'task_checklist'
                            ? 'border-[#38BDF8]'
                            : 'border-[#2A3647]'
                        }`}
                      >
                        <div className="text-[10.5px] font-bold text-[#EDF3FC] uppercase">
                          Nhiệm vụ ({syncedSchema.tasks.length})
                        </div>
                        {syncedSchema.tasks.length === 0 ? (
                          <div className="text-[10px] text-[#74869D]">Chưa có Task.</div>
                        ) : (
                          <div className="space-y-1">
                            {syncedSchema.tasks.map((t, idx) => (
                              <div
                                key={t.id}
                                className={`flex items-start justify-between gap-2 rounded bg-[#0D131D] border border-[#2A3647] ${rowPadClass}`}
                              >
                                <span className="text-[#EDF3FC] break-words leading-snug">
                                  {t.title}
                                </span>
                                <span
                                  className="px-1.5 py-0.5 rounded text-[9px] font-mono shrink-0"
                                  style={
                                    previewState === 'ERROR' && idx === 0
                                      ? { backgroundColor: '#381E24', color: '#F27883' }
                                      : {
                                          backgroundColor: `${syncedSchema.themeAccentHex}25`,
                                          color: syncedSchema.themeAccentHex,
                                        }
                                  }
                                >
                                  {previewState === 'ERROR' && idx === 0
                                    ? 'ERR'
                                    : previewState === 'PAUSED'
                                    ? 'PAUSED'
                                    : t.defaultEnabled
                                    ? 'ON'
                                    : 'OFF'}
                                </span>
                              </div>
                            ))}
                          </div>
                        )}
                      </div>
                    )}

                    {showUtilities && (
                      <div
                        onClick={() => setSelectedComponent('utilities_panel')}
                        className={`rounded-lg border bg-[#141D2B] p-2.5 space-y-1.5 cursor-pointer ${
                          selectedComponent === 'utilities_panel'
                            ? 'border-[#38BDF8]'
                            : 'border-[#2A3647]'
                        }`}
                      >
                        <div className="text-[10.5px] font-bold text-[#EDF3FC] uppercase">
                          Tiện ích ({syncedSchema.utilities.length})
                        </div>
                        <div className="space-y-1">
                          {syncedSchema.utilities.map((u) => (
                            <div
                              key={u.id}
                              className="flex items-center justify-between text-[11px] text-[#A9B8CC] gap-2"
                            >
                              <span className="break-words">{u.label}</span>
                              <span className="font-mono text-[10px] text-[#3BC38D] shrink-0">
                                {u.defaultEnabled ? 'ON' : 'OFF'}
                              </span>
                            </div>
                          ))}
                        </div>
                      </div>
                    )}
                  </div>

                  {showLog && (
                    <div
                      onClick={() => setSelectedComponent('quick_log')}
                      style={{ maxHeight: `${logMaxHeightPx}px` }}
                      className={`rounded-lg border bg-[#090E16] p-2.5 text-[10px] font-mono overflow-y-auto cursor-pointer ${
                        selectedComponent === 'quick_log' ? 'border-[#38BDF8]' : 'border-[#2A3647]'
                      } ${previewState === 'ERROR' ? 'text-[#F27883]' : 'text-[#A7F3D0]'}`}
                    >
                      <div className="text-[#74869D] mb-1">NHẬT KÝ ({logMaxHeightPx}px cap)</div>
                      {previewState === 'ERROR'
                        ? '[ERROR] Từ chối giả lập (Fail-Closed): Không tìm thấy thiết bị ADB thật.'
                        : `[${previewState}] ${syncedSchema.windowTitle} · Ready · Ed25519 Signed`}
                    </div>
                  )}
                </div>
              </div>
            </div>

            {showFooter && (
              <div
                onClick={() => setSelectedComponent('footer_controls')}
                className={`px-3 py-2 border-t border-[#2A3647] bg-[#141D2B] flex items-center justify-between cursor-pointer ${
                  selectedComponent === 'footer_controls' ? 'ring-2 ring-inset ring-[#38BDF8]' : ''
                }`}
              >
                <span
                  className="px-2.5 py-1 rounded text-[10px] font-bold text-white inline-flex items-center gap-1"
                  style={{ backgroundColor: syncedSchema.themeAccentHex }}
                >
                  <Play size={10} />
                  <span>CHẠY NHIỆM VỤ</span>
                </span>
                <span className="text-[10px] font-mono text-[#74869D]">
                  {syncedSchema.layoutPreset} · {syncedSchema.defaultWindowSize[0]}×
                  {syncedSchema.defaultWindowSize[1]}
                </span>
              </div>
            )}
          </div>
        </div>

        {/* Right Column (3 cols): Preflight & Signed Releases */}
        <div className="col-span-12 xl:col-span-3 fp-panel p-3.5 overflow-y-auto space-y-3.5 text-xs">
          <div className="space-y-1.5">
            <div className="font-bold text-[#EDF3FC]">Thư mục Xuất Release (Tuỳ chọn)</div>
            <input
              type="text"
              value={customOutputDir}
              onChange={(e) => setCustomOutputDir(e.target.value)}
              placeholder={`Mặc định: projects/${project.slug}/releases/v${version || project.version}`}
              className="w-full bg-[#101722] border border-[#2A3647] rounded px-2.5 py-1.5 text-[#EDF3FC] font-mono text-[11px] outline-none"
            />
          </div>

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
                Nhấn &quot;Kiểm tra Preflight&quot; hoặc &quot;Build &amp; Xuất EXE&quot;.
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

          {lastBuildReport && (
            <div className="space-y-1.5 pt-2 border-t border-[#2A3647] text-[11px]">
              <div className="font-bold text-[#3BC38D]">Build Report Mới nhất</div>
              <div className="p-2 rounded bg-[#101722] border border-[#3BC38D]/40 font-mono space-y-1 text-[10px]">
                <div className="text-[#EDF3FC] break-all">{lastBuildReport.toolExePath}</div>
                <div className="text-[#A9B8CC]">
                  SHA: {lastBuildReport.toolExeSha256.slice(0, 16)}...
                </div>
                <div className="text-[#A9B8CC]">
                  Key: {lastBuildReport.keyId} · PE Verified:{' '}
                  {String(lastBuildReport.peEmbeddedPubkeyVerified)}
                </div>
              </div>
            </div>
          )}

          <div className="space-y-2 pt-2 border-t border-[#2A3647]">
            <div className="flex items-center justify-between">
              <span className="font-bold text-[#EDF3FC]">Lịch sử Phát hành ({releases.length})</span>
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
                    className="p-2 rounded bg-[#101722] border border-[#2A3647] space-y-0.5 text-[11px]"
                  >
                    <div className="font-bold text-[#EDF3FC] truncate">
                      v{rel.version} · {rel.exePath.split(/[\\/]/).pop()}
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
