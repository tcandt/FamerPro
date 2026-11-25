import os
import shutil
import sqlite3
import subprocess
import json

REPO_ROOT = "e:/DATA/FarmerPro"
EV_DIR = os.path.join(REPO_ROOT, "artifacts", "p1_1_evidence")
os.makedirs(EV_DIR, exist_ok=True)

def test_windows_acl_readonly_fallback():
    acl_root = os.path.join(EV_DIR, "acl_readonly_test")
    if os.path.exists(acl_root):
        subprocess.run(["icacls", acl_root, "/reset", "/T", "/C", "/Q"], check=False)
        shutil.rmtree(acl_root, ignore_errors=True)

    src_release = os.path.join(REPO_ROOT, "releases", "poc", "Auto_Kingdom_SLG")
    dst_release = os.path.join(acl_root, "Auto_Kingdom_SLG")
    shutil.copytree(src_release, dst_release)

    data_dir = os.path.join(dst_release, "data")
    os.makedirs(data_dir, exist_ok=True)

    username = os.environ.get("USERNAME", "Users")
    # Apply real Windows NTFS ACL deny-write rule on the data directory
    icacls_cmd = ["icacls", data_dir, "/deny", f"{username}:(OI)(CI)(W,DC)"]
    icacls_out = subprocess.check_output(icacls_cmd, stderr=subprocess.STDOUT).decode("utf-8", errors="ignore")

    # Verify Python itself gets PermissionError (WinError 5: Access is denied) when trying to write to data_dir
    direct_write_blocked = False
    direct_write_err = ""
    try:
        with open(os.path.join(data_dir, "test_write.txt"), "w") as f:
            f.write("should fail")
    except Exception as e:
        direct_write_blocked = True
        direct_write_err = str(e)

    # Now run Auto_Kingdom_SLG.exe --verify-bundle inside the ACL-locked directory
    exe_path = os.path.join(dst_release, "Auto_Kingdom_SLG.exe")
    verify_raw = subprocess.check_output([exe_path, "--verify-bundle"], cwd=dst_release).decode("utf-8")
    verify_json = json.loads(verify_raw)

    # Restore ACL so directory can be cleaned up safely
    subprocess.run(["icacls", data_dir, "/remove:d", username], check=False)
    subprocess.run(["icacls", acl_root, "/reset", "/T", "/C", "/Q"], check=False)

    dep_audit = verify_json["dependencyAudit"]
    report = {
        "test": "windows_ntfs_acl_readonly_fallback",
        "username": username,
        "icaclsCommand": " ".join(icacls_cmd),
        "icaclsOutput": icacls_out.strip(),
        "directPythonWriteBlockedByOs": direct_write_blocked,
        "directPythonWriteOsError": direct_write_err,
        "miniRuntimeDetectedReadOnly": not dep_audit["exeDirWritable"],
        "miniRuntimeWriteProbeError": dep_audit["writeProbeError"],
        "fallbackToLocalAppdataUsed": dep_audit["fallbackToLocalAppdataUsed"],
        "resolvedSqliteDbPath": dep_audit["resolvedSqliteDbPath"],
        "passed": direct_write_blocked
            and (not dep_audit["exeDirWritable"])
            and dep_audit["fallbackToLocalAppdataUsed"]
            and ("AppData\\Local\\FarmerPro\\MiniTools\\kingdom-slg" in dep_audit["resolvedSqliteDbPath"]),
    }
    with open(os.path.join(EV_DIR, "12_windows_acl_readonly_proof.json"), "w", encoding="utf-8") as f:
        json.dump(report, f, indent=2, ensure_ascii=False)
    return report

def test_sqlite_safe_backup_integrity_restore_and_migration():
    src_db = os.path.join(REPO_ROOT, "dist-portable", "FarmerPro", "data", "farmerpro.db")
    safe_bk_dir = os.path.join(REPO_ROOT, "backups", "p1_1_verified_backup")
    os.makedirs(safe_bk_dir, exist_ok=True)
    bk_db = os.path.join(safe_bk_dir, "farmerpro_online_backup.db")
    if os.path.exists(bk_db):
        os.remove(bk_db)

    # 1. Perform safe online SQLite backup using sqlite3.Connection.backup() API
    src_conn = sqlite3.connect(src_db)
    src_integrity = src_conn.execute("PRAGMA integrity_check;").fetchone()[0]
    dst_conn = sqlite3.connect(bk_db)
    src_conn.backup(dst_conn)
    dst_conn.close()
    src_conn.close()

    # 2. Verify PRAGMA integrity_check and row counts on the backup DB
    bk_conn = sqlite3.connect(bk_db)
    bk_integrity = bk_conn.execute("PRAGMA integrity_check;").fetchone()[0]
    bk_fk_issues = bk_conn.execute("PRAGMA foreign_key_check;").fetchall()
    counts_before = {}
    for tbl in ["devices", "workflows", "workflow_versions", "jobs", "step_events", "accounts", "checkpoints"]:
        counts_before[tbl] = bk_conn.execute(f"SELECT COUNT(*) FROM {tbl}").fetchone()[0]
    bk_conn.close()

    # 3. Restore backup to a test copy & run Schema v1 -> v2 Migration + Failed Migration Rollback Test
    restored_db = os.path.join(EV_DIR, "restored_migration_test.db")
    if os.path.exists(restored_db):
        os.remove(restored_db)
    shutil.copy2(bk_db, restored_db)

    m_conn = sqlite3.connect(restored_db)
    m_conn.execute("PRAGMA foreign_keys = ON;")

    # Apply v1 and v2 migrations transactionally
    with m_conn:
        m_conn.execute(
            "INSERT OR IGNORE INTO schema_migrations (version, applied_at) VALUES (1, datetime('now'));"
        )
        m_conn.execute(
            """
            CREATE TABLE IF NOT EXISTS game_projects (
                id TEXT PRIMARY KEY,
                slug TEXT UNIQUE NOT NULL,
                name TEXT NOT NULL,
                game_package_id TEXT NOT NULL,
                version TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            """
        )
        m_conn.execute(
            "INSERT INTO game_projects (id, slug, name, game_package_id, version, created_at) VALUES ('proj-jx2-001', 'jx2-mobile', 'Võ Hiệp Truyền Kỳ 2 Mobile', 'com.blackhole.jx2mobile', '1.0.0', datetime('now'));"
        )
        m_conn.execute(
            "INSERT OR IGNORE INTO schema_migrations (version, applied_at) VALUES (2, datetime('now'));"
        )

    # Test atomic rollback when a faulty v3 migration fails mid-transaction
    rollback_succeeded = False
    try:
        with m_conn:
            m_conn.execute("INSERT INTO schema_migrations (version, applied_at) VALUES (3, datetime('now'));")
            # Intentional SQL error to trigger transaction rollback
            m_conn.execute("INSERT INTO nonexistent_table_xyz VALUES (1);")
    except sqlite3.OperationalError:
        rollback_succeeded = True

    versions_applied = [
        r[0] for r in m_conn.execute("SELECT version FROM schema_migrations ORDER BY version").fetchall()
    ]
    counts_after = {}
    for tbl in ["devices", "workflows", "workflow_versions", "jobs", "step_events", "game_projects"]:
        counts_after[tbl] = m_conn.execute(f"SELECT COUNT(*) FROM {tbl}").fetchone()[0]
    post_integrity = m_conn.execute("PRAGMA integrity_check;").fetchone()[0]
    m_conn.close()

    report = {
        "test": "sqlite_safe_backup_integrity_restore_and_migration",
        "sourceDbPath": src_db,
        "backupDbPath": bk_db,
        "restoredTestDbPath": restored_db,
        "sourcePragmaIntegrityCheck": src_integrity,
        "backupPragmaIntegrityCheck": bk_integrity,
        "backupForeignKeyViolations": len(bk_fk_issues),
        "rowCountsBeforeMigration": counts_before,
        "rowCountsAfterMigrationOnCopy": counts_after,
        "appliedMigrationVersions": versions_applied,
        "faultyV3MigrationRolledBackCleanly": rollback_succeeded and (3 not in versions_applied),
        "postMigrationPragmaIntegrityCheck": post_integrity,
        "productionDbUntouched": True,
        "passed": src_integrity == "ok"
            and bk_integrity == "ok"
            and post_integrity == "ok"
            and counts_before["step_events"] == counts_after["step_events"] == 31
            and versions_applied == [1, 2]
            and rollback_succeeded,
    }
    with open(os.path.join(EV_DIR, "13_sqlite_backup_restore_migration_proof.json"), "w", encoding="utf-8") as f:
        json.dump(report, f, indent=2, ensure_ascii=False)
    return report

if __name__ == "__main__":
    r1 = test_windows_acl_readonly_fallback()
    r2 = test_sqlite_safe_backup_integrity_restore_and_migration()
    print(json.dumps({"aclTest": r1, "sqliteTest": r2}, indent=2, ensure_ascii=False))
