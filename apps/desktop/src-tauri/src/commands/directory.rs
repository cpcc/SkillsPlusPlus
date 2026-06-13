use crate::commands::app::DbState;
use crate::models::DirectoryRow;
use crate::services::directory as dir_svc;
use rusqlite::params;
use tauri::State;
use uuid::Uuid;

/// Seed built-in dirs then scan all enabled directories, returning the updated list.
#[tauri::command]
pub fn scan_directories(db: State<DbState>) -> Result<Vec<DirectoryRow>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    dir_svc::seed_default_directories(&conn).map_err(|e| e.to_string())?;

    let dirs = dir_svc::list_directories(&conn).map_err(|e| e.to_string())?;

    for d in &dirs {
        if d.enabled {
            let result = dir_svc::scan_directory(&d.id, &d.tool_name, &d.path);
            dir_svc::update_scan_result(&conn, &result).map_err(|e| e.to_string())?;
        }
    }

    dir_svc::list_directories(&conn).map_err(|e| e.to_string())
}

/// List all directories without rescanning.
#[tauri::command]
pub fn list_directories(db: State<DbState>) -> Result<Vec<DirectoryRow>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    dir_svc::list_directories(&conn).map_err(|e| e.to_string())
}

/// Add a custom directory and immediately scan it.
#[tauri::command]
pub fn add_directory(
    db: State<DbState>,
    tool_name: String,
    path: String,
) -> Result<DirectoryRow, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let id = Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO ai_tool_directories \
         (id, tool_name, path, is_default, is_detected, writable, enabled, skill_count) \
         VALUES (?1, ?2, ?3, 0, 0, 0, 1, 0)",
        params![id, tool_name, path],
    )
    .map_err(|e| e.to_string())?;

    let result = dir_svc::scan_directory(&id, &tool_name, &path);
    dir_svc::update_scan_result(&conn, &result).map_err(|e| e.to_string())?;

    Ok(DirectoryRow {
        id,
        tool_name,
        path,
        is_default: false,
        is_detected: result.exists,
        writable: result.writable,
        enabled: true,
        skill_count: result.skill_count,
    })
}

/// Enable or disable a directory.
#[tauri::command]
pub fn toggle_directory(db: State<DbState>, id: String, enabled: bool) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE ai_tool_directories SET enabled = ?1 WHERE id = ?2",
        params![enabled as i64, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Set a directory as the default for its AI tool.
#[tauri::command]
pub fn set_default_directory(db: State<DbState>, id: String) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    let tool_name: String = conn
        .query_row(
            "SELECT tool_name FROM ai_tool_directories WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE ai_tool_directories SET is_default = 0 WHERE tool_name = ?1",
        params![tool_name],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE ai_tool_directories SET is_default = 1 WHERE id = ?1",
        params![id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Delete a directory entry.
#[tauri::command]
pub fn delete_directory(db: State<DbState>, id: String) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM ai_tool_directories WHERE id = ?1",
        params![id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
