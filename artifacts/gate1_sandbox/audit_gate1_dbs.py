import os
import json
import sqlite3

def audit_studio_db(p):
    conn = sqlite3.connect(p)
    cur = conn.cursor()
    migs = [r[0] for r in cur.execute("SELECT version FROM schema_migrations ORDER BY version").fetchall()]
    cols = [r[1] for r in cur.execute("PRAGMA table_info(runtime_variable_envelopes)").fetchall()]
    ic = [r[0] for r in cur.execute("PRAGMA integrity_check").fetchall()]
    fk = cur.execute("PRAGMA foreign_key_check").fetchall()
    projs = cur.execute("SELECT id, slug, name FROM projects").fetchall()
    wfs = cur.execute("SELECT id, project_id, name FROM workflows").fetchall()
    tasks = cur.execute("SELECT COUNT(*) FROM task_definitions").fetchone()[0]
    accs = cur.execute("SELECT COUNT(*) FROM accounts").fetchone()[0]
    pubs = cur.execute("SELECT COUNT(*) FROM publisher_profiles").fetchone()[0]
    rels = cur.execute("SELECT COUNT(*) FROM project_releases").fetchone()[0]
    vars_rows = cur.execute(
        "SELECT var_name, source, verified, confidence, confidence_source, measured_confidence, rule_matched, device_id FROM runtime_variable_envelopes"
    ).fetchall()
    conn.close()
    return {
        "file": os.path.basename(p),
        "schemaMigrations": migs,
        "envColsCount": len(cols),
        "hasAll4ProvenanceCols": all(c in cols for c in ["confidence_source", "measured_confidence", "rule_matched", "device_id"]),
        "integrityCheck": ic,
        "fkViolations": len(fk),
        "projects": projs,
        "workflows": wfs,
        "counts": {"tasks": tasks, "accounts": accs, "publisher_profiles": pubs, "project_releases": rels},
        "runtimeVarRows": vars_rows,
    }

def audit_mini_db(p):
    conn = sqlite3.connect(p)
    cur = conn.cursor()
    tables = [r[0] for r in cur.execute("SELECT name FROM sqlite_master WHERE type='table'").fetchall()]
    migs = [r[0] for r in cur.execute("SELECT version FROM schema_migrations ORDER BY version").fetchall()] if "schema_migrations" in tables else []
    jobs_cols = [r[1] for r in cur.execute("PRAGMA table_info(jobs)").fetchall()] if "jobs" in tables else []
    jobs = cur.execute("SELECT * FROM jobs").fetchall() if "jobs" in tables else []
    evts = cur.execute("SELECT COUNT(*) FROM step_events").fetchone()[0] if "step_events" in tables else 0
    ic = [r[0] for r in cur.execute("PRAGMA integrity_check").fetchall()]
    conn.close()
    return {"file": os.path.basename(p), "schemaMigrations": migs, "jobsCols": jobs_cols, "jobsCount": len(jobs), "jobs": jobs[:10], "stepEventsCount": evts, "integrityCheck": ic}

out = {"studioDbs": {}, "miniDbs": {}}
snap_dir = r"E:\DATA\FarmerPro\artifacts\gate1_sandbox\evidence_offline\db_snapshots"
for fn in ["studio_after_create_reopen.db", "upgraded_rc2_v5_clean.db", "upgraded_rc2_v5_partial_2cols.db", "upgraded_rc2_v5_partial_3cols.db"]:
    p = os.path.join(snap_dir, fn)
    if os.path.exists(p):
        out["studioDbs"][fn] = audit_studio_db(p)

for fn in ["portable_alpha_standalone.db", "portable_beta_standalone.db"]:
    p = os.path.join(snap_dir, fn)
    if os.path.exists(p):
        out["miniDbs"][fn] = audit_mini_db(p)

upd_dir = r"E:\DATA\FarmerPro\artifacts\gate1_sandbox\evidence_offline\updater_evidence"
for fn in ["beta_v100_pre_upgrade.db", "beta_v110_post_upgrade.db", "beta_v100_post_rollback.db", "beta_v110_post_reupgrade.db"]:
    p = os.path.join(upd_dir, fn)
    if os.path.exists(p):
        out["miniDbs"][fn] = audit_mini_db(p)

print(json.dumps(out, indent=2))
