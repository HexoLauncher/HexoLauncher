use std::sync::Mutex;

use tauri::menu::{Menu, MenuEvent, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, RunEvent, Window, WindowEvent};

use crate::state::AppState;

const MAIN: &str = "main";
const TRAY_ID: &str = "background";
const MENU_SHOW: &str = "background-show";
const MENU_QUIT: &str = "background-quit";
#[cfg(target_os = "macos")]
const MENU_APP_QUIT: &str = "app-quit";

static LOCALE: Mutex<String> = Mutex::new(String::new());

struct Labels {
    tooltip: &'static str,
    show: &'static str,
    quit: &'static str,
}

fn labels() -> Labels {
    let locale = LOCALE.lock().map(|l| l.clone()).unwrap_or_default();
    if locale.starts_with("zh") {
        Labels {
            tooltip: "遊戲執行中，啟動器在背景常駐",
            show: "顯示啟動器",
            quit: "結束啟動器（遊戲會繼續執行）",
        }
    } else {
        Labels {
            tooltip: "A game is running; the launcher stays in the background",
            show: "Show launcher",
            quit: "Quit launcher (games keep running)",
        }
    }
}

#[tauri::command]
pub fn set_ui_locale(locale: String) {
    if let Ok(mut current) = LOCALE.lock() {
        *current = locale;
    }
}

fn has_running(app: &AppHandle) -> bool {
    app.state::<AppState>()
        .running_list()
        .map(|list| !list.is_empty())
        .unwrap_or(false)
}

fn in_background(app: &AppHandle) -> bool {
    app.tray_by_id(TRAY_ID).is_some()
}

fn enter(app: &AppHandle) {
    if in_background(app) {
        return;
    }
    let labels = labels();
    let menu =
        MenuItem::with_id(app, MENU_SHOW, labels.show, true, None::<&str>).and_then(|show| {
            let quit = MenuItem::with_id(app, MENU_QUIT, labels.quit, true, None::<&str>)?;
            Menu::with_items(app, &[&show, &quit])
        });
    let Ok(menu) = menu else {
        return;
    };

    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip(labels.tooltip)
        .menu(&menu)
        .show_menu_on_left_click(false);
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    let _ = tray.build(app);
}

fn leave(app: &AppHandle) {
    let _ = app.remove_tray_by_id(TRAY_ID);
}

/// handle cmd q on macos
#[cfg(target_os = "macos")]
pub fn install_app_menu(app: &AppHandle) -> tauri::Result<()> {
    use tauri::menu::MenuItemKind;

    let menu = Menu::default(app)?;
    if let Some(MenuItemKind::Submenu(app_menu)) = menu.items()?.into_iter().next() {
        let items = app_menu.items()?;
        if let Some(MenuItemKind::Predefined(_)) = items.last() {
            app_menu.remove_at(items.len() - 1)?;
        }
        let quit = MenuItem::with_id(
            app,
            MENU_APP_QUIT,
            format!("Quit {}", app.package_info().name),
            true,
            Some("Cmd+Q"),
        )?;
        app_menu.append(&quit)?;
    }
    app.set_menu(menu)?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn request_quit(app: &AppHandle) {
    if !has_running(app) {
        app.exit(0);
        return;
    }
    enter(app);
    for (label, window) in app.webview_windows() {
        if label == MAIN {
            let _ = window.hide();
        } else {
            let _ = window.close();
        }
    }
}

pub fn show_main(app: &AppHandle) {
    leave(app);
    if let Some(window) = app.get_webview_window(MAIN) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        return;
    }
    let Some(config) = app.config().app.windows.iter().find(|w| w.label == MAIN) else {
        return;
    };
    if let Ok(builder) = tauri::WebviewWindowBuilder::from_config(app, config) {
        let _ = builder.build();
    }
}

pub fn quit_if_idle(app: &AppHandle, closing: Option<&str>) {
    if !in_background(app) || has_running(app) {
        return;
    }
    let visible = app.webview_windows().into_iter().any(|(label, window)| {
        Some(label.as_str()) != closing && window.is_visible().unwrap_or(false)
    });
    if !visible {
        app.exit(0);
    }
}

pub fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    match event.id().as_ref() {
        MENU_SHOW => show_main(app),
        MENU_QUIT => app.exit(0),
        #[cfg(target_os = "macos")]
        MENU_APP_QUIT => request_quit(app),
        _ => {}
    }
}

pub fn on_tray_icon_event(app: &AppHandle, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        id,
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        if id == TRAY_ID {
            show_main(app);
        }
    }
}

pub fn on_window_event(window: &Window, event: &WindowEvent) {
    let app = window.app_handle();
    match event {
        WindowEvent::CloseRequested { api, .. } if window.label() == MAIN && has_running(app) => {
            api.prevent_close();
            let _ = window.hide();
            enter(app);
        }
        WindowEvent::Destroyed if window.label() != MAIN => {
            quit_if_idle(app, Some(window.label()));
        }
        _ => {}
    }
}

pub fn on_run_event(app: &AppHandle, event: RunEvent) {
    match event {
        RunEvent::ExitRequested {
            code: None, api, ..
        } if has_running(app) => {
            api.prevent_exit();
            enter(app);
        }
        #[cfg(target_os = "macos")]
        RunEvent::Reopen { .. } => show_main(app),
        _ => {}
    }
}
