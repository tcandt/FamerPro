use fp_bundle_contract::MiniToolUiSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const DEFAULT_LEGACY_PROJECT_ID: &str = "proj-jx2-mobile-001";
pub const DEFAULT_LEGACY_PROJECT_SLUG: &str = "jx2-mobile";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GameProjectRecord {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub game_package_id: String,
    pub version: String,
    pub exe_name: String,
    pub window_title: String,
    pub icon_rel_path: String,
    pub target_width_px: u32,
    pub target_height_px: u32,
    pub bundle_adb: bool,
    pub project_dir_path: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub publisher_profile_id: Option<String>,
    #[serde(default = "default_signing_mode_str")]
    pub signing_mode: String,
    #[serde(default)]
    pub version_info_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PublisherProfileRecord {
    pub id: String,
    pub publisher_id: String,
    pub display_name: String,
    pub organization: String,
    pub website: Option<String>,
    pub support_contact: Option<String>,
    pub copyright: String,
    pub ed25519_key_id: String,
    pub ed25519_pubkey_hex: String,
    pub authenticode_cert_ref: Option<String>,
    pub release_channel: String,
    pub signing_policy: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublisherProfileInput {
    pub id: Option<String>,
    pub publisher_id: String,
    pub display_name: String,
    pub organization: String,
    pub website: Option<String>,
    pub support_contact: Option<String>,
    pub copyright: String,
    pub ed25519_key_id: Option<String>,
    pub ed25519_pubkey_hex: Option<String>,
    pub authenticode_cert_ref: Option<String>,
    pub release_channel: String,
    pub signing_policy: String,
}

fn default_signing_mode_str() -> String {
    "unsigned_internal".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameProjectSummary {
    pub project: GameProjectRecord,
    pub workflows_count: usize,
    pub tasks_count: usize,
    pub templates_count: usize,
    pub modules_count: usize,
    pub accounts_count: usize,
    pub jobs_count: usize,
    pub releases_count: usize,
    pub latest_release_version: Option<String>,
    pub latest_release_exe_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    pub slug: String,
    pub name: String,
    pub game_package_id: String,
    pub version: Option<String>,
    pub exe_name: Option<String>,
    pub window_title: Option<String>,
    pub icon_rel_path: Option<String>,
    pub target_width_px: Option<u32>,
    pub target_height_px: Option<u32>,
    pub bundle_adb: Option<bool>,
    pub enabled_modules: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectRequest {
    pub id: String,
    pub name: Option<String>,
    pub game_package_id: Option<String>,
    pub version: Option<String>,
    pub exe_name: Option<String>,
    pub window_title: Option<String>,
    pub icon_rel_path: Option<String>,
    pub target_width_px: Option<u32>,
    pub target_height_px: Option<u32>,
    pub bundle_adb: Option<bool>,
    #[serde(default)]
    pub publisher_profile_id: Option<String>,
    #[serde(default)]
    pub signing_mode: Option<String>,
    #[serde(default)]
    pub version_info_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloneProjectRequest {
    pub source_project_id: String,
    pub new_slug: String,
    pub new_name: String,
    pub new_game_package_id: Option<String>,
    pub new_exe_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectModuleRecord {
    pub project_id: String,
    pub module_id: String,
    pub packaged_in_release: bool,
    pub enabled_by_default: bool,
    pub config_json: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VisionTemplateInput {
    pub id: Option<String>,
    pub project_id: String,
    pub name: String,
    pub rel_path: String,
    pub width_px: u32,
    pub height_px: u32,
    pub source_crop_x: Option<i32>,
    pub source_crop_y: Option<i32>,
    pub default_confidence: Option<f64>,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VisionTemplateRecord {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub rel_path: String,
    pub width_px: u32,
    pub height_px: u32,
    pub source_crop_x: Option<i32>,
    pub source_crop_y: Option<i32>,
    pub default_confidence: f64,
    pub sha256: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTaskRecord {
    pub id: String,
    pub project_id: String,
    pub workflow_id: String,
    pub category_id: String,
    pub category_short_tag: String,
    pub category_name: String,
    pub category_color_hex: String,
    pub title: String,
    pub priority: i32,
    pub estimate_sec: u32,
    pub default_enabled: bool,
    pub settings_schema_json: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectWorkflowRecord {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub category: String,
    pub draft_json: String,
    pub draft_updated_at: String,
    pub archived_at: Option<String>,
    pub latest_version_no: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectAccountInput {
    pub id: Option<String>,
    pub project_id: String,
    pub alias: String,
    pub game_profile_id: Option<String>,
    pub secret_ref: Option<String>,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectAccountRecord {
    pub id: String,
    pub project_id: String,
    pub alias: String,
    pub game_profile_id: Option<String>,
    pub secret_ref: Option<String>,
    pub state: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectJobRecord {
    pub id: String,
    pub project_id: Option<String>,
    pub device_id: String,
    pub account_id: Option<String>,
    pub workflow_version_id: String,
    pub priority: i32,
    pub state: String,
    pub fencing_generation: i64,
    pub current_node_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectJobInput {
    pub project_id: String,
    pub job_id: String,
    pub device_id: String,
    pub account_id: Option<String>,
    pub workflow_id: String,
    pub workflow_name: String,
    pub graph_json: String,
    pub priority: i32,
    pub state: String,
    pub fencing_generation: u64,
    pub current_node_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointRecordInput {
    pub id: Option<String>,
    pub job_id: String,
    pub node_id: String,
    pub state_json: String,
    pub evidence_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointRecord {
    pub id: String,
    pub job_id: String,
    pub node_id: String,
    pub state_json: String,
    pub evidence_ref: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobResumeContext {
    pub job: ProjectJobRecord,
    pub project_dir_path: String,
    pub workflow_id: String,
    pub workflow_version_no: i64,
    pub workflow_graph_json: String,
    pub workflow_sha256: String,
    pub latest_checkpoint: Option<CheckpointRecord>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectReleaseRecord {
    pub id: String,
    pub project_id: String,
    pub version: String,
    pub output_dir_path: String,
    pub exe_path: String,
    pub exe_size_bytes: u64,
    pub exe_sha256: String,
    pub key_id: String,
    pub publisher_public_key_hex: String,
    pub pe_rcdata_pubkey_id: i64,
    pub signature_ed25519_hex: String,
    pub build_report_json: String,
    pub built_at: String,
    #[serde(default = "default_release_lifecycle_published")]
    pub lifecycle_state: String,
    #[serde(default = "default_release_channel")]
    pub release_channel: String,
    #[serde(default)]
    pub changelog_md: String,
    #[serde(default = "default_min_runtime_ver")]
    pub min_runtime_version: String,
    #[serde(default = "default_schema_v1")]
    pub workflow_schema_version: u32,
    #[serde(default = "default_schema_v1")]
    pub ui_schema_version: u32,
    #[serde(default = "default_module_api_ver")]
    pub module_api_version: String,
    #[serde(default = "default_sqlite_schema_v4")]
    pub sqlite_schema_version: u32,
    #[serde(default)]
    pub git_commit: String,
    #[serde(default = "default_signature_status")]
    pub signature_status: String,
    #[serde(default)]
    pub promoted_at: Option<String>,
    #[serde(default)]
    pub rolled_back_from_id: Option<String>,
    #[serde(default)]
    pub publisher_profile_id: Option<String>,
    #[serde(default = "default_signing_mode_str")]
    pub signing_mode: String,
    #[serde(default = "default_auth_trust_unsigned")]
    pub authenticode_trust_status: String,
    #[serde(default)]
    pub authenticode_thumbprint: Option<String>,
    #[serde(default)]
    pub authenticode_subject: Option<String>,
    #[serde(default)]
    pub trusted_public_release: bool,
}

fn default_auth_trust_unsigned() -> String {
    "unsigned".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseTransitionRecord {
    pub id: String,
    pub project_id: String,
    pub release_id: String,
    pub from_state: String,
    pub to_state: String,
    pub note: String,
    pub transitioned_at: String,
}

fn default_release_lifecycle_published() -> String {
    "Published".to_string()
}
fn default_release_channel() -> String {
    "internal".to_string()
}
fn default_min_runtime_ver() -> String {
    "0.1.0".to_string()
}
fn default_schema_v1() -> u32 {
    1
}
fn default_module_api_ver() -> String {
    "1.0.0".to_string()
}
fn default_sqlite_schema_v4() -> u32 {
    4
}
fn default_signature_status() -> String {
    "Ed25519-Verified".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameProjectDetail {
    pub project: GameProjectRecord,
    pub modules: Vec<ProjectModuleRecord>,
    pub workflows: Vec<ProjectWorkflowRecord>,
    pub templates: Vec<VisionTemplateRecord>,
    pub tasks: Vec<ProjectTaskRecord>,
    pub ui_schema: Option<MiniToolUiSchema>,
    pub accounts: Vec<ProjectAccountRecord>,
    pub releases: Vec<ProjectReleaseRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityReport {
    pub integrity_check: String,
    pub foreign_key_violations: usize,
    pub journal_mode: String,
    pub schema_versions: Vec<i64>,
    pub table_counts: BTreeMap<String, i64>,
    pub is_healthy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupReport {
    pub backup_path: String,
    pub backup_size_bytes: u64,
    pub backup_sha256: String,
    pub integrity: IntegrityReport,
    pub backed_up_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Jx2WorkflowMigrationItem {
    pub source_file: String,
    pub source_workflow_id: String,
    pub dest_workflow_id: String,
    pub dest_version_id: String,
    pub source_sha256: String,
    pub action: String,
    pub version_before: Option<i64>,
    pub version_after: i64,
    pub content_comparison: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Jx2MigrationSummary {
    pub project_id: String,
    pub project_slug: String,
    pub project_dir_path: String,
    pub workflows_imported: usize,
    pub legacy_workflows_preserved: usize,
    pub templates_imported: usize,
    pub tasks_seeded: usize,
    pub modules_seeded: usize,
    pub original_source_preserved: bool,
    #[serde(default)]
    pub workflow_migrations: Vec<Jx2WorkflowMigrationItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CrashFaultPoint {
    None,
    FailTemplateCopy,
    FailManifestWrite,
    FailBeforeDbCommit,
    FailAfterDbCommitTrashPurge,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconcileReport {
    pub total_resolved: usize,
    pub creates_finalized: usize,
    pub uncommitted_dirs_reverted: usize,
    pub mutations_finalized: usize,
    pub mutations_rolled_back: usize,
    pub deletes_finalized: usize,
    pub deletes_restored: usize,
    pub staging_purged: usize,
    pub trash_purged: usize,
    pub tmp_files_purged: usize,
    pub actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbOptimizeReport {
    pub wal_checkpointed: bool,
    pub integrity_check: String,
    pub foreign_key_violations: usize,
    pub is_healthy: bool,
    pub total_projects: usize,
    pub total_jobs: usize,
    pub optimized_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeVariablesSnapshot {
    pub scope_key: String,
    pub variables: std::collections::HashMap<String, serde_json::Value>,
    pub updated_at: Option<String>,
    #[serde(default)]
    pub envelopes: std::collections::BTreeMap<String, fp_domain::RuntimeVariableEnvelope>,
    #[serde(default)]
    pub testkit_envelopes: std::collections::BTreeMap<String, fp_domain::RuntimeVariableEnvelope>,
    #[serde(default)]
    pub stream_seqs: std::collections::HashMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeProvenanceSnapshot {
    pub base_scope_key: String,
    pub live_envelopes: std::collections::BTreeMap<String, fp_domain::RuntimeVariableEnvelope>,
    pub testkit_envelopes: std::collections::BTreeMap<String, fp_domain::RuntimeVariableEnvelope>,
    pub stream_seqs: std::collections::HashMap<String, u64>,
    pub missed_events: Vec<fp_domain::RuntimeVarDeltaEvent>,
    pub loaded_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RollbackVerificationReport {
    pub current_db_path: String,
    pub backup_db_path: String,
    pub safety_snapshot_path: String,
    pub pre_rollback_versions: Vec<i64>,
    pub post_rollback_versions: Vec<i64>,
    pub pre_rollback_table_counts: BTreeMap<String, i64>,
    pub post_rollback_integrity: IntegrityReport,
    pub wal_checkpoint_busy: i64,
    pub reopen_verified: bool,
}
