use crate::models::{DEFAULT_LEGACY_PROJECT_ID, DEFAULT_LEGACY_PROJECT_SLUG};
use anyhow::{bail, Context, Result};
use chrono::Utc;
use fp_bundle_contract::CURRENT_SQLITE_SCHEMA_VERSION;
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

DROP TRIGGER IF EXISTS trg_workflows_cross_move_guard;
CREATE TRIGGER trg_workflows_cross_move_guard
BEFORE UPDATE OF project_id ON workflows
FOR EACH ROW
WHEN OLD.project_id IS NOT NULL AND NEW.project_id != OLD.project_id
BEGIN
    SELECT RAISE(ABORT, 'PROJECT_ISOLATION_VIOLATION: cannot reassign workflow across projects');
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

DROP TRIGGER IF EXISTS trg_task_definitions_isolation_update;
CREATE TRIGGER trg_task_definitions_isolation_update
BEFORE UPDATE ON task_definitions
FOR EACH ROW
WHEN NEW.project_id IS NULL
    OR (OLD.project_id IS NOT NULL AND NEW.project_id != OLD.project_id)
    OR NOT EXISTS (
        SELECT 1 FROM workflows w WHERE w.id = NEW.workflow_id AND w.project_id = NEW.project_id
    )
BEGIN
    SELECT RAISE(ABORT, 'PROJECT_ISOLATION_VIOLATION: task_definitions workflow_id must belong to same project_id and cannot move across projects');
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

DROP TRIGGER IF EXISTS trg_accounts_cross_move_guard;
CREATE TRIGGER trg_accounts_cross_move_guard
BEFORE UPDATE OF project_id ON accounts
FOR EACH ROW
WHEN OLD.project_id IS NOT NULL AND NEW.project_id != OLD.project_id
BEGIN
    SELECT RAISE(ABORT, 'PROJECT_ISOLATION_VIOLATION: cannot reassign account across projects');
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

DROP TRIGGER IF EXISTS trg_workflow_versions_isolation_update;
CREATE TRIGGER trg_workflow_versions_isolation_update
BEFORE UPDATE ON workflow_versions
FOR EACH ROW
WHEN NOT EXISTS (
    SELECT 1 FROM workflows w WHERE w.id = NEW.workflow_id AND w.project_id IS NOT NULL
) OR (
    NEW.workflow_id != OLD.workflow_id AND (
        SELECT w1.project_id FROM workflows w1 WHERE w1.id = OLD.workflow_id
    ) != (
        SELECT w2.project_id FROM workflows w2 WHERE w2.id = NEW.workflow_id
    )
)
BEGIN
    SELECT RAISE(ABORT, 'PROJECT_ISOLATION_VIOLATION: workflow_versions cannot be detached or moved across projects');
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

DROP TRIGGER IF EXISTS trg_jobs_isolation_update;
CREATE TRIGGER trg_jobs_isolation_update
BEFORE UPDATE ON jobs
FOR EACH ROW
WHEN NEW.project_id IS NULL
    OR (OLD.project_id IS NOT NULL AND NEW.project_id != OLD.project_id)
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

CREATE TRIGGER IF NOT EXISTS trg_vision_templates_cross_move_guard
BEFORE UPDATE OF project_id ON vision_templates
FOR EACH ROW
WHEN OLD.project_id IS NOT NULL AND NEW.project_id != OLD.project_id
BEGIN
    SELECT RAISE(ABORT, 'PROJECT_ISOLATION_VIOLATION: cannot reassign vision_template across projects');
END;

CREATE TRIGGER IF NOT EXISTS trg_project_modules_cross_move_guard
BEFORE UPDATE OF project_id ON project_modules
FOR EACH ROW
WHEN OLD.project_id IS NOT NULL AND NEW.project_id != OLD.project_id
BEGIN
    SELECT RAISE(ABORT, 'PROJECT_ISOLATION_VIOLATION: cannot reassign project_module across projects');
END;

CREATE TRIGGER IF NOT EXISTS trg_mini_tool_schemas_cross_move_guard
BEFORE UPDATE OF project_id ON mini_tool_schemas
FOR EACH ROW
WHEN OLD.project_id IS NOT NULL AND NEW.project_id != OLD.project_id
BEGIN
    SELECT RAISE(ABORT, 'PROJECT_ISOLATION_VIOLATION: cannot reassign mini_tool_schema across projects');
END;

CREATE TRIGGER IF NOT EXISTS trg_project_releases_cross_move_guard
BEFORE UPDATE OF project_id ON project_releases
FOR EACH ROW
WHEN OLD.project_id IS NOT NULL AND NEW.project_id != OLD.project_id
BEGIN
    SELECT RAISE(ABORT, 'PROJECT_ISOLATION_VIOLATION: cannot reassign project_release across projects');
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

const V6_PROVENANCE_COLUMNS: &[(&str, &str)] = &[
    (
        "confidence_source",
        "ALTER TABLE runtime_variable_envelopes ADD COLUMN confidence_source TEXT;",
    ),
    (
        "measured_confidence",
        "ALTER TABLE runtime_variable_envelopes ADD COLUMN measured_confidence REAL;",
    ),
    (
        "rule_matched",
        "ALTER TABLE runtime_variable_envelopes ADD COLUMN rule_matched INTEGER;",
    ),
    (
        "device_id",
        "ALTER TABLE runtime_variable_envelopes ADD COLUMN device_id TEXT;",
    ),
];

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

pub fn list_table_columns(conn: &Connection, table: &str) -> Result<Vec<String>> {
    let pragma_sql = format!("PRAGMA table_info({});", table);
    let mut stmt = conn.prepare(&pragma_sql)?;
    let cols = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(cols)
}

pub fn verify_v6_provenance_columns(conn: &Connection) -> Result<()> {
    let cols: HashSet<String> = list_table_columns(conn, "runtime_variable_envelopes")?
        .into_iter()
        .collect();
    for (col, _) in V6_PROVENANCE_COLUMNS {
        if !cols.contains(*col) {
            bail!(
                "SQLITE_SCHEMA_INCOMPATIBLE: table 'runtime_variable_envelopes' is missing required v6 column '{}' — refusing to operate on incomplete schema",
                col
            );
        }
    }
    Ok(())
}

pub const MIGRATION_V3_RUNTIME_PROVENANCE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS runtime_variable_snapshots (
    scope_key TEXT PRIMARY KEY,
    variables_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS runtime_variable_envelopes (
    scope_key TEXT NOT NULL,
    var_name TEXT NOT NULL,
    project_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    job_id TEXT NOT NULL,
    value_type TEXT NOT NULL,
    value_json TEXT NOT NULL,
    source TEXT NOT NULL,
    verified INTEGER NOT NULL DEFAULT 0,
    confidence REAL,
    updated_at_utc TEXT NOT NULL,
    workflow_version TEXT NOT NULL,
    seq INTEGER NOT NULL,
    redacted INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (scope_key, var_name)
);

CREATE TABLE IF NOT EXISTS runtime_var_streams (
    stream_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    last_seq INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS runtime_var_events (
    stream_id TEXT NOT NULL,
    seq INTEGER NOT NULL,
    project_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    job_id TEXT NOT NULL,
    workflow_version TEXT NOT NULL,
    deltas_json TEXT NOT NULL,
    emitted_at_utc TEXT NOT NULL,
    PRIMARY KEY (stream_id, seq)
);

CREATE INDEX IF NOT EXISTS idx_rt_env_project ON runtime_variable_envelopes(project_id, scope_key);
CREATE INDEX IF NOT EXISTS idx_rt_events_stream_seq ON runtime_var_events(stream_id, seq);
"#;

pub const MIGRATION_V4_RELEASE_LIFECYCLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS project_release_transitions (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    release_id TEXT NOT NULL REFERENCES project_releases(id) ON DELETE CASCADE,
    from_state TEXT NOT NULL,
    to_state TEXT NOT NULL,
    note TEXT NOT NULL,
    transitioned_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_release_transitions_rel
    ON project_release_transitions(release_id, transitioned_at DESC);
"#;

pub const MIGRATION_V5_PUBLISHER_PROFILES_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS publisher_profiles (
    id TEXT PRIMARY KEY,
    publisher_id TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    organization TEXT NOT NULL,
    website TEXT,
    support_contact TEXT,
    copyright TEXT NOT NULL,
    ed25519_key_id TEXT NOT NULL DEFAULT '',
    ed25519_pubkey_hex TEXT NOT NULL DEFAULT '',
    authenticode_cert_ref TEXT,
    release_channel TEXT NOT NULL DEFAULT 'internal',
    signing_policy TEXT NOT NULL DEFAULT 'unsigned_internal',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS used_key_rotation_nonces (
    nonce TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    old_pubkey_hex TEXT NOT NULL,
    new_pubkey_hex TEXT NOT NULL,
    accepted_at TEXT NOT NULL
);
"#;

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
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA synchronous = NORMAL;")?;
    let current_mode: String = conn
        .query_row("PRAGMA journal_mode;", [], |r| r.get(0))
        .unwrap_or_default();
    if !current_mode.eq_ignore_ascii_case("wal") {
        conn.execute_batch("PRAGMA journal_mode = WAL;")?;
    }

    // Fast-path: if schema_migrations already exists and has all v1..v4 applied,
    // avoid taking unnecessary DDL write locks during concurrent multi-worker startups.
    let has_mig_tbl: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='schema_migrations';",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if has_mig_tbl == 0 {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);",
            [],
        )?;
    }

    let applied: HashSet<i64> = list_applied_versions(conn)?.into_iter().collect();
    if let Some(max_applied) = applied.iter().copied().max() {
        if max_applied > CURRENT_SQLITE_SCHEMA_VERSION as i64 {
            bail!(
                "SQLITE_DOWNGRADE_INCOMPATIBLE: Database schema v{} exceeds runtime supported max v{}. Upgrade application or restore pre-update backup!",
                max_applied,
                CURRENT_SQLITE_SCHEMA_VERSION
            );
        }
    }

    if applied.contains(&1)
        && applied.contains(&2)
        && applied.contains(&3)
        && applied.contains(&4)
        && applied.contains(&5)
        && applied.contains(&6)
    {
        verify_v6_provenance_columns(conn)?;
        return list_applied_versions(conn);
    }
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

    // Migration v3: P5.1 Runtime Variable Provenance, Anti-Overwrite Partitioning & Delta Streams
    if !applied.contains(&3) {
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute_batch(MIGRATION_V3_RUNTIME_PROVENANCE_SQL)
            .context("Failed applying SQLite migration v3")?;

        // Upgrade pre-P5.1 legacy P4 `runtime_variable_snapshots` into `runtime_variable_envelopes`
        // strictly marked as LEGACY_UNVERIFIED (`verified = 0`) and scrubbed for secrets.
        let mut legacy_rows: Vec<(String, String, String)> = Vec::new();
        {
            let mut st = tx.prepare(
                "SELECT scope_key, variables_json, updated_at FROM runtime_variable_snapshots;",
            )?;
            let mut rows = st.query([])?;
            while let Some(r) = rows.next()? {
                legacy_rows.push((r.get(0)?, r.get(1)?, r.get(2)?));
            }
        }
        for (scope_key, vars_json, updated_at) in legacy_rows {
            if let Ok(map) =
                serde_json::from_str::<std::collections::HashMap<String, serde_json::Value>>(
                    &vars_json,
                )
            {
                let mut scrubbed_map = std::collections::HashMap::new();
                let mut has_vars = false;
                for (var_name, raw_val) in map {
                    let (clean_val, redacted) =
                        fp_security::sanitize_runtime_variable(&var_name, &raw_val);
                    scrubbed_map.insert(var_name.clone(), clean_val.clone());
                    let val_type = fp_domain::infer_json_value_type(&clean_val);
                    let clean_json = serde_json::to_string(&clean_val)?;
                    let proj_id = scope_key.strip_prefix("mini:").unwrap_or(&scope_key);
                    tx.execute(
                        "INSERT OR IGNORE INTO runtime_variable_envelopes (
                            scope_key, var_name, project_id, session_id, job_id,
                            value_type, value_json, source, verified, confidence,
                            updated_at_utc, workflow_version, seq, redacted
                         ) VALUES (?1, ?2, ?3, 'legacy-p4-migration', 'legacy-p4-migration', ?4, ?5, 'LEGACY_UNVERIFIED', 0, NULL, ?6, 'legacy', 1, ?7);",
                        params![
                            scope_key,
                            var_name,
                            proj_id,
                            val_type,
                            clean_json,
                            updated_at,
                            if redacted { 1 } else { 0 }
                        ],
                    )?;
                    has_vars = true;
                }
                if has_vars {
                    let scrubbed_json = serde_json::to_string(&scrubbed_map)?;
                    tx.execute(
                        "UPDATE runtime_variable_snapshots SET variables_json = ?1 WHERE scope_key = ?2;",
                        params![scrubbed_json, scope_key],
                    )?;
                    let proj_id = scope_key.strip_prefix("mini:").unwrap_or(&scope_key);
                    tx.execute(
                        "INSERT OR IGNORE INTO runtime_var_streams (stream_id, project_id, last_seq, updated_at)
                         VALUES (?1, ?2, 1, ?3);",
                        params![scope_key, proj_id, updated_at],
                    )?;
                }
            }
        }

        tx.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (3, ?1);",
            params![now],
        )?;
        tx.commit()?;
    }

    // Migration v4: P5.2 Release Lifecycle, Compatibility Metadata & Transition Audit Ledger
    if !applied.contains(&4) {
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute_batch(MIGRATION_V4_RELEASE_LIFECYCLE_SQL)
            .context("Failed applying SQLite migration v4 tables")?;

        let alters: &[(&str, &str)] = &[
            ("lifecycle_state", "ALTER TABLE project_releases ADD COLUMN lifecycle_state TEXT NOT NULL DEFAULT 'Published';"),
            ("release_channel", "ALTER TABLE project_releases ADD COLUMN release_channel TEXT NOT NULL DEFAULT 'internal';"),
            ("changelog_md", "ALTER TABLE project_releases ADD COLUMN changelog_md TEXT NOT NULL DEFAULT '';"),
            ("min_runtime_version", "ALTER TABLE project_releases ADD COLUMN min_runtime_version TEXT NOT NULL DEFAULT '0.1.0';"),
            ("workflow_schema_version", "ALTER TABLE project_releases ADD COLUMN workflow_schema_version INTEGER NOT NULL DEFAULT 1;"),
            ("ui_schema_version", "ALTER TABLE project_releases ADD COLUMN ui_schema_version INTEGER NOT NULL DEFAULT 1;"),
            ("module_api_version", "ALTER TABLE project_releases ADD COLUMN module_api_version TEXT NOT NULL DEFAULT '1.0.0';"),
            ("sqlite_schema_version", "ALTER TABLE project_releases ADD COLUMN sqlite_schema_version INTEGER NOT NULL DEFAULT 4;"),
            ("git_commit", "ALTER TABLE project_releases ADD COLUMN git_commit TEXT NOT NULL DEFAULT '';"),
            ("signature_status", "ALTER TABLE project_releases ADD COLUMN signature_status TEXT NOT NULL DEFAULT 'Ed25519-Verified';"),
            ("promoted_at", "ALTER TABLE project_releases ADD COLUMN promoted_at TEXT;"),
            ("rolled_back_from_id", "ALTER TABLE project_releases ADD COLUMN rolled_back_from_id TEXT;"),
        ];
        for (col, sql) in alters {
            if !table_has_column(&tx, "project_releases", col)? {
                tx.execute(sql, [])?;
            }
        }

        tx.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (4, ?1);",
            params![now],
        )?;
        tx.commit()?;
    }

    // Migration v5: P5.3 Publisher Profiles, Authenticode & PE VERSIONINFO Metadata, Key Rotation Replay Nonces
    if !applied.contains(&5) {
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute_batch(MIGRATION_V5_PUBLISHER_PROFILES_SQL)
            .context("Failed applying SQLite migration v5 tables")?;

        let proj_alters: &[(&str, &str)] = &[
            ("publisher_profile_id", "ALTER TABLE projects ADD COLUMN publisher_profile_id TEXT;"),
            ("signing_mode", "ALTER TABLE projects ADD COLUMN signing_mode TEXT NOT NULL DEFAULT 'unsigned_internal';"),
            ("version_info_json", "ALTER TABLE projects ADD COLUMN version_info_json TEXT;"),
        ];
        for (col, sql) in proj_alters {
            if !table_has_column(&tx, "projects", col)? {
                tx.execute(sql, [])?;
            }
        }

        let rel_alters: &[(&str, &str)] = &[
            ("publisher_profile_id", "ALTER TABLE project_releases ADD COLUMN publisher_profile_id TEXT;"),
            ("signing_mode", "ALTER TABLE project_releases ADD COLUMN signing_mode TEXT NOT NULL DEFAULT 'unsigned_internal';"),
            ("authenticode_trust_status", "ALTER TABLE project_releases ADD COLUMN authenticode_trust_status TEXT NOT NULL DEFAULT 'unsigned';"),
            ("authenticode_thumbprint", "ALTER TABLE project_releases ADD COLUMN authenticode_thumbprint TEXT;"),
            ("authenticode_subject", "ALTER TABLE project_releases ADD COLUMN authenticode_subject TEXT;"),
            ("trusted_public_release", "ALTER TABLE project_releases ADD COLUMN trusted_public_release INTEGER NOT NULL DEFAULT 0;"),
        ];
        for (col, sql) in rel_alters {
            if !table_has_column(&tx, "project_releases", col)? {
                tx.execute(sql, [])?;
            }
        }

        tx.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (5, ?1);",
            params![now],
        )?;
        tx.commit()?;
    }

    // Migration v6: Formal transactional RC.4 OCR & Device Provenance columns on `runtime_variable_envelopes`.
    // Upgrades pre-existing RC.1/RC.2/RC.3 v5 databases safely without relying on unversioned post-fast-path code.
    if !applied.contains(&6) {
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let cols_before: HashSet<String> =
            list_table_columns(&tx, "runtime_variable_envelopes")?
                .into_iter()
                .collect();

        for (col, sql) in V6_PROVENANCE_COLUMNS {
            if !cols_before.contains(*col) {
                tx.execute(sql, [])
                    .with_context(|| format!("Migration v6 failed adding column '{}'", col))?;
            }
        }

        // Legacy safety backfill: unknown or blank-source rows become LEGACY_UNVERIFIED (`verified = 0`)
        // and no legacy/unverified row is ever stamped with LIVE provenance or measured confidence.
        tx.execute(
            "UPDATE runtime_variable_envelopes
             SET source = 'LEGACY_UNVERIFIED',
                 verified = 0,
                 confidence = NULL,
                 confidence_source = NULL,
                 measured_confidence = NULL,
                 rule_matched = NULL
             WHERE source IS NULL
                OR TRIM(source) = ''
                OR source NOT IN ('LIVE_ADB_OCR', 'LIVE_STATE', 'TESTKIT_MOCK', 'LEGACY_UNVERIFIED', 'UNVERIFIED');",
            [],
        )?;
        tx.execute(
            "UPDATE runtime_variable_envelopes
             SET verified = 0,
                 confidence = NULL,
                 confidence_source = NULL,
                 measured_confidence = NULL,
                 rule_matched = NULL
             WHERE source IN ('LEGACY_UNVERIFIED', 'UNVERIFIED');",
            [],
        )?;

        // Verify actual schema via PRAGMA table_info & zero foreign-key violations BEFORE recording v6
        verify_v6_provenance_columns(&tx)?;
        let fk_violations: i64 = {
            let mut stmt = tx.prepare("PRAGMA foreign_key_check;")?;
            let rows = stmt.query_map([], |_| Ok(1i64))?;
            rows.count() as i64
        };
        if fk_violations > 0 {
            bail!(
                "Migration v6 aborted: detected {} foreign key violation(s)",
                fk_violations
            );
        }

        tx.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (6, ?1);",
            params![now],
        )?;
        tx.commit()?;
    } else {
        verify_v6_provenance_columns(conn)?;
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
