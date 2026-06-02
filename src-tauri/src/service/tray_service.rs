use crate::{AppRuntimeState, EnvSelection};
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

pub const TRAY_ID: &str = "clicky-tray";
pub const MENU_OPEN: &str = "open";
pub const MENU_QUIT: &str = "quit";
pub const MENU_RECENT_0: &str = "recent_0";
pub const MENU_RECENT_1: &str = "recent_1";

/// Builds a short label for one tray quick-switch slot.
fn format_recent_menu_label(item: Option<&EnvSelection>, index: usize) -> String {
    match item {
        Some(env) => format!("{}/{}", env.group, env.env),
        None => format!("最近环境 {}（暂无）", index + 1),
    }
}

fn build_tray_menu(
    app: &AppHandle,
    state: &AppRuntimeState,
) -> Result<Menu<tauri::Wry>, tauri::Error> {
    let recent = state.recent_snapshot();
    let current = recent.first();
    let first = recent.first();
    let second = recent.get(1);

    let open_item = MenuItem::with_id(app, MENU_OPEN, "打开 Clicky", true, None::<&str>)?;
    let recent_0_item = CheckMenuItem::with_id(
        app,
        MENU_RECENT_0,
        format_recent_menu_label(first, 0).as_str(),
        first.is_some(),
        current == first,
        None::<&str>,
    )?;
    let recent_1_item = CheckMenuItem::with_id(
        app,
        MENU_RECENT_1,
        format_recent_menu_label(second, 1).as_str(),
        second.is_some(),
        current == second,
        None::<&str>,
    )?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, MENU_QUIT, "退出", true, None::<&str>)?;

    Menu::with_items(
        app,
        &[
            &open_item,
            &separator,
            &recent_0_item,
            &recent_1_item,
            &separator,
            &quit_item,
        ],
    )
}

/// Rebuilds the tray menu after the recent-env cache changes.
pub fn sync_tray_menu(app: &AppHandle) {
    let state = app.state::<AppRuntimeState>();
    let Ok(menu) = build_tray_menu(app, &state) else {
        return;
    };
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_menu(Some(menu));
    }
}

/// Shows and focuses the existing main window instead of opening a duplicate.
pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Sends a short notification used by tray-triggered actions.
pub fn deliver_tray_feedback(app: &AppHandle, body: &str) {
    let _ = app
        .notification()
        .builder()
        .title("Clicky")
        .body(body)
        .show();
}

/// Creates the tray icon and wires the menu click handlers.
pub fn setup_tray(app: &mut tauri::App) -> Result<(), tauri::Error> {
    let state = app.state::<AppRuntimeState>();
    let app_handle = app.handle().clone();
    let tray_menu = build_tray_menu(&app_handle, &state)?;

    let mut tray_builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&tray_menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |tray, event| {
            crate::appservice::handle_tray_menu_event(tray.app_handle(), event.id.as_ref())
        });

    if let Some(icon) = app.default_window_icon() {
        tray_builder = tray_builder.icon(icon.clone());
    }

    tray_builder.build(app)?;
    Ok(())
}
