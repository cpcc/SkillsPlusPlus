use crate::models::{ConflictInfo, InstallPreview, InstalledSkillRow};
use rusqlite::{params, Connection, Result as SqliteResult};
use std::path::{Path, PathBuf};

// ─── Preview ──────────────────────────────────────────────────────────────────

/// Build the target path: `<directory_path>/<skill_name>`
pub fn target_path(directory_path: &str, skill_name: &str) -> PathBuf {
    let safe_name = skill_name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '-' })
        .collect::<String>();
    PathBuf::from(directory_path).join(safe_name)
}

pub fn build_preview(
    skill_name: &str,
    repo_url: &str,
    directory_path: &str,
) -> InstallPreview {
    let tpath = target_path(directory_path, skill_name);
    let conflict = if tpath.exists() {
        Some(ConflictInfo {
            existing_path: tpath.to_string_lossy().to_string(),
            kind: if tpath.is_dir() {
                "existing_dir".to_string()
            } else {
                "existing_file".to_string()
            },
        })
    } else {
        None
    };
    InstallPreview {
        skill_name: skill_name.to_string(),
        repo_url: repo_url.to_string(),
        target_path: tpath.to_string_lossy().to_string(),
        conflict,
    }
}

// ─── Install ──────────────────────────────────────────────────────────────────

/// Returns log lines on success, or an error string.
pub fn git_clone(repo_url: &str, target: &Path) -> Result<Vec<String>, String> {
    let output = std::process::Command::new("git")
        .args(["clone", "--depth=1", "--progress", repo_url, &target.to_string_lossy()])
        .output()
        .map_err(|e| format!("Failed to run git: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let mut lines: Vec<String> = Vec::new();
    for line in stdout.lines().chain(stderr.lines()) {
        if !line.trim().is_empty() {
            lines.push(line.to_string());
        }
    }

    if output.status.success() {
        Ok(lines)
    } else {
        Err(lines.last().cloned().unwrap_or_else(|| "git clone failed".to_string()))
    }
}

/// Verify the installation by checking the target directory exists and is non-empty.
pub fn verify_install(target: &Path) -> bool {
    target.exists()
        && target.is_dir()
        && std::fs::read_dir(target).map(|mut d| d.next().is_some()).unwrap_or(false)
}

/// Remove an installed skill directory.
pub fn remove_skill_dir(target: &Path) -> Result<(), String> {
    if target.exists() {
        std::fs::remove_dir_all(target)
            .map_err(|e| format!("Failed to remove directory: {e}"))
    } else {
        Ok(())
    }
}

// ─── DB helpers ───────────────────────────────────────────────────────────────

pub fn create_install_task(
    conn: &Connection,
    id: &str,
    skill_id: Option<&str>,
    skill_name: &str,
    tool_name: &str,
    directory_id: &str,
    action: &str,
) -> SqliteResult<()> {
    conn.execute(
        "INSERT INTO install_tasks \
         (id, skill_id, skill_name, tool_name, directory_id, action, status, started_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'running', datetime('now'))",
        params![id, skill_id, skill_name, tool_name, directory_id, action],
    )?;
    Ok(())
}

pub fn finish_install_task(
    conn: &Connection,
    id: &str,
    success: bool,
    error_message: Option<&str>,
) -> SqliteResult<()> {
    let status = if success { "success" } else { "failed" };
    conn.execute(
        "UPDATE install_tasks \
         SET status = ?1, finished_at = datetime('now'), error_message = ?2 \
         WHERE id = ?3",
        params![status, error_message, id],
    )?;
    Ok(())
}

pub fn record_installed_skill(
    conn: &Connection,
    id: &str,
    skill_id: Option<&str>,
    skill_name: &str,
    tool_name: &str,
    directory_id: &str,
    source_id: Option<&str>,
    repo_url: Option<&str>,
) -> SqliteResult<()> {
    conn.execute(
        "INSERT OR REPLACE INTO installed_skills \
         (id, skill_id, name, tool_name, directory_id, source_id, repo_url, installed_at, status) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now'), 'ok')",
        params![id, skill_id, skill_name, tool_name, directory_id, source_id, repo_url],
    )?;
    Ok(())
}

pub fn remove_installed_skill(conn: &Connection, skill_name: &str, directory_id: &str) -> SqliteResult<()> {
    conn.execute(
        "DELETE FROM installed_skills WHERE name = ?1 AND directory_id = ?2",
        params![skill_name, directory_id],
    )?;
    Ok(())
}

pub fn list_installed_skills(conn: &Connection) -> SqliteResult<Vec<InstalledSkillRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, skill_id, name, tool_name, directory_id, source_id, repo_url, installed_at, status \
         FROM installed_skills ORDER BY installed_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(InstalledSkillRow {
            id: row.get(0)?,
            skill_id: row.get(1)?,
            name: row.get(2)?,
            tool_name: row.get(3)?,
            directory_id: row.get(4)?,
            source_id: row.get(5)?,
            repo_url: row.get(6)?,
            installed_at: row.get(7)?,
            status: row.get(8)?,
        })
    })?;
    rows.collect()
}

pub fn list_install_tasks(conn: &Connection, limit: i64) -> SqliteResult<Vec<(String, String, String, String, Option<String>)>> {
    let mut stmt = conn.prepare(
        "SELECT id, skill_name, action, status, error_message \
         FROM install_tasks ORDER BY created_at DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Option<String>>(4)?,
        ))
    })?;
    rows.collect()
}
