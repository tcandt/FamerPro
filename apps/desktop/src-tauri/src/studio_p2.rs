use crate::AppState;
use fp_bundle_contract::{MiniTaskDef, MiniToolUiSchema, TaskCategoryDef};
use fp_security::{
    CredentialMetadata, PortableProducerKeyRecoveryBundle, ProducerKeyBackupBundle,
    ProducerKeyMetadata, ProducerKeystore, SecretNamespace,
};
use fp_storage::{
    BackupReport, CloneProjectRequest, CreateProjectRequest, DbOptimizeReport, GameProjectDetail,
    GameProjectRecord, GameProjectSummary, IntegrityReport, Jx2MigrationSummary,
    ProjectAccountInput, ProjectAccountRecord, ProjectModuleRecord, ProjectTaskRecord,
    ProjectWorkflowRecord, UpdateProjectRequest, VisionTemplateRecord,
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

pub(crate) fn handle_list_projects(st: &AppState) -> Result<Vec<GameProjectSummary>, String> {
    st.storage.list_projects().map_err(|e| e.to_string())
}

pub(crate) fn handle_get_project_detail(
    st: &AppState,
    project_id: &str,
) -> Result<Option<GameProjectDetail>, String> {
    st.storage
        .get_project_detail(project_id)
        .map_err(|e| e.to_string())
}

pub(crate) fn handle_create_project(
    st: &AppState,
    req: CreateProjectRequest,
) -> Result<GameProjectRecord, String> {
    st.storage
        .create_project(req, &st.projects_root)
        .map_err(|e| e.to_string())
}

pub(crate) fn handle_update_project(
    st: &AppState,
    req: UpdateProjectRequest,
) -> Result<GameProjectRecord, String> {
    st.storage.update_project(req).map_err(|e| e.to_string())
}

pub(crate) fn handle_clone_project(
    st: &AppState,
    req: CloneProjectRequest,
) -> Result<GameProjectRecord, String> {
    st.storage
        .clone_project(req, &st.projects_root)
        .map_err(|e| e.to_string())
}

pub(crate) fn handle_delete_project(
    st: &AppState,
    project_id: &str,
    remove_disk_dir: bool,
) -> Result<bool, String> {
    st.storage
        .delete_project(project_id, remove_disk_dir)
        .map_err(|e| e.to_string())
}

pub(crate) fn handle_vault_set_secret(
    st: &AppState,
    scope: &str,
    slug: Option<&str>,
    key: &str,
    secret: &str,
) -> Result<String, String> {
    let ns = parse_namespace(scope, slug)?;
    st.vault
        .set_secret_namespaced(&ns, key, secret)
        .map_err(|e| e.to_string())
}

pub(crate) fn handle_vault_has_secret(
    st: &AppState,
    scope: &str,
    slug: Option<&str>,
    key: &str,
) -> Result<bool, String> {
    let ns = parse_namespace(scope, slug)?;
    st.vault
        .get_secret_namespaced(&ns, key)
        .map(|opt| opt.is_some())
        .map_err(|e| e.to_string())
}

pub(crate) fn handle_vault_list_namespace(
    st: &AppState,
    scope: &str,
    slug: Option<&str>,
) -> Result<Vec<CredentialMetadata>, String> {
    let ns = parse_namespace(scope, slug)?;
    st.vault
        .list_secrets_namespaced(&ns)
        .map_err(|e| e.to_string())
}

pub(crate) fn handle_vault_delete_secret(
    st: &AppState,
    scope: &str,
    slug: Option<&str>,
    key: &str,
) -> Result<bool, String> {
    let ns = parse_namespace(scope, slug)?;
    st.vault
        .delete_secret_namespaced(&ns, key)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_list_projects(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<GameProjectSummary>, String> {
    let st = state.lock().unwrap();
    handle_list_projects(&st)
}

#[tauri::command]
pub fn studio_get_project_detail(
    project_id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Option<GameProjectDetail>, String> {
    let st = state.lock().unwrap();
    handle_get_project_detail(&st, &project_id)
}

#[tauri::command]
pub fn studio_create_project(
    req: CreateProjectRequest,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<GameProjectRecord, String> {
    let st = state.lock().unwrap();
    handle_create_project(&st, req)
}

#[tauri::command]
pub fn studio_update_project(
    req: UpdateProjectRequest,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<GameProjectRecord, String> {
    let st = state.lock().unwrap();
    handle_update_project(&st, req)
}

#[tauri::command]
pub fn studio_clone_project(
    req: CloneProjectRequest,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<GameProjectRecord, String> {
    let st = state.lock().unwrap();
    handle_clone_project(&st, req)
}

#[tauri::command]
pub fn studio_delete_project(
    project_id: String,
    remove_disk_dir: Option<bool>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<bool, String> {
    let st = state.lock().unwrap();
    handle_delete_project(&st, &project_id, remove_disk_dir.unwrap_or(false))
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
    let st = state.lock().unwrap();
    handle_vault_set_secret(&st, &scope, slug.as_deref(), &key, &secret)
}

#[tauri::command]
pub fn studio_vault_has_secret(
    scope: String,
    slug: Option<String>,
    key: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<bool, String> {
    let st = state.lock().unwrap();
    handle_vault_has_secret(&st, &scope, slug.as_deref(), &key)
}

#[tauri::command]
pub fn studio_vault_list_namespace(
    scope: String,
    slug: Option<String>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<CredentialMetadata>, String> {
    let st = state.lock().unwrap();
    handle_vault_list_namespace(&st, &scope, slug.as_deref())
}

#[tauri::command]
pub fn studio_vault_delete_secret(
    scope: String,
    slug: Option<String>,
    key: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<bool, String> {
    let st = state.lock().unwrap();
    handle_vault_delete_secret(&st, &scope, slug.as_deref(), &key)
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

pub(crate) fn handle_export_portable_producer_key(
    keystore_dir: &std::path::Path,
    backup_path: &std::path::Path,
    passphrase: &str,
) -> Result<PortableProducerKeyRecoveryBundle, String> {
    ProducerKeystore::export_portable_recovery_bundle(keystore_dir, backup_path, passphrase)
        .map_err(|e| e.to_string())
}

pub(crate) fn handle_import_portable_producer_key(
    keystore_dir: &std::path::Path,
    backup_path: &std::path::Path,
    passphrase: &str,
) -> Result<ProducerKeyMetadata, String> {
    ProducerKeystore::import_portable_recovery_bundle(backup_path, keystore_dir, passphrase)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn studio_export_portable_producer_key(
    backup_path: String,
    passphrase: String,
) -> Result<PortableProducerKeyRecoveryBundle, String> {
    let ks_dir = ProducerKeystore::default_external_keystore_dir();
    handle_export_portable_producer_key(&ks_dir, PathBuf::from(backup_path).as_path(), &passphrase)
}

#[tauri::command]
pub fn studio_import_portable_producer_key(
    backup_path: String,
    passphrase: String,
) -> Result<ProducerKeyMetadata, String> {
    let ks_dir = ProducerKeystore::default_external_keystore_dir();
    handle_import_portable_producer_key(&ks_dir, PathBuf::from(backup_path).as_path(), &passphrase)
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
pub fn studio_optimize_db(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<DbOptimizeReport, String> {
    let st = state.lock().unwrap();
    st.storage.optimize_wal_and_vacuum().map_err(|e| e.to_string())
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
        let storage = Arc::new(StorageManager::new_in_memory().unwrap());
        let vault = CredentialVault::new();
        let coordinator = fp_workflow_runtime::coordinator::RuntimeCoordinator::new(
            Arc::clone(&storage),
            PathBuf::from("adb"),
        );

        let state = AppState {
            storage,
            adb_runner: AdbRunner::default(),
            coordinator,
            vault,
            db_path: ":memory:".to_string(),
            projects_root: projects_root.clone(),
            is_portable: false,
        };

        let req1: CreateProjectRequest = serde_json::from_value(serde_json::json!({
            "slug": "ipc-game-one",
            "name": "IPC Game One",
            "gamePackageId": "com.ipc.one"
        }))
        .unwrap();
        let p1 = handle_create_project(&state, req1).unwrap();

        let req2: CreateProjectRequest = serde_json::from_value(serde_json::json!({
            "slug": "ipc-game-two",
            "name": "IPC Game Two",
            "gamePackageId": "com.ipc.two"
        }))
        .unwrap();
        let p2 = handle_create_project(&state, req2).unwrap();

        let upd_req: UpdateProjectRequest = serde_json::from_value(serde_json::json!({
            "id": p1.id,
            "version": "1.1.0",
            "name": "IPC Game One Updated"
        }))
        .unwrap();
        let p1_upd = handle_update_project(&state, upd_req).unwrap();
        assert_eq!(p1_upd.version, "1.1.0");
        let ui_synced = state.storage.get_ui_schema(&p1.id).unwrap().unwrap();
        assert_eq!(ui_synced.version_badge, "v1.1.0");

        state
            .storage
            .save_project_workflow(&p1.id, "wf-ipc-1", "IPC WF 1", "NV", "{}")
            .unwrap();
        let summaries = handle_list_projects(&state).unwrap();
        assert_eq!(summaries.len(), 2);
        let p1_detail = handle_get_project_detail(&state, &p1.id).unwrap().unwrap();
        assert_eq!(p1_detail.workflows.len(), 1);
        let p2_detail = handle_get_project_detail(&state, &p2.id).unwrap().unwrap();
        assert_eq!(p2_detail.workflows.len(), 0);

        let clone_req: CloneProjectRequest = serde_json::from_value(serde_json::json!({
            "sourceProjectId": p1.id,
            "newSlug": "ipc-game-one-clone",
            "newName": "IPC Game One Clone"
        }))
        .unwrap();
        let p1_clone = handle_clone_project(&state, clone_req).unwrap();
        assert_eq!(handle_list_projects(&state).unwrap().len(), 3);
        assert!(handle_delete_project(&state, &p1_clone.id, true).unwrap());
        assert_eq!(handle_list_projects(&state).unwrap().len(), 2);

        // Vault IPC handler tests (including error paths)
        assert!(parse_namespace("invalid_scope", None).is_err());
        assert!(parse_namespace("project", None).is_err());
        let uri = handle_vault_set_secret(&state, "project", Some("ipc-game-one"), "test_key", "val1")
            .unwrap();
        assert!(uri.contains("FarmerPro/project/ipc-game-one/test_key"));
        assert!(handle_vault_has_secret(&state, "project", Some("ipc-game-one"), "test_key").unwrap());
        assert!(!handle_vault_has_secret(&state, "project", Some("ipc-game-two"), "test_key").unwrap());
        assert_eq!(
            handle_vault_list_namespace(&state, "project", Some("ipc-game-one"))
                .unwrap()
                .len(),
            1
        );
        assert!(handle_vault_delete_secret(&state, "project", Some("ipc-game-one"), "test_key").unwrap());

        // Portable producer key export/import IPC handler tests
        let ks_src = temp_dir.join("ks_src");
        let ks_dst = temp_dir.join("ks_dst");
        let recovery_file = temp_dir.join("portable_recovery.json");
        let (_, meta_src) = ProducerKeystore::load_or_create_signing_key(&ks_src).unwrap();
        let exported = handle_export_portable_producer_key(
            &ks_src,
            &recovery_file,
            "Studio-IPC-Passphrase-2026",
        )
        .unwrap();
        assert_eq!(exported.key_id, meta_src.key_id);
        assert!(handle_import_portable_producer_key(&ks_dst, &recovery_file, "WrongPass").is_err());
        let imported = handle_import_portable_producer_key(
            &ks_dst,
            &recovery_file,
            "Studio-IPC-Passphrase-2026",
        )
        .unwrap();
        assert_eq!(imported.key_id, meta_src.key_id);
        assert_eq!(imported.public_key_hex, meta_src.public_key_hex);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
