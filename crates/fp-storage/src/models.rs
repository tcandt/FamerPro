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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
}
