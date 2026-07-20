use crate::commands::app::DbState;
use crate::services::sync::{self, ImportResult, SyncConfig, SyncResult, SyncSnapshot, SyncStatus};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use tauri::State;

/// 获取本机设备名（用于快照标识）。
/// 依次尝试 HOSTNAME / COMPUTERNAME 环境变量，回退到 home 目录名。
fn device_name() -> String {
    // Linux/macOS shell 通常导出 HOSTNAME
    if let Ok(name) = std::env::var("HOSTNAME") {
        if !name.is_empty() {
            return name;
        }
    }
    // Windows
    if let Ok(name) = std::env::var("COMPUTERNAME") {
        if !name.is_empty() {
            return name;
        }
    }
    // 回退：home 目录名（如 /Users/alice → alice）
    dirs::home_dir()
        .and_then(|h| h.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "unknown".to_string())
}

// ─── Phase 1: 本地导出/导入 ──────────────────────────────────────────────────

/// 导出同步快照，返回 JSON 字符串。
/// 前端拿到后用 Blob + `<a download>` 保存为文件。
pub fn export_sync_snapshot_inner(conn: &Connection) -> Result<String, String> {
    let snapshot = sync::build_snapshot(conn, &device_name())?;
    serde_json::to_string_pretty(&snapshot).map_err(|e| format!("serialize snapshot: {e}"))
}

#[tauri::command]
pub fn export_sync_snapshot(db: State<DbState>) -> Result<String, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    export_sync_snapshot_inner(&conn)
}

/// 导入同步快照。`json` 为 `.skillspp-sync.json` 文件内容。
pub fn import_sync_snapshot_inner(conn: &Connection, json: &str) -> Result<ImportResult, String> {
    let snapshot: SyncSnapshot =
        serde_json::from_str(json).map_err(|e| format!("parse snapshot JSON: {e}"))?;

    if snapshot.version != 1 {
        return Err(format!(
            "unsupported snapshot version: {} (expected 1)",
            snapshot.version
        ));
    }

    sync::apply_snapshot(conn, &snapshot)
}

#[tauri::command]
pub fn import_sync_snapshot(db: State<DbState>, json: String) -> Result<ImportResult, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    import_sync_snapshot_inner(&conn, &json)
}

// ─── Phase 2: WebDAV 同步 ────────────────────────────────────────────────────

/// 获取 WebDAV 同步配置。
pub fn get_sync_config_inner(conn: &Connection) -> SyncConfig {
    sync::get_sync_config(conn)
}

#[tauri::command]
pub fn get_sync_config(db: State<DbState>) -> Result<SyncConfig, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    Ok(get_sync_config_inner(&conn))
}

/// 保存 WebDAV 同步配置。
pub fn set_sync_config_inner(conn: &Connection, config: &SyncConfig) -> Result<(), String> {
    sync::set_sync_config(conn, config)
}

#[tauri::command]
pub fn set_sync_config(db: State<DbState>, config: SyncConfig) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    set_sync_config_inner(&conn, &config)
}

/// 获取同步状态。
pub fn get_sync_status_inner(conn: &Connection) -> SyncStatus {
    sync::get_sync_status(conn)
}

#[tauri::command]
pub fn get_sync_status(db: State<DbState>) -> Result<SyncStatus, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    Ok(get_sync_status_inner(&conn))
}

/// 执行 WebDAV 同步。
///
/// 内部实现接收 `Arc<Mutex<Connection>>`，因为网络请求是 async 的，
/// 不能持有 DB 锁跨越 `.await` 点。
pub async fn sync_now_inner(db: Arc<Mutex<Connection>>) -> Result<SyncResult, String> {
    sync::sync_now(db, &device_name()).await
}

#[tauri::command]
pub async fn sync_now(db: State<'_, DbState>) -> Result<SyncResult, String> {
    let db_arc = db.0.clone();
    sync_now_inner(db_arc).await
}

/// 测试 WebDAV 连接。
pub async fn test_webdav_connection_inner(config: SyncConfig) -> Result<(), String> {
    sync::test_webdav_connection(&config).await
}

#[tauri::command]
pub async fn test_webdav_connection(config: SyncConfig) -> Result<(), String> {
    test_webdav_connection_inner(config).await
}
