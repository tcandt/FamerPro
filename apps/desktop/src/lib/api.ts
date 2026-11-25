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
  DeviceDetail,
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
    requireTauri('element_inspect_snapshot');
    return await invoke<InspectSnapshotResult>('element_inspect_snapshot', { serial });
  },

  async elementFindMatches(
    snapshot: ElementSnapshot,
    selector: NativeElementSelector,
    requireUnique = true
  ): Promise<SelectorMatch[]> {
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
    requireTauri('element_execute_action');
    return await invoke<ElementActionResult>('element_execute_action', {
      serial,
      matchItem,
      actionKind,
      mode,
    });
  },
};
