#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod error;
mod state;
mod store;

use tauri::Manager;

use state::AppState;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(AppState::default())
        .setup(|app| {
            if let Ok(dir) = store::store_dir() {
                let _ = app.asset_protocol_scope().allow_directory(&dir, false);
            }
            if let Ok(dir) = commands::schematic::block_assets_dir() {
                let _ = std::fs::create_dir_all(&dir);
                let _ = app.asset_protocol_scope().allow_directory(&dir, false);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            store::launcher::get_launcher_config,
            store::launcher::save_launcher_config,
            store::launcher::resolve_paths,
            commands::misc::prepare_base_dir,
            commands::misc::open_external,
            commands::misc::open_launcher_folder,
            commands::theme::set_background_image,
            commands::theme::background_image_path,
            commands::theme::clear_background_image,
            store::accounts::list_accounts,
            store::accounts::add_offline_account,
            store::accounts::remove_account,
            store::accounts::select_account,
            store::accounts::refresh_account,
            commands::auth::ms_login_start,
            commands::auth::ms_login_poll,
            commands::auth::ms_login_cancel,
            commands::version::list_versions,
            commands::version::list_loader_versions,
            commands::install::install_instance,
            commands::install::repair_instance,
            commands::instance::list_instances,
            commands::instance::create_instance,
            commands::instance::get_instance,
            commands::instance::save_instance_meta,
            commands::instance::delete_instance,
            commands::instance::duplicate_instance,
            commands::instance::suggest_instance_id,
            commands::instance::open_instance_folder,
            commands::icon::read_image_file,
            commands::icon::set_instance_icon_image,
            commands::skin::player_skin,
            commands::export::export_instance,
            commands::export::import_instance,
            commands::foreign::detect_foreign_launchers,
            commands::foreign::list_foreign_instances,
            commands::foreign::import_foreign_instances,
            commands::modpack::inspect_modpack,
            commands::modpack::install_modpack_file,
            commands::modpack::update_modpack_file,
            commands::modpack_browse::search_modpacks,
            commands::modpack_browse::list_modpack_versions,
            commands::modpack_browse::get_modpack_detail,
            commands::modpack_browse::install_remote_modpack,
            commands::content::search_content,
            commands::content::list_content_versions,
            commands::content::get_content_detail,
            commands::content::install_content,
            commands::content::install::prepare_mod_install,
            commands::content::install::commit_mod_install,
            commands::content::install::discard_mod_install,
            commands::java::detect_java,
            commands::java::detect_java_candidates,
            commands::java::test_java,
            commands::java::install_java,
            commands::java::ensure_java,
            commands::launch::launch_instance,
            commands::launch::check_instance_java,
            commands::launch::stop_instance,
            commands::launch::running_instances,
            commands::launch::game_log,
            commands::launch::clear_game_log,
            commands::mods::list_mods,
            commands::mods::mod_details,
            commands::mods::set_mod_enabled,
            commands::mods::check_mod_updates,
            commands::mods::apply_mod_update,
            commands::mods::delete_mod,
            commands::assets::list_resource_packs,
            commands::assets::list_shader_packs,
            commands::assets::list_worlds,
            commands::assets::set_active_shader,
            commands::assets::delete_pack,
            commands::assets::open_world_folder,
            commands::assets::delete_world,
            commands::assets::list_servers,
            commands::assets::list_screenshots,
            commands::assets::list_instance_logs,
            commands::assets::read_instance_log,
            commands::blueprint::blueprint_session,
            commands::blueprint::blueprint_terms,
            commands::blueprint::blueprint_login,
            commands::blueprint::blueprint_logout,
            commands::blueprint::blueprint_tags,
            commands::blueprint::blueprint_search,
            commands::blueprint::blueprint_get,
            commands::blueprint::blueprint_install,
            commands::blueprint::blueprint_save_as,
            commands::blueprint::blueprint_preview,
            commands::blueprint::blueprint_upload,
            commands::blueprint::blueprint_update,
            commands::blueprint::blueprint_delete,
            commands::blueprint::blueprint_rerender,
            commands::blueprint::blueprint_add_images,
            commands::blueprint::blueprint_delete_image,
            commands::blueprint::blueprint_set_cover,
            commands::schematic::list_schematics,
            commands::schematic::delete_schematic,
            commands::schematic::import_schematic,
            commands::schematic::read_schematic,
            commands::schematic::local_block_assets,
            commands::schematic::pack_block_assets,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
