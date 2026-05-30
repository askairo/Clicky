use crate::appservice;
use crate::domain::{
    ApplyResult, EnvSummary, ExportRequest, ExportResult, GroupSummary, ImportRequest, ImportSummary,
};
use std::collections::HashMap;
use tauri::State;

#[tauri::command]
pub fn list_groups() -> Result<Vec<GroupSummary>, String> {
    appservice::list_groups()
}

#[tauri::command]
pub fn create_group(group_name: String) -> Result<(), String> {
    appservice::create_group(group_name)
}

#[tauri::command]
pub fn rename_group(old_name: String, new_name: String) -> Result<(), String> {
    appservice::rename_group(old_name, new_name)
}

#[tauri::command]
pub fn delete_group(group_name: String) -> Result<(), String> {
    appservice::delete_group(group_name)
}

#[tauri::command]
pub fn rename_environment(group_name: String, old_name: String, new_name: String) -> Result<(), String> {
    appservice::rename_environment(group_name, old_name, new_name)
}

#[tauri::command]
pub fn delete_environment(group_name: String, env_name: String) -> Result<(), String> {
    appservice::delete_environment(group_name, env_name)
}

#[tauri::command]
pub fn export_config(req: ExportRequest) -> Result<ExportResult, String> {
    appservice::export_config_flow(req)
}

#[tauri::command]
pub fn preview_import_config(req: ImportRequest) -> Result<ImportSummary, String> {
    appservice::preview_import_config_flow(req)
}

#[tauri::command]
pub fn import_config(req: ImportRequest) -> Result<ImportSummary, String> {
    appservice::import_config_flow(req)
}

#[tauri::command]
pub fn list_environments(group_name: String) -> Result<Vec<EnvSummary>, String> {
    appservice::list_environments(group_name)
}

#[tauri::command]
pub fn get_environment_variables(group_name: String, env_name: String) -> Result<HashMap<String, String>, String> {
    appservice::get_environment_variables(group_name, env_name)
}

#[tauri::command]
pub fn detect_active_environments(group_name: String) -> Result<Vec<String>, String> {
    appservice::detect_active_environments(group_name)
}

#[tauri::command]
pub fn save_environment_variables(
    group_name: String,
    env_name: String,
    variables: HashMap<String, String>,
) -> Result<(), String> {
    appservice::save_environment_variables(group_name, env_name, variables)
}

#[tauri::command]
pub fn apply_environment(
    group_name: String,
    env_name: String,
    mode: String,
    state: State<'_, crate::AppRuntimeState>,
) -> Result<ApplyResult, String> {
    let result = appservice::apply_environment_flow(group_name.clone(), env_name.clone(), mode);
    if result.is_ok() {
        crate::remember_recent_env(&state, &group_name, &env_name);
    }
    result
}
