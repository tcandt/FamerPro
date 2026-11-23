use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub const CURRENT_RUNTIME_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const SUPPORTED_WORKFLOW_SCHEMA_VERSION: u32 = 1;
pub const SUPPORTED_UI_SCHEMA_VERSION: u32 = 1;
pub const SUPPORTED_MODULE_API_VERSION: &str = "1.0.0";
pub const CURRENT_SQLITE_SCHEMA_VERSION: u32 = 6;

fn default_sqlite_schema_version() -> u32 {
    3
}

/// Parsed Semantic Version (`MAJOR.MINOR.PATCH[-prerelease]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemVer {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub prerelease: Option<String>,
}

impl SemVer {
    pub fn parse(raw: &str) -> Result<Self> {
        let trimmed = raw.trim().trim_start_matches('v');
        if trimmed.is_empty() {
            bail!("SEMVER_INVALID: Empty version string");
        }
        let (core, pre) = match trimmed.split_once('-') {
            Some((c, p)) => (c, Some(p.to_string())),
            None => (trimmed, None),
        };
        let parts: Vec<&str> = core.split('.').collect();
        if parts.len() != 3 {
            bail!(
                "SEMVER_INVALID: Expected MAJOR.MINOR.PATCH format, got '{}'",
                raw
            );
        }
        let major = parts[0]
            .parse::<u64>()
            .map_err(|_| anyhow::anyhow!("SEMVER_INVALID: bad major in '{}'", raw))?;
        let minor = parts[1]
            .parse::<u64>()
            .map_err(|_| anyhow::anyhow!("SEMVER_INVALID: bad minor in '{}'", raw))?;
        let patch = parts[2]
            .parse::<u64>()
            .map_err(|_| anyhow::anyhow!("SEMVER_INVALID: bad patch in '{}'", raw))?;
        Ok(Self {
            major,
            minor,
            patch,
            prerelease: pre,
        })
    }
}

impl Ord for SemVer {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch)
            .cmp(&(other.major, other.minor, other.patch))
            .then_with(|| match (&self.prerelease, &other.prerelease) {
                (None, None) => Ordering::Equal,
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (Some(a), Some(b)) => a.cmp(b),
            })
    }
}

impl PartialOrd for SemVer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReleaseLifecycleState {
    Draft,
    #[serde(rename = "RC")]
    Rc,
    Published,
    Deprecated,
    RolledBack,
}

impl ReleaseLifecycleState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "Draft",
            Self::Rc => "RC",
            Self::Published => "Published",
            Self::Deprecated => "Deprecated",
            Self::RolledBack => "RolledBack",
        }
    }

    pub fn from_str_checked(s: &str) -> Result<Self> {
        match s.trim() {
            "Draft" => Ok(Self::Draft),
            "RC" | "Rc" => Ok(Self::Rc),
            "Published" => Ok(Self::Published),
            "Deprecated" => Ok(Self::Deprecated),
            "RolledBack" => Ok(Self::RolledBack),
            other => bail!("INVALID_LIFECYCLE_STATE: '{}'", other),
        }
    }

    pub fn can_transition_to(&self, next: Self) -> bool {
        matches!(
            (*self, next),
            (Self::Draft, Self::Rc | Self::Deprecated)
                | (Self::Rc, Self::Published | Self::Deprecated | Self::Draft)
                | (Self::Published, Self::Deprecated | Self::RolledBack)
                | (Self::Deprecated, Self::Published | Self::RolledBack)
                | (Self::RolledBack, Self::Published)
        )
    }
}

/// Version compatibility matrix for Project, Workflow, UI Schema, Module API, SQLite Schema, and Runtime
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilityVersions {
    pub project_schema_version: u32,
    pub workflow_schema_version: u32,
    pub ui_schema_version: u32,
    pub module_api_version: String,
    pub min_runtime_version: String,
    #[serde(default = "default_sqlite_schema_version")]
    pub sqlite_schema_version: u32,
}

impl Default for CompatibilityVersions {
    fn default() -> Self {
        Self {
            project_schema_version: 1,
            workflow_schema_version: SUPPORTED_WORKFLOW_SCHEMA_VERSION,
            ui_schema_version: SUPPORTED_UI_SCHEMA_VERSION,
            module_api_version: SUPPORTED_MODULE_API_VERSION.to_string(),
            min_runtime_version: CURRENT_RUNTIME_VERSION.to_string(),
            sqlite_schema_version: CURRENT_SQLITE_SCHEMA_VERSION,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeCompatibilityProfile {
    pub runtime_version: String,
    pub max_workflow_schema_version: u32,
    pub max_ui_schema_version: u32,
    pub module_api_version: String,
    pub max_sqlite_schema_version: u32,
}

impl Default for RuntimeCompatibilityProfile {
    fn default() -> Self {
        Self {
            runtime_version: CURRENT_RUNTIME_VERSION.to_string(),
            max_workflow_schema_version: SUPPORTED_WORKFLOW_SCHEMA_VERSION,
            max_ui_schema_version: SUPPORTED_UI_SCHEMA_VERSION,
            module_api_version: SUPPORTED_MODULE_API_VERSION.to_string(),
            max_sqlite_schema_version: CURRENT_SQLITE_SCHEMA_VERSION,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilityDimensionCheck {
    pub dimension: String,
    pub required_or_bundle: String,
    pub runtime_supported: String,
    pub compatible: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilityReport {
    pub compatible: bool,
    pub checks: Vec<CompatibilityDimensionCheck>,
    pub blocking_reasons: Vec<String>,
}

/// Evaluates all 5 required compatibility dimensions:
/// Runtime Version, Workflow Schema Version, UI Schema Version, Module API Version, and SQLite Schema Version.
pub fn evaluate_bundle_compatibility(
    compat: &CompatibilityVersions,
    ui_schema_version: u32,
    workflow_versions: &[u32],
    profile: &RuntimeCompatibilityProfile,
) -> CompatibilityReport {
    let mut checks = Vec::new();
    let mut blocking = Vec::new();

    // 1. Runtime Version
    let rt_ok = match (
        SemVer::parse(&profile.runtime_version),
        SemVer::parse(&compat.min_runtime_version),
    ) {
        (Ok(cur), Ok(min_req)) => cur >= min_req,
        _ => false,
    };
    if !rt_ok {
        blocking.push(format!(
            "RUNTIME_VERSION_INCOMPATIBLE: runtime v{} < required minRuntimeVersion v{}",
            profile.runtime_version, compat.min_runtime_version
        ));
    }
    checks.push(CompatibilityDimensionCheck {
        dimension: "RuntimeVersion".to_string(),
        required_or_bundle: format!(">= {}", compat.min_runtime_version),
        runtime_supported: profile.runtime_version.clone(),
        compatible: rt_ok,
        detail: if rt_ok { "Compatible".to_string() } else { "Runtime upgrade required".to_string() },
    });

    // 2. Workflow Schema Version (manifest + all individual workflows)
    let wf_max = workflow_versions
        .iter()
        .copied()
        .max()
        .unwrap_or(compat.workflow_schema_version);
    let wf_ok = compat.workflow_schema_version <= profile.max_workflow_schema_version
        && wf_max <= profile.max_workflow_schema_version;
    if !wf_ok {
        blocking.push(format!(
            "WORKFLOW_SCHEMA_INCOMPATIBLE: bundle workflow schema v{} / max node v{} > supported v{}",
            compat.workflow_schema_version, wf_max, profile.max_workflow_schema_version
        ));
    }
    checks.push(CompatibilityDimensionCheck {
        dimension: "WorkflowSchemaVersion".to_string(),
        required_or_bundle: format!("v{} (max {})", compat.workflow_schema_version, wf_max),
        runtime_supported: format!("<= v{}", profile.max_workflow_schema_version),
        compatible: wf_ok,
        detail: if wf_ok { "Compatible".to_string() } else { "Unsupported workflow schema version".to_string() },
    });

    // 3. UI Schema Version
    let ui_ok = compat.ui_schema_version <= profile.max_ui_schema_version
        && ui_schema_version <= profile.max_ui_schema_version;
    if !ui_ok {
        blocking.push(format!(
            "UI_SCHEMA_INCOMPATIBLE: bundle UI schema v{} (doc v{}) > supported v{}",
            compat.ui_schema_version, ui_schema_version, profile.max_ui_schema_version
        ));
    }
    checks.push(CompatibilityDimensionCheck {
        dimension: "UiSchemaVersion".to_string(),
        required_or_bundle: format!("v{} (ui v{})", compat.ui_schema_version, ui_schema_version),
        runtime_supported: format!("<= v{}", profile.max_ui_schema_version),
        compatible: ui_ok,
        detail: if ui_ok { "Compatible".to_string() } else { "Unsupported UI schema version".to_string() },
    });

    // 4. Module API Version (same major, runtime minor >= required minor)
    let mod_ok = match (
        SemVer::parse(&profile.module_api_version),
        SemVer::parse(&compat.module_api_version),
    ) {
        (Ok(cur), Ok(req)) => cur.major == req.major && cur >= req,
        _ => false,
    };
    if !mod_ok {
        blocking.push(format!(
            "MODULE_API_INCOMPATIBLE: required moduleApiVersion {} vs runtime {}",
            compat.module_api_version, profile.module_api_version
        ));
    }
    checks.push(CompatibilityDimensionCheck {
        dimension: "ModuleApiVersion".to_string(),
        required_or_bundle: compat.module_api_version.clone(),
        runtime_supported: profile.module_api_version.clone(),
        compatible: mod_ok,
        detail: if mod_ok { "Compatible".to_string() } else { "Breaking Module API version".to_string() },
    });

    // 5. SQLite Schema Version
    let sql_ok = compat.sqlite_schema_version <= profile.max_sqlite_schema_version;
    if !sql_ok {
        blocking.push(format!(
            "SQLITE_SCHEMA_INCOMPATIBLE: bundle sqliteSchemaVersion v{} > runtime supported max v{}",
            compat.sqlite_schema_version, profile.max_sqlite_schema_version
        ));
    }
    checks.push(CompatibilityDimensionCheck {
        dimension: "SqliteSchemaVersion".to_string(),
        required_or_bundle: format!("v{}", compat.sqlite_schema_version),
        runtime_supported: format!("<= v{}", profile.max_sqlite_schema_version),
        compatible: sql_ok,
        detail: if sql_ok { "Compatible".to_string() } else { "Target runtime cannot safely open newer DB schema".to_string() },
    });

    CompatibilityReport {
        compatible: blocking.is_empty(),
        checks,
        blocking_reasons: blocking,
    }
}

/// Guards SQLite downgrade safety BEFORE launching an older EXE against an existing DB.
pub fn check_sqlite_downgrade_compatibility(
    live_db_max_version: u32,
    target_exe_supported_max_db_version: u32,
) -> Result<()> {
    if live_db_max_version > target_exe_supported_max_db_version {
        bail!(
            "SQLITE_DOWNGRADE_INCOMPATIBLE: Live database is at schema v{} which exceeds rollback EXE supported max v{}. Restore pre-update SQLite backup first!",
            live_db_max_version,
            target_exe_supported_max_db_version
        );
    }
    Ok(())
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SigningMode {
    #[default]
    UnsignedInternal,
    SelfSignedQa,
    PublicSigned,
}

impl SigningMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::UnsignedInternal => "unsigned_internal",
            Self::SelfSignedQa => "self_signed_qa",
            Self::PublicSigned => "public_signed",
        }
    }

    pub fn from_str_checked(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "unsigned_internal" | "unsigned" => Ok(Self::UnsignedInternal),
            "self_signed_qa" | "self_signed" => Ok(Self::SelfSignedQa),
            "public_signed" | "public" => Ok(Self::PublicSigned),
            other => bail!("INVALID_SIGNING_MODE: '{}'", other),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthenticodeTrustStatus {
    Unsigned,
    SelfSignedUntrustedRoot,
    TrustedPublicChain,
    TamperedOrInvalid,
    ExpiredOrTimestampError,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticodeVerificationReport {
    pub signed: bool,
    pub trust_status: AuthenticodeTrustStatus,
    pub trusted_public_release: bool,
    pub win_trust_error_code: u32,
    pub signer_subject: Option<String>,
    pub signer_issuer: Option<String>,
    pub signer_thumbprint: Option<String>,
    pub timestamp_status: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PeVersionInfoSpec {
    pub company_name: String,
    pub file_description: String,
    pub file_version: String,
    pub internal_name: String,
    pub legal_copyright: String,
    pub original_filename: String,
    pub product_name: String,
    pub product_version: String,
}

impl PeVersionInfoSpec {
    pub fn parse_quad_version(raw: &str) -> [u16; 4] {
        let core = raw.trim().trim_start_matches('v').split('-').next().unwrap_or("0.1.0");
        let nums: Vec<u16> = core
            .split('.')
            .filter_map(|p| p.trim().parse::<u16>().ok())
            .take(4)
            .collect();
        [
            *nums.first().unwrap_or(&0),
            *nums.get(1).unwrap_or(&0),
            *nums.get(2).unwrap_or(&0),
            *nums.get(3).unwrap_or(&0),
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PublisherIdentitySpec {
    pub publisher_id: String,
    pub display_name: String,
    pub organization: String,
    pub website: Option<String>,
    pub support_contact: Option<String>,
    pub copyright: String,
    pub ed25519_key_id: String,
    pub ed25519_pubkey_hex: String,
    /// Non-secret cert reference ONLY (thumbprint or `CERT_STORE://CurrentUser/My/<sha1>` or PFX path; NEVER password/key!)
    pub authenticode_cert_ref: Option<String>,
    pub release_channel: String,
    pub signing_policy: SigningMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KeyRotationDelegation {
    pub publisher_id: String,
    pub product_slug: String,
    pub old_pubkey_hex: String,
    pub new_pubkey_hex: String,
    pub issued_at_utc: String,
    pub expires_at_utc: String,
    pub nonce: String,
    pub signature_by_old_key_hex: String,
}

pub fn canonical_key_rotation_message(
    publisher_id: &str,
    product_slug: &str,
    old_pubkey_hex: &str,
    new_pubkey_hex: &str,
    issued_at_utc: &str,
    expires_at_utc: &str,
    nonce: &str,
) -> Vec<u8> {
    format!(
        "FARMERPRO-KEY-ROTATION-V1|pub={}|slug={}|old={}|new={}|iat={}|exp={}|nonce={}",
        publisher_id.trim(),
        product_slug.trim(),
        old_pubkey_hex.trim().to_ascii_lowercase(),
        new_pubkey_hex.trim().to_ascii_lowercase(),
        issued_at_utc.trim(),
        expires_at_utc.trim(),
        nonce.trim()
    )
    .into_bytes()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseSigningMetadata {
    pub signing_mode: SigningMode,
    pub publisher_id: String,
    pub publisher_organization: String,
    pub release_channel: String,
    pub authenticode_trust_status: AuthenticodeTrustStatus,
    pub trusted_public_release: bool,
    pub signer_thumbprint: Option<String>,
    pub signer_subject: Option<String>,
    pub version_info: PeVersionInfoSpec,
    #[serde(default)]
    pub key_rotation: Option<KeyRotationDelegation>,
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
    #[serde(default)]
    pub publisher: Option<PublisherIdentitySpec>,
    #[serde(default)]
    pub signing: Option<ReleaseSigningMetadata>,
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
    #[serde(default)]
    pub third_party_notices_present: bool,
    #[serde(default)]
    pub updater_bundled: bool,
    #[serde(default)]
    pub requires_internet_if_webview2_missing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DistributedFileEntry {
    pub rel_path: String,
    pub size_bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PortableRuntimeRequirements {
    pub requires_rust: bool,
    pub requires_cargo: bool,
    pub requires_node: bool,
    pub requires_python: bool,
    pub requires_windows_sdk: bool,
    pub webview2_strategy: String,
    pub offline_ready: bool,
    #[serde(default)]
    pub requires_internet_if_webview2_missing: bool,
    pub data_isolation_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleasePackageManifest {
    pub package_schema_version: u32,
    pub package_type: String,
    pub project_slug: String,
    pub product_name: String,
    pub version: String,
    pub exe_name: String,
    pub final_exe_sha256: String,
    pub built_at_utc: String,
    pub requirements: PortableRuntimeRequirements,
    pub publisher: Option<PublisherIdentitySpec>,
    pub signing: Option<ReleaseSigningMetadata>,
    pub distributed_files: Vec<DistributedFileEntry>,
    pub third_party_notices: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TimestampVerificationPolicy {
    #[default]
    AllowOfflineUnsignedTimestamp,
    RequireTimestampWhenPublicSigned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalReleaseVerificationReport {
    pub verified: bool,
    pub release_dir: String,
    pub exe_name: String,
    pub final_exe_sha256: String,
    pub authenticode_checked: bool,
    pub authenticode_trust_status: AuthenticodeTrustStatus,
    pub authenticode_trusted_public_chain: bool,
    pub ed25519_bundle_verified: bool,
    pub pe_root_pubkey_matches: bool,
    pub pe_version_info_valid: bool,
    pub publisher_identity_matches_cert: bool,
    pub publisher_identity_detail: String,
    pub distributed_files_checked: usize,
    pub distributed_file_mismatches: Vec<String>,
    pub issues: Vec<String>,
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
    audit_portable_environment_with_webview2(release_dir, project_slug, wv2_ok, wv2_note)
}

pub fn audit_portable_environment_with_webview2(
    release_dir: &Path,
    project_slug: &str,
    wv2_ok: bool,
    wv2_note: String,
) -> PortableDependencyAudit {
    let bin_dir = release_dir.join("bin");
    let adb_exe = bin_dir.join("adb.exe").exists();
    let adb_dll1 = bin_dir.join("AdbWinApi.dll").exists();
    let adb_dll2 = bin_dir.join("AdbWinUsbApi.dll").exists();

    let data_existed_before = release_dir.join("data").exists();
    let (db_path, exe_dir_writable, fallback_used, probe_err) =
        resolve_writable_runtime_db_path(release_dir, project_slug);
    if !data_existed_before {
        let probe_data_dir = release_dir.join("data");
        if probe_data_dir.is_dir() {
            let _ = std::fs::remove_dir(&probe_data_dir);
        }
    }

    let ps_available = std::env::var("SystemRoot")
        .map(|sr| {
            Path::new(&sr)
                .join("System32\\WindowsPowerShell\\v1.0\\powershell.exe")
                .exists()
        })
        .unwrap_or(true);

    let notices_ok = release_dir.join("THIRD_PARTY_NOTICES.txt").exists();
    let updater_ok = release_dir.join("updater").join("fp-updater.exe").exists()
        || release_dir.join("fp-updater.exe").exists();
    let offline_execution_ready = wv2_ok && adb_exe && adb_dll1 && adb_dll2 && updater_ok;

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
        offline_execution_ready,
        third_party_notices_present: notices_ok,
        updater_bundled: updater_ok,
        requires_internet_if_webview2_missing: true,
    }
}

/// Public WebView2 probe callable by Mini Runtime, Studio, and Tests.
pub fn detect_webview2_runtime() -> (bool, String) {
    let default_paths = [
        Path::new(r"C:\Program Files (x86)\Microsoft\EdgeWebView\Application"),
        Path::new(r"C:\Program Files\Microsoft\EdgeWebView\Application"),
    ];
    let reg_ver = probe_webview2_registry();
    detect_webview2_runtime_at(&default_paths, reg_ver.as_deref())
}

fn probe_webview2_registry() -> Option<String> {
    #[cfg(windows)]
    {
        for key in [
            r"HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
            r"HKCU\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
            r"HKLM\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
        ] {
            if let Ok(out) = std::process::Command::new("reg")
                .args(["query", key, "/v", "pv"])
                .output()
            {
                if out.status.success() {
                    let s = String::from_utf8_lossy(&out.stdout);
                    for line in s.lines() {
                        if let Some(idx) = line.find("REG_SZ") {
                            let v = line[idx + 6..].trim().to_string();
                            if !v.is_empty() && v != "0.0.0.0" {
                                return Some(v);
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

pub fn detect_webview2_runtime_at(paths: &[&Path], reg_version: Option<&str>) -> (bool, String) {
    if let Some(v) = reg_version.map(|s| s.trim()).filter(|s| !s.is_empty()) {
        return (
            true,
            format!(
                "Evergreen Runtime v{} (Registry Client Verified; offline target requires WebView2 Evergreen/Bootstrapper if missing)",
                v
            ),
        );
    }
    for p in paths {
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
        "WEBVIEW2_MISSING: Microsoft Edge WebView2 Runtime not found. Install MicrosoftEdgeWebView2Setup.exe (included in FarmerPro Installer) or download Evergreen Standalone Installer before launching GUI.".to_string(),
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

    #[test]
    fn test_p6_webview2_missing_reports_not_offline_ready() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_base = std::env::temp_dir().join(format!(
            "fp_wv2_missing_test_{}_{}",
            std::process::id(),
            nanos
        ));
        std::fs::create_dir_all(temp_base.join("bin")).unwrap();
        std::fs::create_dir_all(temp_base.join("updater")).unwrap();
        std::fs::write(temp_base.join("bin").join("adb.exe"), b"adb").unwrap();
        std::fs::write(temp_base.join("bin").join("AdbWinApi.dll"), b"dll1").unwrap();
        std::fs::write(temp_base.join("bin").join("AdbWinUsbApi.dll"), b"dll2").unwrap();
        std::fs::write(temp_base.join("updater").join("fp-updater.exe"), b"upd").unwrap();
        std::fs::write(temp_base.join("THIRD_PARTY_NOTICES.txt"), b"notices").unwrap();

        let missing_dir = temp_base.join("nonexistent_webview2");
        let (wv2_ok, wv2_note) = detect_webview2_runtime_at(&[missing_dir.as_path()], None);
        assert!(!wv2_ok);
        assert!(wv2_note.contains("WEBVIEW2_MISSING"));

        let audit = audit_portable_environment_with_webview2(
            &temp_base,
            "wv2-test",
            wv2_ok,
            wv2_note,
        );
        assert!(!audit.webview2_evergreen_installed);
        assert!(!audit.offline_execution_ready);
        assert!(audit.requires_internet_if_webview2_missing);
        assert!(audit.adb_exe_present);
        assert!(audit.updater_bundled);
        assert!(audit.third_party_notices_present);

        let _ = std::fs::remove_dir_all(&temp_base);
    }
}
