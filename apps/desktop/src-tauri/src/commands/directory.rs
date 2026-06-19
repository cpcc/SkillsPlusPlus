use crate::commands::app::DbState;
use crate::models::{DirectoryRow, FileTreeNode};
use crate::services::directory as dir_svc;
use rusqlite::{params, Connection};
use std::path::Path;
use tauri::State;
use uuid::Uuid;

/// Seed built-in dirs then scan all enabled directories, returning the updated list.
pub fn scan_directories_inner(conn: &Connection) -> Result<Vec<DirectoryRow>, String> {
    dir_svc::seed_default_directories(conn).map_err(|e| e.to_string())?;
    let dirs = dir_svc::list_directories(conn).map_err(|e| e.to_string())?;
    for d in &dirs {
        if d.enabled {
            let result = dir_svc::scan_directory(&d.id, &d.tool_name, &d.path);
            dir_svc::update_scan_result(conn, &result).map_err(|e| e.to_string())?;
        }
    }
    dir_svc::list_directories(conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn scan_directories(db: State<DbState>) -> Result<Vec<DirectoryRow>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    scan_directories_inner(&conn)
}

/// List all directories without rescanning.
pub fn list_directories_inner(conn: &Connection) -> Result<Vec<DirectoryRow>, String> {
    dir_svc::list_directories(conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_directories(db: State<DbState>) -> Result<Vec<DirectoryRow>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    list_directories_inner(&conn)
}

/// Add a custom directory and immediately scan it.
pub fn add_directory_inner(
    conn: &Connection,
    tool_name: String,
    path: String,
) -> Result<DirectoryRow, String> {
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO ai_tool_directories \
         (id, tool_name, path, is_default, is_detected, writable, enabled, skill_count) \
         VALUES (?1, ?2, ?3, 0, 0, 0, 1, 0)",
        params![id, tool_name, path],
    )
    .map_err(|e| e.to_string())?;

    let result = dir_svc::scan_directory(&id, &tool_name, &path);
    dir_svc::update_scan_result(conn, &result).map_err(|e| e.to_string())?;

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

#[tauri::command]
pub fn add_directory(
    db: State<DbState>,
    tool_name: String,
    path: String,
) -> Result<DirectoryRow, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    add_directory_inner(&conn, tool_name, path)
}

/// Enable or disable a directory.
pub fn toggle_directory_inner(conn: &Connection, id: String, enabled: bool) -> Result<(), String> {
    conn.execute(
        "UPDATE ai_tool_directories SET enabled = ?1 WHERE id = ?2",
        params![enabled as i64, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn toggle_directory(db: State<DbState>, id: String, enabled: bool) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    toggle_directory_inner(&conn, id, enabled)
}

/// Set a directory as the default for its AI tool.
pub fn set_default_directory_inner(conn: &Connection, id: String) -> Result<(), String> {
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

#[tauri::command]
pub fn set_default_directory(db: State<DbState>, id: String) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    set_default_directory_inner(&conn, id)
}

/// Delete a directory entry.
pub fn delete_directory_inner(conn: &Connection, id: String) -> Result<(), String> {
    conn.execute(
        "DELETE FROM ai_tool_directories WHERE id = ?1",
        params![id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_directory(db: State<DbState>, id: String) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    delete_directory_inner(&conn, id)
}

// ─── Directory tree (drawer) ─────────────────────────────────────────────────

const DEFAULT_TREE_DEPTH: u32 = 4;
const DEFAULT_TREE_MAX_NODES: usize = 5000;

/// Inner logic for `list_directory_tree`. Public + DB-free so the HTTP bridge
/// and unit tests can call it directly.
pub fn list_directory_tree_inner(
    path: &str,
    max_depth: Option<u32>,
) -> Result<FileTreeNode, String> {
    let depth = max_depth.unwrap_or(DEFAULT_TREE_DEPTH);
    dir_svc::walk_directory_tree(Path::new(path), depth, DEFAULT_TREE_MAX_NODES)
}

#[tauri::command]
pub fn list_directory_tree(
    path: String,
    max_depth: Option<u32>,
) -> Result<FileTreeNode, String> {
    list_directory_tree_inner(&path, max_depth)
}

/// Inner logic for `read_text_file`. Default 256 KB cap.
pub fn read_text_file_inner(path: &str) -> Result<Option<String>, String> {
    dir_svc::read_text_file(Path::new(path), 256 * 1024)
}

#[tauri::command]
pub fn read_text_file(path: String) -> Result<Option<String>, String> {
    read_text_file_inner(&path)
}
