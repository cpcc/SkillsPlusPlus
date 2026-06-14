pub mod commands;
pub mod models;
pub mod repositories;
pub mod services;

use commands::app::DbState;
use repositories::db;
use std::sync::{Arc, Mutex};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        // Victauri: embeds an MCP server (127.0.0.1:7373) in debug builds so an
        // external AI agent (e.g. Claude Code) can drive the app for testing.
        // Fixed token so `claude mcp add` registration survives `tauri dev` restarts.
        // No-op in release builds.
        .plugin(
            victauri_plugin::VictauriBuilder::new()
                .auth_token("dev-secret-unchanging")
                .build()
                .expect("failed to build victauri plugin"),
        )
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join("skills_pp.db");

            let conn = db::open(&db_path).expect("Failed to open database");
            db::migrate(&conn).expect("Failed to run database migrations");
            db::seed_sources(&conn).expect("Failed to seed sources");

            // Best-effort: import pre-existing local skills so they appear in
            // the 「已安装」page without needing a manual install.
            if let Err(e) = services::install::import_existing_skills(&conn) {
                eprintln!("[import_existing_skills] startup import failed: {e}");
            }

            let db_arc = Arc::new(Mutex::new(conn));
            app.manage(DbState(std::sync::Arc::clone(&db_arc)));

            // Start HTTP bridge for browser-based debugging (no-op if port is taken).
            let version = app.package_info().version.to_string();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = services::http_bridge::start("127.0.0.1:3030", db_arc, version).await {
                    log::warn!("HTTP bridge stopped: {e}");
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::get_app_info,
            commands::app::check_app_update,
            commands::app::open_release_url,
            commands::directory::scan_directories,
            commands::directory::list_directories,
            commands::directory::add_directory,
            commands::directory::toggle_directory,
            commands::directory::set_default_directory,
            commands::directory::delete_directory,
            commands::source::list_sources,
            commands::source::toggle_source,
            commands::source::list_skills,
            commands::source::refresh_source,
            commands::source::refresh_all_sources,
            commands::source::get_skill,
            commands::source::fetch_skill_md,
            commands::install::preview_install,
            commands::install::install_skill,
            commands::install::reinstall_skill,
            commands::install::uninstall_skill,
            commands::install::list_installed_skills,
            commands::install::check_git_available,
            commands::install::refresh_installed_skills,
            commands::install::check_skill_update,
            commands::install::read_lockfile,
            commands::install::list_canonical_skills,
            commands::install::import_existing_skills,
            commands::install::open_skill_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
