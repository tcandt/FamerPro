import sqlite3
import os
import json

dbs = [
    ("Root Workspace DB", "e:/DATA/FarmerPro/farmerpro.db"),
    ("Portable Release DB", "e:/DATA/FarmerPro/dist-portable/FarmerPro/data/farmerpro.db"),
]

report = {}
for label, path in dbs:
    info = {"path": path, "exists": os.path.exists(path), "tables": {}}
    if os.path.exists(path):
        info["size_bytes"] = os.path.getsize(path)
        conn = sqlite3.connect(path)
        cur = conn.cursor()
        info["journal_mode"] = cur.execute("PRAGMA journal_mode;").fetchone()[0]
        tables = [
            r[0]
            for r in cur.execute(
                "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"
            ).fetchall()
        ]
        for t in tables:
            cnt = cur.execute(f"SELECT COUNT(*) FROM {t}").fetchone()[0]
            cols = [c[1] for c in cur.execute(f"PRAGMA table_info({t})").fetchall()]
            info["tables"][t] = {"row_count": cnt, "columns": cols}
        conn.close()
    report[label] = info

with open("e:/DATA/FarmerPro/artifacts/p0_evidence/sqlite_audit.json", "w", encoding="utf-8") as f:
    json.dump(report, f, indent=2, ensure_ascii=False)

print(json.dumps(report, indent=2, ensure_ascii=False))
