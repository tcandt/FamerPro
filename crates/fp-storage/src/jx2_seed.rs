use crate::fs_atomic::{atomic_write_bytes, ensure_contained_project_dir};
use crate::models::{
    Jx2MigrationSummary, Jx2WorkflowMigrationItem, DEFAULT_LEGACY_PROJECT_ID,
    DEFAULT_LEGACY_PROJECT_SLUG,
};
use anyhow::{bail, Context, Result};
use chrono::Utc;
use fp_bundle_contract::{
    CompatibilityVersions, GameProjectManifest, MiniLayoutPreset, MiniTaskDef, MiniToolUiSchema,
    RuntimeColumnBinding, TaskCategoryDef, TaskSettingField, UtilityToggleDef,
};
use fp_workflow_schema::WorkflowValidator;
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;

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

fn jx2_workflow_category(workflow_id: &str) -> &'static str {
    match workflow_id {
        "wf-jx2-01-login" => "Khởi động & Đăng nhập",
        "wf-jx2-02-char" => "Tạo nhân vật",
        "wf-jx2-03-tutorial" => "Hướng dẫn tân thủ",
        "wf-jx2-04-npc" => "Tương tác NPC",
        "wf-jx2-05-accept" => "Tiếp nhận nhiệm vụ",
        "wf-jx2-06-movement" => "Di chuyển",
        "wf-jx2-07-combat" => "Chiến đấu",
        "wf-jx2-08-verify" => "Xác minh & Thưởng",
        "wf-jx2-09-complete" => "Hoàn tất tân thủ",
        "wf-jx2-e2e-beginner" => "E2E Toàn trình",
        _ => "Chung",
    }
}

pub fn migrate_legacy_jx2_assets(
    conn: &mut Connection,
    legacy_jx2_dir: &Path,
    projects_base_dir: &Path,
) -> Result<Jx2MigrationSummary> {
    if !legacy_jx2_dir.exists() {
        bail!(
            "Legacy JX2 directory not found: {}",
            legacy_jx2_dir.display()
        );
    }

    let proj_dir = projects_base_dir.join(DEFAULT_LEGACY_PROJECT_SLUG);
    let _ = ensure_contained_project_dir(projects_base_dir, &proj_dir)?;
    let proj_wf_dir = proj_dir.join("workflows");
    let proj_tpl_dir = proj_dir.join("templates");
    let proj_brand_dir = proj_dir.join("branding");
    std::fs::create_dir_all(&proj_wf_dir)?;
    std::fs::create_dir_all(&proj_tpl_dir)?;
    std::fs::create_dir_all(&proj_brand_dir)?;

    // Collect and validate the 10 JX2 workflow JSON files
    let mut wf_entries: Vec<(String, String, String, String, String)> = Vec::new();
    let mut wf_rel_files: Vec<String> = Vec::new();
    let mut dir_items: Vec<_> = std::fs::read_dir(legacy_jx2_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().is_file()
                && e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("json"))
                    .unwrap_or(false)
        })
        .collect();
    dir_items.sort_by_key(|e| e.file_name());

    for entry in dir_items {
        let src_path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();
        let content = std::fs::read_to_string(&src_path)
            .with_context(|| format!("Failed reading {}", src_path.display()))?;
        let doc = WorkflowValidator::parse_and_validate(&content)
            .with_context(|| format!("Invalid JX2 workflow JSON: {}", src_path.display()))?;

        // Copy non-destructively to projects/jx2-mobile/workflows/ via atomic write
        atomic_write_bytes(&proj_wf_dir.join(&file_name), content.as_bytes())?;
        wf_rel_files.push(format!("workflows/{}", file_name));

        let category = jx2_workflow_category(&doc.workflow_id).to_string();
        wf_entries.push((file_name, doc.workflow_id, doc.name, category, content));
    }

    // Collect and inspect template PNGs in games/jx2mobile/templates/
    let legacy_tpl_dir = legacy_jx2_dir.join("templates");
    let mut tpl_entries: Vec<(String, String, u32, u32, String)> = Vec::new();
    let mut tpl_rel_files: Vec<String> = Vec::new();
    if legacy_tpl_dir.exists() {
        let mut tpl_items: Vec<_> = std::fs::read_dir(&legacy_tpl_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path().is_file()
                    && e.path()
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| ext.eq_ignore_ascii_case("png"))
                        .unwrap_or(false)
            })
            .collect();
        tpl_items.sort_by_key(|e| e.file_name());

        for entry in tpl_items {
            let src_path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();
            let bytes = std::fs::read(&src_path)?;
            let sha = sha256_hex(&bytes);
            let (w, h) = image::image_dimensions(&src_path).unwrap_or((64, 64));

            // Non-destructive copy into project templates directory
            atomic_write_bytes(&proj_tpl_dir.join(&file_name), &bytes)?;
            let rel_path = format!("templates/{}", file_name);
            tpl_rel_files.push(rel_path.clone());
            let name = file_name.trim_end_matches(".png").to_string();
            tpl_entries.push((name, rel_path, w, h, sha));
        }
    }

    let now = Utc::now().to_rfc3339();
    let proj_dir_str = proj_dir.to_string_lossy().to_string();

    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;

    // 1. Ensure default legacy GameProject exists
    tx.execute(
        "INSERT INTO projects (
            id, slug, name, game_package_id, version, exe_name, window_title,
            icon_rel_path, target_width_px, target_height_px, bundle_adb,
            project_dir_path, created_at, updated_at
        ) VALUES (
            ?1, ?2, ?3, ?4, '1.0.0', 'Auto_JX2_Mobile.exe',
            'Auto Võ Hiệp 2 Mobile v1.0.0', 'branding/icon.ico',
            1920, 1080, 1, ?5, ?6, ?6
        )
        ON CONFLICT(id) DO UPDATE SET
            project_dir_path = excluded.project_dir_path,
            updated_at = excluded.updated_at;",
        params![
            DEFAULT_LEGACY_PROJECT_ID,
            DEFAULT_LEGACY_PROJECT_SLUG,
            "Võ Hiệp Truyền Kỳ 2 Mobile",
            "com.blackhole.jx2mobile",
            proj_dir_str,
            now
        ],
    )?;

    // 2. Attach any legacy NULL project_id workflows/accounts/jobs without losing them
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

    let legacy_preserved: i64 = tx.query_row(
        "SELECT COUNT(*) FROM workflows WHERE project_id = ?1;",
        params![DEFAULT_LEGACY_PROJECT_ID],
        |r| r.get(0),
    )?;

    // 3. Upsert the 10 JX2 workflows safely (INSERT new, UPDATE only if graph matches & metadata upgraded, SKIP if user modified graph, ALREADY_EXISTS if identical)
    let mut workflows_imported = 0usize;
    let mut workflow_migrations = Vec::with_capacity(wf_entries.len());
    for (src_file, wf_id, wf_name, category, wf_json) in &wf_entries {
        let wf_sha = sha256_hex(wf_json.as_bytes());
        let existing_row: Option<(Option<String>, String)> = {
            let mut st =
                tx.prepare("SELECT project_id, draft_json FROM workflows WHERE id = ?1;")?;
            let mut rows = st.query(params![wf_id])?;
            rows.next()?
                .map(|r| Ok::<_, rusqlite::Error>((r.get(0)?, r.get(1)?)))
                .transpose()?
        };

        let existing_ver: Option<(i64, String, String)> = {
            let mut st = tx.prepare(
                "SELECT version_no, id, sha256 FROM workflow_versions WHERE workflow_id = ?1 ORDER BY version_no DESC LIMIT 1;",
            )?;
            let mut rows = st.query(params![wf_id])?;
            rows.next()?
                .map(|r| Ok::<_, rusqlite::Error>((r.get(0)?, r.get(1)?, r.get(2)?)))
                .transpose()?
        };

        match existing_row {
            None => {
                tx.execute(
                    "INSERT INTO workflows (id, project_id, name, category, draft_json, draft_updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6);",
                    params![
                        wf_id,
                        DEFAULT_LEGACY_PROJECT_ID,
                        wf_name,
                        category,
                        wf_json,
                        now
                    ],
                )?;
                let version_id = format!("{}-v1", wf_id);
                tx.execute(
                    "INSERT INTO workflow_versions (id, workflow_id, version_no, graph_json, sha256, created_at)
                     VALUES (?1, ?2, 1, ?3, ?4, ?5);",
                    params![version_id, wf_id, wf_json, wf_sha, now],
                )?;
                workflows_imported += 1;
                workflow_migrations.push(Jx2WorkflowMigrationItem {
                    source_file: src_file.clone(),
                    source_workflow_id: wf_id.clone(),
                    dest_workflow_id: wf_id.clone(),
                    dest_version_id: version_id,
                    source_sha256: wf_sha,
                    action: "INSERT".to_string(),
                    version_before: None,
                    version_after: 1,
                    content_comparison: "NEW_WORKFLOW_IMPORTED".to_string(),
                });
            }
            Some((owner_opt, existing_draft)) => {
                let ver_before = existing_ver.as_ref().map(|v| v.0);
                let cur_ver_no = ver_before.unwrap_or(1);
                let cur_ver_id = existing_ver
                    .as_ref()
                    .map(|v| v.1.clone())
                    .unwrap_or_else(|| format!("{}-v1", wf_id));

                if owner_opt.as_deref() != Some(DEFAULT_LEGACY_PROJECT_ID) {
                    workflow_migrations.push(Jx2WorkflowMigrationItem {
                        source_file: src_file.clone(),
                        source_workflow_id: wf_id.clone(),
                        dest_workflow_id: wf_id.clone(),
                        dest_version_id: cur_ver_id,
                        source_sha256: wf_sha,
                        action: "SKIP".to_string(),
                        version_before: ver_before,
                        version_after: cur_ver_no,
                        content_comparison: "SKIPPED_OWNED_BY_OTHER_PROJECT".to_string(),
                    });
                    continue;
                }

                let existing_sha = sha256_hex(existing_draft.as_bytes());
                if existing_sha == wf_sha {
                    workflows_imported += 1;
                    workflow_migrations.push(Jx2WorkflowMigrationItem {
                        source_file: src_file.clone(),
                        source_workflow_id: wf_id.clone(),
                        dest_workflow_id: wf_id.clone(),
                        dest_version_id: cur_ver_id,
                        source_sha256: wf_sha,
                        action: "ALREADY_EXISTS".to_string(),
                        version_before: ver_before,
                        version_after: cur_ver_no,
                        content_comparison: "EXACT_SHA256_MATCH".to_string(),
                    });
                } else {
                    // Check semantic graph equality: only upgrade if graph (nodes & edges) is identical
                    let existing_val =
                        serde_json::from_str::<serde_json::Value>(&existing_draft).ok();
                    let source_val = serde_json::from_str::<serde_json::Value>(wf_json).ok();
                    let graphs_match = match (&existing_val, &source_val) {
                        (Some(e), Some(s)) => {
                            e.get("graph") == s.get("graph") && e.get("graph").is_some()
                        }
                        _ => false,
                    };

                    if graphs_match {
                        tx.execute(
                            "UPDATE workflows SET name = ?1, category = ?2, draft_json = ?3, draft_updated_at = ?4
                             WHERE id = ?5 AND project_id = ?6;",
                            params![
                                wf_name,
                                category,
                                wf_json,
                                now,
                                wf_id,
                                DEFAULT_LEGACY_PROJECT_ID
                            ],
                        )?;
                        let next_ver = cur_ver_no + 1;
                        let next_ver_id = format!("{}-v{}", wf_id, next_ver);
                        tx.execute(
                            "INSERT INTO workflow_versions (id, workflow_id, version_no, graph_json, sha256, created_at)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6);",
                            params![next_ver_id, wf_id, next_ver, wf_json, wf_sha, now],
                        )?;
                        workflows_imported += 1;
                        workflow_migrations.push(Jx2WorkflowMigrationItem {
                            source_file: src_file.clone(),
                            source_workflow_id: wf_id.clone(),
                            dest_workflow_id: wf_id.clone(),
                            dest_version_id: next_ver_id,
                            source_sha256: wf_sha,
                            action: "UPDATE".to_string(),
                            version_before: ver_before,
                            version_after: next_ver,
                            content_comparison: "IDENTICAL_GRAPH_UPGRADED_TO_PUBLISHED_POLICY"
                                .to_string(),
                        });
                    } else {
                        // Preserve user-modified workflow without overwriting!
                        workflow_migrations.push(Jx2WorkflowMigrationItem {
                            source_file: src_file.clone(),
                            source_workflow_id: wf_id.clone(),
                            dest_workflow_id: wf_id.clone(),
                            dest_version_id: cur_ver_id,
                            source_sha256: wf_sha,
                            action: "SKIP".to_string(),
                            version_before: ver_before,
                            version_after: cur_ver_no,
                            content_comparison: "PRESERVED_USER_CUSTOMIZED_WORKFLOW".to_string(),
                        });
                    }
                }
            }
        }
    }

    // 4. Upsert vision templates
    let mut templates_imported = 0usize;
    for (name, rel_path, w, h, sha) in &tpl_entries {
        let tpl_id = format!("tpl-jx2-{}", name);
        tx.execute(
            "INSERT INTO vision_templates (
                id, project_id, name, rel_path, width_px, height_px,
                default_confidence, sha256, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0.85, ?7, ?8)
            ON CONFLICT(project_id, rel_path) DO UPDATE SET
                name = excluded.name,
                width_px = excluded.width_px,
                height_px = excluded.height_px,
                sha256 = excluded.sha256;",
            params![
                tpl_id,
                DEFAULT_LEGACY_PROJECT_ID,
                name,
                rel_path,
                *w as i64,
                *h as i64,
                sha,
                now
            ],
        )?;
        templates_imported += 1;
    }

    // 5. Seed project modules idempotently
    let modules = [
        ("mod.adb_core", 1, 1),
        ("mod.vision_ocr", 1, 1),
        ("mod.account_vault", 1, 1),
        ("mod.proxy_network", 1, 0),
        ("mod.adb_port_forward", 0, 0),
        ("mod.emulator_optimizer", 1, 1),
    ];
    let mut modules_seeded = 0usize;
    for (mod_id, packaged, enabled) in modules {
        tx.execute(
            "INSERT OR IGNORE INTO project_modules (project_id, module_id, packaged_in_release, enabled_by_default, config_json, updated_at)
             VALUES (?1, ?2, ?3, ?4, '{}', ?5);",
            params![DEFAULT_LEGACY_PROJECT_ID, mod_id, packaged, enabled, now],
        )?;
        modules_seeded += 1;
    }

    // 6. Seed default task definitions for JX2 if none exist yet
    let existing_tasks: i64 = tx.query_row(
        "SELECT COUNT(*) FROM task_definitions WHERE project_id = ?1;",
        params![DEFAULT_LEGACY_PROJECT_ID],
        |r| r.get(0),
    )?;
    let mut tasks_seeded = 0usize;
    let default_tasks = build_default_jx2_tasks();
    if existing_tasks == 0 {
        for (task, cat) in &default_tasks {
            let settings_json = serde_json::to_string(&task.settings_schema)?;
            tx.execute(
                "INSERT INTO task_definitions (
                    id, project_id, workflow_id, category_id, category_short_tag,
                    category_name, category_color_hex, title, priority, estimate_sec,
                    default_enabled, settings_schema_json, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13);",
                params![
                    task.id,
                    DEFAULT_LEGACY_PROJECT_ID,
                    task.workflow_id,
                    cat.id,
                    cat.short_tag,
                    cat.display_name,
                    cat.color_hex,
                    task.title,
                    task.priority,
                    task.estimate_sec as i64,
                    if task.default_enabled { 1 } else { 0 },
                    settings_json,
                    now
                ],
            )?;
            tasks_seeded += 1;
        }
    } else {
        tasks_seeded = existing_tasks as usize;
    }

    // 7. Seed default MiniToolUiSchema if missing
    let ui_schema = build_default_jx2_ui_schema(&default_tasks);
    let ui_schema_json = serde_json::to_string_pretty(&ui_schema)?;
    tx.execute(
        "INSERT OR IGNORE INTO mini_tool_schemas (project_id, schema_version, ui_schema_json, updated_at)
         VALUES (?1, ?2, ?3, ?4);",
        params![
            DEFAULT_LEGACY_PROJECT_ID,
            ui_schema.schema_version as i64,
            ui_schema_json,
            now
        ],
    )?;

    tx.commit()?;

    // Write disk project.json and ui-schema.json if not yet present
    let manifest = GameProjectManifest {
        compatibility: CompatibilityVersions::default(),
        project_id: DEFAULT_LEGACY_PROJECT_ID.to_string(),
        slug: DEFAULT_LEGACY_PROJECT_SLUG.to_string(),
        name: "Võ Hiệp Truyền Kỳ 2 Mobile".to_string(),
        game_package_id: "com.blackhole.jx2mobile".to_string(),
        version: "1.0.0".to_string(),
        exe_name: "Auto_JX2_Mobile.exe".to_string(),
        window_title: "Auto Võ Hiệp 2 Mobile v1.0.0".to_string(),
        icon_rel_path: "branding/icon.ico".to_string(),
        target_resolution: (1920, 1080),
        packaged_modules: vec![
            "mod.adb_core".to_string(),
            "mod.vision_ocr".to_string(),
            "mod.account_vault".to_string(),
            "mod.proxy_network".to_string(),
            "mod.emulator_optimizer".to_string(),
        ],
        workflow_files: wf_rel_files,
        template_files: tpl_rel_files,
        bundle_adb: true,
    };
    let manifest_path = proj_dir.join("project.json");
    if !manifest_path.exists() {
        atomic_write_bytes(&manifest_path, &serde_json::to_vec_pretty(&manifest)?)?;
    }
    let ui_path = proj_dir.join("ui-schema.json");
    if !ui_path.exists() {
        atomic_write_bytes(&ui_path, ui_schema_json.as_bytes())?;
    }

    Ok(Jx2MigrationSummary {
        project_id: DEFAULT_LEGACY_PROJECT_ID.to_string(),
        project_slug: DEFAULT_LEGACY_PROJECT_SLUG.to_string(),
        project_dir_path: proj_dir.to_string_lossy().to_string(),
        workflows_imported,
        legacy_workflows_preserved: legacy_preserved as usize,
        templates_imported,
        tasks_seeded,
        modules_seeded,
        original_source_preserved: legacy_jx2_dir.exists(),
        workflow_migrations,
    })
}

fn build_default_jx2_tasks() -> Vec<(MiniTaskDef, TaskCategoryDef)> {
    let cat_nv = TaskCategoryDef {
        id: "cat-nv".to_string(),
        short_tag: "[NV]".to_string(),
        display_name: "Nhiệm vụ Ngày".to_string(),
        color_hex: "#38BDF8".to_string(),
    };
    let cat_e2e = TaskCategoryDef {
        id: "cat-e2e".to_string(),
        short_tag: "[E2E]".to_string(),
        display_name: "Chuỗi Tân Thủ".to_string(),
        color_hex: "#F59E0B".to_string(),
    };

    vec![
        (
            MiniTaskDef {
                id: "task-jx2-login".to_string(),
                title: "Khởi động & Đăng nhập Chơi Ngay".to_string(),
                category_id: cat_nv.id.clone(),
                workflow_id: "wf-jx2-01-login".to_string(),
                default_enabled: true,
                priority: 1,
                estimate_sec: 25,
                settings_schema: vec![TaskSettingField {
                    key: "auto_skip_dialog".to_string(),
                    label: "Tự động bỏ qua hội thoại NPC".to_string(),
                    field_type: "boolean".to_string(),
                    default_value: serde_json::json!(true),
                    options: None,
                }],
            },
            cat_nv.clone(),
        ),
        (
            MiniTaskDef {
                id: "task-jx2-npc".to_string(),
                title: "Tương tác NPC Dã Tẩu & Nhận Nhiệm vụ".to_string(),
                category_id: cat_nv.id.clone(),
                workflow_id: "wf-jx2-04-npc".to_string(),
                default_enabled: true,
                priority: 2,
                estimate_sec: 45,
                settings_schema: vec![TaskSettingField {
                    key: "max_quest_loops".to_string(),
                    label: "Số chuỗi Dã Tẩu tối đa".to_string(),
                    field_type: "number".to_string(),
                    default_value: serde_json::json!(10),
                    options: None,
                }],
            },
            cat_nv,
        ),
        (
            MiniTaskDef {
                id: "task-jx2-e2e".to_string(),
                title: "Toàn trình Nhiệm vụ Tân Thủ Võ Hiệp 2".to_string(),
                category_id: cat_e2e.id.clone(),
                workflow_id: "wf-jx2-e2e-beginner".to_string(),
                default_enabled: true,
                priority: 3,
                estimate_sec: 90,
                settings_schema: vec![],
            },
            cat_e2e,
        ),
    ]
}

fn build_default_jx2_ui_schema(
    tasks_with_cats: &[(MiniTaskDef, TaskCategoryDef)],
) -> MiniToolUiSchema {
    let mut cats = Vec::new();
    let mut tasks = Vec::new();
    for (t, c) in tasks_with_cats {
        if !cats
            .iter()
            .any(|existing: &TaskCategoryDef| existing.id == c.id)
        {
            cats.push(c.clone());
        }
        tasks.push(t.clone());
    }
    MiniToolUiSchema {
        schema_version: 1,
        project_slug: DEFAULT_LEGACY_PROJECT_SLUG.to_string(),
        window_title: "Auto Võ Hiệp 2 Mobile v1.0.0".to_string(),
        version_badge: "JX2-MMORPG v1.0".to_string(),
        theme_accent_hex: "#F59E0B".to_string(),
        header_bg_hex: "#18131A".to_string(),
        layout_preset: MiniLayoutPreset::Compact2Col,
        default_window_size: (740, 660),
        sections_order: vec![
            "device_table".to_string(),
            "task_and_log_split".to_string(),
            "footer_controls".to_string(),
        ],
        hidden_components: vec![],
        device_columns: vec![
            RuntimeColumnBinding {
                id: "col-level".to_string(),
                label: "Cấp".to_string(),
                runtime_var_key: "char_level".to_string(),
                fallback_display: "Chưa OCR (-)".to_string(),
                min_window_width: 360,
                badge_color: Some("#FBBF24".to_string()),
            },
            RuntimeColumnBinding {
                id: "col-gold".to_string(),
                label: "Vàng".to_string(),
                runtime_var_key: "char_gold".to_string(),
                fallback_display: "Chưa OCR (-)".to_string(),
                min_window_width: 480,
                badge_color: Some("#F59E0B".to_string()),
            },
        ],
        task_categories: cats,
        tasks,
        utilities: vec![UtilityToggleDef {
            id: "util-proxy-route".to_string(),
            label: "Định tuyến SOCKS5 Proxy theo Tài khoản".to_string(),
            default_enabled: false,
            required_module: Some("mod.proxy_network".to_string()),
            settings_schema: vec![],
        }],
        initial_runtime_vars: HashMap::new(),
    }
}
