use crate::models::{DEFAULT_LEGACY_PROJECT_ID, DEFAULT_LEGACY_PROJECT_SLUG};
use anyhow::{bail, Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection};
use std::collections::HashSet;

pub const MIGRATION_V1_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS devices (
    id TEXT PRIMARY KEY,
    adb_serial TEXT UNIQUE,
    alias TEXT NOT NULL,
    android_api INTEGER,
    model TEXT,
    width_px INTEGER NOT NULL,
    height_px INTEGER NOT NULL,
    rotation_deg INTEGER NOT NULL,
    connection_state TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS workflows (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    draft_json TEXT NOT NULL,
    draft_updated_at TEXT NOT NULL,
    archived_at TEXT
);

CREATE TABLE IF NOT EXISTS workflow_versions (
    id TEXT PRIMARY KEY,
    workflow_id TEXT NOT NULL REFERENCES workflows(id),
    version_no INTEGER NOT NULL,
    graph_json TEXT NOT NULL,
    sha256 TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(workflow_id, version_no)
);

CREATE TABLE IF NOT EXISTS accounts (
    id TEXT PRIMARY KEY,
    alias TEXT NOT NULL,
    game_profile_id TEXT,
    secret_ref TEXT,
    state TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS jobs (
    id TEXT PRIMARY KEY,
    device_id TEXT NOT NULL REFERENCES devices(id),
    account_id TEXT REFERENCES accounts(id),
    workflow_version_id TEXT NOT NULL REFERENCES workflow_versions(id),
    priority INTEGER NOT NULL DEFAULT 0,
    state TEXT NOT NULL,
    fencing_generation INTEGER NOT NULL DEFAULT 0,
    current_node_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS step_events (
    id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL REFERENCES jobs(id),
    node_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    attempt_no INTEGER NOT NULL,
    at TEXT NOT NULL,
    payload_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS checkpoints (
    id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL REFERENCES jobs(id),
    node_id TEXT NOT NULL,
    state_json TEXT NOT NULL,
    evidence_ref TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS device_leases (
    device_id TEXT PRIMARY KEY REFERENCES devices(id),
    job_id TEXT NOT NULL REFERENCES jobs(id),
    generation INTEGER NOT NULL,
    expires_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_jobs_state_priority ON jobs(state, priority DESC, created_at);
CREATE INDEX IF NOT EXISTS idx_events_job_time ON step_events(job_id, at);
"#;

pub const MIGRATION_V2_TABLES_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    game_package_id TEXT NOT NULL,
    version TEXT NOT NULL DEFAULT '1.0.0',
    exe_name TEXT NOT NULL,
    window_title TEXT NOT NULL,
    icon_rel_path TEXT NOT NULL DEFAULT 'branding/icon.ico',
    target_width_px INTEGER NOT NULL DEFAULT 1920,
    target_height_px INTEGER NOT NULL DEFAULT 1080,
    bundle_adb INTEGER NOT NULL DEFAULT 1,
    project_dir_path TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS project_modules (
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    module_id TEXT NOT NULL,
    packaged_in_release INTEGER NOT NULL DEFAULT 1,
    enabled_by_default INTEGER NOT NULL DEFAULT 1,
    config_json TEXT NOT NULL DEFAULT '{}',
    updated_at TEXT NOT NULL,
    PRIMARY KEY (project_id, module_id)
);

CREATE TABLE IF NOT EXISTS vision_templates (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    rel_path TEXT NOT NULL,
    width_px INTEGER NOT NULL,
    height_px INTEGER NOT NULL,
    source_crop_x INTEGER,
    source_crop_y INTEGER,
    default_confidence REAL NOT NULL DEFAULT 0.88,
    sha256 TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(project_id, rel_path)
);

CREATE TABLE IF NOT EXISTS task_definitions (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    workflow_id TEXT NOT NULL REFERENCES workflows(id),
    category_id TEXT NOT NULL,
    category_short_tag TEXT NOT NULL,
    category_name TEXT NOT NULL,
    category_color_hex TEXT NOT NULL,
    title TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 1,
    estimate_sec INTEGER NOT NULL DEFAULT 60,
    default_enabled INTEGER NOT NULL DEFAULT 1,
    settings_schema_json TEXT NOT NULL DEFAULT '[]',
    updated_at TEXT NOT NULL,
    FOREIGN KEY (project_id, workflow_id) REFERENCES workflows(project_id, id)
);

CREATE TABLE IF NOT EXISTS mini_tool_schemas (
    project_id TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
    schema_version INTEGER NOT NULL DEFAULT 1,
    ui_schema_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS project_releases (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    version TEXT NOT NULL,
    output_dir_path TEXT NOT NULL,
    exe_path TEXT NOT NULL,
    exe_size_bytes INTEGER NOT NULL,
    exe_sha256 TEXT NOT NULL,
    key_id TEXT NOT NULL,
    publisher_public_key_hex TEXT NOT NULL,
    pe_rcdata_pubkey_id INTEGER NOT NULL DEFAULT 7777,
    signature_ed25519_hex TEXT NOT NULL,
    build_report_json TEXT NOT NULL,
    built_at TEXT NOT NULL
);
"#;

pub const MIGRATION_V2_INDEXES_SQL: &str = r#"
CREATE INDEX IF NOT EXISTS idx_workflows_project ON workflows(project_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_workflows_project_id_pair ON workflows(project_id, id);
CREATE INDEX IF NOT EXISTS idx_workflow_versions_workflow ON workflow_versions(workflow_id);
CREATE INDEX IF NOT EXISTS idx_accounts_project ON accounts(project_id);
CREATE INDEX IF NOT EXISTS idx_jobs_project ON jobs(project_id);
CREATE INDEX IF NOT EXISTS idx_checkpoints_job ON checkpoints(job_id);
CREATE INDEX IF NOT EXISTS idx_templates_project ON vision_templates(project_id);
CREATE INDEX IF NOT EXISTS idx_tasks_project_priority ON task_definitions(project_id, priority ASC);
CREATE INDEX IF NOT EXISTS idx_releases_project_time ON project_releases(project_id, built_at DESC);
"#;

pub const MIGRATION_V2_ISOLATION_TRIGGERS_SQL: &str = r#"
CREATE TRIGGER IF NOT EXISTS trg_workflows_isolation_insert
BEFORE INSERT ON workflows
FOR EACH ROW
WHEN NEW.project_id IS NULL
BEGIN
    SELECT RAISE(ABORT, 'PROJECT_ISOLATION_VIOLATION: workflows.project_id cannot be NULL');
END;

CREATE TRIGGER IF NOT EXISTS trg_workflows_isolation_update
BEFORE UPDATE ON workflows
FOR EACH ROW
WHEN NEW.project_id IS NULL
BEGIN
    SELECT RAISE(ABORT, 'PROJECT_ISOLATION_VIOLATION: workflows.project_id cannot be NULL');
END;

CREATE TRIGGER IF NOT EXISTS trg_workflows_cross_move_guard
BEFORE UPDATE OF project_id ON workflows
FOR EACH ROW
WHEN OLD.project_id IS NOT NULL AND NEW.project_id != OLD.project_id AND (
    EXISTS (SELECT 1 FROM task_definitions t WHERE t.workflow_id = NEW.id)
    OR EXISTS (SELECT 1 FROM jobs j JOIN workflow_versions wv ON wv.id = j.workflow_version_id WHERE wv.workflow_id = NEW.id)
)
BEGIN
    SELECT RAISE(ABORT, 'PROJECT_ISOLATION_VIOLATION: cannot reassign workflow referenced by tasks or jobs across projects');
END;

CREATE TRIGGER IF NOT EXISTS trg_task_definitions_isolation_insert
BEFORE INSERT ON task_definitions
FOR EACH ROW
WHEN NEW.project_id IS NULL OR NOT EXISTS (
    SELECT 1 FROM workflows w WHERE w.id = NEW.workflow_id AND w.project_id = NEW.project_id
)
BEGIN
    SELECT RAISE(ABORT, 'PROJECT_ISOLATION_VIOLATION: task_definitions workflow_id must belong to same project_id');
END;

CREATE TRIGGER IF NOT EXISTS trg_task_definitions_isolation_update
BEFORE UPDATE ON task_definitions
FOR EACH ROW
WHEN NEW.project_id IS NULL OR NOT EXISTS (
    SELECT 1 FROM workflows w WHERE w.id = NEW.workflow_id AND w.project_id = NEW.project_id
)
BEGIN
    SELECT RAISE(ABORT, 'PROJECT_ISOLATION_VIOLATION: task_definitions workflow_id must belong to same project_id');
END;

CREATE TRIGGER IF NOT EXISTS trg_accounts_isolation_insert
BEFORE INSERT ON accounts
FOR EACH ROW
WHEN NEW.project_id IS NULL
BEGIN
    SELECT RAISE(ABORT, 'PROJECT_ISOLATION_VIOLATION: accounts.project_id cannot be NULL');
END;

CREATE TRIGGER IF NOT EXISTS trg_accounts_isolation_update
BEFORE UPDATE ON accounts
FOR EACH ROW
WHEN NEW.project_id IS NULL
BEGIN
    SELECT RAISE(ABORT, 'PROJECT_ISOLATION_VIOLATION: accounts.project_id cannot be NULL');
END;

CREATE TRIGGER IF NOT EXISTS trg_accounts_cross_move_guard
BEFORE UPDATE OF project_id ON accounts
FOR EACH ROW
WHEN OLD.project_id IS NOT NULL AND NEW.project_id != OLD.project_id AND EXISTS (
    SELECT 1 FROM jobs j WHERE j.account_id = NEW.id
)
BEGIN
    SELECT RAISE(ABORT, 'PROJECT_ISOLATION_VIOLATION: cannot reassign account referenced by jobs across projects');
END;

CREATE TRIGGER IF NOT EXISTS trg_workflow_versions_isolation_insert
BEFORE INSERT ON workflow_versions
FOR EACH ROW
WHEN NOT EXISTS (
    SELECT 1 FROM workflows w WHERE w.id = NEW.workflow_id AND w.project_id IS NOT NULL
)
BEGIN
    SELECT RAISE(ABORT, 'PROJECT_ISOLATION_VIOLATION: workflow_versions must reference a project-owned workflow');
END;

CREATE TRIGGER IF NOT EXISTS trg_workflow_versions_isolation_update
BEFORE UPDATE ON workflow_versions
FOR EACH ROW
WHEN NOT EXISTS (
    SELECT 1 FROM workflows w WHERE w.id = NEW.workflow_id AND w.project_id IS NOT NULL
)
BEGIN
    SELECT RAISE(ABORT, 'PROJECT_ISOLATION_VIOLATION: workflow_versions must reference a project-owned workflow');
END;

CREATE TRIGGER IF NOT EXISTS trg_jobs_isolation_insert
BEFORE INSERT ON jobs
FOR EACH ROW
WHEN NEW.project_id IS NULL
    OR NOT EXISTS (
        SELECT 1 FROM workflow_versions wv
        JOIN workflows w ON w.id = wv.workflow_id
        WHERE wv.id = NEW.workflow_version_id AND w.project_id = NEW.project_id
    )
    OR (
        NEW.account_id IS NOT NULL
        AND NOT EXISTS (
            SELECT 1 FROM accounts a WHERE a.id = NEW.account_id AND a.project_id = NEW.project_id
        )
    )
BEGIN
    SELECT RAISE(ABORT, 'PROJECT_ISOLATION_VIOLATION: job project_id must match workflow_version and account project_id');
END;

CREATE TRIGGER IF NOT EXISTS trg_jobs_isolation_update
BEFORE UPDATE ON jobs
FOR EACH ROW
WHEN NEW.project_id IS NULL
    OR NOT EXISTS (
        SELECT 1 FROM workflow_versions wv
        JOIN workflows w ON w.id = wv.workflow_id
        WHERE wv.id = NEW.workflow_version_id AND w.project_id = NEW.project_id
    )
    OR (
        NEW.account_id IS NOT NULL
        AND NOT EXISTS (
            SELECT 1 FROM accounts a WHERE a.id = NEW.account_id AND a.project_id = NEW.project_id
        )
    )
BEGIN
    SELECT RAISE(ABORT, 'PROJECT_ISOLATION_VIOLATION: job project_id must match workflow_version and account project_id');
END;

CREATE TRIGGER IF NOT EXISTS trg_checkpoints_isolation_insert
BEFORE INSERT ON checkpoints
FOR EACH ROW
WHEN NOT EXISTS (
    SELECT 1 FROM jobs j WHERE j.id = NEW.job_id AND j.project_id IS NOT NULL
)
BEGIN
    SELECT RAISE(ABORT, 'PROJECT_ISOLATION_VIOLATION: checkpoint job must belong to a valid project');
END;

CREATE TRIGGER IF NOT EXISTS trg_checkpoints_isolation_update
BEFORE UPDATE ON checkpoints
FOR EACH ROW
WHEN NOT EXISTS (
    SELECT 1 FROM jobs j WHERE j.id = NEW.job_id AND j.project_id IS NOT NULL
)
BEGIN
    SELECT RAISE(ABORT, 'PROJECT_ISOLATION_VIOLATION: checkpoint job must belong to a valid project');
END;

CREATE TRIGGER IF NOT EXISTS trg_step_events_isolation_insert
BEFORE INSERT ON step_events
FOR EACH ROW
WHEN NOT EXISTS (
    SELECT 1 FROM jobs j WHERE j.id = NEW.job_id AND j.project_id IS NOT NULL
)
BEGIN
    SELECT RAISE(ABORT, 'PROJECT_ISOLATION_VIOLATION: step_event job must belong to a valid project');
END;

CREATE TRIGGER IF NOT EXISTS trg_step_events_isolation_update
BEFORE UPDATE ON step_events
FOR EACH ROW
WHEN NOT EXISTS (
    SELECT 1 FROM jobs j WHERE j.id = NEW.job_id AND j.project_id IS NOT NULL
)
BEGIN
    SELECT RAISE(ABORT, 'PROJECT_ISOLATION_VIOLATION: step_event job must belong to a valid project');
END;
"#;

fn table_has_column(conn: &Connection, table: &str, col_name: &str) -> Result<bool> {
    let pragma_sql = format!("PRAGMA table_info({});", table);
    let mut stmt = conn.prepare(&pragma_sql)?;
    let cols = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for c in cols {
        if c? == col_name {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn list_applied_versions(conn: &Connection) -> Result<Vec<i64>> {
    let table_exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='schema_migrations'",
        [],
        |row| row.get(0),
    )?;
    if table_exists == 0 {
        return Ok(Vec::new());
    }
    let mut stmt = conn.prepare("SELECT version FROM schema_migrations ORDER BY version ASC")?;
    let rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;
    let mut versions = Vec::new();
    for r in rows {
        versions.push(r?);
    }
    Ok(versions)
}

pub fn apply_migrations(conn: &mut Connection) -> Result<Vec<i64>> {
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;

    // Ensure schema_migrations exists before querying applied versions
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);",
        [],
    )?;

    let applied: HashSet<i64> = list_applied_versions(conn)?.into_iter().collect();
    let now = Utc::now().to_rfc3339();

    // Migration v1: Baseline tables (idempotent even on pre-tracking v1 DBs)
    if !applied.contains(&1) {
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute_batch(MIGRATION_V1_SQL)
            .context("Failed applying SQLite migration v1")?;
        tx.execute(
            "INSERT OR IGNORE INTO schema_migrations (version, applied_at) VALUES (1, ?1);",
            params![now],
        )?;
        tx.commit()?;
    }

    // Migration v2: Multi-Project & Mini Tool Factory Schema
    if !applied.contains(&2) {
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute_batch(MIGRATION_V2_TABLES_SQL)
            .context("Failed creating v2 multi-project tables")?;

        if !table_has_column(&tx, "workflows", "project_id")? {
            tx.execute(
                "ALTER TABLE workflows ADD COLUMN project_id TEXT REFERENCES projects(id);",
                [],
            )?;
        }
        if !table_has_column(&tx, "workflows", "category")? {
            tx.execute(
                "ALTER TABLE workflows ADD COLUMN category TEXT NOT NULL DEFAULT 'Chung';",
                [],
            )?;
        }
        if !table_has_column(&tx, "accounts", "project_id")? {
            tx.execute(
                "ALTER TABLE accounts ADD COLUMN project_id TEXT REFERENCES projects(id);",
                [],
            )?;
        }
        if !table_has_column(&tx, "jobs", "project_id")? {
            tx.execute(
                "ALTER TABLE jobs ADD COLUMN project_id TEXT REFERENCES projects(id);",
                [],
            )?;
        }

        tx.execute_batch(MIGRATION_V2_INDEXES_SQL)
            .context("Failed creating v2 indexes")?;

        // Backward compatibility: if any existing v1 rows exist with project_id IS NULL,
        // create default legacy GameProject 'proj-jx2-mobile-001' and attach them without data loss.
        let legacy_wf_count: i64 = tx.query_row(
            "SELECT COUNT(*) FROM workflows WHERE project_id IS NULL",
            [],
            |r| r.get(0),
        )?;
        let legacy_acc_count: i64 = tx.query_row(
            "SELECT COUNT(*) FROM accounts WHERE project_id IS NULL",
            [],
            |r| r.get(0),
        )?;
        let legacy_job_count: i64 = tx.query_row(
            "SELECT COUNT(*) FROM jobs WHERE project_id IS NULL",
            [],
            |r| r.get(0),
        )?;

        if legacy_wf_count > 0 || legacy_acc_count > 0 || legacy_job_count > 0 {
            tx.execute(
                "INSERT OR IGNORE INTO projects (
                    id, slug, name, game_package_id, version, exe_name, window_title,
                    icon_rel_path, target_width_px, target_height_px, bundle_adb,
                    project_dir_path, created_at, updated_at
                ) VALUES (
                    ?1, ?2, ?3, ?4, '1.0.0', 'Auto_JX2_Mobile.exe',
                    'Auto Võ Hiệp 2 Mobile v1.0.0', 'branding/icon.ico',
                    1920, 1080, 1, 'projects/jx2-mobile', ?5, ?5
                );",
                params![
                    DEFAULT_LEGACY_PROJECT_ID,
                    DEFAULT_LEGACY_PROJECT_SLUG,
                    "Võ Hiệp Truyền Kỳ 2 Mobile",
                    "com.blackhole.jx2mobile",
                    now
                ],
            )?;
            tx.execute(
                "UPDATE workflows SET project_id = ?1 WHERE project_id IS NULL;",
                params![DEFAULT_LEGACY_PROJECT_ID],
            )?;
            tx.execute(
                "UPDATE accounts SET project_id = ?1 WHERE project_id IS NULL;",
                params![DEFAULT_LEGACY_PROJECT_ID],
            )?;
            tx.execute(
                "UPDATE jobs SET project_id = ?1 WHERE project_id IS NULL;",
                params![DEFAULT_LEGACY_PROJECT_ID],
            )?;
        }

        // Verify zero foreign key violations before committing v2
        let fk_violations: i64 = {
            let mut stmt = tx.prepare("PRAGMA foreign_key_check;")?;
            let rows = stmt.query_map([], |_| Ok(1i64))?;
            rows.count() as i64
        };
        if fk_violations > 0 {
            bail!(
                "Migration v2 aborted: detected {} foreign key violation(s)",
                fk_violations
            );
        }

        tx.execute_batch(MIGRATION_V2_ISOLATION_TRIGGERS_SQL)
            .context("Failed creating v2 SQLite project isolation triggers")?;

        tx.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (2, ?1);",
            params![now],
        )?;
        tx.commit()?;
    } else {
        // Ensure P2.1 SQLite-level project isolation indexes & triggers are present on existing v2 DBs
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute_batch(MIGRATION_V2_INDEXES_SQL)
            .context("Failed ensuring v2 isolation indexes")?;
        tx.execute_batch(MIGRATION_V2_ISOLATION_TRIGGERS_SQL)
            .context("Failed ensuring v2 SQLite project isolation triggers")?;
        tx.commit()?;
    }

    list_applied_versions(conn)
}

/// Applies an explicit transactional migration script (used for future versions or rollback testing).
/// If any statement in `sql_script` fails, the entire transaction rolls back atomically.
pub fn apply_transactional_migration(
    conn: &mut Connection,
    target_version: i64,
    sql_script: &str,
) -> Result<()> {
    let applied: HashSet<i64> = list_applied_versions(conn)?.into_iter().collect();
    if applied.contains(&target_version) {
        return Ok(());
    }
    let now = Utc::now().to_rfc3339();
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    tx.execute(
        "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2);",
        params![target_version, now],
    )?;
    tx.execute_batch(sql_script)?;
    tx.commit()?;
    Ok(())
}
