//! 跨设备同步：导出/导入 JSON 快照 + WebDAV 云端同步。
//!
//! Phase 1：本地导出/导入，无需网络。
//! Phase 2：WebDAV 自动同步，三向合并。
//!
//! 核心概念：
//! - **路径规范化**：导出时绝对路径 → 相对 home 的路径（`/` 分隔符），导入时反向。
//! - **合并策略**：安装记录按 `name + tool_name + directory_path` 去重，取较新的。
//! - **lockfile 合并**：按 key upsert，取 `updatedAt` 较新的。
//! - **三向合并**（Phase 2）：local + remote + base → merged，不自动传播删除。
//!
//! 参见 `docs/plans/cross-device-sync-2026-07-20.md`。

use crate::models::InstallStrategy;
use crate::repositories::settings;
use crate::services::lockfile::{self, LockEntry};
use crate::services::webdav_client::{WebDavClient, WebDavConfig};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

// ─── 路径规范化 ─────────────────────────────────────────────────────────────

/// 将绝对路径转换为相对于 home 目录的路径，统一用 `/` 分隔符。
/// 无法转换时返回原始路径（降级处理）。
///
/// `/Users/alice/.claude/skills` → `.claude/skills`
/// `/home/bob/.agents/skills/my-skill` → `.agents/skills/my-skill`
/// `C:\Users\alice\.claude\skills` → `.claude/skills`
pub fn normalize_path(abs_path: &str) -> String {
    let Some(home) = dirs::home_dir() else {
        return abs_path.replace('\\', "/");
    };
    let abs = PathBuf::from(abs_path);
    match abs.strip_prefix(&home) {
        Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
        Err(_) => abs_path.replace('\\', "/"),
    }
}

/// 将相对路径拼接本地 home 目录，转为平台绝对路径。
///
/// `.claude/skills` + macOS home → `/Users/alice/.claude/skills`
/// `.claude/skills` + Linux home → `/home/bob/.claude/skills`
pub fn denormalize_path(rel_path: &str) -> String {
    let Some(home) = dirs::home_dir() else {
        return rel_path.replace('\\', "/");
    };
    // 统一用 '/' 分隔符，然后 join
    let normalized = rel_path.replace('\\', "/");
    home.join(normalized).to_string_lossy().to_string()
}

// ─── 同步快照数据结构 ───────────────────────────────────────────────────────

/// 同步快照根结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncSnapshot {
    /// 快照格式版本。
    pub version: i64,
    /// 导出时间（ISO 8601）。
    pub exported_at: String,
    /// 导出设备名。
    pub device_name: String,
    /// 导出平台（macos / windows / linux）。
    pub platform: String,

    pub installed_skills: Vec<SyncInstalledSkill>,
    pub custom_directories: Vec<SyncDirectory>,
    pub source_preferences: Vec<SyncSourcePref>,
    pub app_settings: BTreeMap<String, String>,
    pub lockfile: BTreeMap<String, LockEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncInstalledSkill {
    pub name: String,
    pub tool_name: String,
    /// 相对于 home 的目录路径（`/` 分隔符）。
    pub directory_relative_path: String,
    pub source_id: Option<String>,
    pub repo_url: Option<String>,
    pub install_strategy: String,
    pub content_hash: Option<String>,
    /// 相对于 home 的 canonical 路径（`/` 分隔符）。
    pub canonical_relative_path: Option<String>,
    pub installed_at: String,
    pub author: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncDirectory {
    pub id: String,
    pub tool_name: String,
    /// 相对于 home 的目录路径（`/` 分隔符）。
    pub relative_path: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncSourcePref {
    pub id: String,
    pub enabled: bool,
}

// ─── 导入结果 ───────────────────────────────────────────────────────────────

/// 导入操作的汇总结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    /// 新增的安装记录数。
    pub imported_skills: usize,
    /// 已存在被跳过的安装记录数。
    pub skipped_skills: usize,
    /// 新增的自定义目录数。
    pub imported_directories: usize,
    /// 更新的来源站开关数。
    pub updated_sources: usize,
    /// 更新的应用设置数。
    pub updated_settings: usize,
    /// 合并的 lockfile 条目数。
    pub merged_lockfile_entries: usize,
}

impl Default for ImportResult {
    fn default() -> Self {
        Self {
            imported_skills: 0,
            skipped_skills: 0,
            imported_directories: 0,
            updated_sources: 0,
            updated_settings: 0,
            merged_lockfile_entries: 0,
        }
    }
}

// ─── 导出 ───────────────────────────────────────────────────────────────────

/// 从数据库读取所有可同步数据，构建 `SyncSnapshot`。
pub fn build_snapshot(conn: &Connection, device_name: &str) -> Result<SyncSnapshot, String> {
    let installed_skills = export_installed_skills(conn)?;
    let custom_directories = export_custom_directories(conn)?;
    let source_preferences = export_source_preferences(conn)?;
    let app_settings = export_app_settings(conn)?;
    let lockfile_map = lockfile::read_lockfile();

    Ok(SyncSnapshot {
        version: 1,
        exported_at: now_iso8601(),
        device_name: device_name.to_string(),
        platform: std::env::consts::OS.to_string(),
        installed_skills,
        custom_directories,
        source_preferences,
        app_settings,
        lockfile: lockfile_map,
    })
}

fn export_installed_skills(conn: &Connection) -> Result<Vec<SyncInstalledSkill>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT i.name, i.tool_name, COALESCE(d.path, ''), \
                    i.source_id, i.repo_url, i.installed_at, i.install_strategy, \
                    i.content_hash, i.canonical_path, i.author, i.description \
             FROM installed_skills i \
             LEFT JOIN ai_tool_directories d ON i.directory_id = d.id \
             ORDER BY i.installed_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            let strategy_s: String = row.get(6)?;
            Ok(SyncInstalledSkill {
                name: row.get(0)?,
                tool_name: row.get(1)?,
                directory_relative_path: normalize_path(&row.get::<_, String>(2)?),
                source_id: row.get(3)?,
                repo_url: row.get(4)?,
                installed_at: row.get(5)?,
                install_strategy: strategy_s,
                content_hash: row.get(7)?,
                canonical_relative_path: row
                    .get::<_, Option<String>>(8)?
                    .map(|p| normalize_path(&p)),
                author: row.get(9)?,
                description: row.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

fn export_custom_directories(conn: &Connection) -> Result<Vec<SyncDirectory>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, tool_name, path, is_default \
             FROM ai_tool_directories \
             WHERE is_detected = 0 \
             ORDER BY tool_name",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(SyncDirectory {
                id: row.get(0)?,
                tool_name: row.get(1)?,
                relative_path: normalize_path(&row.get::<_, String>(2)?),
                is_default: row.get::<_, i64>(3)? != 0,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

fn export_source_preferences(conn: &Connection) -> Result<Vec<SyncSourcePref>, String> {
    let mut stmt = conn
        .prepare("SELECT id, enabled FROM skill_sources ORDER BY id")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(SyncSourcePref {
                id: row.get(0)?,
                enabled: row.get::<_, i64>(1)? != 0,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

fn export_app_settings(conn: &Connection) -> Result<BTreeMap<String, String>, String> {
    let mut stmt = conn
        .prepare("SELECT key, value FROM app_settings")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?;

    let mut map = BTreeMap::new();
    for row in rows {
        let (k, v) = row.map_err(|e| e.to_string())?;
        map.insert(k, v);
    }
    Ok(map)
}

// ─── 导入 ───────────────────────────────────────────────────────────────────

/// 将同步快照合并到本地数据库。
///
/// 合并策略：
/// - 安装记录：按 `name + denormalized_directory_path` 去重，已存在则跳过。
/// - 自定义目录：按 `denormalized_relative_path` 去重，已存在则跳过。
/// - 来源站开关：直接 UPDATE enabled。
/// - 应用设置：UPSERT。
/// - Lockfile：按 key 合并，取 `updatedAt` 较新的。
pub fn apply_snapshot(conn: &Connection, snapshot: &SyncSnapshot) -> Result<ImportResult, String> {
    let mut result = ImportResult::default();

    // 1. 导入自定义目录（必须在安装记录之前，以便 directory_id 可解析）
    for dir in &snapshot.custom_directories {
        let abs_path = denormalize_path(&dir.relative_path);
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ai_tool_directories WHERE path = ?1",
                params![abs_path],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        if exists == 0 {
            conn.execute(
                "INSERT INTO ai_tool_directories \
                 (id, tool_name, path, is_default, is_detected, writable, enabled, skill_count) \
                 VALUES (?1, ?2, ?3, ?4, 0, 0, 1, 0)",
                params![dir.id, dir.tool_name, abs_path, dir.is_default as i64],
            )
            .map_err(|e| e.to_string())?;
            result.imported_directories += 1;
        }
    }

    // 2. 导入安装记录
    for skill in &snapshot.installed_skills {
        let abs_dir_path = denormalize_path(&skill.directory_relative_path);

        // 通过路径查找 directory_id
        let directory_id: Option<String> = conn
            .query_row(
                "SELECT id FROM ai_tool_directories WHERE path = ?1",
                params![abs_dir_path],
                |row| row.get(0),
            )
            .ok();

        let Some(directory_id) = directory_id else {
            log::warn!(
                "sync: directory not found for skill '{}', path='{}' — skipped",
                skill.name,
                abs_dir_path
            );
            result.skipped_skills += 1;
            continue;
        };

        // 检查是否已存在（name + directory_id）
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM installed_skills WHERE name = ?1 AND directory_id = ?2",
                params![skill.name, directory_id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        if exists > 0 {
            result.skipped_skills += 1;
            continue;
        }

        let strategy = InstallStrategy::parse(&skill.install_strategy);
        let canonical_abs = skill
            .canonical_relative_path
            .as_ref()
            .map(|p| denormalize_path(p));

        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO installed_skills \
             (id, skill_id, name, tool_name, directory_id, source_id, repo_url, \
              installed_at, status, install_strategy, content_hash, canonical_path, \
              author, description) \
             VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?6, ?7, 'ok', ?8, ?9, ?10, ?11, ?12)",
            params![
                id,
                skill.name,
                skill.tool_name,
                directory_id,
                skill.source_id,
                skill.repo_url,
                skill.installed_at,
                strategy.as_str(),
                skill.content_hash,
                canonical_abs,
                skill.author,
                skill.description,
            ],
        )
        .map_err(|e| e.to_string())?;

        result.imported_skills += 1;
    }

    // 3. 导入来源站开关
    for pref in &snapshot.source_preferences {
        let affected = conn
            .execute(
                "UPDATE skill_sources SET enabled = ?1 WHERE id = ?2",
                params![pref.enabled as i64, pref.id],
            )
            .map_err(|e| e.to_string())?;
        if affected > 0 {
            result.updated_sources += 1;
        }
    }

    // 4. 导入应用设置（UPSERT）
    for (key, value) in &snapshot.app_settings {
        settings::set_str(conn, key, value).map_err(|e| e.to_string())?;
        result.updated_settings += 1;
    }

    // 5. 合并 lockfile
    let mut local_lockfile = lockfile::read_lockfile();
    for (key, remote_entry) in &snapshot.lockfile {
        match local_lockfile.get(key) {
            Some(local_entry) => {
                // 取 updatedAt 较新的
                if remote_entry.updated_at > local_entry.updated_at {
                    local_lockfile.insert(key.clone(), remote_entry.clone());
                    result.merged_lockfile_entries += 1;
                }
            }
            None => {
                local_lockfile.insert(key.clone(), remote_entry.clone());
                result.merged_lockfile_entries += 1;
            }
        }
    }
    if result.merged_lockfile_entries > 0 {
        lockfile::write_lockfile(&local_lockfile)?;
    }

    Ok(result)
}

// ─── Phase 2: WebDAV 同步配置 ──────────────────────────────────────────────

/// WebDAV 同步配置（存储在 `app_settings` 表中）。
/// 密码以 base64 编码存储（非加密，仅做简单混淆）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncConfig {
    /// WebDAV 服务器 URL（如 `https://dav.example.com`）。
    #[serde(default)]
    pub webdav_url: String,
    /// WebDAV 用户名。
    #[serde(default)]
    pub webdav_username: String,
    /// WebDAV 密码（明文，传输走 HTTPS Basic Auth）。
    #[serde(default)]
    pub webdav_password: String,
    /// 远端存储路径（如 `/skillspp`）。
    #[serde(default = "default_remote_path")]
    pub webdav_remote_path: String,
    /// 是否启用自动同步。
    #[serde(default)]
    pub auto_sync: bool,
    /// 自动同步间隔（分钟）。
    #[serde(default = "default_auto_sync_interval")]
    pub auto_sync_interval: u32,
}

fn default_remote_path() -> String {
    "/skillspp".to_string()
}

fn default_auto_sync_interval() -> u32 {
    30
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            webdav_url: String::new(),
            webdav_username: String::new(),
            webdav_password: String::new(),
            webdav_remote_path: default_remote_path(),
            auto_sync: false,
            auto_sync_interval: default_auto_sync_interval(),
        }
    }
}

// app_settings 中的 key 常量
const SK_URL: &str = "sync.webdav.url";
const SK_USERNAME: &str = "sync.webdav.username";
const SK_PASSWORD: &str = "sync.webdav.password";
const SK_REMOTE_PATH: &str = "sync.webdav.remotePath";
const SK_AUTO_SYNC: &str = "sync.autoSync";
const SK_AUTO_SYNC_INTERVAL: &str = "sync.autoSyncInterval";
const SK_LAST_SYNC_AT: &str = "sync.lastSyncAt";
const SK_LAST_SYNC_DEVICE: &str = "sync.lastSyncDevice";
const SK_LAST_SYNC_RESULT: &str = "sync.lastSyncResult";
const SK_LAST_SYNC_ERROR: &str = "sync.lastSyncError";
const SK_SYNC_BASE: &str = "sync.lastSyncBase";

/// 从 `app_settings` 读取同步配置。
pub fn get_sync_config(conn: &Connection) -> SyncConfig {
    SyncConfig {
        webdav_url: settings::get_str(conn, SK_URL).ok().flatten().unwrap_or_default(),
        webdav_username: settings::get_str(conn, SK_USERNAME).ok().flatten().unwrap_or_default(),
        webdav_password: settings::get_str(conn, SK_PASSWORD).ok().flatten().unwrap_or_default(),
        webdav_remote_path: settings::get_str(conn, SK_REMOTE_PATH)
            .ok()
            .flatten()
            .unwrap_or_else(default_remote_path),
        auto_sync: settings::get_str(conn, SK_AUTO_SYNC)
            .ok()
            .flatten()
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false),
        auto_sync_interval: settings::get_str(conn, SK_AUTO_SYNC_INTERVAL)
            .ok()
            .flatten()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30),
    }
}

/// 将同步配置写入 `app_settings`。
pub fn set_sync_config(conn: &Connection, config: &SyncConfig) -> Result<(), String> {
    settings::set_str(conn, SK_URL, &config.webdav_url).map_err(|e| e.to_string())?;
    settings::set_str(conn, SK_USERNAME, &config.webdav_username).map_err(|e| e.to_string())?;
    settings::set_str(conn, SK_PASSWORD, &config.webdav_password).map_err(|e| e.to_string())?;
    settings::set_str(conn, SK_REMOTE_PATH, &config.webdav_remote_path).map_err(|e| e.to_string())?;
    settings::set_str(conn, SK_AUTO_SYNC, if config.auto_sync { "1" } else { "0" }).map_err(|e| e.to_string())?;
    settings::set_str(conn, SK_AUTO_SYNC_INTERVAL, &config.auto_sync_interval.to_string())
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Phase 2: 同步状态 ────────────────────────────────────────────────────────

/// 同步状态信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    /// 上次同步时间（ISO 8601），None 表示从未同步。
    #[serde(default)]
    pub last_sync_at: Option<String>,
    /// 上次同步的设备名。
    #[serde(default)]
    pub last_sync_device: Option<String>,
    /// 上次同步结果：`success` / `conflict` / `error`。
    #[serde(default)]
    pub last_sync_result: Option<String>,
    /// 上次同步的错误信息（仅 error 时有值）。
    #[serde(default)]
    pub last_sync_error: Option<String>,
}

impl Default for SyncStatus {
    fn default() -> Self {
        Self {
            last_sync_at: None,
            last_sync_device: None,
            last_sync_result: None,
            last_sync_error: None,
        }
    }
}

/// 从 `app_settings` 读取同步状态。
pub fn get_sync_status(conn: &Connection) -> SyncStatus {
    fn opt(v: rusqlite::Result<Option<String>>) -> Option<String> {
        v.ok().flatten().filter(|s| !s.is_empty())
    }
    SyncStatus {
        last_sync_at: opt(settings::get_str(conn, SK_LAST_SYNC_AT)),
        last_sync_device: opt(settings::get_str(conn, SK_LAST_SYNC_DEVICE)),
        last_sync_result: opt(settings::get_str(conn, SK_LAST_SYNC_RESULT)),
        last_sync_error: opt(settings::get_str(conn, SK_LAST_SYNC_ERROR)),
    }
}

/// 将同步状态写入 `app_settings`。
pub fn set_sync_status(conn: &Connection, status: &SyncStatus) -> Result<(), String> {
    fn opt_to_str(v: &Option<String>) -> &str {
        v.as_deref().unwrap_or("")
    }
    settings::set_str(conn, SK_LAST_SYNC_AT, opt_to_str(&status.last_sync_at)).map_err(|e| e.to_string())?;
    settings::set_str(conn, SK_LAST_SYNC_DEVICE, opt_to_str(&status.last_sync_device)).map_err(|e| e.to_string())?;
    settings::set_str(conn, SK_LAST_SYNC_RESULT, opt_to_str(&status.last_sync_result)).map_err(|e| e.to_string())?;
    settings::set_str(conn, SK_LAST_SYNC_ERROR, opt_to_str(&status.last_sync_error)).map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Phase 2: Sync Base 管理 ──────────────────────────────────────────────────

/// 从 `app_settings` 读取上次同步的基线快照。
pub fn get_sync_base(conn: &Connection) -> Option<SyncSnapshot> {
    let json = settings::get_str(conn, SK_SYNC_BASE).ok().flatten()?;
    match serde_json::from_str::<SyncSnapshot>(&json) {
        Ok(s) => Some(s),
        Err(e) => {
            log::warn!("sync: parse sync base failed: {e}");
            None
        }
    }
}

/// 将合并后的快照保存为新的同步基线。
pub fn save_sync_base(conn: &Connection, snapshot: &SyncSnapshot) -> Result<(), String> {
    let json = serde_json::to_string(snapshot).map_err(|e| format!("serialize sync base: {e}"))?;
    settings::set_str(conn, SK_SYNC_BASE, &json).map_err(|e| e.to_string())
}

// ─── Phase 2: 三向合并 ──────────────────────────────────────────────────────

/// 同步冲突信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncConflict {
    /// 冲突类型：`remote_deleted`（远端已删除，本地仍有）。
    pub kind: String,
    /// skill 名称。
    pub skill_name: String,
    /// 工具名。
    pub tool_name: String,
    /// 目录相对路径。
    pub directory_relative_path: String,
}

/// `sync_now` 操作的汇总结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    /// 从远端拉取的新安装记录数（远端有、本地无）。
    pub pulled_skills: usize,
    /// 推送到远端的安装记录数（本地有、远端无）。
    pub pushed_skills: usize,
    /// 更新的设置数。
    pub updated_settings: usize,
    /// 合并的 lockfile 条目数。
    pub merged_lockfile_entries: usize,
    /// 检测到的冲突列表。
    #[serde(default)]
    pub conflicts: Vec<SyncConflict>,
}

/// 合并键：`name|tool_name|directory_relative_path`
fn skill_key(s: &SyncInstalledSkill) -> String {
    format!("{}|{}|{}", s.name, s.tool_name, s.directory_relative_path)
}

/// 三向合并两个快照。
///
/// 合并策略（并集 + 取较新，不自动传播删除）：
/// - **安装记录**：按 `name + tool_name + directory_relative_path` 去重。
///   - 两端都有 → 取 `installed_at` 较新的。
///   - 只有本地有 → 保留（本地新装）。
///   - 只有远端有 → 保留（远端新装）。
///   - base 有 + 本地有 + 远端无 → 冲突（远端已删除），保留本地。
/// - **自定义目录**：按 `relative_path` 去重，并集。
/// - **来源站开关**：直接取远端值。
/// - **应用设置**：UPSERT（远端覆盖本地）。
/// - **Lockfile**：按 key 合并，取 `updatedAt` 较新的。
pub fn merge_snapshots(
    local: &SyncSnapshot,
    remote: Option<&SyncSnapshot>,
    base: Option<&SyncSnapshot>,
) -> (SyncSnapshot, Vec<SyncConflict>) {
    let mut conflicts = Vec::new();

    // 如果没有远端快照（首次同步），直接用本地
    let Some(remote) = remote else {
        return (local.clone(), conflicts);
    };

    // ── 合并安装记录 ──
    let local_map: BTreeMap<String, &SyncInstalledSkill> = local
        .installed_skills
        .iter()
        .map(|s| (skill_key(s), s))
        .collect();
    let remote_map: BTreeMap<String, &SyncInstalledSkill> = remote
        .installed_skills
        .iter()
        .map(|s| (skill_key(s), s))
        .collect();
    let base_map: BTreeMap<String, &SyncInstalledSkill> = base
        .map(|b| b.installed_skills.iter().map(|s| (skill_key(s), s)).collect())
        .unwrap_or_default();

    let all_keys: BTreeSet<String> = local_map
        .keys()
        .map(|k| k.to_string())
        .chain(remote_map.keys().cloned())
        .chain(base_map.keys().cloned())
        .collect();

    let mut merged_skills = Vec::new();
    for key in &all_keys {
        let l = local_map.get(key).copied();
        let r = remote_map.get(key).copied();
        let b = base_map.get(key).copied();

        match (l, r) {
            (Some(l), Some(r)) => {
                // 两端都有 → 取较新的
                if l.installed_at >= r.installed_at {
                    merged_skills.push(l.clone());
                } else {
                    merged_skills.push(r.clone());
                }
            }
            (Some(l), None) => {
                // 只有本地有
                if b.is_some() {
                    // base 有但远端没有 → 远端删除了
                    conflicts.push(SyncConflict {
                        kind: "remote_deleted".to_string(),
                        skill_name: l.name.clone(),
                        tool_name: l.tool_name.clone(),
                        directory_relative_path: l.directory_relative_path.clone(),
                    });
                }
                // 保留本地
                merged_skills.push(l.clone());
            }
            (None, Some(r)) => {
                // 只有远端有 → 保留
                merged_skills.push(r.clone());
            }
            (None, None) => {}
        }
    }

    // ── 合并自定义目录（并集，按 relative_path 去重）──
    let mut seen_dirs: BTreeSet<String> = BTreeSet::new();
    let mut merged_dirs = Vec::new();
    for d in local.custom_directories.iter().chain(remote.custom_directories.iter()) {
        if seen_dirs.insert(d.relative_path.clone()) {
            merged_dirs.push(d.clone());
        }
    }

    // ── 合并来源站开关（取远端值）──
    let merged_sources = remote.source_preferences.clone();

    // ── 合并应用设置（远端覆盖本地）──
    let mut merged_settings = local.app_settings.clone();
    for (k, v) in &remote.app_settings {
        merged_settings.insert(k.clone(), v.clone());
    }

    // ── 合并 lockfile（取 updatedAt 较新的）──
    let mut merged_lockfile = local.lockfile.clone();
    for (key, remote_entry) in &remote.lockfile {
        match merged_lockfile.get(key) {
            Some(local_entry) => {
                if remote_entry.updated_at > local_entry.updated_at {
                    merged_lockfile.insert(key.clone(), remote_entry.clone());
                }
            }
            None => {
                merged_lockfile.insert(key.clone(), remote_entry.clone());
            }
        }
    }

    let merged = SyncSnapshot {
        version: 1,
        exported_at: now_iso8601(),
        device_name: local.device_name.clone(),
        platform: local.platform.clone(),
        installed_skills: merged_skills,
        custom_directories: merged_dirs,
        source_preferences: merged_sources,
        app_settings: merged_settings,
        lockfile: merged_lockfile,
    };

    (merged, conflicts)
}

// ─── Phase 2: sync_now 引擎 ──────────────────────────────────────────────────

/// 同步快照在 WebDAV 上的文件名。
const REMOTE_FILENAME: &str = "skillspp-sync.json";

/// 执行完整的 WebDAV 同步流程。
///
/// 流程：
/// 1. 构建本地快照
/// 2. 下载远端快照
/// 3. 读取本地 sync base
/// 4. 三向合并 → merged + conflicts
/// 5. 将 merged 应用到本地 DB
/// 6. 上传 merged 到 WebDAV
/// 7. 保存 merged 为新 sync base
/// 8. 更新同步状态
///
/// **注意**：此函数需要 `Arc<Mutex<Connection>>` 而非 `&Connection`，
/// 因为 WebDAV 网络请求是 async 的，不能持有 DB 锁跨越 `.await` 点。
pub async fn sync_now(
    db: Arc<Mutex<Connection>>,
    device_name: &str,
) -> Result<SyncResult, String> {
    // ── Phase 1: 锁 DB，构建快照 + 读取配置 ──
    let (local_snapshot, config, sync_base) = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let snapshot = build_snapshot(&conn, device_name)?;
        let config = get_sync_config(&conn);
        let sync_base = get_sync_base(&conn);
        (snapshot, config, sync_base)
    };
    // MutexGuard 已 drop，后续可安全 .await

    if config.webdav_url.is_empty() {
        return Err("WebDAV 未配置，请先在设置页填写 WebDAV 信息".to_string());
    }

    // ── Phase 2: WebDAV 网络操作（不持有 DB 锁）──
    let dav_config = WebDavConfig {
        url: config.webdav_url.clone(),
        username: config.webdav_username.clone(),
        password: config.webdav_password.clone(),
        remote_path: config.webdav_remote_path.clone(),
    };
    let client = WebDavClient::new(&dav_config)?;

    // 下载远端快照
    let remote_json = client
        .download(&config.webdav_remote_path, REMOTE_FILENAME)
        .await?;
    let remote_snapshot: Option<SyncSnapshot> = remote_json
        .as_deref()
        .and_then(|j| serde_json::from_str(j).ok());

    // 三向合并
    let (merged, conflicts) = merge_snapshots(&local_snapshot, remote_snapshot.as_ref(), sync_base.as_ref());

    // ── Phase 3: 锁 DB，应用合并结果 ──
    let pulled_skills;
    let updated_settings;
    let merged_lockfile_entries;
    {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let import_result = apply_snapshot(&conn, &merged)?;

        pulled_skills = import_result.imported_skills;
        updated_settings = import_result.updated_settings;
        merged_lockfile_entries = import_result.merged_lockfile_entries;

        // 保存 sync base
        save_sync_base(&conn, &merged)?;

        // 更新同步状态
        let result_str = if conflicts.is_empty() { "success" } else { "conflict" };
        set_sync_status(
            &conn,
            &SyncStatus {
                last_sync_at: Some(now_iso8601()),
                last_sync_device: Some(device_name.to_string()),
                last_sync_result: Some(result_str.to_string()),
                last_sync_error: None,
            },
        )?;
    }
    // MutexGuard 已 drop

    // ── Phase 4: 上传合并后的快照到 WebDAV ──
    let merged_json = serde_json::to_string_pretty(&merged)
        .map_err(|e| format!("serialize merged snapshot: {e}"))?;
    client
        .upload(&config.webdav_remote_path, REMOTE_FILENAME, &merged_json)
        .await?;

    // 计算 pushed_skills = 本地有但远端没有的
    let pushed_skills = if let Some(remote) = remote_snapshot {
        let remote_keys: BTreeSet<String> = remote
            .installed_skills
            .iter()
            .map(|s| skill_key(s))
            .collect();
        local_snapshot
            .installed_skills
            .iter()
            .filter(|s| !remote_keys.contains(&skill_key(s)))
            .count()
    } else {
        local_snapshot.installed_skills.len()
    };

    Ok(SyncResult {
        pulled_skills,
        pushed_skills,
        updated_settings,
        merged_lockfile_entries,
        conflicts,
    })
}

/// 测试 WebDAV 连接（不执行同步操作）。
pub async fn test_webdav_connection(config: &SyncConfig) -> Result<(), String> {
    let dav_config = WebDavConfig {
        url: config.webdav_url.clone(),
        username: config.webdav_username.clone(),
        password: config.webdav_password.clone(),
        remote_path: config.webdav_remote_path.clone(),
    };
    let client = WebDavClient::new(&dav_config)?;
    client.test_connection(&config.webdav_remote_path).await
}

// ─── 工具函数 ───────────────────────────────────────────────────────────────

/// 返回当前时间的 ISO 8601 字符串（UTC）。
fn now_iso8601() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();

    // 简易 UTC 时间格式化（不引入 chrono 依赖）
    let (year, month, day, hour, min, sec) = unix_to_utc(secs);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}Z")
}

/// Unix 时间戳 → UTC 年月日时分秒（简易算法，2020-2099 范围内准确）。
fn unix_to_utc(secs: u64) -> (u32, u32, u32, u32, u32, u32) {
    let days_per_400y = 146097;

    let sec = (secs % 60) as u32;
    let min = ((secs / 60) % 60) as u32;
    let hour = ((secs / 3600) % 24) as u32;

    let mut days = secs / 86400;
    // 1970-01-01 → 偏移到 2000-03-01（便于闰年计算）
    // 实际算法：从 1970 开始推算
    days += 719468; // 1970-01-01 → 0000-03-01 的天数
    let era = days / days_per_400y;
    let doe = days - era * days_per_400y; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let year = if m <= 2 { y + 1 } else { y };

    (year as u32, m as u32, d as u32, hour, min, sec)
}

// ─── 测试 ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::db;

    fn open_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        db::migrate(&conn).unwrap();
        db::seed_sources(&conn).unwrap();
        conn
    }

    // ── 路径规范化 ──

    #[test]
    fn normalize_path_strips_home_prefix() {
        let home = dirs::home_dir().unwrap();
        let abs = home.join(".claude/skills");
        let normalized = normalize_path(&abs.to_string_lossy());
        assert_eq!(normalized, ".claude/skills");
    }

    #[test]
    fn normalize_path_handles_backslashes() {
        // 模拟 Windows 路径（即使不在 Windows 上也能测试逻辑）
        let home = dirs::home_dir().unwrap();
        let abs = home.join(".claude").join("skills");
        let abs_str = abs.to_string_lossy().replace('/', "\\");
        let normalized = normalize_path(&abs_str);
        // 应该统一用 '/' 分隔符
        assert!(normalized.contains(".claude/skills"));
        assert!(!normalized.contains('\\'));
    }

    #[test]
    fn denormalize_path_joins_home() {
        let home = dirs::home_dir().unwrap();
        let abs = denormalize_path(".claude/skills");
        let expected = home.join(".claude/skills");
        assert_eq!(abs, expected.to_string_lossy());
    }

    #[test]
    fn normalize_then_denormalize_roundtrips() {
        let home = dirs::home_dir().unwrap();
        let original = home.join(".agents/skills/my-skill").to_string_lossy().to_string();
        let normalized = normalize_path(&original);
        let denormalized = denormalize_path(&normalized);
        assert_eq!(denormalized, original);
    }

    // ── 导出 ──

    #[test]
    fn build_snapshot_includes_all_data() {
        let conn = open_test_db();

        // 添加一个自定义目录
        conn.execute(
            "INSERT INTO ai_tool_directories \
             (id, tool_name, path, is_default, is_detected, writable, enabled, skill_count) \
             VALUES ('custom-1', 'Claude', ?1, 0, 0, 0, 1, 0)",
            params![denormalize_path(".claude/skills-work")],
        )
        .unwrap();

        // 添加一个已安装 skill
        conn.execute(
            "INSERT INTO installed_skills \
             (id, name, tool_name, directory_id, source_id, repo_url, install_strategy, status) \
             VALUES ('u1', 'my-skill', 'Claude', 'custom-1', 'skills_sh', 'https://github.com/a/b', 'git', 'ok')",
            [],
        )
        .unwrap();

        // 写入一个 app_setting
        settings::set_str(&conn, "mirror.enabled", "1").unwrap();

        let snapshot = build_snapshot(&conn, "test-device").unwrap();

        assert_eq!(snapshot.version, 1);
        assert_eq!(snapshot.device_name, "test-device");
        assert_eq!(snapshot.installed_skills.len(), 1);
        assert_eq!(snapshot.installed_skills[0].name, "my-skill");
        assert_eq!(snapshot.installed_skills[0].directory_relative_path, ".claude/skills-work");
        assert_eq!(snapshot.custom_directories.len(), 1);
        assert_eq!(snapshot.custom_directories[0].relative_path, ".claude/skills-work");
        assert!(snapshot.source_preferences.iter().any(|s| s.id == "skills_sh"));
        assert_eq!(snapshot.app_settings.get("mirror.enabled"), Some(&"1".to_string()));
    }

    // ── 导入 ──

    #[test]
    fn apply_snapshot_imports_new_skills() {
        let conn = open_test_db();

        // 先 seed 预置目录（claude-0 等）
        crate::services::directory::seed_default_directories(&conn).unwrap();

        let snapshot = SyncSnapshot {
            version: 1,
            exported_at: "2026-07-20T10:00:00Z".to_string(),
            device_name: "other-device".to_string(),
            platform: "linux".to_string(),
            installed_skills: vec![SyncInstalledSkill {
                name: "imported-skill".to_string(),
                tool_name: "Claude".to_string(),
                directory_relative_path: ".claude/skills".to_string(),
                source_id: Some("skills_sh".to_string()),
                repo_url: Some("https://github.com/a/b".to_string()),
                install_strategy: "git".to_string(),
                content_hash: None,
                canonical_relative_path: None,
                installed_at: "2026-07-20T10:00:00Z".to_string(),
                author: None,
                description: None,
            }],
            custom_directories: vec![],
            source_preferences: vec![],
            app_settings: BTreeMap::new(),
            lockfile: BTreeMap::new(),
        };

        let result = apply_snapshot(&conn, &snapshot).unwrap();

        assert_eq!(result.imported_skills, 1);
        assert_eq!(result.skipped_skills, 0);

        // 验证 DB 中确实有这条记录
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM installed_skills WHERE name = 'imported-skill'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn apply_snapshot_skips_existing_skills() {
        let conn = open_test_db();
        crate::services::directory::seed_default_directories(&conn).unwrap();

        // 先导入一次
        let snapshot = SyncSnapshot {
            version: 1,
            exported_at: "2026-07-20T10:00:00Z".to_string(),
            device_name: "other-device".to_string(),
            platform: "linux".to_string(),
            installed_skills: vec![SyncInstalledSkill {
                name: "my-skill".to_string(),
                tool_name: "Claude".to_string(),
                directory_relative_path: ".claude/skills".to_string(),
                source_id: None,
                repo_url: None,
                install_strategy: "git".to_string(),
                content_hash: None,
                canonical_relative_path: None,
                installed_at: "2026-07-20T10:00:00Z".to_string(),
                author: None,
                description: None,
            }],
            custom_directories: vec![],
            source_preferences: vec![],
            app_settings: BTreeMap::new(),
            lockfile: BTreeMap::new(),
        };

        let result1 = apply_snapshot(&conn, &snapshot).unwrap();
        assert_eq!(result1.imported_skills, 1);

        // 再导入一次 → 应跳过
        let result2 = apply_snapshot(&conn, &snapshot).unwrap();
        assert_eq!(result2.imported_skills, 0);
        assert_eq!(result2.skipped_skills, 1);
    }

    #[test]
    fn apply_snapshot_imports_custom_directories() {
        let conn = open_test_db();

        let snapshot = SyncSnapshot {
            version: 1,
            exported_at: "2026-07-20T10:00:00Z".to_string(),
            device_name: "other-device".to_string(),
            platform: "linux".to_string(),
            installed_skills: vec![],
            custom_directories: vec![SyncDirectory {
                id: "custom-test-1".to_string(),
                tool_name: "Claude".to_string(),
                relative_path: ".claude/skills-extra".to_string(),
                is_default: false,
            }],
            source_preferences: vec![],
            app_settings: BTreeMap::new(),
            lockfile: BTreeMap::new(),
        };

        let result = apply_snapshot(&conn, &snapshot).unwrap();
        assert_eq!(result.imported_directories, 1);

        // 再次导入 → 跳过（路径已存在）
        let result2 = apply_snapshot(&conn, &snapshot).unwrap();
        assert_eq!(result2.imported_directories, 0);
    }

    #[test]
    fn apply_snapshot_updates_source_preferences() {
        let conn = open_test_db();

        // 初始状态：skills_sh enabled = 1
        let enabled: i64 = conn
            .query_row(
                "SELECT enabled FROM skill_sources WHERE id = 'skills_sh'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(enabled, 1);

        let snapshot = SyncSnapshot {
            version: 1,
            exported_at: "2026-07-20T10:00:00Z".to_string(),
            device_name: "other-device".to_string(),
            platform: "linux".to_string(),
            installed_skills: vec![],
            custom_directories: vec![],
            source_preferences: vec![SyncSourcePref {
                id: "skills_sh".to_string(),
                enabled: false,
            }],
            app_settings: BTreeMap::new(),
            lockfile: BTreeMap::new(),
        };

        let result = apply_snapshot(&conn, &snapshot).unwrap();
        assert_eq!(result.updated_sources, 1);

        let enabled: i64 = conn
            .query_row(
                "SELECT enabled FROM skill_sources WHERE id = 'skills_sh'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(enabled, 0);
    }

    #[test]
    fn apply_snapshot_upserts_app_settings() {
        let conn = open_test_db();

        let mut settings_map = BTreeMap::new();
        settings_map.insert("mirror.enabled".to_string(), "0".to_string());
        settings_map.insert("custom.key".to_string(), "custom-value".to_string());

        let snapshot = SyncSnapshot {
            version: 1,
            exported_at: "2026-07-20T10:00:00Z".to_string(),
            device_name: "other-device".to_string(),
            platform: "linux".to_string(),
            installed_skills: vec![],
            custom_directories: vec![],
            source_preferences: vec![],
            app_settings: settings_map,
            lockfile: BTreeMap::new(),
        };

        let result = apply_snapshot(&conn, &snapshot).unwrap();
        assert_eq!(result.updated_settings, 2);

        assert_eq!(
            settings::get_str(&conn, "mirror.enabled").unwrap(),
            Some("0".to_string())
        );
        assert_eq!(
            settings::get_str(&conn, "custom.key").unwrap(),
            Some("custom-value".to_string())
        );
    }

    #[test]
    fn apply_snapshot_merges_lockfile() {
        let conn = open_test_db();

        // 用唯一 key 避免与真实 lockfile 或其它测试运行冲突
        let test_key = format!("test-skill-sync-{}", std::process::id());

        // 清理：确保测试前没有残留
        let mut before = lockfile::read_lockfile();
        before.remove(&test_key);
        let _ = lockfile::write_lockfile(&before);

        let mut lockfile_map = BTreeMap::new();
        lockfile_map.insert(
            test_key.clone(),
            LockEntry {
                source: "owner/repo".to_string(),
                source_type: "github".to_string(),
                source_url: "https://github.com/owner/repo".to_string(),
                skill_path: "SKILL.md".to_string(),
                skill_folder_hash: "abc123".to_string(),
                installed_at: "2026-07-20T10:00:00Z".to_string(),
                updated_at: "2026-07-20T10:00:00Z".to_string(),
            },
        );

        let snapshot = SyncSnapshot {
            version: 1,
            exported_at: "2026-07-20T10:00:00Z".to_string(),
            device_name: "other-device".to_string(),
            platform: "linux".to_string(),
            installed_skills: vec![],
            custom_directories: vec![],
            source_preferences: vec![],
            app_settings: BTreeMap::new(),
            lockfile: lockfile_map,
        };

        let result = apply_snapshot(&conn, &snapshot).unwrap();
        assert_eq!(result.merged_lockfile_entries, 1);

        // 验证 lockfile 被写入
        let local_lf = lockfile::read_lockfile();
        assert!(local_lf.contains_key(&test_key));

        // 清理：删除测试条目
        let mut after = lockfile::read_lockfile();
        after.remove(&test_key);
        let _ = lockfile::write_lockfile(&after);
    }

    // ── 导出 → 导入 round-trip ──

    #[test]
    fn export_then_import_roundtrip() {
        let conn1 = open_test_db();
        crate::services::directory::seed_default_directories(&conn1).unwrap();

        // 在 conn1 中添加数据
        conn1.execute(
            "INSERT INTO installed_skills \
             (id, name, tool_name, directory_id, source_id, repo_url, install_strategy, status) \
             VALUES ('u1', 'roundtrip-skill', 'Claude', 'claude-0', 'skills_sh', 'https://github.com/a/b', 'git', 'ok')",
            [],
        )
        .unwrap();
        settings::set_str(&conn1, "mirror.enabled", "0").unwrap();

        // 导出
        let snapshot = build_snapshot(&conn1, "device-1").unwrap();
        assert_eq!(snapshot.installed_skills.len(), 1);

        // 导入到新 DB
        let conn2 = open_test_db();
        crate::services::directory::seed_default_directories(&conn2).unwrap();

        let result = apply_snapshot(&conn2, &snapshot).unwrap();
        assert_eq!(result.imported_skills, 1);
        assert_eq!(result.updated_settings, 1); // mirror.enabled

        // 验证 conn2 中有 roundtrip-skill
        let count: i64 = conn2
            .query_row(
                "SELECT COUNT(*) FROM installed_skills WHERE name = 'roundtrip-skill'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);

        // 验证 app_setting 被同步
        assert_eq!(
            settings::get_str(&conn2, "mirror.enabled").unwrap(),
            Some("0".to_string())
        );
    }

    // ── 时间格式化 ──

    #[test]
    fn now_iso8601_is_valid_format() {
        let ts = now_iso8601();
        // 2026-07-20T10:30:00Z
        assert!(ts.len() == 20);
        assert!(ts.ends_with('Z'));
        assert!(ts.contains('T'));
        assert!(ts.contains('-'));
        assert!(ts.contains(':'));
    }

    #[test]
    fn unix_to_utc_known_value() {
        // 2026-01-01T00:00:00Z = 1767225600
        let (y, m, d, h, mi, s) = unix_to_utc(1767225600);
        assert_eq!(y, 2026);
        assert_eq!(m, 1);
        assert_eq!(d, 1);
        assert_eq!(h, 0);
        assert_eq!(mi, 0);
        assert_eq!(s, 0);
    }

    // ── Phase 2: 配置读写 ──

    #[test]
    fn sync_config_roundtrip() {
        let conn = open_test_db();
        let config = SyncConfig {
            webdav_url: "https://dav.example.com".to_string(),
            webdav_username: "user".to_string(),
            webdav_password: "pass".to_string(),
            webdav_remote_path: "/mypath".to_string(),
            auto_sync: true,
            auto_sync_interval: 60,
        };
        set_sync_config(&conn, &config).unwrap();
        let loaded = get_sync_config(&conn);
        assert_eq!(loaded.webdav_url, "https://dav.example.com");
        assert_eq!(loaded.webdav_username, "user");
        assert_eq!(loaded.webdav_password, "pass");
        assert_eq!(loaded.webdav_remote_path, "/mypath");
        assert!(loaded.auto_sync);
        assert_eq!(loaded.auto_sync_interval, 60);
    }

    #[test]
    fn sync_config_defaults() {
        let conn = open_test_db();
        let loaded = get_sync_config(&conn);
        assert_eq!(loaded.webdav_url, "");
        assert_eq!(loaded.webdav_remote_path, "/skillspp");
        assert!(!loaded.auto_sync);
        assert_eq!(loaded.auto_sync_interval, 30);
    }

    #[test]
    fn sync_status_roundtrip() {
        let conn = open_test_db();
        let status = SyncStatus {
            last_sync_at: Some("2026-07-20T10:00:00Z".to_string()),
            last_sync_device: Some("MacBook".to_string()),
            last_sync_result: Some("success".to_string()),
            last_sync_error: None,
        };
        set_sync_status(&conn, &status).unwrap();
        let loaded = get_sync_status(&conn);
        assert_eq!(loaded.last_sync_at.as_deref(), Some("2026-07-20T10:00:00Z"));
        assert_eq!(loaded.last_sync_device.as_deref(), Some("MacBook"));
        assert_eq!(loaded.last_sync_result.as_deref(), Some("success"));
        assert!(loaded.last_sync_error.is_none());
    }

    #[test]
    fn sync_base_save_and_load() {
        let conn = open_test_db();
        let snapshot = SyncSnapshot {
            version: 1,
            exported_at: "2026-07-20T10:00:00Z".to_string(),
            device_name: "test".to_string(),
            platform: "macos".to_string(),
            installed_skills: vec![],
            custom_directories: vec![],
            source_preferences: vec![],
            app_settings: BTreeMap::new(),
            lockfile: BTreeMap::new(),
        };
        save_sync_base(&conn, &snapshot).unwrap();
        let loaded = get_sync_base(&conn);
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().device_name, "test");
    }

    // ── Phase 2: 三向合并 ──

    fn make_skill(name: &str, tool: &str, dir: &str, installed_at: &str) -> SyncInstalledSkill {
        SyncInstalledSkill {
            name: name.to_string(),
            tool_name: tool.to_string(),
            directory_relative_path: dir.to_string(),
            source_id: None,
            repo_url: None,
            install_strategy: "git".to_string(),
            content_hash: None,
            canonical_relative_path: None,
            installed_at: installed_at.to_string(),
            author: None,
            description: None,
        }
    }

    fn empty_snapshot() -> SyncSnapshot {
        SyncSnapshot {
            version: 1,
            exported_at: "2026-07-20T10:00:00Z".to_string(),
            device_name: "test".to_string(),
            platform: "macos".to_string(),
            installed_skills: vec![],
            custom_directories: vec![],
            source_preferences: vec![],
            app_settings: BTreeMap::new(),
            lockfile: BTreeMap::new(),
        }
    }

    #[test]
    fn merge_no_remote_returns_local() {
        let local = empty_snapshot();
        let (merged, conflicts) = merge_snapshots(&local, None, None);
        assert_eq!(merged.installed_skills.len(), 0);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn merge_union_of_skills() {
        let mut local = empty_snapshot();
        local.installed_skills.push(make_skill("skill-a", "Claude", ".claude/skills", "2026-07-01T00:00:00Z"));

        let mut remote = empty_snapshot();
        remote.installed_skills.push(make_skill("skill-b", "Claude", ".claude/skills", "2026-07-02T00:00:00Z"));

        let (merged, conflicts) = merge_snapshots(&local, Some(&remote), None);
        assert_eq!(merged.installed_skills.len(), 2);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn merge_takes_newer_for_duplicates() {
        let mut local = empty_snapshot();
        local.installed_skills.push(make_skill("skill-a", "Claude", ".claude/skills", "2026-07-02T00:00:00Z"));

        let mut remote = empty_snapshot();
        remote.installed_skills.push(make_skill("skill-a", "Claude", ".claude/skills", "2026-07-01T00:00:00Z"));

        let (merged, _) = merge_snapshots(&local, Some(&remote), None);
        assert_eq!(merged.installed_skills.len(), 1);
        // 本地较新，应保留本地
        assert_eq!(merged.installed_skills[0].installed_at, "2026-07-02T00:00:00Z");
    }

    #[test]
    fn merge_detects_remote_deleted_conflict() {
        let mut local = empty_snapshot();
        local.installed_skills.push(make_skill("skill-a", "Claude", ".claude/skills", "2026-07-01T00:00:00Z"));

        let mut base = empty_snapshot();
        base.installed_skills.push(make_skill("skill-a", "Claude", ".claude/skills", "2026-07-01T00:00:00Z"));

        let remote = empty_snapshot(); // 远端没有 skill-a

        let (merged, conflicts) = merge_snapshots(&local, Some(&remote), Some(&base));
        // 保留本地
        assert_eq!(merged.installed_skills.len(), 1);
        // 有冲突
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].kind, "remote_deleted");
        assert_eq!(conflicts[0].skill_name, "skill-a");
    }

    #[test]
    fn merge_no_conflict_when_base_empty() {
        let mut local = empty_snapshot();
        local.installed_skills.push(make_skill("skill-a", "Claude", ".claude/skills", "2026-07-01T00:00:00Z"));

        let remote = empty_snapshot();

        // base = None → 不认为是冲突（本地新装）
        let (merged, conflicts) = merge_snapshots(&local, Some(&remote), None);
        assert_eq!(merged.installed_skills.len(), 1);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn merge_directories_union() {
        let mut local = empty_snapshot();
        local.custom_directories.push(SyncDirectory {
            id: "d1".to_string(),
            tool_name: "Claude".to_string(),
            relative_path: ".claude/skills-a".to_string(),
            is_default: false,
        });

        let mut remote = empty_snapshot();
        remote.custom_directories.push(SyncDirectory {
            id: "d2".to_string(),
            tool_name: "Claude".to_string(),
            relative_path: ".claude/skills-b".to_string(),
            is_default: false,
        });

        let (merged, _) = merge_snapshots(&local, Some(&remote), None);
        assert_eq!(merged.custom_directories.len(), 2);
    }

    #[test]
    fn merge_settings_remote_overrides() {
        let mut local = empty_snapshot();
        local.app_settings.insert("key1".to_string(), "local-val".to_string());
        local.app_settings.insert("key2".to_string(), "local-only".to_string());

        let mut remote = empty_snapshot();
        remote.app_settings.insert("key1".to_string(), "remote-val".to_string());

        let (merged, _) = merge_snapshots(&local, Some(&remote), None);
        assert_eq!(merged.app_settings.get("key1"), Some(&"remote-val".to_string()));
        assert_eq!(merged.app_settings.get("key2"), Some(&"local-only".to_string()));
    }
}
