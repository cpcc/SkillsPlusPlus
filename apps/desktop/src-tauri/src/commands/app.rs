use crate::models::AppInfo;
use rusqlite::Connection;
use std::sync::Mutex;
use tauri::{Manager, State};

pub struct DbState(pub Mutex<Connection>);

#[tauri::command]
pub fn get_app_info(app: tauri::AppHandle, db: State<DbState>) -> Result<AppInfo, String> {
    let version = app.package_info().version.to_string();
    let db_path = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("skills_pp.db")
        .to_string_lossy()
        .to_string();

    // verify DB is accessible
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let _: i64 = conn
        .query_row("SELECT COUNT(*) FROM app_settings", [], |row| row.get(0))
        .unwrap_or(0);

    Ok(AppInfo {
        version,
        db_path,
        log_path: String::from("(see app data dir)"),
        platform: std::env::consts::OS.to_string(),
    })
}
