-- FarmerPro P2 SQLite Schema Migration (v1 -> v2)
-- Source: crates/fp-storage/src/migrations.rs

-- === MIGRATION_V1_SQL ===
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

-- === MIGRATION_V2_TABLES_SQL ===
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
    updated_at TEXT NOT NULL
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

-- === MIGRATION_V2_INDEXES_SQL ===
CREATE INDEX IF NOT EXISTS idx_workflows_project ON workflows(project_id);
CREATE INDEX IF NOT EXISTS idx_accounts_project ON accounts(project_id);
CREATE INDEX IF NOT EXISTS idx_jobs_project ON jobs(project_id);
CREATE INDEX IF NOT EXISTS idx_templates_project ON vision_templates(project_id);
CREATE INDEX IF NOT EXISTS idx_tasks_project_priority ON task_definitions(project_id, priority ASC);
CREATE INDEX IF NOT EXISTS idx_releases_project_time ON project_releases(project_id, built_at DESC);

-- === V2 ALTER TABLE & LEGACY BACKFILL (Idempotent inside BEGIN IMMEDIATE tx) ===
ALTER TABLE workflows ADD COLUMN project_id TEXT REFERENCES projects(id);
ALTER TABLE workflows ADD COLUMN category TEXT NOT NULL DEFAULT 'Chung';
ALTER TABLE accounts ADD COLUMN project_id TEXT REFERENCES projects(id);
ALTER TABLE jobs ADD COLUMN project_id TEXT REFERENCES projects(id);
-- Backfill legacy NULL project_id rows -> 'proj-jx2-mobile-001' if legacy data exists
