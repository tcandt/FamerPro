use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Version compatibility matrix for Project, Workflow, UI Schema, Module API, and Runtime
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilityVersions {
    pub project_schema_version: u32,
    pub workflow_schema_version: u32,
    pub ui_schema_version: u32,
    pub module_api_version: String,
    pub min_runtime_version: String,
}

impl Default for CompatibilityVersions {
    fn default() -> Self {
        Self {
            project_schema_version: 1,
            workflow_schema_version: 1,
            ui_schema_version: 1,
            module_api_version: "1.0.0".to_string(),
            min_runtime_version: "0.1.0".to_string(),
        }
    }
}

/// Game Project definition (Platform-level entity, completely decoupled from any single game)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GameProjectManifest {
    pub compatibility: CompatibilityVersions,
    pub project_id: String,
    pub slug: String,
    pub name: String,
    pub game_package_id: String,
    pub version: String,
    pub exe_name: String,
    pub window_title: String,
    pub icon_rel_path: String,
    pub target_resolution: (u32, u32),
    pub packaged_modules: Vec<String>,
    pub workflow_files: Vec<String>,
    pub template_files: Vec<String>,
    pub bundle_adb: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MiniLayoutPreset {
    Compact1Col,
    Compact2Col,
    Expanded,
}

/// Dynamic column binding in the top device/instance table of a Mini Tool
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeColumnBinding {
    pub id: String,
    pub label: String,
    pub runtime_var_key: String,
    pub fallback_display: String,
    pub min_window_width: u32,
    pub badge_color: Option<String>,
}

/// Dynamic task category defined per game (e.g. [NV]/[PB]/[HĐ] for RPG vs [Nông Nghiệp]/[Quân Sự] for SLG)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaskCategoryDef {
    pub id: String,
    pub short_tag: String,
    pub display_name: String,
    pub color_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaskSettingField {
    pub key: String,
    pub label: String,
    pub field_type: String, // "boolean", "number", "select"
    pub default_value: serde_json::Value,
    pub options: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MiniTaskDef {
    pub id: String,
    pub title: String,
    pub category_id: String,
    pub workflow_id: String,
    pub default_enabled: bool,
    pub priority: i32,
    pub estimate_sec: u32,
    pub settings_schema: Vec<TaskSettingField>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UtilityToggleDef {
    pub id: String,
    pub label: String,
    pub default_enabled: bool,
    pub required_module: Option<String>,
    pub settings_schema: Vec<TaskSettingField>,
}

/// Schema-driven UI configuration for a Mini Auto-Tool
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MiniToolUiSchema {
    pub schema_version: u32,
    pub project_slug: String,
    pub window_title: String,
    pub version_badge: String,
    pub theme_accent_hex: String,
    pub header_bg_hex: String,
    pub layout_preset: MiniLayoutPreset,
    pub default_window_size: (u32, u32),
    /// Configurable order of sections (e.g. ["device_table", "main_workspace", "footer_bar"])
    pub sections_order: Vec<String>,
    /// Component IDs hidden in this game's Mini Tool
    pub hidden_components: Vec<String>,
    /// Dynamic runtime columns bound to ExecutionContext.variables
    pub device_columns: Vec<RuntimeColumnBinding>,
    /// Dynamic task categories
    pub task_categories: Vec<TaskCategoryDef>,
    /// Ordered list of tasks
    pub tasks: Vec<MiniTaskDef>,
    /// Dynamic utility toggles
    pub utilities: Vec<UtilityToggleDef>,
    /// Default runtime variable values for preview / initial state
    pub initial_runtime_vars: HashMap<String, serde_json::Value>,
}
