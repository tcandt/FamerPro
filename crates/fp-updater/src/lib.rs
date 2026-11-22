use anyhow::{bail, Context, Result};
use chrono::Utc;
use fp_bundle_contract::{
    check_sqlite_downgrade_compatibility, CompatibilityVersions, MiniToolUiSchema,
    ReleaseBundleManifest, RuntimeCompatibilityProfile, SemVer, CURRENT_SQLITE_SCHEMA_VERSION,
};
use fp_bundle_verify::{
    read_pe_embedded_pubkey_from_exe, validate_safe_relative_path, verify_bundle_compatibility_gate,
    SignedBundleEnvelope, PE_ED25519_PUBKEY_RESOURCE_ID,
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

/// Forward-signed key rotation delegation contract (designed for P5.3):
/// Allows rotating from `old_pubkey_hex` -> `new_pubkey_hex` ONLY when signed by `old_pubkey_hex`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KeyRotationDelegation {
    pub old_pubkey_hex: String,
    pub new_pubkey_hex: String,
    pub issued_at_utc: String,
    pub expires_at_utc: String,
    pub signature_by_old_key_hex: String,
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
}

pub struct MiniToolUpdater {
    pub install_root: PathBuf,
}

impl MiniToolUpdater {
    pub fn new<P: AsRef<Path>>(install_root: P) -> Self {
        Self {
            install_root: install_root.as_ref().to_path_buf(),
        }
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

        // CRITICAL P5.2 ROOT OF TRUST ENFORCEMENT:
        // When updating from an already-installed Mini Tool, the trusted Ed25519 publisher key
        // MUST come from the previously installed trusted binary (or explicit trust anchor),
        // NOT self-asserted solely by the untrusted incoming candidate EXE.
        let effective_trust_pubkey = match trusted_anchor_pubkey_hex {
            Some(anchor) => {
                if !pe_pubkey_hex.eq_ignore_ascii_case(anchor.trim()) {
                    bail!(
                        "UPDATE_ROOT_OF_TRUST_UNTRUSTED: candidate PE RT_RCDATA #{} pubkey '{}' does not match installed trusted publisher pubkey '{}' (key rotation requires P5.3 signed delegation)",
                        PE_ED25519_PUBKEY_RESOURCE_ID,
                        pe_pubkey_hex,
                        anchor.trim()
                    );
                }
                anchor.trim().to_string()
            }
            None => pe_pubkey_hex.clone(),
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

        Ok(VerifiedCandidateRelease {
            manifest,
            ui_schema,
            workflows,
            exe_path,
            pe_pubkey_hex: effective_trust_pubkey,
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

        let verified = Self::verify_candidate_release_dir_with_anchor(
            candidate_release_dir,
            &RuntimeCompatibilityProfile::default(),
            Some(&installed_trusted_pubkey),
        )?;

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

        // Step 5: Verify staged bundle strictly against installed trust anchor & SQLite migration validation
        let staged_verified = Self::verify_candidate_release_dir_with_anchor(
            &staging_dir,
            &RuntimeCompatibilityProfile::default(),
            Some(&installed_trusted_pubkey),
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
            trusted_pubkey_hex: Some(installed_trusted_pubkey.clone()),
            post_update_auto_resume_blocked: auto_blocked,
            blocked_job_ids: unverified_checkpoint_jobs.clone(),
            backup_db_path: journal.backup_db_path.clone(),
            activated_at_utc: Utc::now().to_rfc3339(),
            journal_id: journal.journal_id.clone(),
        };
        self.write_active_pointer_with_shadow(&next_pointer)?;
        journal.stage = UpdateStage::Activated;
        self.write_journal(&mut journal)?;
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
        let prev_verified = Self::verify_candidate_release_dir_with_anchor(
            &prev_dir,
            &RuntimeCompatibilityProfile::default(),
            Some(&installed_trusted_pubkey),
        )?;
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
            trusted_pubkey_hex: Some(installed_trusted_pubkey),
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
                // verify strictly against previous trusted anchor if present!
                let anchor_opt = journal
                    .previous_active_pointer
                    .as_ref()
                    .and_then(|p| p.trusted_pubkey_hex.clone());
                let target_dir = PathBuf::from(&journal.staged_version_dir);
                let forward_ok = if target_dir.exists() {
                    if let Ok(ver_rel) = Self::verify_candidate_release_dir_with_anchor(
                        &target_dir,
                        &RuntimeCompatibilityProfile::default(),
                        anchor_opt.as_deref(),
                    ) {
                        if skip_child_verify_exec {
                            true
                        } else {
                            verify_staged_exe_bundle(&ver_rel.exe_path, &self.data_dir()).is_ok()
                        }
                    } else {
                        false
                    }
                } else {
                    false
                };

                if forward_ok && verify_db_ok_if_exists(&live_db_path) {
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
                        detail: "Crash happened after atomic activation; verified target release and completed journal.".to_string(),
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
        // 2. Fallback to previous pointer in journal
        if let Some(j) = journal_opt {
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
        // 3. Scan `versions/v*` for highest SemVer passing full cryptographic & compatibility check
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
        let ok = unsafe {
            MoveFileExW(
                src_w.as_ptr(),
                dst_w.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        };
        if ok == 0 {
            let code = unsafe { GetLastError() };
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
}
