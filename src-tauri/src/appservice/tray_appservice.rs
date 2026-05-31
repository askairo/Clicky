use crate::service::tray_service;
use crate::{appservice, remember_recent_env};
use tauri::{AppHandle, Emitter, Manager};

pub fn handle_tray_menu_event(app: &AppHandle, menu_id: &str) {
    match menu_id {
        tray_service::MENU_OPEN => tray_service::show_main_window(app),
        tray_service::MENU_QUIT => app.exit(0),
        tray_service::MENU_RECENT_0 => apply_recent_env_by_index(app, 0),
        tray_service::MENU_RECENT_1 => apply_recent_env_by_index(app, 1),
        _ => {}
    }
}

fn apply_recent_env_by_index(app: &AppHandle, index: usize) {
    let state = app.state::<crate::AppRuntimeState>();
    let Some(target) = state.recent_at(index) else {
        let msg = "请先在主界面应用至少一个环境。";
        let _ = app.emit("tray-switch-status", msg);
        tray_service::deliver_tray_feedback(app, msg);
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
            tray_service::sync_tray_menu(app);

            let total = result.variable_results.len();
            let changed = result
                .variable_results
                .iter()
                .filter(|item| item.before.as_deref().unwrap_or("") != item.after.as_str())
                .count();

            let status = format!(
                "已切换到 {}/{}：处理 {} 个，变更 {} 个。",
                target.group, target.env, total, changed
            );
            let _ = app.emit("tray-switch-status", status.clone());
            let _ = app.emit("tray-switched-env", target.clone());
            tray_service::deliver_tray_feedback(app, &status);
        }
        Err(err) => {
            let msg = format!("切换失败：{}", err);
            let _ = app.emit("tray-switch-status", msg.clone());
            tray_service::deliver_tray_feedback(app, &msg);
        }
    }
}
