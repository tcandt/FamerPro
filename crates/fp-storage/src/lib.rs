pub mod fs_atomic;
pub mod jx2_seed;
pub mod migrations;
pub mod models;

use anyhow::{bail, Context, Result};
use chrono::Utc;
use fp_bundle_contract::{
    CompatibilityVersions, GameProjectManifest, MiniLayoutPreset, MiniTaskDef, MiniToolUiSchema,
    ReleaseLifecycleState, SemVer, TaskCategoryDef,
};
use fp_domain::{
    infer_json_value_type, ConnectionState, DeviceSummary, RuntimeVarDeltaEvent,
    RuntimeVariableEnvelope, StepEvent, VariableSource,
};
use fs_atomic::{
    atomic_write_bytes, copy_dir_files_safe, ensure_contained_project_dir, is_reparse_or_symlink,
    reconcile_project_storage_with_witness, validate_safe_rel_path, CommitWitness,
    CreateJournalEntry, DeleteJournalEntry, FileBackupEntry, MutationCommitWitness,
    MutationJournalEntry, JOURNAL_DIR_NAME, STAGING_DIR_NAME, TRASH_DIR_NAME,
};
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub use models::*;

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut s = String::with_capacity(digest.len() * 2);
    for b in digest {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

pub fn validate_project_slug(slug: &str) -> Result<String> {
    let s = slug.trim();
    if s.len() < 2 || s.len() > 64 {
        bail!("Invalid project slug '{}': length must be 2..=64", slug);
    }
    if s.starts_with('-') || s.ends_with('-') || s.contains("..") {
        bail!("Invalid project slug '{}'", slug);
    }
    if !s
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        bail!(
            "Invalid project slug '{}': only lowercase [a-z0-9-] allowed",
            slug
        );
    }
    Ok(s.to_string())
}

fn default_exe_name_from_slug(slug: &str) -> String {
    let pascal: String = slug
        .split('-')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join("_");
    format!(
        "Auto_{}.exe",
        if pascal.is_empty() { "Tool" } else { &pascal }
    )
}

fn ensure_default_fallback_project_conn(conn: &Connection, now: &str) -> Result<String> {
    let legacy_exists: Option<String> = {
        let mut st = conn.prepare("SELECT id FROM projects WHERE id = ?1 LIMIT 1;")?;
        let mut rows = st.query(params![DEFAULT_LEGACY_PROJECT_ID])?;
        rows.next()?.map(|r| r.get(0)).transpose()?
    };
    if let Some(pid) = legacy_exists {
        return Ok(pid);
    }
    conn.execute(
        "INSERT OR IGNORE INTO projects (
            id, slug, name, game_package_id, version, exe_name, window_title,
            icon_rel_path, target_width_px, target_height_px, bundle_adb,
            project_dir_path, created_at, updated_at
        ) VALUES (
            ?1, ?2, 'Võ Hiệp Truyền Kỳ 2 Mobile', 'com.blackhole.jx2mobile', '1.0.0',
            'Auto_JX2_Mobile.exe', 'Auto Võ Hiệp 2 Mobile v1.0.0', 'branding/icon.ico',
            1920, 1080, 1, 'projects/jx2-mobile', ?3, ?3
        );",
        params![DEFAULT_LEGACY_PROJECT_ID, DEFAULT_LEGACY_PROJECT_SLUG, now],
    )?;
    Ok(DEFAULT_LEGACY_PROJECT_ID.to_string())
}

fn check_integrity_on_conn(conn: &Connection) -> Result<IntegrityReport> {
    let integrity_check: String =
        conn.query_row("PRAGMA integrity_check;", [], |row| row.get(0))?;
    let journal_mode: String = conn.query_row("PRAGMA journal_mode;", [], |row| row.get(0))?;

    let fk_violations = {
        let mut stmt = conn.prepare("PRAGMA foreign_key_check;")?;
        let rows = stmt.query_map([], |_| Ok(1usize))?;
        rows.count()
    };

    let schema_versions = migrations::list_applied_versions(conn)?;

    let mut table_counts = BTreeMap::new();
    let mut tbl_stmt = conn.prepare(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name ASC",
    )?;
    let tbl_names: Vec<String> = tbl_stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    for tbl in tbl_names {
        let sql = format!("SELECT COUNT(*) FROM \"{}\";", tbl.replace('"', "\"\""));
        let cnt: i64 = conn.query_row(&sql, [], |r| r.get(0))?;
        table_counts.insert(tbl, cnt);
    }

    let is_healthy = integrity_check.eq_ignore_ascii_case("ok") && fk_violations == 0;
    Ok(IntegrityReport {
        integrity_check,
        foreign_key_violations: fk_violations,
        journal_mode,
        schema_versions,
        table_counts,
        is_healthy,
    })
}

struct SqliteCommitWitness<'a>(&'a Connection);

impl CommitWitness for SqliteCommitWitness<'_> {
    fn has_project(&self, project_id: &str) -> Result<bool> {
        let count: i64 = self.0.query_row(
            "SELECT COUNT(*) FROM projects WHERE id = ?1;",
            params![project_id],
            |r| r.get(0),
        )?;
        Ok(count > 0)
    }

    fn active_project_dirs(&self) -> Result<Vec<PathBuf>> {
        let mut stmt = self.0.prepare("SELECT project_dir_path FROM projects;")?;
        let dirs = stmt
            .query_map([], |r| Ok(PathBuf::from(r.get::<_, String>(0)?)))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(dirs)
    }

    fn matches_mutation_witness(&self, witness: &MutationCommitWitness) -> Result<bool> {
        match witness {
            MutationCommitWitness::ProjectUpdatedAt {
                project_id,
                expected_updated_at,
            } => {
                let count: i64 = self.0.query_row(
                    "SELECT COUNT(*) FROM projects WHERE id = ?1 AND updated_at = ?2;",
                    params![project_id, expected_updated_at],
                    |r| r.get(0),
                )?;
                Ok(count > 0)
            }
            MutationCommitWitness::WorkflowVersionExists {
                project_id,
                workflow_id,
                version_id,
                expected_sha256,
            } => {
                let count: i64 = self.0.query_row(
                    "SELECT COUNT(*) FROM workflow_versions wv
                     JOIN workflows w ON w.id = wv.workflow_id
                     WHERE w.project_id = ?1 AND wv.workflow_id = ?2 AND wv.id = ?3 AND wv.sha256 = ?4;",
                    params![project_id, workflow_id, version_id, expected_sha256],
                    |r| r.get(0),
                )?;
                Ok(count > 0)
            }
            MutationCommitWitness::VisionTemplateExists {
                project_id,
                rel_path,
                expected_sha256,
            } => {
                let count: i64 = self.0.query_row(
                    "SELECT COUNT(*) FROM vision_templates
                     WHERE project_id = ?1 AND rel_path = ?2 AND sha256 = ?3;",
                    params![project_id, rel_path, expected_sha256],
                    |r| r.get(0),
                )?;
                Ok(count > 0)
            }
        }
    }
}

fn begin_mutation_journal(
    proj_dir: &Path,
    project_id: &str,
    live_targets: &[&Path],
    commit_witness: MutationCommitWitness,
    created_at: &str,
) -> Result<Option<(PathBuf, PathBuf)>> {
    let Some(projects_root) = proj_dir.parent() else {
        return Ok(None);
    };
    if !proj_dir.exists() {
        return Ok(None);
    }
    let _ = ensure_contained_project_dir(projects_root, proj_dir)?;

    let journal_id = uuid::Uuid::new_v4().to_string();
    let staging_root = projects_root.join(STAGING_DIR_NAME);
    let journal_root = projects_root.join(JOURNAL_DIR_NAME);
    let backup_dir = staging_root.join(format!("bak_{}", &journal_id[..8]));
    std::fs::create_dir_all(&backup_dir)?;
    std::fs::create_dir_all(&journal_root)?;

    let mut files = Vec::with_capacity(live_targets.len());
    for (idx, live_path) in live_targets.iter().enumerate() {
        let existed_before = live_path.exists();
        let backup_path = if existed_before {
            let bytes = std::fs::read(live_path)?;
            let bp = backup_dir.join(format!("file_{}.bak", idx));
            atomic_write_bytes(&bp, &bytes)?;
            Some(bp.to_string_lossy().to_string())
        } else {
            None
        };
        files.push(FileBackupEntry {
            live_path: live_path.to_string_lossy().to_string(),
            existed_before,
            backup_path,
        });
    }

    let entry = MutationJournalEntry {
        op: "mutation".to_string(),
        journal_id: journal_id.clone(),
        project_id: project_id.to_string(),
        backup_dir: backup_dir.to_string_lossy().to_string(),
        files,
        commit_witness,
        created_at: created_at.to_string(),
    };
    let journal_path = journal_root.join(format!("mutation_{}.json", journal_id));
    atomic_write_bytes(&journal_path, &serde_json::to_vec_pretty(&entry)?)?;
    Ok(Some((journal_path, backup_dir)))
}

fn cleanup_mutation_journal(guard: Option<(PathBuf, PathBuf)>) {
    if let Some((journal_path, backup_dir)) = guard {
        if backup_dir.exists() {
            let _ = std::fs::remove_dir_all(&backup_dir);
        }
        if journal_path.exists() {
            let _ = std::fs::remove_file(&journal_path);
        }
    }
}

#[derive(Clone)]
pub struct StorageManager {
    conn: Arc<Mutex<Connection>>,
    crash_hook: Arc<Mutex<Option<Arc<dyn Fn(&str) + Send + Sync>>>>,
}

impl StorageManager {
    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let _ = conn.busy_timeout(Duration::from_millis(300));
        let sm = Self {
            conn: Arc::new(Mutex::new(conn)),
            crash_hook: Arc::new(Mutex::new(None)),
        };
        sm.run_migrations()?;
        Ok(sm)
    }

    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        if let Some(parent) = db_path.as_ref().parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let conn = Connection::open(db_path)?;
        let _ = conn.busy_timeout(Duration::from_millis(1200));
        let sm = Self {
            conn: Arc::new(Mutex::new(conn)),
            crash_hook: Arc::new(Mutex::new(None)),
        };
        let mut backoff_ms = 20u64;
        for attempt in 0..=6 {
            match sm.run_migrations() {
                Ok(()) => return Ok(sm),
                Err(e)
                    if e.downcast_ref::<rusqlite::Error>()
                        .map(is_sqlite_busy_err)
                        .unwrap_or(false)
                        && attempt < 6 =>
                {
                    std::thread::sleep(Duration::from_millis(backoff_ms));
                    backoff_ms = (backoff_ms * 2).min(200);
                }
                Err(e) => return Err(e),
            }
        }
        Ok(sm)
    }

    pub fn open_and_reconcile<P: AsRef<Path>>(
        db_path: P,
        projects_base_dir: &Path,
    ) -> Result<(Self, ReconcileReport)> {
        let sm = Self::new(db_path)?;
        let report = sm.reconcile_projects_storage_detailed(projects_base_dir)?;
        Ok((sm, report))
    }

    /// Registers an optional diagnostic hook invoked at specific crash-window stages (used in forced-kill tests).
    pub fn set_crash_stage_hook<F>(&self, hook: Option<F>)
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        let mut g = self.crash_hook.lock().unwrap();
        *g = hook.map(|f| Arc::new(f) as Arc<dyn Fn(&str) + Send + Sync>);
    }

    fn emit_crash_stage(&self, stage: &str) {
        let hook_opt = self.crash_hook.lock().unwrap().clone();
        if let Some(hook) = hook_opt {
            hook(stage);
        }
    }

    pub fn run_migrations(&self) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let _ = migrations::apply_migrations(&mut conn)?;
        Ok(())
    }

    pub fn applied_migration_versions(&self) -> Result<Vec<i64>> {
        let conn = self.conn.lock().unwrap();
        migrations::list_applied_versions(&conn)
    }

    pub fn apply_transactional_migration(
        &self,
        target_version: i64,
        sql_script: &str,
    ) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        migrations::apply_transactional_migration(&mut conn, target_version, sql_script)
    }

    pub fn check_integrity(&self) -> Result<IntegrityReport> {
        let conn = self.conn.lock().unwrap();
        check_integrity_on_conn(&conn)
    }

    pub fn optimize_wal_and_vacuum(&self) -> Result<DbOptimizeReport> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); VACUUM;")
            .context("Failed to checkpoint WAL / VACUUM SQLite database")?;
        let integrity = check_integrity_on_conn(&conn)?;
        let total_projects = integrity.table_counts.get("projects").copied().unwrap_or(0) as usize;
        let total_jobs = integrity.table_counts.get("jobs").copied().unwrap_or(0) as usize;
        Ok(DbOptimizeReport {
            wal_checkpointed: true,
            integrity_check: integrity.integrity_check,
            foreign_key_violations: integrity.foreign_key_violations,
            is_healthy: integrity.is_healthy,
            total_projects,
            total_jobs,
            optimized_at_utc: Utc::now().to_rfc3339(),
        })
    }

    fn ensure_runtime_vars_table(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS runtime_variable_snapshots (
                scope_key TEXT PRIMARY KEY,
                variables_json TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );",
        )?;
        Ok(())
    }

    pub fn partition_scope_for_source(base_scope: &str, source: VariableSource) -> String {
        let clean_base = base_scope.trim_end_matches("::testkit");
        if source == VariableSource::TestkitMock {
            format!("{}::testkit", clean_base)
        } else {
            clean_base.to_string()
        }
    }

    pub fn save_runtime_variables(
        &self,
        scope_key: &str,
        variables: &HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let mut scrubbed_vars = HashMap::new();
        for (k, v) in variables {
            let (clean, _) = fp_security::sanitize_runtime_variable(k, v);
            scrubbed_vars.insert(k.clone(), clean);
        }
        let conn = self.conn.lock().unwrap();
        Self::ensure_runtime_vars_table(&conn)?;
        let json = serde_json::to_string(&scrubbed_vars)?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO runtime_variable_snapshots (scope_key, variables_json, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(scope_key) DO UPDATE SET
               variables_json = excluded.variables_json,
               updated_at = excluded.updated_at;",
            params![scope_key, json, now],
        )?;
        Ok(())
    }

    /// Atomically commits a batch of `RuntimeVariableEnvelope`s with strict:
    /// 1) Secret redaction (string, numeric & structured JSON),
    /// 2) Storage-layer Anti-Overwrite Guard (`TESTKIT_MOCK` cannot overwrite `LIVE_*` verified records),
    /// 3) Monotonic `seq` assignment per `stream_id` in `runtime_var_streams`,
    /// 4) Event persistence in `runtime_var_events` for missed/out-of-order recovery.
    pub fn record_runtime_var_deltas(
        &self,
        base_scope_key: &str,
        project_id: &str,
        session_id: &str,
        job_id: &str,
        workflow_version: &str,
        envelopes: Vec<RuntimeVariableEnvelope>,
    ) -> Result<RuntimeVarDeltaEvent> {
        let clean_base = base_scope_key.trim_end_matches("::testkit");
        let is_testkit_batch = envelopes
            .iter()
            .any(|e| e.source == VariableSource::TestkitMock);
        let target_scope = if is_testkit_batch {
            // Even when running in TestKit mode, ensure NO entry is mixed with Live scope
            format!("{}::testkit", clean_base)
        } else {
            clean_base.to_string()
        };
        let stream_id = target_scope.clone();
        let now = Utc::now().to_rfc3339();

        let mut conn = self.conn.lock().unwrap();
        let mut backoff_ms = 15u64;
        let tx = {
            let mut attempt = 0u32;
            loop {
                match conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate) {
                    Ok(t) => break t,
                    Err(e) if is_sqlite_busy_err(&e) && attempt < 12 => {
                        attempt += 1;
                        std::thread::sleep(Duration::from_millis(backoff_ms));
                        backoff_ms = (backoff_ms * 2).min(150);
                    }
                    Err(e) => return Err(e.into()),
                }
            }
        };

        // Anti-Overwrite Invariant: a TESTKIT write may NEVER overwrite an existing verified LIVE variable
        // in the primary scope, even if invoked with raw scope keys.
        if !base_scope_key.ends_with("::testkit") && is_testkit_batch {
            for env in &envelopes {
                let existing_live: Option<(String, i64)> = {
                    let mut st = tx.prepare(
                        "SELECT source, verified FROM runtime_variable_envelopes WHERE scope_key = ?1 AND var_name = ?2;",
                    )?;
                    let mut rows = st.query(params![clean_base, env.name])?;
                    rows.next()?
                        .map(|r| Ok::<_, rusqlite::Error>((r.get(0)?, r.get(1)?)))
                        .transpose()?
                };
                if let Some((src, ver)) = existing_live {
                    if (src == "LIVE_ADB_OCR" || src == "LIVE_STATE") && ver == 1 {
                        // Do NOT allow overwriting Live verified slot in `clean_base`; keep writing ONLY to `::testkit`.
                    }
                }
            }
        }

        // Strict guard if someone directly writes a TESTKIT envelope alongside LIVE target
        if target_scope == clean_base {
            for env in &envelopes {
                if env.source == VariableSource::TestkitMock {
                    bail!(
                        "PROVENANCE_ANTI_OVERWRITE_BLOCKED: TESTKIT_MOCK variable '{}' cannot be written to Live scope '{}'",
                        env.name,
                        clean_base
                    );
                }
            }
        }

        let current_seq: i64 = {
            let mut st =
                tx.prepare("SELECT last_seq FROM runtime_var_streams WHERE stream_id = ?1;")?;
            let mut rows = st.query(params![stream_id])?;
            rows.next()?.map(|r| r.get(0)).transpose()?.unwrap_or(0)
        };
        let next_seq = (current_seq.max(0) as u64) + 1;

        tx.execute(
            "INSERT INTO runtime_var_streams (stream_id, project_id, last_seq, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(stream_id) DO UPDATE SET
               last_seq = excluded.last_seq,
               updated_at = excluded.updated_at;",
            params![stream_id, project_id, next_seq as i64, now],
        )?;

        let mut committed_deltas = Vec::with_capacity(envelopes.len());
        for mut env in envelopes {
            let (clean_val, redacted) =
                fp_security::sanitize_runtime_variable(&env.name, &env.value);
            env.project_id = project_id.to_string();
            env.session_id = session_id.to_string();
            env.job_id = job_id.to_string();
            env.value = clean_val.clone();
            env.value_type = infer_json_value_type(&clean_val);
            env.workflow_version = workflow_version.to_string();
            env.seq = next_seq;
            env.redacted = env.redacted || redacted;
            if env.updated_at_utc.is_empty() {
                env.updated_at_utc = now.clone();
            }

            let val_json = serde_json::to_string(&env.value)?;
            tx.execute(
                "INSERT INTO runtime_variable_envelopes (
                    scope_key, var_name, project_id, session_id, job_id,
                    value_type, value_json, source, verified, confidence,
                    confidence_source, measured_confidence, rule_matched, device_id,
                    updated_at_utc, workflow_version, seq, redacted
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
                 ON CONFLICT(scope_key, var_name) DO UPDATE SET
                    project_id = excluded.project_id,
                    session_id = excluded.session_id,
                    job_id = excluded.job_id,
                    value_type = excluded.value_type,
                    value_json = excluded.value_json,
                    source = excluded.source,
                    verified = excluded.verified,
                    confidence = excluded.confidence,
                    confidence_source = excluded.confidence_source,
                    measured_confidence = excluded.measured_confidence,
                    rule_matched = excluded.rule_matched,
                    device_id = excluded.device_id,
                    updated_at_utc = excluded.updated_at_utc,
                    workflow_version = excluded.workflow_version,
                    seq = excluded.seq,
                    redacted = excluded.redacted;",
                params![
                    target_scope,
                    env.name,
                    env.project_id,
                    env.session_id,
                    env.job_id,
                    env.value_type,
                    val_json,
                    env.source.as_str(),
                    if env.verified { 1 } else { 0 },
                    env.confidence,
                    env.confidence_source.map(|c| c.as_str().to_string()),
                    env.measured_confidence,
                    env.rule_matched.map(|b| if b { 1i64 } else { 0i64 }),
                    env.device_id,
                    env.updated_at_utc,
                    env.workflow_version,
                    next_seq as i64,
                    if env.redacted { 1 } else { 0 }
                ],
            )?;
            committed_deltas.push(env);
        }

        let deltas_json = serde_json::to_string(&committed_deltas)?;
        tx.execute(
            "INSERT INTO runtime_var_events (
                stream_id, seq, project_id, session_id, job_id,
                workflow_version, deltas_json, emitted_at_utc
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8);",
            params![
                stream_id,
                next_seq as i64,
                project_id,
                session_id,
                job_id,
                workflow_version,
                deltas_json,
                now
            ],
        )?;

        // Sync flat snapshot map for legacy summary consumers within the same partitioned scope
        let mut flat_map: HashMap<String, serde_json::Value> = HashMap::new();
        {
            let mut st = tx.prepare(
                "SELECT var_name, value_json FROM runtime_variable_envelopes WHERE scope_key = ?1;",
            )?;
            let mut rows = st.query(params![target_scope])?;
            while let Some(r) = rows.next()? {
                let k: String = r.get(0)?;
                let v_str: String = r.get(1)?;
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&v_str) {
                    flat_map.insert(k, v);
                }
            }
        }
        tx.execute(
            "INSERT INTO runtime_variable_snapshots (scope_key, variables_json, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(scope_key) DO UPDATE SET
               variables_json = excluded.variables_json,
               updated_at = excluded.updated_at;",
            params![target_scope, serde_json::to_string(&flat_map)?, now],
        )?;

        tx.commit()?;

        Ok(RuntimeVarDeltaEvent {
            stream_id,
            scope_key: target_scope,
            project_id: project_id.to_string(),
            session_id: session_id.to_string(),
            job_id: job_id.to_string(),
            seq: next_seq,
            workflow_version: workflow_version.to_string(),
            emitted_at_utc: now,
            deltas: committed_deltas,
        })
    }

    /// Strict explicit guard rejecting attempts to write TESTKIT_MOCK directly onto LIVE verified state.
    pub fn try_write_testkit_over_live_guard(
        &self,
        live_scope_key: &str,
        var_name: &str,
    ) -> Result<()> {
        let clean_live = live_scope_key.trim_end_matches("::testkit");
        let conn = self.conn.lock().unwrap();
        let existing: Option<(String, i64)> = {
            let mut st = conn.prepare(
                "SELECT source, verified FROM runtime_variable_envelopes WHERE scope_key = ?1 AND var_name = ?2;",
            )?;
            let mut rows = st.query(params![clean_live, var_name])?;
            rows.next()?
                .map(|r| Ok::<_, rusqlite::Error>((r.get(0)?, r.get(1)?)))
                .transpose()?
        };
        if let Some((src, ver)) = existing {
            if (src == "LIVE_ADB_OCR" || src == "LIVE_STATE") && ver == 1 {
                bail!(
                    "PROVENANCE_ANTI_OVERWRITE_BLOCKED: Refusing to overwrite verified {} variable '{}' in scope '{}' with TESTKIT_MOCK",
                    src,
                    var_name,
                    clean_live
                );
            }
        }
        Ok(())
    }

    fn query_envelopes_for_scope(
        conn: &Connection,
        scope_key: &str,
    ) -> Result<BTreeMap<String, RuntimeVariableEnvelope>> {
        let mut out = BTreeMap::new();
        let mut st = conn.prepare(
            "SELECT project_id, session_id, job_id, var_name, value_type, value_json,
                    source, verified, confidence, confidence_source, measured_confidence,
                    rule_matched, device_id, updated_at_utc, workflow_version, seq, redacted
             FROM runtime_variable_envelopes
             WHERE scope_key = ?1
             ORDER BY var_name ASC;",
        )?;
        let mut rows = st.query(params![scope_key])?;
        while let Some(r) = rows.next()? {
            let val_str: String = r.get(5)?;
            let src_str: String = r.get(6)?;
            let source = match src_str.as_str() {
                "LIVE_ADB_OCR" => VariableSource::LiveAdbOcr,
                "LIVE_STATE" => VariableSource::LiveState,
                "TESTKIT_MOCK" => VariableSource::TestkitMock,
                "LEGACY_UNVERIFIED" => VariableSource::LegacyUnverified,
                _ => VariableSource::Unverified,
            };
            let var_name: String = r.get(3)?;
            let value: serde_json::Value =
                serde_json::from_str(&val_str).unwrap_or(serde_json::Value::Null);
            let conf_src = r
                .get::<_, Option<String>>(9)?
                .as_deref()
                .and_then(fp_domain::ConfidenceSource::from_db_str);
            let env = RuntimeVariableEnvelope {
                project_id: r.get(0)?,
                session_id: r.get(1)?,
                job_id: r.get(2)?,
                name: var_name.clone(),
                value,
                value_type: r.get(4)?,
                source,
                verified: r.get::<_, i64>(7)? != 0,
                confidence: r.get::<_, Option<f64>>(8)?.map(|v| v as f32),
                confidence_source: conf_src,
                measured_confidence: r.get::<_, Option<f64>>(10)?.map(|v| v as f32),
                rule_matched: r.get::<_, Option<i64>>(11)?.map(|v| v != 0),
                device_id: r.get(12)?,
                updated_at_utc: r.get(13)?,
                workflow_version: r.get(14)?,
                seq: r.get::<_, i64>(15)?.max(0) as u64,
                redacted: r.get::<_, i64>(16)? != 0,
            };
            out.insert(var_name, env);
        }
        Ok(out)
    }

    pub fn load_runtime_snapshot_with_recovery(
        &self,
        base_scope_key: &str,
        since_seq_by_stream: Option<&HashMap<String, u64>>,
    ) -> Result<RuntimeProvenanceSnapshot> {
        let clean_base = base_scope_key.trim_end_matches("::testkit");
        let testkit_scope = format!("{}::testkit", clean_base);
        let conn = self.conn.lock().unwrap();

        let live_envelopes = Self::query_envelopes_for_scope(&conn, clean_base)?;
        let testkit_envelopes = Self::query_envelopes_for_scope(&conn, &testkit_scope)?;

        let mut stream_seqs = HashMap::new();
        for sid in [clean_base, testkit_scope.as_str()] {
            let seq: i64 = {
                let mut st =
                    conn.prepare("SELECT last_seq FROM runtime_var_streams WHERE stream_id = ?1;")?;
                let mut rows = st.query(params![sid])?;
                rows.next()?.map(|r| r.get(0)).transpose()?.unwrap_or(0)
            };
            if seq > 0 {
                stream_seqs.insert(sid.to_string(), seq as u64);
            }
        }

        let mut missed_events = Vec::new();
        for sid in [clean_base, testkit_scope.as_str()] {
            let after_seq = since_seq_by_stream
                .and_then(|m| m.get(sid))
                .copied()
                .unwrap_or(0);
            let mut st = conn.prepare(
                "SELECT stream_id, seq, project_id, session_id, job_id, workflow_version, deltas_json, emitted_at_utc
                 FROM runtime_var_events
                 WHERE stream_id = ?1 AND seq > ?2
                 ORDER BY seq ASC;",
            )?;
            let mut rows = st.query(params![sid, after_seq as i64])?;
            while let Some(r) = rows.next()? {
                let deltas_str: String = r.get(6)?;
                let deltas: Vec<RuntimeVariableEnvelope> =
                    serde_json::from_str(&deltas_str).unwrap_or_default();
                missed_events.push(RuntimeVarDeltaEvent {
                    stream_id: r.get(0)?,
                    scope_key: r.get(0)?,
                    project_id: r.get(2)?,
                    session_id: r.get(3)?,
                    job_id: r.get(4)?,
                    seq: r.get::<_, i64>(1)?.max(0) as u64,
                    workflow_version: r.get(5)?,
                    emitted_at_utc: r.get(7)?,
                    deltas,
                });
            }
        }

        Ok(RuntimeProvenanceSnapshot {
            base_scope_key: clean_base.to_string(),
            live_envelopes,
            testkit_envelopes,
            stream_seqs,
            missed_events,
            loaded_at_utc: Utc::now().to_rfc3339(),
        })
    }

    /// Reads `PRAGMA data_version` to detect cross-process SQLite WAL commits cheaply,
    /// returning both current `data_version` and any `RuntimeVarDeltaEvent`s newer than `after_seqs`.
    pub fn poll_external_runtime_deltas(
        &self,
        base_scope_key: &str,
        after_seqs: &HashMap<String, u64>,
    ) -> Result<(i64, Vec<RuntimeVarDeltaEvent>)> {
        let clean_base = base_scope_key.trim_end_matches("::testkit");
        let testkit_scope = format!("{}::testkit", clean_base);
        let conn = self.conn.lock().unwrap();
        let data_version: i64 = conn
            .query_row("PRAGMA data_version;", [], |r| r.get(0))
            .unwrap_or(0);

        let events =
            query_events_for_streams(&conn, &[clean_base, testkit_scope.as_str()], after_seqs)?;
        Ok((data_version, events))
    }

    /// Compacts `runtime_var_events` keeping only the latest `keep_last_n` sequenced rows per stream,
    /// while leaving `runtime_var_streams.last_seq` and `runtime_variable_envelopes` completely intact.
    pub fn compact_runtime_var_events(&self, stream_id: &str, keep_last_n: usize) -> Result<usize> {
        let mut conn = self.conn.lock().unwrap();
        let mut backoff_ms = 15u64;
        let tx = {
            let mut attempt = 0u32;
            loop {
                match conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate) {
                    Ok(t) => break t,
                    Err(e) if is_sqlite_busy_err(&e) && attempt < 8 => {
                        attempt += 1;
                        std::thread::sleep(Duration::from_millis(backoff_ms));
                        backoff_ms = (backoff_ms * 2).min(150);
                    }
                    Err(e) => return Err(e.into()),
                }
            }
        };
        let cutoff_seq: Option<i64> = {
            let mut st = tx.prepare(
                "SELECT seq FROM runtime_var_events WHERE stream_id = ?1 ORDER BY seq DESC LIMIT 1 OFFSET ?2;",
            )?;
            let mut rows = st.query(params![stream_id, keep_last_n as i64])?;
            rows.next()?.map(|r| r.get(0)).transpose()?
        };
        let deleted = if let Some(cut) = cutoff_seq {
            tx.execute(
                "DELETE FROM runtime_var_events WHERE stream_id = ?1 AND seq <= ?2;",
                params![stream_id, cut],
            )?
        } else {
            0
        };
        tx.commit()?;
        Ok(deleted)
    }

    /// Queries highest applied migration version directly on the DB (used for downgrade safety checks).
    pub fn highest_applied_migration_version(&self) -> Result<u32> {
        let vers = self.applied_migration_versions()?;
        Ok(vers.into_iter().max().unwrap_or(0).max(0) as u32)
    }

    pub fn load_runtime_variables(&self, scope_key: &str) -> Result<RuntimeVariablesSnapshot> {
        let prov = self.load_runtime_snapshot_with_recovery(scope_key, None)?;
        let mut flat_vars = HashMap::new();
        for (k, env) in &prov.testkit_envelopes {
            flat_vars.insert(k.clone(), env.value.clone());
        }
        // LIVE/Legacy always takes precedence over TESTKIT in flat compatibility map!
        for (k, env) in &prov.live_envelopes {
            flat_vars.insert(k.clone(), env.value.clone());
        }

        let conn = self.conn.lock().unwrap();
        Self::ensure_runtime_vars_table(&conn)?;
        let mut stmt = conn.prepare(
            "SELECT variables_json, updated_at FROM runtime_variable_snapshots WHERE scope_key = ?1;",
        )?;
        let mut rows = stmt.query(params![scope_key])?;
        if let Some(row) = rows.next()? {
            let json: String = row.get(0)?;
            let updated_at: String = row.get(1)?;
            let legacy_map: HashMap<String, serde_json::Value> =
                serde_json::from_str(&json).unwrap_or_default();
            for (k, v) in legacy_map {
                flat_vars.entry(k).or_insert(v);
            }
            Ok(RuntimeVariablesSnapshot {
                scope_key: scope_key.to_string(),
                variables: flat_vars,
                updated_at: Some(updated_at),
                envelopes: prov.live_envelopes,
                testkit_envelopes: prov.testkit_envelopes,
                stream_seqs: prov.stream_seqs,
            })
        } else {
            Ok(RuntimeVariablesSnapshot {
                scope_key: scope_key.to_string(),
                variables: flat_vars,
                updated_at: None,
                envelopes: prov.live_envelopes,
                testkit_envelopes: prov.testkit_envelopes,
                stream_seqs: prov.stream_seqs,
            })
        }
    }

    /// Creates a consistent SQLite Online Backup via `rusqlite::backup::Backup`,
    /// then verifies `PRAGMA integrity_check` and `PRAGMA foreign_key_check` on the backup.
    pub fn online_backup_to<P: AsRef<Path>>(&self, dest_path: P) -> Result<BackupReport> {
        let dest = dest_path.as_ref();
        if let Some(parent) = dest.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        if dest.exists() {
            std::fs::remove_file(dest)?;
        }

        {
            let src_conn = self.conn.lock().unwrap();
            let mut dst_conn = Connection::open(dest)?;
            let backup = rusqlite::backup::Backup::new(&src_conn, &mut dst_conn)?;
            backup.run_to_completion(100, Duration::from_millis(5), None)?;
        }

        let verify_conn = Connection::open(dest)?;
        let integrity = check_integrity_on_conn(&verify_conn)?;
        if !integrity.is_healthy {
            bail!(
                "Backup failed integrity verification: integrity='{}', fk_violations={}",
                integrity.integrity_check,
                integrity.foreign_key_violations
            );
        }
        drop(verify_conn);

        let bytes = std::fs::read(dest)?;
        let sha = sha256_hex(&bytes);
        Ok(BackupReport {
            backup_path: dest.to_string_lossy().to_string(),
            backup_size_bytes: bytes.len() as u64,
            backup_sha256: sha,
            integrity,
            backed_up_at_utc: Utc::now().to_rfc3339(),
        })
    }

    /// Creates a pre-migration consistent SQLite Online Backup from `src_db_path` WITHOUT running migrations first,
    /// preserving the exact source schema version in `dest_path`.
    pub fn online_backup_raw_file_to<P: AsRef<Path>, Q: AsRef<Path>>(
        src_db_path: P,
        dest_path: Q,
    ) -> Result<BackupReport> {
        let src = src_db_path.as_ref();
        let dest = dest_path.as_ref();
        if let Some(parent) = dest.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        if dest.exists() {
            std::fs::remove_file(dest)?;
        }

        {
            let src_conn = Connection::open(src)?;
            let mut dst_conn = Connection::open(dest)?;
            let backup = rusqlite::backup::Backup::new(&src_conn, &mut dst_conn)?;
            backup.run_to_completion(100, Duration::from_millis(5), None)?;
        }

        let verify_conn = Connection::open(dest)?;
        let integrity = check_integrity_on_conn(&verify_conn)?;
        if !integrity.is_healthy {
            bail!(
                "Pre-migration raw backup failed integrity verification: integrity='{}', fk_violations={}",
                integrity.integrity_check,
                integrity.foreign_key_violations
            );
        }
        drop(verify_conn);

        let bytes = std::fs::read(dest)?;
        let sha = sha256_hex(&bytes);
        Ok(BackupReport {
            backup_path: dest.to_string_lossy().to_string(),
            backup_size_bytes: bytes.len() as u64,
            backup_sha256: sha,
            integrity,
            backed_up_at_utc: Utc::now().to_rfc3339(),
        })
    }

    /// Restores a SQLite database file from a verified backup using Online Backup API.
    pub fn restore_from_backup<P: AsRef<Path>>(
        backup_path: P,
        target_db_path: P,
    ) -> Result<IntegrityReport> {
        let bk = backup_path.as_ref();
        let target = target_db_path.as_ref();
        if !bk.exists() {
            bail!("Backup file does not exist: {}", bk.display());
        }

        let src_conn = Connection::open(bk)?;
        let pre_check = check_integrity_on_conn(&src_conn)?;
        if !pre_check.is_healthy {
            bail!(
                "Refusing restore: backup '{}' failed integrity check",
                bk.display()
            );
        }

        if let Some(parent) = target.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }

        {
            let mut dst_conn = Connection::open(target)?;
            let backup = rusqlite::backup::Backup::new(&src_conn, &mut dst_conn)?;
            backup.run_to_completion(100, Duration::from_millis(5), None)?;
        }

        let verify_target = Connection::open(target)?;
        check_integrity_on_conn(&verify_target)
    }

    /// Standardized WAL-safe Rollback Runbook execution:
    /// 1. Checkpoints current WAL (`PRAGMA wal_checkpoint(TRUNCATE)`).
    /// 2. Saves a verified pre-rollback safety snapshot via SQLite Online Backup API.
    /// 3. Validates the target backup file (`PRAGMA integrity_check` + `PRAGMA foreign_key_check`).
    /// 4. Restores backup onto `current_db_path` via Online Backup API & checkpoints WAL.
    /// 5. Reopens `current_db_path` and verifies integrity, foreign keys, and schema versions.
    pub fn rollback_database_from_backup<P: AsRef<Path>>(
        current_db_path: P,
        backup_path: P,
        safety_snapshot_path: P,
    ) -> Result<RollbackVerificationReport> {
        let cur = current_db_path.as_ref();
        let bk = backup_path.as_ref();
        let snap = safety_snapshot_path.as_ref();
        if !cur.exists() {
            bail!("Current DB does not exist: {}", cur.display());
        }
        if !bk.exists() {
            bail!("Backup DB does not exist: {}", bk.display());
        }

        let (pre_versions, pre_counts, wal_busy) = {
            let cur_conn = Connection::open(cur)?;
            let (busy, _log, _chk): (i64, i64, i64) =
                cur_conn.query_row("PRAGMA wal_checkpoint(TRUNCATE);", [], |r| {
                    Ok((r.get(0)?, r.get(1)?, r.get(2)?))
                })?;
            let pre_rep = check_integrity_on_conn(&cur_conn)?;

            if let Some(p) = snap.parent() {
                if !p.as_os_str().is_empty() {
                    std::fs::create_dir_all(p)?;
                }
            }
            if snap.exists() {
                std::fs::remove_file(snap)?;
            }
            let mut snap_conn = Connection::open(snap)?;
            {
                let bk_api = rusqlite::backup::Backup::new(&cur_conn, &mut snap_conn)?;
                bk_api.run_to_completion(100, Duration::from_millis(5), None)?;
            }
            drop(snap_conn);
            let snap_verify = Connection::open(snap)?;
            let snap_integ = check_integrity_on_conn(&snap_verify)?;
            if !snap_integ.is_healthy {
                bail!("Pre-rollback safety snapshot failed integrity check");
            }
            (pre_rep.schema_versions, pre_rep.table_counts, busy)
        };

        let restored_integ = Self::restore_from_backup(bk, cur)?;

        // Checkpoint WAL & reopen to prove DB is cleanly usable post-rollback
        let (post_versions, post_integ) = {
            let reopened = Connection::open(cur)?;
            let _ = reopened.query_row("PRAGMA wal_checkpoint(TRUNCATE);", [], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            });
            let rep = check_integrity_on_conn(&reopened)?;
            if !rep.is_healthy {
                bail!("Post-rollback reopened DB failed integrity check");
            }
            (rep.schema_versions.clone(), rep)
        };

        Ok(RollbackVerificationReport {
            current_db_path: cur.to_string_lossy().to_string(),
            backup_db_path: bk.to_string_lossy().to_string(),
            safety_snapshot_path: snap.to_string_lossy().to_string(),
            pre_rollback_versions: pre_versions,
            post_rollback_versions: post_versions,
            pre_rollback_table_counts: pre_counts,
            post_rollback_integrity: if post_integ.is_healthy {
                post_integ
            } else {
                restored_integ
            },
            wal_checkpoint_busy: wal_busy,
            reopen_verified: true,
        })
    }

    /// Reconciles orphan `.fp_staging`, `.fp_trash`, `.fp_journal`, and `.tmp` entries under `projects_base_dir`
    /// against committed rows in SQLite and returns the structured audit report.
    pub fn reconcile_projects_storage_detailed(
        &self,
        projects_base_dir: &Path,
    ) -> Result<ReconcileReport> {
        let conn = self.conn.lock().unwrap();
        reconcile_project_storage_with_witness(projects_base_dir, &SqliteCommitWitness(&conn))
    }

    /// Reconciles orphan `.fp_staging`, `.fp_trash`, and `.fp_journal` entries under `projects_base_dir`
    /// against committed rows in SQLite `projects`.
    pub fn reconcile_projects_storage(&self, projects_base_dir: &Path) -> Result<usize> {
        Ok(self
            .reconcile_projects_storage_detailed(projects_base_dir)?
            .total_resolved)
    }

    /// Migrates legacy 10 JX2 workflows & template PNGs into `proj-jx2-mobile-001` non-destructively.
    pub fn migrate_legacy_jx2_assets(
        &self,
        legacy_jx2_dir: &Path,
        projects_base_dir: &Path,
    ) -> Result<Jx2MigrationSummary> {
        let mut conn = self.conn.lock().unwrap();
        jx2_seed::migrate_legacy_jx2_assets(&mut conn, legacy_jx2_dir, projects_base_dir)
    }

    // =========================================================================
    // V1 BACKWARD-COMPATIBLE METHODS (Devices, Workflows, Jobs, Step Events)
    // =========================================================================

    pub fn upsert_device(&self, dev: &DeviceSummary) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let state_str = serde_json::to_string(&dev.connection_state)?.replace('"', "");
        conn.execute(
            "INSERT INTO devices (id, adb_serial, alias, android_api, model, width_px, height_px, rotation_deg, connection_state, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET
                alias = excluded.alias,
                android_api = excluded.android_api,
                model = excluded.model,
                width_px = excluded.width_px,
                height_px = excluded.height_px,
                rotation_deg = excluded.rotation_deg,
                connection_state = excluded.connection_state,
                updated_at = excluded.updated_at;",
            params![
                dev.id,
                dev.adb_serial,
                dev.alias,
                dev.android_api,
                dev.model,
                dev.width_px,
                dev.height_px,
                dev.rotation_deg,
                state_str,
                dev.updated_at.to_rfc3339()
            ],
        )?;
        Ok(())
    }

    pub fn list_devices(&self) -> Result<Vec<DeviceSummary>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, adb_serial, alias, android_api, model, width_px, height_px, rotation_deg, connection_state, updated_at FROM devices")?;
        let rows = stmt.query_map([], |row| {
            let state_str: String = row.get(8)?;
            let conn_state = match state_str.as_str() {
                "ready" => ConnectionState::Ready,
                "unauthorized" => ConnectionState::Unauthorized,
                "authorizing" => ConnectionState::Authorizing,
                "busy" => ConnectionState::Busy,
                "recovering" => ConnectionState::Recovering,
                _ => ConnectionState::Disconnected,
            };
            let updated_at_str: String = row.get(9)?;
            let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            Ok(DeviceSummary {
                id: row.get(0)?,
                adb_serial: row.get(1)?,
                alias: row.get(2)?,
                android_api: row.get(3)?,
                model: row.get(4)?,
                width_px: row.get(5)?,
                height_px: row.get(6)?,
                rotation_deg: row.get(7)?,
                connection_state: conn_state,
                updated_at,
            })
        })?;

        let mut res = Vec::new();
        for r in rows {
            res.push(r?);
        }
        Ok(res)
    }

    pub fn save_workflow_draft(&self, id: &str, name: &str, draft_json: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        let existing_owner: Option<Option<String>> = {
            let mut st = conn.prepare("SELECT project_id FROM workflows WHERE id = ?1;")?;
            let mut rows = st.query(params![id])?;
            rows.next()?.map(|r| r.get(0)).transpose()?
        };
        let target_pid = match existing_owner {
            Some(Some(owner)) => owner,
            _ => ensure_default_fallback_project_conn(&conn, &now)?,
        };
        conn.execute(
            "INSERT INTO workflows (id, project_id, name, draft_json, draft_updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET
                project_id = excluded.project_id,
                name = excluded.name,
                draft_json = excluded.draft_json,
                draft_updated_at = excluded.draft_updated_at;",
            params![id, target_pid, name, draft_json, now],
        )?;
        Ok(())
    }

    pub fn get_workflow_draft(&self, id: &str) -> Result<Option<(String, String)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT name, draft_json FROM workflows WHERE id = ?1 AND project_id IS NOT NULL;",
        )?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some((row.get(0)?, row.get(1)?)))
        } else {
            Ok(None)
        }
    }

    pub fn ensure_job_exists(
        &self,
        job_id: &str,
        device_id: &str,
        workflow_id: &str,
        workflow_name: &str,
        graph_json: &str,
    ) -> Result<()> {
        self.ensure_job_exists_with_project(
            None,
            job_id,
            device_id,
            workflow_id,
            workflow_name,
            graph_json,
        )
    }

    fn ensure_job_exists_with_project(
        &self,
        project_id: Option<&str>,
        job_id: &str,
        device_id: &str,
        workflow_id: &str,
        workflow_name: &str,
        graph_json: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        // 1. Ensure device exists
        conn.execute(
            "INSERT INTO devices (id, adb_serial, alias, width_px, height_px, rotation_deg, connection_state, updated_at)
             VALUES (?1, ?1, ?1, 1080, 1920, 0, 'ready', ?2)
             ON CONFLICT(id) DO NOTHING;",
            params![device_id, now],
        )?;

        // 2. Resolve non-null project ownership and verify strict project isolation
        let existing_owner: Option<Option<String>> = {
            let mut st = conn.prepare("SELECT project_id FROM workflows WHERE id = ?1")?;
            let mut rows = st.query(params![workflow_id])?;
            rows.next()?.map(|r| r.get(0)).transpose()?
        };

        let resolved_pid = match project_id {
            Some(pid) => {
                if let Some(Some(ref owner)) = existing_owner {
                    if owner != pid {
                        bail!(
                            "PROJECT_ISOLATION_VIOLATION: Workflow '{}' belongs to project '{}', cannot run under '{}'",
                            workflow_id,
                            owner,
                            pid
                        );
                    }
                }
                pid.to_string()
            }
            None => match existing_owner {
                Some(Some(owner)) => owner,
                _ => ensure_default_fallback_project_conn(&conn, &now)?,
            },
        };

        conn.execute(
            "INSERT INTO workflows (id, project_id, name, draft_json, draft_updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO NOTHING;",
            params![workflow_id, resolved_pid, workflow_name, graph_json, now],
        )?;

        // 3. Ensure workflow version exists (reuse existing version ID if already created, e.g. legacy UUID)
        let version_id: String = {
            let mut st = conn.prepare(
                "SELECT id FROM workflow_versions WHERE workflow_id = ?1 ORDER BY version_no DESC LIMIT 1;",
            )?;
            let mut rows = st.query(params![workflow_id])?;
            if let Some(row) = rows.next()? {
                row.get(0)?
            } else {
                let new_ver_id = format!("{}-v1", workflow_id);
                let graph_sha = sha256_hex(graph_json.as_bytes());
                conn.execute(
                    "INSERT INTO workflow_versions (id, workflow_id, version_no, graph_json, sha256, created_at)
                     VALUES (?1, ?2, 1, ?3, ?4, ?5);",
                    params![new_ver_id, workflow_id, graph_json, graph_sha, now],
                )?;
                new_ver_id
            }
        };

        // 4. Ensure job exists (always non-null project_id matching workflow owner and existing job owner)
        let existing_job_owner: Option<Option<String>> = {
            let mut st = conn.prepare("SELECT project_id FROM jobs WHERE id = ?1;")?;
            let mut rows = st.query(params![job_id])?;
            rows.next()?.map(|r| r.get(0)).transpose()?
        };
        if let Some(Some(ref job_owner)) = existing_job_owner {
            if job_owner != &resolved_pid {
                bail!(
                    "PROJECT_ISOLATION_VIOLATION: Job '{}' belongs to project '{}', cannot update from '{}'",
                    job_id,
                    job_owner,
                    resolved_pid
                );
            }
        }
        conn.execute(
            "INSERT INTO jobs (id, project_id, device_id, workflow_version_id, priority, state, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 0, 'running', ?5, ?5)
             ON CONFLICT(id) DO UPDATE SET state = excluded.state, updated_at = excluded.updated_at;",
            params![job_id, resolved_pid, device_id, version_id, now],
        )?;

        Ok(())
    }

    pub fn complete_job(&self, job_id: &str, state: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE jobs SET state = ?1, updated_at = ?2 WHERE id = ?3;",
            params![state, now, job_id],
        )?;
        Ok(())
    }

    pub fn insert_step_event(&self, event: &StepEvent) -> Result<()> {
        let scrubbed_payload =
            if let Ok(mut pj) = serde_json::from_str::<serde_json::Value>(&event.payload_json) {
                if let Some(var_key) = pj
                    .get("varKey")
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                {
                    if let Some(raw_v) = pj.get("value").cloned() {
                        let (clean_v, _) = fp_security::sanitize_runtime_variable(&var_key, &raw_v);
                        pj["value"] = clean_v;
                    }
                }
                let (clean_pj, _) = fp_security::sanitize_runtime_variable("step_event", &pj);
                serde_json::to_string(&clean_pj).unwrap_or_else(|_| event.payload_json.clone())
            } else {
                fp_security::SecretRedactor::new().redact(&event.payload_json)
            };

        let conn = self.conn.lock().unwrap();
        let type_str = serde_json::to_string(&event.event_type)?.replace('"', "");
        conn.execute(
            "INSERT INTO step_events (id, job_id, node_id, event_type, attempt_no, at, payload_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                event.id,
                event.job_id,
                event.node_id,
                type_str,
                event.attempt_no,
                event.at.to_rfc3339(),
                scrubbed_payload
            ],
        )?;
        Ok(())
    }

    pub fn list_step_events(&self, job_id: Option<&str>) -> Result<Vec<StepEvent>> {
        let conn = self.conn.lock().unwrap();
        let query = if job_id.is_some() {
            "SELECT id, job_id, node_id, event_type, attempt_no, at, payload_json FROM step_events WHERE job_id = ?1 ORDER BY at ASC"
        } else {
            "SELECT id, job_id, node_id, event_type, attempt_no, at, payload_json FROM step_events ORDER BY at DESC LIMIT 100"
        };

        let mut stmt = conn.prepare(query)?;
        let map_row = |row: &rusqlite::Row| {
            let type_str: String = row.get(3)?;
            let event_type = match type_str.as_str() {
                "command_sent" => fp_domain::StepEventType::CommandSent,
                "device_ack" => fp_domain::StepEventType::DeviceAck,
                "verification_passed" => fp_domain::StepEventType::VerificationPassed,
                "verification_failed" => fp_domain::StepEventType::VerificationFailed,
                "checkpoint_saved" => fp_domain::StepEventType::CheckpointSaved,
                _ => fp_domain::StepEventType::Unknown,
            };

            let at_str: String = row.get(5)?;
            let at = chrono::DateTime::parse_from_rfc3339(&at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            Ok(StepEvent {
                id: row.get(0)?,
                job_id: row.get(1)?,
                node_id: row.get(2)?,
                event_type,
                attempt_no: row.get(4)?,
                at,
                payload_json: row.get(6)?,
            })
        };

        let events: Result<Vec<StepEvent>, rusqlite::Error> = if let Some(jid) = job_id {
            stmt.query_map(params![jid], map_row)?.collect()
        } else {
            stmt.query_map([], map_row)?.collect()
        };

        Ok(events?)
    }

    // =========================================================================
    // P2 MULTI-PROJECT CRUD & STRICT PROJECT ISOLATION
    // =========================================================================

    fn map_project_row(row: &rusqlite::Row) -> rusqlite::Result<GameProjectRecord> {
        Ok(GameProjectRecord {
            id: row.get(0)?,
            slug: row.get(1)?,
            name: row.get(2)?,
            game_package_id: row.get(3)?,
            version: row.get(4)?,
            exe_name: row.get(5)?,
            window_title: row.get(6)?,
            icon_rel_path: row.get(7)?,
            target_width_px: row.get::<_, i64>(8)? as u32,
            target_height_px: row.get::<_, i64>(9)? as u32,
            bundle_adb: row.get::<_, i64>(10)? != 0,
            project_dir_path: row.get(11)?,
            created_at: row.get(12)?,
            updated_at: row.get(13)?,
            publisher_profile_id: row.get(14)?,
            signing_mode: row.get(15)?,
            version_info_json: row.get(16)?,
        })
    }

    fn fetch_project_by_id_conn(
        conn: &Connection,
        project_id: &str,
    ) -> Result<Option<GameProjectRecord>> {
        let mut stmt = conn.prepare(
            "SELECT id, slug, name, game_package_id, version, exe_name, window_title,
                    icon_rel_path, target_width_px, target_height_px, bundle_adb,
                    project_dir_path, created_at, updated_at,
                    publisher_profile_id, signing_mode, version_info_json
             FROM projects WHERE id = ?1;",
        )?;
        let mut rows = stmt.query(params![project_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Self::map_project_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_project(&self, project_id: &str) -> Result<Option<GameProjectRecord>> {
        let conn = self.conn.lock().unwrap();
        Self::fetch_project_by_id_conn(&conn, project_id)
    }

    pub fn get_project_by_slug(&self, slug: &str) -> Result<Option<GameProjectRecord>> {
        let clean_slug = validate_project_slug(slug)?;
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, slug, name, game_package_id, version, exe_name, window_title,
                    icon_rel_path, target_width_px, target_height_px, bundle_adb,
                    project_dir_path, created_at, updated_at,
                    publisher_profile_id, signing_mode, version_info_json
             FROM projects WHERE slug = ?1;",
        )?;
        let mut rows = stmt.query(params![clean_slug])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Self::map_project_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn create_project(
        &self,
        req: CreateProjectRequest,
        projects_base_dir: &Path,
    ) -> Result<GameProjectRecord> {
        self.create_project_with_fault(req, projects_base_dir, CrashFaultPoint::None)
    }

    pub fn create_project_with_fault(
        &self,
        req: CreateProjectRequest,
        projects_base_dir: &Path,
        fault: CrashFaultPoint,
    ) -> Result<GameProjectRecord> {
        struct StagingGuard(Option<PathBuf>);
        impl Drop for StagingGuard {
            fn drop(&mut self) {
                if let Some(ref p) = self.0 {
                    let _ = std::fs::remove_dir_all(p);
                }
            }
        }

        let slug = validate_project_slug(&req.slug)?;
        let name = req.name.trim().to_string();
        if name.is_empty() {
            bail!("Project name cannot be empty");
        }
        let pkg = req.game_package_id.trim().to_string();
        if pkg.is_empty() {
            bail!("Game package id cannot be empty");
        }

        let project_id = format!("proj-{}-{}", slug, &uuid::Uuid::new_v4().to_string()[..8]);
        let version = req.version.unwrap_or_else(|| "1.0.0".to_string());
        let exe_name = req
            .exe_name
            .unwrap_or_else(|| default_exe_name_from_slug(&slug));
        let window_title = req
            .window_title
            .unwrap_or_else(|| format!("Auto {} v{}", name, version));
        let icon_rel_path = req
            .icon_rel_path
            .unwrap_or_else(|| "branding/icon.ico".to_string());
        let target_w = req.target_width_px.unwrap_or(1280);
        let target_h = req.target_height_px.unwrap_or(720);
        let bundle_adb = req.bundle_adb.unwrap_or(true);
        let now = Utc::now().to_rfc3339();

        let proj_dir = projects_base_dir.join(&slug);
        let _ = ensure_contained_project_dir(projects_base_dir, &proj_dir)?;
        if proj_dir.exists() {
            bail!("Project directory '{}' already exists", proj_dir.display());
        }
        let proj_dir_str = proj_dir.to_string_lossy().to_string();

        let enabled_mods = req.enabled_modules.unwrap_or_else(|| {
            vec![
                "mod.adb_core".to_string(),
                "mod.vision_ocr".to_string(),
                "mod.account_vault".to_string(),
                "mod.emulator_optimizer".to_string(),
            ]
        });

        // 1. Prepare staging directory outside active projects/ namespace
        let staging_root = projects_base_dir.join(STAGING_DIR_NAME);
        std::fs::create_dir_all(&staging_root)?;
        let staging_dir = staging_root.join(format!(
            "{}.{}",
            slug,
            &uuid::Uuid::new_v4().to_string()[..8]
        ));
        std::fs::create_dir_all(staging_dir.join("workflows"))?;
        std::fs::create_dir_all(staging_dir.join("templates"))?;
        std::fs::create_dir_all(staging_dir.join("branding"))?;
        let mut staging_guard = StagingGuard(Some(staging_dir.clone()));

        let all_mods = [
            "mod.adb_core",
            "mod.vision_ocr",
            "mod.account_vault",
            "mod.proxy_network",
            "mod.adb_port_forward",
            "mod.emulator_optimizer",
        ];
        let mut packaged_modules = Vec::new();
        for m in all_mods {
            let is_core = m == "mod.adb_core" || m == "mod.vision_ocr";
            if is_core || enabled_mods.iter().any(|e| e == m) {
                packaged_modules.push(m.to_string());
            }
        }

        let default_ui = MiniToolUiSchema {
            schema_version: 1,
            project_slug: slug.clone(),
            window_title: window_title.clone(),
            version_badge: format!("v{}", version),
            theme_accent_hex: "#1878F5".to_string(),
            header_bg_hex: "#141C28".to_string(),
            layout_preset: MiniLayoutPreset::Compact2Col,
            default_window_size: (740, 660),
            sections_order: vec![
                "device_table".to_string(),
                "task_and_log_split".to_string(),
                "footer_controls".to_string(),
            ],
            hidden_components: vec![],
            device_columns: vec![],
            task_categories: vec![],
            tasks: vec![],
            utilities: vec![],
            initial_runtime_vars: HashMap::new(),
        };
        let ui_json = serde_json::to_string_pretty(&default_ui)?;

        let manifest = GameProjectManifest {
            compatibility: CompatibilityVersions::default(),
            project_id: project_id.clone(),
            slug: slug.clone(),
            name: name.clone(),
            game_package_id: pkg.clone(),
            version: version.clone(),
            exe_name: exe_name.clone(),
            window_title: window_title.clone(),
            icon_rel_path: icon_rel_path.clone(),
            target_resolution: (target_w, target_h),
            packaged_modules: packaged_modules.clone(),
            workflow_files: vec![],
            template_files: vec![],
            bundle_adb,
        };

        if fault == CrashFaultPoint::FailManifestWrite {
            bail!("INJECTED_CRASH_FAULT: simulated manifest write failure during create_project");
        }
        atomic_write_bytes(
            &staging_dir.join("project.json"),
            &serde_json::to_vec_pretty(&manifest)?,
        )?;
        atomic_write_bytes(&staging_dir.join("ui-schema.json"), ui_json.as_bytes())?;

        // Write intent journal BEFORE moving out of staging
        let journal_root = projects_base_dir.join(JOURNAL_DIR_NAME);
        std::fs::create_dir_all(&journal_root)?;
        let create_journal_path = journal_root.join(format!("create_{}.json", project_id));
        let create_entry = CreateJournalEntry {
            op: "create".to_string(),
            project_id: project_id.clone(),
            project_slug: slug.clone(),
            staging_dir: staging_dir.to_string_lossy().to_string(),
            target_dir: proj_dir.to_string_lossy().to_string(),
            created_at: now.clone(),
        };
        atomic_write_bytes(
            &create_journal_path,
            &serde_json::to_vec_pretty(&create_entry)?,
        )?;

        self.emit_crash_stage("create:after_staging_before_rename");

        // 2. Execute SQLite transaction
        let mut conn = self.conn.lock().unwrap();
        let mut backoff_ms = 15u64;
        let tx = {
            let mut attempt = 0u32;
            loop {
                match conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate) {
                    Ok(t) => break t,
                    Err(e) if is_sqlite_busy_err(&e) && attempt < 12 => {
                        attempt += 1;
                        std::thread::sleep(Duration::from_millis(backoff_ms));
                        backoff_ms = (backoff_ms * 2).min(150);
                    }
                    Err(e) => return Err(e.into()),
                }
            }
        };

        tx.execute(
            "INSERT INTO projects (
                id, slug, name, game_package_id, version, exe_name, window_title,
                icon_rel_path, target_width_px, target_height_px, bundle_adb,
                project_dir_path, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13);",
            params![
                project_id,
                slug,
                name,
                pkg,
                version,
                exe_name,
                window_title,
                icon_rel_path,
                target_w as i64,
                target_h as i64,
                if bundle_adb { 1 } else { 0 },
                proj_dir_str,
                now
            ],
        )?;

        for m in all_mods {
            let packaged = packaged_modules.iter().any(|p| p == m);
            tx.execute(
                "INSERT INTO project_modules (project_id, module_id, packaged_in_release, enabled_by_default, config_json, updated_at)
                 VALUES (?1, ?2, ?3, ?4, '{}', ?5);",
                params![
                    project_id,
                    m,
                    if packaged { 1 } else { 0 },
                    if packaged { 1 } else { 0 },
                    now
                ],
            )?;
        }

        tx.execute(
            "INSERT INTO mini_tool_schemas (project_id, schema_version, ui_schema_json, updated_at)
             VALUES (?1, 1, ?2, ?3);",
            params![project_id, ui_json, now],
        )?;

        if fault == CrashFaultPoint::FailBeforeDbCommit {
            let _ = std::fs::remove_file(&create_journal_path);
            bail!("INJECTED_CRASH_FAULT: simulated crash before DB commit in create_project");
        }

        // 3. Atomic rename staging -> final project dir BEFORE commit
        std::fs::rename(&staging_dir, &proj_dir).with_context(|| {
            format!(
                "Failed moving staging project '{}' to '{}'",
                staging_dir.display(),
                proj_dir.display()
            )
        })?;
        staging_guard.0 = None;

        self.emit_crash_stage("create:after_rename_before_db_commit");

        if let Err(e) = tx.commit() {
            let _ = std::fs::remove_dir_all(&proj_dir);
            let _ = std::fs::remove_file(&create_journal_path);
            return Err(e.into());
        }

        self.emit_crash_stage("create:after_db_commit_before_cleanup");
        let _ = std::fs::remove_file(&create_journal_path);

        Ok(GameProjectRecord {
            id: project_id,
            slug,
            name,
            game_package_id: pkg,
            version,
            exe_name,
            window_title,
            icon_rel_path,
            target_width_px: target_w,
            target_height_px: target_h,
            bundle_adb,
            project_dir_path: proj_dir_str,
            created_at: now.clone(),
            updated_at: now,
            publisher_profile_id: None,
            signing_mode: "unsigned_internal".to_string(),
            version_info_json: None,
        })
    }

    pub fn list_projects(&self) -> Result<Vec<GameProjectSummary>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, slug, name, game_package_id, version, exe_name, window_title,
                    icon_rel_path, target_width_px, target_height_px, bundle_adb,
                    project_dir_path, created_at, updated_at,
                    publisher_profile_id, signing_mode, version_info_json
             FROM projects ORDER BY created_at ASC, name ASC;",
        )?;
        let records: Vec<GameProjectRecord> = stmt
            .query_map([], Self::map_project_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let mut summaries = Vec::with_capacity(records.len());
        for project in records {
            let pid = &project.id;
            let workflows_count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM workflows WHERE project_id = ?1;",
                params![pid],
                |r| r.get(0),
            )?;
            let tasks_count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM task_definitions WHERE project_id = ?1;",
                params![pid],
                |r| r.get(0),
            )?;
            let templates_count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM vision_templates WHERE project_id = ?1;",
                params![pid],
                |r| r.get(0),
            )?;
            let modules_count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM project_modules WHERE project_id = ?1 AND packaged_in_release = 1;",
                params![pid],
                |r| r.get(0),
            )?;
            let accounts_count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM accounts WHERE project_id = ?1;",
                params![pid],
                |r| r.get(0),
            )?;
            let jobs_count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM jobs WHERE project_id = ?1;",
                params![pid],
                |r| r.get(0),
            )?;
            let releases_count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM project_releases WHERE project_id = ?1;",
                params![pid],
                |r| r.get(0),
            )?;

            let latest_release: Option<(String, String)> = {
                let mut r_stmt = conn.prepare(
                    "SELECT version, exe_path FROM project_releases WHERE project_id = ?1 ORDER BY built_at DESC LIMIT 1;",
                )?;
                let mut rows = r_stmt.query(params![pid])?;
                if let Some(row) = rows.next()? {
                    Some((row.get(0)?, row.get(1)?))
                } else {
                    None
                }
            };

            summaries.push(GameProjectSummary {
                project,
                workflows_count: workflows_count as usize,
                tasks_count: tasks_count as usize,
                templates_count: templates_count as usize,
                modules_count: modules_count as usize,
                accounts_count: accounts_count as usize,
                jobs_count: jobs_count as usize,
                releases_count: releases_count as usize,
                latest_release_version: latest_release.as_ref().map(|x| x.0.clone()),
                latest_release_exe_path: latest_release.map(|x| x.1),
            });
        }
        Ok(summaries)
    }

    pub fn update_project(&self, req: UpdateProjectRequest) -> Result<GameProjectRecord> {
        self.update_project_with_fault(req, CrashFaultPoint::None)
    }

    pub fn update_project_with_fault(
        &self,
        req: UpdateProjectRequest,
        fault: CrashFaultPoint,
    ) -> Result<GameProjectRecord> {
        let mut conn = self.conn.lock().unwrap();
        let existing = Self::fetch_project_by_id_conn(&conn, &req.id)?
            .with_context(|| format!("Project '{}' not found", req.id))?;

        let name = req
            .name
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| existing.name.clone());
        let game_package_id = req
            .game_package_id
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| existing.game_package_id.clone());
        let version = req
            .version
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| existing.version.clone());
        let exe_name = req
            .exe_name
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| existing.exe_name.clone());
        let window_title = req
            .window_title
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| existing.window_title.clone());
        let icon_rel_path = req
            .icon_rel_path
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| existing.icon_rel_path.clone());
        let target_w = req.target_width_px.unwrap_or(existing.target_width_px);
        let target_h = req.target_height_px.unwrap_or(existing.target_height_px);
        let bundle_adb = req.bundle_adb.unwrap_or(existing.bundle_adb);
        let publisher_profile_id = match req.publisher_profile_id {
            Some(s) => {
                let t = s.trim().to_string();
                if t.is_empty() {
                    None
                } else {
                    Some(t)
                }
            }
            None => existing.publisher_profile_id.clone(),
        };
        let signing_mode = req
            .signing_mode
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| existing.signing_mode.clone());
        let version_info_json = match req.version_info_json {
            Some(s) => {
                let t = s.trim().to_string();
                if t.is_empty() {
                    None
                } else {
                    Some(t)
                }
            }
            None => existing.version_info_json.clone(),
        };
        let now = Utc::now().to_rfc3339();

        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute(
            "UPDATE projects SET
                name = ?1, game_package_id = ?2, version = ?3, exe_name = ?4,
                window_title = ?5, icon_rel_path = ?6, target_width_px = ?7,
                target_height_px = ?8, bundle_adb = ?9, publisher_profile_id = ?10,
                signing_mode = ?11, version_info_json = ?12, updated_at = ?13
             WHERE id = ?14;",
            params![
                name,
                game_package_id,
                version,
                exe_name,
                window_title,
                icon_rel_path,
                target_w as i64,
                target_h as i64,
                if bundle_adb { 1 } else { 0 },
                publisher_profile_id,
                signing_mode,
                version_info_json,
                now,
                req.id
            ],
        )?;

        // Sync mini_tool_schemas (if matching old window_title or version badge, or explicit update)
        let synced_ui_json: Option<String> = {
            let ui_row: Option<(i64, String)> = {
                let mut u_stmt = tx.prepare(
                    "SELECT schema_version, ui_schema_json FROM mini_tool_schemas WHERE project_id = ?1;",
                )?;
                let mut rows = u_stmt.query(params![req.id])?;
                rows.next()?
                    .map(|r| Ok::<_, rusqlite::Error>((r.get(0)?, r.get(1)?)))
                    .transpose()?
            };
            if let Some((ver, raw_ui)) = ui_row {
                let updated_ui =
                    if let Ok(mut parsed) = serde_json::from_str::<MiniToolUiSchema>(&raw_ui) {
                        if parsed.window_title == existing.window_title
                            || parsed.window_title != window_title
                                && window_title != existing.window_title
                        {
                            parsed.window_title = window_title.clone();
                        }
                        if parsed.version_badge == format!("v{}", existing.version) {
                            parsed.version_badge = format!("v{}", version);
                        }
                        serde_json::to_string_pretty(&parsed)?
                    } else {
                        raw_ui
                    };
                tx.execute(
                    "UPDATE mini_tool_schemas SET schema_version = ?1, ui_schema_json = ?2, updated_at = ?3
                     WHERE project_id = ?4;",
                    params![ver, updated_ui, now, req.id],
                )?;
                Some(updated_ui)
            } else {
                None
            }
        };

        // Snapshot disk files for compensating rollback if TX fails
        let proj_dir = Path::new(&existing.project_dir_path);
        let manifest_path = proj_dir.join("project.json");
        let ui_path = proj_dir.join("ui-schema.json");
        let prev_manifest_bytes = if manifest_path.exists() {
            Some(std::fs::read(&manifest_path)?)
        } else {
            None
        };
        let prev_ui_bytes = if ui_path.exists() {
            Some(std::fs::read(&ui_path)?)
        } else {
            None
        };

        let restore_disk_files = || {
            if let Some(ref b) = prev_manifest_bytes {
                let _ = atomic_write_bytes(&manifest_path, b);
            }
            if let Some(ref b) = prev_ui_bytes {
                let _ = atomic_write_bytes(&ui_path, b);
            }
        };

        let mut mutation_journal: Option<(PathBuf, PathBuf)> = None;
        if proj_dir.exists() {
            if is_reparse_or_symlink(proj_dir) {
                bail!(
                    "PROJECT_ISOLATION_VIOLATION: Refusing update on symlinked project dir '{}'",
                    proj_dir.display()
                );
            }
            mutation_journal = begin_mutation_journal(
                proj_dir,
                &req.id,
                &[manifest_path.as_path(), ui_path.as_path()],
                MutationCommitWitness::ProjectUpdatedAt {
                    project_id: req.id.clone(),
                    expected_updated_at: now.clone(),
                },
                &now,
            )?;
            if let Some(ref raw) = prev_manifest_bytes {
                let mut manifest: GameProjectManifest = serde_json::from_slice(raw)
                    .context("Corrupt project.json during update_project")?;
                manifest.name = name.clone();
                manifest.game_package_id = game_package_id.clone();
                manifest.version = version.clone();
                manifest.exe_name = exe_name.clone();
                manifest.window_title = window_title.clone();
                manifest.icon_rel_path = icon_rel_path.clone();
                manifest.target_resolution = (target_w, target_h);
                manifest.bundle_adb = bundle_adb;

                if fault == CrashFaultPoint::FailManifestWrite {
                    restore_disk_files();
                    cleanup_mutation_journal(mutation_journal);
                    bail!("INJECTED_CRASH_FAULT: simulated manifest write failure during update_project");
                }
                if let Err(e) =
                    atomic_write_bytes(&manifest_path, &serde_json::to_vec_pretty(&manifest)?)
                {
                    restore_disk_files();
                    cleanup_mutation_journal(mutation_journal);
                    return Err(e);
                }
            }
            if let Some(ref ui_json) = synced_ui_json {
                if let Err(e) = atomic_write_bytes(&ui_path, ui_json.as_bytes()) {
                    restore_disk_files();
                    cleanup_mutation_journal(mutation_journal);
                    return Err(e);
                }
            }
        }

        self.emit_crash_stage("update:after_disk_writes_before_db_commit");

        if fault == CrashFaultPoint::FailBeforeDbCommit {
            restore_disk_files();
            cleanup_mutation_journal(mutation_journal);
            bail!("INJECTED_CRASH_FAULT: simulated crash before DB commit in update_project");
        }

        if let Err(e) = tx.commit() {
            restore_disk_files();
            cleanup_mutation_journal(mutation_journal);
            return Err(e.into());
        }

        self.emit_crash_stage("update:after_db_commit_before_cleanup");
        cleanup_mutation_journal(mutation_journal);

        Ok(GameProjectRecord {
            id: existing.id,
            slug: existing.slug,
            name,
            game_package_id,
            version,
            exe_name,
            window_title,
            icon_rel_path,
            target_width_px: target_w,
            target_height_px: target_h,
            bundle_adb,
            project_dir_path: existing.project_dir_path,
            created_at: existing.created_at,
            updated_at: now,
            publisher_profile_id,
            signing_mode,
            version_info_json,
        })
    }

    pub fn clone_project(
        &self,
        req: CloneProjectRequest,
        projects_base_dir: &Path,
    ) -> Result<GameProjectRecord> {
        self.clone_project_with_fault(req, projects_base_dir, CrashFaultPoint::None)
    }

    pub fn clone_project_with_fault(
        &self,
        req: CloneProjectRequest,
        projects_base_dir: &Path,
        fault: CrashFaultPoint,
    ) -> Result<GameProjectRecord> {
        struct StagingGuard(Option<PathBuf>);
        impl Drop for StagingGuard {
            fn drop(&mut self) {
                if let Some(ref p) = self.0 {
                    let _ = std::fs::remove_dir_all(p);
                }
            }
        }

        let new_slug = validate_project_slug(&req.new_slug)?;
        let new_name = req.new_name.trim().to_string();
        if new_name.is_empty() {
            bail!("Cloned project name cannot be empty");
        }

        let new_proj_dir = projects_base_dir.join(&new_slug);
        let _ = ensure_contained_project_dir(projects_base_dir, &new_proj_dir)?;
        if new_proj_dir.exists() {
            bail!(
                "Target clone project directory '{}' already exists",
                new_proj_dir.display()
            );
        }
        let new_proj_dir_str = new_proj_dir.to_string_lossy().to_string();
        let new_id = format!(
            "proj-{}-{}",
            new_slug,
            &uuid::Uuid::new_v4().to_string()[..8]
        );
        let now = Utc::now().to_rfc3339();

        let mut conn = self.conn.lock().unwrap();
        let src = Self::fetch_project_by_id_conn(&conn, &req.source_project_id)?
            .with_context(|| format!("Source project '{}' not found", req.source_project_id))?;

        let new_pkg = req
            .new_game_package_id
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| src.game_package_id.clone());
        let new_exe = req
            .new_exe_name
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| default_exe_name_from_slug(&new_slug));
        let new_window_title = format!("Auto {} v{}", new_name, src.version);

        // Prepare isolated staging dir outside active projects/ namespace
        let staging_root = projects_base_dir.join(STAGING_DIR_NAME);
        std::fs::create_dir_all(&staging_root)?;
        let staging_dir = staging_root.join(format!(
            "{}.clone.{}",
            new_slug,
            &uuid::Uuid::new_v4().to_string()[..8]
        ));
        std::fs::create_dir_all(staging_dir.join("workflows"))?;
        std::fs::create_dir_all(staging_dir.join("templates"))?;
        std::fs::create_dir_all(staging_dir.join("branding"))?;
        let mut staging_guard = StagingGuard(Some(staging_dir.clone()));

        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;

        tx.execute(
            "INSERT INTO projects (
                id, slug, name, game_package_id, version, exe_name, window_title,
                icon_rel_path, target_width_px, target_height_px, bundle_adb,
                project_dir_path, publisher_profile_id, signing_mode, version_info_json,
                created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?16);",
            params![
                new_id,
                new_slug,
                new_name,
                new_pkg,
                src.version,
                new_exe,
                new_window_title,
                src.icon_rel_path,
                src.target_width_px as i64,
                src.target_height_px as i64,
                if src.bundle_adb { 1 } else { 0 },
                new_proj_dir_str,
                src.publisher_profile_id,
                src.signing_mode,
                src.version_info_json,
                now
            ],
        )?;

        if fault == CrashFaultPoint::FailTemplateCopy {
            bail!("INJECTED_CRASH_FAULT: simulated template copy failure during clone_project");
        }
        let src_dir = Path::new(&src.project_dir_path);
        copy_dir_files_safe(&src_dir.join("templates"), &staging_dir.join("templates"))?;
        copy_dir_files_safe(&src_dir.join("branding"), &staging_dir.join("branding"))?;

        // Clone project_modules
        let mut packaged_modules: Vec<String> = Vec::new();
        {
            let mut m_stmt = tx.prepare(
                "SELECT module_id, packaged_in_release, enabled_by_default, config_json FROM project_modules WHERE project_id = ?1;",
            )?;
            let mods: Vec<(String, i64, i64, String)> = m_stmt
                .query_map(params![src.id], |r| {
                    Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            for (mod_id, pkg_flag, en_flag, cfg) in mods {
                if pkg_flag != 0 {
                    packaged_modules.push(mod_id.clone());
                }
                tx.execute(
                    "INSERT INTO project_modules (project_id, module_id, packaged_in_release, enabled_by_default, config_json, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6);",
                    params![new_id, mod_id, pkg_flag, en_flag, cfg, now],
                )?;
            }
        }

        // Clone workflows with new project-scoped workflow IDs and build remap table
        let mut wf_id_map: HashMap<String, String> = HashMap::new();
        let mut workflow_files: Vec<String> = Vec::new();
        {
            let mut w_stmt = tx.prepare(
                "SELECT id, name, category, draft_json FROM workflows WHERE project_id = ?1 ORDER BY id ASC;",
            )?;
            let wfs: Vec<(String, String, String, String)> = w_stmt
                .query_map(params![src.id], |r| {
                    Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            for (idx, (old_wf_id, wf_name, cat, draft_json)) in wfs.into_iter().enumerate() {
                let new_wf_id = format!("wf-{}-{:02}", new_slug, idx + 1);
                wf_id_map.insert(old_wf_id.clone(), new_wf_id.clone());

                let updated_json = match serde_json::from_str::<serde_json::Value>(&draft_json) {
                    Ok(mut v) => {
                        if let Some(obj) = v.as_object_mut() {
                            obj.insert(
                                "workflowId".to_string(),
                                serde_json::Value::String(new_wf_id.clone()),
                            );
                        }
                        serde_json::to_string_pretty(&v).unwrap_or(draft_json)
                    }
                    Err(_) => draft_json,
                };
                let sha = sha256_hex(updated_json.as_bytes());
                tx.execute(
                    "INSERT INTO workflows (id, project_id, name, category, draft_json, draft_updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6);",
                    params![new_wf_id, new_id, wf_name, cat, updated_json, now],
                )?;
                tx.execute(
                    "INSERT INTO workflow_versions (id, workflow_id, version_no, graph_json, sha256, created_at)
                     VALUES (?1, ?2, 1, ?3, ?4, ?5);",
                    params![format!("{}-v1", new_wf_id), new_wf_id, updated_json, sha, now],
                )?;
                let rel_wf = format!("workflows/{}.json", new_wf_id);
                atomic_write_bytes(&staging_dir.join(&rel_wf), updated_json.as_bytes())?;
                workflow_files.push(rel_wf);
            }
        }

        // Clone vision_templates
        let mut template_files: Vec<String> = Vec::new();
        {
            let mut t_stmt = tx.prepare(
                "SELECT name, rel_path, width_px, height_px, source_crop_x, source_crop_y, default_confidence, sha256
                 FROM vision_templates WHERE project_id = ?1 ORDER BY rel_path ASC;",
            )?;
            let tpls: Vec<(
                String,
                String,
                i64,
                i64,
                Option<i32>,
                Option<i32>,
                f64,
                String,
            )> = t_stmt
                .query_map(params![src.id], |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                        r.get(6)?,
                        r.get(7)?,
                    ))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            for (idx, (name, rel_path, w, h, cx, cy, conf, sha)) in tpls.into_iter().enumerate() {
                let (clean_rel, _) = validate_safe_rel_path(&staging_dir, &rel_path)?;
                let new_tpl_id = format!("tpl-{}-{:02}", new_slug, idx + 1);
                tx.execute(
                    "INSERT INTO vision_templates (
                        id, project_id, name, rel_path, width_px, height_px,
                        source_crop_x, source_crop_y, default_confidence, sha256, created_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11);",
                    params![new_tpl_id, new_id, name, clean_rel, w, h, cx, cy, conf, sha, now],
                )?;
                template_files.push(clean_rel);
            }
        }

        // Clone task_definitions (remapping workflow_id to cloned workflow)
        {
            let mut tk_stmt = tx.prepare(
                "SELECT workflow_id, category_id, category_short_tag, category_name, category_color_hex,
                        title, priority, estimate_sec, default_enabled, settings_schema_json
                 FROM task_definitions WHERE project_id = ?1;",
            )?;
            let tasks: Vec<(
                String,
                String,
                String,
                String,
                String,
                String,
                i32,
                i64,
                i64,
                String,
            )> = tk_stmt
                .query_map(params![src.id], |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                        r.get(6)?,
                        r.get(7)?,
                        r.get(8)?,
                        r.get(9)?,
                    ))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            for (
                idx,
                (old_wf_id, cat_id, cat_tag, cat_name, cat_col, title, prio, est, def_en, sch),
            ) in tasks.into_iter().enumerate()
            {
                if let Some(mapped_wf_id) = wf_id_map.get(&old_wf_id) {
                    let new_task_id = format!("task-{}-{:02}", new_slug, idx + 1);
                    tx.execute(
                        "INSERT INTO task_definitions (
                            id, project_id, workflow_id, category_id, category_short_tag,
                            category_name, category_color_hex, title, priority, estimate_sec,
                            default_enabled, settings_schema_json, updated_at
                        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13);",
                        params![
                            new_task_id,
                            new_id,
                            mapped_wf_id,
                            cat_id,
                            cat_tag,
                            cat_name,
                            cat_col,
                            title,
                            prio,
                            est,
                            def_en,
                            sch,
                            now
                        ],
                    )?;
                }
            }
        }

        // Clone mini_tool_schemas with updated projectSlug & remapped task workflowIds
        let cloned_ui_opt: Option<(i64, String)> = {
            let mut u_stmt = tx.prepare(
                "SELECT schema_version, ui_schema_json FROM mini_tool_schemas WHERE project_id = ?1;",
            )?;
            let mut rows = u_stmt.query(params![src.id])?;
            rows.next()?
                .map(|r| Ok::<_, rusqlite::Error>((r.get(0)?, r.get(1)?)))
                .transpose()?
        };
        if let Some((ver, ui_json)) = cloned_ui_opt {
            let new_ui_json =
                if let Ok(mut parsed) = serde_json::from_str::<MiniToolUiSchema>(&ui_json) {
                    parsed.project_slug = new_slug.clone();
                    parsed.window_title = new_window_title.clone();
                    for t in &mut parsed.tasks {
                        if let Some(mapped) = wf_id_map.get(&t.workflow_id) {
                            t.workflow_id = mapped.clone();
                        }
                    }
                    serde_json::to_string_pretty(&parsed).unwrap_or(ui_json)
                } else {
                    ui_json
                };
            tx.execute(
                "INSERT INTO mini_tool_schemas (project_id, schema_version, ui_schema_json, updated_at)
                 VALUES (?1, ?2, ?3, ?4);",
                params![new_id, ver, new_ui_json, now],
            )?;
            atomic_write_bytes(&staging_dir.join("ui-schema.json"), new_ui_json.as_bytes())?;
        }

        if fault == CrashFaultPoint::FailManifestWrite {
            bail!("INJECTED_CRASH_FAULT: simulated manifest write failure during clone_project");
        }

        // Write cloned project.json manifest into staging dir
        let cloned_manifest = GameProjectManifest {
            compatibility: CompatibilityVersions::default(),
            project_id: new_id.clone(),
            slug: new_slug.clone(),
            name: new_name.clone(),
            game_package_id: new_pkg.clone(),
            version: src.version.clone(),
            exe_name: new_exe.clone(),
            window_title: new_window_title.clone(),
            icon_rel_path: src.icon_rel_path.clone(),
            target_resolution: (src.target_width_px, src.target_height_px),
            packaged_modules,
            workflow_files,
            template_files,
            bundle_adb: src.bundle_adb,
        };
        atomic_write_bytes(
            &staging_dir.join("project.json"),
            &serde_json::to_vec_pretty(&cloned_manifest)?,
        )?;

        let journal_root = projects_base_dir.join(JOURNAL_DIR_NAME);
        std::fs::create_dir_all(&journal_root)?;
        let clone_journal_path = journal_root.join(format!("create_{}.json", new_id));
        let clone_entry = CreateJournalEntry {
            op: "clone".to_string(),
            project_id: new_id.clone(),
            project_slug: new_slug.clone(),
            staging_dir: staging_dir.to_string_lossy().to_string(),
            target_dir: new_proj_dir.to_string_lossy().to_string(),
            created_at: now.clone(),
        };
        atomic_write_bytes(
            &clone_journal_path,
            &serde_json::to_vec_pretty(&clone_entry)?,
        )?;

        self.emit_crash_stage("clone:after_staging_before_rename");

        if fault == CrashFaultPoint::FailBeforeDbCommit {
            let _ = std::fs::remove_file(&clone_journal_path);
            bail!("INJECTED_CRASH_FAULT: simulated crash before DB commit in clone_project");
        }

        // Atomic move staging -> final project directory BEFORE commit
        std::fs::rename(&staging_dir, &new_proj_dir).with_context(|| {
            format!(
                "Failed moving cloned staging '{}' to '{}'",
                staging_dir.display(),
                new_proj_dir.display()
            )
        })?;
        staging_guard.0 = None;

        self.emit_crash_stage("clone:after_rename_before_db_commit");

        if let Err(e) = tx.commit() {
            let _ = std::fs::remove_dir_all(&new_proj_dir);
            let _ = std::fs::remove_file(&clone_journal_path);
            return Err(e.into());
        }

        self.emit_crash_stage("clone:after_db_commit_before_cleanup");
        let _ = std::fs::remove_file(&clone_journal_path);

        Ok(GameProjectRecord {
            id: new_id,
            slug: new_slug,
            name: new_name,
            game_package_id: new_pkg,
            version: src.version,
            exe_name: new_exe,
            window_title: new_window_title,
            icon_rel_path: src.icon_rel_path,
            target_width_px: src.target_width_px,
            target_height_px: src.target_height_px,
            bundle_adb: src.bundle_adb,
            project_dir_path: new_proj_dir_str,
            created_at: now.clone(),
            updated_at: now,
            publisher_profile_id: src.publisher_profile_id,
            signing_mode: src.signing_mode,
            version_info_json: src.version_info_json,
        })
    }

    pub fn delete_project(&self, project_id: &str, remove_disk_dir: bool) -> Result<bool> {
        self.delete_project_with_fault(project_id, remove_disk_dir, CrashFaultPoint::None)
    }

    pub fn delete_project_with_fault(
        &self,
        project_id: &str,
        remove_disk_dir: bool,
        fault: CrashFaultPoint,
    ) -> Result<bool> {
        let mut conn = self.conn.lock().unwrap();
        let existing = match Self::fetch_project_by_id_conn(&conn, project_id)? {
            Some(p) => p,
            None => return Ok(false),
        };

        // Two-phase crash-safe directory removal:
        // 1. Move project_dir -> .fp_trash/<slug>.<uuid> and write .fp_journal entry BEFORE DB commit
        // 2. Commit DB deletion; if DB commit fails, move .fp_trash/<...> back to project_dir!
        // 3. Purge .fp_trash/<...> & .fp_journal entry after DB commit succeeds.
        let mut moved_to_trash: Option<(PathBuf, PathBuf, Option<PathBuf>)> = None;
        if remove_disk_dir {
            let proj_dir = PathBuf::from(&existing.project_dir_path);
            if proj_dir.exists() {
                if is_reparse_or_symlink(&proj_dir) {
                    bail!(
                        "PROJECT_ISOLATION_VIOLATION: Refusing recursive delete on symlink/junction '{}'",
                        proj_dir.display()
                    );
                }
                if let Some(projects_root) = proj_dir.parent() {
                    let _ = ensure_contained_project_dir(projects_root, &proj_dir)?;
                    let trash_root = projects_root.join(TRASH_DIR_NAME);
                    let journal_root = projects_root.join(JOURNAL_DIR_NAME);
                    std::fs::create_dir_all(&trash_root)?;
                    std::fs::create_dir_all(&journal_root)?;

                    let tag = &uuid::Uuid::new_v4().to_string()[..8];
                    let trash_dir = trash_root.join(format!("{}.{}", existing.slug, tag));
                    let journal_path = journal_root.join(format!("delete_{}.json", existing.id));
                    let entry = DeleteJournalEntry {
                        project_id: existing.id.clone(),
                        project_slug: existing.slug.clone(),
                        original_dir: proj_dir.to_string_lossy().to_string(),
                        trash_dir: trash_dir.to_string_lossy().to_string(),
                        created_at: Utc::now().to_rfc3339(),
                    };
                    atomic_write_bytes(&journal_path, &serde_json::to_vec_pretty(&entry)?)?;
                    if let Err(e) = std::fs::rename(&proj_dir, &trash_dir) {
                        let _ = std::fs::remove_file(&journal_path);
                        return Err(e).with_context(|| {
                            format!(
                                "Failed moving project dir '{}' to trash before DB delete",
                                proj_dir.display()
                            )
                        });
                    }
                    moved_to_trash = Some((proj_dir, trash_dir, Some(journal_path)));
                    self.emit_crash_stage("delete:after_move_to_trash_before_commit");
                }
            }
        }

        let restore_from_trash = |moved: &Option<(PathBuf, PathBuf, Option<PathBuf>)>| {
            if let Some((orig, trash, journal)) = moved {
                if trash.exists() && !orig.exists() {
                    let _ = std::fs::rename(trash, orig);
                }
                if let Some(jp) = journal {
                    let _ = std::fs::remove_file(jp);
                }
            }
        };

        let tx_res: Result<()> = (|| {
            let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            tx.execute(
                "DELETE FROM step_events WHERE job_id IN (
                    SELECT id FROM jobs WHERE project_id = ?1
                    OR workflow_version_id IN (SELECT id FROM workflow_versions WHERE workflow_id IN (SELECT id FROM workflows WHERE project_id = ?1))
                 );",
                params![project_id],
            )?;
            tx.execute(
                "DELETE FROM checkpoints WHERE job_id IN (
                    SELECT id FROM jobs WHERE project_id = ?1
                    OR workflow_version_id IN (SELECT id FROM workflow_versions WHERE workflow_id IN (SELECT id FROM workflows WHERE project_id = ?1))
                 );",
                params![project_id],
            )?;
            tx.execute(
                "DELETE FROM device_leases WHERE job_id IN (
                    SELECT id FROM jobs WHERE project_id = ?1
                    OR workflow_version_id IN (SELECT id FROM workflow_versions WHERE workflow_id IN (SELECT id FROM workflows WHERE project_id = ?1))
                 );",
                params![project_id],
            )?;
            tx.execute(
                "DELETE FROM jobs WHERE project_id = ?1
                 OR workflow_version_id IN (SELECT id FROM workflow_versions WHERE workflow_id IN (SELECT id FROM workflows WHERE project_id = ?1));",
                params![project_id],
            )?;
            tx.execute(
                "DELETE FROM task_definitions WHERE project_id = ?1;",
                params![project_id],
            )?;
            tx.execute(
                "DELETE FROM workflow_versions WHERE workflow_id IN (SELECT id FROM workflows WHERE project_id = ?1);",
                params![project_id],
            )?;
            tx.execute(
                "DELETE FROM workflows WHERE project_id = ?1;",
                params![project_id],
            )?;
            tx.execute(
                "DELETE FROM accounts WHERE project_id = ?1;",
                params![project_id],
            )?;
            tx.execute(
                "DELETE FROM project_modules WHERE project_id = ?1;",
                params![project_id],
            )?;
            tx.execute(
                "DELETE FROM vision_templates WHERE project_id = ?1;",
                params![project_id],
            )?;
            tx.execute(
                "DELETE FROM mini_tool_schemas WHERE project_id = ?1;",
                params![project_id],
            )?;
            tx.execute(
                "DELETE FROM project_releases WHERE project_id = ?1;",
                params![project_id],
            )?;
            tx.execute("DELETE FROM projects WHERE id = ?1;", params![project_id])?;

            if fault == CrashFaultPoint::FailBeforeDbCommit {
                bail!("INJECTED_CRASH_FAULT: simulated crash before DB commit in delete_project");
            }
            tx.commit()?;
            Ok(())
        })();

        if let Err(e) = tx_res {
            restore_from_trash(&moved_to_trash);
            return Err(e);
        }

        self.emit_crash_stage("delete:after_db_commit_before_cleanup");

        if fault == CrashFaultPoint::FailAfterDbCommitTrashPurge {
            bail!("INJECTED_CRASH_FAULT: simulated crash after DB commit before trash purge");
        }

        if let Some((_, trash, journal)) = moved_to_trash {
            if trash.exists() {
                let _ = std::fs::remove_dir_all(&trash);
            }
            if let Some(jp) = journal {
                let _ = std::fs::remove_file(jp);
            }
        }
        Ok(true)
    }

    // -------------------------------------------------------------------------
    // Project Modules
    // -------------------------------------------------------------------------

    pub fn list_project_modules(&self, project_id: &str) -> Result<Vec<ProjectModuleRecord>> {
        if project_id.trim().is_empty() {
            bail!("PROJECT_ISOLATION_VIOLATION: project_id cannot be empty");
        }
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT project_id, module_id, packaged_in_release, enabled_by_default, config_json, updated_at
             FROM project_modules WHERE project_id = ?1 AND project_id IS NOT NULL ORDER BY module_id ASC;",
        )?;
        let rows = stmt.query_map(params![project_id], |r| {
            Ok(ProjectModuleRecord {
                project_id: r.get(0)?,
                module_id: r.get(1)?,
                packaged_in_release: r.get::<_, i64>(2)? != 0,
                enabled_by_default: r.get::<_, i64>(3)? != 0,
                config_json: r.get(4)?,
                updated_at: r.get(5)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn upsert_project_module(
        &self,
        project_id: &str,
        module_id: &str,
        packaged_in_release: bool,
        enabled_by_default: bool,
        config_json: &str,
    ) -> Result<ProjectModuleRecord> {
        let conn = self.conn.lock().unwrap();
        let _ = Self::fetch_project_by_id_conn(&conn, project_id)?
            .with_context(|| format!("Project '{}' not found", project_id))?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO project_modules (project_id, module_id, packaged_in_release, enabled_by_default, config_json, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(project_id, module_id) DO UPDATE SET
                packaged_in_release = excluded.packaged_in_release,
                enabled_by_default = excluded.enabled_by_default,
                config_json = excluded.config_json,
                updated_at = excluded.updated_at;",
            params![
                project_id,
                module_id,
                if packaged_in_release { 1 } else { 0 },
                if enabled_by_default { 1 } else { 0 },
                config_json,
                now
            ],
        )?;
        Ok(ProjectModuleRecord {
            project_id: project_id.to_string(),
            module_id: module_id.to_string(),
            packaged_in_release,
            enabled_by_default,
            config_json: config_json.to_string(),
            updated_at: now,
        })
    }

    // -------------------------------------------------------------------------
    // Project-Scoped Workflows (Strict Isolation)
    // -------------------------------------------------------------------------

    pub fn list_project_workflows(&self, project_id: &str) -> Result<Vec<ProjectWorkflowRecord>> {
        if project_id.trim().is_empty() {
            bail!("PROJECT_ISOLATION_VIOLATION: project_id cannot be empty");
        }
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT w.id, w.project_id, w.name, w.category, w.draft_json,
                    w.draft_updated_at, w.archived_at,
                    (SELECT MAX(version_no) FROM workflow_versions v WHERE v.workflow_id = w.id)
             FROM workflows w
             WHERE w.project_id = ?1 AND w.project_id IS NOT NULL
             ORDER BY w.id ASC;",
        )?;
        let rows = stmt.query_map(params![project_id], |r| {
            Ok(ProjectWorkflowRecord {
                id: r.get(0)?,
                project_id: r.get(1)?,
                name: r.get(2)?,
                category: r.get(3)?,
                draft_json: r.get(4)?,
                draft_updated_at: r.get(5)?,
                archived_at: r.get(6)?,
                latest_version_no: r.get(7)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn save_project_workflow(
        &self,
        project_id: &str,
        workflow_id: &str,
        name: &str,
        category: &str,
        draft_json: &str,
    ) -> Result<ProjectWorkflowRecord> {
        self.save_project_workflow_with_fault(
            project_id,
            workflow_id,
            name,
            category,
            draft_json,
            CrashFaultPoint::None,
        )
    }

    pub fn save_project_workflow_with_fault(
        &self,
        project_id: &str,
        workflow_id: &str,
        name: &str,
        category: &str,
        draft_json: &str,
        fault: CrashFaultPoint,
    ) -> Result<ProjectWorkflowRecord> {
        let pid_clean = project_id.trim();
        let wf_id_clean = workflow_id.trim();
        let name_clean = name.trim();
        if pid_clean.is_empty() {
            bail!("PROJECT_ISOLATION_VIOLATION: project_id cannot be empty");
        }
        if wf_id_clean.is_empty()
            || wf_id_clean.contains('/')
            || wf_id_clean.contains('\\')
            || wf_id_clean.contains("..")
        {
            bail!("Invalid workflow ID '{}'", workflow_id);
        }
        if name_clean.is_empty() {
            bail!("Workflow name cannot be empty");
        }

        let mut conn = self.conn.lock().unwrap();
        let project = Self::fetch_project_by_id_conn(&conn, pid_clean)?
            .with_context(|| format!("Project '{}' not found", pid_clean))?;

        let existing_owner: Option<Option<String>> = {
            let mut st = conn.prepare("SELECT project_id FROM workflows WHERE id = ?1;")?;
            let mut rows = st.query(params![wf_id_clean])?;
            rows.next()?.map(|r| r.get(0)).transpose()?
        };

        match existing_owner {
            Some(Some(ref owner)) if owner != pid_clean => {
                bail!(
                    "PROJECT_ISOLATION_VIOLATION: Workflow '{}' belongs to project '{}', cannot modify from project '{}'",
                    workflow_id,
                    owner,
                    pid_clean
                );
            }
            Some(None) => {
                bail!(
                    "PROJECT_ISOLATION_VIOLATION: Workflow '{}' has NULL project ownership",
                    workflow_id
                );
            }
            _ => {}
        }

        let now = Utc::now().to_rfc3339();
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute(
            "INSERT INTO workflows (id, project_id, name, category, draft_json, draft_updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
                project_id = excluded.project_id,
                name = excluded.name,
                category = excluded.category,
                draft_json = excluded.draft_json,
                draft_updated_at = excluded.draft_updated_at;",
            params![
                wf_id_clean,
                pid_clean,
                name_clean,
                category,
                draft_json,
                now
            ],
        )?;

        let max_ver: Option<i64> = tx.query_row(
            "SELECT MAX(version_no) FROM workflow_versions WHERE workflow_id = ?1;",
            params![wf_id_clean],
            |r| r.get(0),
        )?;
        let next_ver = max_ver.unwrap_or(0) + 1;
        let ver_id = format!("{}-v{}", wf_id_clean, next_ver);
        let sha = sha256_hex(draft_json.as_bytes());
        tx.execute(
            "INSERT INTO workflow_versions (id, workflow_id, version_no, graph_json, sha256, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6);",
            params![ver_id, wf_id_clean, next_ver, draft_json, sha, now],
        )?;

        // Sync workflow JSON & project.json on disk atomically with pre-image intent journal + rollback
        let proj_dir = Path::new(&project.project_dir_path);
        let rel_wf = format!("workflows/{}.json", wf_id_clean);
        let mut disk_rollback: Option<(PathBuf, Option<Vec<u8>>, PathBuf, Option<Vec<u8>>)> = None;
        let mut mutation_journal: Option<(PathBuf, PathBuf)> = None;
        if proj_dir.exists() {
            if is_reparse_or_symlink(proj_dir) {
                bail!(
                    "PROJECT_ISOLATION_VIOLATION: Refusing workflow save on symlinked project dir '{}'",
                    proj_dir.display()
                );
            }
            let (norm_rel, wf_disk_path) = validate_safe_rel_path(proj_dir, &rel_wf)?;
            let manifest_path = proj_dir.join("project.json");
            let prev_wf_bytes = if wf_disk_path.exists() {
                Some(std::fs::read(&wf_disk_path)?)
            } else {
                None
            };
            let prev_manifest_bytes = if manifest_path.exists() {
                Some(std::fs::read(&manifest_path)?)
            } else {
                None
            };

            mutation_journal = begin_mutation_journal(
                proj_dir,
                pid_clean,
                &[wf_disk_path.as_path(), manifest_path.as_path()],
                MutationCommitWitness::WorkflowVersionExists {
                    project_id: pid_clean.to_string(),
                    workflow_id: wf_id_clean.to_string(),
                    version_id: ver_id.clone(),
                    expected_sha256: sha.clone(),
                },
                &now,
            )?;

            let restore_wf_disk =
                |wf_p: &Path, prev_wf: &Option<Vec<u8>>, mf_p: &Path, prev_mf: &Option<Vec<u8>>| {
                    match prev_wf {
                        Some(b) => {
                            let _ = atomic_write_bytes(wf_p, b);
                        }
                        None => {
                            let _ = std::fs::remove_file(wf_p);
                        }
                    }
                    if let Some(b) = prev_mf {
                        let _ = atomic_write_bytes(mf_p, b);
                    }
                };

            if let Err(e) = atomic_write_bytes(&wf_disk_path, draft_json.as_bytes()) {
                restore_wf_disk(
                    &wf_disk_path,
                    &prev_wf_bytes,
                    &manifest_path,
                    &prev_manifest_bytes,
                );
                cleanup_mutation_journal(mutation_journal);
                return Err(e);
            }

            self.emit_crash_stage("workflow:after_wf_write_before_manifest");

            if fault == CrashFaultPoint::FailManifestWrite {
                restore_wf_disk(
                    &wf_disk_path,
                    &prev_wf_bytes,
                    &manifest_path,
                    &prev_manifest_bytes,
                );
                cleanup_mutation_journal(mutation_journal);
                bail!("INJECTED_CRASH_FAULT: simulated manifest write failure during save_project_workflow");
            }

            if let Some(ref raw_mf) = prev_manifest_bytes {
                if let Ok(mut manifest) = serde_json::from_slice::<GameProjectManifest>(raw_mf) {
                    if !manifest.workflow_files.iter().any(|f| f == &norm_rel) {
                        manifest.workflow_files.push(norm_rel);
                        manifest.workflow_files.sort();
                        if let Err(e) = atomic_write_bytes(
                            &manifest_path,
                            &serde_json::to_vec_pretty(&manifest)?,
                        ) {
                            restore_wf_disk(
                                &wf_disk_path,
                                &prev_wf_bytes,
                                &manifest_path,
                                &prev_manifest_bytes,
                            );
                            cleanup_mutation_journal(mutation_journal);
                            return Err(e);
                        }
                    }
                }
            }
            disk_rollback = Some((
                wf_disk_path,
                prev_wf_bytes,
                manifest_path,
                prev_manifest_bytes,
            ));
        }

        self.emit_crash_stage("workflow:after_disk_writes_before_db_commit");

        if fault == CrashFaultPoint::FailBeforeDbCommit {
            if let Some((wf_p, prev_wf, mf_p, prev_mf)) = disk_rollback {
                match prev_wf {
                    Some(b) => {
                        let _ = atomic_write_bytes(&wf_p, &b);
                    }
                    None => {
                        let _ = std::fs::remove_file(&wf_p);
                    }
                }
                if let Some(b) = prev_mf {
                    let _ = atomic_write_bytes(&mf_p, &b);
                }
            }
            cleanup_mutation_journal(mutation_journal);
            bail!(
                "INJECTED_CRASH_FAULT: simulated crash before DB commit in save_project_workflow"
            );
        }

        if let Err(e) = tx.commit() {
            if let Some((wf_p, prev_wf, mf_p, prev_mf)) = disk_rollback {
                match prev_wf {
                    Some(b) => {
                        let _ = atomic_write_bytes(&wf_p, &b);
                    }
                    None => {
                        let _ = std::fs::remove_file(&wf_p);
                    }
                }
                if let Some(b) = prev_mf {
                    let _ = atomic_write_bytes(&mf_p, &b);
                }
            }
            cleanup_mutation_journal(mutation_journal);
            return Err(e.into());
        }

        self.emit_crash_stage("workflow:after_db_commit_before_cleanup");
        cleanup_mutation_journal(mutation_journal);

        Ok(ProjectWorkflowRecord {
            id: wf_id_clean.to_string(),
            project_id: pid_clean.to_string(),
            name: name_clean.to_string(),
            category: category.to_string(),
            draft_json: draft_json.to_string(),
            draft_updated_at: now,
            archived_at: None,
            latest_version_no: Some(next_ver),
        })
    }

    pub fn delete_project_workflow(&self, project_id: &str, workflow_id: &str) -> Result<bool> {
        if project_id.trim().is_empty() {
            bail!("PROJECT_ISOLATION_VIOLATION: project_id cannot be empty");
        }
        let mut conn = self.conn.lock().unwrap();
        let owner: Option<Option<String>> = {
            let mut st = conn.prepare("SELECT project_id FROM workflows WHERE id = ?1;")?;
            let mut rows = st.query(params![workflow_id])?;
            rows.next()?.map(|r| r.get(0)).transpose()?
        };
        match owner {
            None => return Ok(false),
            Some(Some(ref o)) if o != project_id => {
                bail!(
                    "PROJECT_ISOLATION_VIOLATION: Workflow '{}' belongs to project '{}', cannot delete from '{}'",
                    workflow_id,
                    o,
                    project_id
                );
            }
            _ => {}
        }

        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute(
            "DELETE FROM task_definitions WHERE project_id = ?1 AND workflow_id = ?2;",
            params![project_id, workflow_id],
        )?;
        tx.execute(
            "DELETE FROM workflow_versions WHERE workflow_id = ?1;",
            params![workflow_id],
        )?;
        let affected = tx.execute(
            "DELETE FROM workflows WHERE id = ?1 AND project_id = ?2;",
            params![workflow_id, project_id],
        )?;
        tx.commit()?;
        Ok(affected > 0)
    }

    // -------------------------------------------------------------------------
    // Vision Templates (Strict Isolation)
    // -------------------------------------------------------------------------

    pub fn register_vision_template(
        &self,
        input: VisionTemplateInput,
    ) -> Result<VisionTemplateRecord> {
        let conn = self.conn.lock().unwrap();
        let project = Self::fetch_project_by_id_conn(&conn, &input.project_id)?
            .with_context(|| format!("Project '{}' not found", input.project_id))?;

        let proj_dir = Path::new(&project.project_dir_path);
        if is_reparse_or_symlink(proj_dir) {
            bail!(
                "PROJECT_ISOLATION_VIOLATION: Refusing template registration on symlinked project dir '{}'",
                proj_dir.display()
            );
        }
        let (rel_clean, _) = validate_safe_rel_path(proj_dir, &input.rel_path)?;

        let id = input
            .id
            .unwrap_or_else(|| format!("tpl-{}", &uuid::Uuid::new_v4().to_string()[..8]));
        let conf = input.default_confidence.unwrap_or(0.88);
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO vision_templates (
                id, project_id, name, rel_path, width_px, height_px,
                source_crop_x, source_crop_y, default_confidence, sha256, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(project_id, rel_path) DO UPDATE SET
                name = excluded.name,
                width_px = excluded.width_px,
                height_px = excluded.height_px,
                source_crop_x = excluded.source_crop_x,
                source_crop_y = excluded.source_crop_y,
                default_confidence = excluded.default_confidence,
                sha256 = excluded.sha256;",
            params![
                id,
                input.project_id,
                input.name,
                rel_clean,
                input.width_px as i64,
                input.height_px as i64,
                input.source_crop_x,
                input.source_crop_y,
                conf,
                input.sha256,
                now
            ],
        )?;

        let actual_id: String = conn.query_row(
            "SELECT id FROM vision_templates WHERE project_id = ?1 AND rel_path = ?2;",
            params![input.project_id, rel_clean],
            |r| r.get(0),
        )?;

        Ok(VisionTemplateRecord {
            id: actual_id,
            project_id: input.project_id,
            name: input.name,
            rel_path: rel_clean,
            width_px: input.width_px,
            height_px: input.height_px,
            source_crop_x: input.source_crop_x,
            source_crop_y: input.source_crop_y,
            default_confidence: conf,
            sha256: input.sha256,
            created_at: now,
        })
    }

    pub fn save_project_template_file(
        &self,
        input: VisionTemplateInput,
        png_bytes: &[u8],
    ) -> Result<VisionTemplateRecord> {
        self.save_project_template_file_with_fault(input, png_bytes, CrashFaultPoint::None)
    }

    /// Crash-safe save of template image bytes + SQLite `vision_templates` + `project.json` manifest.
    pub fn save_project_template_file_with_fault(
        &self,
        input: VisionTemplateInput,
        png_bytes: &[u8],
        fault: CrashFaultPoint,
    ) -> Result<VisionTemplateRecord> {
        if png_bytes.is_empty() {
            bail!("Template image bytes cannot be empty");
        }
        let mut conn = self.conn.lock().unwrap();
        let project = Self::fetch_project_by_id_conn(&conn, &input.project_id)?
            .with_context(|| format!("Project '{}' not found", input.project_id))?;

        let proj_dir = Path::new(&project.project_dir_path);
        if is_reparse_or_symlink(proj_dir) {
            bail!(
                "PROJECT_ISOLATION_VIOLATION: Refusing template save on symlinked project dir '{}'",
                proj_dir.display()
            );
        }
        let (rel_clean, target_path) = validate_safe_rel_path(proj_dir, &input.rel_path)?;
        let actual_sha = sha256_hex(png_bytes);
        let id = input
            .id
            .unwrap_or_else(|| format!("tpl-{}", &uuid::Uuid::new_v4().to_string()[..8]));
        let conf = input.default_confidence.unwrap_or(0.88);
        let now = Utc::now().to_rfc3339();

        let prev_tpl_bytes = if target_path.exists() {
            Some(std::fs::read(&target_path)?)
        } else {
            None
        };
        let manifest_path = proj_dir.join("project.json");
        let prev_manifest_bytes = if manifest_path.exists() {
            Some(std::fs::read(&manifest_path)?)
        } else {
            None
        };

        let restore_tpl_disk = || {
            match prev_tpl_bytes {
                Some(ref b) => {
                    let _ = atomic_write_bytes(&target_path, b);
                }
                None => {
                    let _ = std::fs::remove_file(&target_path);
                }
            }
            if let Some(ref b) = prev_manifest_bytes {
                let _ = atomic_write_bytes(&manifest_path, b);
            }
        };

        let mutation_journal = begin_mutation_journal(
            proj_dir,
            &input.project_id,
            &[target_path.as_path(), manifest_path.as_path()],
            MutationCommitWitness::VisionTemplateExists {
                project_id: input.project_id.clone(),
                rel_path: rel_clean.clone(),
                expected_sha256: actual_sha.clone(),
            },
            &now,
        )?;

        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute(
            "INSERT INTO vision_templates (
                id, project_id, name, rel_path, width_px, height_px,
                source_crop_x, source_crop_y, default_confidence, sha256, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(project_id, rel_path) DO UPDATE SET
                name = excluded.name,
                width_px = excluded.width_px,
                height_px = excluded.height_px,
                source_crop_x = excluded.source_crop_x,
                source_crop_y = excluded.source_crop_y,
                default_confidence = excluded.default_confidence,
                sha256 = excluded.sha256;",
            params![
                id,
                input.project_id,
                input.name,
                rel_clean,
                input.width_px as i64,
                input.height_px as i64,
                input.source_crop_x,
                input.source_crop_y,
                conf,
                actual_sha,
                now
            ],
        )?;

        if fault == CrashFaultPoint::FailTemplateCopy {
            cleanup_mutation_journal(mutation_journal);
            bail!("INJECTED_CRASH_FAULT: simulated template write failure during save_project_template_file");
        }
        if let Err(e) = atomic_write_bytes(&target_path, png_bytes) {
            restore_tpl_disk();
            cleanup_mutation_journal(mutation_journal);
            return Err(e);
        }

        if fault == CrashFaultPoint::FailManifestWrite {
            restore_tpl_disk();
            cleanup_mutation_journal(mutation_journal);
            bail!("INJECTED_CRASH_FAULT: simulated manifest write failure during save_project_template_file");
        }
        if let Some(ref raw_mf) = prev_manifest_bytes {
            if let Ok(mut manifest) = serde_json::from_slice::<GameProjectManifest>(raw_mf) {
                if !manifest.template_files.iter().any(|f| f == &rel_clean) {
                    manifest.template_files.push(rel_clean.clone());
                    manifest.template_files.sort();
                    if let Err(e) =
                        atomic_write_bytes(&manifest_path, &serde_json::to_vec_pretty(&manifest)?)
                    {
                        restore_tpl_disk();
                        cleanup_mutation_journal(mutation_journal);
                        return Err(e);
                    }
                }
            }
        }

        self.emit_crash_stage("template:after_disk_writes_before_db_commit");

        if fault == CrashFaultPoint::FailBeforeDbCommit {
            restore_tpl_disk();
            cleanup_mutation_journal(mutation_journal);
            bail!("INJECTED_CRASH_FAULT: simulated crash before DB commit in save_project_template_file");
        }

        if let Err(e) = tx.commit() {
            restore_tpl_disk();
            cleanup_mutation_journal(mutation_journal);
            return Err(e.into());
        }

        self.emit_crash_stage("template:after_db_commit_before_cleanup");
        cleanup_mutation_journal(mutation_journal);

        let actual_id: String = conn.query_row(
            "SELECT id FROM vision_templates WHERE project_id = ?1 AND rel_path = ?2;",
            params![input.project_id, rel_clean],
            |r| r.get(0),
        )?;

        Ok(VisionTemplateRecord {
            id: actual_id,
            project_id: input.project_id,
            name: input.name,
            rel_path: rel_clean,
            width_px: input.width_px,
            height_px: input.height_px,
            source_crop_x: input.source_crop_x,
            source_crop_y: input.source_crop_y,
            default_confidence: conf,
            sha256: actual_sha,
            created_at: now,
        })
    }

    pub fn list_vision_templates(&self, project_id: &str) -> Result<Vec<VisionTemplateRecord>> {
        if project_id.trim().is_empty() {
            bail!("PROJECT_ISOLATION_VIOLATION: project_id cannot be empty");
        }
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, name, rel_path, width_px, height_px,
                    source_crop_x, source_crop_y, default_confidence, sha256, created_at
             FROM vision_templates WHERE project_id = ?1 AND project_id IS NOT NULL ORDER BY rel_path ASC;",
        )?;
        let rows = stmt.query_map(params![project_id], |r| {
            Ok(VisionTemplateRecord {
                id: r.get(0)?,
                project_id: r.get(1)?,
                name: r.get(2)?,
                rel_path: r.get(3)?,
                width_px: r.get::<_, i64>(4)? as u32,
                height_px: r.get::<_, i64>(5)? as u32,
                source_crop_x: r.get(6)?,
                source_crop_y: r.get(7)?,
                default_confidence: r.get(8)?,
                sha256: r.get(9)?,
                created_at: r.get(10)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn delete_vision_template(&self, project_id: &str, template_id: &str) -> Result<bool> {
        if project_id.trim().is_empty() {
            bail!("PROJECT_ISOLATION_VIOLATION: project_id cannot be empty");
        }
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute(
            "DELETE FROM vision_templates WHERE id = ?1 AND project_id = ?2;",
            params![template_id, project_id],
        )?;
        Ok(affected > 0)
    }

    // -------------------------------------------------------------------------
    // Task Definitions (Strict Isolation)
    // -------------------------------------------------------------------------

    pub fn list_project_tasks(&self, project_id: &str) -> Result<Vec<ProjectTaskRecord>> {
        if project_id.trim().is_empty() {
            bail!("PROJECT_ISOLATION_VIOLATION: project_id cannot be empty");
        }
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, workflow_id, category_id, category_short_tag,
                    category_name, category_color_hex, title, priority, estimate_sec,
                    default_enabled, settings_schema_json, updated_at
             FROM task_definitions WHERE project_id = ?1 AND project_id IS NOT NULL ORDER BY priority ASC, id ASC;",
        )?;
        let rows = stmt.query_map(params![project_id], |r| {
            Ok(ProjectTaskRecord {
                id: r.get(0)?,
                project_id: r.get(1)?,
                workflow_id: r.get(2)?,
                category_id: r.get(3)?,
                category_short_tag: r.get(4)?,
                category_name: r.get(5)?,
                category_color_hex: r.get(6)?,
                title: r.get(7)?,
                priority: r.get(8)?,
                estimate_sec: r.get::<_, i64>(9)? as u32,
                default_enabled: r.get::<_, i64>(10)? != 0,
                settings_schema_json: r.get(11)?,
                updated_at: r.get(12)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn upsert_project_task(
        &self,
        project_id: &str,
        task: MiniTaskDef,
        category_meta: &TaskCategoryDef,
    ) -> Result<ProjectTaskRecord> {
        let conn = self.conn.lock().unwrap();
        let _ = Self::fetch_project_by_id_conn(&conn, project_id)?
            .with_context(|| format!("Project '{}' not found", project_id))?;

        // Enforce strict isolation: workflow_id MUST belong to the same project_id!
        let wf_owner: Option<Option<String>> = {
            let mut st = conn.prepare("SELECT project_id FROM workflows WHERE id = ?1;")?;
            let mut rows = st.query(params![task.workflow_id])?;
            rows.next()?.map(|r| r.get(0)).transpose()?
        };
        match wf_owner {
            None => bail!("Workflow '{}' does not exist", task.workflow_id),
            Some(Some(owner)) if owner != project_id => {
                bail!(
                    "PROJECT_ISOLATION_VIOLATION: Task '{}' cannot reference workflow '{}' owned by project '{}'",
                    task.id,
                    task.workflow_id,
                    owner
                );
            }
            Some(None) => {
                bail!(
                    "PROJECT_ISOLATION_VIOLATION: Workflow '{}' has no project ownership",
                    task.workflow_id
                );
            }
            _ => {}
        }

        // Also ensure task.id isn't owned by another project
        let existing_task_owner: Option<String> = {
            let mut st = conn.prepare("SELECT project_id FROM task_definitions WHERE id = ?1;")?;
            let mut rows = st.query(params![task.id])?;
            rows.next()?.map(|r| r.get(0)).transpose()?
        };
        if let Some(owner) = existing_task_owner {
            if owner != project_id {
                bail!(
                    "PROJECT_ISOLATION_VIOLATION: Task '{}' belongs to project '{}'",
                    task.id,
                    owner
                );
            }
        }

        let settings_json = serde_json::to_string(&task.settings_schema)?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO task_definitions (
                id, project_id, workflow_id, category_id, category_short_tag,
                category_name, category_color_hex, title, priority, estimate_sec,
                default_enabled, settings_schema_json, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(id) DO UPDATE SET
                workflow_id = excluded.workflow_id,
                category_id = excluded.category_id,
                category_short_tag = excluded.category_short_tag,
                category_name = excluded.category_name,
                category_color_hex = excluded.category_color_hex,
                title = excluded.title,
                priority = excluded.priority,
                estimate_sec = excluded.estimate_sec,
                default_enabled = excluded.default_enabled,
                settings_schema_json = excluded.settings_schema_json,
                updated_at = excluded.updated_at;",
            params![
                task.id,
                project_id,
                task.workflow_id,
                category_meta.id,
                category_meta.short_tag,
                category_meta.display_name,
                category_meta.color_hex,
                task.title,
                task.priority,
                task.estimate_sec as i64,
                if task.default_enabled { 1 } else { 0 },
                settings_json,
                now
            ],
        )?;

        Ok(ProjectTaskRecord {
            id: task.id,
            project_id: project_id.to_string(),
            workflow_id: task.workflow_id,
            category_id: category_meta.id.clone(),
            category_short_tag: category_meta.short_tag.clone(),
            category_name: category_meta.display_name.clone(),
            category_color_hex: category_meta.color_hex.clone(),
            title: task.title,
            priority: task.priority,
            estimate_sec: task.estimate_sec,
            default_enabled: task.default_enabled,
            settings_schema_json: settings_json,
            updated_at: now,
        })
    }

    pub fn delete_project_task(&self, project_id: &str, task_id: &str) -> Result<bool> {
        if project_id.trim().is_empty() {
            bail!("PROJECT_ISOLATION_VIOLATION: project_id cannot be empty");
        }
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute(
            "DELETE FROM task_definitions WHERE id = ?1 AND project_id = ?2;",
            params![task_id, project_id],
        )?;
        Ok(affected > 0)
    }

    // -------------------------------------------------------------------------
    // Mini Tool UI Schema
    // -------------------------------------------------------------------------

    pub fn get_ui_schema(&self, project_id: &str) -> Result<Option<MiniToolUiSchema>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT ui_schema_json FROM mini_tool_schemas WHERE project_id = ?1;")?;
        let mut rows = stmt.query(params![project_id])?;
        if let Some(row) = rows.next()? {
            let json_str: String = row.get(0)?;
            let parsed: MiniToolUiSchema = serde_json::from_str(&json_str)?;
            Ok(Some(parsed))
        } else {
            Ok(None)
        }
    }

    pub fn save_ui_schema(&self, project_id: &str, schema: &MiniToolUiSchema) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let project = Self::fetch_project_by_id_conn(&conn, project_id)?
            .with_context(|| format!("Project '{}' not found", project_id))?;

        if schema.project_slug != project.slug {
            bail!(
                "PROJECT_ISOLATION_VIOLATION: UI schema slug '{}' does not match project '{}' slug '{}'",
                schema.project_slug,
                project_id,
                project.slug
            );
        }

        let json_str = serde_json::to_string_pretty(schema)?;
        let now = Utc::now().to_rfc3339();
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute(
            "INSERT INTO mini_tool_schemas (project_id, schema_version, ui_schema_json, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(project_id) DO UPDATE SET
                schema_version = excluded.schema_version,
                ui_schema_json = excluded.ui_schema_json,
                updated_at = excluded.updated_at;",
            params![project_id, schema.schema_version as i64, json_str, now],
        )?;

        let disk_dir = Path::new(&project.project_dir_path);
        let ui_path = disk_dir.join("ui-schema.json");
        let prev_ui = if ui_path.exists() {
            Some(std::fs::read(&ui_path)?)
        } else {
            None
        };
        if disk_dir.exists() {
            atomic_write_bytes(&ui_path, json_str.as_bytes())?;
        }
        if let Err(e) = tx.commit() {
            if let Some(b) = prev_ui {
                let _ = atomic_write_bytes(&ui_path, &b);
            }
            return Err(e.into());
        }
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Project Accounts, Jobs, Releases & Detail
    // -------------------------------------------------------------------------

    pub fn upsert_project_account(
        &self,
        input: ProjectAccountInput,
    ) -> Result<ProjectAccountRecord> {
        let conn = self.conn.lock().unwrap();
        let _ = Self::fetch_project_by_id_conn(&conn, &input.project_id)?
            .with_context(|| format!("Project '{}' not found", input.project_id))?;

        let id = input
            .id
            .unwrap_or_else(|| format!("acc-{}", &uuid::Uuid::new_v4().to_string()[..8]));
        let now = Utc::now().to_rfc3339();

        let existing_owner: Option<Option<String>> = {
            let mut st = conn.prepare("SELECT project_id FROM accounts WHERE id = ?1;")?;
            let mut rows = st.query(params![id])?;
            rows.next()?.map(|r| r.get(0)).transpose()?
        };
        if let Some(Some(owner)) = existing_owner {
            if owner != input.project_id {
                bail!(
                    "PROJECT_ISOLATION_VIOLATION: Account '{}' belongs to project '{}'",
                    id,
                    owner
                );
            }
        }

        conn.execute(
            "INSERT INTO accounts (id, project_id, alias, game_profile_id, secret_ref, state, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
                alias = excluded.alias,
                game_profile_id = excluded.game_profile_id,
                secret_ref = excluded.secret_ref,
                state = excluded.state,
                updated_at = excluded.updated_at;",
            params![
                id,
                input.project_id,
                input.alias,
                input.game_profile_id,
                input.secret_ref,
                input.state,
                now
            ],
        )?;

        Ok(ProjectAccountRecord {
            id,
            project_id: input.project_id,
            alias: input.alias,
            game_profile_id: input.game_profile_id,
            secret_ref: input.secret_ref,
            state: input.state,
            updated_at: now,
        })
    }

    pub fn list_project_accounts(&self, project_id: &str) -> Result<Vec<ProjectAccountRecord>> {
        if project_id.trim().is_empty() {
            bail!("PROJECT_ISOLATION_VIOLATION: project_id cannot be empty");
        }
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, alias, game_profile_id, secret_ref, state, updated_at
             FROM accounts WHERE project_id = ?1 AND project_id IS NOT NULL ORDER BY updated_at DESC;",
        )?;
        let rows = stmt.query_map(params![project_id], |r| {
            Ok(ProjectAccountRecord {
                id: r.get(0)?,
                project_id: r.get(1)?,
                alias: r.get(2)?,
                game_profile_id: r.get(3)?,
                secret_ref: r.get(4)?,
                state: r.get(5)?,
                updated_at: r.get(6)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn ensure_project_job_exists(
        &self,
        project_id: &str,
        job_id: &str,
        device_id: &str,
        workflow_id: &str,
        workflow_name: &str,
        graph_json: &str,
    ) -> Result<()> {
        self.ensure_job_exists_with_project(
            Some(project_id),
            job_id,
            device_id,
            workflow_id,
            workflow_name,
            graph_json,
        )
    }

    pub fn list_project_jobs(&self, project_id: &str) -> Result<Vec<ProjectJobRecord>> {
        if project_id.trim().is_empty() {
            bail!("PROJECT_ISOLATION_VIOLATION: project_id cannot be empty");
        }
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, device_id, account_id, workflow_version_id, priority, state,
                    fencing_generation, current_node_id, created_at, updated_at
             FROM jobs WHERE project_id = ?1 AND project_id IS NOT NULL ORDER BY created_at DESC;",
        )?;
        let rows = stmt.query_map(params![project_id], |r| {
            Ok(ProjectJobRecord {
                id: r.get(0)?,
                project_id: r.get(1)?,
                device_id: r.get(2)?,
                account_id: r.get(3)?,
                workflow_version_id: r.get(4)?,
                priority: r.get(5)?,
                state: r.get(6)?,
                fencing_generation: r.get(7)?,
                current_node_id: r.get(8)?,
                created_at: r.get(9)?,
                updated_at: r.get(10)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    /// Creates or updates a fully project-scoped job row including optional `account_id`,
    /// scheduler `fencing_generation`, and `current_node_id`.
    pub fn create_or_update_project_job(&self, input: ProjectJobInput) -> Result<ProjectJobRecord> {
        if input.project_id.trim().is_empty() {
            bail!("PROJECT_ISOLATION_VIOLATION: project_id cannot be empty");
        }
        self.ensure_job_exists_with_project(
            Some(&input.project_id),
            &input.job_id,
            &input.device_id,
            &input.workflow_id,
            &input.workflow_name,
            &input.graph_json,
        )?;
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE jobs SET
                account_id = ?1,
                priority = ?2,
                state = ?3,
                fencing_generation = ?4,
                current_node_id = ?5,
                updated_at = ?6
             WHERE id = ?7;",
            params![
                input.account_id,
                input.priority,
                input.state,
                input.fencing_generation as i64,
                input.current_node_id,
                now,
                input.job_id,
            ],
        )?;
        Self::fetch_job_by_id_conn(&conn, &input.job_id)?
            .context("Failed reloading created/updated project job")
    }

    fn fetch_job_by_id_conn(conn: &Connection, job_id: &str) -> Result<Option<ProjectJobRecord>> {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, device_id, account_id, workflow_version_id, priority, state,
                    fencing_generation, current_node_id, created_at, updated_at
             FROM jobs WHERE id = ?1;",
        )?;
        let mut rows = stmt.query(params![job_id])?;
        if let Some(r) = rows.next()? {
            Ok(Some(ProjectJobRecord {
                id: r.get(0)?,
                project_id: r.get(1)?,
                device_id: r.get(2)?,
                account_id: r.get(3)?,
                workflow_version_id: r.get(4)?,
                priority: r.get(5)?,
                state: r.get(6)?,
                fencing_generation: r.get(7)?,
                current_node_id: r.get(8)?,
                created_at: r.get(9)?,
                updated_at: r.get(10)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn get_job(&self, job_id: &str) -> Result<Option<ProjectJobRecord>> {
        let conn = self.conn.lock().unwrap();
        Self::fetch_job_by_id_conn(&conn, job_id)
    }

    pub fn update_job_progress(
        &self,
        job_id: &str,
        state: &str,
        fencing_generation: Option<u64>,
        current_node_id: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE jobs SET
                state = ?1,
                fencing_generation = COALESCE(?2, fencing_generation),
                current_node_id = COALESCE(?3, current_node_id),
                updated_at = ?4
             WHERE id = ?5;",
            params![
                state,
                fencing_generation.map(|g| g as i64),
                current_node_id,
                now,
                job_id,
            ],
        )?;
        Ok(())
    }

    pub fn save_checkpoint(&self, input: CheckpointRecordInput) -> Result<CheckpointRecord> {
        let conn = self.conn.lock().unwrap();
        let id = input
            .id
            .unwrap_or_else(|| format!("cp-{}", uuid::Uuid::new_v4()));
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO checkpoints (id, job_id, node_id, state_json, evidence_ref, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6);",
            params![
                id,
                input.job_id,
                input.node_id,
                input.state_json,
                input.evidence_ref,
                now
            ],
        )?;
        conn.execute(
            "UPDATE jobs SET current_node_id = ?1, updated_at = ?2 WHERE id = ?3;",
            params![input.node_id, now, input.job_id],
        )?;
        Ok(CheckpointRecord {
            id,
            job_id: input.job_id,
            node_id: input.node_id,
            state_json: input.state_json,
            evidence_ref: input.evidence_ref,
            created_at: now,
        })
    }

    pub fn get_latest_checkpoint(&self, job_id: &str) -> Result<Option<CheckpointRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, job_id, node_id, state_json, evidence_ref, created_at
             FROM checkpoints WHERE job_id = ?1 ORDER BY rowid DESC LIMIT 1;",
        )?;
        let mut rows = stmt.query(params![job_id])?;
        if let Some(r) = rows.next()? {
            Ok(Some(CheckpointRecord {
                id: r.get(0)?,
                job_id: r.get(1)?,
                node_id: r.get(2)?,
                state_json: r.get(3)?,
                evidence_ref: r.get(4)?,
                created_at: r.get(5)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn list_job_checkpoints(&self, job_id: &str) -> Result<Vec<CheckpointRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, job_id, node_id, state_json, evidence_ref, created_at
             FROM checkpoints WHERE job_id = ?1 ORDER BY rowid ASC;",
        )?;
        let rows = stmt.query_map(params![job_id], |r| {
            Ok(CheckpointRecord {
                id: r.get(0)?,
                job_id: r.get(1)?,
                node_id: r.get(2)?,
                state_json: r.get(3)?,
                evidence_ref: r.get(4)?,
                created_at: r.get(5)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn load_job_resume_context(&self, job_id: &str) -> Result<Option<JobResumeContext>> {
        let conn = self.conn.lock().unwrap();
        let Some(job) = Self::fetch_job_by_id_conn(&conn, job_id)? else {
            return Ok(None);
        };
        let pid = job
            .project_id
            .as_deref()
            .context("PROJECT_ISOLATION_VIOLATION: Job lacks project ownership")?;
        let project = Self::fetch_project_by_id_conn(&conn, pid)?
            .with_context(|| format!("Project '{}' not found for job '{}'", pid, job_id))?;

        let (workflow_id, workflow_version_no, workflow_graph_json, workflow_sha256): (
            String,
            i64,
            String,
            String,
        ) = conn.query_row(
            "SELECT workflow_id, version_no, graph_json, sha256
             FROM workflow_versions WHERE id = ?1;",
            params![job.workflow_version_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )?;

        let latest_checkpoint = {
            let mut stmt = conn.prepare(
                "SELECT id, job_id, node_id, state_json, evidence_ref, created_at
                 FROM checkpoints WHERE job_id = ?1 ORDER BY rowid DESC LIMIT 1;",
            )?;
            let mut rows = stmt.query(params![job_id])?;
            rows.next()?
                .map(|r| {
                    Ok::<_, rusqlite::Error>(CheckpointRecord {
                        id: r.get(0)?,
                        job_id: r.get(1)?,
                        node_id: r.get(2)?,
                        state_json: r.get(3)?,
                        evidence_ref: r.get(4)?,
                        created_at: r.get(5)?,
                    })
                })
                .transpose()?
        };

        Ok(Some(JobResumeContext {
            job,
            project_dir_path: project.project_dir_path,
            workflow_id,
            workflow_version_no,
            workflow_graph_json,
            workflow_sha256,
            latest_checkpoint,
        }))
    }

    pub fn upsert_device_lease_record(
        &self,
        device_id: &str,
        job_id: &str,
        generation: u64,
        expires_at: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO devices (id, adb_serial, alias, width_px, height_px, rotation_deg, connection_state, updated_at)
             VALUES (?1, ?1, ?1, 1080, 1920, 0, 'busy', ?2)
             ON CONFLICT(id) DO UPDATE SET updated_at = excluded.updated_at;",
            params![device_id, now],
        )?;
        conn.execute(
            "INSERT INTO device_leases (device_id, job_id, generation, expires_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(device_id) DO UPDATE SET
                job_id = excluded.job_id,
                generation = excluded.generation,
                expires_at = excluded.expires_at;",
            params![device_id, job_id, generation as i64, expires_at],
        )?;
        Ok(())
    }

    pub fn delete_device_lease_record(&self, device_id: &str, job_id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute(
            "DELETE FROM device_leases WHERE device_id = ?1 AND job_id = ?2;",
            params![device_id, job_id],
        )?;
        Ok(rows > 0)
    }

    /// Reconciles stale `running` / `verifying` / `queued` jobs left behind by a crashed or
    /// forcibly-killed process when no live OS lock still owns that device.
    pub fn recover_interrupted_jobs_on_startup<F>(
        &self,
        mut is_device_locked_by_live_owner: F,
    ) -> Result<usize>
    where
        F: FnMut(&str) -> bool,
    {
        let mut conn = self.conn.lock().unwrap();
        let candidates: Vec<(String, String, Option<i64>)> = {
            let mut st = conn.prepare(
                "SELECT id, device_id, fencing_generation
                 FROM jobs
                 WHERE LOWER(state) IN ('running', 'verifying', 'queued');",
            )?;
            let mut rows = st.query([])?;
            let mut list = Vec::new();
            while let Some(r) = rows.next()? {
                list.push((r.get(0)?, r.get(1)?, r.get(2)?));
            }
            list
        };

        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let now = Utc::now().to_rfc3339();
        let mut recovered = 0usize;
        for (job_id, device_id, gen_opt) in candidates {
            if is_device_locked_by_live_owner(&device_id) {
                continue;
            }
            tx.execute(
                "UPDATE jobs
                 SET state = 'interrupted',
                     fencing_generation = COALESCE(fencing_generation, 0) + 1,
                     updated_at = ?1
                 WHERE id = ?2 AND LOWER(state) IN ('running', 'verifying', 'queued');",
                params![now, job_id],
            )?;
            tx.execute(
                "DELETE FROM device_leases WHERE device_id = ?1 OR job_id = ?2;",
                params![device_id, job_id],
            )?;
            let _ = gen_opt;
            recovered += 1;
        }
        tx.commit()?;
        Ok(recovered)
    }

    pub fn record_project_release(&self, release: ProjectReleaseRecord) -> Result<()> {
        if release.project_id.trim().is_empty() {
            bail!("PROJECT_ISOLATION_VIOLATION: project_id cannot be empty");
        }
        let _ = SemVer::parse(&release.version)?;
        let _ = ReleaseLifecycleState::from_str_checked(&release.lifecycle_state)?;

        let mut conn = self.conn.lock().unwrap();
        let mut backoff_ms = 15u64;
        let tx = {
            let mut attempt = 0u32;
            loop {
                match conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate) {
                    Ok(t) => break t,
                    Err(e) if is_sqlite_busy_err(&e) && attempt < 12 => {
                        attempt += 1;
                        std::thread::sleep(Duration::from_millis(backoff_ms));
                        backoff_ms = (backoff_ms * 2).min(150);
                    }
                    Err(e) => return Err(e.into()),
                }
            }
        };

        // If recording directly as Published, ensure an immutable Published version is not silently overwritten
        if release.lifecycle_state == "Published" {
            let dup: i64 = tx.query_row(
                "SELECT COUNT(*) FROM project_releases WHERE project_id = ?1 AND version = ?2 AND lifecycle_state = 'Published';",
                params![release.project_id, release.version],
                |r| r.get(0),
            )?;
            if dup > 0 {
                bail!(
                    "RELEASE_VERSION_CONFLICT: Version '{}' is already Published for project '{}'. Bump SemVer or create an RC.",
                    release.version,
                    release.project_id
                );
            }
        }

        tx.execute(
            "INSERT INTO project_releases (
                id, project_id, version, output_dir_path, exe_path, exe_size_bytes,
                exe_sha256, key_id, publisher_public_key_hex, pe_rcdata_pubkey_id,
                signature_ed25519_hex, build_report_json, built_at,
                lifecycle_state, release_channel, changelog_md, min_runtime_version,
                workflow_schema_version, ui_schema_version, module_api_version,
                sqlite_schema_version, git_commit, signature_status, promoted_at, rolled_back_from_id,
                publisher_profile_id, signing_mode, authenticode_trust_status,
                authenticode_thumbprint, authenticode_subject, trusted_public_release
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31);",
            params![
                release.id,
                release.project_id,
                release.version,
                release.output_dir_path,
                release.exe_path,
                release.exe_size_bytes as i64,
                release.exe_sha256,
                release.key_id,
                release.publisher_public_key_hex,
                release.pe_rcdata_pubkey_id,
                release.signature_ed25519_hex,
                release.build_report_json,
                release.built_at,
                release.lifecycle_state,
                release.release_channel,
                release.changelog_md,
                release.min_runtime_version,
                release.workflow_schema_version as i64,
                release.ui_schema_version as i64,
                release.module_api_version,
                release.sqlite_schema_version as i64,
                release.git_commit,
                release.signature_status,
                release.promoted_at,
                release.rolled_back_from_id,
                release.publisher_profile_id,
                release.signing_mode,
                release.authenticode_trust_status,
                release.authenticode_thumbprint,
                release.authenticode_subject,
                if release.trusted_public_release { 1 } else { 0 }
            ],
        )?;
        tx.execute(
            "INSERT INTO project_release_transitions (
                id, project_id, release_id, from_state, to_state, note, transitioned_at
             ) VALUES (?1, ?2, ?3, 'None', ?4, ?5, ?6);",
            params![
                format!("rtr-{}", uuid::Uuid::new_v4()),
                release.project_id,
                release.id,
                release.lifecycle_state,
                format!(
                    "Created release v{} [{}]",
                    release.version, release.release_channel
                ),
                release.built_at
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn get_project_release(&self, release_id: &str) -> Result<Option<ProjectReleaseRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, version, output_dir_path, exe_path, exe_size_bytes,
                    exe_sha256, key_id, publisher_public_key_hex, pe_rcdata_pubkey_id,
                    signature_ed25519_hex, build_report_json, built_at,
                    lifecycle_state, release_channel, changelog_md, min_runtime_version,
                    workflow_schema_version, ui_schema_version, module_api_version,
                    sqlite_schema_version, git_commit, signature_status, promoted_at, rolled_back_from_id,
                    publisher_profile_id, signing_mode, authenticode_trust_status,
                    authenticode_thumbprint, authenticode_subject, trusted_public_release
             FROM project_releases WHERE id = ?1 LIMIT 1;",
        )?;
        let mut rows = stmt.query(params![release_id])?;
        if let Some(r) = rows.next()? {
            Ok(Some(row_to_project_release(r)?))
        } else {
            Ok(None)
        }
    }

    pub fn transition_project_release_lifecycle(
        &self,
        project_id: &str,
        release_id: &str,
        target_state_str: &str,
        note: &str,
    ) -> Result<ProjectReleaseRecord> {
        let target_state = ReleaseLifecycleState::from_str_checked(target_state_str)?;
        let now = Utc::now().to_rfc3339();
        let mut conn = self.conn.lock().unwrap();
        let mut backoff_ms = 15u64;
        let tx = {
            let mut attempt = 0u32;
            loop {
                match conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate) {
                    Ok(t) => break t,
                    Err(e) if is_sqlite_busy_err(&e) && attempt < 12 => {
                        attempt += 1;
                        std::thread::sleep(Duration::from_millis(backoff_ms));
                        backoff_ms = (backoff_ms * 2).min(150);
                    }
                    Err(e) => return Err(e.into()),
                }
            }
        };

        let current_state_str: String = tx
            .query_row(
                "SELECT lifecycle_state FROM project_releases WHERE id = ?1 AND project_id = ?2;",
                params![release_id, project_id],
                |r| r.get(0),
            )
            .context("PROJECT_RELEASE_NOT_FOUND")?;
        let current_state = ReleaseLifecycleState::from_str_checked(&current_state_str)?;

        if current_state != target_state && !current_state.can_transition_to(target_state) {
            bail!(
                "INVALID_RELEASE_TRANSITION: Cannot transition release '{}' from '{}' to '{}'",
                release_id,
                current_state.as_str(),
                target_state.as_str()
            );
        }

        // If promoting/reactivating a release to Published, mark any OTHER currently Published release
        // for this project cleanly as Deprecated (or RolledBack when reactivating an older release).
        let mut prev_published_id: Option<String> = None;
        if target_state == ReleaseLifecycleState::Published {
            let mut st = tx.prepare(
                "SELECT id FROM project_releases WHERE project_id = ?1 AND lifecycle_state = 'Published' AND id != ?2;",
            )?;
            let others: Vec<String> = st
                .query_map(params![project_id, release_id], |r| r.get(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            for other_id in others {
                prev_published_id = Some(other_id.clone());
                let displaced_to = if current_state == ReleaseLifecycleState::Deprecated
                    || current_state == ReleaseLifecycleState::RolledBack
                {
                    "RolledBack"
                } else {
                    "Deprecated"
                };
                tx.execute(
                    "UPDATE project_releases SET lifecycle_state = ?1 WHERE id = ?2;",
                    params![displaced_to, other_id],
                )?;
                tx.execute(
                    "INSERT INTO project_release_transitions (
                        id, project_id, release_id, from_state, to_state, note, transitioned_at
                     ) VALUES (?1, ?2, ?3, 'Published', ?4, ?5, ?6);",
                    params![
                        format!("rtr-{}", uuid::Uuid::new_v4()),
                        project_id,
                        other_id,
                        displaced_to,
                        format!("Superseded by release {} ({})", release_id, note),
                        now
                    ],
                )?;
            }
        }

        let promoted_at_opt = if target_state == ReleaseLifecycleState::Published {
            Some(now.clone())
        } else {
            None
        };
        tx.execute(
            "UPDATE project_releases
             SET lifecycle_state = ?1,
                 promoted_at = COALESCE(?2, promoted_at),
                 rolled_back_from_id = COALESCE(?3, rolled_back_from_id)
             WHERE id = ?4 AND project_id = ?5;",
            params![
                target_state.as_str(),
                promoted_at_opt,
                prev_published_id,
                release_id,
                project_id
            ],
        )?;
        tx.execute(
            "INSERT INTO project_release_transitions (
                id, project_id, release_id, from_state, to_state, note, transitioned_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7);",
            params![
                format!("rtr-{}", uuid::Uuid::new_v4()),
                project_id,
                release_id,
                current_state.as_str(),
                target_state.as_str(),
                note,
                now
            ],
        )?;
        tx.commit()?;

        drop(conn);
        self.get_project_release(release_id)?
            .context("Failed to reload transitioned release")
    }

    pub fn list_project_release_transitions(
        &self,
        project_id: &str,
    ) -> Result<Vec<ReleaseTransitionRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut st = conn.prepare(
            "SELECT id, project_id, release_id, from_state, to_state, note, transitioned_at
             FROM project_release_transitions
             WHERE project_id = ?1
             ORDER BY transitioned_at DESC LIMIT 50;",
        )?;
        let rows = st.query_map(params![project_id], |r| {
            Ok(ReleaseTransitionRecord {
                id: r.get(0)?,
                project_id: r.get(1)?,
                release_id: r.get(2)?,
                from_state: r.get(3)?,
                to_state: r.get(4)?,
                note: r.get(5)?,
                transitioned_at: r.get(6)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn list_project_releases(&self, project_id: &str) -> Result<Vec<ProjectReleaseRecord>> {
        if project_id.trim().is_empty() {
            bail!("PROJECT_ISOLATION_VIOLATION: project_id cannot be empty");
        }
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, version, output_dir_path, exe_path, exe_size_bytes,
                    exe_sha256, key_id, publisher_public_key_hex, pe_rcdata_pubkey_id,
                    signature_ed25519_hex, build_report_json, built_at,
                    lifecycle_state, release_channel, changelog_md, min_runtime_version,
                    workflow_schema_version, ui_schema_version, module_api_version,
                    sqlite_schema_version, git_commit, signature_status, promoted_at, rolled_back_from_id,
                    publisher_profile_id, signing_mode, authenticode_trust_status,
                    authenticode_thumbprint, authenticode_subject, trusted_public_release
             FROM project_releases WHERE project_id = ?1 AND project_id IS NOT NULL ORDER BY built_at DESC;",
        )?;
        let rows = stmt.query_map(params![project_id], row_to_project_release)?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn get_project_detail(&self, project_id: &str) -> Result<Option<GameProjectDetail>> {
        let project = match self.get_project(project_id)? {
            Some(p) => p,
            None => return Ok(None),
        };
        let modules = self.list_project_modules(project_id)?;
        let workflows = self.list_project_workflows(project_id)?;
        let templates = self.list_vision_templates(project_id)?;
        let tasks = self.list_project_tasks(project_id)?;
        let ui_schema = self.get_ui_schema(project_id)?;
        let accounts = self.list_project_accounts(project_id)?;
        let releases = self.list_project_releases(project_id)?;
        Ok(Some(GameProjectDetail {
            project,
            modules,
            workflows,
            templates,
            tasks,
            ui_schema,
            accounts,
            releases,
        }))
    }

    pub fn upsert_publisher_profile(
        &self,
        input: PublisherProfileInput,
    ) -> Result<PublisherProfileRecord> {
        let publisher_id = input.publisher_id.trim().to_string();
        let display_name = input.display_name.trim().to_string();
        let organization = input.organization.trim().to_string();
        let copyright = input.copyright.trim().to_string();
        if publisher_id.is_empty() || display_name.is_empty() || organization.is_empty() {
            bail!("PUBLISHER_PROFILE_INVALID: publisher_id, display_name, and organization are required");
        }
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock().unwrap();
        let existing_id: Option<String> = conn
            .query_row(
                "SELECT id FROM publisher_profiles WHERE id = ?1 OR publisher_id = ?2 LIMIT 1;",
                params![input.id.clone().unwrap_or_default(), publisher_id],
                |r| r.get(0),
            )
            .ok();
        let id = existing_id.unwrap_or_else(|| {
            input
                .id
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| format!("pub-{}", uuid::Uuid::new_v4()))
        });
        let created_at: String = conn
            .query_row(
                "SELECT created_at FROM publisher_profiles WHERE id = ?1 LIMIT 1;",
                params![id],
                |r| r.get(0),
            )
            .unwrap_or_else(|_| now.clone());

        conn.execute(
            "INSERT INTO publisher_profiles (
                id, publisher_id, display_name, organization, website, support_contact,
                copyright, ed25519_key_id, ed25519_pubkey_hex, authenticode_cert_ref,
                release_channel, signing_policy, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
             ON CONFLICT(id) DO UPDATE SET
                publisher_id = excluded.publisher_id,
                display_name = excluded.display_name,
                organization = excluded.organization,
                website = excluded.website,
                support_contact = excluded.support_contact,
                copyright = excluded.copyright,
                ed25519_key_id = excluded.ed25519_key_id,
                ed25519_pubkey_hex = excluded.ed25519_pubkey_hex,
                authenticode_cert_ref = excluded.authenticode_cert_ref,
                release_channel = excluded.release_channel,
                signing_policy = excluded.signing_policy,
                updated_at = excluded.updated_at;",
            params![
                id,
                publisher_id,
                display_name,
                organization,
                input
                    .website
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty()),
                input
                    .support_contact
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty()),
                if copyright.is_empty() {
                    format!("Copyright (C) {} {}", Utc::now().format("%Y"), organization)
                } else {
                    copyright
                },
                input.ed25519_key_id.unwrap_or_default(),
                input.ed25519_pubkey_hex.unwrap_or_default(),
                input
                    .authenticode_cert_ref
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty()),
                if input.release_channel.trim().is_empty() {
                    "internal".to_string()
                } else {
                    input.release_channel.trim().to_string()
                },
                if input.signing_policy.trim().is_empty() {
                    "unsigned_internal".to_string()
                } else {
                    input.signing_policy.trim().to_string()
                },
                created_at,
                now
            ],
        )?;
        drop(conn);
        self.get_publisher_profile(&id)?
            .context("Failed reloading saved publisher profile")
    }

    pub fn list_publisher_profiles(&self) -> Result<Vec<PublisherProfileRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut st = conn.prepare(
            "SELECT id, publisher_id, display_name, organization, website, support_contact,
                    copyright, ed25519_key_id, ed25519_pubkey_hex, authenticode_cert_ref,
                    release_channel, signing_policy, created_at, updated_at
             FROM publisher_profiles ORDER BY updated_at DESC, display_name ASC;",
        )?;
        let rows = st.query_map([], |r| {
            Ok(PublisherProfileRecord {
                id: r.get(0)?,
                publisher_id: r.get(1)?,
                display_name: r.get(2)?,
                organization: r.get(3)?,
                website: r.get(4)?,
                support_contact: r.get(5)?,
                copyright: r.get(6)?,
                ed25519_key_id: r.get(7)?,
                ed25519_pubkey_hex: r.get(8)?,
                authenticode_cert_ref: r.get(9)?,
                release_channel: r.get(10)?,
                signing_policy: r.get(11)?,
                created_at: r.get(12)?,
                updated_at: r.get(13)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn get_publisher_profile(
        &self,
        id_or_publisher_id: &str,
    ) -> Result<Option<PublisherProfileRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut st = conn.prepare(
            "SELECT id, publisher_id, display_name, organization, website, support_contact,
                    copyright, ed25519_key_id, ed25519_pubkey_hex, authenticode_cert_ref,
                    release_channel, signing_policy, created_at, updated_at
             FROM publisher_profiles WHERE id = ?1 OR publisher_id = ?1 LIMIT 1;",
        )?;
        let mut rows = st.query(params![id_or_publisher_id])?;
        if let Some(r) = rows.next()? {
            Ok(Some(PublisherProfileRecord {
                id: r.get(0)?,
                publisher_id: r.get(1)?,
                display_name: r.get(2)?,
                organization: r.get(3)?,
                website: r.get(4)?,
                support_contact: r.get(5)?,
                copyright: r.get(6)?,
                ed25519_key_id: r.get(7)?,
                ed25519_pubkey_hex: r.get(8)?,
                authenticode_cert_ref: r.get(9)?,
                release_channel: r.get(10)?,
                signing_policy: r.get(11)?,
                created_at: r.get(12)?,
                updated_at: r.get(13)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn delete_publisher_profile(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE projects SET publisher_profile_id = NULL WHERE publisher_profile_id = ?1;",
            params![id],
        )?;
        let affected =
            conn.execute("DELETE FROM publisher_profiles WHERE id = ?1;", params![id])?;
        Ok(affected > 0)
    }

    /// Atomically checks and records a single-use KeyRotationDelegation nonce.
    /// Returns `Ok(true)` if fresh and recorded; `Ok(false)` if already used (replay).
    pub fn check_and_record_key_rotation_nonce(
        &self,
        nonce: &str,
        project_id: &str,
        old_pubkey_hex: &str,
        new_pubkey_hex: &str,
    ) -> Result<bool> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let exists: i64 = tx.query_row(
            "SELECT COUNT(*) FROM used_key_rotation_nonces WHERE nonce = ?1;",
            params![nonce],
            |r| r.get(0),
        )?;
        if exists > 0 {
            return Ok(false);
        }
        tx.execute(
            "INSERT INTO used_key_rotation_nonces (nonce, project_id, old_pubkey_hex, new_pubkey_hex, accepted_at)
             VALUES (?1, ?2, ?3, ?4, ?5);",
            params![nonce, project_id, old_pubkey_hex, new_pubkey_hex, Utc::now().to_rfc3339()],
        )?;
        tx.commit()?;
        Ok(true)
    }
}

fn query_events_for_streams(
    conn: &Connection,
    streams: &[&str],
    after_seqs: &HashMap<String, u64>,
) -> Result<Vec<RuntimeVarDeltaEvent>> {
    let mut events = Vec::new();
    for sid in streams {
        let min_seq = after_seqs.get(*sid).copied().unwrap_or(0);
        let mut st = conn.prepare(
            "SELECT stream_id, seq, project_id, session_id, job_id, workflow_version, deltas_json, emitted_at_utc
             FROM runtime_var_events
             WHERE stream_id = ?1 AND seq > ?2
             ORDER BY seq ASC;",
        )?;
        let mut rows = st.query(params![sid, min_seq as i64])?;
        while let Some(r) = rows.next()? {
            let deltas_str: String = r.get(6)?;
            let deltas: Vec<RuntimeVariableEnvelope> =
                serde_json::from_str(&deltas_str).unwrap_or_default();
            events.push(RuntimeVarDeltaEvent {
                stream_id: r.get(0)?,
                scope_key: r.get(0)?,
                project_id: r.get(2)?,
                session_id: r.get(3)?,
                job_id: r.get(4)?,
                seq: r.get::<_, i64>(1)?.max(0) as u64,
                workflow_version: r.get(5)?,
                emitted_at_utc: r.get(7)?,
                deltas,
            });
        }
    }
    Ok(events)
}

fn row_to_project_release(r: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectReleaseRecord> {
    Ok(ProjectReleaseRecord {
        id: r.get(0)?,
        project_id: r.get(1)?,
        version: r.get(2)?,
        output_dir_path: r.get(3)?,
        exe_path: r.get(4)?,
        exe_size_bytes: r.get::<_, i64>(5)? as u64,
        exe_sha256: r.get(6)?,
        key_id: r.get(7)?,
        publisher_public_key_hex: r.get(8)?,
        pe_rcdata_pubkey_id: r.get(9)?,
        signature_ed25519_hex: r.get(10)?,
        build_report_json: r.get(11)?,
        built_at: r.get(12)?,
        lifecycle_state: r.get(13)?,
        release_channel: r.get(14)?,
        changelog_md: r.get(15)?,
        min_runtime_version: r.get(16)?,
        workflow_schema_version: r.get::<_, i64>(17)? as u32,
        ui_schema_version: r.get::<_, i64>(18)? as u32,
        module_api_version: r.get(19)?,
        sqlite_schema_version: r.get::<_, i64>(20)? as u32,
        git_commit: r.get(21)?,
        signature_status: r.get(22)?,
        promoted_at: r.get(23)?,
        rolled_back_from_id: r.get(24)?,
        publisher_profile_id: r.get(25)?,
        signing_mode: r.get(26)?,
        authenticode_trust_status: r.get(27)?,
        authenticode_thumbprint: r.get(28)?,
        authenticode_subject: r.get(29)?,
        trusted_public_release: r.get::<_, i64>(30)? != 0,
    })
}

fn is_sqlite_busy_err(e: &rusqlite::Error) -> bool {
    matches!(
        e,
        rusqlite::Error::SqliteFailure(err, _)
            if err.code == rusqlite::ErrorCode::DatabaseBusy
                || err.code == rusqlite::ErrorCode::DatabaseLocked
    )
}

/// Dedicated persistent-connection SQLite WAL watcher (P5.1 Regression Backlog #1).
/// Strictly compares `PRAGMA data_version` ONLY on this single long-lived connection,
/// with bounded exponential backoff on `SQLITE_BUSY`.
pub struct ExternalWalReader {
    conn: Connection,
    last_data_version: Option<i64>,
    pub busy_retry_count: u64,
}

impl ExternalWalReader {
    pub fn open<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        conn.busy_timeout(Duration::from_millis(250))?;
        let _ = conn.execute_batch("PRAGMA journal_mode = WAL;");
        Ok(Self {
            conn,
            last_data_version: None,
            busy_retry_count: 0,
        })
    }

    pub fn last_observed_data_version(&self) -> Option<i64> {
        self.last_data_version
    }

    /// Returns `(data_version_changed, newly_committed_events)` comparing `PRAGMA data_version`
    /// on the exact same persistent `Connection`.
    pub fn poll_deltas(
        &mut self,
        base_scope_key: &str,
        after_seqs: &HashMap<String, u64>,
    ) -> Result<(bool, Vec<RuntimeVarDeltaEvent>)> {
        let clean_base = base_scope_key.trim_end_matches("::testkit");
        let testkit_scope = format!("{}::testkit", clean_base);

        let mut dv: i64 = 0;
        let mut backoff_ms = 15u64;
        for attempt in 0..=5 {
            match self
                .conn
                .query_row("PRAGMA data_version;", [], |r| r.get::<_, i64>(0))
            {
                Ok(v) => {
                    dv = v;
                    break;
                }
                Err(e) if is_sqlite_busy_err(&e) && attempt < 5 => {
                    self.busy_retry_count += 1;
                    std::thread::sleep(Duration::from_millis(backoff_ms));
                    backoff_ms = (backoff_ms * 2).min(120);
                }
                Err(e) => return Err(e.into()),
            }
        }

        let changed = self
            .last_data_version
            .map(|prev| prev != dv)
            .unwrap_or(true);
        self.last_data_version = Some(dv);
        if !changed {
            return Ok((false, Vec::new()));
        }

        let mut events = Vec::new();
        backoff_ms = 15;
        for attempt in 0..=5 {
            match query_events_for_streams(
                &self.conn,
                &[clean_base, testkit_scope.as_str()],
                after_seqs,
            ) {
                Ok(ev) => {
                    events = ev;
                    break;
                }
                Err(e)
                    if e.downcast_ref::<rusqlite::Error>()
                        .map(is_sqlite_busy_err)
                        .unwrap_or(false)
                        && attempt < 5 =>
                {
                    self.busy_retry_count += 1;
                    std::thread::sleep(Duration::from_millis(backoff_ms));
                    backoff_ms = (backoff_ms * 2).min(120);
                }
                Err(e) => return Err(e),
            }
        }

        Ok((true, events))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqlite_in_memory_migrations_and_crud() {
        let storage = StorageManager::new_in_memory().unwrap();
        let dev = DeviceSummary {
            id: "dev-1".to_string(),
            adb_serial: "serial-123".to_string(),
            alias: "Pixel 6".to_string(),
            android_api: Some(33),
            model: Some("Pixel 6".to_string()),
            width_px: 1080,
            height_px: 2400,
            rotation_deg: 0,
            connection_state: ConnectionState::Ready,
            updated_at: Utc::now(),
        };

        storage.upsert_device(&dev).unwrap();
        let devices = storage.list_devices().unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].alias, "Pixel 6");

        storage
            .save_workflow_draft("wf-1", "Quest Workflow", "{\"nodes\":[]}")
            .unwrap();
        let (name, draft) = storage.get_workflow_draft("wf-1").unwrap().unwrap();
        assert_eq!(name, "Quest Workflow");
        assert_eq!(draft, "{\"nodes\":[]}");
    }

    #[test]
    fn test_sqlite_job_and_step_events_persistence() {
        let storage = StorageManager::new_in_memory().unwrap();
        let job_id = "job-unit-test-101";
        let dev_id = "emulator-5554";
        let wf_id = "wf-test-01";

        storage
            .ensure_job_exists(job_id, dev_id, wf_id, "Test Workflow", "{}")
            .unwrap();

        let event = StepEvent {
            id: "evt-001".to_string(),
            job_id: job_id.to_string(),
            node_id: "n-start".to_string(),
            event_type: fp_domain::StepEventType::CommandSent,
            attempt_no: 1,
            at: Utc::now(),
            payload_json: "{\"status\":\"ok\"}".to_string(),
        };

        storage.insert_step_event(&event).unwrap();
        let events = storage.list_step_events(Some(job_id)).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, "evt-001");
        assert_eq!(events[0].node_id, "n-start");

        storage.complete_job(job_id, "Succeeded").unwrap();
    }

    #[test]
    fn test_sqlite_file_persistence_across_reopen_and_checkpoint_recovery() {
        let temp_dir = std::env::temp_dir();
        let db_file = temp_dir.join(format!(
            "farmerpro_persist_test_{}.db",
            uuid::Uuid::new_v4()
        ));
        let db_path_str = db_file.to_str().unwrap().to_string();

        let job_id = "job-reopen-test-999";
        let dev_id = "emulator-5554";
        let wf_id = "wf-checkpoint-flow";

        // Phase 1: Open DB, store workflow, job, and checkpoint event, then drop connection (close)
        {
            let storage = StorageManager::new(&db_file).unwrap();
            storage
                .save_workflow_draft(
                    wf_id,
                    "Reopen Test Workflow",
                    "{\"nodes\":[{\"id\":\"n-start\"}]}",
                )
                .unwrap();

            storage
                .ensure_job_exists(
                    job_id,
                    dev_id,
                    wf_id,
                    "Reopen Test Workflow",
                    "{\"nodes\":[]}",
                )
                .unwrap();

            let checkpoint_evt = StepEvent {
                id: "evt-chk-001".to_string(),
                job_id: job_id.to_string(),
                node_id: "n-checkpoint".to_string(),
                event_type: fp_domain::StepEventType::CheckpointSaved,
                attempt_no: 1,
                at: Utc::now(),
                payload_json: "{\"checkpoint_name\":\"quest_active\",\"state\":\"saved\"}"
                    .to_string(),
            };
            storage.insert_step_event(&checkpoint_evt).unwrap();
        }

        // Phase 2: Reopen DB from the same file path (simulating app restart)
        {
            let reopened_storage = StorageManager::new(&db_file).unwrap();

            let (name, draft) = reopened_storage.get_workflow_draft(wf_id).unwrap().unwrap();
            assert_eq!(name, "Reopen Test Workflow");
            assert!(draft.contains("n-start"));

            let events = reopened_storage.list_step_events(Some(job_id)).unwrap();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].id, "evt-chk-001");
            assert_eq!(events[0].node_id, "n-checkpoint");
            assert_eq!(
                events[0].event_type,
                fp_domain::StepEventType::CheckpointSaved
            );
            assert!(events[0].payload_json.contains("quest_active"));

            reopened_storage
                .complete_job(job_id, "ResumedAndCompleted")
                .unwrap();
        }

        let _ = std::fs::remove_file(&db_file);
        let _ = std::fs::remove_file(format!("{}-wal", db_path_str));
        let _ = std::fs::remove_file(format!("{}-shm", db_path_str));
    }

    #[test]
    fn test_p2_v1_to_v2_migration_idempotency_and_data_preservation() {
        let temp_dir =
            std::env::temp_dir().join(format!("fp_p2_mig_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_file = temp_dir.join("v1_legacy.db");

        // 1. Create a real pre-P2 v1 database with legacy devices, workflows, jobs, step_events (and empty schema_migrations, matching production v1!)
        {
            let conn = Connection::open(&db_file).unwrap();
            conn.execute_batch(migrations::MIGRATION_V1_SQL).unwrap();
            let now = Utc::now().to_rfc3339();
            conn.execute(
                "INSERT INTO devices (id, adb_serial, alias, width_px, height_px, rotation_deg, connection_state, updated_at)
                 VALUES ('emulator-5554', 'emulator-5554', 'LDPlayer', 1280, 720, 0, 'ready', ?1);",
                params![now],
            ).unwrap();
            conn.execute(
                "INSERT INTO workflows (id, name, draft_json, draft_updated_at)
                 VALUES ('wf-legacy-01', 'Legacy Workflow', '{\"nodes\":[]}', ?1);",
                params![now],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO workflow_versions (id, workflow_id, version_no, graph_json, sha256, created_at)
                 VALUES ('wf-legacy-01-v1', 'wf-legacy-01', 1, '{\"nodes\":[]}', 'abc', ?1);",
                params![now],
            ).unwrap();
            conn.execute(
                "INSERT INTO accounts (id, alias, game_profile_id, secret_ref, state, updated_at)
                 VALUES ('acc-legacy-01', 'MainFarmer', 'Hero01', 'CRED_MGR://legacy/acc1', 'ready', ?1);",
                params![now],
            ).unwrap();
            conn.execute(
                "INSERT INTO jobs (id, device_id, account_id, workflow_version_id, priority, state, created_at, updated_at)
                 VALUES ('job-legacy-01', 'emulator-5554', 'acc-legacy-01', 'wf-legacy-01-v1', 1, 'Succeeded', ?1, ?1);",
                params![now],
            ).unwrap();
            conn.execute(
                "INSERT INTO step_events (id, job_id, node_id, event_type, attempt_no, at, payload_json)
                 VALUES ('evt-leg-1', 'job-legacy-01', 'n-start', 'command_sent', 1, ?1, '{}');",
                params![now],
            ).unwrap();
        }

        // 2. Open with StorageManager -> runs v1..v6 migrations atomically
        let storage = StorageManager::new(&db_file).unwrap();
        assert_eq!(
            storage.applied_migration_versions().unwrap(),
            vec![1, 2, 3, 4, 5, 6]
        );

        // 3. Verify idempotency: running migrations again causes zero errors and keeps versions [1, 2, 3, 4, 5, 6]
        storage.run_migrations().unwrap();
        assert_eq!(
            storage.applied_migration_versions().unwrap(),
            vec![1, 2, 3, 4, 5, 6]
        );

        // 4. Verify 100% data preservation + attachment to default GameProject
        let integrity = storage.check_integrity().unwrap();
        assert!(integrity.is_healthy);
        assert_eq!(integrity.foreign_key_violations, 0);
        assert_eq!(integrity.table_counts.get("workflows"), Some(&1));
        assert_eq!(integrity.table_counts.get("jobs"), Some(&1));
        assert_eq!(integrity.table_counts.get("step_events"), Some(&1));
        assert_eq!(integrity.table_counts.get("accounts"), Some(&1));

        let legacy_wfs = storage
            .list_project_workflows(DEFAULT_LEGACY_PROJECT_ID)
            .unwrap();
        assert_eq!(legacy_wfs.len(), 1);
        assert_eq!(legacy_wfs[0].id, "wf-legacy-01");

        let legacy_jobs = storage
            .list_project_jobs(DEFAULT_LEGACY_PROJECT_ID)
            .unwrap();
        assert_eq!(legacy_jobs.len(), 1);
        assert_eq!(legacy_jobs[0].id, "job-legacy-01");

        drop(storage);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_p2_failed_migration_atomic_rollback() {
        let storage = StorageManager::new_in_memory().unwrap();
        storage
            .save_workflow_draft("wf-keep", "Keep Me", "{}")
            .unwrap();

        let bad_v7_sql = "
            CREATE TABLE should_not_exist (id TEXT PRIMARY KEY);
            INSERT INTO nonexistent_table_triggering_rollback VALUES (1);
        ";
        let res = storage.apply_transactional_migration(7, bad_v7_sql);
        assert!(res.is_err());

        // Version 7 must NOT be recorded and `should_not_exist` table must NOT exist
        assert_eq!(
            storage.applied_migration_versions().unwrap(),
            vec![1, 2, 3, 4, 5, 6]
        );
        let integrity = storage.check_integrity().unwrap();
        assert!(integrity.is_healthy);
        assert!(!integrity.table_counts.contains_key("should_not_exist"));
        assert!(storage.get_workflow_draft("wf-keep").unwrap().is_some());
    }

    #[test]
    fn test_p2_online_backup_integrity_and_restore() {
        let temp_dir =
            std::env::temp_dir().join(format!("fp_p2_backup_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let live_db = temp_dir.join("live.db");
        let backup_db = temp_dir.join("backup.db");
        let projects_root = temp_dir.join("projects");

        {
            let storage = StorageManager::new(&live_db).unwrap();
            let p = storage
                .create_project(
                    CreateProjectRequest {
                        slug: "backup-game".to_string(),
                        name: "Backup Game".to_string(),
                        game_package_id: "com.test.backup".to_string(),
                        version: None,
                        exe_name: None,
                        window_title: None,
                        icon_rel_path: None,
                        target_width_px: None,
                        target_height_px: None,
                        bundle_adb: None,
                        enabled_modules: None,
                    },
                    &projects_root,
                )
                .unwrap();

            storage
                .save_project_workflow(&p.id, "wf-bk-1", "Backup WF", "Chung", "{}")
                .unwrap();

            let bk_report = storage.online_backup_to(&backup_db).unwrap();
            assert!(bk_report.integrity.is_healthy);
            assert_eq!(bk_report.integrity.table_counts.get("projects"), Some(&1));
            assert_eq!(bk_report.integrity.table_counts.get("workflows"), Some(&1));

            // Simulate accidental deletion on live DB
            assert!(storage.delete_project(&p.id, false).unwrap());
            assert_eq!(storage.list_projects().unwrap().len(), 0);
        }

        // Restore from backup and verify all data returns intact
        let post_restore = StorageManager::restore_from_backup(&backup_db, &live_db).unwrap();
        assert!(post_restore.is_healthy);
        assert_eq!(post_restore.table_counts.get("projects"), Some(&1));
        assert_eq!(post_restore.table_counts.get("workflows"), Some(&1));

        let reopened = StorageManager::new(&live_db).unwrap();
        let projs = reopened.list_projects().unwrap();
        assert_eq!(projs.len(), 1);
        assert_eq!(projs[0].project.slug, "backup-game");

        drop(reopened);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_p2_legacy_jx2_10_workflows_and_templates_migration() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let repo_root = manifest_dir.parent().unwrap().parent().unwrap();
        let legacy_jx2_dir = repo_root.join("games").join("jx2mobile");
        if !legacy_jx2_dir.exists() {
            panic!("Expected legacy JX2 assets at {}", legacy_jx2_dir.display());
        }

        let temp_dir =
            std::env::temp_dir().join(format!("fp_p2_jx2_mig_test_{}", uuid::Uuid::new_v4()));
        let db_file = temp_dir.join("jx2_mig.db");
        let projects_dir = temp_dir.join("projects");

        let storage = StorageManager::new(&db_file).unwrap();
        let summary = storage
            .migrate_legacy_jx2_assets(&legacy_jx2_dir, &projects_dir)
            .unwrap();

        assert_eq!(summary.project_id, DEFAULT_LEGACY_PROJECT_ID);
        assert_eq!(summary.workflows_imported, 10);
        assert_eq!(summary.templates_imported, 8);
        assert!(summary.tasks_seeded >= 3);
        assert!(summary.original_source_preserved);

        // Idempotent second run
        let summary2 = storage
            .migrate_legacy_jx2_assets(&legacy_jx2_dir, &projects_dir)
            .unwrap();
        assert_eq!(summary2.workflows_imported, 10);

        let wfs = storage
            .list_project_workflows(DEFAULT_LEGACY_PROJECT_ID)
            .unwrap();
        assert_eq!(wfs.len(), 10);
        let tpls = storage
            .list_vision_templates(DEFAULT_LEGACY_PROJECT_ID)
            .unwrap();
        assert_eq!(tpls.len(), 8);
        for t in &tpls {
            assert!(t.width_px > 0 && t.height_px > 0);
            assert_eq!(t.sha256.len(), 64);
        }

        drop(storage);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_p2_project_crud_clone_and_strict_isolation() {
        let temp_dir =
            std::env::temp_dir().join(format!("fp_p2_isolation_test_{}", uuid::Uuid::new_v4()));
        let projects_root = temp_dir.join("projects");
        let storage = StorageManager::new_in_memory().unwrap();

        // 1. Create Project A & Project B
        let proj_a = storage
            .create_project(
                CreateProjectRequest {
                    slug: "alpha-rpg".to_string(),
                    name: "Alpha RPG".to_string(),
                    game_package_id: "com.alpha.rpg".to_string(),
                    version: Some("1.0.0".to_string()),
                    exe_name: None,
                    window_title: None,
                    icon_rel_path: None,
                    target_width_px: Some(1920),
                    target_height_px: Some(1080),
                    bundle_adb: Some(true),
                    enabled_modules: None,
                },
                &projects_root,
            )
            .unwrap();

        let proj_b = storage
            .create_project(
                CreateProjectRequest {
                    slug: "beta-slg".to_string(),
                    name: "Beta SLG".to_string(),
                    game_package_id: "com.beta.slg".to_string(),
                    version: Some("2.0.0".to_string()),
                    exe_name: Some("Auto_Beta_SLG.exe".to_string()),
                    window_title: None,
                    icon_rel_path: None,
                    target_width_px: Some(1280),
                    target_height_px: Some(720),
                    bundle_adb: Some(false),
                    enabled_modules: Some(vec!["mod.adb_core".to_string()]),
                },
                &projects_root,
            )
            .unwrap();

        // 2. Add workflow, template, task, account, job to Project A
        storage
            .save_project_workflow(
                &proj_a.id,
                "wf-alpha-01",
                "Alpha Quest",
                "NV",
                "{\"nodes\":[]}",
            )
            .unwrap();
        storage
            .register_vision_template(VisionTemplateInput {
                id: None,
                project_id: proj_a.id.clone(),
                name: "btn_alpha".to_string(),
                rel_path: "templates/btn_alpha.png".to_string(),
                width_px: 100,
                height_px: 40,
                source_crop_x: Some(10),
                source_crop_y: Some(20),
                default_confidence: Some(0.9),
                sha256: "a".repeat(64),
            })
            .unwrap();
        let cat_a = TaskCategoryDef {
            id: "cat-nv".to_string(),
            short_tag: "[NV]".to_string(),
            display_name: "Nhiệm vụ".to_string(),
            color_hex: "#38BDF8".to_string(),
        };
        storage
            .upsert_project_task(
                &proj_a.id,
                MiniTaskDef {
                    id: "task-alpha-1".to_string(),
                    title: "Alpha Daily".to_string(),
                    category_id: "cat-nv".to_string(),
                    workflow_id: "wf-alpha-01".to_string(),
                    default_enabled: true,
                    priority: 1,
                    estimate_sec: 30,
                    settings_schema: vec![],
                },
                &cat_a,
            )
            .unwrap();
        storage
            .upsert_project_account(ProjectAccountInput {
                id: Some("acc-alpha-1".to_string()),
                project_id: proj_a.id.clone(),
                alias: "AlphaUser1".to_string(),
                game_profile_id: Some("Server1".to_string()),
                secret_ref: Some("CRED_MGR://FarmerPro/account/alpha-rpg/acc-alpha-1".to_string()),
                state: "ready".to_string(),
            })
            .unwrap();
        storage
            .ensure_project_job_exists(
                &proj_a.id,
                "job-alpha-1",
                "emulator-5554",
                "wf-alpha-01",
                "Alpha Quest",
                "{}",
            )
            .unwrap();

        // 3. Verify Project B sees ZERO of Project A's resources
        assert_eq!(storage.list_project_workflows(&proj_b.id).unwrap().len(), 0);
        assert_eq!(storage.list_vision_templates(&proj_b.id).unwrap().len(), 0);
        assert_eq!(storage.list_project_tasks(&proj_b.id).unwrap().len(), 0);
        assert_eq!(storage.list_project_accounts(&proj_b.id).unwrap().len(), 0);
        assert_eq!(storage.list_project_jobs(&proj_b.id).unwrap().len(), 0);

        // 4. Verify cross-project tampering/linking is blocked with PROJECT_ISOLATION_VIOLATION
        let cross_wf_err =
            storage.save_project_workflow(&proj_b.id, "wf-alpha-01", "Hijack", "NV", "{}");
        assert!(cross_wf_err
            .unwrap_err()
            .to_string()
            .contains("PROJECT_ISOLATION_VIOLATION"));

        let cross_task_err = storage.upsert_project_task(
            &proj_b.id,
            MiniTaskDef {
                id: "task-beta-bad".to_string(),
                title: "Cross Link".to_string(),
                category_id: "cat-nv".to_string(),
                workflow_id: "wf-alpha-01".to_string(),
                default_enabled: true,
                priority: 1,
                estimate_sec: 10,
                settings_schema: vec![],
            },
            &cat_a,
        );
        assert!(cross_task_err
            .unwrap_err()
            .to_string()
            .contains("PROJECT_ISOLATION_VIOLATION"));

        let mut bad_ui = storage.get_ui_schema(&proj_a.id).unwrap().unwrap();
        bad_ui.project_slug = "alpha-rpg".to_string();
        assert!(storage
            .save_ui_schema(&proj_b.id, &bad_ui)
            .unwrap_err()
            .to_string()
            .contains("PROJECT_ISOLATION_VIOLATION"));

        // 5. Clone Project A -> Project C and verify Project C has independent copies
        let proj_c = storage
            .clone_project(
                CloneProjectRequest {
                    source_project_id: proj_a.id.clone(),
                    new_slug: "alpha-rpg-clone".to_string(),
                    new_name: "Alpha RPG Clone".to_string(),
                    new_game_package_id: Some("com.alpha.clone".to_string()),
                    new_exe_name: None,
                },
                &projects_root,
            )
            .unwrap();
        assert_eq!(storage.list_project_workflows(&proj_c.id).unwrap().len(), 1);
        assert_eq!(storage.list_vision_templates(&proj_c.id).unwrap().len(), 1);
        assert_eq!(storage.list_project_tasks(&proj_c.id).unwrap().len(), 1);
        let cloned_manifest_path = Path::new(&proj_c.project_dir_path).join("project.json");
        assert!(cloned_manifest_path.exists());
        let cloned_manifest: GameProjectManifest =
            serde_json::from_slice(&std::fs::read(&cloned_manifest_path).unwrap()).unwrap();
        assert_eq!(cloned_manifest.slug, "alpha-rpg-clone");
        assert_eq!(cloned_manifest.workflow_files.len(), 1);
        assert_eq!(cloned_manifest.template_files.len(), 1);

        // Deleting Project C must NOT affect Project A
        assert!(storage.delete_project(&proj_c.id, true).unwrap());
        assert_eq!(storage.list_project_workflows(&proj_a.id).unwrap().len(), 1);
        assert_eq!(storage.list_project_tasks(&proj_a.id).unwrap().len(), 1);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_p2_disk_manifest_sync_duplicate_slug_and_legacy_version_job() {
        let temp_dir =
            std::env::temp_dir().join(format!("fp_p2_edge_test_{}", uuid::Uuid::new_v4()));
        let projects_root = temp_dir.join("projects");
        let storage = StorageManager::new_in_memory().unwrap();

        let proj = storage
            .create_project(
                CreateProjectRequest {
                    slug: "edge-game".to_string(),
                    name: "Edge Game".to_string(),
                    game_package_id: "com.edge.game".to_string(),
                    version: Some("1.0.0".to_string()),
                    exe_name: None,
                    window_title: None,
                    icon_rel_path: None,
                    target_width_px: Some(1280),
                    target_height_px: Some(720),
                    bundle_adb: Some(true),
                    enabled_modules: None,
                },
                &projects_root,
            )
            .unwrap();

        // Duplicate slug on clone must fail without creating an orphan folder
        let dup_clone = storage.clone_project(
            CloneProjectRequest {
                source_project_id: proj.id.clone(),
                new_slug: "edge-game".to_string(),
                new_name: "Dup".to_string(),
                new_game_package_id: None,
                new_exe_name: None,
            },
            &projects_root,
        );
        assert!(dup_clone.is_err());

        // Update project syncs project.json and ignores empty strings
        let updated = storage
            .update_project(UpdateProjectRequest {
                id: proj.id.clone(),
                name: Some("Edge Game Pro".to_string()),
                version: Some("2.1.0".to_string()),
                exe_name: Some("   ".to_string()),
                window_title: Some("Edge Pro Window".to_string()),
                target_width_px: Some(1920),
                target_height_px: Some(1080),
                bundle_adb: Some(false),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(updated.exe_name, "Auto_Edge_Game.exe");
        assert_eq!(updated.version, "2.1.0");
        assert!(!updated.bundle_adb);
        let disk_manifest: GameProjectManifest = serde_json::from_slice(
            &std::fs::read(Path::new(&proj.project_dir_path).join("project.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(disk_manifest.name, "Edge Game Pro");
        assert_eq!(disk_manifest.version, "2.1.0");
        assert_eq!(disk_manifest.target_resolution, (1920, 1080));
        assert!(!disk_manifest.bundle_adb);

        // Legacy workflow_versions row with non-standard UUID id should work with ensure_project_job_exists
        {
            let conn = storage.conn.lock().unwrap();
            let now = Utc::now().to_rfc3339();
            conn.execute(
                "INSERT INTO workflows (id, project_id, name, category, draft_json, draft_updated_at)
                 VALUES ('wf-legacy-uuid', ?1, 'Legacy UUID WF', 'Chung', '{}', ?2);",
                params![proj.id, now],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO workflow_versions (id, workflow_id, version_no, graph_json, sha256, created_at)
                 VALUES ('custom-uuid-ver-99', 'wf-legacy-uuid', 1, '{}', 'abc', ?1);",
                params![now],
            )
            .unwrap();
        }
        storage
            .ensure_project_job_exists(
                &proj.id,
                "job-legacy-ver-1",
                "emulator-5554",
                "wf-legacy-uuid",
                "Legacy UUID WF",
                "{}",
            )
            .unwrap();
        let jobs = storage.list_project_jobs(&proj.id).unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].workflow_version_id, "custom-uuid-ver-99");

        // Deleting project removes all dependent records cleanly
        assert!(storage.delete_project(&proj.id, true).unwrap());
        assert!(!Path::new(&proj.project_dir_path).exists());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_p2_1_direct_sql_project_isolation_blocks_cross_project_writes() {
        let temp_dir =
            std::env::temp_dir().join(format!("fp_p2_1_sql_iso_{}", uuid::Uuid::new_v4()));
        let projects_root = temp_dir.join("projects");
        let storage = StorageManager::new_in_memory().unwrap();

        let proj_a = storage
            .create_project(
                CreateProjectRequest {
                    slug: "sql-proj-a".to_string(),
                    name: "SQL Project A".to_string(),
                    game_package_id: "com.sql.a".to_string(),
                    version: None,
                    exe_name: None,
                    window_title: None,
                    icon_rel_path: None,
                    target_width_px: None,
                    target_height_px: None,
                    bundle_adb: None,
                    enabled_modules: None,
                },
                &projects_root,
            )
            .unwrap();
        let proj_b = storage
            .create_project(
                CreateProjectRequest {
                    slug: "sql-proj-b".to_string(),
                    name: "SQL Project B".to_string(),
                    game_package_id: "com.sql.b".to_string(),
                    version: None,
                    exe_name: None,
                    window_title: None,
                    icon_rel_path: None,
                    target_width_px: None,
                    target_height_px: None,
                    bundle_adb: None,
                    enabled_modules: None,
                },
                &projects_root,
            )
            .unwrap();

        storage
            .save_project_workflow(&proj_a.id, "wf-sql-a", "WF A", "NV", "{}")
            .unwrap();
        storage
            .save_project_workflow(&proj_b.id, "wf-sql-b", "WF B", "NV", "{}")
            .unwrap();
        storage
            .upsert_project_account(ProjectAccountInput {
                id: Some("acc-sql-a".to_string()),
                project_id: proj_a.id.clone(),
                alias: "AccA".to_string(),
                game_profile_id: None,
                secret_ref: None,
                state: "ready".to_string(),
            })
            .unwrap();
        storage
            .upsert_project_account(ProjectAccountInput {
                id: Some("acc-sql-b".to_string()),
                project_id: proj_b.id.clone(),
                alias: "AccB".to_string(),
                game_profile_id: None,
                secret_ref: None,
                state: "ready".to_string(),
            })
            .unwrap();

        let conn = storage.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO devices (id, adb_serial, alias, width_px, height_px, rotation_deg, connection_state, updated_at)
             VALUES ('dev-sql-1', 'dev-sql-1', 'Dev1', 1080, 1920, 0, 'ready', ?1);",
            params![now],
        )
        .unwrap();

        // 1. Direct SQL INSERT into task_definitions linking Project B -> Project A's workflow MUST FAIL
        let cross_task_ins = conn.execute(
            "INSERT INTO task_definitions (
                id, project_id, workflow_id, category_id, category_short_tag,
                category_name, category_color_hex, title, priority, estimate_sec,
                default_enabled, settings_schema_json, updated_at
             ) VALUES ('task-sql-cross', ?1, 'wf-sql-a', 'c1', '[C]', 'Cat', '#fff', 'Bad', 1, 10, 1, '[]', ?2);",
            params![proj_b.id, now],
        );
        assert!(cross_task_ins.is_err());

        // 2. Valid same-project task insert works, then cross-project UPDATE MUST FAIL
        conn.execute(
            "INSERT INTO task_definitions (
                id, project_id, workflow_id, category_id, category_short_tag,
                category_name, category_color_hex, title, priority, estimate_sec,
                default_enabled, settings_schema_json, updated_at
             ) VALUES ('task-sql-b-ok', ?1, 'wf-sql-b', 'c1', '[C]', 'Cat', '#fff', 'Good', 1, 10, 1, '[]', ?2);",
            params![proj_b.id, now],
        )
        .unwrap();
        let cross_task_upd = conn.execute(
            "UPDATE task_definitions SET workflow_id = 'wf-sql-a' WHERE id = 'task-sql-b-ok';",
            [],
        );
        assert!(cross_task_upd.is_err());

        // 3. Direct SQL NULL project_id inserts on workflows, accounts, jobs MUST FAIL
        assert!(conn
            .execute(
                "INSERT INTO workflows (id, project_id, name, draft_json, draft_updated_at)
                 VALUES ('wf-null', NULL, 'NullWF', '{}', ?1);",
                params![now],
            )
            .is_err());
        assert!(conn
            .execute(
                "INSERT INTO accounts (id, project_id, alias, state, updated_at)
                 VALUES ('acc-null', NULL, 'NullAcc', 'ready', ?1);",
                params![now],
            )
            .is_err());
        assert!(conn
            .execute(
                "INSERT INTO jobs (id, project_id, device_id, workflow_version_id, priority, state, created_at, updated_at)
                 VALUES ('job-null', NULL, 'dev-sql-1', 'wf-sql-a-v1', 0, 'queued', ?1, ?1);",
                params![now],
            )
            .is_err());

        // 4. Direct SQL cross-project job with Project B owning job but Project A's workflow_version_id MUST FAIL
        let cross_job_wf = conn.execute(
            "INSERT INTO jobs (id, project_id, device_id, account_id, workflow_version_id, priority, state, created_at, updated_at)
             VALUES ('job-cross-wf', ?1, 'dev-sql-1', 'acc-sql-b', 'wf-sql-a-v1', 0, 'queued', ?2, ?2);",
            params![proj_b.id, now],
        );
        assert!(cross_job_wf.is_err());

        // 5. Direct SQL cross-project job with Project A owning job + wf_a_v1, but Project B's account MUST FAIL
        let cross_job_acc = conn.execute(
            "INSERT INTO jobs (id, project_id, device_id, account_id, workflow_version_id, priority, state, created_at, updated_at)
             VALUES ('job-cross-acc', ?1, 'dev-sql-1', 'acc-sql-b', 'wf-sql-a-v1', 0, 'queued', ?2, ?2);",
            params![proj_a.id, now],
        );
        assert!(cross_job_acc.is_err());

        // 6. Valid same-project job + checkpoint + step_event succeed, and reassigning referenced workflow/account across projects MUST FAIL
        conn.execute(
            "INSERT INTO jobs (id, project_id, device_id, account_id, workflow_version_id, priority, state, created_at, updated_at)
             VALUES ('job-sql-a-ok', ?1, 'dev-sql-1', 'acc-sql-a', 'wf-sql-a-v1', 0, 'running', ?2, ?2);",
            params![proj_a.id, now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO checkpoints (id, job_id, node_id, state_json, created_at)
             VALUES ('cp-sql-a-ok', 'job-sql-a-ok', 'n1', '{}', ?1);",
            params![now],
        )
        .unwrap();
        assert!(conn
            .execute(
                "UPDATE workflows SET project_id = ?1 WHERE id = 'wf-sql-a';",
                params![proj_b.id],
            )
            .is_err());
        assert!(conn
            .execute(
                "UPDATE accounts SET project_id = ?1 WHERE id = 'acc-sql-a';",
                params![proj_b.id],
            )
            .is_err());
        assert!(conn
            .execute(
                "UPDATE workflow_versions SET workflow_id = 'wf-sql-b' WHERE id = 'wf-sql-a-v1';",
                [],
            )
            .is_err());
        assert!(conn
            .execute(
                "UPDATE task_definitions SET project_id = ?1, workflow_id = 'wf-sql-a' WHERE id = 'task-sql-b-ok';",
                params![proj_a.id],
            )
            .is_err());
        assert!(conn
            .execute(
                "UPDATE jobs SET project_id = ?1 WHERE id = 'job-sql-a-ok';",
                params![proj_b.id],
            )
            .is_err());

        // 7. Reassigning a workflow with only workflow_versions (no tasks/jobs) or vision_templates across projects MUST FAIL
        drop(conn);
        storage
            .save_project_workflow(&proj_a.id, "wf-sql-a-unlinked", "WF A2", "NV", "{}")
            .unwrap();
        storage
            .register_vision_template(VisionTemplateInput {
                id: Some("tpl-sql-a".to_string()),
                project_id: proj_a.id.clone(),
                name: "tpl_a".to_string(),
                rel_path: "templates/tpl_a.png".to_string(),
                width_px: 10,
                height_px: 10,
                source_crop_x: None,
                source_crop_y: None,
                default_confidence: None,
                sha256: "a".repeat(64),
            })
            .unwrap();
        // Cross-project job ID collision via handler MUST FAIL
        assert!(storage
            .ensure_project_job_exists(
                &proj_b.id,
                "job-sql-a-ok",
                "dev-sql-1",
                "wf-sql-b",
                "WF B",
                "{}"
            )
            .unwrap_err()
            .to_string()
            .contains("PROJECT_ISOLATION_VIOLATION"));
        // Fallback v1 save_workflow_draft must NOT hijack proj_a even when proj_a sorts before DEFAULT_LEGACY_PROJECT_ID
        storage
            .save_workflow_draft("wf-v1-fallback-check", "V1 Fallback", "{}")
            .unwrap();
        assert_eq!(storage.list_project_workflows(&proj_a.id).unwrap().len(), 2);
        assert_eq!(
            storage
                .list_project_workflows(DEFAULT_LEGACY_PROJECT_ID)
                .unwrap()
                .len(),
            1
        );

        let conn = storage.conn.lock().unwrap();
        assert!(conn
            .execute(
                "UPDATE workflows SET project_id = ?1 WHERE id = 'wf-sql-a-unlinked';",
                params![proj_b.id],
            )
            .is_err());
        assert!(conn
            .execute(
                "UPDATE vision_templates SET project_id = ?1 WHERE id = 'tpl-sql-a';",
                params![proj_b.id],
            )
            .is_err());

        let rep = check_integrity_on_conn(&conn).unwrap();
        assert!(rep.is_healthy);
        assert_eq!(rep.foreign_key_violations, 0);

        drop(conn);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_p2_1_crash_consistency_create_clone_update_delete_workflow_template() {
        let temp_dir =
            std::env::temp_dir().join(format!("fp_p2_1_crash_test_{}", uuid::Uuid::new_v4()));
        let projects_root = temp_dir.join("projects");
        let storage = StorageManager::new_in_memory().unwrap();

        // 1. Create fault before DB commit -> zero DB rows, zero orphan folders
        let bad_create = storage.create_project_with_fault(
            CreateProjectRequest {
                slug: "crashed-create".to_string(),
                name: "Crashed Create".to_string(),
                game_package_id: "com.crash.create".to_string(),
                version: None,
                exe_name: None,
                window_title: None,
                icon_rel_path: None,
                target_width_px: None,
                target_height_px: None,
                bundle_adb: None,
                enabled_modules: None,
            },
            &projects_root,
            CrashFaultPoint::FailBeforeDbCommit,
        );
        assert!(bad_create.is_err());
        assert!(storage
            .get_project_by_slug("crashed-create")
            .unwrap()
            .is_none());
        assert!(!projects_root.join("crashed-create").exists());
        assert_eq!(
            std::fs::read_dir(projects_root.join(STAGING_DIR_NAME))
                .unwrap()
                .count(),
            0
        );

        // 2. Create a valid base project and add workflow + template file
        let src = storage
            .create_project(
                CreateProjectRequest {
                    slug: "solid-base".to_string(),
                    name: "Solid Base".to_string(),
                    game_package_id: "com.solid.base".to_string(),
                    version: Some("1.0.0".to_string()),
                    exe_name: None,
                    window_title: None,
                    icon_rel_path: None,
                    target_width_px: None,
                    target_height_px: None,
                    bundle_adb: None,
                    enabled_modules: None,
                },
                &projects_root,
            )
            .unwrap();
        let src_dir = PathBuf::from(&src.project_dir_path);

        storage
            .save_project_workflow(&src.id, "wf-solid-1", "Solid WF", "NV", "{\"v\":1}")
            .unwrap();
        assert!(src_dir.join("workflows").join("wf-solid-1.json").exists());

        storage
            .save_project_template_file(
                VisionTemplateInput {
                    id: Some("tpl-solid-1".to_string()),
                    project_id: src.id.clone(),
                    name: "btn_solid".to_string(),
                    rel_path: "templates/btn_solid.png".to_string(),
                    width_px: 16,
                    height_px: 16,
                    source_crop_x: None,
                    source_crop_y: None,
                    default_confidence: Some(0.9),
                    sha256: String::new(),
                },
                b"fake-png-v1",
            )
            .unwrap();
        assert!(src_dir.join("templates").join("btn_solid.png").exists());

        // 2b. Nested template directory must survive clone_project intact
        storage
            .save_project_template_file(
                VisionTemplateInput {
                    id: Some("tpl-solid-nested".to_string()),
                    project_id: src.id.clone(),
                    name: "btn_nested".to_string(),
                    rel_path: "templates/sub/btn_nested.png".to_string(),
                    width_px: 16,
                    height_px: 16,
                    source_crop_x: None,
                    source_crop_y: None,
                    default_confidence: Some(0.9),
                    sha256: String::new(),
                },
                b"fake-nested-png",
            )
            .unwrap();
        let cloned_nested = storage
            .clone_project(
                CloneProjectRequest {
                    source_project_id: src.id.clone(),
                    new_slug: "solid-nested-clone".to_string(),
                    new_name: "Solid Nested Clone".to_string(),
                    new_game_package_id: None,
                    new_exe_name: None,
                },
                &projects_root,
            )
            .unwrap();
        assert!(PathBuf::from(&cloned_nested.project_dir_path)
            .join("templates")
            .join("sub")
            .join("btn_nested.png")
            .exists());
        assert!(storage.delete_project(&cloned_nested.id, true).unwrap());

        // 3. Fault during save_project_workflow -> rolls back BOTH DB and disk workflow/manifest
        let wf_fault = storage.save_project_workflow_with_fault(
            &src.id,
            "wf-solid-1",
            "Corrupted WF",
            "NV",
            "{\"v\":999}",
            CrashFaultPoint::FailBeforeDbCommit,
        );
        assert!(wf_fault.is_err());
        let wf_on_disk =
            std::fs::read_to_string(src_dir.join("workflows").join("wf-solid-1.json")).unwrap();
        assert_eq!(wf_on_disk, "{\"v\":1}");
        assert_eq!(
            storage.list_project_workflows(&src.id).unwrap()[0].draft_json,
            "{\"v\":1}"
        );

        // 4. Fault during save_project_template_file -> rolls back BOTH DB and disk PNG/manifest
        let tpl_fault = storage.save_project_template_file_with_fault(
            VisionTemplateInput {
                id: Some("tpl-solid-1".to_string()),
                project_id: src.id.clone(),
                name: "btn_solid_bad".to_string(),
                rel_path: "templates/btn_solid.png".to_string(),
                width_px: 32,
                height_px: 32,
                source_crop_x: None,
                source_crop_y: None,
                default_confidence: Some(0.9),
                sha256: String::new(),
            },
            b"corrupted-png-v999",
            CrashFaultPoint::FailBeforeDbCommit,
        );
        assert!(tpl_fault.is_err());
        assert_eq!(
            std::fs::read(src_dir.join("templates").join("btn_solid.png")).unwrap(),
            b"fake-png-v1"
        );
        assert_eq!(
            storage.list_vision_templates(&src.id).unwrap()[0].name,
            "btn_solid"
        );

        // 5. Fault during update_project -> rolls back DB, project.json, AND ui-schema.json
        let upd_fault = storage.update_project_with_fault(
            UpdateProjectRequest {
                id: src.id.clone(),
                name: Some("Broken Rename".to_string()),
                version: Some("9.9.9".to_string()),
                window_title: Some("Broken Window".to_string()),
                ..Default::default()
            },
            CrashFaultPoint::FailBeforeDbCommit,
        );
        assert!(upd_fault.is_err());
        let reloaded_src = storage.get_project(&src.id).unwrap().unwrap();
        assert_eq!(reloaded_src.name, "Solid Base");
        assert_eq!(reloaded_src.version, "1.0.0");
        let manifest_disk: GameProjectManifest =
            serde_json::from_slice(&std::fs::read(src_dir.join("project.json")).unwrap()).unwrap();
        assert_eq!(manifest_disk.name, "Solid Base");
        let ui_disk: MiniToolUiSchema =
            serde_json::from_slice(&std::fs::read(src_dir.join("ui-schema.json")).unwrap())
                .unwrap();
        assert_eq!(ui_disk.window_title, src.window_title);

        // 6. Fault during clone_project (template copy & before commit) -> 0 orphan dirs, 0 DB clone rows
        for fault in [
            CrashFaultPoint::FailTemplateCopy,
            CrashFaultPoint::FailManifestWrite,
            CrashFaultPoint::FailBeforeDbCommit,
        ] {
            let clone_err = storage.clone_project_with_fault(
                CloneProjectRequest {
                    source_project_id: src.id.clone(),
                    new_slug: "crashed-clone".to_string(),
                    new_name: "Crashed Clone".to_string(),
                    new_game_package_id: None,
                    new_exe_name: None,
                },
                &projects_root,
                fault,
            );
            assert!(clone_err.is_err());
            assert!(storage
                .get_project_by_slug("crashed-clone")
                .unwrap()
                .is_none());
            assert!(!projects_root.join("crashed-clone").exists());
        }

        // 7. Fault during delete_project BEFORE DB commit -> project directory MUST be restored intact from .fp_trash!
        let del_rollback =
            storage.delete_project_with_fault(&src.id, true, CrashFaultPoint::FailBeforeDbCommit);
        assert!(del_rollback.is_err());
        assert!(storage.get_project(&src.id).unwrap().is_some());
        assert!(src_dir.join("project.json").exists());
        assert!(src_dir.join("templates").join("btn_solid.png").exists());

        // 2b. Nested template directory must survive clone_project intact
        storage
            .save_project_template_file(
                VisionTemplateInput {
                    id: Some("tpl-solid-nested".to_string()),
                    project_id: src.id.clone(),
                    name: "btn_nested".to_string(),
                    rel_path: "templates/sub/btn_nested.png".to_string(),
                    width_px: 16,
                    height_px: 16,
                    source_crop_x: None,
                    source_crop_y: None,
                    default_confidence: Some(0.9),
                    sha256: String::new(),
                },
                b"fake-nested-png",
            )
            .unwrap();
        let cloned_nested = storage
            .clone_project(
                CloneProjectRequest {
                    source_project_id: src.id.clone(),
                    new_slug: "solid-nested-clone".to_string(),
                    new_name: "Solid Nested Clone".to_string(),
                    new_game_package_id: None,
                    new_exe_name: None,
                },
                &projects_root,
            )
            .unwrap();
        assert!(PathBuf::from(&cloned_nested.project_dir_path)
            .join("templates")
            .join("sub")
            .join("btn_nested.png")
            .exists());
        assert!(storage.delete_project(&cloned_nested.id, true).unwrap());

        // 8. Fault during delete_project AFTER DB commit (trash purge interrupted) -> DB gone, dir gone, reconcile cleans trash!
        let del_orphan = storage.delete_project_with_fault(
            &src.id,
            true,
            CrashFaultPoint::FailAfterDbCommitTrashPurge,
        );
        assert!(del_orphan.is_err());
        assert!(storage.get_project(&src.id).unwrap().is_none());
        assert!(!src_dir.exists());
        let cleaned = storage.reconcile_projects_storage(&projects_root).unwrap();
        assert!(cleaned >= 1);
        assert_eq!(
            std::fs::read_dir(projects_root.join(TRASH_DIR_NAME))
                .unwrap()
                .count(),
            0
        );

        // 9. Journal/Trash safety: if project is STILL active in DB and restore cannot overwrite an existing dir,
        // reconcile_projects_storage MUST preserve both trash & journal rather than destroying active project data!
        let p_keep = storage
            .create_project(
                CreateProjectRequest {
                    slug: "keep-active".to_string(),
                    name: "Keep Active".to_string(),
                    game_package_id: "com.keep.active".to_string(),
                    version: None,
                    exe_name: None,
                    window_title: None,
                    icon_rel_path: None,
                    target_width_px: None,
                    target_height_px: None,
                    bundle_adb: None,
                    enabled_modules: None,
                },
                &projects_root,
            )
            .unwrap();
        let keep_orig = PathBuf::from(&p_keep.project_dir_path);
        let keep_trash = projects_root.join(TRASH_DIR_NAME).join("keep-active.saved");
        std::fs::create_dir_all(&keep_trash).unwrap();
        std::fs::write(keep_trash.join("important.txt"), b"must-not-lose").unwrap();
        let keep_journal = projects_root
            .join(JOURNAL_DIR_NAME)
            .join(format!("delete_{}.json", p_keep.id));
        let j_entry = DeleteJournalEntry {
            project_id: p_keep.id.clone(),
            project_slug: p_keep.slug.clone(),
            original_dir: keep_orig.to_string_lossy().to_string(),
            trash_dir: keep_trash.to_string_lossy().to_string(),
            created_at: Utc::now().to_rfc3339(),
        };
        atomic_write_bytes(&keep_journal, &serde_json::to_vec_pretty(&j_entry).unwrap()).unwrap();
        let _ = storage.reconcile_projects_storage(&projects_root).unwrap();
        assert!(keep_trash.join("important.txt").exists());
        assert!(keep_journal.exists());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_p2_1_template_path_and_project_dir_containment() {
        let temp_dir =
            std::env::temp_dir().join(format!("fp_p2_1_path_guard_{}", uuid::Uuid::new_v4()));
        let projects_root = temp_dir.join("projects");
        let outside_dir = temp_dir.join("outside_secret");
        std::fs::create_dir_all(&outside_dir).unwrap();
        std::fs::write(outside_dir.join("leak.png"), b"secret-bytes").unwrap();

        let storage = StorageManager::new_in_memory().unwrap();
        let proj = storage
            .create_project(
                CreateProjectRequest {
                    slug: "safe-proj".to_string(),
                    name: "Safe Proj".to_string(),
                    game_package_id: "com.safe.proj".to_string(),
                    version: None,
                    exe_name: None,
                    window_title: None,
                    icon_rel_path: None,
                    target_width_px: None,
                    target_height_px: None,
                    bundle_adb: None,
                    enabled_modules: None,
                },
                &projects_root,
            )
            .unwrap();

        // Reject traversal, absolute paths, and Windows drive prefixes
        for bad_rel in [
            "../outside_secret/leak.png",
            "templates/../../outside_secret/leak.png",
            "/etc/passwd",
            "C:\\Windows\\System32\\cmd.exe",
            "templates\\..\\..\\leak.png",
        ] {
            assert!(storage
                .register_vision_template(VisionTemplateInput {
                    id: None,
                    project_id: proj.id.clone(),
                    name: "bad".to_string(),
                    rel_path: bad_rel.to_string(),
                    width_px: 10,
                    height_px: 10,
                    source_crop_x: None,
                    source_crop_y: None,
                    default_confidence: None,
                    sha256: "a".repeat(64),
                })
                .is_err());
        }

        // Reject NTFS junction inside project templates/ pointing outside
        #[cfg(windows)]
        {
            let junction_dir = PathBuf::from(&proj.project_dir_path)
                .join("templates")
                .join("junc_out");
            let out = std::process::Command::new("cmd")
                .args([
                    "/C",
                    "mklink",
                    "/J",
                    &junction_dir.to_string_lossy(),
                    &outside_dir.to_string_lossy(),
                ])
                .output();
            if let Ok(o) = out {
                if o.status.success() {
                    let res = storage.register_vision_template(VisionTemplateInput {
                        id: None,
                        project_id: proj.id.clone(),
                        name: "junc".to_string(),
                        rel_path: "templates/junc_out/leak.png".to_string(),
                        width_px: 10,
                        height_px: 10,
                        source_crop_x: None,
                        source_crop_y: None,
                        default_confidence: None,
                        sha256: "a".repeat(64),
                    });
                    assert!(res.is_err());
                    assert!(res
                        .unwrap_err()
                        .to_string()
                        .contains("PROJECT_ISOLATION_VIOLATION"));
                    let _ = std::fs::remove_dir(&junction_dir);
                }
            }
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_p2_1_jx2_migration_preserves_user_edits_and_emits_mapping() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let repo_root = manifest_dir.parent().unwrap().parent().unwrap();
        let legacy_jx2_dir = repo_root.join("games").join("jx2mobile");

        let temp_dir =
            std::env::temp_dir().join(format!("fp_p2_1_jx2_preserve_{}", uuid::Uuid::new_v4()));
        let db_file = temp_dir.join("jx2_user_custom.db");
        let projects_dir = temp_dir.join("projects");

        let storage = StorageManager::new(&db_file).unwrap();
        // First migration on clean DB -> 10 INSERTs
        let sum1 = storage
            .migrate_legacy_jx2_assets(&legacy_jx2_dir, &projects_dir)
            .unwrap();
        assert_eq!(sum1.workflow_migrations.len(), 10);
        assert!(sum1
            .workflow_migrations
            .iter()
            .all(|m| m.action == "INSERT"));

        // User edits `wf-jx2-02-char` graph in Studio
        let user_custom_json = r#"{"schemaVersion":1,"workflowId":"wf-jx2-02-char","name":"Custom User Char","status":"draft","graph":{"nodes":[{"id":"custom-start","type":"flow.start","position":{"x":0,"y":0}},{"id":"custom-end","type":"flow.end","position":{"x":100,"y":0}}],"edges":[{"id":"ce1","source":"custom-start","sourcePort":"flow","target":"custom-end","targetPort":"flow"}]}}"#;
        storage
            .save_project_workflow(
                DEFAULT_LEGACY_PROJECT_ID,
                "wf-jx2-02-char",
                "Custom User Char",
                "Tạo nhân vật",
                user_custom_json,
            )
            .unwrap();

        // Re-running JX2 migration MUST SKIP `wf-jx2-02-char` (preserving user graph) and mark others ALREADY_EXISTS
        let sum2 = storage
            .migrate_legacy_jx2_assets(&legacy_jx2_dir, &projects_dir)
            .unwrap();
        assert_eq!(sum2.workflow_migrations.len(), 10);
        let skipped = sum2
            .workflow_migrations
            .iter()
            .find(|m| m.dest_workflow_id == "wf-jx2-02-char")
            .unwrap();
        assert_eq!(skipped.action, "SKIP");
        assert_eq!(
            skipped.content_comparison,
            "PRESERVED_USER_CUSTOMIZED_WORKFLOW"
        );

        let wfs = storage
            .list_project_workflows(DEFAULT_LEGACY_PROJECT_ID)
            .unwrap();
        let char_wf = wfs.iter().find(|w| w.id == "wf-jx2-02-char").unwrap();
        assert_eq!(char_wf.name, "Custom User Char");
        assert!(char_wf.draft_json.contains("custom-start"));

        drop(storage);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_p2_1_wal_rollback_runbook_roundtrip() {
        let temp_dir =
            std::env::temp_dir().join(format!("fp_p2_1_rollback_runbook_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let live_db = temp_dir.join("live.db");
        let pre_v2_backup = temp_dir.join("pre_v2_backup.db");
        let safety_snap = temp_dir.join("pre_rollback_safety.db");
        let projects_root = temp_dir.join("projects");

        // 1. Seed a v1 DB and capture verified pre-v2 backup
        {
            let conn = Connection::open(&live_db).unwrap();
            conn.execute_batch("PRAGMA journal_mode = WAL;").unwrap();
            conn.execute_batch(migrations::MIGRATION_V1_SQL).unwrap();
            let now = Utc::now().to_rfc3339();
            conn.execute(
                "INSERT INTO workflows (id, name, draft_json, draft_updated_at)
                 VALUES ('wf-v1-only', 'V1 Original', '{}', ?1);",
                params![now],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO workflow_versions (id, workflow_id, version_no, graph_json, sha256, created_at)
                 VALUES ('wf-v1-only-v1', 'wf-v1-only', 1, '{}', 'abc', ?1);",
                params![now],
            )
            .unwrap();
            let _ = conn.query_row("PRAGMA wal_checkpoint(TRUNCATE);", [], |r| {
                r.get::<_, i64>(0)
            });
            let mut bk_conn = Connection::open(&pre_v2_backup).unwrap();
            let bk = rusqlite::backup::Backup::new(&conn, &mut bk_conn).unwrap();
            bk.run_to_completion(100, Duration::from_millis(5), None)
                .unwrap();
        }

        // 2. Upgrade live DB to latest and write project data
        {
            let storage = StorageManager::new(&live_db).unwrap();
            assert_eq!(
                storage.applied_migration_versions().unwrap(),
                vec![1, 2, 3, 4, 5, 6]
            );
            let _ = storage
                .create_project(
                    CreateProjectRequest {
                        slug: "v2-extra".to_string(),
                        name: "V2 Extra".to_string(),
                        game_package_id: "com.v2.extra".to_string(),
                        version: None,
                        exe_name: None,
                        window_title: None,
                        icon_rel_path: None,
                        target_width_px: None,
                        target_height_px: None,
                        bundle_adb: None,
                        enabled_modules: None,
                    },
                    &projects_root,
                )
                .unwrap();
        }

        // 3. Execute standardized WAL rollback runbook (all connections closed -> checkpoint -> safety snap -> restore -> verify -> reopen)
        let report =
            StorageManager::rollback_database_from_backup(&live_db, &pre_v2_backup, &safety_snap)
                .unwrap();
        assert_eq!(report.pre_rollback_versions, vec![1, 2, 3, 4, 5, 6]);
        assert!(report.post_rollback_versions.is_empty());
        assert!(report.post_rollback_integrity.is_healthy);
        assert_eq!(report.post_rollback_integrity.foreign_key_violations, 0);
        assert!(report.reopen_verified);
        assert!(safety_snap.exists());

        // 4. Verify v1 workflow is intact and v2 tables are gone
        let reopened = Connection::open(&live_db).unwrap();
        let wf_cnt: i64 = reopened
            .query_row("SELECT COUNT(*) FROM workflows;", [], |r| r.get(0))
            .unwrap();
        assert_eq!(wf_cnt, 1);
        let proj_tbl_cnt: i64 = reopened
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='projects';",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(proj_tbl_cnt, 0);

        drop(reopened);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_p2_hotfix_wf_jx2_01_login_draft_to_published_does_not_auto_enable_unapproved_tasks() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let repo_root = manifest_dir.parent().unwrap().parent().unwrap();
        let legacy_jx2_dir = repo_root.join("games").join("jx2mobile");

        let temp_dir =
            std::env::temp_dir().join(format!("fp_p2_hotfix_wf_upgrade_{}", uuid::Uuid::new_v4()));
        let db_file = temp_dir.join("jx2_draft_upgrade.db");
        let projects_dir = temp_dir.join("projects");
        std::fs::create_dir_all(&temp_dir).unwrap();

        // 1. Seed a v1 DB containing `wf-jx2-01-login` as `"status":"draft"` with the exact same graph as `01_start_and_login.json`
        let published_src =
            std::fs::read_to_string(legacy_jx2_dir.join("01_start_and_login.json")).unwrap();
        let mut draft_val: serde_json::Value = serde_json::from_str(&published_src).unwrap();
        draft_val
            .as_object_mut()
            .unwrap()
            .insert("status".to_string(), serde_json::json!("draft"));
        let draft_json = serde_json::to_string_pretty(&draft_val).unwrap();
        let now = Utc::now().to_rfc3339();

        {
            let conn = Connection::open(&db_file).unwrap();
            conn.execute_batch("PRAGMA journal_mode = WAL;").unwrap();
            conn.execute_batch(migrations::MIGRATION_V1_SQL).unwrap();
            conn.execute(
                "INSERT INTO workflows (id, name, draft_json, draft_updated_at) VALUES (?1, ?2, ?3, ?4);",
                params!["wf-jx2-01-login", "01. Khởi động và Đăng nhập (Draft)", draft_json, now],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO workflow_versions (id, workflow_id, version_no, graph_json, sha256, created_at)
                 VALUES ('wf-jx2-01-login-v1', 'wf-jx2-01-login', 1, ?1, 'sha256-placeholder', ?2);",
                params![draft_json, now],
            )
            .unwrap();
        }

        // 2. Open with v2 StorageManager, seed an unapproved task (`default_enabled = 0`) bound to `wf-jx2-01-login`
        let storage = StorageManager::new(&db_file).unwrap();
        {
            let conn = storage.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO projects (
                    id, slug, name, game_package_id, version, exe_name, window_title,
                    icon_rel_path, target_width_px, target_height_px, bundle_adb,
                    project_dir_path, created_at, updated_at
                 ) VALUES (
                    ?1, ?2, 'Võ Hiệp Truyền Kỳ 2 Mobile', 'com.blackhole.jx2mobile', '1.0.0',
                    'Auto_JX2_Mobile.exe', 'Auto Võ Hiệp 2 Mobile v1.0.0', 'branding/icon.ico',
                    1920, 1080, 1, ?3, ?4, ?4
                 ) ON CONFLICT(id) DO NOTHING;",
                params![
                    DEFAULT_LEGACY_PROJECT_ID,
                    DEFAULT_LEGACY_PROJECT_SLUG,
                    projects_dir
                        .join(DEFAULT_LEGACY_PROJECT_SLUG)
                        .to_string_lossy()
                        .to_string(),
                    now
                ],
            )
            .unwrap();
            conn.execute(
                "UPDATE workflows SET project_id = ?1 WHERE id = 'wf-jx2-01-login';",
                params![DEFAULT_LEGACY_PROJECT_ID],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO task_definitions (
                    id, project_id, workflow_id, category_id, category_short_tag,
                    category_name, category_color_hex, title, priority, estimate_sec,
                    default_enabled, settings_schema_json, updated_at
                 ) VALUES (
                    'task-unapproved-login', ?1, 'wf-jx2-01-login', 'cat-nv', '[NV]',
                    'Nhiệm vụ Ngày', '#38BDF8', 'Auto Đăng nhập Chưa Phê Duyệt', 1, 25,
                    0, '[]', ?2
                 );",
                params![DEFAULT_LEGACY_PROJECT_ID, now],
            )
            .unwrap();
        }

        // 3. Upgrade via migrate_legacy_jx2_assets (`wf-jx2-01-login` draft -> published v2)
        let summary = storage
            .migrate_legacy_jx2_assets(&legacy_jx2_dir, &projects_dir)
            .unwrap();
        let wf1_mig = summary
            .workflow_migrations
            .iter()
            .find(|m| m.dest_workflow_id == "wf-jx2-01-login")
            .unwrap();
        assert_eq!(wf1_mig.action, "UPDATE");
        assert_eq!(wf1_mig.version_after, 2);

        let wf1 = storage
            .list_project_workflows(DEFAULT_LEGACY_PROJECT_ID)
            .unwrap()
            .into_iter()
            .find(|w| w.id == "wf-jx2-01-login")
            .unwrap();
        let wf1_val: serde_json::Value = serde_json::from_str(&wf1.draft_json).unwrap();
        assert_eq!(
            wf1_val.get("status").and_then(|v| v.as_str()),
            Some("published")
        );

        // CRITICAL CHECK: Unapproved task MUST remain disabled (default_enabled = false), and ZERO jobs auto-queued!
        let tasks = storage
            .list_project_tasks(DEFAULT_LEGACY_PROJECT_ID)
            .unwrap();
        let unapproved = tasks
            .iter()
            .find(|t| t.id == "task-unapproved-login")
            .expect("Unapproved task preserved");
        assert!(!unapproved.default_enabled);

        let jobs_count: i64 = {
            let conn = storage.conn.lock().unwrap();
            conn.query_row("SELECT COUNT(*) FROM jobs;", [], |r| r.get(0))
                .unwrap()
        };
        assert_eq!(jobs_count, 0);

        // 4. Also verify direct `save_project_workflow` upgrade draft -> published leaves tasks & jobs untouched
        let mut custom_draft = wf1_val.clone();
        custom_draft
            .as_object_mut()
            .unwrap()
            .insert("status".to_string(), serde_json::json!("published"));
        storage
            .save_project_workflow(
                DEFAULT_LEGACY_PROJECT_ID,
                "wf-jx2-01-login",
                "01. Khởi động và Đăng nhập (Published)",
                "Khởi động & Đăng nhập",
                &serde_json::to_string_pretty(&custom_draft).unwrap(),
            )
            .unwrap();
        let tasks_after = storage
            .list_project_tasks(DEFAULT_LEGACY_PROJECT_ID)
            .unwrap();
        assert!(
            !tasks_after
                .iter()
                .find(|t| t.id == "task-unapproved-login")
                .unwrap()
                .default_enabled
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_p5_1_migration_v3_legacy_upgrade_and_wal_rollback() {
        let temp_dir =
            std::env::temp_dir().join(format!("fp_p5_1_v3_mig_{}", uuid::Uuid::new_v4()));
        let db_file = temp_dir.join("runtime_legacy_p4.db");
        let pre_v3_backup = temp_dir.join("runtime_pre_v3.bak.db");
        let safety_snap = temp_dir.join("runtime_safety.db");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let now = Utc::now().to_rfc3339();

        // 1. Create a genuine v2 database with P4 `runtime_variable_snapshots`
        {
            let conn = Connection::open(&db_file).unwrap();
            conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")
                .unwrap();
            conn.execute(
                "CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);",
                [],
            ).unwrap();
            conn.execute_batch(migrations::MIGRATION_V1_SQL).unwrap();
            conn.execute_batch(migrations::MIGRATION_V2_TABLES_SQL)
                .unwrap();
            conn.execute_batch(
                "ALTER TABLE workflows ADD COLUMN project_id TEXT REFERENCES projects(id);
                 ALTER TABLE workflows ADD COLUMN category TEXT NOT NULL DEFAULT 'Chung';
                 ALTER TABLE accounts ADD COLUMN project_id TEXT REFERENCES projects(id);
                 ALTER TABLE jobs ADD COLUMN project_id TEXT REFERENCES projects(id);",
            )
            .unwrap();
            conn.execute_batch(migrations::MIGRATION_V2_INDEXES_SQL)
                .unwrap();
            conn.execute_batch(migrations::MIGRATION_V2_ISOLATION_TRIGGERS_SQL)
                .unwrap();
            conn.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (1, ?1);",
                params![now],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (2, ?1);",
                params![now],
            )
            .unwrap();
            conn.execute(
                "CREATE TABLE IF NOT EXISTS runtime_variable_snapshots (
                    scope_key TEXT PRIMARY KEY,
                    variables_json TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO runtime_variable_snapshots (scope_key, variables_json, updated_at)
                 VALUES ('mini:jx2-mobile', '{\"char_gold\":5000,\"api_token\":\"live-tok-99\"}', ?1);",
                params![now],
            ).unwrap();

            let _ = conn.query_row("PRAGMA wal_checkpoint(TRUNCATE);", [], |r| {
                r.get::<_, i64>(0)
            });
            let mut bk_conn = Connection::open(&pre_v3_backup).unwrap();
            let bk = rusqlite::backup::Backup::new(&conn, &mut bk_conn).unwrap();
            bk.run_to_completion(100, Duration::from_millis(5), None)
                .unwrap();
        }

        // 2. Open with v6 StorageManager -> migrates legacy P4 snapshots to LEGACY_UNVERIFIED + scrubs secrets
        {
            let storage = StorageManager::new(&db_file).unwrap();
            assert_eq!(
                storage.applied_migration_versions().unwrap(),
                vec![1, 2, 3, 4, 5, 6]
            );
            let snap = storage
                .load_runtime_snapshot_with_recovery("mini:jx2-mobile", None)
                .unwrap();

            let gold_env = snap
                .live_envelopes
                .get("char_gold")
                .expect("char_gold preserved");
            assert_eq!(gold_env.value, serde_json::json!(5000));
            assert_eq!(gold_env.source, VariableSource::LegacyUnverified);
            assert!(!gold_env.verified);

            let tok_env = snap
                .live_envelopes
                .get("api_token")
                .expect("api_token entry");
            assert!(tok_env.redacted);
            assert_eq!(
                tok_env.value,
                serde_json::json!(fp_security::REDACTED_PLACEHOLDER)
            );
        }

        // 3. Verify rollback back to pre-v3 backup cleanly restores pre-v3 schema versions [1, 2]
        let rb =
            StorageManager::rollback_database_from_backup(&db_file, &pre_v3_backup, &safety_snap)
                .unwrap();
        assert_eq!(rb.pre_rollback_versions, vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(rb.post_rollback_versions, vec![1, 2]);
        assert!(rb.post_rollback_integrity.is_healthy);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_p5_1_anti_overwrite_live_vs_testkit_and_stream_recovery() {
        let storage = StorageManager::new_in_memory().unwrap();
        let now = Utc::now().to_rfc3339();

        // 1. Record verified LIVE variable in primary scope
        let live_env = RuntimeVariableEnvelope {
            project_id: "proj-alpha".to_string(),
            session_id: "sess-live-1".to_string(),
            job_id: "job-live-1".to_string(),
            name: "char_gold".to_string(),
            value: serde_json::json!(88800),
            value_type: "number".to_string(),
            source: VariableSource::LiveAdbOcr,
            verified: true,
            confidence: Some(0.96),
            confidence_source: Some(fp_domain::ConfidenceSource::Measured),
            measured_confidence: Some(0.96),
            rule_matched: Some(true),
            device_id: Some("emulator-5554".to_string()),
            updated_at_utc: now.clone(),
            workflow_version: "wf@v1".to_string(),
            seq: 0,
            redacted: false,
        };
        let ev1 = storage
            .record_runtime_var_deltas(
                "mini:alpha",
                "proj-alpha",
                "sess-live-1",
                "job-live-1",
                "wf@v1",
                vec![live_env],
            )
            .unwrap();
        assert_eq!(ev1.seq, 1);
        assert_eq!(ev1.scope_key, "mini:alpha");

        // 2. Direct attempt by TESTKIT to overwrite LIVE verified variable is rejected fail-closed
        let guard_err = storage
            .try_write_testkit_over_live_guard("mini:alpha", "char_gold")
            .unwrap_err()
            .to_string();
        assert!(guard_err.contains("PROVENANCE_ANTI_OVERWRITE_BLOCKED"));

        // 3. Normal TESTKIT writes automatically route to isolated `mini:alpha::testkit` scope & monotonic seq
        let tk_env1 = RuntimeVariableEnvelope {
            project_id: "proj-alpha".to_string(),
            session_id: "sess-tk-1".to_string(),
            job_id: "job-tk-1".to_string(),
            name: "char_gold".to_string(),
            value: serde_json::json!(111),
            value_type: "number".to_string(),
            source: VariableSource::TestkitMock,
            verified: false,
            confidence: None,
            confidence_source: None,
            measured_confidence: None,
            rule_matched: None,
            device_id: None,
            updated_at_utc: now.clone(),
            workflow_version: "wf@v1".to_string(),
            seq: 0,
            redacted: false,
        };
        let tk_ev1 = storage
            .record_runtime_var_deltas(
                "mini:alpha",
                "proj-alpha",
                "sess-tk-1",
                "job-tk-1",
                "wf@v1",
                vec![tk_env1],
            )
            .unwrap();
        assert_eq!(tk_ev1.scope_key, "mini:alpha::testkit");
        assert_eq!(tk_ev1.seq, 1);

        let tk_env2 = RuntimeVariableEnvelope {
            project_id: "proj-alpha".to_string(),
            session_id: "sess-tk-2".to_string(),
            job_id: "job-tk-2".to_string(),
            name: "resource_count".to_string(),
            value: serde_json::json!(250),
            value_type: "number".to_string(),
            source: VariableSource::TestkitMock,
            verified: false,
            confidence: None,
            confidence_source: None,
            measured_confidence: None,
            rule_matched: None,
            device_id: None,
            updated_at_utc: now,
            workflow_version: "wf@v1".to_string(),
            seq: 0,
            redacted: false,
        };
        let tk_ev2 = storage
            .record_runtime_var_deltas(
                "mini:alpha",
                "proj-alpha",
                "sess-tk-2",
                "job-tk-2",
                "wf@v1",
                vec![tk_env2],
            )
            .unwrap();
        assert_eq!(tk_ev2.seq, 2);

        // 4. Verify recovery with since_seq recovers ONLY missed events & LIVE stays 88800 untouched!
        let mut since = HashMap::new();
        since.insert("mini:alpha".to_string(), 1);
        since.insert("mini:alpha::testkit".to_string(), 1);
        let rec = storage
            .load_runtime_snapshot_with_recovery("mini:alpha", Some(&since))
            .unwrap();
        assert_eq!(
            rec.live_envelopes.get("char_gold").unwrap().value,
            serde_json::json!(88800)
        );
        assert!(rec.live_envelopes.get("char_gold").unwrap().verified);
        assert_eq!(
            rec.testkit_envelopes.get("char_gold").unwrap().value,
            serde_json::json!(111)
        );
        assert!(!rec.testkit_envelopes.get("char_gold").unwrap().verified);
        assert_eq!(rec.missed_events.len(), 1);
        assert_eq!(rec.missed_events[0].seq, 2);
        assert_eq!(rec.missed_events[0].deltas[0].name, "resource_count");
    }

    #[test]
    fn test_p5_1_backlog_persistent_wal_reader_concurrency_stress_and_compaction() {
        let temp_dir =
            std::env::temp_dir().join(format!("fp_p5_1_wal_stress_{}", uuid::Uuid::new_v4()));
        let db_file = temp_dir.join("wal_stress.db");
        let projects_dir = temp_dir.join("projects");
        std::fs::create_dir_all(&projects_dir).unwrap();

        let storage_w1 = StorageManager::new(&db_file).unwrap();
        let storage_w2 = StorageManager::new(&db_file).unwrap();
        let mut reader = ExternalWalReader::open(&db_file).unwrap();
        let mut seen_seqs: HashMap<String, u64> = HashMap::new();

        // Initial poll records initial data_version on this single persistent reader Connection
        let (_chg0, ev0) = reader.poll_deltas("mini:stress", &seen_seqs).unwrap();
        assert!(ev0.is_empty());
        let dv0 = reader.last_observed_data_version();
        assert!(dv0.is_some());

        // Concurrent writers (w1 & w2) writing 120 events across two threads (60 each = 120 total)
        // Measure commit latency distribution + test PRAGMA data_version change detection on persistent connection!
        let latencies_us =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::<u64>::with_capacity(120)));
        let l1 = latencies_us.clone();
        let l2 = latencies_us.clone();
        let s1 = storage_w1.clone();
        let s2 = storage_w2.clone();

        let h1 = std::thread::spawn(move || {
            for i in 0..60 {
                let t0 = std::time::Instant::now();
                let env = RuntimeVariableEnvelope {
                    project_id: "stress".to_string(),
                    session_id: "w1".to_string(),
                    job_id: format!("j1-{}", i),
                    name: "counter_a".to_string(),
                    value: serde_json::json!(i),
                    value_type: "number".to_string(),
                    source: VariableSource::TestkitMock,
                    verified: false,
                    confidence: None,
                    confidence_source: None,
                    measured_confidence: None,
                    rule_matched: None,
                    device_id: None,
                    updated_at_utc: Utc::now().to_rfc3339(),
                    workflow_version: "wf@v1".to_string(),
                    seq: 0,
                    redacted: false,
                };
                s1.record_runtime_var_deltas(
                    "mini:stress",
                    "stress",
                    "w1",
                    "j1",
                    "wf@v1",
                    vec![env],
                )
                .unwrap();
                l1.lock().unwrap().push(t0.elapsed().as_micros() as u64);
            }
        });
        let h2 = std::thread::spawn(move || {
            for i in 0..60 {
                let t0 = std::time::Instant::now();
                let env = RuntimeVariableEnvelope {
                    project_id: "stress".to_string(),
                    session_id: "w2".to_string(),
                    job_id: format!("j2-{}", i),
                    name: "counter_b".to_string(),
                    value: serde_json::json!(i),
                    value_type: "number".to_string(),
                    source: VariableSource::TestkitMock,
                    verified: false,
                    confidence: None,
                    confidence_source: None,
                    measured_confidence: None,
                    rule_matched: None,
                    device_id: None,
                    updated_at_utc: Utc::now().to_rfc3339(),
                    workflow_version: "wf@v1".to_string(),
                    seq: 0,
                    redacted: false,
                };
                s2.record_runtime_var_deltas(
                    "mini:stress",
                    "stress",
                    "w2",
                    "j2",
                    "wf@v1",
                    vec![env],
                )
                .unwrap();
                l2.lock().unwrap().push(t0.elapsed().as_micros() as u64);
            }
        });

        h1.join().unwrap();
        h2.join().unwrap();

        // Persistent reader detects data_version change and recovers ALL 120 events without missing 1!
        let (changed, events) = reader.poll_deltas("mini:stress", &seen_seqs).unwrap();
        assert!(changed);
        assert_eq!(events.len(), 120);
        for (idx, ev) in events.iter().enumerate() {
            assert_eq!(ev.seq, (idx as u64) + 1);
            seen_seqs.insert(ev.stream_id.clone(), ev.seq);
        }

        // Subsequent poll with zero new commits reports changed == false on same persistent connection!
        let (changed2, events2) = reader.poll_deltas("mini:stress", &seen_seqs).unwrap();
        assert!(!changed2);
        assert!(events2.is_empty());

        let mut sorted_lat = latencies_us.lock().unwrap().clone();
        sorted_lat.sort_unstable();
        let p50_us = sorted_lat[sorted_lat.len() / 2];
        let p95_us = sorted_lat[(sorted_lat.len() * 95) / 100];
        let wal_bytes = std::fs::metadata(temp_dir.join("wal_stress.db-wal"))
            .map(|m| m.len())
            .unwrap_or(0);
        eprintln!(
            "[P5.1 WAL Benchmark] 120 concurrent writes: P50={}us, P95={}us, WAL={}B, busy_retries={}",
            p50_us, p95_us, wal_bytes, reader.busy_retry_count
        );

        // Verify event log compaction keeps latest 25 events while preserving last_seq=120 and latest values!
        let deleted = storage_w1
            .compact_runtime_var_events("mini:stress::testkit", 25)
            .unwrap();
        assert_eq!(deleted, 95);
        let snap = storage_w1
            .load_runtime_snapshot_with_recovery("mini:stress", None)
            .unwrap();
        assert_eq!(snap.stream_seqs.get("mini:stress::testkit"), Some(&120));
        assert_eq!(snap.missed_events.len(), 25);
        assert!(snap.testkit_envelopes.contains_key("counter_a"));
        assert!(snap.testkit_envelopes.contains_key("counter_b"));

        drop(reader);
        drop(storage_w1);
        drop(storage_w2);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_p5_2_release_lifecycle_transitions_and_semver() {
        let temp_dir =
            std::env::temp_dir().join(format!("fp_p5_2_rel_lifecycle_{}", uuid::Uuid::new_v4()));
        let projects_root = temp_dir.join("projects");
        let db_file = temp_dir.join("studio.db");
        let storage = StorageManager::new(&db_file).unwrap();
        let proj = storage
            .create_project(
                CreateProjectRequest {
                    slug: "lifecycle-game".to_string(),
                    name: "Lifecycle Game".to_string(),
                    game_package_id: "com.lifecycle.game".to_string(),
                    version: Some("1.0.0".to_string()),
                    exe_name: None,
                    window_title: None,
                    icon_rel_path: None,
                    target_width_px: None,
                    target_height_px: None,
                    bundle_adb: None,
                    enabled_modules: None,
                },
                &projects_root,
            )
            .unwrap();

        // 1. Seed RC 1.0.0 -> Promote to Published
        let r1 = ProjectReleaseRecord {
            id: "rel-100".to_string(),
            project_id: proj.id.clone(),
            version: "1.0.0".to_string(),
            output_dir_path: "rel/1.0.0".to_string(),
            exe_path: "rel/1.0.0/Auto.exe".to_string(),
            exe_size_bytes: 1000,
            exe_sha256: "aaa".to_string(),
            key_id: "k1".to_string(),
            publisher_public_key_hex: "pub".to_string(),
            pe_rcdata_pubkey_id: 7777,
            signature_ed25519_hex: "sig".to_string(),
            build_report_json: "{}".to_string(),
            built_at: "2026-11-22T01:00:00Z".to_string(),
            lifecycle_state: "RC".to_string(),
            changelog_md: "Initial RC".to_string(),
            ..Default::default()
        };
        storage.record_project_release(r1).unwrap();
        let p1 = storage
            .transition_project_release_lifecycle(&proj.id, "rel-100", "Published", "Promote 1.0.0")
            .unwrap();
        assert_eq!(p1.lifecycle_state, "Published");

        // 2. Seed RC 1.1.0 -> Promote to Published -> automatically deprecates 1.0.0!
        let r2 = ProjectReleaseRecord {
            id: "rel-110".to_string(),
            project_id: proj.id.clone(),
            version: "1.1.0".to_string(),
            output_dir_path: "rel/1.1.0".to_string(),
            exe_path: "rel/1.1.0/Auto.exe".to_string(),
            exe_size_bytes: 1200,
            exe_sha256: "bbb".to_string(),
            key_id: "k1".to_string(),
            publisher_public_key_hex: "pub".to_string(),
            pe_rcdata_pubkey_id: 7777,
            signature_ed25519_hex: "sig2".to_string(),
            build_report_json: "{}".to_string(),
            built_at: "2026-11-22T02:00:00Z".to_string(),
            lifecycle_state: "RC".to_string(),
            changelog_md: "Add new raid task".to_string(),
            ..Default::default()
        };
        storage.record_project_release(r2).unwrap();
        let _ = storage
            .transition_project_release_lifecycle(&proj.id, "rel-110", "Published", "Ship 1.1.0")
            .unwrap();

        let old_after = storage.get_project_release("rel-100").unwrap().unwrap();
        assert_eq!(old_after.lifecycle_state, "Deprecated");

        // 3. Rollback to rel-100 -> reactivates rel-100 as Published and marks rel-110 as RolledBack!
        let rb = storage
            .transition_project_release_lifecycle(
                &proj.id,
                "rel-100",
                "Published",
                "Rollback from 1.1.0",
            )
            .unwrap();
        assert_eq!(rb.lifecycle_state, "Published");
        let new_after = storage.get_project_release("rel-110").unwrap().unwrap();
        assert_eq!(new_after.lifecycle_state, "RolledBack");

        let history = storage.list_project_release_transitions(&proj.id).unwrap();
        assert!(history.len() >= 5);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_p5_1_long_run_multi_worker_wal_stress_compaction_metrics() {
        let temp_dir =
            std::env::temp_dir().join(format!("fp_p5_1_long_stress_{}", uuid::Uuid::new_v4()));
        let db_path = temp_dir.join("wal_long_stress.db");
        let setup = StorageManager::new(&db_path).unwrap();
        setup
            .create_project(
                CreateProjectRequest {
                    slug: "long-stress".to_string(),
                    name: "Long Stress".to_string(),
                    game_package_id: "com.stress.long".to_string(),
                    version: None,
                    exe_name: None,
                    window_title: None,
                    icon_rel_path: None,
                    target_width_px: None,
                    target_height_px: None,
                    bundle_adb: None,
                    enabled_modules: None,
                },
                &temp_dir.join("projects"),
            )
            .unwrap();
        drop(setup);

        #[cfg(windows)]
        fn sample_proc_resources() -> (u64, u64, usize) {
            #[repr(C)]
            struct FileTime {
                dw_low: u32,
                dw_high: u32,
            }
            #[repr(C)]
            struct Pmc {
                cb: u32,
                page_fault_count: u32,
                peak_ws: usize,
                ws: usize,
                quota_peak_pp: usize,
                quota_pp: usize,
                quota_peak_npp: usize,
                quota_npp: usize,
                pf_usage: usize,
                peak_pf_usage: usize,
            }
            #[link(name = "kernel32")]
            unsafe extern "system" {
                fn GetCurrentProcess() -> *mut core::ffi::c_void;
                fn GetProcessTimes(
                    h: *mut core::ffi::c_void,
                    c: *mut FileTime,
                    e: *mut FileTime,
                    k: *mut FileTime,
                    u: *mut FileTime,
                ) -> i32;
                fn K32GetProcessMemoryInfo(
                    h: *mut core::ffi::c_void,
                    pmc: *mut Pmc,
                    cb: u32,
                ) -> i32;
            }
            unsafe {
                let h = GetCurrentProcess();
                let mut c = FileTime {
                    dw_low: 0,
                    dw_high: 0,
                };
                let mut e = FileTime {
                    dw_low: 0,
                    dw_high: 0,
                };
                let mut k = FileTime {
                    dw_low: 0,
                    dw_high: 0,
                };
                let mut u = FileTime {
                    dw_low: 0,
                    dw_high: 0,
                };
                GetProcessTimes(h, &mut c, &mut e, &mut k, &mut u);
                let cpu_100ns = ((k.dw_high as u64) << 32 | k.dw_low as u64)
                    + ((u.dw_high as u64) << 32 | u.dw_low as u64);
                let mut pmc = std::mem::zeroed::<Pmc>();
                pmc.cb = std::mem::size_of::<Pmc>() as u32;
                K32GetProcessMemoryInfo(h, &mut pmc, pmc.cb);
                (cpu_100ns / 10_000, pmc.ws as u64, pmc.peak_ws)
            }
        }
        #[cfg(not(windows))]
        fn sample_proc_resources() -> (u64, u64, usize) {
            (0, 0, 0)
        }

        let (cpu_before_ms, ws_before, _) = sample_proc_resources();
        let wall_t0 = std::time::Instant::now();

        // Spawn 4 concurrent writer threads (4 * 750 = 3,000 events), 1 persistent ExternalWalReader thread,
        // and concurrent compaction executions while writes happen!
        const WORKERS: usize = 4;
        const EVENTS_PER_WORKER: usize = 750;
        const TOTAL_EVENTS: usize = WORKERS * EVENTS_PER_WORKER;

        let writers_done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let all_latencies = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u64>::with_capacity(
            TOTAL_EVENTS,
        )));
        let reader_seen_seqs = std::sync::Arc::new(std::sync::Mutex::new(
            Vec::<u64>::with_capacity(TOTAL_EVENTS),
        ));

        let r_done = writers_done.clone();
        let r_seqs = reader_seen_seqs.clone();
        let r_db = db_path.clone();
        let reader_handle = std::thread::spawn(move || {
            let mut reader = ExternalWalReader::open(&r_db).unwrap();
            let mut seen_map: HashMap<String, u64> = HashMap::new();
            let mut total_polls = 0u64;
            loop {
                total_polls += 1;
                if let Ok((changed, evs)) = reader.poll_deltas("mini:long-stress", &seen_map) {
                    if changed && !evs.is_empty() {
                        let mut g = r_seqs.lock().unwrap();
                        for ev in evs {
                            let cur = seen_map.entry(ev.stream_id.clone()).or_insert(0);
                            if ev.seq > *cur {
                                assert_eq!(
                                    ev.seq,
                                    *cur + 1,
                                    "Lost or out-of-order seq during concurrent compaction!"
                                );
                                *cur = ev.seq;
                                g.push(ev.seq);
                            }
                        }
                    }
                }
                if r_done.load(std::sync::atomic::Ordering::Relaxed)
                    && seen_map
                        .get("mini:long-stress::testkit")
                        .copied()
                        .unwrap_or(0)
                        >= TOTAL_EVENTS as u64
                {
                    break;
                }
                std::thread::sleep(Duration::from_millis(2));
            }
            (total_polls, reader.busy_retry_count)
        });

        let c_done = writers_done.clone();
        let c_db = db_path.clone();
        let compact_handle = std::thread::spawn(move || {
            let compactor = StorageManager::new(&c_db).unwrap();
            let mut total_deleted = 0usize;
            while !c_done.load(std::sync::atomic::Ordering::Relaxed) {
                if let Ok(d) =
                    compactor.compact_runtime_var_events("mini:long-stress::testkit", 300)
                {
                    total_deleted += d;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            total_deleted
        });

        let mut writer_handles = Vec::with_capacity(WORKERS);
        for w in 0..WORKERS {
            let w_db = db_path.clone();
            let w_lats = all_latencies.clone();
            writer_handles.push(std::thread::spawn(move || {
                let st = StorageManager::new(&w_db).unwrap();
                for i in 0..EVENTS_PER_WORKER {
                    let t0 = std::time::Instant::now();
                    let env = RuntimeVariableEnvelope {
                        project_id: "long-stress".to_string(),
                        session_id: format!("w{}", w),
                        job_id: format!("j{}-{}", w, i),
                        name: format!("worker_{}_val", w),
                        value: serde_json::json!(i),
                        value_type: "number".to_string(),
                        source: VariableSource::TestkitMock,
                        verified: false,
                        confidence: None,
                        confidence_source: None,
                        measured_confidence: None,
                        rule_matched: None,
                        device_id: None,
                        updated_at_utc: Utc::now().to_rfc3339(),
                        workflow_version: "wf@v1".to_string(),
                        seq: 0,
                        redacted: false,
                    };
                    st.record_runtime_var_deltas(
                        "mini:long-stress",
                        "long-stress",
                        &format!("w{}", w),
                        &format!("j{}", w),
                        "wf@v1",
                        vec![env],
                    )
                    .unwrap();
                    w_lats.lock().unwrap().push(t0.elapsed().as_micros() as u64);
                }
            }));
        }

        for h in writer_handles {
            h.join().unwrap();
        }
        writers_done.store(true, std::sync::atomic::Ordering::Relaxed);
        let total_compacted_midflight = compact_handle.join().unwrap();
        let (total_polls, reader_busy_retries) = reader_handle.join().unwrap();

        let wal_peak_bytes = std::fs::metadata(temp_dir.join("wal_long_stress.db-wal"))
            .map(|m| m.len())
            .unwrap_or(0);
        let verifier = StorageManager::new(&db_path).unwrap();
        let deleted_final = verifier
            .compact_runtime_var_events("mini:long-stress::testkit", 100)
            .unwrap();
        let snap = verifier
            .load_runtime_snapshot_with_recovery("mini:long-stress", None)
            .unwrap();
        assert_eq!(
            snap.stream_seqs.get("mini:long-stress::testkit"),
            Some(&(TOTAL_EVENTS as u64))
        );
        assert_eq!(reader_seen_seqs.lock().unwrap().len(), TOTAL_EVENTS);
        assert!(verifier.check_integrity().unwrap().is_healthy);

        let wall_ms = wall_t0.elapsed().as_millis();
        let (cpu_after_ms, ws_after, ws_peak) = sample_proc_resources();
        let mut lats = all_latencies.lock().unwrap().clone();
        lats.sort_unstable();
        let p50_us = lats[lats.len() / 2];
        let p95_us = lats[(lats.len() * 95) / 100];
        let p99_us = lats[(lats.len() * 99) / 100];
        let max_us = *lats.last().unwrap_or(&0);
        eprintln!(
            "[P5.1 Long-Run Stress JSON] {{\"events\":{},\"workers\":{},\"wallMs\":{},\"cpuDeltaMs\":{},\"wsBeforeKb\":{},\"wsAfterKb\":{},\"wsPeakKb\":{},\"p50Us\":{},\"p95Us\":{},\"p99Us\":{},\"maxUs\":{},\"walBytesPeak\":{},\"compactedMidflight\":{},\"compactedFinal\":{},\"readerPolls\":{},\"readerBusyRetries\":{}}}",
            TOTAL_EVENTS,
            WORKERS,
            wall_ms,
            cpu_after_ms.saturating_sub(cpu_before_ms),
            ws_before / 1024,
            ws_after / 1024,
            ws_peak / 1024,
            p50_us,
            p95_us,
            p99_us,
            max_us,
            wal_peak_bytes,
            total_compacted_midflight,
            deleted_final,
            total_polls,
            reader_busy_retries
        );

        drop(verifier);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_p5_3_publisher_profile_crud_v5_and_nonce_replay() {
        let storage = StorageManager::new_in_memory().unwrap();
        assert_eq!(
            storage.applied_migration_versions().unwrap(),
            vec![1, 2, 3, 4, 5, 6]
        );
        // Re-running migrations is idempotent
        storage.run_migrations().unwrap();
        assert_eq!(
            storage.applied_migration_versions().unwrap(),
            vec![1, 2, 3, 4, 5, 6]
        );

        let prof_a = storage
            .upsert_publisher_profile(PublisherProfileInput {
                id: None,
                publisher_id: "pub-studio-alpha".to_string(),
                display_name: "Alpha Studio".to_string(),
                organization: "Alpha Automation Co., Ltd.".to_string(),
                website: Some("https://alpha.example.invalid".to_string()),
                support_contact: Some("support@alpha.example.invalid".to_string()),
                copyright: "Copyright (C) 2026 Alpha Automation Co., Ltd.".to_string(),
                ed25519_key_id: Some("key-alpha-1".to_string()),
                ed25519_pubkey_hex: Some("aa".repeat(32)),
                authenticode_cert_ref: Some("CERT_STORE://CurrentUser/My/DEADBEEF".to_string()),
                release_channel: "qa".to_string(),
                signing_policy: "self_signed_qa".to_string(),
            })
            .unwrap();
        assert_eq!(prof_a.publisher_id, "pub-studio-alpha");
        assert_eq!(prof_a.signing_policy, "self_signed_qa");

        let listed = storage.list_publisher_profiles().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, prof_a.id);

        // Key rotation nonce replay protection
        assert!(storage
            .check_and_record_key_rotation_nonce("nonce-1234567890ab", "proj-1", "oldpk", "newpk")
            .unwrap());
        assert!(!storage
            .check_and_record_key_rotation_nonce("nonce-1234567890ab", "proj-1", "oldpk", "newpk")
            .unwrap());

        assert!(storage.delete_publisher_profile(&prof_a.id).unwrap());
        assert!(storage.list_publisher_profiles().unwrap().is_empty());
    }

    #[test]
    fn test_p6_rc4_sqlite_v5_rc2_upgrade_to_v6_preserves_data_and_backfills_safely() {
        let temp_dir =
            std::env::temp_dir().join(format!("fp_rc4_v5_upgrade_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_file = temp_dir.join("rc2_v5.db");
        let now = "2026-11-22T18:00:00Z";

        // 1. Construct a genuine RC.2 v5 SQLite database WITHOUT the 4 v6 provenance columns
        {
            let conn = Connection::open(&db_file).unwrap();
            conn.execute_batch(
                "PRAGMA journal_mode = WAL;
                 PRAGMA foreign_keys = ON;",
            )
            .unwrap();
            conn.execute_batch(migrations::MIGRATION_V1_SQL).unwrap();
            conn.execute_batch(migrations::MIGRATION_V2_TABLES_SQL)
                .unwrap();
            conn.execute_batch(
                "ALTER TABLE workflows ADD COLUMN project_id TEXT REFERENCES projects(id);
                 ALTER TABLE workflows ADD COLUMN category TEXT NOT NULL DEFAULT 'Chung';
                 ALTER TABLE accounts ADD COLUMN project_id TEXT REFERENCES projects(id);
                 ALTER TABLE jobs ADD COLUMN project_id TEXT REFERENCES projects(id);",
            )
            .unwrap();
            conn.execute_batch(migrations::MIGRATION_V2_INDEXES_SQL)
                .unwrap();
            conn.execute_batch(migrations::MIGRATION_V2_ISOLATION_TRIGGERS_SQL)
                .unwrap();
            conn.execute_batch(migrations::MIGRATION_V3_RUNTIME_PROVENANCE_SQL)
                .unwrap();
            conn.execute_batch(migrations::MIGRATION_V4_RELEASE_LIFECYCLE_SQL)
                .unwrap();
            conn.execute_batch(
                "ALTER TABLE project_releases ADD COLUMN lifecycle_state TEXT NOT NULL DEFAULT 'Published';
                 ALTER TABLE project_releases ADD COLUMN release_channel TEXT NOT NULL DEFAULT 'internal';
                 ALTER TABLE project_releases ADD COLUMN changelog_md TEXT NOT NULL DEFAULT '';
                 ALTER TABLE project_releases ADD COLUMN min_runtime_version TEXT NOT NULL DEFAULT '0.1.0';
                 ALTER TABLE project_releases ADD COLUMN workflow_schema_version INTEGER NOT NULL DEFAULT 1;
                 ALTER TABLE project_releases ADD COLUMN ui_schema_version INTEGER NOT NULL DEFAULT 1;
                 ALTER TABLE project_releases ADD COLUMN module_api_version TEXT NOT NULL DEFAULT '1.0.0';
                 ALTER TABLE project_releases ADD COLUMN sqlite_schema_version INTEGER NOT NULL DEFAULT 5;
                 ALTER TABLE project_releases ADD COLUMN git_commit TEXT NOT NULL DEFAULT '';
                 ALTER TABLE project_releases ADD COLUMN signature_status TEXT NOT NULL DEFAULT 'Ed25519-Verified';
                 ALTER TABLE project_releases ADD COLUMN promoted_at TEXT;
                 ALTER TABLE project_releases ADD COLUMN rolled_back_from_id TEXT;",
            )
            .unwrap();
            conn.execute_batch(migrations::MIGRATION_V5_PUBLISHER_PROFILES_SQL)
                .unwrap();
            conn.execute_batch(
                "ALTER TABLE projects ADD COLUMN publisher_profile_id TEXT;
                 ALTER TABLE projects ADD COLUMN signing_mode TEXT NOT NULL DEFAULT 'unsigned_internal';
                 ALTER TABLE projects ADD COLUMN version_info_json TEXT;
                 ALTER TABLE project_releases ADD COLUMN publisher_profile_id TEXT;
                 ALTER TABLE project_releases ADD COLUMN signing_mode TEXT NOT NULL DEFAULT 'unsigned_internal';
                 ALTER TABLE project_releases ADD COLUMN authenticode_trust_status TEXT NOT NULL DEFAULT 'unsigned';
                 ALTER TABLE project_releases ADD COLUMN authenticode_thumbprint TEXT;
                 ALTER TABLE project_releases ADD COLUMN authenticode_subject TEXT;
                 ALTER TABLE project_releases ADD COLUMN trusted_public_release INTEGER NOT NULL DEFAULT 0;",
            )
            .unwrap();
            for v in 1..=5 {
                conn.execute(
                    "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2);",
                    params![v, now],
                )
                .unwrap();
            }

            // Seed RC.2 records across all key domain tables
            conn.execute(
                "INSERT INTO projects (id, slug, name, game_package_id, version, exe_name, window_title, icon_rel_path, target_width_px, target_height_px, bundle_adb, project_dir_path, created_at, updated_at, signing_mode)
                 VALUES ('proj-rc2', 'rc2-game', 'RC2 Game', 'com.rc2.game', '1.0.0', 'Auto_RC2.exe', 'Auto RC2', 'branding/icon.ico', 1280, 720, 1, 'projects/rc2-game', ?1, ?1, 'unsigned_internal');",
                params![now],
            ).unwrap();
            conn.execute(
                "INSERT INTO workflows (id, project_id, category, name, draft_json, draft_updated_at)
                 VALUES ('wf-rc2', 'proj-rc2', 'Daily', 'RC2 Daily', '{\"schemaVersion\":1}', ?1);",
                params![now],
            ).unwrap();
            conn.execute(
                "INSERT INTO task_definitions (id, project_id, workflow_id, category_id, category_short_tag, category_name, category_color_hex, title, priority, estimate_sec, default_enabled, settings_schema_json, updated_at)
                 VALUES ('task-rc2', 'proj-rc2', 'wf-rc2', 'daily', 'DL', 'Daily', '#1878F5', 'Run Daily', 1, 30, 1, '[]', ?1);",
                params![now],
            ).unwrap();
            conn.execute(
                "INSERT INTO accounts (id, project_id, alias, game_profile_id, secret_ref, state, updated_at)
                 VALUES ('acc-rc2', 'proj-rc2', 'rc2_acc', 'rc2-game', 'dpapi_ref_rc2_token', 'Ready', ?1);",
                params![now],
            ).unwrap();
            conn.execute(
                "INSERT INTO publisher_profiles (id, publisher_id, display_name, organization, copyright, created_at, updated_at)
                 VALUES ('pub-1', 'pub-rc2', 'RC2 Org', 'RC2 Org', 'Copyright 2026', ?1, ?1);",
                params![now],
            ).unwrap();
            conn.execute(
                "INSERT INTO project_releases (id, project_id, version, output_dir_path, exe_path, exe_size_bytes, exe_sha256, key_id, publisher_public_key_hex, signature_ed25519_hex, build_report_json, built_at, lifecycle_state, release_channel, sqlite_schema_version)
                 VALUES ('rel-rc2', 'proj-rc2', '1.0.0', 'dist/rc2', 'dist/rc2/Auto_RC2.exe', 1024, 'abcd', 'k1', 'pk1', 'sig1', '{}', ?1, 'Published', 'internal', 5);",
                params![now],
            ).unwrap();
            conn.execute(
                "INSERT INTO runtime_var_streams (stream_id, project_id, last_seq, updated_at)
                 VALUES ('proj-rc2', 'proj-rc2', 7, ?1);",
                params![now],
            )
            .unwrap();
            // Insert legacy unverified + malformed empty-source row + historical RC.2 LIVE_STATE row
            conn.execute(
                "INSERT INTO runtime_variable_envelopes (scope_key, var_name, project_id, session_id, job_id, value_type, value_json, source, verified, confidence, updated_at_utc, workflow_version, seq, redacted)
                 VALUES ('proj-rc2', 'legacy_ocr_val', 'proj-rc2', 's-rc2', 'j-rc2', 'string', '\"Settings\"', 'LEGACY_UNVERIFIED', 1, 0.94, ?1, 'wf-rc2@v1', 7, 0);",
                params![now],
            ).unwrap();
            conn.execute(
                "INSERT INTO runtime_variable_envelopes (scope_key, var_name, project_id, session_id, job_id, value_type, value_json, source, verified, confidence, updated_at_utc, workflow_version, seq, redacted)
                 VALUES ('proj-rc2', 'legacy_blank_src', 'proj-rc2', 's-rc2', 'j-rc2', 'number', '42', '', 1, 0.88, ?1, 'wf-rc2@v1', 7, 0);",
                params![now],
            ).unwrap();
        }

        // Verify raw pre-upgrade DB has only 14 columns (missing the 4 v6 provenance columns)
        {
            let pre_conn = Connection::open(&db_file).unwrap();
            let pre_cols =
                migrations::list_table_columns(&pre_conn, "runtime_variable_envelopes").unwrap();
            assert!(!pre_cols.contains(&"confidence_source".to_string()));
        }

        // 2. Open with v6 StorageManager (and reopen a second time for idempotency)
        {
            let storage = StorageManager::new(&db_file).unwrap();
            assert_eq!(
                storage.applied_migration_versions().unwrap(),
                vec![1, 2, 3, 4, 5, 6]
            );

            // Query runtime vars (exact path that failed with `no such column: confidence_source` on RC.3!)
            let vars = storage.load_runtime_variables("proj-rc2").unwrap();
            assert_eq!(vars.envelopes.len(), 2);
            let leg = vars.envelopes.get("legacy_ocr_val").unwrap();
            assert_eq!(leg.value, serde_json::json!("Settings"));
            assert_eq!(leg.source, VariableSource::LegacyUnverified);
            assert!(!leg.verified, "Legacy rows must NOT claim verified=true");
            assert_eq!(
                leg.confidence, None,
                "Legacy rows must NOT claim fake confidence"
            );
            assert_eq!(leg.confidence_source, None);
            assert_eq!(leg.measured_confidence, None);

            let blank = vars.envelopes.get("legacy_blank_src").unwrap();
            assert_eq!(blank.source, VariableSource::LegacyUnverified);
            assert!(!blank.verified);
            assert_eq!(blank.confidence_source, None);

            // Domain rows preserved
            assert_eq!(storage.list_projects().unwrap().len(), 1);
            assert_eq!(storage.list_project_workflows("proj-rc2").unwrap().len(), 1);
            assert_eq!(storage.list_project_tasks("proj-rc2").unwrap().len(), 1);
            assert_eq!(storage.list_project_accounts("proj-rc2").unwrap().len(), 1);
            assert_eq!(storage.list_publisher_profiles().unwrap().len(), 1);
            assert_eq!(storage.list_project_releases("proj-rc2").unwrap().len(), 1);

            let integrity = storage.check_integrity().unwrap();
            assert!(integrity.is_healthy);
            assert_eq!(integrity.foreign_key_violations, 0);

            // Writing new OCR provenance envelope succeeds cleanly on upgraded v6 DB
            storage
                .record_runtime_var_deltas(
                    "proj-rc2",
                    "proj-rc2",
                    "s-rc4",
                    "j-rc4",
                    "wf-rc2@v1",
                    vec![RuntimeVariableEnvelope {
                        project_id: "proj-rc2".to_string(),
                        session_id: "s-rc4".to_string(),
                        job_id: "j-rc4".to_string(),
                        name: "ocr_status_text".to_string(),
                        value: serde_json::json!("Network & internet"),
                        value_type: "string".to_string(),
                        source: VariableSource::LiveAdbOcr,
                        verified: true,
                        confidence: None,
                        confidence_source: Some(fp_domain::ConfidenceSource::Heuristic),
                        measured_confidence: None,
                        rule_matched: Some(true),
                        device_id: Some("emulator-5554".to_string()),
                        updated_at_utc: now.to_string(),
                        workflow_version: "wf-rc2@v1".to_string(),
                        seq: 0,
                        redacted: false,
                    }],
                )
                .unwrap();
        }

        // 3. Reopen again -> strictly idempotent
        {
            let storage2 = StorageManager::new(&db_file).unwrap();
            assert_eq!(
                storage2.applied_migration_versions().unwrap(),
                vec![1, 2, 3, 4, 5, 6]
            );
            let vars2 = storage2.load_runtime_variables("proj-rc2").unwrap();
            let ocr_env = vars2.envelopes.get("ocr_status_text").unwrap();
            assert_eq!(
                ocr_env.confidence_source,
                Some(fp_domain::ConfidenceSource::Heuristic)
            );
            assert_eq!(ocr_env.device_id.as_deref(), Some("emulator-5554"));
            assert_eq!(ocr_env.rule_matched, Some(true));
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_p6_rc4_sqlite_incompatible_future_or_missing_column_fails_closed() {
        let temp_dir =
            std::env::temp_dir().join(format!("fp_rc4_incompat_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        // 1. Future schema version (v99) blocked with SQLITE_DOWNGRADE_INCOMPATIBLE
        let future_db = temp_dir.join("future_v99.db");
        {
            let _ = StorageManager::new(&future_db).unwrap();
            let conn = Connection::open(&future_db).unwrap();
            conn.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (99, '2026-11-23T00:00:00Z');",
                [],
            ).unwrap();
        }
        let err_future = match StorageManager::new(&future_db) {
            Ok(_) => panic!("Expected SQLITE_DOWNGRADE_INCOMPATIBLE error"),
            Err(e) => e.to_string(),
        };
        assert!(
            err_future.contains("SQLITE_DOWNGRADE_INCOMPATIBLE"),
            "got: {}",
            err_future
        );

        // 2. Corrupt/incomplete DB claiming v6 while missing required provenance column -> blocked with SQLITE_SCHEMA_INCOMPATIBLE
        let missing_col_db = temp_dir.join("missing_col_v6.db");
        {
            let conn = Connection::open(&missing_col_db).unwrap();
            conn.execute_batch(migrations::MIGRATION_V1_SQL).unwrap();
            conn.execute_batch(migrations::MIGRATION_V2_TABLES_SQL)
                .unwrap();
            conn.execute_batch(migrations::MIGRATION_V3_RUNTIME_PROVENANCE_SQL)
                .unwrap();
            for v in 1..=6 {
                conn.execute(
                    "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, '2026-11-23T00:00:00Z');",
                    params![v],
                ).unwrap();
            }
        }
        let err_missing = match StorageManager::new(&missing_col_db) {
            Ok(_) => panic!("Expected SQLITE_SCHEMA_INCOMPATIBLE error"),
            Err(e) => e.to_string(),
        };
        assert!(
            err_missing.contains("SQLITE_SCHEMA_INCOMPATIBLE"),
            "got: {}",
            err_missing
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_p6_rc4_startup_recover_interrupted_jobs() {
        let storage = StorageManager::new_in_memory().unwrap();
        let proj = storage
            .create_project(
                CreateProjectRequest {
                    slug: "rec-proj".to_string(),
                    name: "Rec Proj".to_string(),
                    game_package_id: "com.rec".to_string(),
                    version: None,
                    exe_name: None,
                    window_title: None,
                    icon_rel_path: None,
                    target_width_px: None,
                    target_height_px: None,
                    bundle_adb: None,
                    enabled_modules: None,
                },
                &std::env::temp_dir().join(format!("fp_rec_proj_{}", uuid::Uuid::new_v4())),
            )
            .unwrap();
        let wf = storage
            .save_project_workflow(&proj.id, "wf-rec", "Rec WF", "Daily", "{\"schemaVersion\":1,\"workflowId\":\"wf-rec\",\"name\":\"Rec\",\"status\":\"published\",\"graph\":{\"nodes\":[{\"id\":\"n1\",\"type\":\"flow.start\",\"position\":{\"x\":0,\"y\":0}},{\"id\":\"n2\",\"type\":\"flow.end\",\"position\":{\"x\":100,\"y\":0}}],\"edges\":[{\"id\":\"e1\",\"source\":\"n1\",\"sourcePort\":\"flow\",\"target\":\"n2\",\"targetPort\":\"flow\"}]}}")
            .unwrap();
        let _ = wf;

        let _ = storage
            .create_or_update_project_job(ProjectJobInput {
                project_id: proj.id.clone(),
                job_id: "job-dead-crash".to_string(),
                device_id: "emulator-5554".to_string(),
                account_id: None,
                workflow_id: "wf-rec".to_string(),
                workflow_name: "Rec WF".to_string(),
                graph_json: "{}".to_string(),
                priority: 1,
                state: "running".to_string(),
                fencing_generation: 3,
                current_node_id: Some("n1".to_string()),
            })
            .unwrap();
        storage
            .upsert_device_lease_record(
                "emulator-5554",
                "job-dead-crash",
                3,
                "2026-11-23T23:59:59Z",
            )
            .unwrap();

        let _ = storage
            .create_or_update_project_job(ProjectJobInput {
                project_id: proj.id.clone(),
                job_id: "job-live-other".to_string(),
                device_id: "emulator-5556".to_string(),
                account_id: None,
                workflow_id: "wf-rec".to_string(),
                workflow_name: "Rec WF".to_string(),
                graph_json: "{}".to_string(),
                priority: 1,
                state: "running".to_string(),
                fencing_generation: 1,
                current_node_id: Some("n1".to_string()),
            })
            .unwrap();

        let recovered = storage
            .recover_interrupted_jobs_on_startup(|dev_id| dev_id == "emulator-5556")
            .unwrap();
        assert_eq!(recovered, 1);

        let dead_job = storage.get_job("job-dead-crash").unwrap().unwrap();
        assert_eq!(dead_job.state, "interrupted");
        assert_eq!(dead_job.fencing_generation, 4);

        let live_job = storage.get_job("job-live-other").unwrap().unwrap();
        assert_eq!(live_job.state, "running");
        let _ = std::fs::remove_dir_all(&proj.project_dir_path);
    }
}
