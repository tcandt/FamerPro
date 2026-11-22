use crate::AppState;
use fp_bundle_contract::{MiniTaskDef, MiniToolUiSchema, TaskCategoryDef};
use fp_security::{
    CredentialMetadata, ProducerKeyBackupBundle, ProducerKeyMetadata, ProducerKeystore,
    SecretNamespace,
};
use fp_storage::{
    BackupReport, CloneProjectRequest, CreateProjectRequest, GameProjectDetail, GameProjectRecord,
    GameProjectSummary, IntegrityReport, Jx2MigrationSummary, ProjectAccountInput,
    ProjectAccountRecord, ProjectModuleRecord, ProjectTaskRecord, ProjectWorkflowRecord,
    UpdateProjectRequest, VisionTemplateRecord,
};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::State;

fn parse_namespace(scope: &str, slug: Option<&str>) -> Result<SecretNamespace, String> {
    match scope.trim().to_lowercase().as_str() {
        "project" => {
            let s = slug
                .filter(|v| !v.trim().is_empty())
                .ok_or_else(|| "project_slug is required for scope 'project'".to_string())?;
            Ok(SecretNamespace::Project(s.to_string()))
        }
        "minitool" | "mini_tool" => {
            let s = slug
                .filter(|v| !v.trim().is_empty())
                .ok_or_else(|| "tool_slug is required for scope 'minitool'".to_string())?;
            Ok(SecretNamespace::MiniTool(s.to_string()))
        }
        "account" => {
            let s = slug
                .filter(|v| !v.trim().is_empty())
                .ok_or_else(|| "project_slug is required for scope 'account'".to_string())?;
            Ok(SecretNamespace::Account(s.to_string()))
        }
        "system" => Ok(SecretNamespace::System),
        other => Err(format!(
            "Unsupported vault namespace scope '{}'; expected project|minitool|account|system",
            other
        )),
    }
}

#[tauri::command]
pub fn studio_list_projects(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<GameProjectSummary>, String> {
    let st = state.lock().unwrap();
    st.storage.list_projects().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_get_project_detail(
    project_id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Option<GameProjectDetail>, String> {
    let st = state.lock().unwrap();
    st.storage
        .get_project_detail(&project_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_create_project(
    req: CreateProjectRequest,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<GameProjectRecord, String> {
    let st = state.lock().unwrap();
    st.storage
        .create_project(req, &st.projects_root)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_update_project(
    req: UpdateProjectRequest,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<GameProjectRecord, String> {
    let st = state.lock().unwrap();
    st.storage.update_project(req).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_clone_project(
    req: CloneProjectRequest,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<GameProjectRecord, String> {
    let st = state.lock().unwrap();
    st.storage
        .clone_project(req, &st.projects_root)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_delete_project(
    project_id: String,
    remove_disk_dir: Option<bool>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<bool, String> {
    let st = state.lock().unwrap();
    st.storage
        .delete_project(&project_id, remove_disk_dir.unwrap_or(false))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_list_project_workflows(
    project_id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<ProjectWorkflowRecord>, String> {
    let st = state.lock().unwrap();
    st.storage
        .list_project_workflows(&project_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_save_project_workflow(
    project_id: String,
    workflow_id: String,
    name: String,
    category: Option<String>,
    draft_json: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<ProjectWorkflowRecord, String> {
    let st = state.lock().unwrap();
    st.storage
        .save_project_workflow(
            &project_id,
            &workflow_id,
            &name,
            category.as_deref().unwrap_or("Chung"),
            &draft_json,
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_list_project_templates(
    project_id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<VisionTemplateRecord>, String> {
    let st = state.lock().unwrap();
    st.storage
        .list_vision_templates(&project_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_list_project_tasks(
    project_id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<ProjectTaskRecord>, String> {
    let st = state.lock().unwrap();
    st.storage
        .list_project_tasks(&project_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_upsert_project_task(
    project_id: String,
    task: MiniTaskDef,
    category_meta: TaskCategoryDef,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<ProjectTaskRecord, String> {
    let st = state.lock().unwrap();
    st.storage
        .upsert_project_task(&project_id, task, &category_meta)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_delete_project_task(
    project_id: String,
    task_id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<bool, String> {
    let st = state.lock().unwrap();
    st.storage
        .delete_project_task(&project_id, &task_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_list_project_modules(
    project_id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<ProjectModuleRecord>, String> {
    let st = state.lock().unwrap();
    st.storage
        .list_project_modules(&project_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_upsert_project_module(
    project_id: String,
    module_id: String,
    packaged_in_release: bool,
    enabled_by_default: bool,
    config_json: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<ProjectModuleRecord, String> {
    let st = state.lock().unwrap();
    st.storage
        .upsert_project_module(
            &project_id,
            &module_id,
            packaged_in_release,
            enabled_by_default,
            &config_json,
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_get_ui_schema(
    project_id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Option<MiniToolUiSchema>, String> {
    let st = state.lock().unwrap();
    st.storage
        .get_ui_schema(&project_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_save_ui_schema(
    project_id: String,
    schema: MiniToolUiSchema,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let st = state.lock().unwrap();
    st.storage
        .save_ui_schema(&project_id, &schema)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_list_project_accounts(
    project_id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<ProjectAccountRecord>, String> {
    let st = state.lock().unwrap();
    st.storage
        .list_project_accounts(&project_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_upsert_project_account(
    input: ProjectAccountInput,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<ProjectAccountRecord, String> {
    let st = state.lock().unwrap();
    st.storage
        .upsert_project_account(input)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_vault_set_secret(
    scope: String,
    slug: Option<String>,
    key: String,
    secret: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<String, String> {
    let ns = parse_namespace(&scope, slug.as_deref())?;
    let st = state.lock().unwrap();
    st.vault
        .set_secret_namespaced(&ns, &key, &secret)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_vault_has_secret(
    scope: String,
    slug: Option<String>,
    key: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<bool, String> {
    let ns = parse_namespace(&scope, slug.as_deref())?;
    let st = state.lock().unwrap();
    st.vault
        .get_secret_namespaced(&ns, &key)
        .map(|opt| opt.is_some())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_vault_list_namespace(
    scope: String,
    slug: Option<String>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<CredentialMetadata>, String> {
    let ns = parse_namespace(&scope, slug.as_deref())?;
    let st = state.lock().unwrap();
    st.vault
        .list_secrets_namespaced(&ns)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_vault_delete_secret(
    scope: String,
    slug: Option<String>,
    key: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<bool, String> {
    let ns = parse_namespace(&scope, slug.as_deref())?;
    let st = state.lock().unwrap();
    st.vault
        .delete_secret_namespaced(&ns, &key)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_get_producer_key_status() -> Result<ProducerKeyMetadata, String> {
    let ks_dir = ProducerKeystore::default_external_keystore_dir();
    let (_, meta) = ProducerKeystore::load_or_create_signing_key(&ks_dir)
        .map_err(|e| e.to_string())?;
    Ok(meta)
}

#[tauri::command]
pub fn studio_rotate_producer_key(reason: Option<String>) -> Result<ProducerKeyMetadata, String> {
    let ks_dir = ProducerKeystore::default_external_keystore_dir();
    let (_, meta) = ProducerKeystore::rotate_producer_key(
        &ks_dir,
        reason.as_deref().unwrap_or("studio_ui_rotation"),
    )
    .map_err(|e| e.to_string())?;
    Ok(meta)
}

#[tauri::command]
pub fn studio_backup_producer_key(backup_path: String) -> Result<ProducerKeyBackupBundle, String> {
    let ks_dir = ProducerKeystore::default_external_keystore_dir();
    ProducerKeystore::backup_signing_key(&ks_dir, PathBuf::from(backup_path).as_path())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_restore_producer_key(backup_path: String) -> Result<ProducerKeyMetadata, String> {
    let ks_dir = ProducerKeystore::default_external_keystore_dir();
    ProducerKeystore::restore_signing_key(PathBuf::from(backup_path).as_path(), &ks_dir)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_backup_database(
    dest_path: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<BackupReport, String> {
    let st = state.lock().unwrap();
    st.storage
        .online_backup_to(PathBuf::from(dest_path))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_check_db_integrity(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<IntegrityReport, String> {
    let st = state.lock().unwrap();
    st.storage.check_integrity().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_migrate_legacy_jx2(
    legacy_jx2_dir: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Jx2MigrationSummary, String> {
    let st = state.lock().unwrap();
    st.storage
        .migrate_legacy_jx2_assets(PathBuf::from(legacy_jx2_dir).as_path(), &st.projects_root)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use fp_adb::AdbRunner;
    use fp_security::CredentialVault;
    use fp_storage::StorageManager;

    #[test]
    fn test_tauri_p2_project_crud_and_isolation_layer() {
        let temp_dir = std::env::temp_dir().join(format!(
            "fp_tauri_p2_ipc_test_{}",
            uuid::Uuid::new_v4()
        ));
        let projects_root = temp_dir.join("projects");
        let storage = StorageManager::new_in_memory().unwrap();
        let vault = CredentialVault::new();

        let state = AppState {
            storage,
            adb_runner: AdbRunner::default(),
            vault,
            db_path: ":memory:".to_string(),
            projects_root: projects_root.clone(),
            is_portable: false,
        };

        let p1 = state
            .storage
            .create_project(
                CreateProjectRequest {
                    slug: "ipc-game-one".to_string(),
                    name: "IPC Game One".to_string(),
                    game_package_id: "com.ipc.one".to_string(),
                    version: None,
                    exe_name: None,
                    window_title: None,
                    icon_rel_path: None,
                    target_width_px: None,
                    target_height_px: None,
                    bundle_adb: None,
                    enabled_modules: None,
                },
                &state.projects_root,
            )
            .unwrap();

        let p2 = state
            .storage
            .create_project(
                CreateProjectRequest {
                    slug: "ipc-game-two".to_string(),
                    name: "IPC Game Two".to_string(),
                    game_package_id: "com.ipc.two".to_string(),
                    version: None,
                    exe_name: None,
                    window_title: None,
                    icon_rel_path: None,
                    target_width_px: None,
                    target_height_px: None,
                    bundle_adb: None,
                    enabled_modules: None,
                },
                &state.projects_root,
            )
            .unwrap();

        state
            .storage
            .save_project_workflow(&p1.id, "wf-ipc-1", "IPC WF 1", "NV", "{}")
            .unwrap();
        assert_eq!(state.storage.list_projects().unwrap().len(), 2);
        assert_eq!(
            state.storage.list_project_workflows(&p1.id).unwrap().len(),
            1
        );
        assert_eq!(
            state.storage.list_project_workflows(&p2.id).unwrap().len(),
            0
        );

        let ns1 = parse_namespace("project", Some("ipc-game-one")).unwrap();
        let ns2 = parse_namespace("project", Some("ipc-game-two")).unwrap();
        state
            .vault
            .set_secret_namespaced(&ns1, "test_key", "val1")
            .unwrap();
        assert_eq!(
            state
                .vault
                .get_secret_namespaced(&ns1, "test_key")
                .unwrap()
                .as_deref(),
            Some("val1")
        );
        assert_eq!(
            state.vault.get_secret_namespaced(&ns2, "test_key").unwrap(),
            None
        );
        assert!(state.vault.delete_secret_namespaced(&ns1, "test_key").unwrap());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
