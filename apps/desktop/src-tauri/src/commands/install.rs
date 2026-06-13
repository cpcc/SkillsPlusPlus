use crate::commands::app::DbState;
use crate::models::{InstallPreview, InstallStrategy, InstallTaskRow, InstalledSkillRow};
use crate::services::canonical_store as cstore;
use crate::services::install as svc;
use crate::services::lockfile::{self, LockEntry};
use crate::services::skill_md;
use rusqlite::{params, Connection};
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tauri::State;
use uuid::Uuid;

// ─── 锁文件 / canonical store 视图类型 ─────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct LockEntryView {
    pub source: String,
    #[serde(rename = "sourceType")]
    pub source_type: String,
    #[serde(rename = "sourceUrl")]
    pub source_url: String,
    #[serde(rename = "skillPath")]
    pub skill_path: String,
    #[serde(rename = "skillFolderHash")]
    pub skill_folder_hash: String,
    #[serde(rename = "installedAt")]
    pub installed_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

impl From<LockEntry> for LockEntryView {
    fn from(e: LockEntry) -> Self {
        LockEntryView {
            source: e.source,
            source_type: e.source_type,
            source_url: e.source_url,
            skill_path: e.skill_path,
            skill_folder_hash: e.skill_folder_hash,
            installed_at: e.installed_at,
            updated_at: e.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CanonicalSkillView {
    pub name: String,
    pub path: String,
    pub description: Option<String>,
    #[serde(rename = "hasSkillMd")]
    pub has_skill_md: bool,
}

// ─── Preview ──────────────────────────────────────────────────────────────────

/// Build a preview before installing (shows target path, strategy-specific paths, and any conflict).
pub fn preview_install_inner(
    conn: &Connection,
    skill_name: String,
    repo_url: String,
    directory_id: String,
    strategy: Option<InstallStrategy>,
) -> Result<InstallPreview, String> {
    let dir_path: String = conn
        .query_row(
            "SELECT path FROM ai_tool_directories WHERE id = ?1",
            params![directory_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let strategy = strategy.unwrap_or_default();
    Ok(svc::build_preview(&skill_name, &repo_url, &dir_path, strategy))
}

#[tauri::command]
pub fn preview_install(
    db: State<DbState>,
    skill_name: String,
    repo_url: String,
    directory_id: String,
    strategy: Option<InstallStrategy>,
) -> Result<InstallPreview, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    preview_install_inner(&conn, skill_name, repo_url, directory_id, strategy)
}

/// Install a skill via the requested strategy.
/// `overwrite` = true removes existing dir before installing.
pub async fn install_skill_inner(
    db: Arc<Mutex<Connection>>,
    skill_id: Option<String>,
    skill_name: String,
    repo_url: String,
    directory_id: String,
    overwrite: bool,
    strategy: Option<InstallStrategy>,
    archive_url: Option<String>,
) -> Result<InstallTaskRow, String> {
    let strategy = strategy.unwrap_or_default();

    // Look up directory path + tool_name
    let (dir_path, tool_name): (String, String) = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT path, tool_name FROM ai_tool_directories WHERE id = ?1",
            params![directory_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| e.to_string())?
    };

    let target = svc::target_path(&dir_path, &skill_name);
    let task_id = Uuid::new_v4().to_string();

    // Create task record
    {
        let conn = db.lock().map_err(|e| e.to_string())?;
        svc::create_install_task(
            &conn,
            &task_id,
            skill_id.as_deref(),
            &skill_name,
            &tool_name,
            &directory_id,
            "install",
        )
        .map_err(|e| e.to_string())?;
    }

    // Remove existing if overwrite requested.
    // skills_cli 策略下，target 实际是 agent_link_dir（不含 skill_name），
    // 因此 overwrite 删的是 `<agent_link_dir>/<skill_name>` 的 symlink。
    if overwrite {
        let to_remove = if strategy == InstallStrategy::SkillsCli {
            target.join(&skill_name)
        } else {
            target.clone()
        };
        if to_remove.exists() {
            let _ = svc::remove_skill_dir(&to_remove);
        }
    }

    // 跑安装（阻塞线程）。
    let skill_name_cl = skill_name.clone();
    let repo_url_cl = repo_url.clone();
    let archive_cl = archive_url.clone();
    let dir_path_cl = dir_path.clone();
    let install_result = tokio::task::spawn_blocking(move || {
        svc::install_dispatch(
            strategy,
            &skill_name_cl,
            &repo_url_cl,
            archive_cl.as_deref(),
            std::path::Path::new(&dir_path_cl),
        )
    })
    .await
    .map_err(|e| format!("install task join: {e}"));

    let (success, log_lines, error_msg, content_hash, canonical, _symlink) = match install_result {
        Ok(Ok(outcome)) => {
            // skills_cli：canonical 与 symlink 都必须存在；其它策略：target 目录非空。
            let success = if strategy == InstallStrategy::SkillsCli {
                outcome.canonical_path.as_deref().map(|p| p.exists()).unwrap_or(false)
                    && outcome.symlink_path.as_deref().map(|p| p.exists()).unwrap_or(false)
            } else {
                svc::verify_install(&target)
            };
            let error_msg = if success {
                None
            } else {
                Some("Installation verification failed".to_string())
            };
            (
                success,
                outcome.log_lines.clone(),
                error_msg,
                Some(outcome.content_hash),
                outcome.canonical_path,
                outcome.symlink_path,
            )
        }
        Ok(Err(e)) => (false, vec![e.clone()], Some(e), None, None, None),
        Err(e) => (false, vec![e.clone()], Some(e), None, None, None),
    };

    // Update task + record installed skill
    {
        let conn = db.lock().map_err(|e| e.to_string())?;
        svc::finish_install_task(&conn, &task_id, success, error_msg.as_deref())
            .map_err(|e| e.to_string())?;

        if success {
            let installed_id = Uuid::new_v4().to_string();
            svc::record_installed_skill(
                &conn,
                &installed_id,
                skill_id.as_deref(),
                &skill_name,
                &tool_name,
                &directory_id,
                None,
                Some(&repo_url),
                strategy,
                content_hash.as_deref(),
                canonical
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string())
                    .as_deref(),
            )
            .map_err(|e| e.to_string())?;
        }
    }

    Ok(InstallTaskRow {
        id: task_id,
        skill_id,
        skill_name,
        tool_name,
        directory_id,
        action: "install".to_string(),
        status: if success { "success".to_string() } else { "failed".to_string() },
        started_at: None,
        finished_at: None,
        error_message: error_msg,
        log_lines,
    })
}

#[tauri::command]
pub async fn install_skill(
    db: State<'_, DbState>,
    skill_id: Option<String>,
    skill_name: String,
    repo_url: String,
    directory_id: String,
    overwrite: bool,
    strategy: Option<InstallStrategy>,
    archive_url: Option<String>,
) -> Result<InstallTaskRow, String> {
    install_skill_inner(
        std::sync::Arc::clone(&db.0),
        skill_id,
        skill_name,
        repo_url,
        directory_id,
        overwrite,
        strategy,
        archive_url,
    )
    .await
}

/// Reinstall = remove + install.
pub async fn reinstall_skill_inner(
    db: Arc<Mutex<Connection>>,
    skill_id: Option<String>,
    skill_name: String,
    repo_url: String,
    directory_id: String,
    strategy: Option<InstallStrategy>,
    archive_url: Option<String>,
) -> Result<InstallTaskRow, String> {
    install_skill_inner(db, skill_id, skill_name, repo_url, directory_id, true, strategy, archive_url).await
}

#[tauri::command]
pub async fn reinstall_skill(
    db: State<'_, DbState>,
    skill_id: Option<String>,
    skill_name: String,
    repo_url: String,
    directory_id: String,
    strategy: Option<InstallStrategy>,
    archive_url: Option<String>,
) -> Result<InstallTaskRow, String> {
    reinstall_skill_inner(std::sync::Arc::clone(&db.0), skill_id, skill_name, repo_url, directory_id, strategy, archive_url).await
}

/// Uninstall: remove directory + DB record. skills_cli 只删 symlink，保留 canonical。
pub fn uninstall_skill_inner(
    conn: &Connection,
    skill_name: String,
    directory_id: String,
) -> Result<InstallTaskRow, String> {
    let (dir_path, strategy_s): (String, String) = conn
        .query_row(
            "SELECT d.path, COALESCE(i.install_strategy, 'git') \
             FROM ai_tool_directories d \
             LEFT JOIN installed_skills i ON i.directory_id = d.id AND i.name = ?1 \
             WHERE d.id = ?2",
            params![skill_name, directory_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| e.to_string())?;
    let strategy = InstallStrategy::parse(&strategy_s);

    let target = svc::target_path(&dir_path, &skill_name);
    let task_id = Uuid::new_v4().to_string();

    let (success, error_msg) = match svc::remove_skill_dir(&target) {
        Ok(_) => (true, None),
        Err(e) => (false, Some(e)),
    };

    if success {
        svc::remove_installed_skill(conn, &skill_name, &directory_id)
            .map_err(|e| e.to_string())?;
    }
    // Persist task record
    conn.execute(
        "INSERT INTO install_tasks \
         (id, skill_name, tool_name, directory_id, action, status, started_at, finished_at, error_message) \
         SELECT ?1, ?2, tool_name, ?3, 'uninstall', ?4, datetime('now'), datetime('now'), ?5 \
         FROM ai_tool_directories WHERE id = ?3",
        params![
            task_id,
            skill_name,
            directory_id,
            if success { "success" } else { "failed" },
            error_msg,
        ],
    )
    .map_err(|e| e.to_string())?;

    // skills_cli 卸载：仅删 symlink（上面 remove_skill_dir 已处理），保留 canonical。
    // 其它策略：直接删目录。两者行为已在上面统一完成，这里无需额外动作。
    let _ = strategy;

    Ok(InstallTaskRow {
        id: task_id,
        skill_id: None,
        skill_name,
        tool_name: String::new(),
        directory_id,
        action: "uninstall".to_string(),
        status: if success { "success".to_string() } else { "failed".to_string() },
        started_at: None,
        finished_at: None,
        error_message: error_msg,
        log_lines: vec![],
    })
}

#[tauri::command]
pub fn uninstall_skill(
    db: State<DbState>,
    skill_name: String,
    directory_id: String,
) -> Result<InstallTaskRow, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    uninstall_skill_inner(&conn, skill_name, directory_id)
}

/// List all installed skills (with real-time filesystem status check).
pub fn list_installed_skills_inner(conn: &Connection) -> Result<Vec<InstalledSkillRow>, String> {
    svc::refresh_installed_status(conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_installed_skills(db: State<DbState>) -> Result<Vec<InstalledSkillRow>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    list_installed_skills_inner(&conn)
}

/// Refresh all installed skills status by scanning filesystem.
pub fn refresh_installed_skills_inner(conn: &Connection) -> Result<Vec<InstalledSkillRow>, String> {
    svc::refresh_installed_status(conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn refresh_installed_skills(db: State<DbState>) -> Result<Vec<InstalledSkillRow>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    refresh_installed_skills_inner(&conn)
}

/// Check for updates on a single installed skill and update its status.
pub fn check_skill_update_inner(
    conn: &Connection,
    skill_id: String,
) -> Result<InstalledSkillRow, String> {
    svc::refresh_single_skill_status(conn, &skill_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Skill not found".to_string())
}

#[tauri::command]
pub fn check_skill_update(
    db: State<DbState>,
    skill_id: String,
) -> Result<InstalledSkillRow, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    check_skill_update_inner(&conn, skill_id)
}

/// List recent install tasks (last 50).
pub fn list_install_tasks_inner(conn: &Connection) -> Result<Vec<InstallTaskRow>, String> {
    let raw = svc::list_install_tasks(conn, 50).map_err(|e| e.to_string())?;
    Ok(raw.into_iter().map(|(id, skill_name, action, status, error_message)| InstallTaskRow {
        id,
        skill_id: None,
        skill_name,
        tool_name: String::new(),
        directory_id: String::new(),
        action,
        status,
        started_at: None,
        finished_at: None,
        error_message,
        log_lines: vec![],
    }).collect())
}

#[tauri::command]
pub fn list_install_tasks(db: State<DbState>) -> Result<Vec<InstallTaskRow>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    list_install_tasks_inner(&conn)
}

/// Check if git is available on this system.
pub fn check_git_available_inner() -> bool {
    std::process::Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[tauri::command]
pub fn check_git_available() -> bool {
    check_git_available_inner()
}

/// 直接读 `~/.agents/.skill-lock.json`（与 `npx skills` 互通）。
#[tauri::command]
pub fn read_lockfile() -> Result<BTreeMap<String, LockEntryView>, String> {
    Ok(lockfile::read_lockfile()
        .into_iter()
        .map(|(k, v)| (k, LockEntryView::from(v)))
        .collect())
}

/// 列 `~/.agents/skills/*/`，每个目录尝试解析 SKILL.md frontmatter。
#[tauri::command]
pub fn list_canonical_skills() -> Result<Vec<CanonicalSkillView>, String> {
    let root = cstore::canonical_root().ok_or_else(|| "cannot resolve home dir".to_string())?;
    if !root.exists() {
        return Ok(vec![]);
    }
    let mut out = vec![];
    let rd = std::fs::read_dir(&root).map_err(|e| format!("readdir canonical: {e}"))?;
    for entry in rd.flatten() {
        let p = entry.path();
        if !p.is_dir() {
            continue;
        }
        let name = match p.file_name().and_then(|s| s.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        let manifest = skill_md::parse_skill_md(&p);
        out.push(CanonicalSkillView {
            name: name.clone(),
            path: p.to_string_lossy().to_string(),
            description: manifest.as_ref().and_then(|m| m.description.clone()),
            has_skill_md: manifest.is_some(),
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}
