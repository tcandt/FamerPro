import { invoke } from '@tauri-apps/api/core';
import type {
  ActionExecutionMode,
  ActiveDeviceLeaseView,
  AdbPortMappingsOverview,
  AppPackageInfo,
  AppStatus,
  BackgroundJobStarted,
  BackupReport,
  Bounds,
  BuilderReleaseExportResult,
  BuildPreflightReport,
  CloneProjectRequest,
  CompatibilityReport,
  CreateProjectRequest,
  CredentialMetadata,
  DbOptimizeReport,
  DetectionResult,
  DeviceDetail,
  ActionReceipt,
  DeviceHealthReport,
  DevicePortMapping,
  DeviceSummary,
  ElementActionKind,
  ElementActionResult,
  ElementNode,
  ElementSnapshot,
  GameProjectDetail,
  GameProjectRecord,
  GameProjectSummary,
  GeneratedSelector,
  InspectSnapshotResult,
  IntegrityReport,
  JobResumeDetail,
  Jx2MigrationSummary,
  LdPlayerInstance,
  LibraryElementRecord,
  ManagedInstallInspection,
  MiniTaskDef,
  MiniToolUiSchema,
  NativeElementSelector,
  ProducerKeyMetadata,
  ProjectAccountInput,
  ProjectAccountRecord,
  ProjectJobRecord,
  ProjectModuleRecord,
  ProjectReleaseRecord,
  ProjectTaskRecord,
  ProjectWorkflowRecord,
  ProxyConfig,
  ProxyProbeReport,
  RecordedWorkflowStep,
  StepExecutionResult,
  StreamFrameResponse,
  WorkflowReplayReport,
  ReconcileOutcome,
  ReleaseTransitionRecord,
  RoiRect,
  RunProjectWorkflowRequest,
  RunWorkflowExecutionSummary,
  RuntimeVariablesSnapshot,
  ScanRangeResult,
  SelectorMatch,
  StepEvent,
  TaskCategoryDef,
  TemplateMatchPreviewReport,
  UpdateOutcome,
  UpdateProjectRequest,
  ValidationReport,
  VisionTemplateRecord,
} from '../types';

export const isTauri = (): boolean => {
  return typeof window !== 'undefined' && ('__TAURI_INTERNALS__' in window || '__TAURI__' in window);
};

export interface ExecuteWorkflowResponse {
  job_id: string;
  state: string;
  events_count: number;
}

export interface NodeTestResult {
  success: boolean;
  duration_ms: number;
  message: string;
  point_tapped?: { x: number; y: number } | null;
}

function requireTauri(cmd: string): void {
  if (!isTauri()) {
    throw new Error(`Tauri Desktop IPC '${cmd}' không khả dụng ở chế độ xem web thuần.`);
  }
}

const inMemoryElementLibrary: LibraryElementRecord[] = [
  {
    id: 'elem_saved_1',
    projectId: 'default_project',
    name: 'btn_browser_app',
    packageFilter: 'com.android.launcher3',
    primaryStrategy: 'native',
    selectorJson: JSON.stringify({ resource_id: null, text: 'Browser', class_name: 'android.widget.TextView' }),
    fallbackSelectorsJson: '[]',
    boundsJson: JSON.stringify({ x: 321, y: 822, width: 101, height: 135 }),
    version: 1,
    createdAt: '2026-11-25T03:00:00Z',
    updatedAt: '2026-11-25T03:00:00Z',
  },
  {
    id: 'elem_saved_2',
    projectId: 'default_project',
    name: 'ocr_reward_claim',
    packageFilter: 'com.demo.game',
    primaryStrategy: 'ocr',
    selectorJson: JSON.stringify({ ocr: { text: 'Nhận thưởng', min_confidence: 0.85 } }),
    fallbackSelectorsJson: '[]',
    boundsJson: JSON.stringify({ x: 180, y: 450, width: 180, height: 60 }),
    version: 1,
    createdAt: '2026-11-25T03:10:00Z',
    updatedAt: '2026-11-25T03:10:00Z',
  },
];

export const api = {
  // -------------------------------------------------------------------------
  // Window & Legacy V1 Commands
  // -------------------------------------------------------------------------
  async windowMinimize(): Promise<void> {
    if (!isTauri()) return;
    try {
      await invoke('app_window_minimize');
    } catch (e) {
      console.warn('Failed to minimize window:', e);
    }
  },

  async windowToggleMaximize(): Promise<void> {
    if (!isTauri()) return;
    try {
      await invoke('app_window_toggle_maximize');
    } catch (e) {
      console.warn('Failed to toggle maximize:', e);
    }
  },

  async windowClose(): Promise<void> {
    if (!isTauri()) return;
    try {
      await invoke('app_window_close');
    } catch (e) {
      console.warn('Failed to close window:', e);
    }
  },

  async getStatus(): Promise<AppStatus | null> {
    if (!isTauri()) return null;
    try {
      return await invoke<AppStatus>('app_get_status');
    } catch (e) {
      console.warn('Failed to call app_get_status:', e);
      return null;
    }
  },

  async listDevices(): Promise<DeviceSummary[]> {
    if (!isTauri()) return [];
    try {
      return await invoke<DeviceSummary[]>('list_devices');
    } catch (e) {
      console.warn('Failed to call list_devices:', e);
      return [];
    }
  },

  async inspectDevice(deviceId: string): Promise<DeviceDetail | null> {
    if (!isTauri()) return null;
    try {
      return await invoke<DeviceDetail>('inspect_device', { deviceId });
    } catch (e) {
      console.warn('Failed to call inspect_device:', e);
      return null;
    }
  },

  async captureDevice(deviceId: string): Promise<string | null> {
    if (!isTauri()) return null;
    try {
      return await invoke<string>('capture_device', { deviceId });
    } catch (e) {
      console.warn('Failed to call capture_device:', e);
      return null;
    }
  },

  async validateWorkflow(workflowJson: string): Promise<ValidationReport> {
    if (!isTauri()) {
      try {
        const doc = JSON.parse(workflowJson);
        const hasStart = doc.graph?.nodes?.some((n: any) => n.type === 'flow.start');
        const hasEnd = doc.graph?.nodes?.some((n: any) => n.type === 'flow.end');
        const errors: string[] = [];
        if (!hasStart) errors.push("Thiếu node 'flow.start' (Bắt đầu)");
        if (!hasEnd) errors.push("Thiếu node 'flow.end' (Kết thúc)");
        return { is_valid: errors.length === 0, errors, warnings: [] };
      } catch (err: any) {
        return { is_valid: false, errors: [err.message], warnings: [] };
      }
    }
    return await invoke<ValidationReport>('validate_workflow', { workflowJson });
  },

  async executeWorkflow(
    workflowJson: string,
    deviceId: string,
    mock: boolean = true
  ): Promise<ExecuteWorkflowResponse> {
    requireTauri('execute_workflow');
    return await invoke<ExecuteWorkflowResponse>('execute_workflow', { workflowJson, deviceId, mock });
  },

  async executeTestNode(
    deviceId: string,
    nodeType: string,
    configJson: string
  ): Promise<NodeTestResult> {
    requireTauri('execute_test_node');
    return await invoke<NodeTestResult>('execute_test_node', {
      deviceId,
      nodeType,
      configJson,
    });
  },

  async getAuditEvents(jobId?: string): Promise<StepEvent[]> {
    if (!isTauri()) return [];
    try {
      return await invoke<StepEvent[]>('get_audit_events', { jobId });
    } catch (e) {
      console.warn('Failed to call get_audit_events:', e);
      return [];
    }
  },

  async saveWorkflowDraft(workflowId: string, name: string, draftJson: string): Promise<boolean> {
    if (!isTauri()) return false;
    try {
      return await invoke<boolean>('save_workflow_draft', { workflowId, name, draftJson });
    } catch (e) {
      console.warn('Failed to call save_workflow_draft:', e);
      return false;
    }
  },

  async getWorkflowDraft(workflowId: string): Promise<string | null> {
    if (!isTauri()) return null;
    try {
      return await invoke<string | null>('get_workflow_draft', { workflowId });
    } catch (e) {
      console.warn('Failed to call get_workflow_draft:', e);
      return null;
    }
  },

  async adbConnectEndpoint(endpoint: string): Promise<string> {
    requireTauri('adb_connect_endpoint');
    return await invoke<string>('adb_connect_endpoint', { endpoint });
  },

  async adbDisconnectEndpoint(endpoint: string): Promise<string> {
    requireTauri('adb_disconnect_endpoint');
    return await invoke<string>('adb_disconnect_endpoint', { endpoint });
  },

  async adbScanRange(
    subnet?: string,
    port?: number,
    emulatorPorts?: boolean
  ): Promise<ScanRangeResult> {
    if (!isTauri()) {
      return { scanned_count: 0, connected_endpoints: [] };
    }
    return await invoke<ScanRangeResult>('adb_scan_range', {
      subnet,
      port,
      emulatorPorts,
    });
  },

  async autoDiscoverDevices(): Promise<DeviceSummary[]> {
    if (!isTauri()) return [];
    return await invoke<DeviceSummary[]>('auto_discover_devices');
  },

  async listDevicePackages(deviceId: string, userOnly: boolean = true): Promise<AppPackageInfo[]> {
    if (!isTauri()) return [];
    return await invoke<AppPackageInfo[]>('list_device_packages', {
      deviceId,
      userOnly,
    });
  },

  async launchDeviceApp(deviceId: string, packageName: string): Promise<string> {
    requireTauri('launch_device_app');
    return await invoke<string>('launch_device_app', { deviceId, packageName });
  },

  async stopDeviceApp(deviceId: string, packageName: string): Promise<string> {
    requireTauri('stop_device_app');
    return await invoke<string>('stop_device_app', { deviceId, packageName });
  },

  async isDeviceAppRunning(deviceId: string, packageName: string): Promise<boolean> {
    if (!isTauri()) return false;
    return await invoke<boolean>('is_device_app_running', { deviceId, packageName });
  },

  async ldplayerListInstances(): Promise<LdPlayerInstance[]> {
    if (!isTauri()) return [];
    try {
      return await invoke<LdPlayerInstance[]>('ldplayer_list_instances');
    } catch (e) {
      console.warn('Failed to call ldplayer_list_instances:', e);
      return [];
    }
  },

  async ldplayerAction(action: string, index?: number): Promise<string> {
    requireTauri('ldplayer_action');
    return await invoke<string>('ldplayer_action', { action, index });
  },

  async ldplayerDetectPath(): Promise<string | null> {
    if (!isTauri()) return null;
    try {
      return await invoke<string | null>('ldplayer_detect_path');
    } catch (e) {
      console.warn('Failed to call ldplayer_detect_path:', e);
      return null;
    }
  },

  // -------------------------------------------------------------------------
  // P2 Studio Multi-Project, Modules, Tasks, UI Schema, Vault & DB Integrity
  // -------------------------------------------------------------------------
  async studioListProjects(): Promise<GameProjectSummary[]> {
    if (!isTauri()) return [];
    return await invoke<GameProjectSummary[]>('studio_list_projects');
  },

  async studioGetProjectDetail(projectId: string): Promise<GameProjectDetail | null> {
    if (!isTauri()) return null;
    return await invoke<GameProjectDetail | null>('studio_get_project_detail', { projectId });
  },

  async studioCreateProject(req: CreateProjectRequest): Promise<GameProjectRecord> {
    requireTauri('studio_create_project');
    return await invoke<GameProjectRecord>('studio_create_project', { req });
  },

  async studioUpdateProject(req: UpdateProjectRequest): Promise<GameProjectRecord> {
    requireTauri('studio_update_project');
    return await invoke<GameProjectRecord>('studio_update_project', { req });
  },

  async studioCloneProject(req: CloneProjectRequest): Promise<GameProjectRecord> {
    requireTauri('studio_clone_project');
    return await invoke<GameProjectRecord>('studio_clone_project', { req });
  },

  async studioDeleteProject(projectId: string, removeDiskDir: boolean = true): Promise<boolean> {
    requireTauri('studio_delete_project');
    return await invoke<boolean>('studio_delete_project', { projectId, removeDiskDir });
  },

  async studioListProjectWorkflows(projectId: string): Promise<ProjectWorkflowRecord[]> {
    if (!isTauri()) return [];
    return await invoke<ProjectWorkflowRecord[]>('studio_list_project_workflows', { projectId });
  },

  async studioSaveProjectWorkflow(
    projectId: string,
    workflowId: string,
    name: string,
    category: string | undefined,
    draftJson: string
  ): Promise<ProjectWorkflowRecord> {
    requireTauri('studio_save_project_workflow');
    return await invoke<ProjectWorkflowRecord>('studio_save_project_workflow', {
      projectId,
      workflowId,
      name,
      category,
      draftJson,
    });
  },

  async studioListProjectTemplates(projectId: string): Promise<VisionTemplateRecord[]> {
    if (!isTauri()) return [];
    return await invoke<VisionTemplateRecord[]>('studio_list_project_templates', { projectId });
  },

  async studioListProjectTasks(projectId: string): Promise<ProjectTaskRecord[]> {
    if (!isTauri()) return [];
    return await invoke<ProjectTaskRecord[]>('studio_list_project_tasks', { projectId });
  },

  async studioUpsertProjectTask(
    projectId: string,
    task: MiniTaskDef,
    categoryMeta: TaskCategoryDef
  ): Promise<ProjectTaskRecord> {
    requireTauri('studio_upsert_project_task');
    return await invoke<ProjectTaskRecord>('studio_upsert_project_task', {
      projectId,
      task,
      categoryMeta,
    });
  },

  async studioDeleteProjectTask(projectId: string, taskId: string): Promise<boolean> {
    requireTauri('studio_delete_project_task');
    return await invoke<boolean>('studio_delete_project_task', { projectId, taskId });
  },

  async studioListProjectModules(projectId: string): Promise<ProjectModuleRecord[]> {
    if (!isTauri()) return [];
    return await invoke<ProjectModuleRecord[]>('studio_list_project_modules', { projectId });
  },

  async studioUpsertProjectModule(
    projectId: string,
    moduleId: string,
    packagedInRelease: boolean,
    enabledByDefault: boolean,
    configJson: string
  ): Promise<ProjectModuleRecord> {
    requireTauri('studio_upsert_project_module');
    return await invoke<ProjectModuleRecord>('studio_upsert_project_module', {
      projectId,
      moduleId,
      packagedInRelease,
      enabledByDefault,
      configJson,
    });
  },

  async studioGetUiSchema(projectId: string): Promise<MiniToolUiSchema | null> {
    if (!isTauri()) return null;
    return await invoke<MiniToolUiSchema | null>('studio_get_ui_schema', { projectId });
  },

  async studioSaveUiSchema(projectId: string, schema: MiniToolUiSchema): Promise<void> {
    requireTauri('studio_save_ui_schema');
    await invoke('studio_save_ui_schema', { projectId, schema });
  },

  async studioListProjectAccounts(projectId: string): Promise<ProjectAccountRecord[]> {
    if (!isTauri()) return [];
    return await invoke<ProjectAccountRecord[]>('studio_list_project_accounts', { projectId });
  },

  async studioUpsertProjectAccount(input: ProjectAccountInput): Promise<ProjectAccountRecord> {
    requireTauri('studio_upsert_project_account');
    return await invoke<ProjectAccountRecord>('studio_upsert_project_account', { input });
  },

  async studioVaultSetSecret(
    scope: string,
    slug: string | undefined,
    key: string,
    secret: string
  ): Promise<string> {
    requireTauri('studio_vault_set_secret');
    return await invoke<string>('studio_vault_set_secret', { scope, slug, key, secret });
  },

  async studioVaultHasSecret(
    scope: string,
    slug: string | undefined,
    key: string
  ): Promise<boolean> {
    if (!isTauri()) return false;
    return await invoke<boolean>('studio_vault_has_secret', { scope, slug, key });
  },

  async studioVaultListNamespace(
    scope: string,
    slug: string | undefined
  ): Promise<CredentialMetadata[]> {
    if (!isTauri()) return [];
    return await invoke<CredentialMetadata[]>('studio_vault_list_namespace', { scope, slug });
  },

  async studioVaultDeleteSecret(
    scope: string,
    slug: string | undefined,
    key: string
  ): Promise<boolean> {
    requireTauri('studio_vault_delete_secret');
    return await invoke<boolean>('studio_vault_delete_secret', { scope, slug, key });
  },

  async studioGetProducerKeyStatus(): Promise<ProducerKeyMetadata | null> {
    if (!isTauri()) return null;
    return await invoke<ProducerKeyMetadata>('studio_get_producer_key_status');
  },

  async studioRotateProducerKey(reason?: string): Promise<ProducerKeyMetadata> {
    requireTauri('studio_rotate_producer_key');
    return await invoke<ProducerKeyMetadata>('studio_rotate_producer_key', { reason });
  },

  async studioExportPortableProducerKey(backupPath: string, passphrase: string): Promise<any> {
    requireTauri('studio_export_portable_producer_key');
    return await invoke('studio_export_portable_producer_key', { backupPath, passphrase });
  },

  async studioImportPortableProducerKey(
    backupPath: string,
    passphrase: string
  ): Promise<ProducerKeyMetadata> {
    requireTauri('studio_import_portable_producer_key');
    return await invoke<ProducerKeyMetadata>('studio_import_portable_producer_key', {
      backupPath,
      passphrase,
    });
  },

  async studioBackupDatabase(destPath: string): Promise<BackupReport> {
    requireTauri('studio_backup_database');
    return await invoke<BackupReport>('studio_backup_database', { destPath });
  },

  async studioCheckDbIntegrity(): Promise<IntegrityReport | null> {
    if (!isTauri()) return null;
    return await invoke<IntegrityReport>('studio_check_db_integrity');
  },

  async studioOptimizeDb(): Promise<DbOptimizeReport> {
    requireTauri('studio_optimize_db');
    return await invoke<DbOptimizeReport>('studio_optimize_db');
  },

  async studioMigrateLegacyJx2(legacyJx2Dir: string): Promise<Jx2MigrationSummary> {
    requireTauri('studio_migrate_legacy_jx2');
    return await invoke<Jx2MigrationSummary>('studio_migrate_legacy_jx2', { legacyJx2Dir });
  },

  // -------------------------------------------------------------------------
  // P3 Studio Runtime Coordinator, Device Lab, Vision ROI, Proxy & Builder
  // -------------------------------------------------------------------------
  async studioRunProjectWorkflow(
    req: RunProjectWorkflowRequest
  ): Promise<RunWorkflowExecutionSummary> {
    requireTauri('studio_run_project_workflow');
    return await invoke<RunWorkflowExecutionSummary>('studio_run_project_workflow', { req });
  },

  async studioStartProjectWorkflow(
    req: RunProjectWorkflowRequest
  ): Promise<BackgroundJobStarted> {
    requireTauri('studio_start_project_workflow');
    return await invoke<BackgroundJobStarted>('studio_start_project_workflow', { req });
  },

  async studioPauseJob(jobId: string): Promise<void> {
    requireTauri('studio_pause_job');
    await invoke('studio_pause_job', { jobId });
  },

  async studioResumeJob(jobId: string): Promise<void> {
    requireTauri('studio_resume_job');
    await invoke('studio_resume_job', { jobId });
  },

  async studioSuspendJobToCheckpoint(jobId: string): Promise<void> {
    requireTauri('studio_suspend_job_to_checkpoint');
    await invoke('studio_suspend_job_to_checkpoint', { jobId });
  },

  async studioCancelJob(jobId: string): Promise<void> {
    requireTauri('studio_cancel_job');
    await invoke('studio_cancel_job', { jobId });
  },

  async studioResumeJobFromCheckpoint(
    jobId: string,
    leaseTtlSecs?: number,
    workflowTimeoutMs?: number
  ): Promise<RunWorkflowExecutionSummary> {
    requireTauri('studio_resume_job_from_checkpoint');
    return await invoke<RunWorkflowExecutionSummary>('studio_resume_job_from_checkpoint', {
      jobId,
      leaseTtlSecs,
      workflowTimeoutMs,
    });
  },

  async studioListProjectJobs(projectId: string): Promise<ProjectJobRecord[]> {
    if (!isTauri()) return [];
    return await invoke<ProjectJobRecord[]>('studio_list_project_jobs', { projectId });
  },

  async studioGetJobResumeDetail(jobId: string): Promise<JobResumeDetail | null> {
    if (!isTauri()) return null;
    return await invoke<JobResumeDetail | null>('studio_get_job_resume_detail', { jobId });
  },

  async studioListActiveDeviceLeases(): Promise<ActiveDeviceLeaseView[]> {
    if (!isTauri()) return [];
    return await invoke<ActiveDeviceLeaseView[]>('studio_list_active_device_leases');
  },

  async studioProbeDeviceHealth(serial: string): Promise<DeviceHealthReport> {
    requireTauri('studio_probe_device_health');
    return await invoke<DeviceHealthReport>('studio_probe_device_health', { serial });
  },

  async studioReconnectDevice(endpoint: string): Promise<DeviceHealthReport> {
    requireTauri('studio_reconnect_device');
    return await invoke<DeviceHealthReport>('studio_reconnect_device', { endpoint });
  },

  async studioListDevicePorts(serial: string): Promise<AdbPortMappingsOverview> {
    requireTauri('studio_list_device_ports');
    return await invoke<AdbPortMappingsOverview>('studio_list_device_ports', { serial });
  },

  async studioAddDeviceForward(
    serial: string,
    localSpec: string,
    remoteSpec: string
  ): Promise<DevicePortMapping> {
    requireTauri('studio_add_device_forward');
    return await invoke<DevicePortMapping>('studio_add_device_forward', {
      serial,
      localSpec,
      remoteSpec,
    });
  },

  async studioRemoveDeviceForward(serial: string, localSpec: string): Promise<void> {
    requireTauri('studio_remove_device_forward');
    await invoke('studio_remove_device_forward', { serial, localSpec });
  },

  async studioAddDeviceReverse(
    serial: string,
    remoteSpec: string,
    localSpec: string
  ): Promise<DevicePortMapping> {
    requireTauri('studio_add_device_reverse');
    return await invoke<DevicePortMapping>('studio_add_device_reverse', {
      serial,
      remoteSpec,
      localSpec,
    });
  },

  async studioRemoveDeviceReverse(serial: string, remoteSpec: string): Promise<void> {
    requireTauri('studio_remove_device_reverse');
    await invoke('studio_remove_device_reverse', { serial, remoteSpec });
  },

  async studioCropAndSaveTemplate(params: {
    projectId: string;
    name: string;
    relPath: string;
    framePngBase64: string;
    roi: RoiRect;
    defaultConfidence?: number;
  }): Promise<VisionTemplateRecord> {
    requireTauri('studio_crop_and_save_template');
    return await invoke<VisionTemplateRecord>('studio_crop_and_save_template', params);
  },

  async studioCaptureAndCropTemplate(params: {
    projectId: string;
    serial: string;
    name: string;
    relPath: string;
    roi: RoiRect;
    defaultConfidence?: number;
  }): Promise<VisionTemplateRecord> {
    requireTauri('studio_capture_and_crop_template');
    return await invoke<VisionTemplateRecord>('studio_capture_and_crop_template', params);
  },

  async studioTestProjectTemplateMatch(params: {
    projectId: string;
    templateId: string;
    framePngBase64: string;
    frameWidth: number;
    frameHeight: number;
    minConfidence?: number;
    saveAnnotatedRelPath?: string;
  }): Promise<TemplateMatchPreviewReport> {
    requireTauri('studio_test_project_template_match');
    return await invoke<TemplateMatchPreviewReport>('studio_test_project_template_match', params);
  },

  async studioScaleRoiForGeometry(params: {
    bounds: Bounds;
    fromWidth: number;
    fromHeight: number;
    toWidth: number;
    toHeight: number;
  }): Promise<Bounds> {
    requireTauri('studio_scale_roi_for_geometry');
    return await invoke<Bounds>('studio_scale_roi_for_geometry', params);
  },

  async studioProbeProjectProxy(projectId: string, config: ProxyConfig): Promise<ProxyProbeReport> {
    requireTauri('studio_probe_project_proxy');
    return await invoke<ProxyProbeReport>('studio_probe_project_proxy', { projectId, config });
  },

  async studioBuilderPreflight(
    projectId: string,
    runtimeExeOverride?: string
  ): Promise<BuildPreflightReport> {
    requireTauri('studio_builder_preflight');
    return await invoke<BuildPreflightReport>('studio_builder_preflight', {
      projectId,
      runtimeExeOverride,
    });
  },

  async studioBuilderExportRelease(
    projectId: string,
    outputDir?: string,
    runtimeExeOverride?: string,
    options?: import('../types').BuilderExportOptions
  ): Promise<BuilderReleaseExportResult> {
    requireTauri('studio_builder_export_release');
    return await invoke<BuilderReleaseExportResult>('studio_builder_export_release', {
      projectId,
      outputDir,
      runtimeExeOverride,
      options,
    });
  },

  async studioListPublisherProfiles(): Promise<import('../types').PublisherProfileRecord[]> {
    if (!isTauri()) return [];
    return await invoke('studio_list_publisher_profiles');
  },

  async studioUpsertPublisherProfile(
    input: import('../types').PublisherProfileInput
  ): Promise<import('../types').PublisherProfileRecord> {
    requireTauri('studio_upsert_publisher_profile');
    return await invoke('studio_upsert_publisher_profile', { input });
  },

  async studioDeletePublisherProfile(id: string): Promise<boolean> {
    requireTauri('studio_delete_publisher_profile');
    return await invoke('studio_delete_publisher_profile', { id });
  },

  async studioCreateSelfSignedQaCert(
    subjectCn?: string,
    validDays?: number
  ): Promise<import('../types').QaSelfSignedCertInfo> {
    requireTauri('studio_create_self_signed_qa_cert');
    return await invoke('studio_create_self_signed_qa_cert', { subjectCn, validDays });
  },

  async studioSigningPreflight(
    projectId: string,
    signingMode?: string,
    certRef?: string
  ): Promise<import('../types').SigningPreflightReport> {
    requireTauri('studio_signing_preflight');
    return await invoke('studio_signing_preflight', { projectId, signingMode, certRef });
  },

  async studioDiscoverSigningToolchain(): Promise<import('../types').SigningToolchainStatus> {
    requireTauri('studio_discover_signing_toolchain');
    return await invoke('studio_discover_signing_toolchain');
  },

  async studioVerifyFinalRelease(
    releaseDir: string,
    requirePublicChain = false
  ): Promise<import('../types').FinalReleaseVerificationReport> {
    requireTauri('studio_verify_final_release');
    return await invoke('studio_verify_final_release', { releaseDir, requirePublicChain });
  },

  async studioRotateProjectEd25519KeyWithDelegation(
    projectId: string,
    publisherId?: string,
    validDays?: number
  ): Promise<import('../types').KeyRotationWithDelegationResult> {
    requireTauri('studio_rotate_project_ed25519_key_with_delegation');
    return await invoke('studio_rotate_project_ed25519_key_with_delegation', {
      projectId,
      publisherId,
      validDays,
    });
  },

  async studioEnsureProjectIcon(params: {
    projectId: string;
    iconBase64?: string;
    accentHex?: string;
    overwrite?: boolean;
  }): Promise<string> {
    requireTauri('studio_ensure_project_icon');
    return await invoke<string>('studio_ensure_project_icon', params);
  },

  async studioGetProjectRuntimeVars(projectId: string): Promise<RuntimeVariablesSnapshot | null> {
    if (!isTauri()) return null;
    return await invoke<RuntimeVariablesSnapshot>('studio_get_project_runtime_vars', { projectId });
  },

  async studioListProjectReleases(projectId: string): Promise<ProjectReleaseRecord[]> {
    if (!isTauri()) return [];
    return await invoke<ProjectReleaseRecord[]>('studio_list_project_releases', { projectId });
  },

  async studioCheckReleaseCompatibility(projectId: string): Promise<CompatibilityReport> {
    requireTauri('studio_check_release_compatibility');
    return await invoke<CompatibilityReport>('studio_check_release_compatibility', { projectId });
  },

  async studioTransitionReleaseLifecycle(params: {
    projectId: string;
    releaseId: string;
    targetState: string;
    note?: string;
  }): Promise<ProjectReleaseRecord> {
    requireTauri('studio_transition_release_lifecycle');
    return await invoke<ProjectReleaseRecord>('studio_transition_release_lifecycle', params);
  },

  async studioListProjectReleaseTransitions(
    projectId: string
  ): Promise<ReleaseTransitionRecord[]> {
    if (!isTauri()) return [];
    return await invoke<ReleaseTransitionRecord[]>('studio_list_project_release_transitions', {
      projectId,
    });
  },

  async studioDeployMinitoolRelease(
    installRoot: string,
    releaseDir: string
  ): Promise<UpdateOutcome> {
    requireTauri('studio_deploy_minitool_release');
    return await invoke<UpdateOutcome>('studio_deploy_minitool_release', {
      installRoot,
      releaseDir,
    });
  },

  async studioRollbackMinitoolInstall(
    installRoot: string,
    restoreBackup = false
  ): Promise<UpdateOutcome> {
    requireTauri('studio_rollback_minitool_install');
    return await invoke<UpdateOutcome>('studio_rollback_minitool_install', {
      installRoot,
      restoreBackup,
    });
  },

  async studioReconcileMinitoolInstall(installRoot: string): Promise<ReconcileOutcome> {
    requireTauri('studio_reconcile_minitool_install');
    return await invoke<ReconcileOutcome>('studio_reconcile_minitool_install', { installRoot });
  },

  async studioInspectMinitoolInstall(installRoot: string): Promise<ManagedInstallInspection> {
    requireTauri('studio_inspect_minitool_install');
    return await invoke<ManagedInstallInspection>('studio_inspect_minitool_install', {
      installRoot,
    });
  },

  // --- Element Inspector & Smart Action API ---
  async elementInspectSnapshot(serial: string): Promise<InspectSnapshotResult> {
    if (!isTauri()) {
      const nodes: ElementNode[] = [
        {
          id: 0,
          parent_id: null,
          children_ids: [1],
          index: 0,
          text: '',
          resource_id: '',
          class_name: 'android.widget.FrameLayout',
          package: 'com.android.launcher3',
          content_desc: '',
          checkable: false,
          checked: false,
          clickable: false,
          enabled: true,
          focusable: false,
          focused: false,
          scrollable: false,
          long_clickable: false,
          password: false,
          selected: false,
          bounds: { x: 0, y: 0, width: 540, height: 960 },
          raw_bounds: { x: 0, y: 0, width: 540, height: 960 },
          depth: 0,
          path: '/hierarchy/android.widget.FrameLayout[0]',
          is_visible: true,
        },
        {
          id: 1,
          parent_id: 0,
          children_ids: [2, 3, 4, 5, 6, 7, 8, 9],
          index: 0,
          text: '',
          resource_id: 'com.android.launcher3:id/drag_layer',
          class_name: 'android.widget.FrameLayout',
          package: 'com.android.launcher3',
          content_desc: '',
          checkable: false,
          checked: false,
          clickable: false,
          enabled: true,
          focusable: false,
          focused: false,
          scrollable: false,
          long_clickable: false,
          password: false,
          selected: false,
          bounds: { x: 0, y: 0, width: 540, height: 960 },
          raw_bounds: { x: 0, y: 0, width: 540, height: 960 },
          depth: 1,
          path: '/hierarchy/android.widget.FrameLayout[0]/FrameLayout[0]',
          is_visible: true,
        },
        {
          id: 2,
          parent_id: 1,
          children_ids: [],
          index: 0,
          text: '10:16',
          resource_id: 'com.android.deskclock:id/clock',
          class_name: 'android.widget.TextView',
          package: 'com.android.deskclock',
          content_desc: '10:16',
          checkable: false,
          checked: false,
          clickable: false,
          enabled: true,
          focusable: false,
          focused: false,
          scrollable: false,
          long_clickable: false,
          password: false,
          selected: false,
          bounds: { x: 99, y: 57, width: 342, height: 169 },
          raw_bounds: { x: 99, y: 57, width: 342, height: 169 },
          depth: 2,
          path: '/hierarchy/clock',
          is_visible: true,
        },
        {
          id: 3,
          parent_id: 1,
          children_ids: [],
          index: 1,
          text: 'Phone',
          resource_id: '',
          class_name: 'android.widget.TextView',
          package: 'com.android.launcher3',
          content_desc: 'Phone',
          checkable: false,
          checked: false,
          clickable: true,
          enabled: true,
          focusable: true,
          focused: false,
          scrollable: false,
          long_clickable: true,
          password: false,
          selected: false,
          bounds: { x: 18, y: 335, width: 101, height: 145 },
          raw_bounds: { x: 18, y: 335, width: 101, height: 145 },
          depth: 2,
          path: '/hierarchy/phone',
          is_visible: true,
        },
        {
          id: 4,
          parent_id: 1,
          children_ids: [],
          index: 2,
          text: 'Play Store',
          resource_id: '',
          class_name: 'android.widget.TextView',
          package: 'com.android.launcher3',
          content_desc: 'Play Store',
          checkable: false,
          checked: false,
          clickable: true,
          enabled: true,
          focusable: true,
          focused: false,
          scrollable: false,
          long_clickable: true,
          password: false,
          selected: false,
          bounds: { x: 220, y: 335, width: 101, height: 145 },
          raw_bounds: { x: 220, y: 335, width: 101, height: 145 },
          depth: 2,
          path: '/hierarchy/playstore',
          is_visible: true,
        },
        {
          id: 5,
          parent_id: 1,
          children_ids: [],
          index: 3,
          text: 'Võ Hiệp Truyền Kỳ 2',
          resource_id: '',
          class_name: 'android.widget.TextView',
          package: 'com.android.launcher3',
          content_desc: 'Võ Hiệp Truyền Kỳ 2',
          checkable: false,
          checked: false,
          clickable: true,
          enabled: true,
          focusable: true,
          focused: false,
          scrollable: false,
          long_clickable: true,
          password: false,
          selected: false,
          bounds: { x: 119, y: 480, width: 101, height: 145 },
          raw_bounds: { x: 119, y: 480, width: 101, height: 145 },
          depth: 2,
          path: '/hierarchy/game',
          is_visible: true,
        },
        {
          id: 6,
          parent_id: 1,
          children_ids: [],
          index: 4,
          text: 'Messaging',
          resource_id: '',
          class_name: 'android.widget.TextView',
          package: 'com.android.launcher3',
          content_desc: 'Messaging',
          checkable: false,
          checked: false,
          clickable: true,
          enabled: true,
          focusable: true,
          focused: false,
          scrollable: false,
          long_clickable: true,
          password: false,
          selected: false,
          bounds: { x: 119, y: 822, width: 101, height: 135 },
          raw_bounds: { x: 119, y: 822, width: 101, height: 135 },
          depth: 2,
          path: '/hierarchy/messaging',
          is_visible: true,
        },
        {
          id: 7,
          parent_id: 1,
          children_ids: [],
          index: 5,
          text: 'Contacts',
          resource_id: '',
          class_name: 'android.widget.TextView',
          package: 'com.android.launcher3',
          content_desc: 'Contacts',
          checkable: false,
          checked: false,
          clickable: true,
          enabled: true,
          focusable: true,
          focused: false,
          scrollable: false,
          long_clickable: true,
          password: false,
          selected: false,
          bounds: { x: 220, y: 822, width: 101, height: 135 },
          raw_bounds: { x: 220, y: 822, width: 101, height: 135 },
          depth: 2,
          path: '/hierarchy/contacts',
          is_visible: true,
        },
        {
          id: 8,
          parent_id: 1,
          children_ids: [10],
          index: 6,
          text: 'Browser',
          resource_id: 'com.android.launcher3:id/icon_browser',
          class_name: 'android.widget.FrameLayout',
          package: 'com.android.launcher3',
          content_desc: 'Browser',
          checkable: false,
          checked: false,
          clickable: true,
          enabled: true,
          focusable: true,
          focused: false,
          scrollable: false,
          long_clickable: true,
          password: false,
          selected: false,
          bounds: { x: 321, y: 822, width: 101, height: 135 },
          raw_bounds: { x: 321, y: 822, width: 101, height: 135 },
          depth: 2,
          path: '/hierarchy/browser_frame',
          is_visible: true,
        },
        {
          id: 9,
          parent_id: 1,
          children_ids: [],
          index: 7,
          text: 'Camera',
          resource_id: '',
          class_name: 'android.widget.TextView',
          package: 'com.android.launcher3',
          content_desc: 'Camera',
          checkable: false,
          checked: false,
          clickable: true,
          enabled: true,
          focusable: true,
          focused: false,
          scrollable: false,
          long_clickable: true,
          password: false,
          selected: false,
          bounds: { x: 422, y: 822, width: 100, height: 135 },
          raw_bounds: { x: 422, y: 822, width: 100, height: 135 },
          depth: 2,
          path: '/hierarchy/camera',
          is_visible: true,
        },
        {
          id: 10,
          parent_id: 8,
          children_ids: [],
          index: 0,
          text: 'Browser',
          resource_id: 'com.android.launcher3:id/title',
          class_name: 'android.widget.TextView',
          package: 'com.android.launcher3',
          content_desc: 'Browser',
          checkable: false,
          checked: false,
          clickable: false,
          enabled: true,
          focusable: false,
          focused: false,
          scrollable: false,
          long_clickable: false,
          password: false,
          selected: false,
          bounds: { x: 330, y: 910, width: 83, height: 35 },
          raw_bounds: { x: 330, y: 910, width: 83, height: 35 },
          depth: 3,
          path: '/hierarchy/browser_frame/title',
          is_visible: true,
        },
      ];

      return {
        snapshot: {
          id: 'snap_galaxy_s7_live',
          timestamp_ms: Date.now(),
          rotation_deg: 0,
          screen_width: 540,
          screen_height: 960,
          package_name: 'com.android.launcher3',
          nodes,
          root_ids: [0],
        },
        screenshot_base64: '/live_device_screen.png',
        capture_duration_ms: 38,
        dump_duration_ms: 104,
        parse_duration_ms: 2,
        total_duration_ms: 144,
        device_serial: serial,
        package_name: 'com.android.launcher3',
      };
    }
    requireTauri('element_inspect_snapshot');
    return await invoke<InspectSnapshotResult>('element_inspect_snapshot', { serial });
  },

  async elementFindMatches(
    snapshot: ElementSnapshot,
    selector: NativeElementSelector,
    requireUnique = true
  ): Promise<SelectorMatch[]> {
    if (!isTauri()) {
      const matchNode = snapshot.nodes.find(
        (n) => (selector.text && n.text === selector.text) || (selector.resource_id && n.resource_id === selector.resource_id)
      ) || snapshot.nodes[8];
      return [
        {
          node_id: matchNode.id,
          bounds: matchNode.bounds,
          confidence: 1.0,
          strategy_used: 'MockNativeMatch',
          is_clickable: matchNode.clickable,
          resolved_clickable_ancestor_id: matchNode.clickable ? null : matchNode.parent_id,
          resolved_bounds: matchNode.bounds,
        },
      ];
    }
    requireTauri('element_find_matches');
    return await invoke<SelectorMatch[]>('element_find_matches', {
      snapshot,
      selector,
      requireUnique,
    });
  },

  async elementGenerateSelectors(
    snapshot: ElementSnapshot,
    nodeId: number
  ): Promise<GeneratedSelector[]> {
    if (!isTauri()) {
      const node = snapshot.nodes[nodeId] || snapshot.nodes[0];
      const selectors: GeneratedSelector[] = [];
      if (node.resource_id) {
        selectors.push({
          strategy_name: 'Resource ID',
          selector: { resource_id: node.resource_id },
          uniqueness_score: 100,
          match_count: 1,
          expression: `element.id("${node.resource_id}")`,
          is_recommended: true,
        });
      }
      if (node.text) {
        selectors.push({
          strategy_name: 'Exact Text',
          selector: { text: node.text },
          uniqueness_score: 100,
          match_count: 1,
          expression: `element.text("${node.text}")`,
          is_recommended: !node.resource_id,
        });
      }
      selectors.push({
        strategy_name: 'Class & Hierarchy',
        selector: { class_name: node.class_name },
        uniqueness_score: 85,
        match_count: 1,
        expression: `element.class("${node.class_name}")`,
        is_recommended: false,
      });
      selectors.push({
        strategy_name: 'Absolute XPath',
        selector: { xpath: node.path },
        uniqueness_score: 100,
        match_count: 1,
        expression: `xpath("${node.path}")`,
        is_recommended: false,
      });
      return selectors;
    }
    requireTauri('element_generate_selectors');
    return await invoke<GeneratedSelector[]>('element_generate_selectors', {
      snapshot,
      nodeId,
    });
  },

  async elementFindAtPoint(
    snapshot: ElementSnapshot,
    x: number,
    y: number
  ): Promise<ElementNode | null> {
    if (!isTauri()) {
      let candidate: ElementNode | null = null;
      let minArea = Infinity;
      for (const n of snapshot.nodes) {
        if (
          n.is_visible &&
          x >= n.bounds.x &&
          x <= n.bounds.x + n.bounds.width &&
          y >= n.bounds.y &&
          y <= n.bounds.y + n.bounds.height
        ) {
          const area = n.bounds.width * n.bounds.height;
          if (area < minArea) {
            minArea = area;
            candidate = n;
          }
        }
      }
      return candidate;
    }
    requireTauri('element_find_at_point');
    return await invoke<ElementNode | null>('element_find_at_point', {
      snapshot,
      x,
      y,
    });
  },

  async elementExecuteAction(
    serial: string,
    matchItem: SelectorMatch,
    actionKind: ElementActionKind,
    mode: ActionExecutionMode = 'ClickableParent'
  ): Promise<ElementActionResult> {
    if (!isTauri()) {
      const safeX = matchItem.bounds.x + matchItem.bounds.width * 0.5;
      const safeY = matchItem.bounds.y + matchItem.bounds.height * 0.5;
      return {
        success: true,
        execution_ms: 38,
        plan: {
          mode,
          target_bounds: matchItem.bounds,
          safe_point: { x: safeX, y: safeY },
          is_clickable_parent_used: mode === 'ClickableParent' && !matchItem.is_clickable,
          execution_kind: actionKind,
        },
        new_screenshot_base64: null,
        post_verification_success: true,
        message: `Action executed at (${Math.round(safeX)}, ${Math.round(safeY)}) via ${mode}`,
      };
    }
    requireTauri('element_execute_action');
    return await invoke<ElementActionResult>('element_execute_action', {
      serial,
      matchItem,
      actionKind,
      mode,
    });
  },

  async elementTestDetection(
    snapshot: ElementSnapshot | null,
    selector: any,
    requireUnique = true
  ): Promise<DetectionResult> {
    if (!isTauri()) {
      const targetBounds = { x: 321, y: 822, width: 101, height: 135 };
      return {
        target_bounds: targetBounds,
        resolved_bounds: targetBounds,
        confidence: 0.99,
        candidate_count: 1,
        strategy_used: 'Native: com.android.launcher3:id/icon_browser',
        is_clickable: true,
        resolved_clickable_ancestor_id: null,
        matches: [
          {
            node_id: 8,
            bounds: targetBounds,
            confidence: 0.99,
            strategy_used: 'Native',
            is_clickable: true,
            resolved_clickable_ancestor_id: null,
            resolved_bounds: targetBounds,
          },
        ],
      };
    }
    requireTauri('element_test_detection');
    return await invoke<DetectionResult>('element_test_detection', {
      snapshot,
      selector,
      requireUnique,
    });
  },

  async elementSaveToLibrary(record: LibraryElementRecord): Promise<void> {
    if (!isTauri()) {
      inMemoryElementLibrary.unshift(record);
      return;
    }
    requireTauri('element_save_to_library');
    return await invoke<void>('element_save_to_library', { record });
  },

  async elementListLibrary(projectId: string): Promise<LibraryElementRecord[]> {
    if (!isTauri()) {
      return [...inMemoryElementLibrary];
    }
    requireTauri('element_list_library');
    return await invoke<LibraryElementRecord[]>('element_list_library', { projectId });
  },

  async elementDeleteFromLibrary(id: string): Promise<void> {
    if (!isTauri()) {
      const idx = inMemoryElementLibrary.findIndex((e) => e.id === id);
      if (idx !== -1) inMemoryElementLibrary.splice(idx, 1);
      return;
    }
    requireTauri('element_delete_from_library');
    return await invoke<void>('element_delete_from_library', { id });
  },

  // ========================================================================
  // Phase 2.2: Device Studio Direct Hardware Control & Streaming
  // ========================================================================

  async devicePressKey(serial: string, keycode: number): Promise<ActionReceipt> {
    requireTauri('device_press_key');
    return await invoke<ActionReceipt>('device_press_key', { serial, keycode });
  },

  async deviceTap(serial: string, x: number, y: number): Promise<ActionReceipt> {
    requireTauri('device_tap');
    return await invoke<ActionReceipt>('device_tap', { serial, x, y });
  },

  async deviceSwipe(
    serial: string,
    startX: number,
    startY: number,
    endX: number,
    endY: number,
    durationMs: number
  ): Promise<ActionReceipt> {
    requireTauri('device_swipe');
    return await invoke<ActionReceipt>('device_swipe', {
      serial,
      startX,
      startY,
      endX,
      endY,
      durationMs,
    });
  },

  async deviceInputText(serial: string, text: string): Promise<ActionReceipt> {
    requireTauri('device_input_text');
    return await invoke<ActionReceipt>('device_input_text', { serial, text });
  },

  async deviceStreamFrame(serial: string): Promise<StreamFrameResponse> {
    requireTauri('device_stream_frame');
    return await invoke<StreamFrameResponse>('device_stream_frame', { serial });
  },

  // ========================================================================
  // Phase 2.2: Smart Workflow Recorder API
  // ========================================================================

  async recorderRecordTouch(
    serial: string,
    x: number,
    y: number,
    stepNumber: number,
    actionType: string,
    textInput?: string
  ): Promise<RecordedWorkflowStep> {
    requireTauri('recorder_record_touch');
    return await invoke<RecordedWorkflowStep>('recorder_record_touch', {
      serial,
      x,
      y,
      stepNumber,
      actionType,
      textInput,
    });
  },

  async recorderReplayStep(
    serial: string,
    step: RecordedWorkflowStep
  ): Promise<StepExecutionResult> {
    requireTauri('recorder_replay_step');
    return await invoke<StepExecutionResult>('recorder_replay_step', {
      serial,
      step,
    });
  },

  async recorderReplayAll(
    serial: string,
    steps: RecordedWorkflowStep[]
  ): Promise<WorkflowReplayReport> {
    requireTauri('recorder_replay_all');
    return await invoke<WorkflowReplayReport>('recorder_replay_all', {
      serial,
      steps,
    });
  },

  async recorderExportWorkflow(
    workflowId: string,
    name: string,
    steps: RecordedWorkflowStep[]
  ): Promise<any> {
    requireTauri('recorder_export_workflow');
    return await invoke<any>('recorder_export_workflow', {
      workflowId,
      name,
      steps,
    });
  },

  async recorderSaveWorkflow(
    workflowId: string,
    name: string,
    docJson: string
  ): Promise<boolean> {
    requireTauri('recorder_save_workflow');
    return await invoke<boolean>('recorder_save_workflow', {
      workflowId,
      name,
      docJson,
    });
  },
};
