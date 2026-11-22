pub mod jx2_seed;
pub mod migrations;
pub mod models;

use anyhow::{bail, Context, Result};
use chrono::Utc;
use fp_bundle_contract::{
    CompatibilityVersions, GameProjectManifest, MiniLayoutPreset, MiniTaskDef, MiniToolUiSchema,
    TaskCategoryDef,
};
use fp_domain::{ConnectionState, DeviceSummary, StepEvent};
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
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
    format!("Auto_{}.exe", if pascal.is_empty() { "Tool" } else { &pascal })
}

fn copy_dir_files(src: &Path, dst: &Path) -> Result<()> {
    if !src.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let p = entry.path();
        if p.is_file() {
            std::fs::copy(&p, dst.join(entry.file_name()))?;
        }
    }
    Ok(())
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

#[derive(Clone)]
pub struct StorageManager {
    conn: Arc<Mutex<Connection>>,
}

impl StorageManager {
    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let sm = Self {
            conn: Arc::new(Mutex::new(conn)),
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
        let sm = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        sm.run_migrations()?;
        Ok(sm)
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

    pub fn apply_transactional_migration(&self, target_version: i64, sql_script: &str) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        migrations::apply_transactional_migration(&mut conn, target_version, sql_script)
    }

    pub fn check_integrity(&self) -> Result<IntegrityReport> {
        let conn = self.conn.lock().unwrap();
        check_integrity_on_conn(&conn)
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
        conn.execute(
            "INSERT INTO workflows (id, name, draft_json, draft_updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                draft_json = excluded.draft_json,
                draft_updated_at = excluded.draft_updated_at;",
            params![id, name, draft_json, now],
        )?;
        Ok(())
    }

    pub fn get_workflow_draft(&self, id: &str) -> Result<Option<(String, String)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT name, draft_json FROM workflows WHERE id = ?1")?;
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

        // 2. Ensure workflow exists (respecting project ownership if provided)
        if let Some(pid) = project_id {
            let existing_owner: Option<Option<String>> = {
                let mut st = conn.prepare("SELECT project_id FROM workflows WHERE id = ?1")?;
                let mut rows = st.query(params![workflow_id])?;
                rows.next()?.map(|r| r.get(0)).transpose()?
            };
            if let Some(Some(owner)) = existing_owner {
                if owner != pid {
                    bail!(
                        "PROJECT_ISOLATION_VIOLATION: Workflow '{}' belongs to project '{}', cannot run under '{}'",
                        workflow_id,
                        owner,
                        pid
                    );
                }
            }
            conn.execute(
                "INSERT INTO workflows (id, project_id, name, draft_json, draft_updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO NOTHING;",
                params![workflow_id, pid, workflow_name, graph_json, now],
            )?;
        } else {
            conn.execute(
                "INSERT INTO workflows (id, name, draft_json, draft_updated_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(id) DO NOTHING;",
                params![workflow_id, workflow_name, graph_json, now],
            )?;
        }

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

        // 4. Ensure job exists
        conn.execute(
            "INSERT INTO jobs (id, project_id, device_id, workflow_version_id, priority, state, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 0, 'running', ?5, ?5)
             ON CONFLICT(id) DO UPDATE SET state = excluded.state, updated_at = excluded.updated_at;",
            params![job_id, project_id, device_id, version_id, now],
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
                event.payload_json
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
        })
    }

    fn fetch_project_by_id_conn(conn: &Connection, project_id: &str) -> Result<Option<GameProjectRecord>> {
        let mut stmt = conn.prepare(
            "SELECT id, slug, name, game_package_id, version, exe_name, window_title,
                    icon_rel_path, target_width_px, target_height_px, bundle_adb,
                    project_dir_path, created_at, updated_at
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
                    project_dir_path, created_at, updated_at
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
        let proj_dir_str = proj_dir.to_string_lossy().to_string();

        let enabled_mods = req.enabled_modules.unwrap_or_else(|| {
            vec![
                "mod.adb_core".to_string(),
                "mod.vision_ocr".to_string(),
                "mod.account_vault".to_string(),
                "mod.emulator_optimizer".to_string(),
            ]
        });

        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;

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
            let packaged = is_core || enabled_mods.iter().any(|e| e == m);
            if packaged {
                packaged_modules.push(m.to_string());
            }
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
        tx.execute(
            "INSERT INTO mini_tool_schemas (project_id, schema_version, ui_schema_json, updated_at)
             VALUES (?1, 1, ?2, ?3);",
            params![project_id, ui_json, now],
        )?;

        // Create workspace directories & manifest files only after DB constraints pass
        std::fs::create_dir_all(proj_dir.join("workflows"))?;
        std::fs::create_dir_all(proj_dir.join("templates"))?;
        std::fs::create_dir_all(proj_dir.join("branding"))?;

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
            packaged_modules,
            workflow_files: vec![],
            template_files: vec![],
            bundle_adb,
        };
        std::fs::write(
            proj_dir.join("project.json"),
            serde_json::to_vec_pretty(&manifest)?,
        )?;
        std::fs::write(proj_dir.join("ui-schema.json"), ui_json.as_bytes())?;

        tx.commit()?;

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
        })
    }

    pub fn list_projects(&self) -> Result<Vec<GameProjectSummary>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, slug, name, game_package_id, version, exe_name, window_title,
                    icon_rel_path, target_width_px, target_height_px, bundle_adb,
                    project_dir_path, created_at, updated_at
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
        let conn = self.conn.lock().unwrap();
        let existing = Self::fetch_project_by_id_conn(&conn, &req.id)?
            .with_context(|| format!("Project '{}' not found", req.id))?;

        let name = req
            .name
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or(existing.name);
        let game_package_id = req
            .game_package_id
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or(existing.game_package_id);
        let version = req
            .version
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or(existing.version);
        let exe_name = req
            .exe_name
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or(existing.exe_name);
        let window_title = req
            .window_title
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or(existing.window_title);
        let icon_rel_path = req
            .icon_rel_path
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or(existing.icon_rel_path);
        let target_w = req.target_width_px.unwrap_or(existing.target_width_px);
        let target_h = req.target_height_px.unwrap_or(existing.target_height_px);
        let bundle_adb = req.bundle_adb.unwrap_or(existing.bundle_adb);
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "UPDATE projects SET
                name = ?1, game_package_id = ?2, version = ?3, exe_name = ?4,
                window_title = ?5, icon_rel_path = ?6, target_width_px = ?7,
                target_height_px = ?8, bundle_adb = ?9, updated_at = ?10
             WHERE id = ?11;",
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
                now,
                req.id
            ],
        )?;

        // Sync project.json on disk if present
        let manifest_path = Path::new(&existing.project_dir_path).join("project.json");
        if manifest_path.exists() {
            if let Ok(raw) = std::fs::read(&manifest_path) {
                if let Ok(mut manifest) = serde_json::from_slice::<GameProjectManifest>(&raw) {
                    manifest.name = name.clone();
                    manifest.game_package_id = game_package_id.clone();
                    manifest.version = version.clone();
                    manifest.exe_name = exe_name.clone();
                    manifest.window_title = window_title.clone();
                    manifest.icon_rel_path = icon_rel_path.clone();
                    manifest.target_resolution = (target_w, target_h);
                    manifest.bundle_adb = bundle_adb;
                    if let Ok(bytes) = serde_json::to_vec_pretty(&manifest) {
                        let _ = std::fs::write(&manifest_path, bytes);
                    }
                }
            }
        }

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
        })
    }

    pub fn clone_project(
        &self,
        req: CloneProjectRequest,
        projects_base_dir: &Path,
    ) -> Result<GameProjectRecord> {
        let new_slug = validate_project_slug(&req.new_slug)?;
        let new_name = req.new_name.trim().to_string();
        if new_name.is_empty() {
            bail!("Cloned project name cannot be empty");
        }

        let new_proj_dir = projects_base_dir.join(&new_slug);
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

        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;

        tx.execute(
            "INSERT INTO projects (
                id, slug, name, game_package_id, version, exe_name, window_title,
                icon_rel_path, target_width_px, target_height_px, bundle_adb,
                project_dir_path, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13);",
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
                now
            ],
        )?;

        // Prepare disk folders & copy templates/branding ONLY after DB uniqueness check passes
        let src_dir = Path::new(&src.project_dir_path);
        std::fs::create_dir_all(new_proj_dir.join("workflows"))?;
        std::fs::create_dir_all(new_proj_dir.join("templates"))?;
        std::fs::create_dir_all(new_proj_dir.join("branding"))?;
        copy_dir_files(&src_dir.join("templates"), &new_proj_dir.join("templates"))?;
        copy_dir_files(&src_dir.join("branding"), &new_proj_dir.join("branding"))?;

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

                // Update workflowId inside JSON if parseable
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
                std::fs::write(new_proj_dir.join(&rel_wf), updated_json.as_bytes())?;
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
            let tpls: Vec<(String, String, i64, i64, Option<i32>, Option<i32>, f64, String)> = t_stmt
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
                let new_tpl_id = format!("tpl-{}-{:02}", new_slug, idx + 1);
                tx.execute(
                    "INSERT INTO vision_templates (
                        id, project_id, name, rel_path, width_px, height_px,
                        source_crop_x, source_crop_y, default_confidence, sha256, created_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11);",
                    params![new_tpl_id, new_id, name, rel_path, w, h, cx, cy, conf, sha, now],
                )?;
                template_files.push(rel_path);
            }
        }

        // Clone task_definitions (remapping workflow_id to cloned workflow)
        {
            let mut tk_stmt = tx.prepare(
                "SELECT workflow_id, category_id, category_short_tag, category_name, category_color_hex,
                        title, priority, estimate_sec, default_enabled, settings_schema_json
                 FROM task_definitions WHERE project_id = ?1;",
            )?;
            let tasks: Vec<(String, String, String, String, String, String, i32, i64, i64, String)> =
                tk_stmt
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
            for (idx, (old_wf_id, cat_id, cat_tag, cat_name, cat_col, title, prio, est, def_en, sch)) in
                tasks.into_iter().enumerate()
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
            let new_ui_json = if let Ok(mut parsed) = serde_json::from_str::<MiniToolUiSchema>(&ui_json) {
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
            std::fs::write(new_proj_dir.join("ui-schema.json"), new_ui_json.as_bytes())?;
        }

        // Write cloned project.json manifest on disk
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
        std::fs::write(
            new_proj_dir.join("project.json"),
            serde_json::to_vec_pretty(&cloned_manifest)?,
        )?;

        tx.commit()?;

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
        })
    }

    pub fn delete_project(&self, project_id: &str, remove_disk_dir: bool) -> Result<bool> {
        let mut conn = self.conn.lock().unwrap();
        let existing = match Self::fetch_project_by_id_conn(&conn, project_id)? {
            Some(p) => p,
            None => return Ok(false),
        };

        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        // Delete dependent records in FK order (including any jobs tied via workflow_versions)
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
        tx.commit()?;

        if remove_disk_dir {
            let p = Path::new(&existing.project_dir_path);
            if p.exists() {
                let _ = std::fs::remove_dir_all(p);
            }
        }
        Ok(true)
    }

    // -------------------------------------------------------------------------
    // Project Modules
    // -------------------------------------------------------------------------

    pub fn list_project_modules(&self, project_id: &str) -> Result<Vec<ProjectModuleRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT project_id, module_id, packaged_in_release, enabled_by_default, config_json, updated_at
             FROM project_modules WHERE project_id = ?1 ORDER BY module_id ASC;",
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
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT w.id, COALESCE(w.project_id, ''), w.name, w.category, w.draft_json,
                    w.draft_updated_at, w.archived_at,
                    (SELECT MAX(version_no) FROM workflow_versions v WHERE v.workflow_id = w.id)
             FROM workflows w
             WHERE w.project_id = ?1
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
        let wf_id_clean = workflow_id.trim();
        let name_clean = name.trim();
        if wf_id_clean.is_empty() {
            bail!("Workflow ID cannot be empty");
        }
        if name_clean.is_empty() {
            bail!("Workflow name cannot be empty");
        }
        let mut conn = self.conn.lock().unwrap();
        let _ = Self::fetch_project_by_id_conn(&conn, project_id)?
            .with_context(|| format!("Project '{}' not found", project_id))?;

        let existing_owner: Option<Option<String>> = {
            let mut st = conn.prepare("SELECT project_id FROM workflows WHERE id = ?1;")?;
            let mut rows = st.query(params![wf_id_clean])?;
            rows.next()?.map(|r| r.get(0)).transpose()?
        };

        if let Some(Some(owner)) = existing_owner {
            if owner != project_id {
                bail!(
                    "PROJECT_ISOLATION_VIOLATION: Workflow '{}' belongs to project '{}', cannot modify from project '{}'",
                    workflow_id,
                    owner,
                    project_id
                );
            }
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
            params![workflow_id, project_id, name, category, draft_json, now],
        )?;

        let max_ver: Option<i64> = tx.query_row(
            "SELECT MAX(version_no) FROM workflow_versions WHERE workflow_id = ?1;",
            params![workflow_id],
            |r| r.get(0),
        )?;
        let next_ver = max_ver.unwrap_or(0) + 1;
        let ver_id = format!("{}-v{}", workflow_id, next_ver);
        let sha = sha256_hex(draft_json.as_bytes());
        tx.execute(
            "INSERT INTO workflow_versions (id, workflow_id, version_no, graph_json, sha256, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6);",
            params![ver_id, workflow_id, next_ver, draft_json, sha, now],
        )?;
        tx.commit()?;

        Ok(ProjectWorkflowRecord {
            id: workflow_id.to_string(),
            project_id: project_id.to_string(),
            name: name.to_string(),
            category: category.to_string(),
            draft_json: draft_json.to_string(),
            draft_updated_at: now,
            archived_at: None,
            latest_version_no: Some(next_ver),
        })
    }

    pub fn delete_project_workflow(&self, project_id: &str, workflow_id: &str) -> Result<bool> {
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
        let _ = Self::fetch_project_by_id_conn(&conn, &input.project_id)?
            .with_context(|| format!("Project '{}' not found", input.project_id))?;

        let rel_clean = input.rel_path.trim().replace('\\', "/");
        if rel_clean.is_empty() || rel_clean.contains("..") || rel_clean.starts_with('/') {
            bail!("Invalid relative template path '{}'", input.rel_path);
        }

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

    pub fn list_vision_templates(&self, project_id: &str) -> Result<Vec<VisionTemplateRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, name, rel_path, width_px, height_px,
                    source_crop_x, source_crop_y, default_confidence, sha256, created_at
             FROM vision_templates WHERE project_id = ?1 ORDER BY rel_path ASC;",
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
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, workflow_id, category_id, category_short_tag,
                    category_name, category_color_hex, title, priority, estimate_sec,
                    default_enabled, settings_schema_json, updated_at
             FROM task_definitions WHERE project_id = ?1 ORDER BY priority ASC, id ASC;",
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
        let conn = self.conn.lock().unwrap();
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
        conn.execute(
            "INSERT INTO mini_tool_schemas (project_id, schema_version, ui_schema_json, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(project_id) DO UPDATE SET
                schema_version = excluded.schema_version,
                ui_schema_json = excluded.ui_schema_json,
                updated_at = excluded.updated_at;",
            params![project_id, schema.schema_version as i64, json_str, now],
        )?;

        let disk_dir = Path::new(&project.project_dir_path);
        if disk_dir.exists() {
            std::fs::write(disk_dir.join("ui-schema.json"), json_str.as_bytes())?;
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
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, COALESCE(project_id, ''), alias, game_profile_id, secret_ref, state, updated_at
             FROM accounts WHERE project_id = ?1 ORDER BY updated_at DESC;",
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
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, device_id, account_id, workflow_version_id, priority, state,
                    fencing_generation, current_node_id, created_at, updated_at
             FROM jobs WHERE project_id = ?1 ORDER BY created_at DESC;",
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

    pub fn record_project_release(&self, release: ProjectReleaseRecord) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO project_releases (
                id, project_id, version, output_dir_path, exe_path, exe_size_bytes,
                exe_sha256, key_id, publisher_public_key_hex, pe_rcdata_pubkey_id,
                signature_ed25519_hex, build_report_json, built_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13);",
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
                release.built_at
            ],
        )?;
        Ok(())
    }

    pub fn list_project_releases(&self, project_id: &str) -> Result<Vec<ProjectReleaseRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, version, output_dir_path, exe_path, exe_size_bytes,
                    exe_sha256, key_id, publisher_public_key_hex, pe_rcdata_pubkey_id,
                    signature_ed25519_hex, build_report_json, built_at
             FROM project_releases WHERE project_id = ?1 ORDER BY built_at DESC;",
        )?;
        let rows = stmt.query_map(params![project_id], |r| {
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
            })
        })?;
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
        let temp_dir = std::env::temp_dir().join(format!(
            "fp_p2_mig_test_{}",
            uuid::Uuid::new_v4()
        ));
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
            ).unwrap();
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

        // 2. Open with StorageManager -> runs v1 + v2 migrations atomically
        let storage = StorageManager::new(&db_file).unwrap();
        assert_eq!(storage.applied_migration_versions().unwrap(), vec![1, 2]);

        // 3. Verify idempotency: running migrations again causes zero errors and keeps versions [1, 2]
        storage.run_migrations().unwrap();
        assert_eq!(storage.applied_migration_versions().unwrap(), vec![1, 2]);

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

        let legacy_jobs = storage.list_project_jobs(DEFAULT_LEGACY_PROJECT_ID).unwrap();
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

        let bad_v3_sql = "
            CREATE TABLE should_not_exist (id TEXT PRIMARY KEY);
            INSERT INTO nonexistent_table_triggering_rollback VALUES (1);
        ";
        let res = storage.apply_transactional_migration(3, bad_v3_sql);
        assert!(res.is_err());

        // Version 3 must NOT be recorded and `should_not_exist` table must NOT exist
        assert_eq!(storage.applied_migration_versions().unwrap(), vec![1, 2]);
        let integrity = storage.check_integrity().unwrap();
        assert!(integrity.is_healthy);
        assert!(!integrity.table_counts.contains_key("should_not_exist"));
        assert!(storage.get_workflow_draft("wf-keep").unwrap().is_some());
    }

    #[test]
    fn test_p2_online_backup_integrity_and_restore() {
        let temp_dir = std::env::temp_dir().join(format!(
            "fp_p2_backup_test_{}",
            uuid::Uuid::new_v4()
        ));
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

        let temp_dir = std::env::temp_dir().join(format!(
            "fp_p2_jx2_mig_test_{}",
            uuid::Uuid::new_v4()
        ));
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
        let temp_dir = std::env::temp_dir().join(format!(
            "fp_p2_isolation_test_{}",
            uuid::Uuid::new_v4()
        ));
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
            .save_project_workflow(&proj_a.id, "wf-alpha-01", "Alpha Quest", "NV", "{\"nodes\":[]}")
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
        let cross_wf_err = storage.save_project_workflow(
            &proj_b.id,
            "wf-alpha-01",
            "Hijack",
            "NV",
            "{}",
        );
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
        let temp_dir = std::env::temp_dir().join(format!(
            "fp_p2_edge_test_{}",
            uuid::Uuid::new_v4()
        ));
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
                game_package_id: None,
                version: Some("2.1.0".to_string()),
                exe_name: Some("   ".to_string()),
                window_title: Some("Edge Pro Window".to_string()),
                icon_rel_path: None,
                target_width_px: Some(1920),
                target_height_px: Some(1080),
                bundle_adb: Some(false),
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
}
