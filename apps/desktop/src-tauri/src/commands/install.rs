use crate::commands::app::DbState;
use crate::models::{InstallPreview, InstallTaskRow, InstalledSkillRow};
use crate::services::install as svc;
use rusqlite::params;
use std::path::PathBuf;
use tauri::State;
use uuid::Uuid;

/// Build a preview before installing (shows target path and any conflict).
#[tauri::command]
pub fn preview_install(
    db: State<DbState>,
    skill_name: String,
    repo_url: String,
    directory_id: String,
) -> Result<InstallPreview, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let dir_path: String = conn
        .query_row(
            "SELECT path FROM ai_tool_directories WHERE id = ?1",
            params![directory_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    Ok(svc::build_preview(&skill_name, &repo_url, &dir_path))
}

/// Install a skill: git clone into target directory.
/// `overwrite` = true removes existing dir before cloning.
#[tauri::command]
pub async fn install_skill(
    db: State<'_, DbState>,
    skill_id: Option<String>,
    skill_name: String,
    repo_url: String,
    directory_id: String,
    overwrite: bool,
) -> Result<InstallTaskRow, String> {
    // Look up directory path
    let (dir_path, tool_name): (String, String) = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
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
        let conn = db.0.lock().map_err(|e| e.to_string())?;
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

    // Remove existing if overwrite requested
    if overwrite && target.exists() {
        svc::remove_skill_dir(&target)?;
    }

    // Run git clone (blocking — acceptable for desktop install UX)
    let (success, log_lines, error_msg) = tokio::task::spawn_blocking({
        let repo_url = repo_url.clone();
        let target = target.clone();
        move || match svc::git_clone(&repo_url, &target) {
            Ok(lines) => (true, lines, None),
            Err(e) => (false, vec![e.clone()], Some(e)),
        }
    })
    .await
    .map_err(|e| e.to_string())?;

    // Verify
    let verified = success && svc::verify_install(&target);
    let final_success = success && verified;
    let final_error = if !final_success && error_msg.is_none() {
        Some("Installation verification failed".to_string())
    } else {
        error_msg
    };

    // Update task + record installed skill
    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        svc::finish_install_task(&conn, &task_id, final_success, final_error.as_deref())
            .map_err(|e| e.to_string())?;

        if final_success {
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
        status: if final_success { "success".to_string() } else { "failed".to_string() },
        started_at: None,
        finished_at: None,
        error_message: final_error,
        log_lines,
    })
}

/// Reinstall = remove + install.
#[tauri::command]
pub async fn reinstall_skill(
    db: State<'_, DbState>,
    skill_id: Option<String>,
    skill_name: String,
    repo_url: String,
    directory_id: String,
) -> Result<InstallTaskRow, String> {
    install_skill(db, skill_id, skill_name, repo_url, directory_id, true).await
}

/// Uninstall: remove directory + DB record.
#[tauri::command]
pub fn uninstall_skill(
    db: State<DbState>,
    skill_name: String,
    directory_id: String,
) -> Result<InstallTaskRow, String> {
    let dir_path: String = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT path FROM ai_tool_directories WHERE id = ?1",
            params![directory_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?
    };

    let target = svc::target_path(&dir_path, &skill_name);
    let task_id = Uuid::new_v4().to_string();

    let (success, error_msg) = match svc::remove_skill_dir(&target) {
        Ok(_) => (true, None),
        Err(e) => (false, Some(e)),
    };

    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        if success {
            svc::remove_installed_skill(&conn, &skill_name, &directory_id)
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
    }

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

/// List all installed skills.
#[tauri::command]
pub fn list_installed_skills(db: State<DbState>) -> Result<Vec<InstalledSkillRow>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    svc::list_installed_skills(&conn).map_err(|e| e.to_string())
}

/// List recent install tasks (last 50).
#[tauri::command]
pub fn list_install_tasks(db: State<DbState>) -> Result<Vec<InstallTaskRow>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let raw = svc::list_install_tasks(&conn, 50).map_err(|e| e.to_string())?;
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

/// Check if git is available on this system.
#[tauri::command]
pub fn check_git_available() -> bool {
    std::process::Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// Helper used by reinstall to resolve directory path
fn _dir_path_for_id(conn: &rusqlite::Connection, directory_id: &str) -> Result<PathBuf, String> {
    conn.query_row(
        "SELECT path FROM ai_tool_directories WHERE id = ?1",
        params![directory_id],
        |row| row.get::<_, String>(0),
    )
    .map(PathBuf::from)
    .map_err(|e| e.to_string())
}
