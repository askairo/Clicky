mod appservice;
mod controller;
mod domain;
mod service;

use crate::service::storage_service;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, WindowEvent};
use tauri_plugin_notification::NotificationExt;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnvSelection {
    pub group: String,
    pub env: String,
}

#[derive(Default)]
pub struct AppRuntimeState {
    recent: Mutex<VecDeque<EnvSelection>>,
}

impl AppRuntimeState {
    fn recent_file_path() -> Result<PathBuf, String> {
        let db = storage_service::db_path()?;
        let base = db
            .parent()
            .ok_or_else(|| "failed to resolve clicky data directory".to_string())?;
        Ok(base.join("recent_envs.json"))
    }

    fn load_from_disk(&self) -> Result<(), String> {
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

    fn save_to_disk(&self) -> Result<(), String> {
        let path = Self::recent_file_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create recent env directory {}: {}", parent.display(), e))?;
        }
        let recent = self.recent.lock().expect("recent selections lock poisoned");
        let snapshot = recent.iter().take(2).cloned().collect::<Vec<_>>();
        let content = serde_json::to_string_pretty(&snapshot)
            .map_err(|e| format!("failed to serialize recent envs: {}", e))?;
        fs::write(&path, content)
            .map_err(|e| format!("failed to write recent env file {}: {}", path.display(), e))
    }

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

    fn switch_target(&self) -> Option<EnvSelection> {
        let recent = self.recent.lock().expect("recent selections lock poisoned");
        if recent.len() < 2 {
            return None;
        }
        recent.get(1).cloned()
    }

}

pub fn remember_recent_env(state: &AppRuntimeState, group: &str, env: &str) {
    state.remember(group, env);
}

fn notify(app: &AppHandle, body: &str) {
    let _ = app
        .notification()
        .builder()
        .title("Clicky")
        .body(body)
        .show();
}

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

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn switch_recent_env(app: &AppHandle) {
    let state = app.state::<AppRuntimeState>();
    let Some(target) = state.switch_target() else {
        let msg = "请先应用最近用过的两个环境。";
        let _ = app.emit("tray-switch-status", msg);
        notify(app, msg);
        return;
    };

    let applied = appservice::apply_environment_flow(
        target.group.clone(),
        target.env.clone(),
        "persistent".to_string(),
    );

    match applied {
        Ok(result) => {
            remember_recent_env(&state, &target.group, &target.env);
            let total = result.variable_results.len();
            let changed = result
                .variable_results
                .iter()
                .filter(|item| item.before.as_deref().unwrap_or("") != item.after.as_str())
                .count();
            let _ = app.emit(
                "tray-switch-status",
                format!(
                    "托盘已切换到 {}/{}：处理 {} 个，实际变更 {} 个。",
                    target.group, target.env, total, changed
                ),
            );
            notify(
                app,
                &format!(
                    "已切换到 {}/{}：处理 {} 个，实际变更 {} 个。",
                    target.group, target.env, total, changed
                ),
            );
        }
        Err(err) => {
            let msg = format!("托盘切换失败：{}", err);
            let _ = app.emit("tray-switch-status", msg.clone());
            notify(app, &msg);
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if let Err(e) = init_storage() {
        eprintln!("storage initialization failed: {}", e);
    }

    tauri::Builder::default()
        .manage(AppRuntimeState::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let state = app.state::<AppRuntimeState>();
            let _ = state.load_from_disk();

            let open_item = MenuItem::with_id(app, "open", "打开 Clicky", true, None::<&str>)?;
            let switch_item = MenuItem::with_id(app, "switch_recent", "切换最近环境", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&open_item, &switch_item, &quit_item])?;

            let mut tray_builder = TrayIconBuilder::new()
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |tray, event| match event.id.as_ref() {
                    "open" => show_main_window(tray.app_handle()),
                    "quit" => tray.app_handle().exit(0),
                    "switch_recent" => switch_recent_env(tray.app_handle()),
                    _ => {}
                });

            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }

            tray_builder.build(app)?;

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
            controller::commands::apply_environment
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
