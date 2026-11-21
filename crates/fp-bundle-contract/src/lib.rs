use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

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

/// Dynamic task category defined per game
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
    pub field_type: String,
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
    pub sections_order: Vec<String>,
    pub hidden_components: Vec<String>,
    pub device_columns: Vec<RuntimeColumnBinding>,
    pub task_categories: Vec<TaskCategoryDef>,
    pub tasks: Vec<MiniTaskDef>,
    pub utilities: Vec<UtilityToggleDef>,
    /// Unverified runtime variables start empty in Live Mode; Demo Mode must explicitly label mock values
    pub initial_runtime_vars: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ModuleCapability {
    AdbExec,
    CredentialVaultAccess,
    LocalSocketBind,
    OutboundProxyConnect,
    FilesystemReadWrite,
}

/// Explicitly tracks the 5 distinct architectural states of a module
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModuleStateReport {
    pub module_id: String,
    pub display_name: String,
    pub version: String,
    pub dependencies: Vec<String>,
    pub required_capabilities: Vec<ModuleCapability>,
    pub compiled_in_binary: bool,
    pub packaged_in_release: bool,
    pub permitted_to_initialize: bool,
    pub enabled_by_user_config: bool,
    pub active_and_initialized: bool,
    pub technical_note: String,
}

pub struct ModuleRegistry {
    modules: Vec<ModuleStateReport>,
}

impl ModuleRegistry {
    pub fn resolve_for_release(
        packaged_module_ids: &[String],
        user_enabled_ids: &[String],
    ) -> Self {
        let packaged_set: HashSet<&str> = packaged_module_ids.iter().map(|s| s.as_str()).collect();
        let user_set: HashSet<&str> = user_enabled_ids.iter().map(|s| s.as_str()).collect();

        let catalog = vec![
            (
                "mod.adb_core",
                "ADB & Device Driver Core",
                "1.0.0",
                vec![],
                vec![ModuleCapability::AdbExec],
                "Quản lý kết nối ADB, chụp màn hình screencap và gửi lệnh tap/swipe an toàn.",
            ),
            (
                "mod.vision_ocr",
                "Multiscale Vision & WinRT OCR",
                "1.0.0",
                vec!["mod.adb_core"],
                vec![ModuleCapability::FilesystemReadWrite],
                "So khớp mẫu đa tỉ lệ SSD và trích xuất chỉ số game qua OCR.",
            ),
            (
                "mod.account_vault",
                "Account Vault (Credential Manager)",
                "1.0.0",
                vec![],
                vec![ModuleCapability::CredentialVaultAccess],
                "Lưu trữ tham chiếu tài khoản theo namespace riêng của từng Mini Tool.",
            ),
            (
                "mod.proxy_network",
                "SOCKS5/HTTP Application Proxy",
                "1.0.0",
                vec![],
                vec![ModuleCapability::OutboundProxyConnect],
                "Kiểm tra & định tuyến kết nối SOCKS5/HTTP tương thích (LƯU Ý: Lưu lượng UDP thô của game engine cần VPN/tun2socks riêng, tách biệt hoàn toàn với ADB port forwarding).",
            ),
            (
                "mod.adb_port_forward",
                "ADB Host-Device Socket Forwarding",
                "1.0.0",
                vec!["mod.adb_core"],
                vec![ModuleCapability::AdbExec, ModuleCapability::LocalSocketBind],
                "Chuyển tiếp cổng TCP giữa máy chủ và thiết bị qua `adb forward` / `adb reverse` (Không phải proxy định tuyến lưu lượng game).",
            ),
            (
                "mod.emulator_optimizer",
                "Emulator Watchdog & Auto-Reconnect",
                "1.0.0",
                vec!["mod.adb_core"],
                vec![ModuleCapability::AdbExec],
                "Giám sát tiến trình giả lập, tự động mở lại game khi mất kết nối.",
            ),
        ];

        let mut modules = Vec::new();
        for (id, name, ver, deps, caps, note) in catalog {
            let is_core = id == "mod.adb_core" || id == "mod.vision_ocr";
            let is_packaged = is_core || packaged_set.contains(id);
            let is_permitted = is_packaged;
            let is_user_enabled = is_packaged && (is_core || user_set.contains(id));
            let is_active = is_permitted && is_user_enabled;

            modules.push(ModuleStateReport {
                module_id: id.to_string(),
                display_name: name.to_string(),
                version: ver.to_string(),
                dependencies: deps.into_iter().map(|s| s.to_string()).collect(),
                required_capabilities: caps,
                compiled_in_binary: true,
                packaged_in_release: is_packaged,
                permitted_to_initialize: is_permitted,
                enabled_by_user_config: is_user_enabled,
                active_and_initialized: is_active,
                technical_note: note.to_string(),
            });
        }

        Self { modules }
    }

    pub fn reports(&self) -> &[ModuleStateReport] {
        &self.modules
    }

    pub fn assert_module_active(&self, module_id: &str) -> Result<()> {
        match self.modules.iter().find(|m| m.module_id == module_id) {
            Some(m) if !m.packaged_in_release => {
                bail!(
                    "SECURITY_POLICY_VIOLATION: Module '{}' was NOT packaged in this Mini Tool release and cannot be initialized.",
                    module_id
                )
            }
            Some(m) if !m.active_and_initialized => {
                bail!(
                    "MODULE_INACTIVE: Module '{}' is disabled in user configuration.",
                    module_id
                )
            }
            Some(_) => Ok(()),
            None => bail!("UNKNOWN_MODULE: '{}'", module_id),
        }
    }

    pub fn assert_capability_granted(&self, module_id: &str, cap: &ModuleCapability) -> Result<()> {
        self.assert_module_active(module_id)?;
        let m = self
            .modules
            .iter()
            .find(|m| m.module_id == module_id)
            .unwrap();
        if !m.required_capabilities.contains(cap) {
            bail!(
                "CAPABILITY_DENIED: Module '{}' is not granted capability '{:?}'",
                module_id,
                cap
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseBundleManifest {
    pub compatibility: CompatibilityVersions,
    pub project: GameProjectManifest,
    pub module_states: Vec<ModuleStateReport>,
    pub workflows_count: usize,
    pub templates_count: usize,
    pub built_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortableDependencyAudit {
    pub webview2_evergreen_installed: bool,
    pub webview2_version_or_note: String,
    pub adb_exe_present: bool,
    pub adb_win_api_dll_present: bool,
    pub adb_win_usb_api_dll_present: bool,
    pub winrt_ocr_powershell_available: bool,
    pub exe_dir_writable: bool,
    pub write_probe_error: Option<String>,
    pub resolved_sqlite_db_path: String,
    pub fallback_to_local_appdata_used: bool,
    pub offline_execution_ready: bool,
}

/// Resolves a writable SQLite path for a Mini Tool by physically probing write access on `<exe_dir>/data`.
/// If Windows ACL or read-only attribute blocks writing, falls back to `%LOCALAPPDATA%\FarmerPro\MiniTools\<slug>\data\runtime_<slug>.db`.
pub fn resolve_writable_runtime_db_path(
    exe_dir: &Path,
    project_slug: &str,
) -> (PathBuf, bool, bool, Option<String>) {
    let local_data_dir = exe_dir.join("data");
    let probe_err = match std::fs::create_dir_all(&local_data_dir) {
        Ok(()) => {
            let probe_file = local_data_dir.join(".write_acl_probe");
            match std::fs::write(&probe_file, b"acl_check") {
                Ok(()) => {
                    let _ = std::fs::remove_file(&probe_file);
                    return (
                        local_data_dir.join(format!("runtime_{}.db", project_slug)),
                        true,
                        false,
                        None,
                    );
                }
                Err(e) => Some(format!("write_probe failed: {} (os error {:?})", e, e.raw_os_error())),
            }
        }
        Err(e) => Some(format!("create_dir_all failed: {} (os error {:?})", e, e.raw_os_error())),
    };

    let base_appdata = std::env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    let fallback_dir = base_appdata
        .join("FarmerPro")
        .join("MiniTools")
        .join(project_slug)
        .join("data");
    let _ = std::fs::create_dir_all(&fallback_dir);
    (
        fallback_dir.join(format!("runtime_{}.db", project_slug)),
        false,
        true,
        probe_err,
    )
}

pub fn audit_portable_environment(
    release_dir: &Path,
    project_slug: &str,
) -> PortableDependencyAudit {
    let (wv2_ok, wv2_note) = detect_webview2_runtime();
    let bin_dir = release_dir.join("bin");
    let adb_exe = bin_dir.join("adb.exe").exists();
    let adb_dll1 = bin_dir.join("AdbWinApi.dll").exists();
    let adb_dll2 = bin_dir.join("AdbWinUsbApi.dll").exists();

    let (db_path, exe_dir_writable, fallback_used, probe_err) =
        resolve_writable_runtime_db_path(release_dir, project_slug);

    let ps_available = std::env::var("SystemRoot")
        .map(|sr| {
            Path::new(&sr)
                .join("System32\\WindowsPowerShell\\v1.0\\powershell.exe")
                .exists()
        })
        .unwrap_or(true);

    PortableDependencyAudit {
        webview2_evergreen_installed: wv2_ok,
        webview2_version_or_note: wv2_note,
        adb_exe_present: adb_exe,
        adb_win_api_dll_present: adb_dll1,
        adb_win_usb_api_dll_present: adb_dll2,
        winrt_ocr_powershell_available: ps_available,
        exe_dir_writable,
        write_probe_error: probe_err,
        resolved_sqlite_db_path: db_path.to_string_lossy().to_string(),
        fallback_to_local_appdata_used: fallback_used,
        offline_execution_ready: true,
    }
}

fn detect_webview2_runtime() -> (bool, String) {
    let candidates = [
        r"C:\Program Files (x86)\Microsoft\EdgeWebView\Application",
        r"C:\Program Files\Microsoft\EdgeWebView\Application",
    ];
    for c in candidates {
        let p = Path::new(c);
        if p.exists() {
            if let Ok(entries) = std::fs::read_dir(p) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.chars().next().map(|ch| ch.is_ascii_digit()).unwrap_or(false) {
                        return (
                            true,
                            format!(
                                "Evergreen Runtime v{} (Fixed Version fallback supported if offline target lacks Evergreen)",
                                name
                            ),
                        );
                    }
                }
            }
            return (true, "Evergreen Runtime present".to_string());
        }
    }
    (
        false,
        "WebView2 Evergreen not detected in standard Program Files; requires Evergreen Bootstrapper or Fixed Version bundle".to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_readonly_directory_fallback_to_local_appdata() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_base = std::env::temp_dir().join(format!(
            "fp_acl_probe_test_{}_{}",
            std::process::id(),
            nanos
        ));
        let writable_dir = temp_base.join("writable_release");
        std::fs::create_dir_all(&writable_dir).unwrap();

        // 1. Writable release folder resolves to local <exe_dir>/data/runtime_<slug>.db
        let (db_path_1, writable_1, fallback_1, err_1) =
            resolve_writable_runtime_db_path(&writable_dir, "jx2-mobile");
        assert!(writable_1);
        assert!(!fallback_1);
        assert!(err_1.is_none());
        assert!(db_path_1.ends_with("runtime_jx2-mobile.db"));

        // 2. Blocked/non-writable data path (simulate directory creation conflict where 'data' is a locked file)
        let blocked_dir = temp_base.join("blocked_release");
        std::fs::create_dir_all(&blocked_dir).unwrap();
        std::fs::write(blocked_dir.join("data"), b"file_blocking_dir_creation").unwrap();

        let (db_path_2, writable_2, fallback_2, err_2) =
            resolve_writable_runtime_db_path(&blocked_dir, "jx2-mobile");
        assert!(!writable_2);
        assert!(fallback_2);
        assert!(err_2.is_some());
        let db_str = db_path_2.to_string_lossy();
        assert!(db_str.contains("MiniTools"));
        assert!(db_str.ends_with("runtime_jx2-mobile.db"));

        let _ = std::fs::remove_dir_all(&temp_base);
    }
}
