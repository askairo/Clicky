mod appservice;
mod controller;
mod domain;
mod service;

use crate::service::storage_service;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager, RunEvent, WindowEvent};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnvSelection {
    pub group: String,
    pub env: String,
}

/// Keeps a short most-recently-used list for tray quick actions.
#[derive(Default)]
pub struct AppRuntimeState {
    recent: Mutex<VecDeque<EnvSelection>>,
}

impl AppRuntimeState {
    /// Stores the recent-env cache beside the local SQLite database.
    fn recent_file_path() -> Result<PathBuf, String> {
        let db = storage_service::db_path()?;
        let base = db
            .parent()
            .ok_or_else(|| "failed to resolve clicky data directory".to_string())?;
        Ok(base.join("recent_envs.json"))
    }

    /// Reloads the recent-env cache on startup.
    pub(crate) fn load_from_disk(&self) -> Result<(), String> {
        let path = Self::recent_file_path()?;
        if !path.exists() {
            return Ok(());
        }
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("failed to read recent env file {}: {}", path.display(), e))?;
        let list = serde_json::from_str::<Vec<EnvSelection>>(&content)
            .map_err(|e| format!("failed to parse recent env file {}: {}", path.display(), e))?;

        let mut recent = self.recent.lock().expect("recent selections lock poisoned");
        recent.clear();
        for item in list.into_iter().take(2) {
            recent.push_back(item);
        }
        Ok(())
    }

    /// Best-effort persistence for tray history updates.
    fn save_to_disk(&self) -> Result<(), String> {
        let path = Self::recent_file_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "failed to create recent env directory {}: {}",
                    parent.display(),
                    e
                )
            })?;
        }
        let recent = self.recent.lock().expect("recent selections lock poisoned");
        let snapshot = recent.iter().take(2).cloned().collect::<Vec<_>>();
        let content = serde_json::to_string_pretty(&snapshot)
            .map_err(|e| format!("failed to serialize recent envs: {}", e))?;
        fs::write(&path, content)
            .map_err(|e| format!("failed to write recent env file {}: {}", path.display(), e))
    }

    /// Moves the selected environment to the front of the recent list.
    fn remember(&self, group: &str, env: &str) {
        let mut recent = self.recent.lock().expect("recent selections lock poisoned");
        recent.retain(|item| !(item.group == group && item.env == env));
        recent.push_front(EnvSelection {
            group: group.to_string(),
            env: env.to_string(),
        });
        while recent.len() > 2 {
            recent.pop_back();
        }
        drop(recent);
        let _ = self.save_to_disk();
    }

    /// Returns the Nth recent environment, if available.
    pub(crate) fn recent_at(&self, index: usize) -> Option<EnvSelection> {
        let recent = self.recent.lock().expect("recent selections lock poisoned");
        recent.get(index).cloned()
    }

    /// Returns the subset used to render the tray menu.
    pub(crate) fn recent_snapshot(&self) -> Vec<EnvSelection> {
        let recent = self.recent.lock().expect("recent selections lock poisoned");
        recent.iter().take(2).cloned().collect()
    }
}

/// Records the latest environment selection for tray reuse.
pub fn remember_recent_env(state: &AppRuntimeState, group: &str, env: &str) {
    state.remember(group, env);
}

/// Returns the most recent environment selection, if one exists.
pub fn current_recent_env(state: &AppRuntimeState) -> Option<EnvSelection> {
    state.recent_at(0)
}

/// Creates local storage on first launch and imports legacy YAML when available.
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
/// Starts the Tauri application and wires the tray, plugins, and commands.
pub fn run() {
    if let Err(e) = service::log_service::init() {
        eprintln!("failed to initialize logger: {}", e);
    } else {
        info!("logger initialized");
    }

    if let Err(e) = init_storage() {
        error!("storage initialization failed: {}", e);
    }

    tauri::Builder::default()
        .manage(AppRuntimeState::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let state = app.state::<AppRuntimeState>();
            let _ = state.load_from_disk();
            service::tray_service::setup_tray(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
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
            controller::commands::apply_environment,
            controller::commands::get_current_env_selection,
            controller::commands::get_runtime_capabilities
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            #[cfg(target_os = "macos")]
            if let RunEvent::Reopen { has_visible_windows, .. } = event {
                if !has_visible_windows {
                    service::tray_service::show_main_window(app);
                }
            }
        });
}
