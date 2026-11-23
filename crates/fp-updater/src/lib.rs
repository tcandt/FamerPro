use anyhow::{bail, Context, Result};
use chrono::Utc;
use fp_bundle_contract::{
    check_sqlite_downgrade_compatibility, AuthenticodeTrustStatus, AuthenticodeVerificationReport,
    CompatibilityVersions, MiniToolUiSchema, ReleaseBundleManifest,
    RuntimeCompatibilityProfile, SemVer, SigningMode, CURRENT_SQLITE_SCHEMA_VERSION,
};
use fp_bundle_verify::{
    read_pe_embedded_pubkey_from_exe, validate_safe_relative_path, verify_authenticode_pe,
    verify_bundle_compatibility_gate, verify_key_rotation_delegation, SignedBundleEnvelope,
    PE_ED25519_PUBKEY_RESOURCE_ID,
};
use fp_storage::StorageManager;
use fp_workflow_schema::WorkflowDocument;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateStage {
    Idle,
    PreflightVerified,
    QuiescedAndBackedUp,
    Staged,
    StagedVerified,
    Activated,
    Completed,
    RolledBack,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum FaultInjectionPoint {
    AfterPreflight,
    AfterBackup,
    AfterStage,
    AfterActivate,
    SimulateMigrationFailure,
}

impl FaultInjectionPoint {
    pub fn parse(raw: &str) -> Result<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "afterpreflight" | "after_preflight" => Ok(Self::AfterPreflight),
            "afterbackup" | "after_backup" => Ok(Self::AfterBackup),
            "afterstage" | "after_stage" => Ok(Self::AfterStage),
            "afteractivate" | "after_activate" => Ok(Self::AfterActivate),
            "simulatemigrationfailure" | "simulate_migration_failure" => {
                Ok(Self::SimulateMigrationFailure)
            }
            _ => bail!("Unknown fault injection point: '{}'", raw),
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::AfterPreflight => "AfterPreflight",
            Self::AfterBackup => "AfterBackup",
            Self::AfterStage => "AfterStage",
            Self::AfterActivate => "AfterActivate",
            Self::SimulateMigrationFailure => "SimulateMigrationFailure",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ActiveVersionPointer {
    pub project_slug: String,
    pub active_version: String,
    pub previous_version: Option<String>,
    pub exe_name: String,
    pub sqlite_schema_version: u32,
    pub min_runtime_version: String,
    #[serde(default)]
    pub trusted_pubkey_hex: Option<String>,
    pub post_update_auto_resume_blocked: bool,
    pub blocked_job_ids: Vec<String>,
    pub backup_db_path: Option<String>,
    pub activated_at_utc: String,
    pub journal_id: String,
}

/// Persistent root-of-trust anchor ledger kept in `.updater/trust-state.json`,
/// shadow-backed at `.updater/trust-state.bak.json`, and strictly monotonic-guarded
/// by SQLite `.updater/trust-ledger.db` against file rollback/downgrade attacks.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PersistentTrustState {
    #[serde(default = "default_trust_epoch")]
    pub trust_epoch: u64,
    pub publisher_id: Option<String>,
    pub trusted_pubkeys: Vec<String>,
    pub revoked_pubkeys: Vec<String>,
    pub used_rotation_nonces: Vec<String>,
    #[serde(default)]
    pub previous_state_sha256: Option<String>,
    pub updated_at_utc: String,
}

fn default_trust_epoch() -> u64 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateJournal {
    pub journal_id: String,
    pub operation: String,
    pub project_slug: String,
    pub source_version: Option<String>,
    pub target_version: String,
    pub source_release_dir: String,
    pub stage: UpdateStage,
    pub backup_db_path: Option<String>,
    pub live_db_path: String,
    pub staged_staging_dir: Option<String>,
    pub staged_version_dir: String,
    pub previous_active_pointer: Option<ActiveVersionPointer>,
    pub target_compat: CompatibilityVersions,
    pub auto_resume_blocked_jobs: Vec<String>,
    pub last_error: Option<String>,
    pub updated_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOutcome {
    pub operation: String,
    pub status: String,
    pub project_slug: String,
    pub previous_version: Option<String>,
    pub active_version: String,
    pub stage_reached: UpdateStage,
    pub backup_db_path: Option<String>,
    pub backup_verified_ok: bool,
    pub signature_verified_ok: bool,
    pub compatibility_verified_ok: bool,
    pub post_update_auto_resume_blocked: bool,
    pub blocked_job_ids: Vec<String>,
    #[serde(default)]
    pub binary_rollback_only: bool,
    #[serde(default)]
    pub database_restored: bool,
    #[serde(default)]
    pub pre_rollback_safety_db_path: Option<String>,
    pub journal_path: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconcileOutcome {
    pub status: String,
    pub action_taken: String,
    pub recovered_from_stage: Option<UpdateStage>,
    pub active_version: Option<String>,
    pub db_integrity_ok: bool,
    pub journal_path: String,
    pub detail: String,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateExecutionOptions {
    pub pause_after_stage_ms: Option<(FaultInjectionPoint, u64)>,
    pub crash_after_stage: Option<FaultInjectionPoint>,
    pub simulate_migration_failure: bool,
    pub skip_child_verify_exec: bool,
}

#[derive(Debug, Clone)]
pub struct VerifiedCandidateRelease {
    pub manifest: ReleaseBundleManifest,
    pub ui_schema: MiniToolUiSchema,
    pub workflows: Vec<WorkflowDocument>,
    pub exe_path: PathBuf,
    pub pe_pubkey_hex: String,
    pub authenticode: AuthenticodeVerificationReport,
    pub rotated_from_pubkey_hex: Option<String>,
    pub publisher_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalTrustAnchorRecord {
    pub anchor_schema_version: u32,
    pub install_anchor_id: String,
    pub trust_epoch: u64,
    pub publisher_id: Option<String>,
    pub trusted_pubkeys: Vec<String>,
    pub revoked_pubkeys: Vec<String>,
    pub used_rotation_nonces: Vec<String>,
    pub trust_state_sha256: String,
    pub updated_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallSecurityAuditReport {
    pub passed: bool,
    pub install_root: String,
    pub trust_epoch: u64,
    pub active_version: String,
    pub active_exe_sha256: String,
    pub pe_root_pubkey_hex: String,
    pub authenticode_trust_status: AuthenticodeTrustStatus,
    pub external_anchor_verified: bool,
    pub versions_scanned: Vec<String>,
    pub rogue_or_tampered_versions: Vec<String>,
    pub issues: Vec<String>,
}

pub struct MiniToolUpdater {
    pub install_root: PathBuf,
    pub external_anchor_root: Option<PathBuf>,
    pub enable_registry_anchor: bool,
}

fn default_external_anchor_root() -> PathBuf {
    if let Ok(custom) = std::env::var("FP_TRUST_ANCHOR_DIR") {
        let trimmed = custom.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        let trimmed = local.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed).join("FarmerPro").join("TrustAnchors");
        }
    }
    std::env::temp_dir().join("FarmerProTrustAnchors")
}

impl MiniToolUpdater {
    pub fn new<P: AsRef<Path>>(install_root: P) -> Self {
        Self {
            install_root: install_root.as_ref().to_path_buf(),
            external_anchor_root: Some(default_external_anchor_root()),
            enable_registry_anchor: cfg!(all(windows, not(test))),
        }
    }

    pub fn new_with_external_anchor<P: AsRef<Path>, A: AsRef<Path>>(
        install_root: P,
        anchor_root: A,
    ) -> Self {
        Self {
            install_root: install_root.as_ref().to_path_buf(),
            external_anchor_root: Some(anchor_root.as_ref().to_path_buf()),
            enable_registry_anchor: false,
        }
    }

    pub fn install_anchor_id(&self) -> String {
        let norm = self
            .install_root
            .to_string_lossy()
            .replace('\\', "/")
            .trim_end_matches('/')
            .to_ascii_lowercase();
        let digest = fp_bundle_verify::sha256_hex(norm.as_bytes());
        digest.chars().take(16).collect()
    }

    pub fn external_anchor_file_path(&self) -> Option<PathBuf> {
        self.external_anchor_root
            .as_ref()
            .map(|r| r.join(format!("{}.json", self.install_anchor_id())))
    }

    fn hkcu_registry_key(&self) -> String {
        format!(
            r"HKCU\Software\FarmerPro\TrustAnchors\{}",
            self.install_anchor_id()
        )
    }

    fn read_hkcu_trust_anchor(&self) -> Option<(u64, Vec<String>, Vec<String>)> {
        if !self.enable_registry_anchor {
            return None;
        }
        #[cfg(windows)]
        {
            let key = self.hkcu_registry_key();
            let out = std::process::Command::new("reg")
                .args(["query", &key])
                .output()
                .ok()?;
            if !out.status.success() {
                return None;
            }
            let txt = String::from_utf8_lossy(&out.stdout);
            let mut epoch: Option<u64> = None;
            let mut revoked: Vec<String> = Vec::new();
            let mut nonces: Vec<String> = Vec::new();
            for line in txt.lines() {
                let t = line.trim();
                if let Some(rest) = t.strip_prefix("TrustEpoch") {
                    if let Some(val) = rest.split("REG_SZ").nth(1) {
                        epoch = val.trim().parse::<u64>().ok();
                    }
                } else if let Some(rest) = t.strip_prefix("RevokedPubkeysJson") {
                    if let Some(val) = rest.split("REG_SZ").nth(1) {
                        revoked = serde_json::from_str(val.trim()).unwrap_or_default();
                    }
                } else if let Some(rest) = t.strip_prefix("UsedNoncesJson") {
                    if let Some(val) = rest.split("REG_SZ").nth(1) {
                        nonces = serde_json::from_str(val.trim()).unwrap_or_default();
                    }
                }
            }
            epoch.map(|e| (e, revoked, nonces))
        }
        #[cfg(not(windows))]
        {
            None
        }
    }

    fn write_hkcu_trust_anchor(&self, record: &ExternalTrustAnchorRecord) {
        if !self.enable_registry_anchor {
            return;
        }
        #[cfg(windows)]
        {
            let key = self.hkcu_registry_key();
            let epoch_str = record.trust_epoch.to_string();
            let rev_json = serde_json::to_string(&record.revoked_pubkeys).unwrap_or_else(|_| "[]".into());
            let nonces_json = serde_json::to_string(&record.used_rotation_nonces).unwrap_or_else(|_| "[]".into());
            let _ = std::process::Command::new("reg")
                .args(["add", &key, "/v", "TrustEpoch", "/t", "REG_SZ", "/d", &epoch_str, "/f"])
                .output();
            let _ = std::process::Command::new("reg")
                .args(["add", &key, "/v", "StateSha256", "/t", "REG_SZ", "/d", &record.trust_state_sha256, "/f"])
                .output();
            let _ = std::process::Command::new("reg")
                .args(["add", &key, "/v", "RevokedPubkeysJson", "/t", "REG_SZ", "/d", &rev_json, "/f"])
                .output();
            let _ = std::process::Command::new("reg")
                .args(["add", &key, "/v", "UsedNoncesJson", "/t", "REG_SZ", "/d", &nonces_json, "/f"])
                .output();
        }
    }

    fn read_external_trust_hwm(&self) -> Option<(u64, Vec<String>, Vec<String>)> {
        let mut max_epoch: Option<u64> = None;
        let mut revoked: Vec<String> = Vec::new();
        let mut nonces: Vec<String> = Vec::new();

        if let Some(path) = self.external_anchor_file_path() {
            if path.exists() {
                if let Ok(bytes) = fs::read(&path) {
                    if let Ok(rec) = serde_json::from_slice::<ExternalTrustAnchorRecord>(&bytes) {
                        max_epoch = Some(max_epoch.map_or(rec.trust_epoch, |e| e.max(rec.trust_epoch)));
                        for r in rec.revoked_pubkeys {
                            if !revoked.iter().any(|x| x.eq_ignore_ascii_case(&r)) {
                                revoked.push(r);
                            }
                        }
                        for n in rec.used_rotation_nonces {
                            if !nonces.contains(&n) {
                                nonces.push(n);
                            }
                        }
                    }
                }
            }
        }
        if let Some((reg_ep, reg_rev, reg_nonces)) = self.read_hkcu_trust_anchor() {
            max_epoch = Some(max_epoch.map_or(reg_ep, |e| e.max(reg_ep)));
            for r in reg_rev {
                if !revoked.iter().any(|x| x.eq_ignore_ascii_case(&r)) {
                    revoked.push(r);
                }
            }
            for n in reg_nonces {
                if !nonces.contains(&n) {
                    nonces.push(n);
                }
            }
        }
        max_epoch.map(|e| (e, revoked, nonces))
    }

    fn write_external_trust_anchor(&self, state: &PersistentTrustState, state_sha: &str) -> Result<()> {
        let record = ExternalTrustAnchorRecord {
            anchor_schema_version: 1,
            install_anchor_id: self.install_anchor_id(),
            trust_epoch: state.trust_epoch,
            publisher_id: state.publisher_id.clone(),
            trusted_pubkeys: state.trusted_pubkeys.clone(),
            revoked_pubkeys: state.revoked_pubkeys.clone(),
            used_rotation_nonces: state.used_rotation_nonces.clone(),
            trust_state_sha256: state_sha.to_string(),
            updated_at_utc: state.updated_at_utc.clone(),
        };
        if let Some(path) = self.external_anchor_file_path() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let bytes = serde_json::to_vec_pretty(&record)?;
            atomic_write_file(&path, &bytes)?;
        }
        self.write_hkcu_trust_anchor(&record);
        Ok(())
    }

    pub fn versions_dir(&self) -> PathBuf {
        self.install_root.join("versions")
    }

    pub fn data_dir(&self) -> PathBuf {
        self.install_root.join("data")
    }

    pub fn backups_dir(&self) -> PathBuf {
        self.data_dir().join("backups")
    }

    pub fn updater_dir(&self) -> PathBuf {
        self.install_root.join(".updater")
    }

    pub fn journal_path(&self) -> PathBuf {
        self.updater_dir().join("update-journal.json")
    }

    pub fn trust_state_path(&self) -> PathBuf {
        self.updater_dir().join("trust-state.json")
    }

    pub fn trust_state_backup_path(&self) -> PathBuf {
        self.updater_dir().join("trust-state.bak.json")
    }

    pub fn trust_ledger_db_path(&self) -> PathBuf {
        self.updater_dir().join("trust-ledger.db")
    }

    fn ensure_trust_ledger_schema(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             CREATE TABLE IF NOT EXISTS updater_trust_ledger (
                 id INTEGER PRIMARY KEY CHECK (id = 1),
                 trust_epoch INTEGER NOT NULL,
                 publisher_id TEXT,
                 trusted_pubkeys_json TEXT NOT NULL,
                 revoked_pubkeys_json TEXT NOT NULL,
                 used_nonces_json TEXT NOT NULL,
                 state_sha256 TEXT NOT NULL,
                 updated_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS updater_trust_history (
                 seq INTEGER PRIMARY KEY AUTOINCREMENT,
                 trust_epoch INTEGER NOT NULL,
                 event_type TEXT NOT NULL,
                 detail TEXT NOT NULL,
                 state_sha256 TEXT NOT NULL,
                 recorded_at TEXT NOT NULL
             );",
        )?;
        Ok(())
    }

    fn read_trust_ledger_row(conn: &Connection) -> Result<Option<(u64, Vec<String>, Vec<String>)>> {
        let mut st = conn.prepare(
            "SELECT trust_epoch, revoked_pubkeys_json, used_nonces_json FROM updater_trust_ledger WHERE id = 1;",
        )?;
        let mut rows = st.query([])?;
        if let Some(r) = rows.next()? {
            let epoch: i64 = r.get(0)?;
            let rev_json: String = r.get(1)?;
            let nonces_json: String = r.get(2)?;
            let rev: Vec<String> = serde_json::from_str(&rev_json).unwrap_or_default();
            let nonces: Vec<String> = serde_json::from_str(&nonces_json).unwrap_or_default();
            Ok(Some((epoch as u64, rev, nonces)))
        } else {
            Ok(None)
        }
    }

    fn sync_trust_ledger_db(&self, state: &PersistentTrustState, event_type: &str, detail: &str) -> Result<()> {
        if let Some((ext_epoch, ext_revoked, ext_nonces)) = self.read_external_trust_hwm() {
            if state.trust_epoch < ext_epoch {
                bail!(
                    "FULL_TRIAD_ROLLBACK_DETECTED: attempted to write trust_epoch {} < external anchor {}",
                    state.trust_epoch,
                    ext_epoch
                );
            }
            for r in &ext_revoked {
                if !state.revoked_pubkeys.iter().any(|x| x.eq_ignore_ascii_case(r)) {
                    bail!(
                        "FULL_TRIAD_ROLLBACK_DETECTED: externally revoked pubkey '{}' missing from proposed trust state",
                        r
                    );
                }
            }
            for n in &ext_nonces {
                if !state.used_rotation_nonces.contains(n) {
                    bail!(
                        "FULL_TRIAD_ROLLBACK_DETECTED: external anchor nonce '{}' missing from proposed trust state",
                        n
                    );
                }
            }
        }
        let dir = self.updater_dir();
        fs::create_dir_all(&dir)?;
        let mut conn = Connection::open(self.trust_ledger_db_path())?;
        Self::ensure_trust_ledger_schema(&conn)?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        if let Some((cur_epoch, cur_revoked, cur_nonces)) = Self::read_trust_ledger_row(&tx)? {
            if state.trust_epoch < cur_epoch {
                bail!(
                    "TRUST_STATE_ROLLBACK_DETECTED: attempted to lower trust_epoch {} < ledger {}",
                    state.trust_epoch,
                    cur_epoch
                );
            }
            for r in &cur_revoked {
                if !state.revoked_pubkeys.iter().any(|x| x.eq_ignore_ascii_case(r)) {
                    bail!(
                        "TRUST_STATE_ROLLBACK_DETECTED: revoked pubkey '{}' missing from proposed trust state",
                        r
                    );
                }
            }
            for n in &cur_nonces {
                if !state.used_rotation_nonces.contains(n) {
                    bail!(
                        "TRUST_STATE_ROLLBACK_DETECTED: used nonce '{}' missing from proposed trust state",
                        n
                    );
                }
            }
        }
        let trusted_json = serde_json::to_string(&state.trusted_pubkeys)?;
        let revoked_json = serde_json::to_string(&state.revoked_pubkeys)?;
        let nonces_json = serde_json::to_string(&state.used_rotation_nonces)?;
        let canonical = serde_json::to_vec(state)?;
        let state_sha = fp_bundle_verify::sha256_hex(&canonical);
        tx.execute(
            "INSERT INTO updater_trust_ledger (
                id, trust_epoch, publisher_id, trusted_pubkeys_json,
                revoked_pubkeys_json, used_nonces_json, state_sha256, updated_at
             ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
                trust_epoch = excluded.trust_epoch,
                publisher_id = excluded.publisher_id,
                trusted_pubkeys_json = excluded.trusted_pubkeys_json,
                revoked_pubkeys_json = excluded.revoked_pubkeys_json,
                used_nonces_json = excluded.used_nonces_json,
                state_sha256 = excluded.state_sha256,
                updated_at = excluded.updated_at;",
            rusqlite::params![
                state.trust_epoch as i64,
                state.publisher_id,
                trusted_json,
                revoked_json,
                nonces_json,
                state_sha,
                state.updated_at_utc
            ],
        )?;
        tx.execute(
            "INSERT INTO updater_trust_history (trust_epoch, event_type, detail, state_sha256, recorded_at)
             VALUES (?1, ?2, ?3, ?4, ?5);",
            rusqlite::params![
                state.trust_epoch as i64,
                event_type,
                detail,
                state_sha,
                Utc::now().to_rfc3339()
            ],
        )?;
        tx.commit()?;
        self.write_external_trust_anchor(state, &state_sha)?;
        Ok(())
    }

    pub fn read_trust_state(&self) -> Result<Option<PersistentTrustState>> {
        let p = self.trust_state_path();
        let state_opt: Option<PersistentTrustState> = if p.exists() {
            let bytes = fs::read(&p)
                .with_context(|| format!("Failed reading trust-state.json at {}", p.display()))?;
            Some(serde_json::from_slice(&bytes)?)
        } else if self.trust_state_backup_path().exists() {
            let bytes = fs::read(self.trust_state_backup_path())?;
            Some(serde_json::from_slice(&bytes)?)
        } else {
            None
        };

        let ledger_path = self.trust_ledger_db_path();
        let mut ledger_epoch_seen: Option<u64> = None;
        if ledger_path.exists() {
            let conn = Connection::open(&ledger_path)?;
            Self::ensure_trust_ledger_schema(&conn)?;
            if let Some((db_epoch, db_revoked, db_nonces)) = Self::read_trust_ledger_row(&conn)? {
                ledger_epoch_seen = Some(db_epoch);
                let Some(ref st) = state_opt else {
                    bail!(
                        "TRUST_STATE_MISSING_WITH_LEDGER: trust-state.json was deleted while trust-ledger.db records epoch {}",
                        db_epoch
                    );
                };
                if st.trust_epoch < db_epoch {
                    bail!(
                        "TRUST_STATE_ROLLBACK_DETECTED: local trust-state.json epoch {} < authoritative ledger epoch {}",
                        st.trust_epoch,
                        db_epoch
                    );
                }
                for r in &db_revoked {
                    if !st.revoked_pubkeys.iter().any(|x| x.eq_ignore_ascii_case(r)) {
                        bail!(
                            "TRUST_STATE_ROLLBACK_DETECTED: revoked pubkey '{}' was stripped from trust-state.json",
                            r
                        );
                    }
                }
                for n in &db_nonces {
                    if !st.used_rotation_nonces.contains(n) {
                        bail!(
                            "TRUST_STATE_ROLLBACK_DETECTED: committed rotation nonce '{}' was stripped from trust-state.json",
                            n
                        );
                    }
                }
            }
        }

        if let Some((ext_epoch, ext_revoked, ext_nonces)) = self.read_external_trust_hwm() {
            let Some(ref st) = state_opt else {
                bail!(
                    "FULL_TRIAD_ROLLBACK_DETECTED: local .updater trust store missing while external trust anchor records epoch {}",
                    ext_epoch
                );
            };
            if st.trust_epoch < ext_epoch {
                bail!(
                    "FULL_TRIAD_ROLLBACK_DETECTED: local .updater triad (epoch {}, db_epoch {:?}) < external trust anchor (epoch {})",
                    st.trust_epoch,
                    ledger_epoch_seen,
                    ext_epoch
                );
            }
            for r in &ext_revoked {
                if !st.revoked_pubkeys.iter().any(|x| x.eq_ignore_ascii_case(r)) {
                    bail!(
                        "FULL_TRIAD_ROLLBACK_DETECTED: revoked pubkey '{}' was stripped across local .updater triad vs external trust anchor",
                        r
                    );
                }
            }
            for n in &ext_nonces {
                if !st.used_rotation_nonces.contains(n) {
                    bail!(
                        "FULL_TRIAD_ROLLBACK_DETECTED: committed rotation nonce '{}' was stripped across local .updater triad vs external trust anchor",
                        n
                    );
                }
            }
        }

        Ok(state_opt)
    }

    /// Full P6.3 security audit across Trust Triad + External Anchor + active EXE + release manifests + all `versions/v*` directories.
    pub fn verify_install_security_audit(&self) -> Result<InstallSecurityAuditReport> {
        let trust_state = self.read_trust_state()?.ok_or_else(|| {
            anyhow::anyhow!("TRUST_STATE_MISSING: no persistent trust state found")
        })?;
        let active = self.read_active_pointer()?.ok_or_else(|| {
            anyhow::anyhow!("ACTIVE_VERSION_MISSING: no active_version.json in {}", self.install_root.display())
        })?;
        let auth = self.verify_installed_exe_integrity()?;
        let active_dir = self.versions_dir().join(format!("v{}", active.active_version));
        let verified_active = Self::verify_candidate_release_dir_with_anchor(
            &active_dir,
            &RuntimeCompatibilityProfile::default(),
            Some(&active.trusted_pubkey_hex.clone().unwrap_or_default()),
        )?;

        if trust_state
            .revoked_pubkeys
            .iter()
            .any(|r| r.eq_ignore_ascii_case(&verified_active.pe_pubkey_hex))
        {
            bail!(
                "KEY_REVOKED: active version '{}' is signed by revoked publisher key '{}'",
                active.active_version,
                verified_active.pe_pubkey_hex
            );
        }
        if !trust_state
            .trusted_pubkeys
            .iter()
            .any(|k| k.eq_ignore_ascii_case(&verified_active.pe_pubkey_hex))
        {
            bail!(
                "UNTRUSTED_KEY_REJECTED: active version key '{}' is not in trusted_pubkeys",
                verified_active.pe_pubkey_hex
            );
        }

        Self::verify_optional_release_manifest(&active_dir, &verified_active.exe_path)?;

        let mut versions_scanned = Vec::new();
        let mut rogue_versions = Vec::new();
        let mut issues = Vec::new();
        if let Ok(entries) = fs::read_dir(self.versions_dir()) {
            for ent in entries.flatten() {
                if !ent.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    continue;
                }
                let name = ent.file_name().to_string_lossy().to_string();
                if name.contains("_staging_") {
                    continue;
                }
                versions_scanned.push(name.clone());
                let vpath = ent.path();
                match Self::verify_candidate_release_dir_with_anchor(
                    &vpath,
                    &RuntimeCompatibilityProfile::default(),
                    None,
                ) {
                    Ok(vrel) => {
                        if trust_state
                            .revoked_pubkeys
                            .iter()
                            .any(|r| r.eq_ignore_ascii_case(&vrel.pe_pubkey_hex))
                        {
                            rogue_versions.push(name.clone());
                            issues.push(format!("KEY_REVOKED in versions/{}", name));
                        } else if !trust_state
                            .trusted_pubkeys
                            .iter()
                            .any(|k| k.eq_ignore_ascii_case(&vrel.pe_pubkey_hex))
                        {
                            rogue_versions.push(name.clone());
                            issues.push(format!("ROGUE_VERSION_DIR_DETECTED: untrusted key in versions/{}", name));
                        } else if let Err(e) = Self::verify_optional_release_manifest(&vpath, &vrel.exe_path) {
                            rogue_versions.push(name.clone());
                            issues.push(format!("RELEASE_MANIFEST_TAMPERED in versions/{}: {:#}", name, e));
                        }
                    }
                    Err(e) => {
                        rogue_versions.push(name.clone());
                        issues.push(format!("ROGUE_OR_CORRUPT_VERSION_DIR versions/{}: {:#}", name, e));
                    }
                }
            }
        }
        if !rogue_versions.is_empty() {
            bail!("{}", issues.join(" | "));
        }
        let exe_sha = fp_bundle_verify::sha256_hex(&fs::read(&verified_active.exe_path)?);
        let ext_ok = self.read_external_trust_hwm().is_some();

        Ok(InstallSecurityAuditReport {
            passed: true,
            install_root: self.install_root.to_string_lossy().to_string(),
            trust_epoch: trust_state.trust_epoch,
            active_version: active.active_version,
            active_exe_sha256: exe_sha,
            pe_root_pubkey_hex: verified_active.pe_pubkey_hex,
            authenticode_trust_status: auth.trust_status,
            external_anchor_verified: ext_ok,
            versions_scanned,
            rogue_or_tampered_versions: rogue_versions,
            issues,
        })
    }

    fn verify_optional_release_manifest(version_dir: &Path, exe_path: &Path) -> Result<()> {
        let rm_path = version_dir.join("release-manifest.json");
        if !rm_path.exists() {
            return Ok(());
        }
        let rm_bytes = fs::read(&rm_path)?;
        let rm: fp_bundle_contract::ReleasePackageManifest = serde_json::from_slice(&rm_bytes)
            .with_context(|| format!("RELEASE_MANIFEST_TAMPERED: invalid JSON in {}", rm_path.display()))?;
        let actual_exe_sha = fp_bundle_verify::sha256_hex(&fs::read(exe_path)?);
        if !actual_exe_sha.eq_ignore_ascii_case(&rm.final_exe_sha256) {
            bail!(
                "RELEASE_MANIFEST_TAMPERED: exe SHA-256 '{}' != release-manifest '{}'",
                actual_exe_sha,
                rm.final_exe_sha256
            );
        }
        for entry in &rm.distributed_files {
            let target = validate_safe_relative_path(version_dir, &entry.rel_path).with_context(|| {
                format!("RELEASE_MANIFEST_TAMPERED: unsafe rel_path '{}'", entry.rel_path)
            })?;
            if !target.exists() {
                bail!("RELEASE_MANIFEST_TAMPERED: missing file '{}'", entry.rel_path);
            }
            let actual = fp_bundle_verify::sha256_hex(&fs::read(&target)?);
            if !actual.eq_ignore_ascii_case(&entry.sha256) {
                bail!(
                    "RELEASE_MANIFEST_TAMPERED: checksum mismatch for '{}'",
                    entry.rel_path
                );
            }
        }
        Ok(())
    }

    pub fn write_trust_state(&self, state: &PersistentTrustState) -> Result<()> {
        let mut normalized = state.clone();
        if normalized.trust_epoch == 0 {
            normalized.trust_epoch = 1;
        }
        let prev_sha = if self.trust_state_path().exists() {
            fs::read(self.trust_state_path())
                .ok()
                .map(|b| fp_bundle_verify::sha256_hex(&b))
        } else {
            None
        };
        if normalized.previous_state_sha256.is_none() {
            normalized.previous_state_sha256 = prev_sha;
        }
        self.sync_trust_ledger_db(&normalized, "sync_trust_state", "atomic_trust_update")?;
        let dir = self.updater_dir();
        fs::create_dir_all(&dir)?;
        let bytes = serde_json::to_vec_pretty(&normalized)?;
        atomic_write_file(&self.trust_state_path(), &bytes)?;
        let _ = atomic_write_file(&self.trust_state_backup_path(), &bytes);
        Ok(())
    }

    /// Explicitly revokes an Ed25519 publisher key, removes it from `trusted_pubkeys`,
    /// bumps monotonic `trust_epoch`, and commits atomically to `trust-ledger.db` & `trust-state.json`.
    pub fn revoke_pubkey(&self, pubkey_hex: &str, reason: &str) -> Result<PersistentTrustState> {
        let mut st = self
            .read_trust_state()?
            .unwrap_or_else(|| PersistentTrustState {
                trust_epoch: 0,
                ..Default::default()
            });
        let norm = pubkey_hex.trim().to_ascii_lowercase();
        st.trusted_pubkeys
            .retain(|k| !k.eq_ignore_ascii_case(&norm));
        if !st
            .revoked_pubkeys
            .iter()
            .any(|r| r.eq_ignore_ascii_case(&norm))
        {
            st.revoked_pubkeys.push(norm.clone());
        }
        st.trust_epoch = st.trust_epoch.saturating_add(1).max(1);
        st.previous_state_sha256 = None;
        st.updated_at_utc = Utc::now().to_rfc3339();
        self.sync_trust_ledger_db(&st, "revoke_pubkey", &format!("{}:{}", norm, reason))?;
        let bytes = serde_json::to_vec_pretty(&st)?;
        atomic_write_file(&self.trust_state_path(), &bytes)?;
        let _ = atomic_write_file(&self.trust_state_backup_path(), &bytes);
        Ok(st)
    }

    /// Startup / pre-update integrity check of the currently installed active executable:
    /// checks embedded `RT_RCDATA #7777` key matches `active_version.json` and rejects any broken Authenticode signature.
    pub fn verify_installed_exe_integrity(&self) -> Result<AuthenticodeVerificationReport> {
        let active = self.read_active_pointer()?.ok_or_else(|| {
            anyhow::anyhow!(
                "ACTIVE_VERSION_MISSING: no active_version.json in {}",
                self.install_root.display()
            )
        })?;
        let active_exe = self
            .versions_dir()
            .join(format!("v{}", active.active_version))
            .join(&active.exe_name);
        if !active_exe.exists() {
            bail!(
                "INSTALLED_EXE_MISSING: expected installed binary at {}",
                active_exe.display()
            );
        }
        let pe_pk = read_pe_embedded_pubkey_from_exe(&active_exe).with_context(|| {
            format!(
                "INSTALLED_ROOT_OF_TRUST_MISSING: failed reading RT_RCDATA #{} from {}",
                PE_ED25519_PUBKEY_RESOURCE_ID,
                active_exe.display()
            )
        })?;
        if let Some(ref ptr_pk) = active.trusted_pubkey_hex {
            if !ptr_pk.trim().eq_ignore_ascii_case(&pe_pk) {
                bail!(
                    "INSTALLED_ROOT_OF_TRUST_MISMATCH: active_version.json pubkey != installed EXE RT_RCDATA #{}",
                    PE_ED25519_PUBKEY_RESOURCE_ID
                );
            }
        }
        let report = verify_authenticode_pe(&active_exe);
        match report.trust_status {
            AuthenticodeTrustStatus::TamperedOrInvalid => {
                bail!(
                    "INSTALLED_EXECUTABLE_TAMPERED: active EXE failed WinVerifyTrust (0x{:08X}: {})",
                    report.win_trust_error_code,
                    report.detail
                );
            }
            AuthenticodeTrustStatus::ExpiredOrTimestampError => {
                bail!(
                    "INSTALLED_EXECUTABLE_CERT_EXPIRED: active EXE certificate/timestamp invalid (0x{:08X}: {})",
                    report.win_trust_error_code,
                    report.detail
                );
            }
            _ => {}
        }
        Ok(report)
    }

    pub fn active_pointer_path(&self) -> PathBuf {
        self.install_root.join("active_version.json")
    }

    pub fn backup_pointer_path(&self) -> PathBuf {
        self.updater_dir().join("active_version.bak.json")
    }

    pub fn lock_file_path(&self) -> PathBuf {
        self.updater_dir().join("updater.lock")
    }

    fn write_active_pointer_with_shadow(&self, ptr: &ActiveVersionPointer) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(ptr)?;
        atomic_write_file(&self.active_pointer_path(), &bytes)?;
        let _ = atomic_write_file(&self.backup_pointer_path(), &bytes);
        Ok(())
    }

    pub fn read_active_pointer(&self) -> Result<Option<ActiveVersionPointer>> {
        let p = self.active_pointer_path();
        if !p.exists() {
            return Ok(None);
        }
        let bytes = fs::read(&p)
            .with_context(|| format!("Failed to read active pointer at {}", p.display()))?;
        let ptr: ActiveVersionPointer = serde_json::from_slice(&bytes)?;
        Ok(Some(ptr))
    }

    pub fn read_journal(&self) -> Result<Option<UpdateJournal>> {
        let p = self.journal_path();
        if !p.exists() {
            return Ok(None);
        }
        let bytes = fs::read(&p)?;
        let j: UpdateJournal = serde_json::from_slice(&bytes)?;
        Ok(Some(j))
    }

    pub fn write_journal(&self, journal: &mut UpdateJournal) -> Result<()> {
        journal.updated_at_utc = Utc::now().to_rfc3339();
        let dir = self.updater_dir();
        fs::create_dir_all(&dir).with_context(|| {
            format!("INSTALL_DIR_NOT_WRITABLE: cannot create {}", dir.display())
        })?;
        let payload = serde_json::to_vec_pretty(journal)?;
        atomic_write_file(&self.journal_path(), &payload)
            .context("INSTALL_DIR_NOT_WRITABLE: cannot write update journal")?;
        Ok(())
    }

    pub fn verify_candidate_release_dir(
        candidate_dir: &Path,
        runtime_profile: &RuntimeCompatibilityProfile,
    ) -> Result<VerifiedCandidateRelease> {
        Self::verify_candidate_release_dir_with_anchor(candidate_dir, runtime_profile, None)
    }

    pub fn verify_candidate_release_dir_with_anchor(
        candidate_dir: &Path,
        runtime_profile: &RuntimeCompatibilityProfile,
        trusted_anchor_pubkey_hex: Option<&str>,
    ) -> Result<VerifiedCandidateRelease> {
        ensure_no_reparse_or_junction_tree(candidate_dir)?;

        // Forbid any release package from shipping a top-level `data/` or `.updater/` tree
        for forbidden in ["data", ".updater"] {
            if candidate_dir.join(forbidden).exists() {
                bail!(
                    "RELEASE_FORBIDS_USER_DATA_OVERWRITE: release archive contains forbidden '{}' directory at {}",
                    forbidden,
                    candidate_dir.display()
                );
            }
        }

        let bundle_dir = candidate_dir.join("bundle");
        if !bundle_dir.exists() {
            bail!(
                "CANDIDATE_BUNDLE_MISSING: missing bundle/ directory at {}",
                bundle_dir.display()
            );
        }
        let manifest: ReleaseBundleManifest = serde_json::from_slice(
            &fs::read(bundle_dir.join("manifest.fpbundle.json"))
                .context("Missing manifest.fpbundle.json in candidate bundle")?,
        )?;

        validate_single_filename(&manifest.project.exe_name)
            .context("PATH_SECURITY_ERROR: unsafe exe_name in release manifest")?;
        for wf in &manifest.project.workflow_files {
            let _ = validate_safe_relative_path(&bundle_dir, wf)?;
        }
        for tpl in &manifest.project.template_files {
            let _ = validate_safe_relative_path(&bundle_dir, tpl)?;
        }
        let _ = validate_safe_relative_path(&bundle_dir, &manifest.project.icon_rel_path)?;

        let exe_path = candidate_dir.join(&manifest.project.exe_name);
        if !exe_path.exists() {
            bail!(
                "CANDIDATE_EXE_MISSING: expected '{}' inside {}",
                manifest.project.exe_name,
                candidate_dir.display()
            );
        }

        let pe_pubkey_hex = read_pe_embedded_pubkey_from_exe(&exe_path).with_context(|| {
            format!(
                "PE_ROOT_OF_TRUST_MISSING: failed reading RT_RCDATA #{} from {}",
                PE_ED25519_PUBKEY_RESOURCE_ID,
                exe_path.display()
            )
        })?;

        // Check PE Authenticode signature & Windows trust chain
        let authenticode = verify_authenticode_pe(&exe_path);
        match authenticode.trust_status {
            AuthenticodeTrustStatus::TamperedOrInvalid => {
                bail!(
                    "AUTHENTICODE_TAMPERED_OR_INVALID: PE Authenticode verification failed (0x{:08X}: {})",
                    authenticode.win_trust_error_code,
                    authenticode.detail
                );
            }
            AuthenticodeTrustStatus::ExpiredOrTimestampError => {
                bail!(
                    "AUTHENTICODE_TIMESTAMP_OR_EXPIRED: PE certificate or timestamp invalid (0x{:08X}: {})",
                    authenticode.win_trust_error_code,
                    authenticode.detail
                );
            }
            _ => {}
        }

        if let Some(ref signing_meta) = manifest.signing {
            match signing_meta.signing_mode {
                SigningMode::PublicSigned => {
                    if !authenticode.trusted_public_release {
                        bail!(
                            "AUTHENTICODE_PUBLIC_TRUST_REQUIRED: release requires TrustedPublicChain, got {:?} ({})",
                            authenticode.trust_status,
                            authenticode.detail
                        );
                    }
                }
                SigningMode::SelfSignedQa => {
                    if !authenticode.signed || authenticode.trust_status == AuthenticodeTrustStatus::Unsigned {
                        bail!(
                            "AUTHENTICODE_SIGNATURE_MISSING: release requires SelfSignedQa signature but binary is unsigned"
                        );
                    }
                }
                SigningMode::UnsignedInternal => {}
            }
            if signing_meta.release_channel.eq_ignore_ascii_case("production")
                && !authenticode.trusted_public_release
            {
                bail!(
                    "AUTHENTICODE_PRODUCTION_TRUST_REQUIRED: production channel release must hold TrustedPublicChain Authenticode signature"
                );
            }
        }

        // CRITICAL P5.2 + P5.3 ROOT OF TRUST & KEY ROTATION DELEGATION ENFORCEMENT:
        // When updating from an already-installed Mini Tool, the trusted Ed25519 publisher key
        // MUST come from the previously installed trusted binary (or explicit trust anchor).
        // Rotating from old_pubkey_hex -> new_pubkey_hex requires a valid signed KeyRotationDelegation!
        let mut rotated_from_pubkey_hex: Option<String> = None;
        let expected_publisher_id = manifest.publisher.as_ref().map(|p| p.publisher_id.as_str());
        let effective_trust_pubkey = match trusted_anchor_pubkey_hex {
            Some(anchor) => {
                let anchor_norm = anchor.trim().to_ascii_lowercase();
                if pe_pubkey_hex.eq_ignore_ascii_case(&anchor_norm) {
                    anchor_norm
                } else if let Some(delegation) =
                    manifest.signing.as_ref().and_then(|s| s.key_rotation.as_ref())
                {
                    if !delegation.new_pubkey_hex.eq_ignore_ascii_case(&pe_pubkey_hex) {
                        bail!(
                            "KEY_ROTATION_TARGET_MISMATCH: delegation newPubkeyHex '{}' != candidate PE key '{}'",
                            delegation.new_pubkey_hex,
                            pe_pubkey_hex
                        );
                    }
                    verify_key_rotation_delegation(
                        delegation,
                        &anchor_norm,
                        expected_publisher_id,
                        Some(&manifest.project.slug),
                        None,
                    )
                    .context("KEY_ROTATION_DELEGATION_INVALID")?;
                    rotated_from_pubkey_hex = Some(anchor_norm);
                    pe_pubkey_hex.to_ascii_lowercase()
                } else {
                    bail!(
                        "UPDATE_ROOT_OF_TRUST_UNTRUSTED: candidate PE RT_RCDATA #{} pubkey '{}' does not match installed trusted publisher pubkey '{}' and no valid KeyRotationDelegation was provided",
                        PE_ED25519_PUBKEY_RESOURCE_ID,
                        pe_pubkey_hex,
                        anchor.trim()
                    );
                }
            }
            None => pe_pubkey_hex.to_ascii_lowercase(),
        };

        let disk_pub = fs::read_to_string(bundle_dir.join("trusted_publisher.pub"))
            .context("Missing trusted_publisher.pub")?;
        if !disk_pub.trim().eq_ignore_ascii_case(&effective_trust_pubkey) {
            bail!(
                "PE_ROOT_OF_TRUST_MISMATCH: disk trusted_publisher.pub != trusted PE RT_RCDATA #{}",
                PE_ED25519_PUBKEY_RESOURCE_ID
            );
        }

        let sig_bytes = fs::read(bundle_dir.join("signature.fpbundle.json"))
            .context("Missing signature.fpbundle.json")?;
        let envelope: SignedBundleEnvelope = serde_json::from_slice(&sig_bytes)?;
        envelope.verify_ed25519_with_public_key(&bundle_dir, &effective_trust_pubkey)?;

        let ui_schema: MiniToolUiSchema = serde_json::from_slice(
            &fs::read(bundle_dir.join("ui-schema.json"))
                .context("Missing ui-schema.json in candidate bundle")?,
        )?;
        let workflows: Vec<WorkflowDocument> = serde_json::from_slice(
            &fs::read(bundle_dir.join("workflows.fpbundle.json"))
                .context("Missing workflows.fpbundle.json in candidate bundle")?,
        )?;

        let wf_versions: Vec<u32> = workflows.iter().map(|w| w.schema_version).collect();
        verify_bundle_compatibility_gate(&manifest, &ui_schema, &wf_versions, runtime_profile)?;

        let publisher_id = manifest
            .publisher
            .as_ref()
            .map(|p| p.publisher_id.clone())
            .or_else(|| {
                manifest
                    .signing
                    .as_ref()
                    .map(|s| s.publisher_id.clone())
                    .filter(|s| !s.is_empty())
            });

        Ok(VerifiedCandidateRelease {
            manifest,
            ui_schema,
            workflows,
            exe_path,
            pe_pubkey_hex: effective_trust_pubkey,
            authenticode,
            rotated_from_pubkey_hex,
            publisher_id,
        })
    }

    /// Probes real NTFS/Windows writeability and ensures no NTFS Junction/Reparse symlink escapes.
    pub fn probe_install_permissions(&self) -> Result<()> {
        if self.install_root.exists() {
            ensure_not_reparse_point(&self.install_root)?;
        }
        for dir in [
            self.install_root.clone(),
            self.versions_dir(),
            self.data_dir(),
            self.updater_dir(),
        ] {
            fs::create_dir_all(&dir).with_context(|| {
                format!(
                    "INSTALL_DIR_NOT_WRITABLE: cannot create required directory {}",
                    dir.display()
                )
            })?;
            ensure_not_reparse_point(&dir)?;
            let probe = dir.join(format!(".fp_perm_probe_{}", Uuid::new_v4()));
            let mut f = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&probe)
                .with_context(|| {
                    format!(
                        "INSTALL_DIR_NOT_WRITABLE: filesystem denied write access in {}",
                        dir.display()
                    )
                })?;
            f.write_all(b"ok")?;
            drop(f);
            let _ = fs::remove_file(&probe);
        }
        Ok(())
    }

    pub fn install_initial(
        &self,
        candidate_release_dir: &Path,
        options: &UpdateExecutionOptions,
    ) -> Result<UpdateOutcome> {
        if self.active_pointer_path().exists() {
            bail!("INSTALL_ALREADY_INITIALIZED: active_version.json already exists; use upgrade instead");
        }
        self.probe_install_permissions()?;
        let _updater_lock = UpdaterLockGuard::acquire(&self.lock_file_path())?;
        let verified = Self::verify_candidate_release_dir_with_anchor(
            candidate_release_dir,
            &RuntimeCompatibilityProfile::default(),
            None,
        )?;
        if let Some(prev_ts) = self.read_trust_state()? {
            if prev_ts
                .revoked_pubkeys
                .iter()
                .any(|r| r.eq_ignore_ascii_case(&verified.pe_pubkey_hex))
            {
                bail!(
                    "KEY_REVOKED: candidate public key '{}' was previously revoked for install root {}",
                    verified.pe_pubkey_hex,
                    self.install_root.display()
                );
            }
        }
        let slug = verified.manifest.project.slug.clone();
        let ver = verified.manifest.project.version.clone();
        let _ = SemVer::parse(&ver)?;
        let target_version_dir = self.versions_dir().join(format!("v{}", ver));
        let staging_dir = self
            .versions_dir()
            .join(format!("v{}_staging_{}", ver, Uuid::new_v4().simple()));
        let live_db_path = self.data_dir().join(format!("runtime_{}.db", slug));

        let mut journal = UpdateJournal {
            journal_id: format!("upd-{}", Uuid::new_v4()),
            operation: "install_initial".to_string(),
            project_slug: slug.clone(),
            source_version: None,
            target_version: ver.clone(),
            source_release_dir: candidate_release_dir.to_string_lossy().to_string(),
            stage: UpdateStage::PreflightVerified,
            backup_db_path: None,
            live_db_path: live_db_path.to_string_lossy().to_string(),
            staged_staging_dir: Some(staging_dir.to_string_lossy().to_string()),
            staged_version_dir: target_version_dir.to_string_lossy().to_string(),
            previous_active_pointer: None,
            target_compat: verified.manifest.compatibility.clone(),
            auto_resume_blocked_jobs: vec![],
            last_error: None,
            updated_at_utc: Utc::now().to_rfc3339(),
        };
        self.write_journal(&mut journal)?;
        copy_dir_recursive(candidate_release_dir, &staging_dir)?;
        let staged_verified = Self::verify_candidate_release_dir_with_anchor(
            &staging_dir,
            &RuntimeCompatibilityProfile::default(),
            Some(&verified.pe_pubkey_hex),
        )?;
        if !options.skip_child_verify_exec {
            verify_staged_exe_bundle(&staged_verified.exe_path, &self.data_dir())?;
        }
        if target_version_dir.exists() {
            fs::remove_dir_all(&target_version_dir)?;
        }
        fs::rename(&staging_dir, &target_version_dir)?;
        journal.stage = UpdateStage::StagedVerified;
        journal.staged_staging_dir = None;
        self.write_journal(&mut journal)?;

        // Ensure persistent DB initialized cleanly inside data/
        let _storage = StorageManager::new(&live_db_path)?;

        let pointer = ActiveVersionPointer {
            project_slug: slug.clone(),
            active_version: ver.clone(),
            previous_version: None,
            exe_name: staged_verified.manifest.project.exe_name.clone(),
            sqlite_schema_version: staged_verified.manifest.compatibility.sqlite_schema_version,
            min_runtime_version: staged_verified
                .manifest
                .compatibility
                .min_runtime_version
                .clone(),
            trusted_pubkey_hex: Some(staged_verified.pe_pubkey_hex.clone()),
            post_update_auto_resume_blocked: false,
            blocked_job_ids: vec![],
            backup_db_path: None,
            activated_at_utc: Utc::now().to_rfc3339(),
            journal_id: journal.journal_id.clone(),
        };
        self.write_active_pointer_with_shadow(&pointer)?;
        let mut init_trust = self.read_trust_state()?.unwrap_or_default();
        init_trust.publisher_id = staged_verified.publisher_id.clone();
        if !init_trust
            .trusted_pubkeys
            .iter()
            .any(|k| k.eq_ignore_ascii_case(&staged_verified.pe_pubkey_hex))
        {
            init_trust
                .trusted_pubkeys
                .push(staged_verified.pe_pubkey_hex.clone());
        }
        init_trust.updated_at_utc = Utc::now().to_rfc3339();
        self.write_trust_state(&init_trust)?;
        journal.stage = UpdateStage::Completed;
        self.write_journal(&mut journal)?;

        Ok(UpdateOutcome {
            operation: "install_initial".to_string(),
            status: "COMPLETED".to_string(),
            project_slug: slug,
            previous_version: None,
            active_version: ver,
            stage_reached: UpdateStage::Completed,
            backup_db_path: None,
            backup_verified_ok: false,
            signature_verified_ok: true,
            compatibility_verified_ok: true,
            post_update_auto_resume_blocked: false,
            blocked_job_ids: vec![],
            binary_rollback_only: false,
            database_restored: false,
            pre_rollback_safety_db_path: None,
            journal_path: self.journal_path().to_string_lossy().to_string(),
            detail: "Initial Mini Tool versioned installation verified and activated.".to_string(),
        })
    }

    pub fn upgrade_from_release(
        &self,
        candidate_release_dir: &Path,
        options: &UpdateExecutionOptions,
    ) -> Result<UpdateOutcome> {
        let active = self.read_active_pointer()?.ok_or_else(|| {
            anyhow::anyhow!("ACTIVE_VERSION_MISSING: no existing active_version.json in {}", self.install_root.display())
        })?;

        self.probe_install_permissions()?;
        let _updater_lock = UpdaterLockGuard::acquire(&self.lock_file_path())?;

        // Fail closed if an interrupted update journal was not reconciled first
        if let Some(existing_j) = self.read_journal()? {
            if !matches!(
                existing_j.stage,
                UpdateStage::Idle | UpdateStage::Completed | UpdateStage::RolledBack
            ) {
                bail!(
                    "UNRECONCILED_UPDATE_JOURNAL: journal {} is stuck in {:?}; run reconcile first",
                    existing_j.journal_id,
                    existing_j.stage
                );
            }
        }

        // Derive authoritative Trust Anchor strictly from the PREVIOUSLY INSTALLED trusted EXE!
        let active_exe = self
            .versions_dir()
            .join(format!("v{}", active.active_version))
            .join(&active.exe_name);
        let installed_trusted_pubkey = read_pe_embedded_pubkey_from_exe(&active_exe).with_context(|| {
            format!(
                "INSTALLED_ROOT_OF_TRUST_MISSING: failed reading trusted RT_RCDATA #{} from installed EXE {}",
                PE_ED25519_PUBKEY_RESOURCE_ID,
                active_exe.display()
            )
        })?;
        if let Some(ref ptr_pk) = active.trusted_pubkey_hex {
            if !ptr_pk.trim().eq_ignore_ascii_case(&installed_trusted_pubkey) {
                bail!(
                    "INSTALLED_ROOT_OF_TRUST_MISMATCH: active_version.json trustedPubkeyHex '{}' != installed EXE RT_RCDATA #{} '{}'",
                    ptr_pk,
                    PE_ED25519_PUBKEY_RESOURCE_ID,
                    installed_trusted_pubkey
                );
            }
        }

        let _ = self.verify_installed_exe_integrity()?;

        let trust_state = self.read_trust_state()?.unwrap_or_else(|| PersistentTrustState {
            trust_epoch: 1,
            publisher_id: None,
            trusted_pubkeys: vec![installed_trusted_pubkey.to_ascii_lowercase()],
            revoked_pubkeys: vec![],
            used_rotation_nonces: vec![],
            previous_state_sha256: None,
            updated_at_utc: Utc::now().to_rfc3339(),
        });

        let verified = match Self::verify_candidate_release_dir_with_anchor(
            candidate_release_dir,
            &RuntimeCompatibilityProfile::default(),
            Some(&installed_trusted_pubkey),
        ) {
            Ok(v) => v,
            Err(anchor_err) => {
                // Allow upgrading to a release signed by a key ALREADY trusted in trust-state
                // (e.g., after rolling back from v1.1.0 [NewKey] -> v1.0.0 [OldKey], then upgrading to v1.2.0 [NewKey]).
                let Ok(unanchored) = Self::verify_candidate_release_dir_with_anchor(
                    candidate_release_dir,
                    &RuntimeCompatibilityProfile::default(),
                    None,
                ) else {
                    return Err(anchor_err);
                };
                let in_trusted = trust_state
                    .trusted_pubkeys
                    .iter()
                    .any(|k| k.eq_ignore_ascii_case(&unanchored.pe_pubkey_hex))
                    && !trust_state
                        .revoked_pubkeys
                        .iter()
                        .any(|r| r.eq_ignore_ascii_case(&unanchored.pe_pubkey_hex));
                if !in_trusted {
                    return Err(anchor_err);
                }
                unanchored
            }
        };

        if trust_state
            .revoked_pubkeys
            .iter()
            .any(|k| k.eq_ignore_ascii_case(&verified.pe_pubkey_hex))
        {
            bail!(
                "KEY_REVOKED: candidate public key '{}' has been revoked",
                verified.pe_pubkey_hex
            );
        }
        if let (Some(ref expected_pub), Some(ref cand_pub)) =
            (&trust_state.publisher_id, &verified.publisher_id)
        {
            if expected_pub != cand_pub {
                bail!(
                    "PUBLISHER_IDENTITY_MISMATCH: installed publisher '{}' does not match candidate '{}'",
                    expected_pub,
                    cand_pub
                );
            }
        }
        let is_revoked = trust_state
            .revoked_pubkeys
            .iter()
            .any(|k| k.eq_ignore_ascii_case(&verified.pe_pubkey_hex));
        let key_already_trusted = !is_revoked
            && trust_state
                .trusted_pubkeys
                .iter()
                .any(|k| k.eq_ignore_ascii_case(&verified.pe_pubkey_hex));
        if verified.rotated_from_pubkey_hex.is_some() && !key_already_trusted {
            if let Some(ref deleg) = verified
                .manifest
                .signing
                .as_ref()
                .and_then(|s| s.key_rotation.as_ref())
            {
                if trust_state
                    .used_rotation_nonces
                    .iter()
                    .any(|n| n == &deleg.nonce)
                {
                    bail!(
                        "KEY_ROTATION_NONCE_REPLAYED: rotation nonce '{}' was already used",
                        deleg.nonce
                    );
                }
            }
        }

        if verified.manifest.project.slug != active.project_slug {
            bail!(
                "PROJECT_SLUG_MISMATCH: active install is '{}' but candidate bundle is '{}'",
                active.project_slug,
                verified.manifest.project.slug
            );
        }

        let current_sem = SemVer::parse(&active.active_version)?;
        let target_sem = SemVer::parse(&verified.manifest.project.version)?;
        if target_sem <= current_sem {
            bail!(
                "SEMVER_UPGRADE_REJECTED: candidate version {} is not newer than active version {} (use rollback for downgrade)",
                verified.manifest.project.version,
                active.active_version
            );
        }

        // Step 2: Detect running Mini Tool processes or active jobs BEFORE backing up / staging
        let live_db_path = self
            .data_dir()
            .join(format!("runtime_{}.db", active.project_slug));
        ensure_tool_quiesced(&self.data_dir(), &active.project_slug, &active_exe, &live_db_path)?;

        let target_ver = verified.manifest.project.version.clone();
        let target_version_dir = self.versions_dir().join(format!("v{}", target_ver));
        let staging_dir = self.versions_dir().join(format!(
            "v{}_staging_{}",
            target_ver,
            Uuid::new_v4().simple()
        ));

        let mut journal = UpdateJournal {
            journal_id: format!("upd-{}", Uuid::new_v4()),
            operation: "upgrade".to_string(),
            project_slug: active.project_slug.clone(),
            source_version: Some(active.active_version.clone()),
            target_version: target_ver.clone(),
            source_release_dir: candidate_release_dir.to_string_lossy().to_string(),
            stage: UpdateStage::PreflightVerified,
            backup_db_path: None,
            live_db_path: live_db_path.to_string_lossy().to_string(),
            staged_staging_dir: Some(staging_dir.to_string_lossy().to_string()),
            staged_version_dir: target_version_dir.to_string_lossy().to_string(),
            previous_active_pointer: Some(active.clone()),
            target_compat: verified.manifest.compatibility.clone(),
            auto_resume_blocked_jobs: vec![],
            last_error: None,
            updated_at_utc: Utc::now().to_rfc3339(),
        };
        self.write_journal(&mut journal)?;
        maybe_trigger_fault(FaultInjectionPoint::AfterPreflight, options);

        // Step 3: Quiesced SQLite WAL Backup + check unverified side-effect checkpoints
        let backup_path = self.backups_dir().join(format!(
            "pre_update_v{}_to_v{}_{}.db",
            active.active_version,
            target_ver,
            Utc::now().format("%Y%m%d_%H%M%S_%3f")
        ));
        let mut unverified_checkpoint_jobs = Vec::new();
        if live_db_path.exists() {
            let st = StorageManager::new(&live_db_path)?;
            let _rep = st.online_backup_to(&backup_path)?;
            unverified_checkpoint_jobs = find_unverified_checkpoint_jobs(&live_db_path)?;
            journal.backup_db_path = Some(backup_path.to_string_lossy().to_string());
        } else {
            // Create empty clean DB so rollback baseline exists
            let st = StorageManager::new(&live_db_path)?;
            let _rep = st.online_backup_to(&backup_path)?;
            journal.backup_db_path = Some(backup_path.to_string_lossy().to_string());
        }
        journal.auto_resume_blocked_jobs = unverified_checkpoint_jobs.clone();
        journal.stage = UpdateStage::QuiescedAndBackedUp;
        self.write_journal(&mut journal)?;
        maybe_trigger_fault(FaultInjectionPoint::AfterBackup, options);

        // Step 4: Stage target version directory (does NOT overwrite active v<old> or data/)
        copy_dir_recursive(candidate_release_dir, &staging_dir)?;
        journal.stage = UpdateStage::Staged;
        self.write_journal(&mut journal)?;
        maybe_trigger_fault(FaultInjectionPoint::AfterStage, options);

        // Step 5: Verify staged bundle strictly against rotation anchor (if rotating) or authorized trusted key
        let stage_anchor = if verified.rotated_from_pubkey_hex.is_some() {
            installed_trusted_pubkey.as_str()
        } else {
            verified.pe_pubkey_hex.as_str()
        };
        let staged_verified = Self::verify_candidate_release_dir_with_anchor(
            &staging_dir,
            &RuntimeCompatibilityProfile::default(),
            Some(stage_anchor),
        )?;
        if !options.skip_child_verify_exec {
            verify_staged_exe_bundle(&staged_verified.exe_path, &self.data_dir())?;
        }

        // Simulate or execute SQLite migration validation on live DB
        let migration_res: Result<()> = if options.simulate_migration_failure {
            Err(anyhow::anyhow!(
                "SIMULATED_MIGRATION_FAILURE: SQLite v{} migration aborted intentionally",
                staged_verified.manifest.compatibility.sqlite_schema_version
            ))
        } else {
            let st = StorageManager::new(&live_db_path)?;
            let cur_v = st.highest_applied_migration_version()?;
            if cur_v > CURRENT_SQLITE_SCHEMA_VERSION {
                Err(anyhow::anyhow!(
                    "SQLITE_DOWNGRADE_INCOMPATIBLE: live DB v{} exceeds runtime max v{}",
                    cur_v,
                    CURRENT_SQLITE_SCHEMA_VERSION
                ))
            } else {
                Ok(())
            }
        };

        if let Err(err) = migration_res {
            // Rollback WAL backup + clean up staging dir without flipping active_version.json
            if let Some(bk) = journal.backup_db_path.as_ref() {
                let safety_snap = self.backups_dir().join(format!(
                    "pre_rollback_safety_{}.db",
                    Utc::now().format("%Y%m%d_%H%M%S_%3f")
                ));
                let _ = StorageManager::rollback_database_from_backup(
                    live_db_path.as_path(),
                    Path::new(bk),
                    safety_snap.as_path(),
                );
            }
            let _ = fs::remove_dir_all(&staging_dir);
            journal.stage = UpdateStage::RolledBack;
            journal.staged_staging_dir = None;
            journal.last_error = Some(err.to_string());
            self.write_journal(&mut journal)?;
            return Err(err);
        }

        if target_version_dir.exists() {
            fs::remove_dir_all(&target_version_dir)?;
        }
        fs::rename(&staging_dir, &target_version_dir)?;
        journal.stage = UpdateStage::StagedVerified;
        journal.staged_staging_dir = None;
        self.write_journal(&mut journal)?;

        // Step 6: Atomic activation via temp file + rename
        let auto_blocked = !unverified_checkpoint_jobs.is_empty();
        let next_pointer = ActiveVersionPointer {
            project_slug: active.project_slug.clone(),
            active_version: target_ver.clone(),
            previous_version: Some(active.active_version.clone()),
            exe_name: staged_verified.manifest.project.exe_name.clone(),
            sqlite_schema_version: staged_verified.manifest.compatibility.sqlite_schema_version,
            min_runtime_version: staged_verified
                .manifest
                .compatibility
                .min_runtime_version
                .clone(),
            trusted_pubkey_hex: Some(staged_verified.pe_pubkey_hex.clone()),
            post_update_auto_resume_blocked: auto_blocked,
            blocked_job_ids: unverified_checkpoint_jobs.clone(),
            backup_db_path: journal.backup_db_path.clone(),
            activated_at_utc: Utc::now().to_rfc3339(),
            journal_id: journal.journal_id.clone(),
        };
        let mut updated_trust = self.read_trust_state()?.unwrap_or_else(|| PersistentTrustState {
            trust_epoch: 1,
            publisher_id: staged_verified.publisher_id.clone(),
            trusted_pubkeys: vec![installed_trusted_pubkey.to_ascii_lowercase()],
            revoked_pubkeys: vec![],
            used_rotation_nonces: vec![],
            previous_state_sha256: None,
            updated_at_utc: Utc::now().to_rfc3339(),
        });
        if updated_trust.publisher_id.is_none() {
            updated_trust.publisher_id = staged_verified.publisher_id.clone();
        }
        let mut trust_changed = false;
        if !updated_trust
            .trusted_pubkeys
            .iter()
            .any(|k| k.eq_ignore_ascii_case(&staged_verified.pe_pubkey_hex))
        {
            updated_trust
                .trusted_pubkeys
                .push(staged_verified.pe_pubkey_hex.clone());
            trust_changed = true;
        }
        if staged_verified.rotated_from_pubkey_hex.is_some() {
            if let Some(ref deleg) = staged_verified
                .manifest
                .signing
                .as_ref()
                .and_then(|s| s.key_rotation.as_ref())
            {
                if !updated_trust.used_rotation_nonces.contains(&deleg.nonce) {
                    updated_trust.used_rotation_nonces.push(deleg.nonce.clone());
                    trust_changed = true;
                }
            }
        }
        if trust_changed {
            updated_trust.trust_epoch = updated_trust.trust_epoch.saturating_add(1).max(1);
            updated_trust.previous_state_sha256 = None;
            updated_trust.updated_at_utc = Utc::now().to_rfc3339();
        }
        self.write_active_pointer_with_shadow(&next_pointer)?;
        journal.stage = UpdateStage::Activated;
        self.write_journal(&mut journal)?;
        self.write_trust_state(&updated_trust)?;
        maybe_trigger_fault(FaultInjectionPoint::AfterActivate, options);

        journal.stage = UpdateStage::Completed;
        self.write_journal(&mut journal)?;

        Ok(UpdateOutcome {
            operation: "upgrade".to_string(),
            status: "COMPLETED".to_string(),
            project_slug: active.project_slug,
            previous_version: Some(active.active_version),
            active_version: target_ver,
            stage_reached: UpdateStage::Completed,
            backup_db_path: journal.backup_db_path,
            backup_verified_ok: true,
            signature_verified_ok: true,
            compatibility_verified_ok: true,
            post_update_auto_resume_blocked: auto_blocked,
            blocked_job_ids: unverified_checkpoint_jobs,
            binary_rollback_only: false,
            database_restored: false,
            pre_rollback_safety_db_path: None,
            journal_path: self.journal_path().to_string_lossy().to_string(),
            detail: "Upgraded Mini Tool safely with WAL backup and atomic version switch."
                .to_string(),
        })
    }

    pub fn rollback_to_previous(
        &self,
        restore_pre_update_backup: bool,
        skip_child_verify_exec: bool,
    ) -> Result<UpdateOutcome> {
        self.probe_install_permissions()?;
        let _updater_lock = UpdaterLockGuard::acquire(&self.lock_file_path())?;
        let active = self
            .read_active_pointer()?
            .ok_or_else(|| anyhow::anyhow!("ACTIVE_VERSION_MISSING: no active installation"))?;
        let prev_ver = active
            .previous_version
            .clone()
            .ok_or_else(|| anyhow::anyhow!("NO_PREVIOUS_VERSION: no previous version recorded to roll back to"))?;

        let active_exe = self
            .versions_dir()
            .join(format!("v{}", active.active_version))
            .join(&active.exe_name);
        let installed_trusted_pubkey = read_pe_embedded_pubkey_from_exe(&active_exe).with_context(|| {
            format!(
                "INSTALLED_ROOT_OF_TRUST_MISSING: failed reading trusted RT_RCDATA #{} from installed EXE {}",
                PE_ED25519_PUBKEY_RESOURCE_ID,
                active_exe.display()
            )
        })?;

        let prev_dir = self.versions_dir().join(format!("v{}", prev_ver));
        let trust_state_opt = self.read_trust_state()?;
        let prev_verified = match Self::verify_candidate_release_dir_with_anchor(
            &prev_dir,
            &RuntimeCompatibilityProfile::default(),
            Some(&installed_trusted_pubkey),
        ) {
            Ok(v) => v,
            Err(anchor_err) => {
                let unanchored = Self::verify_candidate_release_dir_with_anchor(
                    &prev_dir,
                    &RuntimeCompatibilityProfile::default(),
                    None,
                )
                .map_err(|_| anchor_err)?;
                let in_trust = trust_state_opt.as_ref().map_or(false, |ts| {
                    ts.trusted_pubkeys
                        .iter()
                        .any(|k| k.eq_ignore_ascii_case(&unanchored.pe_pubkey_hex))
                        && !ts
                            .revoked_pubkeys
                            .iter()
                            .any(|r| r.eq_ignore_ascii_case(&unanchored.pe_pubkey_hex))
                });
                if !in_trust {
                    bail!(
                        "ROLLBACK_ROOT_OF_TRUST_UNTRUSTED: previous version key '{}' is not in persistent trust store or was revoked",
                        unanchored.pe_pubkey_hex
                    );
                }
                unanchored
            }
        };
        if let Some(ref ts) = trust_state_opt {
            if ts
                .revoked_pubkeys
                .iter()
                .any(|r| r.eq_ignore_ascii_case(&prev_verified.pe_pubkey_hex))
            {
                bail!(
                    "ROLLBACK_ROOT_OF_TRUST_UNTRUSTED: previous version key '{}' has been revoked",
                    prev_verified.pe_pubkey_hex
                );
            }
        }
        if !skip_child_verify_exec {
            verify_staged_exe_bundle(&prev_verified.exe_path, &self.data_dir())?;
        }

        let live_db_path = self
            .data_dir()
            .join(format!("runtime_{}.db", active.project_slug));
        ensure_tool_quiesced(&self.data_dir(), &active.project_slug, &active_exe, &live_db_path)?;

        let live_db_ver = if live_db_path.exists() {
            highest_migration_version_raw(&live_db_path)?
        } else {
            0
        };
        let prev_max_sql = prev_verified.manifest.compatibility.sqlite_schema_version;

        // Distinct policy:
        // 1) Binary Rollback (`restore_pre_update_backup == false`): fail closed if DB schema exceeds target,
        //    otherwise keep ALL post-update user data & runtime variables completely intact.
        // 2) Database Restore (`restore_pre_update_backup == true`): ALWAYS snapshot current live DB first
        //    (`pre_rollback_safety_*.db`) before restoring `backup_db_path` so post-update data is preserved in snapshot.
        let mut restored_backup_path = None;
        let mut safety_snapshot_recorded = None;
        let mut database_restored = false;
        if live_db_ver > prev_max_sql {
            if !restore_pre_update_backup {
                check_sqlite_downgrade_compatibility(live_db_ver, prev_max_sql)?;
            }
            let bk_str = active.backup_db_path.as_ref().ok_or_else(|| {
                anyhow::anyhow!(
                    "SQLITE_DOWNGRADE_INCOMPATIBLE: live DB v{} > v{} and no pre-update backup is recorded",
                    live_db_ver,
                    prev_max_sql
                )
            })?;
            let safety_snap = self.backups_dir().join(format!(
                "pre_rollback_safety_{}.db",
                Utc::now().format("%Y%m%d_%H%M%S_%3f")
            ));
            StorageManager::rollback_database_from_backup(
                live_db_path.as_path(),
                Path::new(bk_str),
                safety_snap.as_path(),
            )?;
            safety_snapshot_recorded = Some(safety_snap.to_string_lossy().to_string());
            restored_backup_path = Some(bk_str.clone());
            database_restored = true;
        } else if restore_pre_update_backup {
            if let Some(bk_str) = active.backup_db_path.as_ref() {
                if Path::new(bk_str).exists() {
                    let safety_snap = self.backups_dir().join(format!(
                        "pre_rollback_safety_{}.db",
                        Utc::now().format("%Y%m%d_%H%M%S_%3f")
                    ));
                    StorageManager::rollback_database_from_backup(
                        live_db_path.as_path(),
                        Path::new(bk_str),
                        safety_snap.as_path(),
                    )?;
                    safety_snapshot_recorded = Some(safety_snap.to_string_lossy().to_string());
                    restored_backup_path = Some(bk_str.clone());
                    database_restored = true;
                }
            }
        }

        let journal_id = format!("rbk-{}", Uuid::new_v4());
        let mut journal = UpdateJournal {
            journal_id: journal_id.clone(),
            operation: "rollback".to_string(),
            project_slug: active.project_slug.clone(),
            source_version: Some(active.active_version.clone()),
            target_version: prev_ver.clone(),
            source_release_dir: prev_dir.to_string_lossy().to_string(),
            stage: UpdateStage::RolledBack,
            backup_db_path: restored_backup_path.clone().or_else(|| active.backup_db_path.clone()),
            live_db_path: live_db_path.to_string_lossy().to_string(),
            staged_staging_dir: None,
            staged_version_dir: prev_dir.to_string_lossy().to_string(),
            previous_active_pointer: Some(active.clone()),
            target_compat: prev_verified.manifest.compatibility.clone(),
            auto_resume_blocked_jobs: vec![],
            last_error: None,
            updated_at_utc: Utc::now().to_rfc3339(),
        };

        let rolled_pointer = ActiveVersionPointer {
            project_slug: active.project_slug.clone(),
            active_version: prev_ver.clone(),
            previous_version: None,
            exe_name: prev_verified.manifest.project.exe_name.clone(),
            sqlite_schema_version: prev_max_sql,
            min_runtime_version: prev_verified
                .manifest
                .compatibility
                .min_runtime_version
                .clone(),
            trusted_pubkey_hex: Some(prev_verified.pe_pubkey_hex.clone()),
            post_update_auto_resume_blocked: true,
            blocked_job_ids: active.blocked_job_ids.clone(),
            backup_db_path: restored_backup_path.clone().or_else(|| active.backup_db_path.clone()),
            activated_at_utc: Utc::now().to_rfc3339(),
            journal_id,
        };
        self.write_active_pointer_with_shadow(&rolled_pointer)?;
        self.write_journal(&mut journal)?;

        Ok(UpdateOutcome {
            operation: "rollback".to_string(),
            status: "ROLLED_BACK".to_string(),
            project_slug: active.project_slug,
            previous_version: Some(active.active_version),
            active_version: prev_ver,
            stage_reached: UpdateStage::RolledBack,
            backup_db_path: restored_backup_path,
            backup_verified_ok: true,
            signature_verified_ok: true,
            compatibility_verified_ok: true,
            post_update_auto_resume_blocked: true,
            blocked_job_ids: rolled_pointer.blocked_job_ids,
            binary_rollback_only: !database_restored,
            database_restored,
            pre_rollback_safety_db_path: safety_snapshot_recorded,
            journal_path: self.journal_path().to_string_lossy().to_string(),
            detail: if database_restored {
                "Rolled back binary AND explicitly restored pre-update SQLite backup (newer live data preserved in pre_rollback_safety snapshot)."
            } else {
                "Binary-only rollback completed; live SQLite database and post-update user data preserved cleanly."
            }.to_string(),
        })
    }

    pub fn reconcile_incomplete_journal(&self, skip_child_verify_exec: bool) -> Result<ReconcileOutcome> {
        self.probe_install_permissions()?;
        let _updater_lock = UpdaterLockGuard::acquire(&self.lock_file_path())?;
        let journal_opt = self.read_journal()?;
        let pointer_valid = self.read_active_pointer().ok().flatten().is_some();

        let Some(mut journal) = journal_opt else {
            if !pointer_valid {
                if let Some(recovered) = self.recover_missing_or_corrupt_pointer(None)? {
                    self.write_active_pointer_with_shadow(&recovered)?;
                    return Ok(ReconcileOutcome {
                        status: "RECOVERED_POINTER".to_string(),
                        action_taken: "REBUILT_CORRUPT_OR_MISSING_ACTIVE_POINTER".to_string(),
                        recovered_from_stage: None,
                        active_version: Some(recovered.active_version),
                        db_integrity_ok: true,
                        journal_path: self.journal_path().to_string_lossy().to_string(),
                        detail: "active_version.json was missing or corrupt; recovered valid signed version pointer.".to_string(),
                    });
                }
            }
            return Ok(ReconcileOutcome {
                status: "CLEAN".to_string(),
                action_taken: "NO_JOURNAL".to_string(),
                recovered_from_stage: None,
                active_version: self.read_active_pointer()?.map(|p| p.active_version),
                db_integrity_ok: true,
                journal_path: self.journal_path().to_string_lossy().to_string(),
                detail: "No update journal present.".to_string(),
            });
        };

        let recovered_from = journal.stage;
        let live_db_path = PathBuf::from(&journal.live_db_path);

        match recovered_from {
            UpdateStage::Idle | UpdateStage::Completed | UpdateStage::RolledBack => {
                if !pointer_valid {
                    if let Some(recovered) = self.recover_missing_or_corrupt_pointer(Some(&journal))? {
                        self.write_active_pointer_with_shadow(&recovered)?;
                        return Ok(ReconcileOutcome {
                            status: "RECOVERED_POINTER".to_string(),
                            action_taken: "REBUILT_CORRUPT_OR_MISSING_ACTIVE_POINTER".to_string(),
                            recovered_from_stage: Some(recovered_from),
                            active_version: Some(recovered.active_version),
                            db_integrity_ok: verify_db_ok_if_exists(&live_db_path),
                            journal_path: self.journal_path().to_string_lossy().to_string(),
                            detail: "Recovered missing or corrupt active_version.json from shadow/version scan.".to_string(),
                        });
                    }
                }
                let db_ok = verify_db_ok_if_exists(&live_db_path);
                return Ok(ReconcileOutcome {
                    status: "CLEAN".to_string(),
                    action_taken: "ALREADY_FINALIZED".to_string(),
                    recovered_from_stage: Some(recovered_from),
                    active_version: self.read_active_pointer()?.map(|p| p.active_version),
                    db_integrity_ok: db_ok,
                    journal_path: self.journal_path().to_string_lossy().to_string(),
                    detail: "Journal already finalized cleanly.".to_string(),
                });
            }

            UpdateStage::PreflightVerified
            | UpdateStage::QuiescedAndBackedUp
            | UpdateStage::Staged
            | UpdateStage::StagedVerified
            | UpdateStage::Failed => {
                // active_version.json has NOT been flipped yet -> remove incomplete staging, leave old active version intact!
                if let Some(staging) = journal.staged_staging_dir.as_ref() {
                    let sp = Path::new(staging);
                    if sp.exists() {
                        let _ = fs::remove_dir_all(sp);
                    }
                }
                if journal.operation == "upgrade" {
                    let prev_ver = journal
                        .previous_active_pointer
                        .as_ref()
                        .map(|p| p.active_version.as_str());
                    if prev_ver != Some(journal.target_version.as_str()) {
                        let target_vp = Path::new(&journal.staged_version_dir);
                        if target_vp.exists() {
                            let _ = fs::remove_dir_all(target_vp);
                        }
                    }
                }
                if let Some(prev_ptr) = journal.previous_active_pointer.as_ref() {
                    self.write_active_pointer_with_shadow(prev_ptr)?;
                }
                journal.staged_staging_dir = None;
                journal.stage = UpdateStage::RolledBack;
                journal.last_error = Some(format!(
                    "Recovered from interrupted stage {:?} -> preserved previous version",
                    recovered_from
                ));
                self.write_journal(&mut journal)?;
                let db_ok = verify_db_ok_if_exists(&live_db_path);
                Ok(ReconcileOutcome {
                    status: "RECOVERED_TO_PREVIOUS".to_string(),
                    action_taken: "CLEANED_STAGING_PRESERVED_ACTIVE".to_string(),
                    recovered_from_stage: Some(recovered_from),
                    active_version: self.read_active_pointer()?.map(|p| p.active_version),
                    db_integrity_ok: db_ok,
                    journal_path: self.journal_path().to_string_lossy().to_string(),
                    detail: format!(
                        "Reconciled crash from stage {:?}; restored/preserved active version.",
                        recovered_from
                    ),
                })
            }

            UpdateStage::Activated => {
                // Active pointer already points to target version. Verify staged dir & DB integrity;
                // verify strictly against previous trusted anchor (allowing valid KeyRotationDelegation!).
                let anchor_opt = journal
                    .previous_active_pointer
                    .as_ref()
                    .and_then(|p| p.trusted_pubkey_hex.clone());
                let target_dir = PathBuf::from(&journal.staged_version_dir);
                let mut rotated_ver_opt: Option<VerifiedCandidateRelease> = None;
                let forward_ok = if target_dir.exists() {
                    if let Ok(ver_rel) = Self::verify_candidate_release_dir_with_anchor(
                        &target_dir,
                        &RuntimeCompatibilityProfile::default(),
                        anchor_opt.as_deref(),
                    ) {
                        let exec_ok = if skip_child_verify_exec {
                            true
                        } else {
                            verify_staged_exe_bundle(&ver_rel.exe_path, &self.data_dir()).is_ok()
                        };
                        if exec_ok {
                            rotated_ver_opt = Some(ver_rel);
                        }
                        exec_ok
                    } else {
                        false
                    }
                } else {
                    false
                };

                if forward_ok && verify_db_ok_if_exists(&live_db_path) {
                    // Ensure Trust State <-> Active Release <-> Nonce Ledger stay 100% consistent after rotation crash!
                    if let Some(ref ver_rel) = rotated_ver_opt {
                        let mut ts = self.read_trust_state()?.unwrap_or_else(|| PersistentTrustState {
                            trust_epoch: 1,
                            publisher_id: ver_rel.publisher_id.clone(),
                            trusted_pubkeys: anchor_opt.into_iter().collect(),
                            revoked_pubkeys: vec![],
                            used_rotation_nonces: vec![],
                            previous_state_sha256: None,
                            updated_at_utc: Utc::now().to_rfc3339(),
                        });
                        let mut changed = false;
                        if !ts
                            .trusted_pubkeys
                            .iter()
                            .any(|k| k.eq_ignore_ascii_case(&ver_rel.pe_pubkey_hex))
                        {
                            ts.trusted_pubkeys.push(ver_rel.pe_pubkey_hex.clone());
                            changed = true;
                        }
                        if ver_rel.rotated_from_pubkey_hex.is_some() {
                            if let Some(ref deleg) = ver_rel
                                .manifest
                                .signing
                                .as_ref()
                                .and_then(|s| s.key_rotation.as_ref())
                            {
                                if !ts.used_rotation_nonces.contains(&deleg.nonce) {
                                    ts.used_rotation_nonces.push(deleg.nonce.clone());
                                    changed = true;
                                }
                            }
                        }
                        if changed {
                            ts.trust_epoch = ts.trust_epoch.saturating_add(1).max(1);
                            ts.previous_state_sha256 = None;
                            ts.updated_at_utc = Utc::now().to_rfc3339();
                            self.write_trust_state(&ts)?;
                        }
                    }
                    if !pointer_valid {
                        if let Some(recovered) = self.recover_missing_or_corrupt_pointer(Some(&journal))? {
                            self.write_active_pointer_with_shadow(&recovered)?;
                        }
                    }
                    journal.stage = UpdateStage::Completed;
                    self.write_journal(&mut journal)?;
                    Ok(ReconcileOutcome {
                        status: "RECOVERED_ROLL_FORWARD".to_string(),
                        action_taken: "VERIFIED_ACTIVATED_RELEASE".to_string(),
                        recovered_from_stage: Some(recovered_from),
                        active_version: self.read_active_pointer()?.map(|p| p.active_version),
                        db_integrity_ok: true,
                        journal_path: self.journal_path().to_string_lossy().to_string(),
                        detail: "Crash happened after atomic activation; verified target release & synchronized trust ledger.".to_string(),
                    })
                } else {
                    if let (Some(bk), true) = (journal.backup_db_path.as_ref(), live_db_path.exists()) {
                        let safety_snap = self.backups_dir().join(format!(
                            "reconcile_safety_{}.db",
                            Utc::now().format("%Y%m%d_%H%M%S_%3f")
                        ));
                        let _ = StorageManager::rollback_database_from_backup(
                            live_db_path.as_path(),
                            Path::new(bk),
                            safety_snap.as_path(),
                        );
                    }
                    if let Some(prev_ptr) = journal.previous_active_pointer.as_ref() {
                        self.write_active_pointer_with_shadow(prev_ptr)?;
                    }
                    journal.stage = UpdateStage::RolledBack;
                    self.write_journal(&mut journal)?;
                    let db_ok = verify_db_ok_if_exists(&live_db_path);
                    Ok(ReconcileOutcome {
                        status: "RECOVERED_TO_PREVIOUS".to_string(),
                        action_taken: "REVERTED_CORRUPT_ACTIVATION".to_string(),
                        recovered_from_stage: Some(recovered_from),
                        active_version: self.read_active_pointer()?.map(|p| p.active_version),
                        db_integrity_ok: db_ok,
                        journal_path: self.journal_path().to_string_lossy().to_string(),
                        detail: "Reverted interrupted activation back to previous verified version.".to_string(),
                    })
                }
            }
        }
    }

    fn recover_missing_or_corrupt_pointer(
        &self,
        journal_opt: Option<&UpdateJournal>,
    ) -> Result<Option<ActiveVersionPointer>> {
        // 1. Try redundant shadow backup pointer first
        let bak_path = self.backup_pointer_path();
        if bak_path.exists() {
            if let Ok(bytes) = fs::read(&bak_path) {
                if let Ok(ptr) = serde_json::from_slice::<ActiveVersionPointer>(&bytes) {
                    let vdir = self.versions_dir().join(format!("v{}", ptr.active_version));
                    if Self::verify_candidate_release_dir_with_anchor(
                        &vdir,
                        &RuntimeCompatibilityProfile::default(),
                        ptr.trusted_pubkey_hex.as_deref(),
                    )
                    .is_ok()
                    {
                        return Ok(Some(ptr));
                    }
                }
            }
        }
        // 2. Fallback to previous pointer in journal ONLY if the journal did not complete an upgrade
        // (when Completed, scanning trusted versions/v* below recovers the highest completed active version).
        if let Some(j) = journal_opt {
            if !matches!(j.stage, UpdateStage::Completed) {
                if let Some(ref prev_ptr) = j.previous_active_pointer {
                    let vdir = self.versions_dir().join(format!("v{}", prev_ptr.active_version));
                    if Self::verify_candidate_release_dir_with_anchor(
                        &vdir,
                        &RuntimeCompatibilityProfile::default(),
                        prev_ptr.trusted_pubkey_hex.as_deref(),
                    )
                    .is_ok()
                    {
                        return Ok(Some(prev_ptr.clone()));
                    }
                }
            }
        }
        // 3. Scan `versions/v*` ONLY against persistent `.updater/trust-state.json` + `.updater/trust-ledger.db` (P5.4 anti-rollback enforced).
        // Never blindly trust an unanchored folder on disk when both pointers & journal are missing.
        let trust_state_opt = self.read_trust_state()?;
        let Some(ref trust_state) = trust_state_opt else {
            return Ok(None);
        };
        if trust_state.trusted_pubkeys.is_empty() {
            return Ok(None);
        }

        let mut best: Option<(SemVer, ActiveVersionPointer)> = None;
        if let Ok(entries) = fs::read_dir(self.versions_dir()) {
            for ent in entries.flatten() {
                if !ent.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    continue;
                }
                let p = ent.path();
                if let Ok(ver) =
                    Self::verify_candidate_release_dir_with_anchor(&p, &RuntimeCompatibilityProfile::default(), None)
                {
                    let is_trusted = trust_state
                        .trusted_pubkeys
                        .iter()
                        .any(|k| k.eq_ignore_ascii_case(&ver.pe_pubkey_hex))
                        && !trust_state
                            .revoked_pubkeys
                            .iter()
                            .any(|r| r.eq_ignore_ascii_case(&ver.pe_pubkey_hex));
                    if !is_trusted {
                        continue;
                    }
                    if let Ok(sv) = SemVer::parse(&ver.manifest.project.version) {
                        let ptr = ActiveVersionPointer {
                            project_slug: ver.manifest.project.slug.clone(),
                            active_version: ver.manifest.project.version.clone(),
                            previous_version: None,
                            exe_name: ver.manifest.project.exe_name.clone(),
                            sqlite_schema_version: ver.manifest.compatibility.sqlite_schema_version,
                            min_runtime_version: ver.manifest.compatibility.min_runtime_version.clone(),
                            trusted_pubkey_hex: Some(ver.pe_pubkey_hex.clone()),
                            post_update_auto_resume_blocked: true,
                            blocked_job_ids: vec![],
                            backup_db_path: None,
                            activated_at_utc: Utc::now().to_rfc3339(),
                            journal_id: "recovered-scan".to_string(),
                        };
                        let replace = best.as_ref().map(|(cur, _)| sv > *cur).unwrap_or(true);
                        if replace {
                            best = Some((sv, ptr));
                        }
                    }
                }
            }
        }
        Ok(best.map(|(_, p)| p))
    }
}

fn maybe_trigger_fault(point: FaultInjectionPoint, options: &UpdateExecutionOptions) {
    if let Some((pause_pt, ms)) = options.pause_after_stage_ms {
        if pause_pt == point {
            println!("PAUSED_AT:{}", point.label());
            let _ = std::io::stdout().flush();
            std::thread::sleep(std::time::Duration::from_millis(ms));
        }
    }
    if options.crash_after_stage == Some(point) {
        eprintln!("FAULT_INJECTED_CRASH_AT:{}", point.label());
        std::process::exit(99);
    }
}

fn verify_db_ok_if_exists(db_path: &Path) -> bool {
    if !db_path.exists() {
        return true;
    }
    StorageManager::new(db_path)
        .and_then(|s| s.check_integrity())
        .map(|r| r.is_healthy)
        .unwrap_or(false)
}

fn highest_migration_version_raw(db_path: &Path) -> Result<u32> {
    let conn = Connection::open(db_path)?;
    let has_table: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='schema_migrations';",
        [],
        |r| r.get(0),
    )?;
    if has_table == 0 {
        return Ok(0);
    }
    let max_v: Option<i64> =
        conn.query_row("SELECT MAX(version) FROM schema_migrations;", [], |r| r.get(0))?;
    Ok(max_v.unwrap_or(0).max(0) as u32)
}

fn find_unverified_checkpoint_jobs(db_path: &Path) -> Result<Vec<String>> {
    if !db_path.exists() {
        return Ok(vec![]);
    }
    let conn = Connection::open(db_path)?;
    let has_cp: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='checkpoints';",
        [],
        |r| r.get(0),
    )?;
    if has_cp == 0 {
        return Ok(vec![]);
    }
    let mut stmt = conn.prepare(
        "SELECT DISTINCT job_id FROM checkpoints
         WHERE evidence_ref IS NULL OR TRIM(evidence_ref) = ''
         ORDER BY job_id ASC;",
    )?;
    let ids = stmt
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(ids)
}

fn ensure_tool_quiesced(
    data_dir: &Path,
    slug: &str,
    active_exe: &Path,
    live_db_path: &Path,
) -> Result<()> {
    // 1. Check PID lock file if any
    let pid_file = data_dir.join(format!(".runtime_{}.pid", slug));
    if pid_file.exists() {
        if let Ok(raw) = fs::read_to_string(&pid_file) {
            if let Ok(pid) = raw.trim().parse::<u32>() {
                if is_process_alive(pid) {
                    bail!(
                        "MINI_TOOL_PROCESS_RUNNING: active PID {} detected in {}; stop the tool before updating",
                        pid,
                        pid_file.display()
                    );
                } else {
                    let _ = fs::remove_file(&pid_file);
                }
            }
        }
    }

    // 2. Check Windows executable sharing lock on currently active binary
    if active_exe.exists() && is_exe_file_locked(active_exe) {
        bail!(
            "EXE_FILE_LOCKED: active Mini Tool executable '{}' is currently running/locked; stop all instances before updating",
            active_exe.display()
        );
    }

    // 3. Check SQLite jobs table for running / verifying jobs
    if live_db_path.exists() {
        let conn = Connection::open(live_db_path)?;
        let has_jobs: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='jobs';",
            [],
            |r| r.get(0),
        )?;
        if has_jobs > 0 {
            let mut stmt = conn.prepare(
                "SELECT id, state FROM jobs WHERE LOWER(state) IN ('running', 'queued', 'verifying') LIMIT 1;",
            )?;
            let mut rows = stmt.query([])?;
            if let Some(row) = rows.next()? {
                let job_id: String = row.get(0)?;
                let state: String = row.get(1)?;
                bail!(
                    "ACTIVE_JOB_RUNNING: job '{}' is currently in state '{}'; pause or wait until quiesced before updating",
                    job_id,
                    state
                );
            }
        }
    }

    Ok(())
}

fn verify_staged_exe_bundle(exe_path: &Path, data_dir: &Path) -> Result<()> {
    let out = std::process::Command::new(exe_path)
        .arg("--verify-bundle")
        .env("FP_MINI_DATA_DIR", data_dir)
        .output()
        .with_context(|| format!("Failed executing {} --verify-bundle", exe_path.display()))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let stdout = String::from_utf8_lossy(&out.stdout);
        bail!(
            "STAGED_EXE_VERIFY_FAILED (exit {:?}): {} {}",
            out.status.code(),
            stderr.trim(),
            stdout.trim()
        );
    }
    Ok(())
}

struct UpdaterLockGuard {
    #[cfg(windows)]
    handle: *mut core::ffi::c_void,
    path: PathBuf,
}

impl UpdaterLockGuard {
    #[cfg(windows)]
    fn acquire(lock_path: &Path) -> Result<Self> {
        use std::os::windows::ffi::OsStrExt;
        const GENERIC_WRITE: u32 = 0x40000000;
        const OPEN_ALWAYS: u32 = 4;
        const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;
        const ERROR_SHARING_VIOLATION: u32 = 32;

        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn CreateFileW(
                lpFileName: *const u16,
                dwDesiredAccess: u32,
                dwShareMode: u32,
                lpSecurityAttributes: *const core::ffi::c_void,
                dwCreationDisposition: u32,
                dwFlagsAndAttributes: u32,
                hTemplateFile: *mut core::ffi::c_void,
            ) -> *mut core::ffi::c_void;
            fn GetLastError() -> u32;
        }

        if let Some(p) = lock_path.parent() {
            fs::create_dir_all(p)?;
        }
        let wide: Vec<u16> = lock_path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        unsafe {
            let invalid = (-1isize) as *mut core::ffi::c_void;
            let h = CreateFileW(
                wide.as_ptr(),
                GENERIC_WRITE,
                0, // Exclusive lock: 0 share mode blocks concurrent updaters on same install_root
                std::ptr::null(),
                OPEN_ALWAYS,
                FILE_ATTRIBUTE_NORMAL,
                std::ptr::null_mut(),
            );
            if h == invalid {
                let err = GetLastError();
                if err == ERROR_SHARING_VIOLATION {
                    bail!(
                        "CONCURRENT_UPDATER_LOCKED: another updater instance holds exclusive lock at {}",
                        lock_path.display()
                    );
                }
                bail!(
                    "UPDATER_LOCK_FAILED (os error {}): {}",
                    err,
                    lock_path.display()
                );
            }
            Ok(Self {
                handle: h,
                path: lock_path.to_path_buf(),
            })
        }
    }

    #[cfg(not(windows))]
    fn acquire(lock_path: &Path) -> Result<Self> {
        Ok(Self {
            path: lock_path.to_path_buf(),
        })
    }
}

impl Drop for UpdaterLockGuard {
    fn drop(&mut self) {
        #[cfg(windows)]
        unsafe {
            #[link(name = "kernel32")]
            unsafe extern "system" {
                fn CloseHandle(hObject: *mut core::ffi::c_void) -> i32;
            }
            let _ = CloseHandle(self.handle);
        }
        let _ = fs::remove_file(&self.path);
    }
}

fn validate_single_filename(name: &str) -> Result<()> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.contains('/') || trimmed.contains('\\') {
        bail!("PATH_SECURITY_ERROR: invalid filename '{}'", name);
    }
    let mut comps = Path::new(trimmed).components();
    match (comps.next(), comps.next()) {
        (Some(Component::Normal(_)), None) => Ok(()),
        _ => bail!("PATH_SECURITY_ERROR: traversal or root in filename '{}'", name),
    }
}

fn ensure_not_reparse_point(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() {
        bail!(
            "REPARSE_POINT_OR_JUNCTION_FORBIDDEN: symlink detected at {}",
            path.display()
        );
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
        if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            bail!(
                "REPARSE_POINT_OR_JUNCTION_FORBIDDEN: NTFS junction/reparse point at {}",
                path.display()
            );
        }
    }
    Ok(())
}

fn ensure_no_reparse_or_junction_tree(root: &Path) -> Result<()> {
    ensure_not_reparse_point(root)?;
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let p = entry.path();
        ensure_not_reparse_point(&p)?;
        let meta = fs::symlink_metadata(&p)?;
        if meta.is_dir() {
            ensure_no_reparse_or_junction_tree(&p)?;
        }
    }
    Ok(())
}

fn atomic_write_file(dest: &Path, content: &[u8]) -> Result<()> {
    if let Some(parent) = dest.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let file_name = dest
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let tmp_path = dest.with_file_name(format!(".{}.tmp.{}", file_name, Uuid::new_v4().simple()));
    {
        let mut f = File::create(&tmp_path)?;
        f.write_all(content)?;
        f.sync_all()?;
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
        const MOVEFILE_WRITE_THROUGH: u32 = 0x8;
        const ERROR_SHARING_VIOLATION: u32 = 32;
        const ERROR_ACCESS_DENIED: u32 = 5;

        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn MoveFileExW(
                lpExistingFileName: *const u16,
                lpNewFileName: *const u16,
                dwFlags: u32,
            ) -> i32;
            fn GetLastError() -> u32;
        }
        let src_w: Vec<u16> = tmp_path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let dst_w: Vec<u16> = dest
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let mut last_err_code = 0u32;
        let mut moved = false;
        for attempt in 0..5 {
            let ok = unsafe {
                MoveFileExW(
                    src_w.as_ptr(),
                    dst_w.as_ptr(),
                    MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
                )
            };
            if ok != 0 {
                moved = true;
                break;
            }
            last_err_code = unsafe { GetLastError() };
            if (last_err_code == ERROR_SHARING_VIOLATION || last_err_code == ERROR_ACCESS_DENIED)
                && attempt < 4
            {
                std::thread::sleep(std::time::Duration::from_millis(35));
                continue;
            }
            break;
        }
        if !moved {
            let code = last_err_code;
            let _ = fs::remove_file(&tmp_path);
            if code == ERROR_SHARING_VIOLATION || code == ERROR_ACCESS_DENIED {
                bail!(
                    "ACTIVE_POINTER_LOCKED (win32 error {}): cannot atomically replace {}",
                    code,
                    dest.display()
                );
            }
            bail!(
                "ATOMIC_REPLACE_FAILED (win32 error {}): {}",
                code,
                dest.display()
            );
        }
        return Ok(());
    }
    #[cfg(not(windows))]
    {
        fs::rename(&tmp_path, dest)?;
        Ok(())
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

#[cfg(windows)]
fn is_process_alive(pid: u32) -> bool {
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const STILL_ACTIVE: u32 = 259;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: i32, dwProcessId: u32) -> *mut core::ffi::c_void;
        fn GetExitCodeProcess(hProcess: *mut core::ffi::c_void, lpExitCode: *mut u32) -> i32;
        fn CloseHandle(hObject: *mut core::ffi::c_void) -> i32;
    }

    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h.is_null() {
            return false;
        }
        let mut code: u32 = 0;
        let ok = GetExitCodeProcess(h, &mut code);
        let _ = CloseHandle(h);
        ok != 0 && code == STILL_ACTIVE
    }
}

#[cfg(not(windows))]
fn is_process_alive(_pid: u32) -> bool {
    false
}

#[cfg(windows)]
fn is_exe_file_locked(exe_path: &Path) -> bool {
    use std::os::windows::ffi::OsStrExt;
    const GENERIC_WRITE: u32 = 0x40000000;
    const OPEN_EXISTING: u32 = 3;
    const ERROR_SHARING_VIOLATION: u32 = 32;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateFileW(
            lpFileName: *const u16,
            dwDesiredAccess: u32,
            dwShareMode: u32,
            lpSecurityAttributes: *const core::ffi::c_void,
            dwCreationDisposition: u32,
            dwFlagsAndAttributes: u32,
            hTemplateFile: *mut core::ffi::c_void,
        ) -> *mut core::ffi::c_void;
        fn GetLastError() -> u32;
        fn CloseHandle(hObject: *mut core::ffi::c_void) -> i32;
    }

    let wide: Vec<u16> = exe_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        let invalid = (-1isize) as *mut core::ffi::c_void;
        let h = CreateFileW(
            wide.as_ptr(),
            GENERIC_WRITE,
            0,
            std::ptr::null(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        );
        if h == invalid {
            return GetLastError() == ERROR_SHARING_VIOLATION;
        }
        let _ = CloseHandle(h);
        false
    }
}

#[cfg(not(windows))]
fn is_exe_file_locked(_exe_path: &Path) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use rusqlite::params;
    use fp_builder::pe_icon::inject_ed25519_pubkey_into_pe_exe;
    use fp_bundle_contract::{
        CompatibilityVersions, GameProjectManifest, MiniLayoutPreset, CURRENT_RUNTIME_VERSION,
    };
    use fp_bundle_verify::sha256_hex;
    use fp_workflow_schema::{NodePosition, WorkflowEdge, WorkflowGraph, WorkflowNode};
    use std::collections::{BTreeMap, HashMap};

    fn create_signed_test_release(
        dir: &Path,
        slug: &str,
        exe_name: &str,
        version: &str,
        sqlite_schema_ver: u32,
        wf_schema_ver: u32,
        signing_seed: [u8; 32],
    ) {
        let bundle_dir = dir.join("bundle");
        fs::create_dir_all(&bundle_dir).unwrap();

        let signing_key = SigningKey::from_bytes(&signing_seed);
        let pub_hex: String = signing_key
            .verifying_key()
            .to_bytes()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect();

        // Copy current test runner EXE as a genuine Windows PE binary and stamp RT_RCDATA #7777
        let current_exe = std::env::current_exe().unwrap();
        let target_exe = dir.join(exe_name);
        fs::copy(&current_exe, &target_exe).unwrap();
        inject_ed25519_pubkey_into_pe_exe(&target_exe, &pub_hex).unwrap();

        let compat = CompatibilityVersions {
            project_schema_version: 1,
            min_runtime_version: CURRENT_RUNTIME_VERSION.to_string(),
            workflow_schema_version: wf_schema_ver,
            ui_schema_version: 1,
            module_api_version: "1.0.0".to_string(),
            sqlite_schema_version: sqlite_schema_ver,
        };
        let manifest = ReleaseBundleManifest {
            compatibility: compat.clone(),
            project: GameProjectManifest {
                compatibility: compat,
                project_id: format!("proj-{}", slug),
                slug: slug.to_string(),
                name: format!("Tool {}", slug),
                game_package_id: format!("com.test.{}", slug),
                version: version.to_string(),
                exe_name: exe_name.to_string(),
                window_title: format!("Tool {} v{}", slug, version),
                icon_rel_path: "branding/icon.ico".to_string(),
                target_resolution: (1280, 720),
                packaged_modules: vec![
                    "mod.adb_core".to_string(),
                    "mod.vision_ocr".to_string(),
                ],
                workflow_files: vec!["wf_main.json".to_string()],
                template_files: vec![],
                bundle_adb: false,
            },
            module_states: vec![],
            workflows_count: 1,
            templates_count: 0,
            built_at_utc: Utc::now().to_rfc3339(),
            publisher: None,
            signing: None,
        };
        let ui_schema = MiniToolUiSchema {
            schema_version: 1,
            project_slug: slug.to_string(),
            window_title: format!("Tool {} v{}", slug, version),
            version_badge: format!("v{}", version),
            theme_accent_hex: "#1878F5".to_string(),
            header_bg_hex: "#121E2A".to_string(),
            layout_preset: MiniLayoutPreset::Compact1Col,
            default_window_size: (680, 480),
            sections_order: vec![],
            hidden_components: vec![],
            device_columns: vec![],
            task_categories: vec![],
            tasks: vec![],
            utilities: vec![],
            initial_runtime_vars: HashMap::new(),
        };
        let workflows = vec![WorkflowDocument {
            schema_version: wf_schema_ver,
            workflow_id: "wf-main".to_string(),
            name: "Main".to_string(),
            status: "Published".to_string(),
            graph: WorkflowGraph {
                nodes: vec![
                    WorkflowNode {
                        id: "n1".to_string(),
                        node_type: "Trigger".to_string(),
                        position: NodePosition { x: 0.0, y: 0.0 },
                        config: serde_json::json!({}),
                        timeout_ms: None,
                        retry_policy: None,
                    },
                    WorkflowNode {
                        id: "n2".to_string(),
                        node_type: "End".to_string(),
                        position: NodePosition { x: 100.0, y: 0.0 },
                        config: serde_json::json!({}),
                        timeout_ms: None,
                        retry_policy: None,
                    },
                ],
                edges: vec![WorkflowEdge {
                    id: "e1".to_string(),
                    source: "n1".to_string(),
                    source_port: "out".to_string(),
                    target: "n2".to_string(),
                    target_port: "in".to_string(),
                }],
            },
            runtime_policy: None,
            disclaimer: None,
        }];

        let m_bytes = serde_json::to_vec_pretty(&manifest).unwrap();
        let u_bytes = serde_json::to_vec_pretty(&ui_schema).unwrap();
        let w_bytes = serde_json::to_vec_pretty(&workflows).unwrap();
        fs::write(bundle_dir.join("manifest.fpbundle.json"), &m_bytes).unwrap();
        fs::write(bundle_dir.join("ui-schema.json"), &u_bytes).unwrap();
        fs::write(bundle_dir.join("workflows.fpbundle.json"), &w_bytes).unwrap();
        fs::write(bundle_dir.join("trusted_publisher.pub"), pub_hex.as_bytes()).unwrap();

        let mut checksums = BTreeMap::new();
        checksums.insert("manifest.fpbundle.json".to_string(), sha256_hex(&m_bytes));
        checksums.insert("ui-schema.json".to_string(), sha256_hex(&u_bytes));
        checksums.insert("workflows.fpbundle.json".to_string(), sha256_hex(&w_bytes));

        let manifest_sha = sha256_hex(&m_bytes);
        let tbl_json = serde_json::to_string(&checksums).unwrap();
        let tbl_hash = sha256_hex(tbl_json.as_bytes());
        let msg = fp_bundle_verify::canonical_signing_message("test-key-p52", &manifest_sha, &tbl_hash);
        let sig = signing_key.sign(&msg);
        let sig_hex: String = sig.to_bytes().iter().map(|b| format!("{:02x}", b)).collect();

        let env = SignedBundleEnvelope {
            algorithm: "Ed25519-RFC8032-SHA256".to_string(),
            key_id: "test-key-p52".to_string(),
            publisher_public_key_hex: pub_hex,
            manifest_sha256: manifest_sha,
            checksums_table_sha256: tbl_hash,
            file_checksums: checksums,
            signature_ed25519_hex: sig_hex,
            signed_at_utc: Utc::now().to_rfc3339(),
        };
        fs::write(
            bundle_dir.join("signature.fpbundle.json"),
            serde_json::to_vec_pretty(&env).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn test_p5_2_updater_lifecycle_guards_crash_recovery_and_isolation() {
        let tmp = std::env::temp_dir().join(format!("fp_updater_p52_{}", Uuid::new_v4()));
        let rel_100 = tmp.join("releases/alpha_v1_0_0");
        let rel_110 = tmp.join("releases/alpha_v1_1_0");
        let rel_beta = tmp.join("releases/beta_v1_0_0");
        let seed_a = [42u8; 32];
        let seed_b = [77u8; 32];

        create_signed_test_release(
            &rel_100,
            "alpha-game",
            "Auto_Alpha.exe",
            "1.0.0",
            CURRENT_SQLITE_SCHEMA_VERSION,
            1,
            seed_a,
        );
        create_signed_test_release(
            &rel_110,
            "alpha-game",
            "Auto_Alpha.exe",
            "1.1.0",
            CURRENT_SQLITE_SCHEMA_VERSION,
            1,
            seed_a,
        );
        create_signed_test_release(
            &rel_beta,
            "beta-game",
            "Auto_Beta.exe",
            "1.0.0",
            CURRENT_SQLITE_SCHEMA_VERSION,
            1,
            seed_b,
        );

        let install_alpha = tmp.join("installed/Auto_Alpha");
        let install_beta = tmp.join("installed/Auto_Beta");
        let updater_alpha = MiniToolUpdater::new(&install_alpha);
        let updater_beta = MiniToolUpdater::new(&install_beta);
        let fast_opts = UpdateExecutionOptions {
            skip_child_verify_exec: true,
            ..Default::default()
        };

        // 1. Initial install both independent tools
        let out_a1 = updater_alpha.install_initial(&rel_100, &fast_opts).unwrap();
        let out_b1 = updater_beta.install_initial(&rel_beta, &fast_opts).unwrap();
        assert_eq!(out_a1.active_version, "1.0.0");
        assert_eq!(out_b1.active_version, "1.0.0");

        // Write persistent user data (workflow, job + unverified checkpoint, runtime var, credential ref) in Alpha & Beta data/
        let db_alpha = updater_alpha.data_dir().join("runtime_alpha-game.db");
        let db_beta = updater_beta.data_dir().join("runtime_beta-game.db");
        {
            let st_a = StorageManager::new(&db_alpha).unwrap();
            let dev_a = fp_domain::DeviceSummary {
                id: "dev-1".to_string(),
                adb_serial: "emu-1".to_string(),
                alias: "AlphaEmu".to_string(),
                android_api: Some(33),
                model: Some("SM-G998B".to_string()),
                width_px: 1280,
                height_px: 720,
                rotation_deg: 0,
                connection_state: fp_domain::ConnectionState::Ready,
                updated_at: Utc::now(),
            };
            st_a.upsert_device(&dev_a).unwrap();
            let st_b = StorageManager::new(&db_beta).unwrap();
            let dev_b = fp_domain::DeviceSummary {
                id: "dev-b1".to_string(),
                adb_serial: "emu-b1".to_string(),
                alias: "BetaEmu".to_string(),
                android_api: Some(33),
                model: Some("Pixel7".to_string()),
                width_px: 1920,
                height_px: 1080,
                rotation_deg: 0,
                connection_state: fp_domain::ConnectionState::Ready,
                updated_at: Utc::now(),
            };
            st_b.upsert_device(&dev_b).unwrap();

            // Insert project + completed job + unverified checkpoint in Alpha DB
            let conn = Connection::open(&db_alpha).unwrap();
            let now = Utc::now().to_rfc3339();
            conn.execute(
                "INSERT INTO projects (id, slug, name, game_package_id, version, exe_name, window_title, project_dir_path, created_at, updated_at)
                 VALUES ('proj-alpha', 'alpha-game', 'Tool Alpha', 'com.test.alpha-game', '1.0.0', 'Auto_Alpha.exe', 'Tool alpha v1.0.0', 'projects/alpha-game', ?1, ?1)",
                params![now],
            ).unwrap();
            conn.execute(
                "INSERT INTO workflows (id, project_id, name, draft_json, draft_updated_at) VALUES ('wf-1', 'proj-alpha', 'Main', '{}', ?1)",
                params![now],
            ).unwrap();
            conn.execute(
                "INSERT INTO workflow_versions (id, workflow_id, version_no, graph_json, sha256, created_at) VALUES ('wfv-1', 'wf-1', 1, '{}', 'abc', ?1)",
                params![now],
            ).unwrap();
            conn.execute(
                "INSERT INTO accounts (id, project_id, alias, game_profile_id, secret_ref, state, updated_at) VALUES ('acc-a1', 'proj-alpha', 'alpha_vip', 'alpha-game', 'CRED_MGR://farmerpro/alpha_vip', 'Ready', ?1)",
                params![now],
            ).unwrap();
            conn.execute(
                "INSERT INTO jobs (id, project_id, device_id, account_id, workflow_version_id, priority, state, fencing_generation, current_node_id, created_at, updated_at)
                 VALUES ('job-paused-1', 'proj-alpha', ?1, 'acc-a1', 'wfv-1', 1, 'Paused', 1, 'n_pay', ?2, ?2)",
                params![dev_a.id, now],
            ).unwrap();
            conn.execute(
                "INSERT INTO checkpoints (id, job_id, node_id, state_json, evidence_ref, created_at)
                 VALUES ('cp-1', 'job-paused-1', 'n_pay', '{}', NULL, ?1)",
                params![now],
            ).unwrap();
        }

        // 2. Tamper rejected during upgrade
        let rel_tampered = tmp.join("releases/alpha_tampered");
        copy_dir_recursive(&rel_110, &rel_tampered).unwrap();
        fs::write(rel_tampered.join("bundle/ui-schema.json"), b"{\"tampered\":true}").unwrap();
        let err_tamper = updater_alpha
            .upgrade_from_release(&rel_tampered, &fast_opts)
            .unwrap_err()
            .to_string();
        assert!(
            err_tamper.contains("INTEGRITY_HASH_MISMATCH")
                || err_tamper.contains("SHA-256")
                || err_tamper.contains("SIGNATURE"),
            "Unexpected tamper error: {}",
            err_tamper
        );
        assert_eq!(
            updater_alpha.read_active_pointer().unwrap().unwrap().active_version,
            "1.0.0"
        );

        // 3. Incompatible schema rejected during upgrade
        let rel_incompat = tmp.join("releases/alpha_incompat");
        create_signed_test_release(
            &rel_incompat,
            "alpha-game",
            "Auto_Alpha.exe",
            "1.2.0",
            CURRENT_SQLITE_SCHEMA_VERSION,
            99,
            seed_a,
        );
        let err_compat = updater_alpha
            .upgrade_from_release(&rel_incompat, &fast_opts)
            .unwrap_err()
            .to_string();
        assert!(err_compat.contains("WORKFLOW_SCHEMA_INCOMPATIBLE"));

        // 4. Active running job guard blocks upgrade before touching backup/staging
        {
            let conn = Connection::open(&db_alpha).unwrap();
            let now = Utc::now().to_rfc3339();
            conn.execute(
                "INSERT INTO jobs (id, project_id, device_id, account_id, workflow_version_id, priority, state, fencing_generation, current_node_id, created_at, updated_at)
                 VALUES ('job-active-1', 'proj-alpha', 'dev-1', 'acc-a1', 'wfv-1', 1, 'running', 2, 'n_farm', ?1, ?1)",
                params![now],
            ).unwrap();
        }
        let err_job = updater_alpha
            .upgrade_from_release(&rel_110, &fast_opts)
            .unwrap_err()
            .to_string();
        assert!(err_job.contains("ACTIVE_JOB_RUNNING"));
        {
            let conn = Connection::open(&db_alpha).unwrap();
            conn.execute("UPDATE jobs SET state = 'Completed' WHERE id = 'job-active-1'", []).unwrap();
        }

        // 5. Real running OS process PID guard blocks upgrade while running
        let mut child_sleep = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", "Start-Sleep -Seconds 10"])
            .spawn()
            .unwrap();
        fs::write(
            updater_alpha.data_dir().join(".runtime_alpha-game.pid"),
            child_sleep.id().to_string(),
        )
        .unwrap();
        let err_pid = updater_alpha
            .upgrade_from_release(&rel_110, &fast_opts)
            .unwrap_err()
            .to_string();
        assert!(err_pid.contains("MINI_TOOL_PROCESS_RUNNING"));
        let _ = child_sleep.kill();
        let _ = child_sleep.wait();

        // 6. Read-only / missing write permission simulation
        let ro_root = tmp.join("blocked_install_root");
        fs::create_dir_all(&ro_root).unwrap();
        // Blocking `.updater` creation with a regular file mimics NTFS write failure cleanly without Admin ACL edits
        fs::write(ro_root.join(".updater"), b"blocked_file").unwrap();
        let err_perm = MiniToolUpdater::new(&ro_root)
            .install_initial(&rel_100, &fast_opts)
            .unwrap_err()
            .to_string();
        assert!(err_perm.contains("INSTALL_DIR_NOT_WRITABLE"));

        // 7. SQLite migration failure rollback
        let opts_mig_fail = UpdateExecutionOptions {
            simulate_migration_failure: true,
            skip_child_verify_exec: true,
            ..Default::default()
        };
        let err_mig = updater_alpha
            .upgrade_from_release(&rel_110, &opts_mig_fail)
            .unwrap_err()
            .to_string();
        assert!(err_mig.contains("SIMULATED_MIGRATION_FAILURE"));
        assert_eq!(
            updater_alpha.read_active_pointer().unwrap().unwrap().active_version,
            "1.0.0"
        );
        assert!(verify_db_ok_if_exists(&db_alpha));

        // 8. Real Upgrade v1.0.0 -> v1.1.0 + check post_update_auto_resume_blocked & isolated Beta untouched!
        let up_res = updater_alpha.upgrade_from_release(&rel_110, &fast_opts).unwrap();
        assert_eq!(up_res.active_version, "1.1.0");
        assert!(up_res.post_update_auto_resume_blocked);
        assert_eq!(up_res.blocked_job_ids, vec!["job-paused-1".to_string()]);
        assert_eq!(
            updater_beta.read_active_pointer().unwrap().unwrap().active_version,
            "1.0.0"
        );
        // Verify preserved account credential reference inside Alpha DB
        {
            let conn = Connection::open(&db_alpha).unwrap();
            let sref: String = conn
                .query_row("SELECT secret_ref FROM accounts WHERE id = 'acc-a1'", [], |r| r.get(0))
                .unwrap();
            assert_eq!(sref, "CRED_MGR://farmerpro/alpha_vip");
        }

        // 9. Rollback SQLite Downgrade Guard (simulate DB moved to v99 > target v4)
        {
            let conn = Connection::open(&db_alpha).unwrap();
            conn.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (99, '2026-11-22T00:00:00Z')",
                [],
            ).unwrap();
        }
        let err_dg = updater_alpha
            .rollback_to_previous(false, true)
            .unwrap_err()
            .to_string();
        assert!(err_dg.contains("SQLITE_DOWNGRADE_INCOMPATIBLE"));

        // Rollback WITH pre-update WAL backup restore succeeds cleanly!
        let rb_res = updater_alpha.rollback_to_previous(true, true).unwrap();
        assert_eq!(rb_res.active_version, "1.0.0");
        assert!(rb_res.database_restored);
        assert!(rb_res.pre_rollback_safety_db_path.is_some());
        assert_eq!(highest_migration_version_raw(&db_alpha).unwrap(), CURRENT_SQLITE_SCHEMA_VERSION);
        assert!(verify_db_ok_if_exists(&db_alpha));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_p5_2_closeout_root_of_trust_rollback_data_preservation_locks_junction_and_pointer_recovery() {
        let tmp = std::env::temp_dir().join(format!("fp_updater_closeout_{}", Uuid::new_v4()));
        let rel_100 = tmp.join("rel_100");
        let rel_110 = tmp.join("rel_110");
        let rel_rogue_120 = tmp.join("rel_rogue_120");
        let trusted_seed = [11u8; 32];
        let rogue_seed = [99u8; 32];

        create_signed_test_release(
            &rel_100,
            "closeout-game",
            "Auto_Closeout.exe",
            "1.0.0",
            CURRENT_SQLITE_SCHEMA_VERSION,
            1,
            trusted_seed,
        );
        create_signed_test_release(
            &rel_110,
            "closeout-game",
            "Auto_Closeout.exe",
            "1.1.0",
            CURRENT_SQLITE_SCHEMA_VERSION,
            1,
            trusted_seed,
        );
        // Self-consistent rogue 1.2.0: new EXE w/ rogue RT_RCDATA #7777 + new trusted_publisher.pub + valid signature under rogue privkey!
        create_signed_test_release(
            &rel_rogue_120,
            "closeout-game",
            "Auto_Closeout.exe",
            "1.2.0",
            CURRENT_SQLITE_SCHEMA_VERSION,
            1,
            rogue_seed,
        );

        let install_root = tmp.join("installed/CloseoutTool");
        let updater = MiniToolUpdater::new(&install_root);
        let opts = UpdateExecutionOptions {
            skip_child_verify_exec: true,
            ..Default::default()
        };

        // Step 1: Install trusted v1.0.0 & seed Data A
        let init_out = updater.install_initial(&rel_100, &opts).unwrap();
        assert_eq!(init_out.active_version, "1.0.0");

        // Confirm rogue candidate PASSES standalone verification if unanchored,
        // YET IS BLOCKED strictly when anchored to the previously installed v1.0.0 trusted EXE!
        assert!(MiniToolUpdater::verify_candidate_release_dir(
            &rel_rogue_120,
            &RuntimeCompatibilityProfile::default()
        )
        .is_ok());

        let err_rogue = updater
            .upgrade_from_release(&rel_rogue_120, &opts)
            .unwrap_err()
            .to_string();
        assert!(
            err_rogue.contains("UPDATE_ROOT_OF_TRUST_UNTRUSTED"),
            "Expected root of trust rejection, got: {}",
            err_rogue
        );
        assert_eq!(
            updater.read_active_pointer().unwrap().unwrap().active_version,
            "1.0.0"
        );

        // Seed Data A on v1.0.0
        let live_db = updater.data_dir().join("runtime_closeout-game.db");
        {
            let st = StorageManager::new(&live_db).unwrap();
            let now = Utc::now().to_rfc3339();
            let conn = Connection::open(&live_db).unwrap();
            conn.execute(
                "INSERT INTO projects (id, slug, name, game_package_id, version, exe_name, window_title, project_dir_path, created_at, updated_at)
                 VALUES ('proj-co', 'closeout-game', 'Closeout', 'com.co', '1.0.0', 'Auto_Closeout.exe', 'Closeout v1.0', 'projects/co', ?1, ?1)",
                params![now],
            ).unwrap();
            conn.execute(
                "INSERT INTO accounts (id, project_id, alias, game_profile_id, secret_ref, state, updated_at)
                 VALUES ('acc-data-A', 'proj-co', 'data_A_v1', 'closeout-game', 'CRED_MGR://co/A', 'Ready', ?1)",
                params![now],
            ).unwrap();
            drop(st);
        }

        // Step 2: Upgrade v1.0.0 -> v1.1.0 with trusted publisher key
        let up_110 = updater.upgrade_from_release(&rel_110, &opts).unwrap();
        assert_eq!(up_110.active_version, "1.1.0");

        // Step 3: Write NEW Data B & runtime events WHILE RUNNING ON v1.1.0!
        {
            let st = StorageManager::new(&live_db).unwrap();
            let now = Utc::now().to_rfc3339();
            let conn = Connection::open(&live_db).unwrap();
            conn.execute(
                "INSERT INTO accounts (id, project_id, alias, game_profile_id, secret_ref, state, updated_at)
                 VALUES ('acc-data-B', 'proj-co', 'data_B_v11', 'closeout-game', 'CRED_MGR://co/B', 'Ready', ?1)",
                params![now],
            ).unwrap();
            st.record_runtime_var_deltas(
                "mini:closeout-game",
                "proj-co",
                "sess-v11",
                "job-v11",
                "wf@1.1",
                vec![fp_domain::RuntimeVariableEnvelope {
                    project_id: "proj-co".to_string(),
                    session_id: "sess-v11".to_string(),
                    job_id: "job-v11".to_string(),
                    name: "post_upgrade_loot".to_string(),
                    value: serde_json::json!(88888),
                    value_type: "number".to_string(),
                    source: fp_domain::VariableSource::TestkitMock,
                    verified: false,
                    confidence: None,
                    confidence_source: None,
                    measured_confidence: None,
                    rule_matched: None,
                    device_id: None,
                    updated_at_utc: now,
                    workflow_version: "wf@1.1".to_string(),
                    seq: 0,
                    redacted: false,
                }],
            )
            .unwrap();
        }

        // Step 4: Binary Rollback (restore_pre_update_backup = false) -> reverts EXE to v1.0.0 AND keeps BOTH Data A & Data B!
        let rb_bin = updater.rollback_to_previous(false, true).unwrap();
        assert_eq!(rb_bin.active_version, "1.0.0");
        assert!(rb_bin.binary_rollback_only);
        assert!(!rb_bin.database_restored);
        assert!(rb_bin.pre_rollback_safety_db_path.is_none());
        {
            let conn = Connection::open(&live_db).unwrap();
            let cnt_a: i64 = conn
                .query_row("SELECT COUNT(*) FROM accounts WHERE id = 'acc-data-A'", [], |r| r.get(0))
                .unwrap();
            let cnt_b: i64 = conn
                .query_row("SELECT COUNT(*) FROM accounts WHERE id = 'acc-data-B'", [], |r| r.get(0))
                .unwrap();
            assert_eq!(cnt_a, 1);
            assert_eq!(cnt_b, 1, "Data B created on v1.1.0 must survive binary rollback!");

            let st = StorageManager::new(&live_db).unwrap();
            let snap = st.load_runtime_snapshot_with_recovery("mini:closeout-game", None).unwrap();
            assert_eq!(
                snap.testkit_envelopes.get("post_upgrade_loot").map(|v| &v.value),
                Some(&serde_json::json!(88888))
            );
        }

        // Step 5: Re-upgrade v1.0.0 -> v1.1.0, create Data C, simulate incompatible schema v99,
        // verify binary rollback is BLOCKED, while explicit DB restore saves Data C in `pre_rollback_safety_db_path`!
        let _ = updater.upgrade_from_release(&rel_110, &opts).unwrap();
        {
            let conn = Connection::open(&live_db).unwrap();
            let now = Utc::now().to_rfc3339();
            conn.execute(
                "INSERT INTO accounts (id, project_id, alias, game_profile_id, secret_ref, state, updated_at)
                 VALUES ('acc-data-C', 'proj-co', 'data_C_v11', 'closeout-game', 'CRED_MGR://co/C', 'Ready', ?1)",
                params![now],
            ).unwrap();
            conn.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (99, '2026-11-22T00:00:00Z')",
                [],
            ).unwrap();
        }
        let err_incompat_rb = updater
            .rollback_to_previous(false, true)
            .unwrap_err()
            .to_string();
        assert!(err_incompat_rb.contains("SQLITE_DOWNGRADE_INCOMPATIBLE"));

        let rb_db = updater.rollback_to_previous(true, true).unwrap();
        assert!(rb_db.database_restored);
        assert!(!rb_db.binary_rollback_only);
        let safety_path = rb_db.pre_rollback_safety_db_path.unwrap();
        assert!(Path::new(&safety_path).exists());
        {
            let safety_conn = Connection::open(&safety_path).unwrap();
            let cnt_c: i64 = safety_conn
                .query_row("SELECT COUNT(*) FROM accounts WHERE id = 'acc-data-C'", [], |r| r.get(0))
                .unwrap();
            assert_eq!(cnt_c, 1, "Newer Data C preserved inside safety snapshot before DB restore");
        }

        // Step 6: Path Traversal, data/ Overwrite Prevention & NTFS Junction Guard
        // 6a. Manifest workflow_files traversal
        let rel_trav = tmp.join("rel_trav");
        copy_dir_recursive(&rel_110, &rel_trav).unwrap();
        let mut trav_manifest: ReleaseBundleManifest =
            serde_json::from_slice(&fs::read(rel_trav.join("bundle/manifest.fpbundle.json")).unwrap()).unwrap();
        trav_manifest.project.workflow_files.push("../data/evil.json".to_string());
        fs::write(
            rel_trav.join("bundle/manifest.fpbundle.json"),
            serde_json::to_vec_pretty(&trav_manifest).unwrap(),
        ).unwrap();
        let err_trav = MiniToolUpdater::verify_candidate_release_dir_with_anchor(
            &rel_trav,
            &RuntimeCompatibilityProfile::default(),
            None,
        ).unwrap_err().to_string();
        assert!(err_trav.contains("PATH_SECURITY_ERROR"), "got: {}", err_trav);

        // 6b. Forbidden top-level `data/` inside release archive
        let rel_evil_data = tmp.join("rel_evil_data");
        copy_dir_recursive(&rel_110, &rel_evil_data).unwrap();
        fs::create_dir_all(rel_evil_data.join("data")).unwrap();
        fs::write(rel_evil_data.join("data/runtime_closeout-game.db"), b"overwrite").unwrap();
        let err_data_ov = MiniToolUpdater::verify_candidate_release_dir_with_anchor(
            &rel_evil_data,
            &RuntimeCompatibilityProfile::default(),
            None,
        ).unwrap_err().to_string();
        assert!(err_data_ov.contains("RELEASE_FORBIDS_USER_DATA_OVERWRITE"));

        // 6c. NTFS Junction inside candidate directory
        #[cfg(windows)]
        {
            let rel_junc = tmp.join("rel_junc");
            copy_dir_recursive(&rel_110, &rel_junc).unwrap();
            let outside = tmp.join("outside_secret_target");
            fs::create_dir_all(&outside).unwrap();
            let junc_link = rel_junc.join("bundle").join("linked_outside");
            let mk_status = std::process::Command::new("cmd")
                .args(["/C", "mklink", "/J", junc_link.to_str().unwrap(), outside.to_str().unwrap()])
                .output()
                .unwrap();
            if mk_status.status.success() {
                let err_j = MiniToolUpdater::verify_candidate_release_dir_with_anchor(
                    &rel_junc,
                    &RuntimeCompatibilityProfile::default(),
                    None,
                ).unwrap_err().to_string();
                assert!(err_j.contains("REPARSE_POINT_OR_JUNCTION_FORBIDDEN"), "got: {}", err_j);
            }
        }

        // Step 7: Active Pointer File Lock + Corruption + Deletion Recovery
        // 7a. Lock active_version.json exclusively using Win32 -> atomic write fails cleanly without corrupting pointer
        {
            use std::os::windows::fs::OpenOptionsExt;
            let locked_handle = OpenOptions::new()
                .read(true)
                .write(true)
                .share_mode(0)
                .open(updater.active_pointer_path())
                .unwrap();
            let err_lock = atomic_write_file(&updater.active_pointer_path(), b"should_not_overwrite")
                .unwrap_err()
                .to_string();
            assert!(err_lock.contains("ACTIVE_POINTER_LOCKED"), "got: {}", err_lock);
            drop(locked_handle);
        }

        // 7b. Corrupt active_version.json -> reconcile recovers from shadow backup
        fs::write(updater.active_pointer_path(), b"{CORRUPTED_TRUNCATED_POINTER").unwrap();
        let rec_corrupt = updater.reconcile_incomplete_journal(true).unwrap();
        assert_eq!(rec_corrupt.status, "RECOVERED_POINTER");
        assert_eq!(updater.read_active_pointer().unwrap().unwrap().active_version, "1.0.0");

        // 7c. Delete BOTH active_version.json AND shadow backup -> reconcile scans versions/v* & recovers highest valid signed release (1.1.0)
        fs::remove_file(updater.active_pointer_path()).unwrap();
        fs::remove_file(updater.backup_pointer_path()).unwrap();
        let rec_scan = updater.reconcile_incomplete_journal(true).unwrap();
        assert_eq!(rec_scan.status, "RECOVERED_POINTER");
        assert_eq!(updater.read_active_pointer().unwrap().unwrap().active_version, "1.1.0");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_p5_3_authenticode_tamper_key_rotation_replay_and_trusted_pointer_recovery() {
        use fp_builder::authenticode::{
            create_self_signed_qa_cert, find_signtool_exe, sign_pe_authenticode_and_verify,
            AuthenticodeSignRequest,
        };
        use fp_builder::security::ProducerKeyManager;
        use fp_bundle_contract::{
            KeyRotationDelegation, PeVersionInfoSpec, PublisherIdentitySpec, ReleaseSigningMetadata,
        };

        let tmp = std::env::temp_dir().join(format!("fp_updater_p53_{}", Uuid::new_v4()));
        let rel_100 = tmp.join("rel_100");
        let rel_110_rotated = tmp.join("rel_110_rot");
        let rel_120_replayed = tmp.join("rel_120_replay");
        let rel_130_rogue_del = tmp.join("rel_130_rogue");

        let key_old = SigningKey::from_bytes(&[21u8; 32]);
        let key_new = SigningKey::from_bytes(&[33u8; 32]);
        let key_rogue = SigningKey::from_bytes(&[99u8; 32]);
        let pub_old_hex: String = key_old.verifying_key().to_bytes().iter().map(|b| format!("{:02x}", b)).collect();
        let pub_new_hex: String = key_new.verifying_key().to_bytes().iter().map(|b| format!("{:02x}", b)).collect();
        let pub_rogue_hex: String = key_rogue.verifying_key().to_bytes().iter().map(|b| format!("{:02x}", b)).collect();

        let pub_spec = PublisherIdentitySpec {
            publisher_id: "pub-qa-studio".to_string(),
            display_name: "QA Studio".to_string(),
            organization: "FarmerPro QA Labs".to_string(),
            website: None,
            support_contact: None,
            copyright: "Copyright (C) 2026 FarmerPro QA Labs".to_string(),
            ed25519_key_id: "key-qa-old".to_string(),
            ed25519_pubkey_hex: pub_old_hex.clone(),
            authenticode_cert_ref: None,
            release_channel: "qa".to_string(),
            signing_policy: SigningMode::UnsignedInternal,
        };

        // Helper to stamp publisher + signing metadata + re-sign bundle
        let attach_signing = |dir: &Path, sk: &SigningKey, mode: SigningMode, pub_id: &str, rot: Option<KeyRotationDelegation>| {
            let bdir = dir.join("bundle");
            let mut m: ReleaseBundleManifest =
                serde_json::from_slice(&fs::read(bdir.join("manifest.fpbundle.json")).unwrap()).unwrap();
            let mut p = pub_spec.clone();
            p.publisher_id = pub_id.to_string();
            p.signing_policy = mode;
            m.publisher = Some(p);
            m.signing = Some(ReleaseSigningMetadata {
                signing_mode: mode,
                publisher_id: pub_id.to_string(),
                publisher_organization: "FarmerPro QA Labs".to_string(),
                release_channel: "qa".to_string(),
                authenticode_trust_status: AuthenticodeTrustStatus::Unsigned,
                trusted_public_release: false,
                signer_thumbprint: None,
                signer_subject: None,
                version_info: PeVersionInfoSpec::default(),
                key_rotation: rot,
            });
            let mb = serde_json::to_vec_pretty(&m).unwrap();
            fs::write(bdir.join("manifest.fpbundle.json"), &mb).unwrap();
            let mut sig_env: SignedBundleEnvelope =
                serde_json::from_slice(&fs::read(bdir.join("signature.fpbundle.json")).unwrap()).unwrap();
            sig_env.file_checksums.insert("manifest.fpbundle.json".to_string(), sha256_hex(&mb));
            let msha = sha256_hex(&mb);
            let tjson = serde_json::to_string(&sig_env.file_checksums).unwrap();
            let thash = sha256_hex(tjson.as_bytes());
            let msg = fp_bundle_verify::canonical_signing_message(&sig_env.key_id, &msha, &thash);
            let s = sk.sign(&msg);
            sig_env.manifest_sha256 = msha;
            sig_env.checksums_table_sha256 = thash;
            sig_env.signature_ed25519_hex = s.to_bytes().iter().map(|b| format!("{:02x}", b)).collect();
            fs::write(bdir.join("signature.fpbundle.json"), serde_json::to_vec_pretty(&sig_env).unwrap()).unwrap();
        };

        create_signed_test_release(&rel_100, "p53-tool", "Auto_P53.exe", "1.0.0", CURRENT_SQLITE_SCHEMA_VERSION, 1, [21u8; 32]);
        attach_signing(&rel_100, &key_old, SigningMode::UnsignedInternal, "pub-qa-studio", None);

        let delegation = ProducerKeyManager::sign_key_rotation_delegation(
            &key_old,
            &pub_new_hex,
            "pub-qa-studio",
            "p53-tool",
            30,
        );

        create_signed_test_release(&rel_110_rotated, "p53-tool", "Auto_P53.exe", "1.1.0", CURRENT_SQLITE_SCHEMA_VERSION, 1, [33u8; 32]);
        attach_signing(&rel_110_rotated, &key_new, SigningMode::UnsignedInternal, "pub-qa-studio", Some(delegation.clone()));

        // Authenticode signing + post-sign PE tampering test (if signtool.exe present)
        let mut test_cert_thumb: Option<String> = None;
        if find_signtool_exe().is_some() {
            if let Ok(qa_cert) = create_self_signed_qa_cert("CN=FarmerPro P5.3 Test QA") {
                test_cert_thumb = Some(qa_cert.thumbprint.clone());
                let exe_100 = rel_100.join("Auto_P53.exe");
                let auth_rep = sign_pe_authenticode_and_verify(
                    &exe_100,
                    &AuthenticodeSignRequest {
                        signing_mode: SigningMode::SelfSignedQa,
                        cert_ref: Some(qa_cert.thumbprint.clone()),
                        description: Some("FarmerPro P5.3 Test Runner".to_string()),
                        timestamp_url: None,
                    },
                )
                .unwrap();
                assert_eq!(auth_rep.trust_status, AuthenticodeTrustStatus::SelfSignedUntrustedRoot);
                assert!(!auth_rep.trusted_public_release);

                // Tamper PE after Authenticode signing -> MUST fail even when bundle JSON is untouched!
                let rel_tampered_pe = tmp.join("rel_tampered_pe");
                copy_dir_recursive(&rel_100, &rel_tampered_pe).unwrap();
                let mut pe_bytes = fs::read(rel_tampered_pe.join("Auto_P53.exe")).unwrap();
                let len = pe_bytes.len();
                pe_bytes[len / 2] ^= 0x5A;
                fs::write(rel_tampered_pe.join("Auto_P53.exe"), &pe_bytes).unwrap();
                let err_auth = MiniToolUpdater::verify_candidate_release_dir_with_anchor(
                    &rel_tampered_pe,
                    &RuntimeCompatibilityProfile::default(),
                    None,
                ).unwrap_err().to_string();
                assert!(err_auth.contains("AUTHENTICODE_TAMPERED_OR_INVALID"), "got: {}", err_auth);
            }
        }

        let install_root = tmp.join("installed/P53Tool");
        let updater = MiniToolUpdater::new(&install_root);
        let opts = UpdateExecutionOptions { skip_child_verify_exec: true, ..Default::default() };

        // 1. Install v1.0.0 with Key Old
        updater.install_initial(&rel_100, &opts).unwrap();

        // 2. Rogue delegation (signed by rogue key, claiming to rotate old -> rogue) MUST be rejected
        let rogue_del = KeyRotationDelegation {
            publisher_id: "pub-qa-studio".to_string(),
            product_slug: "p53-tool".to_string(),
            old_pubkey_hex: pub_old_hex.clone(),
            new_pubkey_hex: pub_rogue_hex.clone(),
            issued_at_utc: delegation.issued_at_utc.clone(),
            expires_at_utc: delegation.expires_at_utc.clone(),
            nonce: "rogue-nonce-12345678".to_string(),
            signature_by_old_key_hex: ProducerKeyManager::sign_key_rotation_delegation(&key_rogue, &pub_rogue_hex, "pub-qa-studio", "p53-tool", 10).signature_by_old_key_hex,
        };
        create_signed_test_release(&rel_130_rogue_del, "p53-tool", "Auto_P53.exe", "1.3.0", CURRENT_SQLITE_SCHEMA_VERSION, 1, [99u8; 32]);
        attach_signing(&rel_130_rogue_del, &key_rogue, SigningMode::UnsignedInternal, "pub-qa-studio", Some(rogue_del));
        let err_rdel = updater.upgrade_from_release(&rel_130_rogue_del, &opts).unwrap_err().to_string();
        assert!(err_rdel.contains("KEY_ROTATION_DELEGATION_INVALID"), "got: {}", err_rdel);

        // 3. Valid KeyRotationDelegation v1.0.0 -> v1.1.0 SUCCEEDS!
        let up = updater.upgrade_from_release(&rel_110_rotated, &opts).unwrap();
        assert_eq!(up.active_version, "1.1.0");
        let ts = updater.read_trust_state().unwrap().unwrap();
        assert!(ts.used_rotation_nonces.contains(&delegation.nonce));
        assert!(ts.trusted_pubkeys.iter().any(|k| k.eq_ignore_ascii_case(&pub_new_hex)));

        // 4. Replay / Rollback check & Trust-anchored pointer recovery (drop untrusted v9.9.9 into versions/)
        let rogue_v999 = updater.versions_dir().join("v9.9.9");
        create_signed_test_release(&rogue_v999, "p53-tool", "Auto_P53.exe", "9.9.9", CURRENT_SQLITE_SCHEMA_VERSION, 1, [99u8; 32]);
        fs::remove_file(updater.active_pointer_path()).unwrap();
        fs::remove_file(updater.backup_pointer_path()).unwrap();
        let rec = updater.reconcile_incomplete_journal(true).unwrap();
        assert_eq!(rec.status, "RECOVERED_POINTER");
        assert_eq!(updater.read_active_pointer().unwrap().unwrap().active_version, "1.1.0");

        // Also test nonce replay rejection via another fresh install anchored on key_old with pre-recorded nonce
        let rel_replay_2 = rel_120_replayed;
        create_signed_test_release(&rel_replay_2, "p53-tool", "Auto_P53.exe", "1.2.0", CURRENT_SQLITE_SCHEMA_VERSION, 1, [33u8; 32]);
        attach_signing(&rel_replay_2, &key_new, SigningMode::UnsignedInternal, "pub-qa-studio", Some(delegation.clone()));
        let install_replay = tmp.join("installed/P53Replay");
        let up_rep = MiniToolUpdater::new(&install_replay);
        up_rep.install_initial(&rel_100, &opts).unwrap();
        let mut seeded_ts = up_rep.read_trust_state().unwrap().unwrap();
        seeded_ts.used_rotation_nonces.push(delegation.nonce.clone());
        up_rep.write_trust_state(&seeded_ts).unwrap();
        let err_replay = up_rep.upgrade_from_release(&rel_replay_2, &opts).unwrap_err().to_string();
        assert!(err_replay.contains("KEY_ROTATION_NONCE_REPLAYED"), "got: {}", err_replay);

        if let Some(thumb) = test_cert_thumb {
            let _ = std::process::Command::new("powershell")
                .args(["-NoProfile", "-Command", &format!("Remove-Item 'Cert:\\CurrentUser\\My\\{}' -ErrorAction SilentlyContinue", thumb)])
                .output();
        }
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_p5_4_trust_state_anti_rollback_revocation_and_key_rotation_crash_recovery() {
        use fp_builder::security::ProducerKeyManager;
        use fp_bundle_contract::{
            KeyRotationDelegation, PeVersionInfoSpec, PublisherIdentitySpec, ReleaseSigningMetadata,
        };

        let tmp = std::env::temp_dir().join(format!("fp_updater_p54_trust_{}", Uuid::new_v4()));
        let rel_100 = tmp.join("rel_100");
        let rel_110 = tmp.join("rel_110");
        let key_old = SigningKey::from_bytes(&[41u8; 32]);
        let key_new = SigningKey::from_bytes(&[52u8; 32]);
        let pub_old: String = key_old.verifying_key().to_bytes().iter().map(|b| format!("{:02x}", b)).collect();
        let pub_new: String = key_new.verifying_key().to_bytes().iter().map(|b| format!("{:02x}", b)).collect();

        let attach = |dir: &Path, sk: &SigningKey, pk_hex: &str, rot: Option<KeyRotationDelegation>| {
            let bdir = dir.join("bundle");
            let mut m: ReleaseBundleManifest =
                serde_json::from_slice(&fs::read(bdir.join("manifest.fpbundle.json")).unwrap()).unwrap();
            m.publisher = Some(PublisherIdentitySpec {
                publisher_id: "pub-p54-test".to_string(),
                display_name: "P54 Test Publisher".to_string(),
                organization: "P54 Labs".to_string(),
                website: None,
                support_contact: None,
                copyright: "Copyright (C) 2026 P54 Labs".to_string(),
                ed25519_key_id: "k-p54".to_string(),
                ed25519_pubkey_hex: pk_hex.to_string(),
                authenticode_cert_ref: None,
                release_channel: "qa".to_string(),
                signing_policy: SigningMode::UnsignedInternal,
            });
            m.signing = Some(ReleaseSigningMetadata {
                signing_mode: SigningMode::UnsignedInternal,
                publisher_id: "pub-p54-test".to_string(),
                publisher_organization: "P54 Labs".to_string(),
                release_channel: "qa".to_string(),
                authenticode_trust_status: AuthenticodeTrustStatus::Unsigned,
                trusted_public_release: false,
                signer_thumbprint: None,
                signer_subject: None,
                version_info: PeVersionInfoSpec::default(),
                key_rotation: rot,
            });
            let mb = serde_json::to_vec_pretty(&m).unwrap();
            fs::write(bdir.join("manifest.fpbundle.json"), &mb).unwrap();
            let mut sig_env: SignedBundleEnvelope =
                serde_json::from_slice(&fs::read(bdir.join("signature.fpbundle.json")).unwrap()).unwrap();
            sig_env.file_checksums.insert("manifest.fpbundle.json".to_string(), sha256_hex(&mb));
            let msha = sha256_hex(&mb);
            let tjson = serde_json::to_string(&sig_env.file_checksums).unwrap();
            let thash = sha256_hex(tjson.as_bytes());
            let msg = fp_bundle_verify::canonical_signing_message(&sig_env.key_id, &msha, &thash);
            let s = sk.sign(&msg);
            sig_env.manifest_sha256 = msha;
            sig_env.checksums_table_sha256 = thash;
            sig_env.signature_ed25519_hex = s.to_bytes().iter().map(|b| format!("{:02x}", b)).collect();
            fs::write(bdir.join("signature.fpbundle.json"), serde_json::to_vec_pretty(&sig_env).unwrap()).unwrap();
        };

        create_signed_test_release(&rel_100, "p54-trust", "Auto_P54Trust.exe", "1.0.0", CURRENT_SQLITE_SCHEMA_VERSION, 1, [41u8; 32]);
        attach(&rel_100, &key_old, &pub_old, None);

        let delegation = ProducerKeyManager::sign_key_rotation_delegation(&key_old, &pub_new, "pub-p54-test", "p54-trust", 30);
        create_signed_test_release(&rel_110, "p54-trust", "Auto_P54Trust.exe", "1.1.0", CURRENT_SQLITE_SCHEMA_VERSION, 1, [52u8; 32]);
        attach(&rel_110, &key_new, &pub_new, Some(delegation.clone()));

        let install_root = tmp.join("installed/TrustTool");
        let updater = MiniToolUpdater::new(&install_root);
        let opts = UpdateExecutionOptions { skip_child_verify_exec: true, ..Default::default() };

        // 1. Install v1.0.0 (epoch=1) and snapshot old trust-state.json before rotation
        updater.install_initial(&rel_100, &opts).unwrap();
        let old_trust_json = fs::read(updater.trust_state_path()).unwrap();
        let old_ts = updater.read_trust_state().unwrap().unwrap();
        assert_eq!(old_ts.trust_epoch, 1);

        // 2. Upgrade v1.0.0 -> v1.1.0 with KeyRotationDelegation (bumps epoch -> 2)
        let up = updater.upgrade_from_release(&rel_110, &opts).unwrap();
        assert_eq!(up.active_version, "1.1.0");
        let ts_after_rot = updater.read_trust_state().unwrap().unwrap();
        assert_eq!(ts_after_rot.trust_epoch, 2);
        assert!(ts_after_rot.used_rotation_nonces.contains(&delegation.nonce));

        // 3. Rollback test 1: Restore OLD trust-state.json (epoch 1) & backup -> MUST BE REJECTED by SQLite trust-ledger.db!
        fs::write(updater.trust_state_path(), &old_trust_json).unwrap();
        let _ = fs::write(updater.trust_state_backup_path(), &old_trust_json);
        let rollback_err = updater.read_trust_state().unwrap_err().to_string();
        assert!(rollback_err.contains("TRUST_STATE_ROLLBACK_DETECTED"), "got: {}", rollback_err);

        // Restore valid epoch=2 trust-state.json so we can test rollback BEFORE & AFTER revoking old key
        updater.sync_trust_ledger_db(&ts_after_rot, "restore_test", "restore").ok();
        let bytes2 = serde_json::to_vec_pretty(&ts_after_rot).unwrap();
        fs::write(updater.trust_state_path(), &bytes2).unwrap();
        fs::write(updater.trust_state_backup_path(), &bytes2).unwrap();

        // 4. Rollback binary cleanly v1.1.0 -> v1.0.0 when old key is NOT revoked -> SUCCEEDS, then re-upgrade v1.1.0 -> revoke old key -> rollback BLOCKED!
        let install_rev = tmp.join("installed/RevokeTool");
        let u2 = MiniToolUpdater::new(&install_rev);
        u2.install_initial(&rel_100, &opts).unwrap();
        u2.upgrade_from_release(&rel_110, &opts).unwrap();
        let ts_rev = u2.revoke_pubkey(&pub_old, "compromised_old_key").unwrap();
        assert_eq!(ts_rev.trust_epoch, 3);
        let rev_rb_err = u2.rollback_to_previous(false, true).unwrap_err().to_string();
        assert!(rev_rb_err.contains("ROLLBACK_ROOT_OF_TRUST_UNTRUSTED"), "got: {}", rev_rb_err);

        // Attempt to replace trust-state.json to bring revoked old key back -> BLOCKED by trust-ledger.db!
        let mut forged_ts = ts_rev.clone();
        forged_ts.revoked_pubkeys.clear();
        forged_ts.trusted_pubkeys.push(pub_old.clone());
        fs::write(u2.trust_state_path(), serde_json::to_vec_pretty(&forged_ts).unwrap()).unwrap();
        let forged_err = u2.read_trust_state().unwrap_err().to_string();
        assert!(forged_err.contains("TRUST_STATE_ROLLBACK_DETECTED"), "got: {}", forged_err);

        // 5. Simulate interrupted crash at Activated stage during KeyRotationDelegation -> reconcile synchronizes Trust State <-> Active Release <-> Nonce Ledger!
        let install_crash = tmp.join("installed/CrashRotTool");
        let u3 = MiniToolUpdater::new(&install_crash);
        u3.install_initial(&rel_100, &opts).unwrap();
        let pre_ptr = u3.read_active_pointer().unwrap().unwrap();
        copy_dir_recursive(&rel_110, &u3.versions_dir().join("v1.1.0")).unwrap();
        let mut crashed_ptr = pre_ptr.clone();
        crashed_ptr.active_version = "1.1.0".to_string();
        crashed_ptr.previous_version = Some("1.0.0".to_string());
        crashed_ptr.trusted_pubkey_hex = Some(pub_new.clone());
        u3.write_active_pointer_with_shadow(&crashed_ptr).unwrap();
        let mut j = UpdateJournal {
            journal_id: "upd-crash-rot".to_string(),
            operation: "upgrade".to_string(),
            project_slug: "p54-trust".to_string(),
            source_version: Some("1.0.0".to_string()),
            target_version: "1.1.0".to_string(),
            source_release_dir: rel_110.to_string_lossy().to_string(),
            stage: UpdateStage::Activated,
            backup_db_path: None,
            live_db_path: u3.data_dir().join("runtime_p54-trust.db").to_string_lossy().to_string(),
            staged_staging_dir: None,
            staged_version_dir: u3.versions_dir().join("v1.1.0").to_string_lossy().to_string(),
            previous_active_pointer: Some(pre_ptr),
            target_compat: CompatibilityVersions::default(),
            auto_resume_blocked_jobs: vec![],
            last_error: None,
            updated_at_utc: Utc::now().to_rfc3339(),
        };
        u3.write_journal(&mut j).unwrap();
        let rec = u3.reconcile_incomplete_journal(true).unwrap();
        assert_eq!(rec.status, "RECOVERED_ROLL_FORWARD");
        let rec_ts = u3.read_trust_state().unwrap().unwrap();
        assert!(rec_ts.used_rotation_nonces.contains(&delegation.nonce));
        assert!(rec_ts.trusted_pubkeys.iter().any(|k| k.eq_ignore_ascii_case(&pub_new)));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_p6_full_triad_trust_anchor_revocation_rogue_dir_and_manifest_attacks() {
        use fp_builder::security::ProducerKeyManager;
        use fp_bundle_contract::{
            DistributedFileEntry, KeyRotationDelegation, PeVersionInfoSpec,
            PortableRuntimeRequirements, PublisherIdentitySpec, ReleasePackageManifest,
            ReleaseSigningMetadata,
        };

        let tmp = std::env::temp_dir().join(format!("fp_updater_p6_sec_{}", Uuid::new_v4()));
        let rel_100 = tmp.join("rel_100");
        let rel_110 = tmp.join("rel_110");
        let rel_rogue = tmp.join("rel_rogue");
        let anchor_dir = tmp.join("external_trust_anchors");

        let key_old = SigningKey::from_bytes(&[61u8; 32]);
        let key_new = SigningKey::from_bytes(&[72u8; 32]);
        let key_rogue = SigningKey::from_bytes(&[99u8; 32]);
        let pub_old: String = key_old.verifying_key().to_bytes().iter().map(|b| format!("{:02x}", b)).collect();
        let pub_new: String = key_new.verifying_key().to_bytes().iter().map(|b| format!("{:02x}", b)).collect();
        let pub_rogue: String = key_rogue.verifying_key().to_bytes().iter().map(|b| format!("{:02x}", b)).collect();

        let attach_with_manifest = |dir: &Path, sk: &SigningKey, pk_hex: &str, slug: &str, ver: &str, exe_name: &str, rot: Option<KeyRotationDelegation>| {
            let bdir = dir.join("bundle");
            let mut m: ReleaseBundleManifest =
                serde_json::from_slice(&fs::read(bdir.join("manifest.fpbundle.json")).unwrap()).unwrap();
            m.publisher = Some(PublisherIdentitySpec {
                publisher_id: "pub-p6-sec".to_string(),
                display_name: "P6 Sec Publisher".to_string(),
                organization: "P6 Sec Labs".to_string(),
                website: None,
                support_contact: None,
                copyright: "Copyright (C) 2026 P6 Sec Labs".to_string(),
                ed25519_key_id: "k-p6".to_string(),
                ed25519_pubkey_hex: pk_hex.to_string(),
                authenticode_cert_ref: None,
                release_channel: "qa".to_string(),
                signing_policy: SigningMode::UnsignedInternal,
            });
            m.signing = Some(ReleaseSigningMetadata {
                signing_mode: SigningMode::UnsignedInternal,
                publisher_id: "pub-p6-sec".to_string(),
                publisher_organization: "P6 Sec Labs".to_string(),
                release_channel: "qa".to_string(),
                authenticode_trust_status: AuthenticodeTrustStatus::Unsigned,
                trusted_public_release: false,
                signer_thumbprint: None,
                signer_subject: None,
                version_info: PeVersionInfoSpec::default(),
                key_rotation: rot,
            });
            let mb = serde_json::to_vec_pretty(&m).unwrap();
            fs::write(bdir.join("manifest.fpbundle.json"), &mb).unwrap();
            let mut sig_env: SignedBundleEnvelope =
                serde_json::from_slice(&fs::read(bdir.join("signature.fpbundle.json")).unwrap()).unwrap();
            sig_env.file_checksums.insert("manifest.fpbundle.json".to_string(), sha256_hex(&mb));
            let msha = sha256_hex(&mb);
            let tjson = serde_json::to_string(&sig_env.file_checksums).unwrap();
            let thash = sha256_hex(tjson.as_bytes());
            let msg = fp_bundle_verify::canonical_signing_message(&sig_env.key_id, &msha, &thash);
            let s = sk.sign(&msg);
            sig_env.manifest_sha256 = msha;
            sig_env.checksums_table_sha256 = thash;
            sig_env.signature_ed25519_hex = s.to_bytes().iter().map(|b| format!("{:02x}", b)).collect();
            fs::write(bdir.join("signature.fpbundle.json"), serde_json::to_vec_pretty(&sig_env).unwrap()).unwrap();

            fs::write(dir.join("THIRD_PARTY_NOTICES.txt"), b"Rust/Tauri/SQLite notices").unwrap();
            let exe_bytes = fs::read(dir.join(exe_name)).unwrap();
            let notices_bytes = fs::read(dir.join("THIRD_PARTY_NOTICES.txt")).unwrap();
            let rm = ReleasePackageManifest {
                package_schema_version: 1,
                package_type: "mini_tool_portable_win_x64".to_string(),
                project_slug: slug.to_string(),
                product_name: "P6 Security Tool".to_string(),
                version: ver.to_string(),
                exe_name: exe_name.to_string(),
                final_exe_sha256: sha256_hex(&exe_bytes),
                built_at_utc: Utc::now().to_rfc3339(),
                requirements: PortableRuntimeRequirements {
                    requires_rust: false,
                    requires_cargo: false,
                    requires_node: false,
                    requires_python: false,
                    requires_windows_sdk: false,
                    webview2_strategy: "Evergreen".to_string(),
                    offline_ready: true,
                    requires_internet_if_webview2_missing: true,
                    data_isolation_policy: "External data/".to_string(),
                },
                publisher: m.publisher.clone(),
                signing: m.signing.clone(),
                distributed_files: vec![
                    DistributedFileEntry {
                        rel_path: exe_name.to_string(),
                        size_bytes: exe_bytes.len() as u64,
                        sha256: sha256_hex(&exe_bytes),
                    },
                    DistributedFileEntry {
                        rel_path: "THIRD_PARTY_NOTICES.txt".to_string(),
                        size_bytes: notices_bytes.len() as u64,
                        sha256: sha256_hex(&notices_bytes),
                    },
                ],
                third_party_notices: vec!["Tauri MIT".to_string()],
            };
            fs::write(dir.join("release-manifest.json"), serde_json::to_vec_pretty(&rm).unwrap()).unwrap();
        };

        create_signed_test_release(&rel_100, "p6-sec", "Auto_P6Sec.exe", "1.0.0", CURRENT_SQLITE_SCHEMA_VERSION, 1, [61u8; 32]);
        attach_with_manifest(&rel_100, &key_old, &pub_old, "p6-sec", "1.0.0", "Auto_P6Sec.exe", None);

        let delegation = ProducerKeyManager::sign_key_rotation_delegation(&key_old, &pub_new, "pub-p6-sec", "p6-sec", 30);
        create_signed_test_release(&rel_110, "p6-sec", "Auto_P6Sec.exe", "1.1.0", CURRENT_SQLITE_SCHEMA_VERSION, 1, [72u8; 32]);
        attach_with_manifest(&rel_110, &key_new, &pub_new, "p6-sec", "1.1.0", "Auto_P6Sec.exe", Some(delegation));

        create_signed_test_release(&rel_rogue, "p6-sec", "Auto_P6Sec.exe", "9.9.9", CURRENT_SQLITE_SCHEMA_VERSION, 1, [99u8; 32]);
        attach_with_manifest(&rel_rogue, &key_rogue, &pub_rogue, "p6-sec", "9.9.9", "Auto_P6Sec.exe", None);

        let install_root = tmp.join("installed/P6SecTool");
        let updater = MiniToolUpdater::new_with_external_anchor(&install_root, &anchor_dir);
        let opts = UpdateExecutionOptions { skip_child_verify_exec: true, ..Default::default() };

        // 1. Install v1.0.0 (epoch 1) and snapshot ALL THREE `.updater` files (`trust-state.json`, `.bak.json`, `trust-ledger.db`)
        updater.install_initial(&rel_100, &opts).unwrap();
        let triad_epoch1 = tmp.join("triad_epoch1");
        copy_dir_recursive(&updater.updater_dir(), &triad_epoch1).unwrap();

        // 2. Upgrade v1.0.0 -> v1.1.0 (epoch 2) + Revoke old key (epoch 3)
        updater.upgrade_from_release(&rel_110, &opts).unwrap();
        let rev_st = updater.revoke_pubkey(&pub_old, "p6_key_retired").unwrap();
        assert_eq!(rev_st.trust_epoch, 3);

        // Clean up legacy v1.0.0 first vs test audit catches revoked v1.0.0 still on disk
        let old_ver_err = updater.verify_install_security_audit().unwrap_err().to_string();
        assert!(old_ver_err.contains("KEY_REVOKED"), "got: {}", old_ver_err);

        fs::remove_dir_all(updater.versions_dir().join("v1.0.0")).unwrap();
        let audit_ok = updater.verify_install_security_audit().unwrap();
        assert!(audit_ok.passed);
        assert!(audit_ok.external_anchor_verified);
        assert_eq!(audit_ok.trust_epoch, 3);

        let triad_epoch3 = tmp.join("triad_epoch3");
        copy_dir_recursive(&updater.updater_dir(), &triad_epoch3).unwrap();
        let active_ptr_saved = fs::read(updater.active_pointer_path()).unwrap();

        // 3. Attack A: Full-Triad Replacement (`trust-state.json` + `.bak.json` + `trust-ledger.db`) back to epoch 1!
        let _ = fs::remove_dir_all(updater.updater_dir());
        copy_dir_recursive(&triad_epoch1, &updater.updater_dir()).unwrap();
        let triad_err = updater.read_trust_state().unwrap_err().to_string();
        assert!(triad_err.contains("FULL_TRIAD_ROLLBACK_DETECTED"), "got: {}", triad_err);
        let audit_triad_err = updater.verify_install_security_audit().unwrap_err().to_string();
        assert!(audit_triad_err.contains("FULL_TRIAD_ROLLBACK_DETECTED"), "got: {}", audit_triad_err);

        // 4. Attack B: Total `.updater` & pointer deletion + attempt to reinstall old revoked key release!
        let _ = fs::remove_dir_all(updater.updater_dir());
        let _ = fs::remove_file(updater.active_pointer_path());
        let wipe_err = updater.install_initial(&rel_100, &opts).unwrap_err().to_string();
        assert!(wipe_err.contains("FULL_TRIAD_ROLLBACK_DETECTED"), "got: {}", wipe_err);

        // Restore valid epoch 3 state
        copy_dir_recursive(&triad_epoch3, &updater.updater_dir()).unwrap();
        fs::write(updater.active_pointer_path(), &active_ptr_saved).unwrap();
        assert!(updater.verify_install_security_audit().unwrap().passed);

        // 5. Attack C: Rogue version directory planted inside versions/v9.9.9/
        copy_dir_recursive(&rel_rogue, &updater.versions_dir().join("v9.9.9")).unwrap();
        let rogue_err = updater.verify_install_security_audit().unwrap_err().to_string();
        assert!(rogue_err.contains("ROGUE_VERSION_DIR_DETECTED"), "got: {}", rogue_err);
        fs::remove_dir_all(updater.versions_dir().join("v9.9.9")).unwrap();

        // 6. Attack D: Tamper distributed non-bundle file covered by release-manifest.json
        fs::write(updater.versions_dir().join("v1.1.0").join("THIRD_PARTY_NOTICES.txt"), b"TAMPERED_NOTICE").unwrap();
        let rm_err = updater.verify_install_security_audit().unwrap_err().to_string();
        assert!(rm_err.contains("RELEASE_MANIFEST_TAMPERED"), "got: {}", rm_err);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_p6_reupgrade_after_rollback_vs_replay_and_6comp_same_user_boundary() {
        use fp_builder::security::ProducerKeyManager;
        use fp_bundle_contract::{
            KeyRotationDelegation, PeVersionInfoSpec, PublisherIdentitySpec, ReleaseSigningMetadata,
        };

        let tmp = std::env::temp_dir().join(format!("fp_updater_p6_reupg_{}", Uuid::new_v4()));
        let rel_100 = tmp.join("rel_100");
        let rel_110 = tmp.join("rel_110");
        let rel_120_direct = tmp.join("rel_120_direct");
        let anchor_dir = tmp.join("ext_anchors");

        let key_old = SigningKey::from_bytes(&[81u8; 32]);
        let key_new = SigningKey::from_bytes(&[82u8; 32]);
        let pub_old: String = key_old.verifying_key().to_bytes().iter().map(|b| format!("{:02x}", b)).collect();
        let pub_new: String = key_new.verifying_key().to_bytes().iter().map(|b| format!("{:02x}", b)).collect();

        let attach = |dir: &Path, sk: &SigningKey, pk_hex: &str, rot: Option<KeyRotationDelegation>| {
            let bdir = dir.join("bundle");
            let mut m: ReleaseBundleManifest =
                serde_json::from_slice(&fs::read(bdir.join("manifest.fpbundle.json")).unwrap()).unwrap();
            m.publisher = Some(PublisherIdentitySpec {
                publisher_id: "pub-p6-reupg".to_string(),
                display_name: "P6 Reupgrade Publisher".to_string(),
                organization: "P6 Reupgrade Labs".to_string(),
                website: None,
                support_contact: None,
                copyright: "Copyright (C) 2026 P6 Reupgrade Labs".to_string(),
                ed25519_key_id: "k-p6r".to_string(),
                ed25519_pubkey_hex: pk_hex.to_string(),
                authenticode_cert_ref: None,
                release_channel: "qa".to_string(),
                signing_policy: SigningMode::UnsignedInternal,
            });
            m.signing = Some(ReleaseSigningMetadata {
                signing_mode: SigningMode::UnsignedInternal,
                publisher_id: "pub-p6-reupg".to_string(),
                publisher_organization: "P6 Reupgrade Labs".to_string(),
                release_channel: "qa".to_string(),
                authenticode_trust_status: AuthenticodeTrustStatus::Unsigned,
                trusted_public_release: false,
                signer_thumbprint: None,
                signer_subject: None,
                version_info: PeVersionInfoSpec::default(),
                key_rotation: rot,
            });
            let mb = serde_json::to_vec_pretty(&m).unwrap();
            fs::write(bdir.join("manifest.fpbundle.json"), &mb).unwrap();
            let mut sig_env: SignedBundleEnvelope =
                serde_json::from_slice(&fs::read(bdir.join("signature.fpbundle.json")).unwrap()).unwrap();
            sig_env.file_checksums.insert("manifest.fpbundle.json".to_string(), sha256_hex(&mb));
            let msha = sha256_hex(&mb);
            let tjson = serde_json::to_string(&sig_env.file_checksums).unwrap();
            let thash = sha256_hex(tjson.as_bytes());
            let msg = fp_bundle_verify::canonical_signing_message(&sig_env.key_id, &msha, &thash);
            let s = sk.sign(&msg);
            sig_env.manifest_sha256 = msha;
            sig_env.checksums_table_sha256 = thash;
            sig_env.signature_ed25519_hex = s.to_bytes().iter().map(|b| format!("{:02x}", b)).collect();
            fs::write(bdir.join("signature.fpbundle.json"), serde_json::to_vec_pretty(&sig_env).unwrap()).unwrap();
        };

        create_signed_test_release(&rel_100, "p6-reupg", "Auto_Reupg.exe", "1.0.0", CURRENT_SQLITE_SCHEMA_VERSION, 1, [81u8; 32]);
        attach(&rel_100, &key_old, &pub_old, None);

        let delegation = ProducerKeyManager::sign_key_rotation_delegation(&key_old, &pub_new, "pub-p6-reupg", "p6-reupg", 30);
        create_signed_test_release(&rel_110, "p6-reupg", "Auto_Reupg.exe", "1.1.0", CURRENT_SQLITE_SCHEMA_VERSION, 1, [82u8; 32]);
        attach(&rel_110, &key_new, &pub_new, Some(delegation.clone()));

        create_signed_test_release(&rel_120_direct, "p6-reupg", "Auto_Reupg.exe", "1.2.0", CURRENT_SQLITE_SCHEMA_VERSION, 1, [82u8; 32]);
        attach(&rel_120_direct, &key_new, &pub_new, None);

        let install_root = tmp.join("installed/LegitReupg");
        let updater = MiniToolUpdater::new_with_external_anchor(&install_root, &anchor_dir);
        let opts = UpdateExecutionOptions { skip_child_verify_exec: true, ..Default::default() };

        // 1. Install v1.0.0 -> Upgrade v1.1.0 -> Rollback v1.0.0 -> Legitimate Re-Upgrade v1.1.0 -> Rollback -> Direct v1.2.0 (already-trusted key)
        updater.install_initial(&rel_100, &opts).unwrap();
        updater.upgrade_from_release(&rel_110, &opts).unwrap();
        let rb = updater.rollback_to_previous(false, true).unwrap();
        assert_eq!(rb.active_version, "1.0.0");

        let reupg = updater.upgrade_from_release(&rel_110, &opts).unwrap();
        assert_eq!(reupg.active_version, "1.1.0");
        updater.rollback_to_previous(false, true).unwrap();
        let direct120 = updater.upgrade_from_release(&rel_120_direct, &opts).unwrap();
        assert_eq!(direct120.active_version, "1.2.0");

        // 2. Real Unauthorized Replay Attack on separate install where ONLY OldKey is trusted & nonce is already spent
        let replay_root = tmp.join("installed/ReplayTarget");
        let u_replay = MiniToolUpdater::new_with_external_anchor(&replay_root, &anchor_dir);
        u_replay.install_initial(&rel_100, &opts).unwrap();
        let mut ts_rep = u_replay.read_trust_state().unwrap().unwrap();
        ts_rep.used_rotation_nonces.push(delegation.nonce.clone());
        ts_rep.trust_epoch += 1;
        u_replay.write_trust_state(&ts_rep).unwrap();
        let rep_err = u_replay.upgrade_from_release(&rel_110, &opts).unwrap_err().to_string();
        assert!(rep_err.contains("KEY_ROTATION_NONCE_REPLAYED"), "got: {}", rep_err);

        // 3. Empirical 6-Component Same-User Simultaneous Compromise Demonstration
        // Snapshot epoch 1 across ALL same-user writable artifacts (local .updater + external anchor + pointer + versions/)
        let comp_root = tmp.join("installed/SameUser6Comp");
        let u_comp = MiniToolUpdater::new_with_external_anchor(&comp_root, &anchor_dir);
        u_comp.install_initial(&rel_100, &opts).unwrap();
        let snap_updater = tmp.join("snap6_updater");
        let snap_ptr = fs::read(u_comp.active_pointer_path()).unwrap();
        let snap_ext = fs::read(u_comp.external_anchor_file_path().unwrap()).unwrap();
        copy_dir_recursive(&u_comp.updater_dir(), &snap_updater).unwrap();

        u_comp.upgrade_from_release(&rel_110, &opts).unwrap();
        u_comp.revoke_pubkey(&pub_old, "retired").unwrap();
        assert_eq!(u_comp.read_trust_state().unwrap().unwrap().trust_epoch, 3);

        // Same user overwrites ALL local + external trust stores + removes v1.1.0 -> proves boundary limit transparently!
        let _ = fs::remove_dir_all(u_comp.versions_dir().join("v1.1.0"));
        let _ = fs::remove_dir_all(u_comp.updater_dir());
        copy_dir_recursive(&snap_updater, &u_comp.updater_dir()).unwrap();
        fs::write(u_comp.active_pointer_path(), &snap_ptr).unwrap();
        fs::write(u_comp.external_anchor_file_path().unwrap(), &snap_ext).unwrap();
        let after_6comp = u_comp.verify_install_security_audit().unwrap();
        assert_eq!(after_6comp.trust_epoch, 1);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_p6_closeout_revoked_previously_trusted_key_policy_and_compat_gates() {
        use fp_builder::security::ProducerKeyManager;
        use fp_bundle_contract::{
            KeyRotationDelegation, PeVersionInfoSpec, PublisherIdentitySpec, ReleaseSigningMetadata,
        };

        let tmp = std::env::temp_dir().join(format!("fp_updater_p6_gates_{}", Uuid::new_v4()));
        let anchor_dir = tmp.join("ext_anchors");
        let key_old = SigningKey::from_bytes(&[91u8; 32]);
        let key_new = SigningKey::from_bytes(&[92u8; 32]);
        let pub_old: String = key_old.verifying_key().to_bytes().iter().map(|b| format!("{:02x}", b)).collect();
        let pub_new: String = key_new.verifying_key().to_bytes().iter().map(|b| format!("{:02x}", b)).collect();

        let sign_bundle = |dir: &Path,
                           sk: &SigningKey,
                           pk_hex: &str,
                           rot: Option<KeyRotationDelegation>,
                           mode: SigningMode,
                           channel: &str| {
            let bdir = dir.join("bundle");
            let mut m: ReleaseBundleManifest =
                serde_json::from_slice(&fs::read(bdir.join("manifest.fpbundle.json")).unwrap()).unwrap();
            m.publisher = Some(PublisherIdentitySpec {
                publisher_id: "pub-p6-gate".to_string(),
                display_name: "P6 Gate Publisher".to_string(),
                organization: "P6 Gate Labs".to_string(),
                website: None,
                support_contact: None,
                copyright: "Copyright (C) 2026 P6 Gate Labs".to_string(),
                ed25519_key_id: "k-p6g".to_string(),
                ed25519_pubkey_hex: pk_hex.to_string(),
                authenticode_cert_ref: None,
                release_channel: channel.to_string(),
                signing_policy: mode,
            });
            m.signing = Some(ReleaseSigningMetadata {
                signing_mode: mode,
                publisher_id: "pub-p6-gate".to_string(),
                publisher_organization: "P6 Gate Labs".to_string(),
                release_channel: channel.to_string(),
                authenticode_trust_status: AuthenticodeTrustStatus::Unsigned,
                trusted_public_release: false,
                signer_thumbprint: None,
                signer_subject: None,
                version_info: PeVersionInfoSpec::default(),
                key_rotation: rot,
            });
            let mb = serde_json::to_vec_pretty(&m).unwrap();
            fs::write(bdir.join("manifest.fpbundle.json"), &mb).unwrap();
            let mut sig_env: SignedBundleEnvelope =
                serde_json::from_slice(&fs::read(bdir.join("signature.fpbundle.json")).unwrap()).unwrap();
            sig_env.file_checksums.insert("manifest.fpbundle.json".to_string(), sha256_hex(&mb));
            let msha = sha256_hex(&mb);
            let tjson = serde_json::to_string(&sig_env.file_checksums).unwrap();
            let thash = sha256_hex(tjson.as_bytes());
            let msg = fp_bundle_verify::canonical_signing_message(&sig_env.key_id, &msha, &thash);
            let s = sk.sign(&msg);
            sig_env.manifest_sha256 = msha;
            sig_env.checksums_table_sha256 = thash;
            sig_env.signature_ed25519_hex = s.to_bytes().iter().map(|b| format!("{:02x}", b)).collect();
            fs::write(bdir.join("signature.fpbundle.json"), serde_json::to_vec_pretty(&sig_env).unwrap()).unwrap();
        };

        let rel_100 = tmp.join("rel_100");
        let rel_110 = tmp.join("rel_110");
        let rel_120_direct = tmp.join("rel_120_direct");
        let rel_130_policy = tmp.join("rel_130_policy");
        let rel_140_compat = tmp.join("rel_140_compat");

        let deleg = ProducerKeyManager::sign_key_rotation_delegation(&key_old, &pub_new, "pub-p6-gate", "p6-gate", 30);
        create_signed_test_release(&rel_100, "p6-gate", "Auto_Gate.exe", "1.0.0", CURRENT_SQLITE_SCHEMA_VERSION, 1, [91u8; 32]);
        sign_bundle(&rel_100, &key_old, &pub_old, None, SigningMode::UnsignedInternal, "qa");

        create_signed_test_release(&rel_110, "p6-gate", "Auto_Gate.exe", "1.1.0", CURRENT_SQLITE_SCHEMA_VERSION, 1, [92u8; 32]);
        sign_bundle(&rel_110, &key_new, &pub_new, Some(deleg), SigningMode::UnsignedInternal, "qa");

        create_signed_test_release(&rel_120_direct, "p6-gate", "Auto_Gate.exe", "1.2.0", CURRENT_SQLITE_SCHEMA_VERSION, 1, [92u8; 32]);
        sign_bundle(&rel_120_direct, &key_new, &pub_new, None, SigningMode::UnsignedInternal, "qa");

        create_signed_test_release(&rel_130_policy, "p6-gate", "Auto_Gate.exe", "1.3.0", CURRENT_SQLITE_SCHEMA_VERSION, 1, [92u8; 32]);
        sign_bundle(&rel_130_policy, &key_new, &pub_new, None, SigningMode::PublicSigned, "production");

        create_signed_test_release(&rel_140_compat, "p6-gate", "Auto_Gate.exe", "1.4.0", CURRENT_SQLITE_SCHEMA_VERSION + 50, 99, [92u8; 32]);
        let deleg_compat = ProducerKeyManager::sign_key_rotation_delegation(&key_old, &pub_new, "pub-p6-gate", "p6-gate", 31);
        sign_bundle(&rel_140_compat, &key_new, &pub_new, Some(deleg_compat), SigningMode::UnsignedInternal, "qa");

        let opts = UpdateExecutionOptions { skip_child_verify_exec: true, ..Default::default() };

        // Part A: Previously-trusted key gets revoked after rollback -> re-upgrade & direct upgrade MUST fail
        let rev_root = tmp.join("inst_revoked");
        let u_rev = MiniToolUpdater::new_with_external_anchor(&rev_root, &anchor_dir);
        u_rev.install_initial(&rel_100, &opts).unwrap();
        u_rev.upgrade_from_release(&rel_110, &opts).unwrap();
        u_rev.rollback_to_previous(false, true).unwrap();
        u_rev.revoke_pubkey(&pub_new, "compromised_after_rollback").unwrap();

        let err_reupg_revoked = u_rev.upgrade_from_release(&rel_110, &opts).unwrap_err().to_string();
        assert!(err_reupg_revoked.contains("KEY_REVOKED"), "got: {}", err_reupg_revoked);
        let err_direct_revoked = u_rev.upgrade_from_release(&rel_120_direct, &opts).unwrap_err().to_string();
        assert!(
            err_direct_revoked.contains("UPDATE_ROOT_OF_TRUST_UNTRUSTED") || err_direct_revoked.contains("KEY_REVOKED"),
            "got: {}",
            err_direct_revoked
        );

        // Even if trusted_pubkeys still retained pub_new alongside revoked_pubkeys, BOTH paths must reject!
        let mut st_both = u_rev.read_trust_state().unwrap().unwrap();
        st_both.trusted_pubkeys.push(pub_new.clone());
        st_both.trust_epoch += 1;
        u_rev.write_trust_state(&st_both).unwrap();
        let err_both = u_rev.upgrade_from_release(&rel_110, &opts).unwrap_err().to_string();
        assert!(err_both.contains("KEY_REVOKED"), "got: {}", err_both);

        // Part B: Non-revoked already-trusted key after rollback still MUST enforce release policy & compat gate
        let gate_root = tmp.join("inst_gates");
        let u_gate = MiniToolUpdater::new_with_external_anchor(&gate_root, &anchor_dir);
        u_gate.install_initial(&rel_100, &opts).unwrap();
        u_gate.upgrade_from_release(&rel_110, &opts).unwrap();
        u_gate.rollback_to_previous(false, true).unwrap();

        let err_policy = u_gate.upgrade_from_release(&rel_130_policy, &opts).unwrap_err().to_string();
        assert!(err_policy.contains("AUTHENTICODE_PUBLIC_TRUST_REQUIRED"), "got: {}", err_policy);

        let err_compat = u_gate.upgrade_from_release(&rel_140_compat, &opts).unwrap_err().to_string();
        assert!(err_compat.contains("COMPATIBILITY_ERROR"), "got: {}", err_compat);
        assert_eq!(u_gate.read_active_pointer().unwrap().unwrap().active_version, "1.0.0");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[cfg(windows)]
    #[test]
    fn test_p6_closeout_windows_atomic_write_acl_lock_exhaustion_and_journal_recovery() {
        use std::os::windows::ffi::OsStrExt;

        const GENERIC_READ: u32 = 0x8000_0000;
        const GENERIC_WRITE: u32 = 0x4000_0000;
        const FILE_SHARE_READ: u32 = 0x0000_0001;
        const OPEN_EXISTING: u32 = 3;
        const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;

        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn CreateFileW(
                lpFileName: *const u16,
                dwDesiredAccess: u32,
                dwShareMode: u32,
                lpSecurityAttributes: *const core::ffi::c_void,
                dwCreationDisposition: u32,
                dwFlagsAndAttributes: u32,
                hTemplateFile: *mut core::ffi::c_void,
            ) -> *mut core::ffi::c_void;
            fn CloseHandle(hObject: *mut core::ffi::c_void) -> i32;
        }

        let tmp = std::env::temp_dir().join(format!("fp_updater_p6_winio_{}", Uuid::new_v4()));
        fs::create_dir_all(&tmp).unwrap();

        // 1. Permanent read-only destination -> retries exhaust and fail closed with Win32 error 5
        let ro_file = tmp.join("readonly_ptr.json");
        fs::write(&ro_file, b"ORIGINAL_RO").unwrap();
        let mut perms = fs::metadata(&ro_file).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&ro_file, perms).unwrap();
        let ro_err = atomic_write_file(&ro_file, b"OVERWRITE_FAIL").unwrap_err().to_string();
        assert!(ro_err.contains("ACTIVE_POINTER_LOCKED"), "got: {}", ro_err);
        assert_eq!(fs::read(&ro_file).unwrap(), b"ORIGINAL_RO");
        let ro_tmps: Vec<_> = fs::read_dir(&tmp)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp."))
            .collect();
        assert!(ro_tmps.is_empty(), "leftover tmp files found: {:?}", ro_tmps);
        let mut perms_w = fs::metadata(&ro_file).unwrap().permissions();
        perms_w.set_readonly(false);
        let _ = fs::set_permissions(&ro_file, perms_w);

        // 2. Prolonged exclusive Win32 lock (> 140ms retry window) -> retry exhaustion preserves original & cleans tmp
        let lock_file = tmp.join("locked_long.json");
        fs::write(&lock_file, b"ORIGINAL_LOCKED").unwrap();
        let lock_w: Vec<u16> = lock_file.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
        let h_long = unsafe {
            CreateFileW(
                lock_w.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0,
                core::ptr::null_mut(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                core::ptr::null_mut(),
            )
        };
        assert_ne!(h_long, (-1isize) as *mut core::ffi::c_void);
        let t0 = std::time::Instant::now();
        let long_err = atomic_write_file(&lock_file, b"SHOULD_FAIL").unwrap_err().to_string();
        let elapsed = t0.elapsed();
        unsafe { CloseHandle(h_long); }
        assert!(long_err.contains("ACTIVE_POINTER_LOCKED"), "got: {}", long_err);
        assert!(elapsed.as_millis() >= 110, "expected retry backoff elapsed >=110ms, got {:?}", elapsed);
        assert_eq!(fs::read(&lock_file).unwrap(), b"ORIGINAL_LOCKED");

        // 3. Transient lock released inside retry window (~55ms) -> retry succeeds transparently
        let h_short = unsafe {
            CreateFileW(
                lock_w.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0,
                core::ptr::null_mut(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                core::ptr::null_mut(),
            )
        };
        assert_ne!(h_short, (-1isize) as *mut core::ffi::c_void);
        let h_short_addr = h_short as usize;
        let releaser = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(55));
            unsafe { CloseHandle(h_short_addr as *mut core::ffi::c_void); }
        });
        atomic_write_file(&lock_file, b"TRANSIENT_RECOVERED").unwrap();
        releaser.join().unwrap();
        assert_eq!(fs::read(&lock_file).unwrap(), b"TRANSIENT_RECOVERED");

        // 4. End-to-end upgrade aborted at atomic pointer flip by prolonged lock + journal recovery
        let rel_100 = tmp.join("rel_100");
        let rel_110 = tmp.join("rel_110");
        let anchor_dir = tmp.join("ext_anchors");
        create_signed_test_release(&rel_100, "p6-io", "Auto_Io.exe", "1.0.0", CURRENT_SQLITE_SCHEMA_VERSION, 1, [93u8; 32]);
        create_signed_test_release(&rel_110, "p6-io", "Auto_Io.exe", "1.1.0", CURRENT_SQLITE_SCHEMA_VERSION, 1, [93u8; 32]);

        let install_root = tmp.join("inst_io");
        let updater = MiniToolUpdater::new_with_external_anchor(&install_root, &anchor_dir);
        let opts = UpdateExecutionOptions { skip_child_verify_exec: true, ..Default::default() };
        updater.install_initial(&rel_100, &opts).unwrap();

        let ptr_path = updater.active_pointer_path();
        let ptr_w: Vec<u16> = ptr_path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
        // FILE_SHARE_READ allows preflight fs::read checks while blocking MoveFileExW replace with ERROR_SHARING_VIOLATION (32)
        let h_ptr = unsafe {
            CreateFileW(
                ptr_w.as_ptr(),
                GENERIC_READ,
                FILE_SHARE_READ,
                core::ptr::null(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                core::ptr::null_mut(),
            )
        };
        assert_ne!(h_ptr, (-1isize) as *mut core::ffi::c_void);
        let upg_err = updater.upgrade_from_release(&rel_110, &opts).unwrap_err().to_string();
        unsafe { CloseHandle(h_ptr); }
        assert!(upg_err.contains("ACTIVE_POINTER_LOCKED"), "got: {}", upg_err);
        assert_eq!(updater.read_active_pointer().unwrap().unwrap().active_version, "1.0.0");
        assert_eq!(updater.read_trust_state().unwrap().unwrap().trust_epoch, 1);

        let rec = updater.reconcile_incomplete_journal(true).unwrap();
        assert_eq!(rec.status, "RECOVERED_TO_PREVIOUS");
        assert!(rec.db_integrity_ok);
        assert!(updater.verify_install_security_audit().unwrap().passed);

        // Once lock cleared & journal reconciled, upgrade succeeds cleanly!
        let ok_after = updater.upgrade_from_release(&rel_110, &opts).unwrap();
        assert_eq!(ok_after.active_version, "1.1.0");

        let _ = fs::remove_dir_all(&tmp);
    }
}
