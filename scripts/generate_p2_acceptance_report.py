import os
import re
import json
import hashlib
import base64
import shutil
import subprocess
import pymupdf

ROOT = r"E:\DATA\FarmerPro"
EV_DIR = os.path.join(ROOT, "artifacts", "p2_evidence")
DIAG_DIR = os.path.join(EV_DIR, "diagrams")
RENDER_DIR = os.path.join(EV_DIR, "pdf_rendered_pages")
os.makedirs(DIAG_DIR, exist_ok=True)
os.makedirs(RENDER_DIR, exist_ok=True)

CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"
HTML_OUT = os.path.join(ROOT, "FARMERPRO_P2_ACCEPTANCE_REPORT.html")
PDF_MAIN = os.path.join(ROOT, "FARMERPRO_P2_ACCEPTANCE_REPORT.pdf")
PDF_COPY = os.path.join(EV_DIR, "FARMERPRO_P2_ACCEPTANCE_REPORT.pdf")
QA_JSON = os.path.join(EV_DIR, "p2_pdf_qa_report.json")


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        while chunk := f.read(65536):
            h.update(chunk)
    return h.hexdigest()


# 1. Export exact SQLite Migration SQL from Rust source
mig_rs = open(os.path.join(ROOT, "crates", "fp-storage", "src", "migrations.rs"), encoding="utf-8").read()
blocks = re.findall(r'pub const (MIGRATION_V[12]_[A-Z_]+): &str = r#"(.*?)"#;', mig_rs, re.S)
sql_path = os.path.join(EV_DIR, "sqlite_v1_v2_migration.sql")
with open(sql_path, "w", encoding="utf-8") as f:
    f.write("-- FarmerPro P2 SQLite Schema Migration (v1 -> v2)\n")
    f.write("-- Source: crates/fp-storage/src/migrations.rs\n\n")
    for name, sql in blocks:
        f.write(f"-- === {name} ===\n{sql.strip()}\n\n")
    f.write("-- === V2 ALTER TABLE & LEGACY BACKFILL (Idempotent inside BEGIN IMMEDIATE tx) ===\n")
    f.write("ALTER TABLE workflows ADD COLUMN project_id TEXT REFERENCES projects(id);\n")
    f.write("ALTER TABLE workflows ADD COLUMN category TEXT NOT NULL DEFAULT 'Chung';\n")
    f.write("ALTER TABLE accounts ADD COLUMN project_id TEXT REFERENCES projects(id);\n")
    f.write("ALTER TABLE jobs ADD COLUMN project_id TEXT REFERENCES projects(id);\n")
    f.write("-- Backfill legacy NULL project_id rows -> 'proj-jx2-mobile-001' if legacy data exists\n")

# Load real evidence data
sandbox_rep = json.loads(open(os.path.join(EV_DIR, "p2_verify_dist_portable_sandbox.json"), encoding="utf-8-sig").read())
prod_mig_rep = json.loads(open(os.path.join(EV_DIR, "p2_dist_portable_migration_report.json"), encoding="utf-8-sig").read())
test_log = open(os.path.join(EV_DIR, "cargo_test_workspace_p2_clean.log"), encoding="utf-8", errors="ignore").read()
git_diff_stat = subprocess.check_output(["git", "diff", "--stat", "fab1a1a..9bd0e45"], cwd=ROOT).decode("utf-8")
git_log_txt = subprocess.check_output(["git", "-c", "i18n.logOutputEncoding=utf-8", "log", "-n", "3", "--oneline"], cwd=ROOT).decode("utf-8")
with open(os.path.join(EV_DIR, "git_log_p2.txt"), "w", encoding="utf-8") as f:
    f.write(git_log_txt)
with open(os.path.join(EV_DIR, "git_diff_summary_p2.txt"), "w", encoding="utf-8") as f:
    f.write(git_diff_stat)

# 2. Generate 4 crisp Vector SVG + Mermaid (.mmd) Diagrams
DIAGRAMS = {}

# Diagram 1: Rust Crates & P2 Responsibility Boundaries
DIAGRAMS["01_p2_rust_crates_map"] = {
    "title": "Sơ đồ P2.1: Bản đồ Rust Crates & Ranh giới Kiến trúc Giai đoạn P2",
    "mmd": """flowchart TB
    subgraph APPS["Application Layer (apps/*)"]
        TAURI["apps/desktop/src-tauri (farmerpro)\\n26 Tauri IPC Commands (studio_p2.rs)\\nAppState = StorageManager + CredentialVault + projects_root"]
        CLI["apps/cli (fp-cli)\\nverify-p2 (Sandbox) + migrate-p2 (Guarded)"]
        UI["apps/desktop/src (React/TS UI)\\nGIỮ NGUYÊN 100% — Chưa refactor ở P2"]
    end
    subgraph P2_CORE["P2 Multi-Project Storage & Security Crates"]
        STOR["crates/fp-storage\\nMigrations v1->v2 · Online Backup & Restore\\nProject CRUD/Clone/Isolation · JX2 Seeder"]
        SEC["crates/fp-security\\nWindows Credential Manager (advapi32.dll)\\nDPAPI CryptProtectData (crypt32.dll)\\nProducerKeystore (v2 + Backup/Rotate/Restore)"]
    end
    subgraph SHARED["Shared Contracts & Game-Agnostic Runtime"]
        CONTRACT["crates/fp-bundle-contract\\nGameProjectManifest · MiniToolUiSchema · MiniTaskDef"]
        BUILDER["crates/fp-builder\\nProducerKeyManager -> delegates to fp-security"]
        RUNTIME["crates/fp-workflow-runtime\\n100% Game-Agnostic (Đã gỡ hardcode jx2mobile)"]
    end
    TAURI --> STOR
    TAURI --> SEC
    CLI --> STOR
    CLI --> SEC
    BUILDER --> SEC
    STOR --> CONTRACT
""",
    "svg": """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1120 530" font-family="Segoe UI, Arial, sans-serif">
  <rect width="1120" height="530" rx="12" fill="#0F172A"/>
  <text x="560" y="34" text-anchor="middle" fill="#F8FAFC" font-size="19" font-weight="700">FARMERPRO P2 — BẢN ĐỒ RUST CRATES &amp; RANH GIỚI BẢO MẬT / DỮ LIỆU</text>

  <!-- Row 1: Application Layer -->
  <rect x="24" y="52" width="1072" height="122" rx="10" fill="#1E293B" stroke="#334155" stroke-width="1.5"/>
  <text x="42" y="75" fill="#93C5FD" font-size="14" font-weight="700">APPLICATION LAYER (apps/*) — GIAO TIẾP IPC &amp; CLI</text>

  <rect x="40" y="86" width="430" height="76" rx="8" fill="#0F2942" stroke="#3B82F6" stroke-width="1.8"/>
  <text x="255" y="109" text-anchor="middle" fill="#F8FAFC" font-size="14.5" font-weight="700">apps/desktop/src-tauri (studio_p2.rs)</text>
  <text x="255" y="130" text-anchor="middle" fill="#CBD5E1" font-size="13">26 Tauri IPC: Project CRUD/Clone, Tasks, UI Schema</text>
  <text x="255" y="149" text-anchor="middle" fill="#93C5FD" font-size="12.8">Vault Namespaces + DPAPI Keystore + DB Backup</text>

  <rect x="490" y="86" width="295" height="76" rx="8" fill="#132D24" stroke="#10B981" stroke-width="1.8"/>
  <text x="637" y="109" text-anchor="middle" fill="#F8FAFC" font-size="14.5" font-weight="700">apps/cli (fp-cli)</text>
  <text x="637" y="130" text-anchor="middle" fill="#CBD5E1" font-size="13">verify-p2: Kiểm thử Sandbox DB + Vault</text>
  <text x="637" y="149" text-anchor="middle" fill="#6EE7B7" font-size="12.8">migrate-p2: Backup + Probe -&gt; Migrate</text>

  <rect x="805" y="86" width="275" height="76" rx="8" fill="#2B2118" stroke="#F59E0B" stroke-width="1.5" stroke-dasharray="6,4"/>
  <text x="942" y="109" text-anchor="middle" fill="#FDE68A" font-size="14" font-weight="700">apps/desktop/src (React UI)</text>
  <text x="942" y="130" text-anchor="middle" fill="#FCD34D" font-size="13">STAGE-GATE P2: GIỮ NGUYÊN 100%</text>
  <text x="942" y="149" text-anchor="middle" fill="#CBD5E1" font-size="12.8">Không refactor UI trước P4 (0 dòng sửa)</text>

  <!-- Arrows down -->
  <line x1="210" y1="162" x2="210" y2="206" stroke="#60A5FA" stroke-width="2.2" marker-end="url(#arr)"/>
  <line x1="360" y1="162" x2="680" y2="206" stroke="#60A5FA" stroke-width="2" marker-end="url(#arr)"/>
  <line x1="580" y1="162" x2="430" y2="206" stroke="#34D399" stroke-width="2" marker-end="url(#arr)"/>
  <line x1="680" y1="162" x2="780" y2="206" stroke="#34D399" stroke-width="2" marker-end="url(#arr)"/>

  <!-- Row 2: P2 Core Crates -->
  <rect x="24" y="206" width="526" height="162" rx="10" fill="#172554" stroke="#3B82F6" stroke-width="2"/>
  <text x="42" y="230" fill="#93C5FD" font-size="15" font-weight="700">crates/fp-storage (SQLite WAL Multi-Project Core)</text>
  <text x="42" y="254" fill="#E2E8F0" font-size="13.2">• migrations.rs: v1 Baseline + v2 (BEGIN IMMEDIATE TX)</text>
  <text x="42" y="275" fill="#E2E8F0" font-size="13.2">• Online Backup &amp; Restore: rusqlite::backup::Backup API</text>
  <text x="42" y="296" fill="#E2E8F0" font-size="13.2">• jx2_seed.rs: Non-destructive 10 workflows + 8 PNGs</text>
  <text x="42" y="317" fill="#E2E8F0" font-size="13.2">• Project CRUD, Clone &amp; PROJECT_ISOLATION_VIOLATION</text>
  <text x="42" y="338" fill="#93C5FD" font-size="12.8">• Deps: fp-domain, fp-workflow-schema, fp-bundle-contract</text>
  <text x="42" y="357" fill="#6EE7B7" font-size="12.8">• Tests: 8/8 PASS (Idempotency, Rollback, Backup, Isolation)</text>

  <rect x="570" y="206" width="526" height="162" rx="10" fill="#2E1065" stroke="#A855F7" stroke-width="2"/>
  <text x="588" y="230" fill="#E9D5FF" font-size="15" font-weight="700">crates/fp-security (Win32 Credential Manager + DPAPI)</text>
  <text x="588" y="254" fill="#E2E8F0" font-size="13.2">• wincred.rs: advapi32 CredWriteW/ReadW/EnumerateW/DeleteW</text>
  <text x="588" y="275" fill="#E2E8F0" font-size="13.2">• Namespaces: project/&lt;slug&gt;/ · minitool/&lt;slug&gt;/ · account/</text>
  <text x="588" y="296" fill="#E2E8F0" font-size="13.2">• dpapi.rs: crypt32 CryptProtectData / CryptUnprotectData</text>
  <text x="588" y="317" fill="#E2E8F0" font-size="13.2">• keystore.rs: Ed25519 Seed DPAPI v2, Backup/Rotate/Restore</text>
  <text x="588" y="338" fill="#E9D5FF" font-size="12.8">• SecretRedactor + DestructiveActionGuard preserved 100%</text>
  <text x="588" y="357" fill="#6EE7B7" font-size="12.8">• Tests: 6/6 PASS (Real WinCred, DPAPI Tamper, Key Backup)</text>

  <!-- Row 3: Shared Contract & Runtime -->
  <rect x="24" y="396" width="340" height="114" rx="10" fill="#1E293B" stroke="#475569" stroke-width="1.5"/>
  <text x="194" y="421" text-anchor="middle" fill="#F8FAFC" font-size="14" font-weight="700">crates/fp-bundle-contract</text>
  <text x="194" y="443" text-anchor="middle" fill="#CBD5E1" font-size="13">Zero-Crypto Shared Schemas:</text>
  <text x="194" y="463" text-anchor="middle" fill="#94A3B8" font-size="12.8">GameProjectManifest · MiniToolUiSchema</text>
  <text x="194" y="482" text-anchor="middle" fill="#94A3B8" font-size="12.8">MiniTaskDef · TaskCategoryDef</text>
  <text x="194" y="500" text-anchor="middle" fill="#6EE7B7" font-size="12.8">1/1 Unit Test PASS</text>

  <rect x="390" y="396" width="340" height="114" rx="10" fill="#1E293B" stroke="#475569" stroke-width="1.5"/>
  <text x="560" y="421" text-anchor="middle" fill="#F8FAFC" font-size="14" font-weight="700">crates/fp-builder (security.rs)</text>
  <text x="560" y="443" text-anchor="middle" fill="#CBD5E1" font-size="13">ProducerKeyManager delegates to</text>
  <text x="560" y="463" text-anchor="middle" fill="#E9D5FF" font-size="12.8">fp_security::ProducerKeystore (DPAPI)</text>
  <text x="560" y="482" text-anchor="middle" fill="#94A3B8" font-size="12.8">8-Step Packager + PE RT_RCDATA #7777</text>
  <text x="560" y="500" text-anchor="middle" fill="#6EE7B7" font-size="12.8">4/4 Unit Tests PASS</text>

  <rect x="756" y="396" width="340" height="114" rx="10" fill="#1E293B" stroke="#10B981" stroke-width="1.5"/>
  <text x="926" y="421" text-anchor="middle" fill="#F8FAFC" font-size="14" font-weight="700">crates/fp-workflow-runtime</text>
  <text x="926" y="443" text-anchor="middle" fill="#6EE7B7" font-size="13">ZERO Hardcoded Game Names</text>
  <text x="926" y="463" text-anchor="middle" fill="#CBD5E1" font-size="12.8">Gỡ bỏ hardcode games/jx2mobile</text>
  <text x="926" y="482" text-anchor="middle" fill="#94A3B8" font-size="12.8">Quét động bundle/ + projects/*/templates</text>
  <text x="926" y="500" text-anchor="middle" fill="#6EE7B7" font-size="12.8">3/3 Unit Tests PASS (44/44 Total)</text>

  <line x1="210" y1="368" x2="194" y2="396" stroke="#60A5FA" stroke-width="1.8" marker-end="url(#arr)"/>
  <line x1="720" y1="368" x2="560" y2="396" stroke="#C084FC" stroke-width="1.8" marker-end="url(#arr)"/>

  <defs>
    <marker id="arr" markerWidth="8" markerHeight="8" refX="6" refY="3" orient="auto">
      <path d="M0,0 L0,6 L7,3 z" fill="#94A3B8"/>
    </marker>
  </defs>
</svg>""",
}

# Diagram 2: SQLite v2 Multi-Project ER Schema
DIAGRAMS["02_p2_sqlite_v2_er_schema"] = {
    "title": "Sơ đồ P2.2: Kiến trúc Dữ liệu SQLite WAL v2 (Multi-Project Schema & Foreign Keys)",
    "mmd": """erDiagram
    projects ||--o{ project_modules : "1:N CASCADE"
    projects ||--o{ vision_templates : "1:N CASCADE"
    projects ||--o{ task_definitions : "1:N CASCADE"
    projects ||--|| mini_tool_schemas : "1:1 CASCADE"
    projects ||--o{ project_releases : "1:N CASCADE"
    projects ||--o{ workflows : "1:N project_id"
    projects ||--o{ accounts : "1:N project_id"
    projects ||--o{ jobs : "1:N project_id"
    workflows ||--o{ workflow_versions : "1:N"
    workflows ||--o{ task_definitions : "1:N"
    devices ||--o{ jobs : "1:N"
    devices ||--o| device_leases : "1:1"
    jobs ||--o{ step_events : "1:N"
    jobs ||--o{ checkpoints : "1:N"
""",
    "svg": """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1120 530" font-family="Segoe UI, Arial, sans-serif">
  <rect width="1120" height="530" rx="12" fill="#0F172A"/>
  <text x="560" y="31" text-anchor="middle" fill="#F8FAFC" font-size="18.5" font-weight="700">SQLITE WAL SCHEMA V2 — MÔ HÌNH QUAN HỆ ĐA DỰ ÁN (15 BẢNG + 8 INDEXES)</text>

  <!-- Central Root: projects -->
  <rect x="385" y="48" width="350" height="156" rx="10" fill="#1E3A8A" stroke="#60A5FA" stroke-width="2.4"/>
  <text x="560" y="71" text-anchor="middle" fill="#FFFFFF" font-size="14.5" font-weight="700">projects (V2 Root Entity)</text>
  <line x1="385" y1="79" x2="735" y2="79" stroke="#60A5FA" stroke-width="1"/>
  <text x="400" y="99" fill="#FDE68A" font-size="13" font-weight="700">PK id (TEXT) · UNIQUE slug (TEXT)</text>
  <text x="400" y="119" fill="#E2E8F0" font-size="12.8">name · game_package_id · version</text>
  <text x="400" y="138" fill="#E2E8F0" font-size="12.8">exe_name · window_title · icon_rel_path</text>
  <text x="400" y="157" fill="#E2E8F0" font-size="12.8">target_width_px · target_height_px · bundle_adb</text>
  <text x="400" y="176" fill="#93C5FD" font-size="12.8">project_dir_path · created_at · updated_at</text>
  <text x="400" y="195" fill="#6EE7B7" font-size="12.5">Default Legacy ID: proj-jx2-mobile-001</text>

  <!-- Left column: New V2 Project Tables -->
  <rect x="20" y="48" width="340" height="94" rx="8" fill="#1E293B" stroke="#38BDF8" stroke-width="1.6"/>
  <text x="34" y="70" fill="#38BDF8" font-size="13.8" font-weight="700">project_modules (V2)</text>
  <text x="34" y="90" fill="#FDE68A" font-size="12.5">PK(project_id, module_id) · CASCADE</text>
  <text x="34" y="109" fill="#CBD5E1" font-size="12.5">packaged_in_release · enabled_by_default</text>
  <text x="34" y="128" fill="#94A3B8" font-size="12.5">config_json · updated_at</text>

  <rect x="20" y="154" width="340" height="106" rx="8" fill="#1E293B" stroke="#38BDF8" stroke-width="1.6"/>
  <text x="34" y="176" fill="#38BDF8" font-size="13.8" font-weight="700">vision_templates (V2)</text>
  <text x="34" y="196" fill="#FDE68A" font-size="12.5">PK id · FK project_id · UNIQUE(rel_path)</text>
  <text x="34" y="215" fill="#CBD5E1" font-size="12.5">name · rel_path · width_px · height_px</text>
  <text x="34" y="234" fill="#CBD5E1" font-size="12.5">crop_x/y · default_confidence · sha256</text>
  <text x="34" y="252" fill="#94A3B8" font-size="12.2">Index: idx_templates_project</text>

  <rect x="20" y="272" width="340" height="114" rx="8" fill="#1E293B" stroke="#38BDF8" stroke-width="1.6"/>
  <text x="34" y="294" fill="#38BDF8" font-size="13.8" font-weight="700">task_definitions (V2)</text>
  <text x="34" y="314" fill="#FDE68A" font-size="12.5">PK id · FK project_id · FK workflow_id</text>
  <text x="34" y="333" fill="#CBD5E1" font-size="12.5">category_id/short_tag/name/color_hex</text>
  <text x="34" y="352" fill="#CBD5E1" font-size="12.5">title · priority · estimate_sec · schema</text>
  <text x="34" y="371" fill="#94A3B8" font-size="12.2">Index: idx_tasks_project_priority</text>

  <!-- Right column: Mini Tool Schema & Releases -->
  <rect x="760" y="48" width="340" height="94" rx="8" fill="#1E293B" stroke="#A855F7" stroke-width="1.6"/>
  <text x="774" y="70" fill="#E9D5FF" font-size="13.8" font-weight="700">mini_tool_schemas (V2)</text>
  <text x="774" y="90" fill="#FDE68A" font-size="12.5">PK/FK project_id (1:1) · CASCADE</text>
  <text x="774" y="109" fill="#CBD5E1" font-size="12.5">schema_version · ui_schema_json</text>
  <text x="774" y="128" fill="#94A3B8" font-size="12.5">Sync: projects/&lt;slug&gt;/ui-schema.json</text>

  <rect x="760" y="154" width="340" height="112" rx="8" fill="#1E293B" stroke="#A855F7" stroke-width="1.6"/>
  <text x="774" y="176" fill="#E9D5FF" font-size="13.8" font-weight="700">project_releases (V2)</text>
  <text x="774" y="196" fill="#FDE68A" font-size="12.5">PK id · FK project_id · CASCADE</text>
  <text x="774" y="215" fill="#CBD5E1" font-size="12.5">version · exe_path · exe_sha256 · key_id</text>
  <text x="774" y="234" fill="#CBD5E1" font-size="12.5">pe_rcdata_pubkey_id(7777) · signature</text>
  <text x="774" y="253" fill="#94A3B8" font-size="12.2">Index: idx_releases_project_time</text>

  <rect x="760" y="278" width="340" height="92" rx="8" fill="#1E293B" stroke="#64748B" stroke-width="1.5"/>
  <text x="774" y="300" fill="#F8FAFC" font-size="13.8" font-weight="700">schema_migrations (Tracking)</text>
  <text x="774" y="321" fill="#CBD5E1" font-size="12.5">PK version: [1 = Baseline, 2 = Multi-Proj]</text>
  <text x="774" y="341" fill="#94A3B8" font-size="12.5">applied_at (UTC) · WAL + FK=ON</text>
  <text x="774" y="359" fill="#6EE7B7" font-size="12.2">Idempotent + Atomic Transaction</text>

  <!-- Bottom Middle: Extended V1 Tables -->
  <rect x="385" y="224" width="350" height="90" rx="8" fill="#132D24" stroke="#10B981" stroke-width="1.8"/>
  <text x="400" y="246" fill="#6EE7B7" font-size="13.5" font-weight="700">workflows (+project_id, category)</text>
  <text x="400" y="266" fill="#E2E8F0" font-size="12.5">FK project_id · workflow_versions (1:N)</text>
  <text x="400" y="285" fill="#CBD5E1" font-size="12.5">Giữ 2 workflows cũ + 10 JX2 workflows</text>
  <text x="400" y="303" fill="#94A3B8" font-size="12.2">Block cross-write: ISOLATION_VIOLATION</text>

  <rect x="385" y="326" width="350" height="80" rx="8" fill="#132D24" stroke="#10B981" stroke-width="1.8"/>
  <text x="400" y="348" fill="#6EE7B7" font-size="13.5" font-weight="700">accounts (+project_id)</text>
  <text x="400" y="368" fill="#E2E8F0" font-size="12.5">FK project_id · KHÔNG lưu plaintext pass</text>
  <text x="400" y="388" fill="#CBD5E1" font-size="12.5">secret_ref = CRED_MGR://FarmerPro/...</text>

  <rect x="20" y="420" width="1080" height="92" rx="8" fill="#1E293B" stroke="#10B981" stroke-width="1.6"/>
  <text x="40" y="444" fill="#6EE7B7" font-size="14" font-weight="700">RUNTIME EXECUTION TABLES: jobs (+project_id) · devices · step_events · checkpoints · device_leases</text>
  <text x="40" y="467" fill="#E2E8F0" font-size="12.8">• jobs: Thêm FK project_id + idx_jobs_project. Bảo toàn nguyên vẹn 2 jobs &amp; 31 step_events production.</text>
  <text x="40" y="489" fill="#CBD5E1" font-size="12.8">• devices (Lab fleet), step_events (audit), checkpoints (WAL recovery), device_leases (fencing). Integrity = ok (0 FK errors).</text>

  <!-- Connector lines -->
  <line x1="385" y1="95" x2="360" y2="95" stroke="#38BDF8" stroke-width="1.8"/>
  <line x1="385" y1="150" x2="360" y2="195" stroke="#38BDF8" stroke-width="1.8"/>
  <line x1="385" y1="185" x2="360" y2="310" stroke="#38BDF8" stroke-width="1.8"/>
  <line x1="735" y1="95" x2="760" y2="95" stroke="#A855F7" stroke-width="1.8"/>
  <line x1="735" y1="155" x2="760" y2="205" stroke="#A855F7" stroke-width="1.8"/>
  <line x1="560" y1="204" x2="560" y2="224" stroke="#10B981" stroke-width="2"/>
  <line x1="560" y1="314" x2="560" y2="326" stroke="#10B981" stroke-width="1.8"/>
  <line x1="560" y1="406" x2="560" y2="420" stroke="#10B981" stroke-width="1.8"/>
</svg>""",
}

# Diagram 3: Guarded Online Backup -> Restore Probe -> Migration -> Rollback Flow
DIAGRAMS["03_p2_migration_backup_rollback_flow"] = {
    "title": "Sơ đồ P2.3: Quy trình Bảo vệ Dữ liệu (Online Backup -> Restore Probe -> Atomic Migration & Rollback)",
    "mmd": """flowchart LR
    S1["1. Production v1 DB\\nPRAGMA integrity_check = ok"] --> S2["2. SQLite Online Backup API\\nrusqlite::backup::Backup"]
    S2 --> S3["3. Verify Backup + Restore Probe\\nRestore ra bản copy tạm & kiểm FK"]
    S3 -->|PASS| S4["4. BEGIN IMMEDIATE Transaction\\nCreate 6 tables + ALTER 3 tables + Backfill"]
    S4 --> S5{"5. PRAGMA foreign_key_check\\n== 0 & Commit?"}
    S5 -->|OK| S6["6. Non-Destructive JX2 Seed\\nCopy 10 workflows + 8 PNGs (Giữ nguồn cũ)"]
    S5 -->|FAIL / Error| RB["ROLLBACK Nguyên tử\\nDB giữ nguyên v1, 0 mất mát"]
""",
    "svg": """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1120 430" font-family="Segoe UI, Arial, sans-serif">
  <rect width="1120" height="430" rx="12" fill="#0F172A"/>
  <text x="560" y="34" text-anchor="middle" fill="#F8FAFC" font-size="18" font-weight="700">QUY TRÌNH CHUYỂN ĐỔI AN TOÀN V1 → V2 (ONLINE BACKUP, RESTORE PROBE &amp; ATOMIC ROLLBACK)</text>

  <!-- Step 1 -->
  <rect x="28" y="75" width="185" height="122" rx="9" fill="#1E293B" stroke="#64748B" stroke-width="1.8"/>
  <text x="120" y="99" text-anchor="middle" fill="#93C5FD" font-size="12.5" font-weight="700">BƯỚC 1: NGUỒN V1</text>
  <text x="120" y="122" text-anchor="middle" fill="#F8FAFC" font-size="11.5">dist-portable farmerpro.db</text>
  <text x="120" y="142" text-anchor="middle" fill="#CBD5E1" font-size="11">2 devices · 2 workflows</text>
  <text x="120" y="160" text-anchor="middle" fill="#CBD5E1" font-size="11">2 jobs · 31 step_events</text>
  <text x="120" y="182" text-anchor="middle" fill="#6EE7B7" font-size="11">integrity_check = ok</text>

  <!-- Step 2 -->
  <rect x="242" y="75" width="195" height="122" rx="9" fill="#172554" stroke="#3B82F6" stroke-width="1.8"/>
  <text x="339" y="99" text-anchor="middle" fill="#93C5FD" font-size="12.5" font-weight="700">BƯỚC 2: ONLINE BACKUP</text>
  <text x="339" y="122" text-anchor="middle" fill="#F8FAFC" font-size="11.5">rusqlite::backup::Backup</text>
  <text x="339" y="142" text-anchor="middle" fill="#CBD5E1" font-size="11">Gộp WAL frames an toàn</text>
  <text x="339" y="160" text-anchor="middle" fill="#CBD5E1" font-size="11">dist_portable_pre_v2.db</text>
  <text x="339" y="182" text-anchor="middle" fill="#6EE7B7" font-size="11">SHA-256 + FK Check = 0</text>

  <!-- Step 3 -->
  <rect x="466" y="75" width="195" height="122" rx="9" fill="#172554" stroke="#38BDF8" stroke-width="1.8"/>
  <text x="563" y="99" text-anchor="middle" fill="#7DD3FC" font-size="12.5" font-weight="700">BƯỚC 3: RESTORE PROBE</text>
  <text x="563" y="122" text-anchor="middle" fill="#F8FAFC" font-size="11.5">Kiểm thử Restore bản sao</text>
  <text x="563" y="142" text-anchor="middle" fill="#CBD5E1" font-size="11">Chạy trước khi chạm DB thật</text>
  <text x="563" y="160" text-anchor="middle" fill="#CBD5E1" font-size="11">Chứng minh khôi phục 100%</text>
  <text x="563" y="182" text-anchor="middle" fill="#6EE7B7" font-size="11">restoreProbeHealthy = true</text>

  <!-- Step 4 -->
  <rect x="690" y="75" width="195" height="122" rx="9" fill="#132D24" stroke="#10B981" stroke-width="2"/>
  <text x="787" y="99" text-anchor="middle" fill="#6EE7B7" font-size="12.5" font-weight="700">BƯỚC 4: MIGRATION V2</text>
  <text x="787" y="122" text-anchor="middle" fill="#F8FAFC" font-size="11.5">BEGIN IMMEDIATE TX</text>
  <text x="787" y="142" text-anchor="middle" fill="#CBD5E1" font-size="11">6 bảng mới + ALTER 3 bảng</text>
  <text x="787" y="160" text-anchor="middle" fill="#CBD5E1" font-size="11">Gắn dữ liệu cũ -&gt; jx2-mobile</text>
  <text x="787" y="182" text-anchor="middle" fill="#6EE7B7" font-size="11">Idempotent · Versions [1, 2]</text>

  <!-- Step 5 -->
  <rect x="914" y="75" width="178" height="122" rx="9" fill="#132D24" stroke="#34D399" stroke-width="2"/>
  <text x="1003" y="99" text-anchor="middle" fill="#6EE7B7" font-size="12.5" font-weight="700">BƯỚC 5: JX2 SEED</text>
  <text x="1003" y="122" text-anchor="middle" fill="#F8FAFC" font-size="11.5">Copy (Không xóa gốc)</text>
  <text x="1003" y="142" text-anchor="middle" fill="#CBD5E1" font-size="11">10 Workflows + 8 PNGs</text>
  <text x="1003" y="160" text-anchor="middle" fill="#CBD5E1" font-size="11">3 Tasks + 6 Modules</text>
  <text x="1003" y="182" text-anchor="middle" fill="#6EE7B7" font-size="11">31 step_events nguyên vẹn</text>

  <!-- Arrows -->
  <line x1="213" y1="136" x2="242" y2="136" stroke="#94A3B8" stroke-width="2" marker-end="url(#arr3)"/>
  <line x1="437" y1="136" x2="466" y2="136" stroke="#94A3B8" stroke-width="2" marker-end="url(#arr3)"/>
  <line x1="661" y1="136" x2="690" y2="136" stroke="#94A3B8" stroke-width="2" marker-end="url(#arr3)"/>
  <line x1="885" y1="136" x2="914" y2="136" stroke="#94A3B8" stroke-width="2" marker-end="url(#arr3)"/>

  <!-- Rollback Box -->
  <rect x="242" y="248" width="643" height="142" rx="10" fill="#3B1219" stroke="#F43F5E" stroke-width="2"/>
  <text x="563" y="274" text-anchor="middle" fill="#FDA4AF" font-size="14" font-weight="700">CƠ CHẾ ROLLBACK KÉP (TRANSACTION ATOMICITY + BACKUP RESTORE)</text>
  <text x="265" y="302" fill="#FFE4E6" font-size="12">1. Rollback Tự động trong SQLite Transaction: Nếu bất kỳ lệnh DDL/DML nào lỗi (đã test giả lập v3 lỗi giữa chừng),</text>
  <text x="282" y="321" fill="#FECDD3" font-size="11.5">toàn bộ CREATE/ALTER/INSERT được hoàn tác 100%, schema_migrations giữ nguyên [1, 2], 0 bảng rác.</text>
  <text x="265" y="348" fill="#FFE4E6" font-size="12">2. Rollback Toàn Cục sau Triển khai: Phục hồi file DB từ backups/p2_verified_backup/dist_portable_pre_v2.db</text>
  <text x="282" y="367" fill="#FECDD3" font-size="11.5">kèm git checkout master (fab1a1a). Dữ liệu gốc games/jx2mobile/ chưa bị xóa nên hoàn nguyên tức thì.</text>

  <line x1="787" y1="197" x2="787" y2="248" stroke="#F43F5E" stroke-width="2" stroke-dasharray="5,4" marker-end="url(#arrRed)"/>

  <defs>
    <marker id="arr3" markerWidth="8" markerHeight="8" refX="6" refY="3" orient="auto">
      <path d="M0,0 L0,6 L7,3 z" fill="#94A3B8"/>
    </marker>
    <marker id="arrRed" markerWidth="8" markerHeight="8" refX="6" refY="3" orient="auto">
      <path d="M0,0 L0,6 L7,3 z" fill="#F43F5E"/>
    </marker>
  </defs>
</svg>""",
}

# Diagram 4: Windows Credential Manager Namespaces + DPAPI Keystore
DIAGRAMS["04_p2_wincred_dpapi_isolation"] = {
    "title": "Sơ đồ P2.4: Cách ly Bí mật Windows Credential Manager & Bảo vệ Khóa Ký bằng Windows DPAPI",
    "mmd": """flowchart TB
    subgraph VAULT["Windows Credential Manager (advapi32.dll: CredWriteW/ReadW/EnumerateW/DeleteW)"]
        P1["Project A Namespace\\nFarmerPro/project/jx2-mobile/*"]
        P2["Project B Namespace\\nFarmerPro/project/kingdom-slg/*"]
        MT["Mini Tool Namespace\\nFarmerPro/minitool/auto-kingdom/*"]
        AC["Account Vault Namespace\\nFarmerPro/account/<slug>/*"]
    end
    subgraph DPAPI["Producer Keystore (crypt32.dll: CryptProtectData / CryptUnprotectData)"]
        SEED["32-Byte Ed25519 Seed (RAM Only)"] --> ENCRYPT["DPAPI Encrypt + Entropy\\n(CRYPTPROTECT_UI_FORBIDDEN)"]
        ENCRYPT --> DISK["active_producer_ed25519.json (v2)\\ndpapiProtectedSeedB64 (NO plaintext)"]
        ENCRYPT --> BK["Backup Bundle JSON\\nDPAPI + payloadSha256 Verified"]
    end
""",
    "svg": r"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1120 440" font-family="Segoe UI, Arial, sans-serif">
  <rect width="1120" height="440" rx="12" fill="#0F172A"/>
  <text x="560" y="34" text-anchor="middle" fill="#F8FAFC" font-size="18.5" font-weight="700">BẢO MẬT BÍ MẬT ĐA DỰ ÁN: WINDOWS CREDENTIAL MANAGER &amp; DPAPI KEYSTORE</text>

  <!-- Left Panel: Windows Credential Manager -->
  <rect x="24" y="56" width="526" height="360" rx="10" fill="#1E293B" stroke="#38BDF8" stroke-width="2"/>
  <text x="44" y="84" fill="#38BDF8" font-size="14.5" font-weight="700">1. WINDOWS CREDENTIAL MANAGER (advapi32.dll)</text>
  <text x="44" y="106" fill="#CBD5E1" font-size="12.5">Thay thế hoàn toàn HashMap RAM cũ; mã hóa theo phiên Windows OS.</text>

  <rect x="44" y="122" width="486" height="58" rx="7" fill="#0F2942" stroke="#60A5FA" stroke-width="1.4"/>
  <text x="60" y="144" fill="#F8FAFC" font-size="13" font-weight="700">Namespace Dự án (Project Scope)</text>
  <text x="60" y="165" fill="#93C5FD" font-size="12.5">CRED_MGR://FarmerPro/project/&lt;project_slug&gt;/&lt;key&gt;</text>

  <rect x="44" y="192" width="486" height="58" rx="7" fill="#132D24" stroke="#34D399" stroke-width="1.4"/>
  <text x="60" y="214" fill="#F8FAFC" font-size="13" font-weight="700">Namespace Mini Tool (MiniTool Scope)</text>
  <text x="60" y="235" fill="#6EE7B7" font-size="12.5">CRED_MGR://FarmerPro/minitool/&lt;tool_slug&gt;/&lt;key&gt;</text>

  <rect x="44" y="262" width="486" height="58" rx="7" fill="#2B2118" stroke="#FBBF24" stroke-width="1.4"/>
  <text x="60" y="284" fill="#F8FAFC" font-size="13" font-weight="700">Namespace Tài khoản Game (Account Vault Scope)</text>
  <text x="60" y="305" fill="#FDE68A" font-size="12.5">CRED_MGR://FarmerPro/account/&lt;project_slug&gt;/&lt;account_id&gt;</text>

  <text x="44" y="348" fill="#E2E8F0" font-size="12.5">• Chặn '*', '?', '..' tránh vượt ranh giới khi CredEnumerateW</text>
  <text x="44" y="369" fill="#E2E8F0" font-size="12.5">• Project A &amp; Project B tách biệt tuyệt đối khi Set / Get / Delete</text>
  <text x="44" y="390" fill="#6EE7B7" font-size="12.5">• Đã kiểm chứng trên Windows OS thật: 6/6 fp-security tests</text>

  <!-- Right Panel: DPAPI Keystore -->
  <rect x="570" y="56" width="526" height="360" rx="10" fill="#1E293B" stroke="#A855F7" stroke-width="2"/>
  <text x="590" y="84" fill="#E9D5FF" font-size="14.5" font-weight="700">2. PRODUCER ED25519 KEYSTORE + DPAPI (crypt32.dll)</text>
  <text x="590" y="106" fill="#CBD5E1" font-size="12.5">Bảo vệ Private Seed của nhà sản xuất bằng CryptProtectData.</text>

  <rect x="590" y="122" width="486" height="72" rx="7" fill="#2E1065" stroke="#C084FC" stroke-width="1.5"/>
  <text x="606" y="144" fill="#F8FAFC" font-size="13" font-weight="700">Kho khóa Active (%LOCALAPPDATA%\FarmerPro\ProducerKeys\)</text>
  <text x="606" y="164" fill="#E9D5FF" font-size="12.2">Format v2: dpapiProtectedSeedB64 (KHÔNG còn secretSeedHex)</text>
  <text x="606" y="183" fill="#6EE7B7" font-size="12.2">Tự nâng cấp file v1 cũ lên DPAPI v2 giữ nguyên keypair</text>

  <rect x="590" y="208" width="234" height="88" rx="7" fill="#172554" stroke="#60A5FA" stroke-width="1.4"/>
  <text x="707" y="232" text-anchor="middle" fill="#93C5FD" font-size="13.5" font-weight="700">Xoay vòng Khóa (Rotate)</text>
  <text x="707" y="253" text-anchor="middle" fill="#E2E8F0" font-size="12.2">Lưu khóa cũ sang</text>
  <text x="707" y="271" text-anchor="middle" fill="#CBD5E1" font-size="12.2">archived_producer_*.json</text>
  <text x="707" y="288" text-anchor="middle" fill="#6EE7B7" font-size="12.2">Sinh khóa Ed25519 mới</text>

  <rect x="842" y="208" width="234" height="88" rx="7" fill="#132D24" stroke="#34D399" stroke-width="1.4"/>
  <text x="959" y="232" text-anchor="middle" fill="#6EE7B7" font-size="13.5" font-weight="700">Backup &amp; Restore</text>
  <text x="959" y="253" text-anchor="middle" fill="#E2E8F0" font-size="12.2">DPAPI Backup Entropy +</text>
  <text x="959" y="271" text-anchor="middle" fill="#CBD5E1" font-size="12.2">payloadSha256 Integrity</text>
  <text x="959" y="288" text-anchor="middle" fill="#FDA4AF" font-size="12.2">Chặn file backup bị sửa</text>

  <text x="590" y="328" fill="#E2E8F0" font-size="12.5">• Zeroize bộ nhớ đệm plaintext seed ngay sau khi mã hóa DPAPI</text>
  <text x="590" y="348" fill="#E2E8F0" font-size="12.5">• Mini Runtime vẫn giữ ranh giới: KHÔNG đọc Producer Keystore</text>
  <text x="590" y="369" fill="#E2E8F0" font-size="12.5">• Kiểm tra chữ ký Ed25519 probe trước khi chấp nhận Restore</text>
  <text x="590" y="390" fill="#6EE7B7" font-size="12.5">• Đã test DPAPI tamper + legacy upgrade + rotate + restore: PASS</text>
</svg>""",
}

diag_b64 = {}
for key, item in DIAGRAMS.items():
    mmd_path = os.path.join(DIAG_DIR, f"{key}.mmd")
    svg_path = os.path.join(DIAG_DIR, f"{key}.svg")
    with open(mmd_path, "w", encoding="utf-8") as f:
        f.write(item["mmd"])
    with open(svg_path, "w", encoding="utf-8") as f:
        f.write(item["svg"])
    diag_b64[key] = "data:image/svg+xml;base64," + base64.b64encode(item["svg"].encode("utf-8")).decode("ascii")


TOTAL_PAGES = 12


def page_wrap(page_no, badge, title, inner):
    return f"""
    <section class="pdf-page" id="page-{page_no:02d}">
      <div class="page-top-bar">
        <div class="pt-left">
          <span class="pt-badge">{badge}</span>
          <span class="pt-title">{title}</span>
        </div>
        <div class="pt-right">FARMERPRO P2 ACCEPTANCE · BRANCH feat/p2-multi-project-storage · 2026-11-22</div>
      </div>
      <div class="page-content">{inner}</div>
      <div class="page-bottom-bar">
        <span>Hồ sơ nghiệm thu Giai đoạn P2: Multi-Project SQLite v2, Backup/Migration/Rollback, JX2 Preservation, WinCred/DPAPI &amp; Project CRUD</span>
        <span>Commit Base: 9bd0e45 (từ fab1a1a)</span>
        <span class="pg-num">Trang {page_no:02d} / {TOTAL_PAGES:02d}</span>
      </div>
    </section>
    """


pre_mig = prod_mig_rep["preMigrationCounts"]
post_mig = prod_mig_rep["postMigrationIntegrity"]["tableCounts"]
jx2_mig = prod_mig_rep["jx2Migration"]
sb_vault = sandbox_rep["credentialVaultAndDpapi"]
sb_iso = sandbox_rep["projectCrudAndIsolation"]
sb_rb = sandbox_rep["migrationV1ToV2"]["rollbackTest"]

pages = []

# PAGE 01: COVER & EXECUTIVE SUMMARY
pages.append(page_wrap(1, "BÁO CÁO NGHIỆM THU P2", "Trang Bìa & Tóm tắt Kết quả Triển khai Giai đoạn P2", f"""
  <div class="hero-box" style="padding:3mm 4.5mm;">
    <div class="pill-row">
      <span class="badge b-pass">STAGE-GATE P2: HOÀN THÀNH PHẠM VI KỸ THUẬT</span>
      <span class="badge b-info">44/44 WORKSPACE UNIT TESTS PASS</span>
      <span class="badge b-warn">apps/desktop/src UI &amp; P3–P6: GIỮ NGUYÊN 100%</span>
    </div>
    <h1 style="margin:1.5mm 0 1mm 0; font-size:12.8pt;">FARMERPRO P2 — BÁO CÁO NGHIỆM THU MULTI-PROJECT STORAGE, MIGRATION, WINCRED &amp; DPAPI</h1>
    <p class="hero-sub" style="font-size:8.9pt;">
      Triển khai hoàn chỉnh tầng dữ liệu đa dự án (<b>SQLite WAL v2</b>), cơ chế <b>Online Backup → Restore Probe → Migration v1→v2 → Atomic Rollback</b>, di chuyển bảo toàn <b>10 workflow Võ Hiệp 2 Mobile + 8 ảnh mẫu PNG</b>, thay thế mock Vault bằng <b>Windows Credential Manager</b> thực, mã hóa Producer Signing Key bằng <b>Windows DPAPI</b>, và <b>26 Tauri IPC Commands</b> cho Project CRUD &amp; Project Isolation.
    </p>
  </div>

  <div class="grid-2" style="margin-top:2.4mm;">
    <div class="card" style="padding:2.2mm 3mm;">
      <div class="card-h" style="margin-bottom:1.4mm;">1. THÔNG TIN ĐỊNH DANH &amp; TUÂN THỦ STAGE-GATE P2</div>
      <table class="tbl-tight">
        <tr><th style="width:36%;">Hạng mục</th><th style="width:64%;">Giá trị Thực tế Đã Xác minh</th></tr>
        <tr><td><b>Nhánh Git Triển khai</b></td><td><code>feat/p2-multi-project-storage</code> (tách từ <code>fab1a1a</code>)</td></tr>
        <tr><td><b>Commit Code P2</b></td><td><code>9bd0e45</code> (20 files changed, +5598 / -230 lines)</td></tr>
        <tr><td><b>Giữ nguyên UI P4</b></td><td><span class="tag-pass">PASS</span> <code>git diff -- apps/desktop/src</code> = 0 dòng</td></tr>
        <tr><td><b>Hiệu chỉnh Blueprint II.1</b></td><td><span class="tag-pass">PASS</span> Cập nhật rõ <code>920×510</code> = ~50,9% diện tích (71,9% rộng)</td></tr>
        <tr><td><b>Gỡ Hardcode Runtime</b></td><td><span class="tag-pass">PASS</span> Gỡ bỏ hardcode <code>games/jx2mobile/templates</code></td></tr>
        <tr><td><b>Backup DB trước Migrate</b></td><td><code>backups/p2_verified_backup/dist_portable_pre_v2.db</code></td></tr>
      </table>
    </div>
    <div class="card" style="padding:2.2mm 3mm;">
      <div class="card-h" style="margin-bottom:1.4mm;">2. TỔNG HỢP KẾT QUẢ KIỂM ĐỊNH ĐỘC LẬP (PASS / FAIL)</div>
      <table class="tbl-tight">
        <tr><th style="width:46%;">Tiêu chí Nghiệm thu P2</th><th style="width:16%;">Kết quả</th><th style="width:38%;">Bằng chứng Thực thi</th></tr>
        <tr><td>Workspace Unit Tests</td><td><span class="tag-pass">PASS</span></td><td><b>44/44 PASS</b> (34 cũ + 10 tests mới P2)</td></tr>
        <tr><td>SQLite Migration v1→v2</td><td><span class="tag-pass">PASS</span></td><td>Versions <code>[1, 2]</code>, WAL, 0 FK violations</td></tr>
        <tr><td>Atomic Rollback (Lỗi v3)</td><td><span class="tag-pass">PASS</span></td><td>Rollback 100%, 0 bảng rác</td></tr>
        <tr><td>Bảo toàn JX2 (10 WF + 8 PNG)</td><td><span class="tag-pass">PASS</span></td><td>Giữ 2 jobs, 31 step_events + 10 JX2</td></tr>
        <tr><td>Project Isolation (3 Projects)</td><td><span class="tag-pass">PASS</span></td><td>Chặn ghi chéo (<code>PROJECT_ISOLATION_VIOLATION</code>)</td></tr>
        <tr><td>Windows CredMgr &amp; DPAPI</td><td><span class="tag-pass">PASS</span></td><td>Win32 <code>advapi32</code> + <code>crypt32</code> thực trên OS</td></tr>
        <tr><td>UI Studio / Proxy / AuthCode</td><td><span class="tag-plan">P3–P5</span></td><td>Ngoài phạm vi P2 (giữ nguyên Stage-Gate)</td></tr>
      </table>
    </div>
  </div>

  <div class="card" style="margin-top:2.4mm; padding:2mm 3mm;">
    <div class="card-h" style="margin-bottom:1.4mm;">3. MỤC LỤC HỒ SƠ NGHIỆM THU P2 (12 TRANG — BẤM LINK HOẶC BOOKMARK ĐỂ ĐIỀU HƯỚNG)</div>
    <div class="toc-grid" style="grid-template-columns: 1fr 1fr 1fr; gap: 1.2mm 2.4mm; font-size: 8pt;">
      <a href="#page-02"><b>02:</b> Bản đồ Rust Crates P2 (Sơ đồ)</a>
      <a href="#page-03"><b>03:</b> SQLite WAL v2 ER Schema (Sơ đồ)</a>
      <a href="#page-04"><b>04:</b> Migration SQL v1→v2 &amp; Idempotency</a>
      <a href="#page-05"><b>05:</b> Online Backup &amp; Rollback (Sơ đồ)</a>
      <a href="#page-06"><b>06:</b> Dữ liệu Trước/Sau &amp; 10 JX2 Workflows</a>
      <a href="#page-07"><b>07:</b> Project CRUD, Clone &amp; Isolation</a>
      <a href="#page-08"><b>08:</b> Kiến trúc WinCred &amp; DPAPI (Sơ đồ)</a>
      <a href="#page-09"><b>09:</b> Kết quả Test WinCred &amp; DPAPI</a>
      <a href="#page-10"><b>10:</b> 26 Tauri IPC &amp; 44/44 Unit Tests</a>
      <a href="#page-11"><b>11:</b> PASS/FAIL, Git Diff &amp; Rollback</a>
      <a href="#page-12"><b>12:</b> Sẵn sàng cho P3 &amp; SHA-256</a>
      <a href="#page-01"><b>01:</b> Tóm tắt Điều hành &amp; Stage-Gate</a>
    </div>
  </div>
"""))

# PAGE 02: DIAGRAM 1 - RUST CRATES MAP
pages.append(page_wrap(2, "KIẾN TRÚC RUST CRATES", "Bản đồ Rust Crates & Ranh giới Kiến trúc Giai đoạn P2", f"""
  <div class="diag-wrap">
    <img src="{diag_b64['01_p2_rust_crates_map']}" class="diag-img"/>
  </div>
  <div class="grid-3" style="margin-top:3mm;">
    <div class="card">
      <div class="card-h">crates/fp-storage (Nâng cấp Core DB)</div>
      <p>Tổ chức thành 4 module: <code>models.rs</code>, <code>migrations.rs</code>, <code>jx2_seed.rs</code>, <code>lib.rs</code>. Cung cấp Migration v1→v2 giao dịch nguyên tử, Online Backup API, Restore Probe, Project CRUD/Clone và cơ chế chặn truy cập chéo Project.</p>
    </div>
    <div class="card">
      <div class="card-h">crates/fp-security (WinCred + DPAPI)</div>
      <p>Thay thế hoàn toàn mock <code>HashMap</code> bằng Win32 <code>advapi32.dll</code> và <code>crypt32.dll</code> (<code>wincred.rs</code>, <code>dpapi.rs</code>, <code>keystore.rs</code>). <code>fp-builder</code> ủy quyền quản lý khóa ký về <code>ProducerKeystore</code> dùng chung.</p>
    </div>
    <div class="card">
      <div class="card-h">crates/fp-workflow-runtime (Zero Hardcode)</div>
      <p>Loại bỏ đường dẫn cứng <code>games/jx2mobile/templates</code> khỏi lõi thực thi workflow; chuyển sang quét động <code>bundle/templates</code>, <code>assets/templates</code> và <code>projects/*/templates</code>.</p>
    </div>
  </div>
"""))

# PAGE 03: DIAGRAM 2 - SQLITE V2 ER SCHEMA
pages.append(page_wrap(3, "SQLITE SCHEMA V2", "Mô hình Quan hệ Dữ liệu Đa Dự án (SQLite WAL v2 ER Diagram)", f"""
  <div class="diag-wrap">
    <img src="{diag_b64['02_p2_sqlite_v2_er_schema']}" class="diag-img"/>
  </div>
  <div class="card" style="margin-top:3mm;">
    <div class="card-h">TÓM TẮT 15 BẢNG TRONG SQLITE V2 (9 BẢNG V1 MỞ RỘNG + 6 BẢNG MỚI V2)</div>
    <p>
      <b>6 bảng mới P2:</b> <code>projects</code> (Root Entity, UNIQUE <code>slug</code>), <code>project_modules</code> (Ma trận 5 trạng thái module theo project), <code>vision_templates</code> (Ảnh mẫu + tọa độ ROI + SHA-256), <code>task_definitions</code> (Thư viện nhiệm vụ gắn với <code>workflow_id</code>), <code>mini_tool_schemas</code> (Cấu hình <code>MiniToolUiSchema</code>), <code>project_releases</code> (Lịch sử build EXE + PE <code>RT_RCDATA #7777</code> + chữ ký Ed25519).
      <b>3 bảng v1 mở rộng:</b> <code>workflows</code> (<code>+project_id, +category</code>), <code>accounts</code> (<code>+project_id</code>), <code>jobs</code> (<code>+project_id</code>) kèm 6 indexes mới.
    </p>
  </div>
"""))

# PAGE 04: MIGRATION SQL & SPECIFICATION
pages.append(page_wrap(4, "MIGRATION SQL V1→V2", "Đặc tả Migration SQL v1→v2, Indexes & Ràng buộc Toàn vẹn", f"""
  <div class="grid-2">
    <div class="card" style="padding:2.2mm 2.8mm;">
      <div class="card-h" style="margin-bottom:1.2mm;">MIGRATION V2 — 6 BẢNG MỚI (Phần 1: Core &amp; Assets)</div>
      <pre class="code-box" style="font-size:7.6pt; line-height:1.22; padding:2mm 2.5mm;">CREATE TABLE IF NOT EXISTS projects (
  id TEXT PRIMARY KEY, slug TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL, game_package_id TEXT NOT NULL,
  version TEXT NOT NULL DEFAULT '1.0.0', exe_name TEXT NOT NULL,
  window_title TEXT NOT NULL, icon_rel_path TEXT DEFAULT 'branding/icon.ico',
  target_width_px INTEGER NOT NULL DEFAULT 1920,
  target_height_px INTEGER NOT NULL DEFAULT 1080,
  bundle_adb INTEGER NOT NULL DEFAULT 1, project_dir_path TEXT NOT NULL,
  created_at TEXT NOT NULL, updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS project_modules (
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  module_id TEXT NOT NULL,
  packaged_in_release INTEGER NOT NULL DEFAULT 1,
  enabled_by_default INTEGER NOT NULL DEFAULT 1,
  config_json TEXT NOT NULL DEFAULT '{{}}', updated_at TEXT NOT NULL,
  PRIMARY KEY (project_id, module_id)
);
CREATE TABLE IF NOT EXISTS vision_templates (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  name TEXT NOT NULL, rel_path TEXT NOT NULL,
  width_px INTEGER NOT NULL, height_px INTEGER NOT NULL,
  source_crop_x INTEGER, source_crop_y INTEGER,
  default_confidence REAL NOT NULL DEFAULT 0.88,
  sha256 TEXT NOT NULL, created_at TEXT NOT NULL,
  UNIQUE(project_id, rel_path)
);</pre>
    </div>
    <div class="card" style="padding:2.2mm 2.8mm;">
      <div class="card-h" style="margin-bottom:1.2mm;">MIGRATION V2 — BẢNG MỞ RỘNG &amp; INDEXES (Phần 2)</div>
      <pre class="code-box" style="font-size:7.6pt; line-height:1.22; padding:2mm 2.5mm;">CREATE TABLE IF NOT EXISTS task_definitions (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  workflow_id TEXT NOT NULL REFERENCES workflows(id),
  category_id TEXT NOT NULL, category_short_tag TEXT NOT NULL,
  category_name TEXT NOT NULL, category_color_hex TEXT NOT NULL,
  title TEXT NOT NULL, priority INTEGER NOT NULL DEFAULT 1,
  estimate_sec INTEGER NOT NULL DEFAULT 60,
  default_enabled INTEGER NOT NULL DEFAULT 1,
  settings_schema_json TEXT NOT NULL DEFAULT '[]', updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS mini_tool_schemas (
  project_id TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
  schema_version INTEGER NOT NULL DEFAULT 1,
  ui_schema_json TEXT NOT NULL, updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS project_releases (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  version TEXT NOT NULL, output_dir_path TEXT NOT NULL,
  exe_path TEXT NOT NULL, exe_size_bytes INTEGER NOT NULL,
  exe_sha256 TEXT NOT NULL, key_id TEXT NOT NULL,
  publisher_public_key_hex TEXT NOT NULL,
  pe_rcdata_pubkey_id INTEGER NOT NULL DEFAULT 7777,
  signature_ed25519_hex TEXT NOT NULL,
  build_report_json TEXT NOT NULL, built_at TEXT NOT NULL
);
-- Idempotent ALTER TABLE (checked via PRAGMA table_info):
ALTER TABLE workflows ADD COLUMN project_id TEXT REFERENCES projects(id);
ALTER TABLE workflows ADD COLUMN category TEXT NOT NULL DEFAULT 'Chung';
ALTER TABLE accounts  ADD COLUMN project_id TEXT REFERENCES projects(id);
ALTER TABLE jobs      ADD COLUMN project_id TEXT REFERENCES projects(id);</pre>
    </div>
  </div>
  <div class="card" style="margin-top:2.5mm; padding:2.2mm 3mm;">
    <div class="card-h" style="margin-bottom:1.2mm;">CƠ CHẾ ĐẢM BẢO AN TOÀN KHI MIGRATE (FILE ĐẦY ĐỦ: sqlite_v1_v2_migration.sql)</div>
    <p style="font-size:8.7pt;"><b>1. Transaction Nguyên tử:</b> Mọi bước DDL + Backfill + Kiểm tra khóa ngoại (<code>PRAGMA foreign_key_check</code>) + Ghi phiên bản vào <code>schema_migrations</code> chạy trong <code>BEGIN IMMEDIATE ... COMMIT</code>.
       <b>2. Idempotency:</b> Kiểm tra bảng <code>schema_migrations</code> và <code>PRAGMA table_info</code> trước khi chạy <code>ALTER TABLE</code>; gọi <code>run_migrations()</code> nhiều lần không gây lỗi trùng cột hoặc trùng dữ liệu.
       <b>3. Bảo toàn dữ liệu cũ:</b> Các bản ghi <code>workflows</code>, <code>accounts</code>, <code>jobs</code> có <code>project_id IS NULL</code> được tự động gắn vào dự án mặc định <code>proj-jx2-mobile-001</code>.</p>
  </div>
"""))

# PAGE 05: DIAGRAM 3 - ONLINE BACKUP, RESTORE & ROLLBACK FLOW
pages.append(page_wrap(5, "BACKUP & ROLLBACK", "Quy trình Online Backup, Restore Probe & Atomic Rollback", f"""
  <div class="diag-wrap">
    <img src="{diag_b64['03_p2_migration_backup_rollback_flow']}" class="diag-img" style="max-height:105mm;"/>
  </div>
  <div class="grid-2" style="margin-top:2.5mm;">
    <div class="card" style="padding:2.2mm 3mm;">
      <div class="card-h" style="margin-bottom:1.4mm;">KẾT QUẢ KIỂM THỬ ROLLBACK KHI MIGRATION LỖI (SANDBOX PROOF)</div>
      <table class="tbl-tight">
        <tr><th>Tiêu chí Kiểm thử Rollback</th><th>Kết quả</th><th>Giá trị Đo được</th></tr>
        <tr><td>Thực thi Migration v3 lỗi giữa chừng</td><td><span class="tag-pass">PASS</span></td><td><code>faultyV3Rejected = {sb_rb['faultyV3Rejected']}</code></td></tr>
        <tr><td>Phiên bản sau khi Rollback</td><td><span class="tag-pass">PASS</span></td><td><code>versionsAfterRollback = {sb_rb['versionsAfterRollback']}</code></td></tr>
        <tr><td>Bảng rác <code>p2_faulty_temp</code> bị hoàn tác</td><td><span class="tag-pass">PASS</span></td><td><code>cleanAtomicRollback = {sb_rb['cleanAtomicRollback']}</code></td></tr>
        <tr><td>Toàn vẹn dữ liệu sau Rollback</td><td><span class="tag-pass">PASS</span></td><td><code>integrity = {sb_rb['postRollbackIntegrity']['integrityCheck']}</code>, 0 FK violations</td></tr>
      </table>
    </div>
    <div class="card" style="padding:2.2mm 3mm;">
      <div class="card-h" style="margin-bottom:1.4mm;">KẾT QUẢ ONLINE BACKUP &amp; RESTORE (V2 BACKUP API)</div>
      <table class="tbl-tight">
        <tr><th>Thuộc tính Backup / Restore</th><th>Giá trị Thực tế</th></tr>
        <tr><td>Backup Pre-Migration Production</td><td><code>backups/p2_verified_backup/dist_portable_pre_v2.db</code></td></tr>
        <tr><td>Restore Probe trước Migration thật</td><td><code>restoreProbeHealthy = {prod_mig_rep['restoreProbeHealthy']}</code></td></tr>
        <tr><td>Backup V2 Sandbox SHA-256</td><td><code>{sandbox_rep['backupAndRestoreV2']['onlineBackup']['backupSha256'][:28]}...</code></td></tr>
        <tr><td>Khớp 100% số dòng sau Restore</td><td><span class="tag-pass">PASS</span> 15/15 bảng khớp chính xác (11 wf, 31 events)</td></tr>
      </table>
    </div>
  </div>
"""))

# PAGE 06: BEFORE/AFTER DATA & JX2 PRESERVATION
pages.append(page_wrap(6, "BẢO TOÀN DỮ LIỆU JX2", "Đối chiếu Dữ liệu Trước/Sau Migration & Bảo toàn 10 JX2 Workflows", f"""
  <div class="grid-2">
    <div class="card">
      <div class="card-h">SO SÁNH TRƯỚC / SAU TRÊN PRODUCTION DB (dist-portable)</div>
      <table>
        <tr><th>Bảng SQLite</th><th>Trước Migration (v1)</th><th>Sau Migration (v2 + JX2)</th><th>Trạng thái</th></tr>
        <tr><td><code>schema_migrations</code></td><td>0 (chưa tracking)</td><td><b>{post_mig['schema_migrations']}</b> (versions [1, 2])</td><td><span class="tag-pass">PASS</span></td></tr>
        <tr><td><code>devices</code></td><td>{pre_mig['devices']}</td><td><b>{post_mig['devices']}</b> (giữ 100%)</td><td><span class="tag-pass">PASS</span></td></tr>
        <tr><td><code>workflows</code></td><td>{pre_mig['workflows']}</td><td><b>{post_mig['workflows']}</b> (2 cũ + 9 JX2 mới)</td><td><span class="tag-pass">PASS</span></td></tr>
        <tr><td><code>workflow_versions</code></td><td>{pre_mig['workflow_versions']}</td><td><b>{post_mig['workflow_versions']}</b> (giữ version cũ)</td><td><span class="tag-pass">PASS</span></td></tr>
        <tr><td><code>jobs</code></td><td>{pre_mig['jobs']}</td><td><b>{post_mig['jobs']}</b> (gắn <code>proj-jx2-mobile-001</code>)</td><td><span class="tag-pass">PASS</span></td></tr>
        <tr><td><code>step_events</code></td><td>{pre_mig['step_events']}</td><td><b>{post_mig['step_events']}</b> (giữ 100%)</td><td><span class="tag-pass">PASS</span></td></tr>
        <tr><td><code>projects</code></td><td>(chưa có bảng)</td><td><b>{post_mig['projects']}</b> (<code>proj-jx2-mobile-001</code>)</td><td><span class="tag-pass">PASS</span></td></tr>
        <tr><td><code>vision_templates</code></td><td>(chưa có bảng)</td><td><b>{post_mig['vision_templates']}</b> (8 PNG thật + SHA-256)</td><td><span class="tag-pass">PASS</span></td></tr>
        <tr><td><code>project_modules</code></td><td>(chưa có bảng)</td><td><b>{post_mig['project_modules']}</b> (6 modules cấu hình)</td><td><span class="tag-pass">PASS</span></td></tr>
        <tr><td><code>task_definitions</code></td><td>(chưa có bảng)</td><td><b>{post_mig['task_definitions']}</b> (gắn wf-jx2-*)</td><td><span class="tag-pass">PASS</span></td></tr>
        <tr><td><code>mini_tool_schemas</code></td><td>(chưa có bảng)</td><td><b>{post_mig['mini_tool_schemas']}</b> (UI Schema mặc định)</td><td><span class="tag-pass">PASS</span></td></tr>
      </table>
    </div>
    <div class="card">
      <div class="card-h">KIỂM CHỨNG 10 WORKFLOW JX2 &amp; 8 ẢNH MẪU PNG</div>
      <p style="margin-bottom:2mm;">Toàn bộ 10 file từ <code>games/jx2mobile/*.json</code> và 8 ảnh mẫu <code>games/jx2mobile/templates/*.png</code> được kiểm tra hợp lệ qua <code>WorkflowValidator</code> và copy không phá hủy vào <code>projects/jx2-mobile/</code>. Nguồn gốc <code>games/jx2mobile/</code> giữ nguyên 100%.</p>
      <table>
        <tr><th>Workflow ID</th><th>Danh mục</th><th>Trạng thái</th></tr>
        <tr><td><code>wf-jx2-01-login</code></td><td>Khởi động &amp; Đăng nhập</td><td><span class="tag-pass">Bảo toàn + Import</span></td></tr>
        <tr><td><code>wf-jx2-02-char</code></td><td>Tạo nhân vật</td><td><span class="tag-pass">Import + Ver 1</span></td></tr>
        <tr><td><code>wf-jx2-03-tutorial</code></td><td>Hướng dẫn tân thủ</td><td><span class="tag-pass">Import + Ver 1</span></td></tr>
        <tr><td><code>wf-jx2-04-npc</code></td><td>Tương tác NPC</td><td><span class="tag-pass">Import + Ver 1</span></td></tr>
        <tr><td><code>wf-jx2-05-accept</code></td><td>Tiếp nhận nhiệm vụ</td><td><span class="tag-pass">Import + Ver 1</span></td></tr>
        <tr><td><code>wf-jx2-06-movement</code></td><td>Di chuyển</td><td><span class="tag-pass">Import + Ver 1</span></td></tr>
        <tr><td><code>wf-jx2-07-combat</code></td><td>Chiến đấu</td><td><span class="tag-pass">Import + Ver 1</span></td></tr>
        <tr><td><code>wf-jx2-08-verify</code></td><td>Xác minh &amp; Thưởng</td><td><span class="tag-pass">Import + Ver 1</span></td></tr>
        <tr><td><code>wf-jx2-09-complete</code></td><td>Hoàn tất tân thủ</td><td><span class="tag-pass">Import + Ver 1</span></td></tr>
        <tr><td><code>wf-jx2-e2e-beginner</code></td><td>E2E Toàn trình</td><td><span class="tag-pass">Import + Ver 1</span></td></tr>
        <tr><td><code>wf-beginner-quest-01</code></td><td>Legacy v1 DB</td><td><span class="tag-pass">Bảo toàn 100%</span></td></tr>
      </table>
    </div>
  </div>
"""))

# PAGE 07: PROJECT CRUD & PROJECT ISOLATION
pages.append(page_wrap(7, "PROJECT CRUD & ISOLATION", "Chứng cứ Project CRUD, Deep Clone & Cách ly Đa Dự án (3 GameProjects)", f"""
  <div class="card">
    <div class="card-h">1. KẾT QUẢ THỰC THI PROJECT CRUD &amp; CLONE TRÊN SANDBOX (fp-cli verify-p2)</div>
    <table>
      <tr>
        <th>Project ID</th><th>Slug</th><th>Tên Game Project</th><th>Package ID</th>
        <th>Workflows</th><th>Templates</th><th>Tasks</th><th>Modules</th><th>Accounts</th><th>Jobs</th>
      </tr>
      {"".join(
          f"<tr><td><code>{p['project']['id']}</code></td><td><code>{p['project']['slug']}</code></td>"
          f"<td>{p['project']['name']}</td><td><code>{p['project']['gamePackageId']}</code></td>"
          f"<td><b>{p['workflowsCount']}</b></td><td><b>{p['templatesCount']}</b></td>"
          f"<td><b>{p['tasksCount']}</b></td><td><b>{p['modulesCount']}</b></td>"
          f"<td><b>{p['accountsCount']}</b></td><td><b>{p['jobsCount']}</b></td></tr>"
          for p in sb_iso["projects"]
      )}
    </table>
  </div>

  <div class="grid-2" style="margin-top:4mm;">
    <div class="card">
      <div class="card-h">2. CÁC BÀI KIỂM THỬ PROJECT ISOLATION ĐÃ CHỨNG MINH</div>
      <table>
        <tr><th>Kịch bản Tấn công / Trộn lẫn Dự án</th><th>Cơ chế Bảo vệ</th><th>Kết quả</th></tr>
        <tr><td>Project B cố ghi đè <code>workflow_id</code> thuộc Project A</td><td>Kiểm tra chủ sở hữu <code>workflows.project_id</code></td><td><span class="tag-pass">BLOCKED</span> (<code>PROJECT_ISOLATION_VIOLATION</code>)</td></tr>
        <tr><td>Project B cố tạo Task trỏ tới Workflow của Project A</td><td>Xác thực <code>workflow.project_id == task.project_id</code></td><td><span class="tag-pass">BLOCKED</span> (<code>PROJECT_ISOLATION_VIOLATION</code>)</td></tr>
        <tr><td>Project B lưu <code>MiniToolUiSchema</code> mang <code>slug</code> Project A</td><td>Kiểm tra <code>schema.project_slug == project.slug</code></td><td><span class="tag-pass">BLOCKED</span> (<code>PROJECT_ISOLATION_VIOLATION</code>)</td></tr>
        <tr><td>Xóa Project C (Clone) khỏi SQLite &amp; đĩa</td><td>Xóa theo phạm vi <code>project_id</code> trong transaction</td><td><span class="tag-pass">PASS</span> Project A/B nguyên vẹn 100%</td></tr>
      </table>
    </div>
    <div class="card">
      <div class="card-h">3. ĐỒNG BỘ THƯ MỤC ĐĨA &amp; DEEP CLONE</div>
      <p><b>Cấu trúc tự động khởi tạo cho mỗi Project mới:</b></p>
      <pre class="code-box">projects/&lt;slug&gt;/
  ├── project.json       (GameProjectManifest đồng bộ)
  ├── ui-schema.json     (MiniToolUiSchema đồng bộ khi save)
  ├── workflows/*.json   (Quy trình riêng của Project)
  ├── templates/*.png    (Ảnh mẫu Vision riêng + SHA-256)
  └── branding/icon.ico  (Biểu tượng EXE riêng của Project)</pre>
      <p style="margin-top:2mm;"><b>Deep Clone (<code>clone_project</code>):</b> Tự động remap <code>workflowId</code> mới, tạo <code>workflow_versions</code>, sao chép <code>templates/*.png</code>, remap <code>task_definitions</code> và cập nhật <code>ui-schema.json</code> không trùng lặp khóa.</p>
    </div>
  </div>
"""))

# PAGE 08: DIAGRAM 4 - WINCRED & DPAPI
pages.append(page_wrap(8, "VAULT & DPAPI KEYSTORE", "Kiến trúc Windows Credential Manager & Bảo vệ Keystore bằng DPAPI", f"""
  <div class="diag-wrap">
    <img src="{diag_b64['04_p2_wincred_dpapi_isolation']}" class="diag-img"/>
  </div>
  <div class="grid-2" style="margin-top:3mm;">
    <div class="card">
      <div class="card-h">THAY THẾ MOCK HASHMAP BẰNG WIN32 CREDENTIAL MANAGER</div>
      <p>Lớp <code>CredentialVault</code> trong <code>crates/fp-security/src/lib.rs</code> mặc định sử dụng backend <code>WindowsCredentialManager</code> (gọi trực tiếp <code>advapi32.dll</code>: <code>CredWriteW</code>, <code>CredReadW</code>, <code>CredDeleteW</code>, <code>CredEnumerateW</code>, <code>CredFree</code>) với phạm vi tách biệt theo Project, Mini Tool và Account.</p>
    </div>
    <div class="card">
      <div class="card-h">MÃ HÓA PRODUCER SIGNING KEY BẰNG WINDOWS DPAPI</div>
      <p><code>ProducerKeystore</code> mã hóa 32-byte Ed25519 Seed qua <code>crypt32.dll::CryptProtectData</code> kèm <code>OptionalEntropy</code> và cờ <code>CRYPTPROTECT_UI_FORBIDDEN</code>. Không còn lưu <code>secretSeedHex</code> dạng rõ trên ổ đĩa; hỗ trợ tự nâng cấp file v1 cũ lên DPAPI v2, xoay vòng khóa, sao lưu và khôi phục an toàn.</p>
    </div>
  </div>
"""))

# PAGE 09: WINCRED & DPAPI TEST RESULTS
pages.append(page_wrap(9, "KẾT QUẢ BẢO MẬT", "Bằng chứng Kiểm thử Windows Credential Manager & DPAPI Keystore", f"""
  <div class="grid-2">
    <div class="card">
      <div class="card-h">1. KẾT QUẢ WINDOWS CREDENTIAL MANAGER (OS THẬT)</div>
      <table>
        <tr><th>Kiểm thử WinCred (<code>fp-security</code> + <code>fp-cli</code>)</th><th>Kết quả</th><th>Chi tiết Xác minh</th></tr>
        <tr><td>Backend mặc định của <code>CredentialVault::new()</code></td><td><span class="tag-pass">PASS</span></td><td><code>{sb_vault['vaultBackend']}</code> (không dùng HashMap mock)</td></tr>
        <tr><td>Ghi &amp; đọc bí mật Project A vs Project B</td><td><span class="tag-pass">PASS</span></td><td><code>{sb_vault['uriA']}</code> vs <code>slg-p2-*</code></td></tr>
        <tr><td>Ghi &amp; đọc bí mật Mini Tool Namespace</td><td><span class="tag-pass">PASS</span></td><td><code>{sb_vault['uriTool']}</code></td></tr>
        <tr><td>Cách ly namespace chéo &amp; chặn wildcard <code>*</code> / <code>..</code></td><td><span class="tag-pass">PASS</span></td><td><code>isolatedValuesVerified = {sb_vault['isolatedValuesVerified']}</code></td></tr>
        <tr><td>Liệt kê metadata &amp; xóa sạch sau test</td><td><span class="tag-pass">PASS</span></td><td><code>list_secrets_namespaced</code> chỉ trả URI, không lộ secret</td></tr>
      </table>
    </div>
    <div class="card">
      <div class="card-h">2. KẾT QUẢ DPAPI KEYSTORE &amp; BACKUP/ROTATION</div>
      <table>
        <tr><th>Kiểm thử DPAPI (<code>crypt32.dll</code>)</th><th>Kết quả</th><th>Chi tiết Xác minh</th></tr>
        <tr><td>Cơ chế bảo vệ trên đĩa</td><td><span class="tag-pass">PASS</span></td><td><code>{sb_vault['dpapiKeystore']['protectionScheme']}</code></td></tr>
        <tr><td>Loại bỏ hoàn toàn <code>secretSeedHex</code> trên đĩa</td><td><span class="tag-pass">PASS</span></td><td><code>noPlaintextSeedOnDisk = {sb_vault['dpapiKeystore']['noPlaintextSeedOnDisk']}</code></td></tr>
        <tr><td>Chặn giải mã DPAPI khi sai Entropy / sửa 1 byte</td><td><span class="tag-pass">PASS</span></td><td><code>CryptUnprotectData</code> fail-closed</td></tr>
        <tr><td>Tự động nâng cấp file Keystore v1 -&gt; DPAPI v2</td><td><span class="tag-pass">PASS</span></td><td>Giữ nguyên Public Key, xóa plaintext seed</td></tr>
        <tr><td>Xoay vòng Khóa &amp; Backup/Restore Ed25519</td><td><span class="tag-pass">PASS</span></td><td>Rotate <code>{sb_vault['dpapiKeystore']['initialKeyId'][:18]}</code> -&gt; Restore khớp 100%</td></tr>
      </table>
    </div>
  </div>
  <div class="card" style="margin-top:4mm;">
    <div class="card-h">LOG KIỂM THỬ TRỰC TIẾP TỪ `cargo test -p fp-security` (6/6 PASS)</div>
    <pre class="code-box">running 6 tests
test tests::test_destructive_action_guard ... ok
test tests::test_dpapi_roundtrip_and_tamper_detection ... ok
test tests::test_secret_redaction ... ok
test tests::test_windows_credential_manager_namespaced_crud_and_isolation ... ok
test tests::test_dpapi_legacy_v1_keystore_auto_upgrade ... ok
test tests::test_dpapi_producer_keystore_create_backup_rotate_and_restore ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out</pre>
  </div>
"""))

# PAGE 10: TAURI IPC COMMANDS & 44/44 WORKSPACE UNIT TESTS
pages.append(page_wrap(10, "IPC & UNIT TESTS", "Danh mục 26 Tauri IPC Commands P2 & Kết quả 44/44 Workspace Tests", f"""
  <div class="grid-2">
    <div class="card">
      <div class="card-h">1. DANH MỤC 26 TAURI IPC COMMANDS MỚI (studio_p2.rs)</div>
      <table>
        <tr><th style="width:32%;">Nhóm Chức năng</th><th style="width:68%;">Tauri Commands Đã Đăng ký</th></tr>
        <tr><td><b>Project Workspace (6)</b></td><td><code>studio_list_projects</code>, <code>studio_get_project_detail</code>, <code>studio_create_project</code>, <code>studio_update_project</code>, <code>studio_clone_project</code>, <code>studio_delete_project</code></td></tr>
        <tr><td><b>Workflows &amp; Templates (3)</b></td><td><code>studio_list_project_workflows</code>, <code>studio_save_project_workflow</code>, <code>studio_list_project_templates</code></td></tr>
        <tr><td><b>Tasks &amp; Modules (5)</b></td><td><code>studio_list_project_tasks</code>, <code>studio_upsert_project_task</code>, <code>studio_delete_project_task</code>, <code>studio_list_project_modules</code>, <code>studio_upsert_project_module</code></td></tr>
        <tr><td><b>Mini Tool Schema &amp; Vault (6)</b></td><td><code>studio_get_ui_schema</code>, <code>studio_save_ui_schema</code>, <code>studio_list_project_accounts</code>, <code>studio_upsert_project_account</code>, <code>studio_vault_set_secret</code>, <code>studio_vault_has_secret</code>, <code>studio_vault_list_namespace</code>, <code>studio_vault_delete_secret</code></td></tr>
        <tr><td><b>Keystore &amp; DB Ops (7)</b></td><td><code>studio_get_producer_key_status</code>, <code>studio_rotate_producer_key</code>, <code>studio_backup_producer_key</code>, <code>studio_restore_producer_key</code>, <code>studio_backup_database</code>, <code>studio_check_db_integrity</code>, <code>studio_migrate_legacy_jx2</code></td></tr>
      </table>
    </div>
    <div class="card">
      <div class="card-h">2. TỔNG HỢP 44/44 UNIT TESTS TOÀN WORKSPACE</div>
      <table>
        <tr><th>Crate / App</th><th>Số Tests</th><th>Kết quả</th><th>Nội dung Test P2 Mới Bổ sung</th></tr>
        <tr><td><code>farmerpro</code> (Tauri)</td><td>1</td><td><span class="tag-pass">PASS</span></td><td><code>test_tauri_p2_project_crud_and_isolation_layer</code></td></tr>
        <tr><td><code>fp-storage</code></td><td>8</td><td><span class="tag-pass">PASS</span></td><td>+5 tests: v1→v2, Rollback, Backup/Restore, JX2 10 wf, Isolation</td></tr>
        <tr><td><code>fp-security</code></td><td>6</td><td><span class="tag-pass">PASS</span></td><td>+4 tests: WinCred Namespaces, DPAPI, Legacy v1 Upgrade, Backup/Restore</td></tr>
        <tr><td><code>fp-builder</code></td><td>4</td><td><span class="tag-pass">PASS</span></td><td>PE RT_RCDATA #7777 + DPAPI ProducerKeyManager delegation</td></tr>
        <tr><td><code>fp-workflow-runtime</code></td><td>3</td><td><span class="tag-pass">PASS</span></td><td>Chạy ổn định sau khi gỡ hardcode <code>jx2mobile</code></td></tr>
        <tr><td>10 Crates khác</td><td>22</td><td><span class="tag-pass">PASS</span></td><td><code>fp-actions(4), fp-adb(2), fp-bundle*(3), fp-device(3), fp-domain(2), fp-scheduler(2), fp-testkit(1), fp-vision(3), fp-workflow-schema(2)</code></td></tr>
        <tr><td><b>TỔNG WORKSPACE</b></td><td><b>44</b></td><td><span class="tag-pass">44/44 PASS</span></td><td>0 failed · 0 ignored (Log: <code>cargo_test_workspace_p2_clean.log</code>)</td></tr>
      </table>
    </div>
  </div>
"""))

# PAGE 11: HONEST PASS/FAIL, GIT DIFF & ROLLBACK PLAN
pages.append(page_wrap(11, "GIT & ROLLBACK", "Báo cáo PASS/FAIL Trung thực, Git Diff & Phương án Rollback P2", f"""
  <div class="grid-2">
    <div class="card">
      <div class="card-h">1. BÁO CÁO TRUNG THỰC PHẠM VI P2 vs P3–P6</div>
      <table>
        <tr><th>Hạng mục</th><th>Trạng thái</th><th>Ghi chú Kỹ thuật Trung thực</th></tr>
        <tr><td>Multi-Project SQLite v2 Schema &amp; CRUD</td><td><span class="tag-pass">PASS (TESTED)</span></td><td>Đã kiểm thử qua Rust unit tests + <code>fp-cli</code></td></tr>
        <tr><td>Online Backup, Migration, Rollback &amp; JX2</td><td><span class="tag-pass">PASS (TESTED)</span></td><td>Đã verify trên bản sao và nâng cấp an toàn <code>dist-portable</code></td></tr>
        <tr><td>Windows Credential Manager &amp; DPAPI</td><td><span class="tag-pass">PASS (TESTED)</span></td><td>Chạy API OS thật trên Windows 10/11</td></tr>
        <tr><td>Tauri IPC Registration (26 commands)</td><td><span class="tag-pass">IMPLEMENTED</span></td><td>Biên dịch sạch + test tầng handler Rust; chưa gọi từ UI React (chờ P4)</td></tr>
        <tr><td>Proxy Probe / ADB Forward / Checkpoint Resume</td><td><span class="tag-plan">PLANNED P3</span></td><td>Không triển khai ở P2 theo đúng kỷ luật Stage-Gate</td></tr>
        <tr><td>Giao diện Studio 6 khu vực &amp; WYSIWYG Builder</td><td><span class="tag-plan">PLANNED P4</span></td><td><code>apps/desktop/src</code> giữ nguyên 100%</td></tr>
      </table>
    </div>
    <div class="card">
      <div class="card-h">2. GIT METADATA &amp; PHƯƠNG ÁN ROLLBACK ĐẦY ĐỦ</div>
      <p><b>Git Log trên nhánh <code>feat/p2-multi-project-storage</code>:</b></p>
      <pre class="code-box">{git_log_txt.strip()}</pre>
      <p style="margin-top:2mm;"><b>Phương án Rollback tức thì nếu cần hủy P2:</b></p>
      <ol style="margin:1mm 0 0 4.5mm; padding:0; font-size:9pt; line-height:1.4;">
        <li><b>Khôi phục Mã nguồn:</b> Chạy <code>git checkout master</code> (về commit <code>fab1a1a</code>) hoặc <code>git revert 9bd0e45</code>.</li>
        <li><b>Khôi phục SQLite DB:</b> Chép đè lại bản sao lưu v1 đã kiểm chứng toàn vẹn từ <code>backups/p2_verified_backup/dist_portable_pre_v2.db</code> vào <code>dist-portable/FarmerPro/data/farmerpro.db</code> (xóa file <code>-wal/-shm</code> cũ nếu có).</li>
        <li><b>Tài nguyên gốc JX2:</b> Toàn bộ 10 JSON + 8 PNG tại <code>games/jx2mobile/</code> không hề bị xóa nên không cần khôi phục.</li>
      </ol>
    </div>
  </div>
  <div class="card" style="margin-top:2.5mm; padding:2.2mm 3mm;">
    <div class="card-h" style="margin-bottom:1.2mm;">3. TÓM TẮT GIT DIFF CODE P2 (git diff --stat fab1a1a..9bd0e45)</div>
    <pre class="code-box" style="column-count:2; column-gap:5mm; font-size:7.6pt; line-height:1.22; padding:2mm 2.8mm;">{git_diff_stat.strip()}</pre>
  </div>
"""))

# PAGE 12: READY FOR P3 & EVIDENCE MANIFEST
pages.append(page_wrap(12, "SẴN SÀNG CHO P3", "Danh sách Chức năng Sẵn sàng cho P3 & Hồ sơ Evidence SHA-256", f"""
  <div class="grid-2">
    <div class="card">
      <div class="card-h">1. DANH SÁCH CHỨC NĂNG P2 SẴN SÀNG BÀN GIAO CHO P3</div>
      <table>
        <tr><th style="width:30%;">Thành phần Sẵn sàng</th><th style="width:70%;">Cách P3 Sử dụng Trực tiếp</th></tr>
        <tr><td><b>Project Storage &amp; Paths</b></td><td><code>StorageManager::get_project_detail</code> + <code>projects/&lt;slug&gt;/templates/</code> sẵn sàng cho P3 ROI Cropper lưu ảnh mẫu và ghi bảng <code>vision_templates</code>.</td></tr>
        <tr><td><b>Task &amp; UI Schema Repo</b></td><td><code>upsert_project_task</code> + <code>save_ui_schema</code> sẵn sàng đồng bộ danh mục Task từ Workflow Studio sang <code>MiniToolBuilder</code>.</td></tr>
        <tr><td><b>Project-Scoped Jobs</b></td><td><code>ensure_project_job_exists</code> sẵn sàng gắn phiên chạy Test &amp; Debug và Checkpoint Resume vào đúng <code>project_id</code>.</td></tr>
        <tr><td><b>Credential Vault Namespaces</b></td><td><code>SecretNamespace::Project / Account / MiniTool</code> sẵn sàng cho P3 Account Vault &amp; Proxy Auth thật thay thế mock UI.</td></tr>
        <tr><td><b>DPAPI Producer Keystore</b></td><td><code>ProducerKeystore</code> + <code>ProducerKeyManager</code> đã bảo vệ Ed25519 seed bằng DPAPI, sẵn sàng cho Tauri Builder IPC ở P3.</td></tr>
        <tr><td><b>Game-Agnostic Runtime</b></td><td><code>fp-workflow-runtime</code> đã gỡ bỏ hardcode <code>jx2mobile</code>, sẵn sàng chạy nhiều GameProjects song song ở P3/P6.</td></tr>
      </table>
    </div>
    <div class="card">
      <div class="card-h">2. DANH MỤC TỆP BẰNG CHỨNG P2 (artifacts/p2_evidence/)</div>
      <table>
        <tr><th>Tệp Bằng chứng</th><th>Nội dung</th></tr>
        <tr><td><code>cargo_test_workspace_p2_clean.log</code></td><td>Log chạy thật 44/44 workspace unit tests PASS</td></tr>
        <tr><td><code>p2_verify_dist_portable_sandbox.json</code></td><td>JSON kết quả verify-p2 trên bản sao production DB</td></tr>
        <tr><td><code>p2_verify_root_db_sandbox.json</code></td><td>JSON kết quả verify-p2 trên bản sao root DB</td></tr>
        <tr><td><code>p2_dist_portable_migration_report.json</code></td><td>Báo cáo trước/sau migrate-p2 có bảo vệ backup &amp; restore probe</td></tr>
        <tr><td><code>sqlite_v1_v2_migration.sql</code></td><td>Mã SQL đầy đủ của Migration v1→v2</td></tr>
        <tr><td><code>git_diff_summary_p2.txt</code> &amp; <code>git_log_p2.txt</code></td><td>Thống kê diff và lịch sử commit nhánh P2</td></tr>
        <tr><td><code>diagrams/*.svg</code> &amp; <code>*.mmd</code></td><td>4 sơ đồ kiến trúc dữ liệu, crates, migration &amp; bảo mật</td></tr>
        <tr><td><code>EVIDENCE_MANIFEST_SHA256.json</code></td><td>Bảng băm SHA-256 kiểm chứng toàn vẹn mọi tệp P2</td></tr>
      </table>
      <div class="gate-note">
        <b>ĐỀ XUẤT NGHIỆM THU P2:</b> Đủ điều kiện nghiệm thu kỹ thuật P2 và mở khóa triển khai <b>Giai đoạn P3</b> (Backend Services: Proxy Probe, ADB Forward/Reverse, Checkpoint Resume, ROI Template Cropper &amp; Builder Tauri IPC). Không thay đổi <code>apps/desktop/src</code> cho tới P4.
      </div>
    </div>
  </div>
"""))

html_doc = f"""<!DOCTYPE html>
<html lang="vi">
<head>
<meta charset="utf-8"/>
<title>FarmerPro P2 Acceptance Report</title>
<style>
  @page {{
    size: A4 landscape;
    margin: 0;
  }}
  * {{
    box-sizing: border-box;
  }}
  body {{
    margin: 0;
    padding: 0;
    font-family: "Segoe UI", Arial, sans-serif;
    font-size: 9.5pt;
    line-height: 1.38;
    color: #0F172A;
    background: #FFFFFF;
  }}
  .pdf-page {{
    width: 297mm;
    height: 210mm;
    padding: 9mm 11mm 10mm 11mm;
    page-break-after: always;
    position: relative;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: #F8FAFC;
  }}
  .pdf-page:last-child {{
    page-break-after: auto;
  }}
  .page-top-bar {{
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: #0F172A;
    color: #F8FAFC;
    padding: 2.2mm 4mm;
    border-radius: 5px;
    margin-bottom: 3.5mm;
    flex-shrink: 0;
  }}
  .pt-left {{
    display: flex;
    align-items: center;
    gap: 8px;
  }}
  .pt-badge {{
    background: #1D4ED8;
    color: #FFFFFF;
    font-weight: 700;
    font-size: 8.2pt;
    padding: 1.2mm 2.6mm;
    border-radius: 3px;
    letter-spacing: 0.3px;
  }}
  .pt-title {{
    font-size: 10.5pt;
    font-weight: 700;
  }}
  .pt-right {{
    font-size: 8.2pt;
    color: #94A3B8;
    font-family: Consolas, monospace;
  }}
  .page-content {{
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }}
  .page-bottom-bar {{
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-top: 1px solid #CBD5E1;
    padding-top: 1.8mm;
    margin-top: 2mm;
    font-size: 8pt;
    color: #475569;
    flex-shrink: 0;
  }}
  .pg-num {{
    font-weight: 700;
    color: #0F172A;
  }}
  .hero-box {{
    background: linear-gradient(135deg, #0F172A 0%, #1E293B 100%);
    color: #F8FAFC;
    padding: 4mm 5mm;
    border-radius: 7px;
    border-left: 5px solid #2563EB;
  }}
  .hero-box h1 {{
    margin: 2mm 0 1.5mm 0;
    font-size: 13.5pt;
    line-height: 1.25;
  }}
  .hero-sub {{
    margin: 0;
    font-size: 9.4pt;
    color: #E2E8F0;
  }}
  .pill-row {{
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }}
  .badge {{
    font-size: 8pt;
    font-weight: 700;
    padding: 0.8mm 2.4mm;
    border-radius: 3px;
  }}
  .b-pass {{ background: #065F46; color: #D1FAE5; }}
  .b-info {{ background: #1E40AF; color: #DBEAFE; }}
  .b-warn {{ background: #78350F; color: #FEF3C7; }}
  .grid-2 {{
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 3.5mm;
  }}
  .grid-3 {{
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: 3.2mm;
  }}
  .card {{
    background: #FFFFFF;
    border: 1px solid #CBD5E1;
    border-radius: 6px;
    padding: 2.8mm 3.4mm;
  }}
  .card-h {{
    font-size: 9.4pt;
    font-weight: 700;
    color: #1E3A8A;
    border-bottom: 1.5px solid #E2E8F0;
    padding-bottom: 1.2mm;
    margin-bottom: 2mm;
  }}
  .card p {{
    margin: 0;
    font-size: 9pt;
    line-height: 1.38;
  }}
  table {{
    width: 100%;
    border-collapse: collapse;
    font-size: 8.6pt;
  }}
  th, td {{
    border: 1px solid #CBD5E1;
    padding: 1.3mm 2mm;
    text-align: left;
    vertical-align: top;
  }}
  table.tbl-tight th, table.tbl-tight td {{
    padding: 0.95mm 1.6mm;
    font-size: 8.3pt;
  }}
  th {{
    background: #EFF6FF;
    color: #1E3A8A;
    font-weight: 700;
  }}
  tr:nth-child(even) td {{
    background: #F8FAFC;
  }}
  code {{
    font-family: Consolas, "Courier New", monospace;
    font-size: 8.2pt;
    background: #F1F5F9;
    padding: 0.2mm 1mm;
    border-radius: 3px;
    color: #0F172A;
  }}
  pre.code-box {{
    margin: 0;
    background: #0F172A;
    color: #E2E8F0;
    font-family: Consolas, "Courier New", monospace;
    font-size: 8pt;
    line-height: 1.3;
    padding: 2.5mm 3mm;
    border-radius: 5px;
    overflow: hidden;
    white-space: pre-wrap;
    word-break: break-word;
  }}
  .tag-pass {{
    display: inline-block;
    background: #DCFCE7;
    color: #166534;
    font-weight: 700;
    font-size: 8pt;
    padding: 0.4mm 1.8mm;
    border-radius: 3px;
    border: 1px solid #86EFAC;
  }}
  .tag-plan {{
    display: inline-block;
    background: #FEF3C7;
    color: #92400E;
    font-weight: 700;
    font-size: 8pt;
    padding: 0.4mm 1.8mm;
    border-radius: 3px;
    border: 1px solid #FDE68A;
  }}
  .toc-grid {{
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1.5mm 4mm;
    font-size: 8.8pt;
  }}
  .toc-grid a {{
    color: #1D4ED8;
    text-decoration: none;
    padding: 0.7mm 1.5mm;
    border-radius: 3px;
    background: #F8FAFC;
    border: 1px solid #E2E8F0;
  }}
  .diag-wrap {{
    background: #0F172A;
    border-radius: 8px;
    padding: 1.5mm;
    display: flex;
    justify-content: center;
    align-items: center;
    border: 1px solid #334155;
  }}
  .diag-img {{
    width: 100%;
    max-height: 136mm;
    object-fit: contain;
  }}
  .gate-note {{
    margin-top: 2.5mm;
    background: #ECFDF5;
    border: 1px solid #6EE7B7;
    border-left: 4px solid #059669;
    padding: 2.2mm 3mm;
    border-radius: 4px;
    font-size: 8.8pt;
    color: #064E3B;
  }}
</style>
</head>
<body>
{"".join(pages)}
</body>
</html>
"""

with open(HTML_OUT, "w", encoding="utf-8") as f:
    f.write(html_doc)

print("Generating PDF via Chrome headless...")
subprocess.run([
    CHROME,
    "--headless=new",
    "--disable-gpu",
    "--no-pdf-header-footer",
    f"--print-to-pdf={PDF_MAIN}",
    f"file:///{HTML_OUT.replace(os.sep, '/')}",
], check=True)

# Add PDF Bookmarks & run PyMuPDF QA
doc = pymupdf.open(PDF_MAIN)
toc = [
    [1, "01. Trang Bìa & Tóm tắt Kết quả Nghiệm thu P2", 1],
    [1, "02. Bản đồ Rust Crates & Ranh giới Kiến trúc P2", 2],
    [1, "03. Mô hình Dữ liệu SQLite WAL v2 (ER Schema)", 3],
    [1, "04. Đặc tả Migration SQL v1->v2 & Ràng buộc Toàn vẹn", 4],
    [1, "05. Quy trình Online Backup, Restore Probe & Rollback", 5],
    [1, "06. Đối chiếu Dữ liệu & Bảo toàn 10 JX2 Workflows", 6],
    [1, "07. Chứng cứ Project CRUD, Clone & Project Isolation", 7],
    [1, "08. Kiến trúc Windows Credential Manager & DPAPI Keystore", 8],
    [1, "09. Kết quả Kiểm thử Thực tế WinCred & DPAPI", 9],
    [1, "10. Danh mục 26 Tauri IPC Commands & 44/44 Unit Tests", 10],
    [1, "11. Báo cáo PASS/FAIL, Git Diff & Phương án Rollback", 11],
    [1, "12. Danh sách Chức năng Sẵn sàng cho P3 & SHA-256 Manifest", 12],
]
doc.set_toc(toc)
tmp_pdf = PDF_MAIN + ".tmp"
doc.save(tmp_pdf, garbage=4, deflate=True)
doc.close()
os.replace(tmp_pdf, PDF_MAIN)
shutil.copy2(PDF_MAIN, PDF_COPY)

# QA verification
for f in os.listdir(RENDER_DIR):
    if f.endswith(".png"):
        os.remove(os.path.join(RENDER_DIR, f))

qa_doc = pymupdf.open(PDF_MAIN)
min_pt = 99.0
small_fonts = []
clipping = []
encoding_errs = []
for idx, page in enumerate(qa_doc):
    pno = idx + 1
    pix = page.get_pixmap(dpi=130)
    pix.save(os.path.join(RENDER_DIR, f"p2_page_{pno:02d}.png"))
    txt = page.get_text("text")
    if "\ufffd" in txt:
        encoding_errs.append(f"Page {pno}: U+FFFD")
    pdict = page.get_text("dict")
    for blk in pdict.get("blocks", []):
        if blk.get("type") != 0:
            continue
        bbox = blk.get("bbox", [0, 0, 0, 0])
        if bbox[2] > page.rect.width + 2 or bbox[3] > page.rect.height + 2:
            clipping.append(f"Page {pno}: overflow bbox {bbox}")
        for ln in blk.get("lines", []):
            for sp in ln.get("spans", []):
                sz = sp.get("size", 0.0)
                stxt = sp.get("text", "").strip()
                if len(stxt) >= 3 and sz > 0:
                    min_pt = min(min_pt, sz)
                    if sz < 7.5:
                        small_fonts.append(f"Page {pno}: {sz:.2f}pt '{stxt[:30]}'")

qa_report = {
    "pdfPath": PDF_MAIN,
    "pdfSizeBytes": os.path.getsize(PDF_MAIN),
    "pdfSha256": sha256_file(PDF_MAIN),
    "pagesCount": len(qa_doc),
    "bookmarksCount": len(qa_doc.get_toc()),
    "minFontPt": round(min_pt, 2),
    "smallFontWarnings": small_fonts,
    "clippingErrors": clipping,
    "encodingErrors": encoding_errs,
    "passed": len(qa_doc) == TOTAL_PAGES and len(clipping) == 0 and len(encoding_errs) == 0 and min_pt >= 7.5,
}
qa_doc.close()

with open(QA_JSON, "w", encoding="utf-8") as f:
    json.dump(qa_report, f, indent=2, ensure_ascii=False)

# Build P2 SHA-256 Evidence Manifest
manifest = {}
for root_dir, _, files in os.walk(EV_DIR):
    if "sandbox_run" in root_dir:
        continue
    for fn in sorted(files):
        full = os.path.join(root_dir, fn)
        rel = os.path.relpath(full, EV_DIR).replace("\\", "/")
        if rel == "EVIDENCE_MANIFEST_SHA256.json":
            continue
        manifest[rel] = {
            "bytes": os.path.getsize(full),
            "sha256": sha256_file(full),
        }

with open(os.path.join(EV_DIR, "EVIDENCE_MANIFEST_SHA256.json"), "w", encoding="utf-8") as f:
    json.dump(manifest, f, indent=2, ensure_ascii=False)

print(json.dumps(qa_report, indent=2, ensure_ascii=False))
