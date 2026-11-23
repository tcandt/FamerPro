import os
import shutil
import json
import hashlib
import sqlite3
import subprocess
import re

ROOT = r"E:\DATA\FarmerPro"
STAGE = os.path.join(ROOT, "artifacts", "gate1_sandbox", "rc4_ro_stage")
if os.path.exists(STAGE):
    shutil.rmtree(STAGE)
os.makedirs(os.path.join(STAGE, "fixtures"), exist_ok=True)
os.makedirs(os.path.join(STAGE, "scripts"), exist_ok=True)

DIST = os.path.join(ROOT, "artifacts", "p6_evidence", "dist_releases")
for fn in [
    "FarmerPro_0.1.0_x64-setup.exe",
    "P6_Alpha_v1.0.0_win-x64.zip",
    "P6_Beta_v1.0.0_win-x64.zip",
]:
    shutil.copy2(os.path.join(DIST, fn), os.path.join(STAGE, fn))
shutil.copytree(
    os.path.join(DIST, "P6_Beta_v1.1.0_win-x64"),
    os.path.join(STAGE, "P6_Beta_v1.1.0_win-x64"),
)

mig_src = subprocess.check_output(
    ["git", "show", "b5a0567:crates/fp-storage/src/migrations.rs"], cwd=ROOT
).decode("utf-8")


def grab(name):
    m = re.search(rf'pub const {name}: &str = r#"(.*?)"#;', mig_src, re.S)
    assert m, f"missing {name}"
    return m.group(1)


def create_v5_db(db_path, partial_cols=0):
    conn = sqlite3.connect(db_path)
    conn.execute("PRAGMA journal_mode = WAL;")
    conn.execute("PRAGMA foreign_keys = ON;")
    conn.executescript(grab("MIGRATION_V1_SQL"))
    conn.executescript(grab("MIGRATION_V2_TABLES_SQL"))
    for sql in [
        "ALTER TABLE workflows ADD COLUMN project_id TEXT;",
        "ALTER TABLE accounts ADD COLUMN project_id TEXT;",
        "ALTER TABLE jobs ADD COLUMN project_id TEXT;",
        "ALTER TABLE workflows ADD COLUMN category TEXT NOT NULL DEFAULT 'General';",
    ]:
        conn.execute(sql)
    conn.executescript(grab("MIGRATION_V2_INDEXES_SQL"))
    conn.executescript(grab("MIGRATION_V3_RUNTIME_PROVENANCE_SQL"))
    conn.executescript(grab("MIGRATION_V4_RELEASE_LIFECYCLE_SQL"))
    for sql in [
        "ALTER TABLE project_releases ADD COLUMN lifecycle_state TEXT NOT NULL DEFAULT 'Published';",
        "ALTER TABLE project_releases ADD COLUMN release_channel TEXT NOT NULL DEFAULT 'internal';",
        "ALTER TABLE project_releases ADD COLUMN changelog_md TEXT NOT NULL DEFAULT '';",
        "ALTER TABLE project_releases ADD COLUMN min_runtime_version TEXT NOT NULL DEFAULT '0.1.0';",
        "ALTER TABLE project_releases ADD COLUMN workflow_schema_version INTEGER NOT NULL DEFAULT 1;",
        "ALTER TABLE project_releases ADD COLUMN ui_schema_version INTEGER NOT NULL DEFAULT 1;",
        "ALTER TABLE project_releases ADD COLUMN module_api_version TEXT NOT NULL DEFAULT '1.0.0';",
        "ALTER TABLE project_releases ADD COLUMN sqlite_schema_version INTEGER NOT NULL DEFAULT 5;",
        "ALTER TABLE project_releases ADD COLUMN git_commit TEXT NOT NULL DEFAULT '';",
        "ALTER TABLE project_releases ADD COLUMN signature_status TEXT NOT NULL DEFAULT 'Ed25519-Verified';",
        "ALTER TABLE project_releases ADD COLUMN promoted_at TEXT;",
        "ALTER TABLE project_releases ADD COLUMN rolled_back_from_id TEXT;",
    ]:
        conn.execute(sql)
    conn.executescript(grab("MIGRATION_V5_PUBLISHER_PROFILES_SQL"))
    for sql in [
        "ALTER TABLE projects ADD COLUMN publisher_profile_id TEXT;",
        "ALTER TABLE projects ADD COLUMN signing_mode TEXT NOT NULL DEFAULT 'unsigned_internal';",
        "ALTER TABLE projects ADD COLUMN version_info_json TEXT;",
        "ALTER TABLE project_releases ADD COLUMN publisher_profile_id TEXT;",
        "ALTER TABLE project_releases ADD COLUMN signing_mode TEXT NOT NULL DEFAULT 'unsigned_internal';",
        "ALTER TABLE project_releases ADD COLUMN authenticode_trust_status TEXT NOT NULL DEFAULT 'unsigned';",
        "ALTER TABLE project_releases ADD COLUMN authenticode_thumbprint TEXT;",
        "ALTER TABLE project_releases ADD COLUMN authenticode_subject TEXT;",
        "ALTER TABLE project_releases ADD COLUMN trusted_public_release INTEGER NOT NULL DEFAULT 0;",
    ]:
        conn.execute(sql)

    if partial_cols >= 1:
        conn.execute("ALTER TABLE runtime_variable_envelopes ADD COLUMN confidence_source TEXT;")
    if partial_cols >= 2:
        conn.execute("ALTER TABLE runtime_variable_envelopes ADD COLUMN device_id TEXT;")
    if partial_cols >= 3:
        conn.execute("ALTER TABLE runtime_variable_envelopes ADD COLUMN measured_confidence REAL;")

    now = "2026-11-22T18:00:00Z"
    for v in range(1, 6):
        conn.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?, ?);",
            (v, now),
        )
    conn.execute(
        """INSERT INTO projects (id, slug, name, game_package_id, version, exe_name, window_title, icon_rel_path, target_width_px, target_height_px, bundle_adb, project_dir_path, created_at, updated_at, signing_mode)
           VALUES ('proj-g1-rc2', 'gate1-rc2-game', 'Gate1 RC2 Game', 'com.gate1.rc2', '1.0.0', 'Auto_Gate1.exe', 'Auto Gate1', 'branding/icon.ico', 1280, 720, 1, 'projects/gate1-rc2', ?, ?, 'unsigned_internal');""",
        (now, now),
    )
    conn.execute(
        """INSERT INTO workflows (id, project_id, category, name, draft_json, draft_updated_at)
           VALUES ('wf-g1-rc2', 'proj-g1-rc2', 'Daily', 'Gate1 RC2 Workflow', '{"schemaVersion":1}', ?);""",
        (now,),
    )
    conn.execute(
        """INSERT INTO task_definitions (id, project_id, workflow_id, category_id, category_short_tag, category_name, category_color_hex, title, priority, estimate_sec, default_enabled, settings_schema_json, updated_at)
           VALUES ('task-g1-rc2', 'proj-g1-rc2', 'wf-g1-rc2', 'daily', 'DL', 'Daily', '#1878F5', 'Gate1 Daily', 1, 30, 1, '[]', ?);""",
        (now,),
    )
    conn.execute(
        """INSERT INTO accounts (id, project_id, alias, game_profile_id, secret_ref, state, updated_at)
           VALUES ('acc-g1-rc2', 'proj-g1-rc2', 'g1_rc2_acc', 'gate1-rc2', 'dpapi_ref_g1_rc2', 'Ready', ?);""",
        (now,),
    )
    conn.execute(
        """INSERT INTO publisher_profiles (id, publisher_id, display_name, organization, copyright, created_at, updated_at)
           VALUES ('pub-g1', 'pub-g1-rc2', 'Gate1 Org', 'Gate1 Org', 'Copyright 2026', ?, ?);""",
        (now, now),
    )
    conn.execute(
        """INSERT INTO project_releases (id, project_id, version, output_dir_path, exe_path, exe_size_bytes, exe_sha256, key_id, publisher_public_key_hex, signature_ed25519_hex, build_report_json, built_at, lifecycle_state, release_channel, sqlite_schema_version)
           VALUES ('rel-g1-rc2', 'proj-g1-rc2', '1.0.0', 'dist/g1', 'dist/g1/Auto_Gate1.exe', 1024, 'abcd', 'k1', 'pk1', 'sig1', '{}', ?, 'Published', 'internal', 5);""",
        (now,),
    )
    conn.execute(
        """INSERT INTO runtime_var_streams (stream_id, project_id, last_seq, updated_at)
           VALUES ('proj-g1-rc2', 'proj-g1-rc2', 7, ?);""",
        (now,),
    )
    conn.execute(
        """INSERT INTO runtime_variable_envelopes (scope_key, var_name, project_id, session_id, job_id, value_type, value_json, source, verified, confidence, updated_at_utc, workflow_version, seq, redacted)
           VALUES ('proj-g1-rc2', 'legacy_g1_ocr', 'proj-g1-rc2', 's-rc2', 'j-rc2', 'string', '"Settings"', 'LEGACY_UNVERIFIED', 1, 0.94, ?, 'wf-g1-rc2@v1', 7, 0);""",
        (now,),
    )
    conn.commit()
    # Close WAL checkpoint cleanly into single .db file for portable read-only fixture copy
    conn.execute("PRAGMA wal_checkpoint(TRUNCATE);")
    conn.close()


create_v5_db(os.path.join(STAGE, "fixtures", "rc2_v5_clean.db"), partial_cols=0)
create_v5_db(os.path.join(STAGE, "fixtures", "rc2_v5_partial_2cols.db"), partial_cols=2)
create_v5_db(os.path.join(STAGE, "fixtures", "rc2_v5_partial_3cols.db"), partial_cols=3)


def sha256(p):
    h = hashlib.sha256()
    with open(p, "rb") as f:
        for c in iter(lambda: f.read(65536), b""):
            h.update(c)
    return h.hexdigest()


man = {}
for r, _, files in os.walk(STAGE):
    for fn in sorted(files):
        fp = os.path.join(r, fn)
        rel = os.path.relpath(fp, STAGE).replace("\\", "/")
        man[rel] = {"bytes": os.path.getsize(fp), "sha256": sha256(fp)}
with open(os.path.join(STAGE, "stage_manifest_sha256.json"), "w", encoding="utf-8") as f:
    json.dump(man, f, indent=2)
print("STAGED_FILES:", len(man))
