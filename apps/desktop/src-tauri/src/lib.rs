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

            app.manage(DbState(Mutex::new(conn)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![commands::app::get_app_info])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
