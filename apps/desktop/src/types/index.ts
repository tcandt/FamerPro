export type ConnectionState =
  | 'disconnected'
  | 'unauthorized'
  | 'authorizing'
  | 'ready'
  | 'busy'
  | 'recovering'
  | 'blocked';

export type RuntimeSourceStatus = 'LIVE' | 'OFFLINE' | 'TESTKIT' | 'ERROR';

export type StudioViewId =
  | 'projects'
  | 'studio'
  | 'tasks'
  | 'devices'
  | 'runs'
  | 'builder'
  | 'accounts'
  | 'network'
  | 'settings'
  | 'overview'
  | 'workflows'
  | 'gallery';

export interface DeviceSummary {
  id: string;
  adb_serial: string;
  alias: string;
  android_api?: number;
  model?: string;
  width_px: number;
  height_px: number;
  rotation_deg: number;
  connection_state: ConnectionState;
  updated_at: string;
  is_mock?: boolean;
}

export function formatAndroidVersion(apiLevel?: number | null): string {
  if (!apiLevel) return 'Android (Không rõ)';
  const apiMap: Record<number, string> = {
    35: 'Android 15 (API 35)',
    34: 'Android 14 (API 34)',
    33: 'Android 13 (API 33)',
    32: 'Android 12L (API 32)',
    31: 'Android 12 (API 31)',
    30: 'Android 11 (API 30)',
    29: 'Android 10 (API 29)',
    28: 'Android 9 (API 28)',
    27: 'Android 8.1 (API 27)',
    26: 'Android 8.0 (API 26)',
  };
  return apiMap[apiLevel] || `Android API ${apiLevel}`;
}

export interface DeviceDetail {
  summary: DeviceSummary;
  battery_level?: number;
  is_screen_on: boolean;
  ip_address?: string;
  installed_packages: string[];
}

export interface WorkflowNodeData extends Record<string, unknown> {
  label: string;
  nodeType: string;
  category: 'flow' | 'app' | 'vision' | 'action' | 'verify' | 'state' | 'logic';
  config: Record<string, any>;
  status?: 'idle' | 'running' | 'success' | 'failed';
  timeoutMs?: number;
}

export interface WorkflowEdgeData {
  portType: 'flow' | 'data' | 'boolean';
}

export interface ValidationReport {
  is_valid: boolean;
  errors: string[];
  warnings: string[];
}

export interface StepEvent {
  id: string;
  job_id: string;
  node_id: string;
  event_type:
  | 'command_sent'
  | 'device_ack'
  | 'verification_passed'
  | 'verification_failed'
  | 'checkpoint_saved'
  | 'unknown';
  attempt_no: number;
  at: string;
  payload_json: string;
}

export interface AppStatus {
  version: string;
  adb_available: boolean;
  adb_version: string;
  adb_path: string;
  db_status: string;
  db_path: string;
  devices_count: number;
  is_portable: boolean;
}

export interface AppPackageInfo {
  package_name: string;
  display_name: string;
  is_game: boolean;
  is_system: boolean;
}

export interface LdPlayerInstance {
  index: number;
  name: string;
  is_running: boolean;
  pid: number;
  top_handle: number;
  width: number;
  height: number;
  dpi: number;
}

export interface ScanRangeResult {
  scanned_count: number;
  connected_endpoints: string[];
}

// =========================================================================
// P2 Multi-Project, Bundle Contract & Security Types
// =========================================================================

export type MiniLayoutPreset = 'compact1_col' | 'compact2_col' | 'expanded';

export interface RuntimeColumnBinding {
  id: string;
  label: string;
  runtimeVarKey: string;
  fallbackDisplay: string;
  minWindowWidth: number;
  badgeColor?: string | null;
}

export interface TaskCategoryDef {
  id: string;
  shortTag: string;
  displayName: string;
  colorHex: string;
}

export interface TaskSettingField {
  key: string;
  label: string;
  fieldType: string;
  defaultValue: any;
  options?: string[] | null;
}

export interface MiniTaskDef {
  id: string;
  title: string;
  categoryId: string;
  workflowId: string;
  defaultEnabled: boolean;
  priority: number;
  estimateSec: number;
  settingsSchema: TaskSettingField[];
}

export interface UtilityToggleDef {
  id: string;
  label: string;
  defaultEnabled: boolean;
  requiredModule?: string | null;
  settingsSchema: TaskSettingField[];
}

export interface MiniToolUiSchema {
  schemaVersion: number;
  projectSlug: string;
  windowTitle: string;
  versionBadge: string;
  themeAccentHex: string;
  headerBgHex: string;
  layoutPreset: MiniLayoutPreset;
  defaultWindowSize: [number, number];
  sectionsOrder: string[];
  hiddenComponents: string[];
  deviceColumns: RuntimeColumnBinding[];
  taskCategories: TaskCategoryDef[];
  tasks: MiniTaskDef[];
  utilities: UtilityToggleDef[];
  initialRuntimeVars: Record<string, any>;
}

export interface PeVersionInfoSpec {
  companyName: string;
  fileDescription: string;
  fileVersion: string;
  internalName: string;
  legalCopyright: string;
  originalFilename: string;
  productName: string;
  productVersion: string;
}

export type SigningMode = 'unsigned_internal' | 'self_signed_qa' | 'public_signed';

export interface PublisherProfileRecord {
  id: string;
  publisherId: string;
  displayName: string;
  organization: string;
  website?: string | null;
  supportContact?: string | null;
  copyright: string;
  ed25519KeyId: string;
  ed25519PubkeyHex: string;
  authenticodeCertRef?: string | null;
  releaseChannel: string;
  signingPolicy: string;
  createdAt: string;
  updatedAt: string;
}

export interface PublisherProfileInput {
  id?: string | null;
  publisherId: string;
  displayName: string;
  organization: string;
  website?: string | null;
  supportContact?: string | null;
  copyright: string;
  ed25519KeyId?: string | null;
  ed25519PubkeyHex?: string | null;
  authenticodeCertRef?: string | null;
  releaseChannel: string;
  signingPolicy: string;
}

export interface KeyRotationDelegation {
  publisherId: string;
  productSlug: string;
  oldPubkeyHex: string;
  newPubkeyHex: string;
  issuedAtUtc: string;
  expiresAtUtc: string;
  nonce: string;
  signatureByOldKeyHex: string;
}

export interface QaSelfSignedCertInfo {
  subject: string;
  thumbprint: string;
  storeLocation: string;
  certRef: string;
  notAfterUtc: string;
}

export interface SigningPreflightReport {
  ready: boolean;
  signtoolAvailable: boolean;
  signtoolPath?: string | null;
  signingMode: SigningMode;
  publisherProfilePresent: boolean;
  certRefPresent: boolean;
  keystoreKeyId?: string | null;
  issues: string[];
  warnings: string[];
}

export interface BuilderExportOptions {
  signingMode?: SigningMode | null;
  publisherProfileId?: string | null;
  authenticodeCertRef?: string | null;
  timestampUrl?: string | null;
  releaseChannel?: string | null;
  versionInfo?: PeVersionInfoSpec | null;
  keyRotationDelegation?: KeyRotationDelegation | null;
  includePendingDelegation?: boolean | null;
}

export interface KeyRotationWithDelegationResult {
  newMeta: ProducerKeyMetadata;
  delegation: KeyRotationDelegation;
  delegationSavedPath: string;
}

export interface SigningToolchainStatus {
  signtoolFound: boolean;
  discoveredPath?: string | null;
  sdkBundledInApp: boolean;
  searchedLocations: string[];
  unsignedInternalSupported: boolean;
  guidance: string;
}

export interface FinalReleaseVerificationReport {
  verified: boolean;
  releaseDir: string;
  exeName: string;
  finalExeSha256: string;
  authenticodeChecked: boolean;
  authenticodeTrustStatus: string;
  authenticodeTrustedPublicChain: boolean;
  ed25519BundleVerified: boolean;
  peRootPubkeyMatches: boolean;
  peVersionInfoValid: boolean;
  publisherIdentityMatchesCert: boolean;
  publisherIdentityDetail: string;
  distributedFilesChecked: number;
  distributedFileMismatches: string[];
  issues: string[];
}

export interface GameProjectRecord {
  id: string;
  slug: string;
  name: string;
  gamePackageId: string;
  version: string;
  exeName: string;
  windowTitle: string;
  iconRelPath: string;
  targetWidthPx: number;
  targetHeightPx: number;
  bundleAdb: boolean;
  projectDirPath: string;
  createdAt: string;
  updatedAt: string;
  publisherProfileId?: string | null;
  signingMode?: string;
  versionInfoJson?: string | null;
}

export interface GameProjectSummary {
  project: GameProjectRecord;
  workflowsCount: number;
  tasksCount: number;
  templatesCount: number;
  modulesCount: number;
  accountsCount: number;
  jobsCount: number;
  releasesCount: number;
  latestReleaseVersion?: string | null;
  latestReleaseExePath?: string | null;
}

export interface CreateProjectRequest {
  slug: string;
  name: string;
  gamePackageId: string;
  version?: string | null;
  exeName?: string | null;
  windowTitle?: string | null;
  iconRelPath?: string | null;
  targetWidthPx?: number | null;
  targetHeightPx?: number | null;
  bundleAdb?: boolean | null;
  enabledModules?: string[] | null;
}

export interface UpdateProjectRequest {
  id: string;
  name?: string | null;
  gamePackageId?: string | null;
  version?: string | null;
  exeName?: string | null;
  windowTitle?: string | null;
  iconRelPath?: string | null;
  targetWidthPx?: number | null;
  targetHeightPx?: number | null;
  bundleAdb?: boolean | null;
  publisherProfileId?: string | null;
  signingMode?: string | null;
  versionInfoJson?: string | null;
}

export interface CloneProjectRequest {
  sourceProjectId: string;
  newSlug: string;
  newName: string;
  newGamePackageId?: string | null;
  newExeName?: string | null;
}

export interface ProjectModuleRecord {
  projectId: string;
  moduleId: string;
  packagedInRelease: boolean;
  enabledByDefault: boolean;
  configJson: string;
  updatedAt: string;
}

export interface VisionTemplateRecord {
  id: string;
  projectId: string;
  name: string;
  relPath: string;
  widthPx: number;
  heightPx: number;
  sourceCropX?: number | null;
  sourceCropY?: number | null;
  defaultConfidence: number;
  sha256: string;
  createdAt: string;
}

export interface ProjectTaskRecord {
  id: string;
  projectId: string;
  workflowId: string;
  categoryId: string;
  categoryShortTag: string;
  categoryName: string;
  categoryColorHex: string;
  title: string;
  priority: number;
  estimateSec: number;
  defaultEnabled: boolean;
  settingsSchemaJson: string;
  updatedAt: string;
}

export interface ProjectWorkflowRecord {
  id: string;
  projectId: string;
  name: string;
  category: string;
  draftJson: string;
  draftUpdatedAt: string;
  archivedAt?: string | null;
  latestVersionNo?: number | null;
}

export interface ProjectAccountInput {
  id?: string | null;
  projectId: string;
  alias: string;
  gameProfileId?: string | null;
  secretRef?: string | null;
  state: string;
}

export interface ProjectAccountRecord {
  id: string;
  projectId: string;
  alias: string;
  gameProfileId?: string | null;
  secretRef?: string | null;
  state: string;
  updatedAt: string;
}

export interface ProjectReleaseRecord {
  id: string;
  projectId: string;
  version: string;
  outputDirPath: string;
  exePath: string;
  exeSizeBytes: number;
  exeSha256: string;
  keyId: string;
  publisherPublicKeyHex: string;
  peRcdataPubkeyId: number;
  signatureEd25519Hex: string;
  buildReportJson: string;
  builtAt: string;
  lifecycleState: string;
  releaseChannel: string;
  changelogMd: string;
  minRuntimeVersion: string;
  workflowSchemaVersion: number;
  uiSchemaVersion: number;
  moduleApiVersion: string;
  sqliteSchemaVersion: number;
  gitCommit?: string | null;
  signatureStatus: string;
  promotedAt?: string | null;
  rolledBackFromId?: string | null;
  publisherProfileId?: string | null;
  signingMode?: string;
  authenticodeTrustStatus?: string;
  authenticodeThumbprint?: string | null;
  authenticodeSubject?: string | null;
  trustedPublicRelease?: boolean;
}

export interface ReleaseTransitionRecord {
  id: string;
  projectId: string;
  releaseId: string;
  fromState: string;
  toState: string;
  note: string;
  transitionedAt: string;
}

export interface CompatibilityDimensionCheck {
  dimension: string;
  requiredOrBundle: string;
  runtimeSupported: string;
  compatible: boolean;
  detail: string;
}

export interface CompatibilityReport {
  compatible: boolean;
  checks: CompatibilityDimensionCheck[];
  blockingReasons: string[];
}

export interface ActiveVersionPointer {
  projectSlug: string;
  activeVersion: string;
  previousVersion?: string | null;
  exeName: string;
  sqliteSchemaVersion: number;
  minRuntimeVersion: string;
  postUpdateAutoResumeBlocked: boolean;
  blockedJobIds: string[];
  backupDbPath?: string | null;
  activatedAtUtc: string;
  journalId: string;
}

export interface UpdateJournal {
  journalId: string;
  operation: string;
  projectSlug: string;
  sourceVersion?: string | null;
  targetVersion: string;
  stage: string;
  backupDbPath?: string | null;
  liveDbPath: string;
  lastError?: string | null;
  updatedAtUtc: string;
}

export interface UpdateOutcome {
  operation: string;
  status: string;
  projectSlug: string;
  previousVersion?: string | null;
  activeVersion: string;
  stageReached: string;
  backupDbPath?: string | null;
  backupVerifiedOk: boolean;
  signatureVerifiedOk: boolean;
  compatibilityVerifiedOk: boolean;
  postUpdateAutoResumeBlocked: boolean;
  blockedJobIds: string[];
  binaryRollbackOnly?: boolean;
  databaseRestored?: boolean;
  preRollbackSafetyDbPath?: string | null;
  journalPath: string;
  detail: string;
}

export interface ReconcileOutcome {
  status: string;
  actionTaken: string;
  recoveredFromStage?: string | null;
  activeVersion?: string | null;
  dbIntegrityOk: boolean;
  journalPath: string;
  detail: string;
}

export interface ManagedInstallInspection {
  installRoot: string;
  activePointer?: ActiveVersionPointer | null;
  journal?: UpdateJournal | null;
}

export interface GameProjectDetail {
  project: GameProjectRecord;
  modules: ProjectModuleRecord[];
  workflows: ProjectWorkflowRecord[];
  templates: VisionTemplateRecord[];
  tasks: ProjectTaskRecord[];
  uiSchema?: MiniToolUiSchema | null;
  accounts: ProjectAccountRecord[];
  releases: ProjectReleaseRecord[];
}

export interface IntegrityReport {
  integrityCheck: string;
  foreignKeyViolations: number;
  journalMode: string;
  schemaVersions: number[];
  tableCounts: Record<string, number>;
  isHealthy: boolean;
}

export interface BackupReport {
  backupPath: string;
  backupSizeBytes: number;
  backupSha256: string;
  integrity: IntegrityReport;
  backedUpAtUtc: string;
}

export interface ProducerKeyMetadata {
  keyId: string;
  publicKeyHex: string;
  algorithm: string;
  createdAtUtc: string;
  keyStorePath: string;
}

export interface CredentialMetadata {
  uri: string;
  scope: string;
  projectSlug: string;
  keyName: string;
}

export interface Jx2MigrationSummary {
  projectId: string;
  projectSlug: string;
  projectDirPath: string;
  workflowsImported: number;
  legacyWorkflowsPreserved: number;
  templatesImported: number;
  tasksSeeded: number;
  modulesSeeded: number;
  originalSourcePreserved: boolean;
}

// =========================================================================
// P3 Runtime Coordinator, Vision/ROI, Proxy, ADB & Builder Types
// =========================================================================

export interface ProjectJobRecord {
  id: string;
  projectId?: string | null;
  deviceId: string;
  accountId?: string | null;
  workflowVersionId: string;
  priority: number;
  state: string;
  fencingGeneration: number;
  currentNodeId?: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface CheckpointRecord {
  id: string;
  jobId: string;
  nodeId: string;
  stateJson: string;
  evidenceRef?: string | null;
  createdAt: string;
}

export interface JobResumeContext {
  job: ProjectJobRecord;
  projectDirPath: string;
  workflowId: string;
  workflowVersionNo: number;
  workflowGraphJson: string;
  workflowSha256: string;
  latestCheckpoint?: CheckpointRecord | null;
}

export interface JobResumeDetail {
  resumeContext: JobResumeContext;
  stepEvents: StepEvent[];
}

export interface RunProjectWorkflowRequest {
  jobId?: string | null;
  projectId: string;
  workflowId: string;
  taskId?: string | null;
  deviceId: string;
  accountId?: string | null;
  priority: number;
  leaseTtlSecs: number;
  workflowTimeoutMs?: number | null;
  initialVariables?: Record<string, any>;
}

export interface RunWorkflowExecutionSummary {
  jobId: string;
  projectId: string;
  workflowId: string;
  workflowVersionId: string;
  workflowSha256: string;
  taskId?: string | null;
  deviceId: string;
  accountId?: string | null;
  fencingGeneration: number;
  adapterKind: string;
  state: string;
  currentNodeId?: string | null;
  checkpointId?: string | null;
  resumedFromNodeId?: string | null;
  blockReason?: string | null;
  eventsRecorded: number;
  variables: Record<string, any>;
}

export interface BackgroundJobStarted {
  jobId: string;
  deviceId: string;
  state: string;
}

export interface ActiveDeviceLeaseView {
  deviceId: string;
  jobId: string;
  generation: number;
  expiresAt: string;
}

export interface DeviceHealthReport {
  serial: string;
  reachable: boolean;
  connectionState: ConnectionState;
  widthPx: number;
  heightPx: number;
  rotationDeg: number;
  latencyMs: number;
  error?: string | null;
}

export interface DevicePortMapping {
  direction: string;
  serial: string;
  local: string;
  remote: string;
}

export interface AdbPortMappingsOverview {
  serial: string;
  forwards: DevicePortMapping[];
  reverses: DevicePortMappingsOverviewItem[];
  note: string;
}

type DevicePortMappingsOverviewItem = DevicePortMapping;

export interface RoiRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface Bounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface TemplateMatchPreviewReport {
  projectId: string;
  templateId: string;
  templateName: string;
  matched: boolean;
  confidence?: number | null;
  bounds?: Bounds | null;
  suggestedTapPoint?: { x: number; y: number } | null;
  annotatedPngBase64?: string | null;
  savedAnnotatedPath?: string | null;
  error?: string | null;
}

export type ProxyProtocol = 'tcp_only' | 'socks5' | 'http_connect';

export interface ProxyConfig {
  host: string;
  port: number;
  protocol: ProxyProtocol;
  username?: string | null;
  password?: string | null;
  timeoutMs: number;
  targetProbeHost?: string | null;
  targetProbePort?: number | null;
}

export interface ProxyProbeReport {
  host: string;
  port: number;
  protocol: ProxyProtocol;
  reachable: boolean;
  handshakeOk: boolean;
  authStatus: string;
  targetConnectOk?: boolean | null;
  tcpConnectMs?: number | null;
  totalLatencyMs: number;
  error?: string | null;
  disclaimer: string;
  checkedAtUtc: string;
}

export interface BuildStepMetric {
  stepNo: number;
  name: string;
  durationMs: number;
  status: string;
  detail: string;
}

export interface BuildPreflightReport {
  ready: boolean;
  projectId: string;
  projectSlug: string;
  projectName: string;
  exeName: string;
  workflowCount: number;
  templateCount: number;
  runtimeExePresent: boolean;
  iconPresent: boolean;
  adbReady: boolean;
  keystoreKeyId?: string | null;
  issues: string[];
  checks: BuildStepMetric[];
  modules: any[];
}

export interface BuildReport {
  projectId: string;
  projectSlug: string;
  toolExeName: string;
  toolExePath: string;
  toolExeSizeBytes: number;
  toolExeSha256: string;
  peIconLayersInjected: number;
  peEmbeddedPubkeyResourceId: number;
  peEmbeddedPubkeyVerified: boolean;
  authenticodeStatus: string;
  signingMode?: SigningMode;
  trustedPublicRelease?: boolean;
  versionInfo?: PeVersionInfoSpec | null;
  authenticodeVerification?: {
    signed: boolean;
    trustStatus: string;
    trustedPublicRelease: boolean;
    winTrustErrorCode: number;
    signerSubject?: string | null;
    signerIssuer?: string | null;
    signerThumbprint?: string | null;
    timestampStatus: string;
    detail: string;
  } | null;
  keyId: string;
  publisherPublicKeyHex: string;
  externalKeystorePath: string;
  signatureAlgorithm: string;
  signatureEd25519Hex: string;
  totalDurationMs: number;
  steps: BuildStepMetric[];
}

export interface BuilderReleaseExportResult {
  release: ProjectReleaseRecord;
  buildReport: BuildReport;
}

export interface DbOptimizeReport {
  walCheckpointed: boolean;
  integrityCheck: string;
  foreignKeyViolations: number;
  isHealthy: boolean;
  totalProjects: number;
  totalJobs: number;
  optimizedAtUtc: string;
}

export interface RuntimeVariablesSnapshot {
  scopeKey: string;
  variables: Record<string, any>;
  updatedAt?: string | null;
}

// --- Universal Element Inspector & Smart Action Types ---
export interface ElementBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface ElementNode {
  id: number;
  parent_id: number | null;
  children_ids: number[];
  index: number;
  text: string;
  resource_id: string;
  class_name: string;
  package: string;
  content_desc: string;
  checkable: boolean;
  checked: boolean;
  clickable: boolean;
  enabled: boolean;
  focusable: boolean;
  focused: boolean;
  scrollable: boolean;
  long_clickable: boolean;
  password: boolean;
  selected: boolean;
  bounds: ElementBounds;
  raw_bounds: ElementBounds;
  depth: number;
  path: string;
  is_visible: boolean;
}

export interface ElementSnapshot {
  id: string;
  timestamp_ms: number;
  rotation_deg: number;
  screen_width: number;
  screen_height: number;
  package_name: string | null;
  nodes: ElementNode[];
  root_ids: number[];
}

export interface NativeElementSelector {
  resource_id?: string | null;
  text?: string | null;
  text_regex?: string | null;
  content_desc?: string | null;
  class_name?: string | null;
  package?: string | null;
  clickable?: boolean | null;
  enabled?: boolean | null;
  xpath?: string | null;
  search_region?: ElementBounds | null;
}

export interface SelectorMatch {
  node_id: number | null;
  bounds: ElementBounds;
  confidence: number;
  strategy_used: string;
  is_clickable: boolean;
  resolved_clickable_ancestor_id: number | null;
  resolved_bounds: ElementBounds;
}

export interface GeneratedSelector {
  strategy_name: string;
  selector: NativeElementSelector;
  uniqueness_score: number;
  match_count: number;
  expression: string;
  is_recommended: boolean;
}

export interface InspectSnapshotResult {
  snapshot: ElementSnapshot;
  screenshot_base64: string;
  capture_duration_ms: number;
  dump_duration_ms: number;
  parse_duration_ms: number;
  total_duration_ms: number;
  device_serial: string;
  package_name: string | null;
}

export type ActionExecutionMode = 'DynamicBoundsTap' | 'ClickableParent' | 'SemanticClick';

export type ElementActionKind =
  | 'Tap'
  | 'DoubleTap'
  | { LongPress: { duration_ms: number } }
  | { SetText: { text: string; clear_first: boolean } }
  | { WaitAppear: { timeout_ms: number } }
  | { WaitDisappear: { timeout_ms: number } }
  | 'AssertExists';

export interface ElementActionPlan {
  mode: ActionExecutionMode;
  target_bounds: ElementBounds;
  safe_point: { x: number; y: number };
  is_clickable_parent_used: boolean;
  execution_kind: ElementActionKind;
}

export interface ElementActionResult {
  success: boolean;
  plan: ElementActionPlan;
  execution_ms: number;
  new_screenshot_base64: string | null;
  post_verification_success: boolean;
  message: string;
}

export interface DetectionResult {
  target_bounds: ElementBounds;
  resolved_bounds: ElementBounds;
  confidence: number;
  candidate_count: number;
  strategy_used: string;
  is_clickable: boolean;
  resolved_clickable_ancestor_id?: number | null;
  matches: SelectorMatch[];
}

export type VirtualSelectorMode = 'native' | 'ocr' | 'image' | 'point' | 'region' | 'hybrid';

export type Selector =
  | { Point: { x: number; y: number; is_relative: boolean } }
  | { Native: NativeElementSelector }
  | { Ocr: { text: string; confidence_threshold?: number; exact_match?: boolean } }
  | { Image: { template_id: string; threshold?: number } }
  | { Region: { x: number; y: number; width: number; height: number } };

export interface LibraryElementRecord {
  id: string;
  projectId: string;
  name: string;
  packageFilter?: string | null;
  primaryStrategy: string;
  selectorJson: string;
  fallbackSelectorsJson?: string;
  boundsJson?: string | null;
  version: number;
  createdAt: string;
  updatedAt: string;
}

// ========================================================================
// Phase 2.2: Device Studio & Smart Workflow Recorder Types
// ========================================================================

export interface StreamFrameResponse {
  screenshot_base64: string;
  width: number;
  height: number;
  rotation_deg: number;
  timestamp_ms: number;
  latency_ms: number;
}

export interface ActionReceipt {
  success: boolean;
  duration_ms: number;
  error?: string | null;
  point_tapped?: { x: number; y: number } | null;
  verify_passed: boolean;
}

export interface RecordedWorkflowStep {
  id: string;
  stepNumber: number;
  actionType: string;
  label: string;
  selector?: Selector | null;
  resolvedBounds?: ElementBounds | null;
  point?: { x: number; y: number } | null;
  swipePath?: { start_x: number; start_y: number; end_x: number; end_y: number; duration_ms: number } | null;
  keyCode?: number | null;
  inputText?: string | null;
  waitMs?: number | null;
  assertionRule?: string | null;
  beforeScreenshot?: string | null;
  afterScreenshot?: string | null;
  durationMs: number;
  status: string;
  enabled: boolean;
  uniquenessScore: number;
  strategyUsed: string;
  notes?: string | null;
}

export interface StepExecutionResult {
  stepId: string;
  success: boolean;
  durationMs: number;
  errorMessage?: string | null;
  verified: boolean;
  targetPoint?: { x: number; y: number } | null;
}

export interface WorkflowReplayReport {
  totalSteps: number;
  executedSteps: number;
  passedSteps: number;
  failedSteps: number;
  totalDurationMs: number;
  results: StepExecutionResult[];
}

