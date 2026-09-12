// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod error;
mod state;

use state::AppState;

fn main() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            // 目錄與雜項
            commands::misc::default_base_dir,
            commands::misc::prepare_base_dir,
            commands::misc::open_external,
            // 版本
            commands::version::list_versions,
            commands::version::list_loader_versions,
            // 安裝
            commands::install::install_instance,
            // instance 管理
            commands::instance::list_instances,
            commands::instance::get_instance,
            commands::instance::delete_instance,
            commands::instance::open_instance_folder,
            // Java
            commands::java::detect_java,
            commands::java::install_java,
            commands::java::ensure_java,
            // 啟動
            commands::launch::launch_instance,
            commands::launch::stop_instance,
            commands::launch::running_instances,
            // Microsoft 驗證
            commands::auth::ms_login_start,
            commands::auth::ms_login_poll,
            commands::auth::ms_login_cancel,
            commands::auth::ms_login_refresh,
            // 模組
            commands::mods::list_mods,
            commands::mods::check_mod_updates,
            commands::mods::apply_mod_update,
            commands::mods::delete_mod,
            // 資源包 / 光影 / 存檔
            commands::assets::list_resource_packs,
            commands::assets::list_shader_packs,
            commands::assets::list_worlds,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
