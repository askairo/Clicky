mod appservice;
mod controller;
mod domain;
mod service;

use crate::service::storage_service;

fn init_storage() -> Result<(), String> {
    let path = storage_service::db_path()?;
    let db_exists = path.exists();

    let mut conn = storage_service::open_db()?;
    storage_service::ensure_db_schema(&conn)?;

    if db_exists {
        return Ok(());
    }

    if let Ok(yaml_cfg) = storage_service::load_config_from_yaml() {
        storage_service::save_config_to_db(&mut conn, &yaml_cfg)?;
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if let Err(e) = init_storage() {
        eprintln!("storage initialization failed: {}", e);
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            controller::commands::list_groups,
            controller::commands::create_group,
            controller::commands::rename_group,
            controller::commands::delete_group,
            controller::commands::rename_environment,
            controller::commands::delete_environment,
            controller::commands::export_config,
            controller::commands::preview_import_config,
            controller::commands::import_config,
            controller::commands::list_environments,
            controller::commands::get_environment_variables,
            controller::commands::detect_active_environments,
            controller::commands::save_environment_variables,
            controller::commands::apply_environment
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
