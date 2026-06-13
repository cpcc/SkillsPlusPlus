pub mod commands;
pub mod models;
pub mod repositories;
pub mod services;

use commands::app::DbState;
use repositories::db;
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join("skills_pp.db");

            let conn = db::open(&db_path).expect("Failed to open database");
            db::migrate(&conn).expect("Failed to run database migrations");
            db::seed_sources(&conn).expect("Failed to seed sources");

            app.manage(DbState(Mutex::new(conn)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::get_app_info,
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
            commands::install::preview_install,
            commands::install::install_skill,
            commands::install::reinstall_skill,
            commands::install::uninstall_skill,
            commands::install::list_installed_skills,
            commands::install::list_install_tasks,
            commands::install::check_git_available,
            commands::install::refresh_installed_skills,
            commands::install::check_skill_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
