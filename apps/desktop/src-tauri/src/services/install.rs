use crate::models::{ConflictInfo, InstallPreview, InstalledSkillRow};
use rusqlite::{params, Connection, Result as SqliteResult};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

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
        "SELECT i.id, i.skill_id, i.name, i.tool_name, i.directory_id, \
                COALESCE(d.path, '') as directory_path, \
                i.source_id, i.repo_url, i.installed_at, i.status \
         FROM installed_skills i \
         LEFT JOIN ai_tool_directories d ON i.directory_id = d.id \
         ORDER BY i.installed_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(InstalledSkillRow {
            id: row.get(0)?,
            skill_id: row.get(1)?,
            name: row.get(2)?,
            tool_name: row.get(3)?,
            directory_id: row.get(4)?,
            directory_path: row.get(5)?,
            source_id: row.get(6)?,
            repo_url: row.get(7)?,
            installed_at: row.get(8)?,
            status: row.get(9)?,
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

// ─── Status refresh ──────────────────────────────────────────────────────────

/// Compute real status for an installed skill by checking the filesystem.
/// Returns "ok", "missing", or "changed".
fn compute_skill_status(skill_name: &str, directory_path: &str) -> &'static str {
    if directory_path.is_empty() {
        return "missing";
    }
    let target = target_path(directory_path, skill_name);
    if !target.exists() || !target.is_dir() {
        return "missing";
    }
    // Check if directory is non-empty
    let has_content = std::fs::read_dir(&target)
        .map(|mut d| d.next().is_some())
        .unwrap_or(false);
    if !has_content {
        return "changed";
    }
    "ok"
}

/// Refresh status of all installed skills by scanning filesystem, then update DB.
/// Returns the updated list.
pub fn refresh_installed_status(conn: &Connection) -> SqliteResult<Vec<InstalledSkillRow>> {
    // Load all installed skills with directory paths
    let mut skills = list_installed_skills(conn)?;
    for skill in &mut skills {
        let real_status = compute_skill_status(&skill.name, &skill.directory_path);
        if real_status != skill.status {
            // Update DB
            conn.execute(
                "UPDATE installed_skills SET status = ?1 WHERE id = ?2",
                params![real_status, skill.id],
            )?;
            skill.status = real_status.to_string();
        }
    }
    Ok(skills)
}

// ─── Check update ────────────────────────────────────────────────────────────

/// Check if a skill's git repo has updates available.
/// Returns true if remote has newer commits than local.
pub fn check_update_available(skill_dir: &Path) -> Result<bool, String> {
    if !skill_dir.join(".git").exists() {
        return Ok(false); // Not a git repo, can't check
    }

    // Fetch remote
    let fetch_output = std::process::Command::new("git")
        .args(["fetch", "origin", "--dry-run"])
        .current_dir(skill_dir)
        .output()
        .map_err(|e| format!("Failed to run git fetch: {e}"))?;

    if !fetch_output.status.success() {
        return Err(
            String::from_utf8_lossy(&fetch_output.stderr)
                .lines()
                .last()
                .unwrap_or("git fetch failed")
                .to_string(),
        );
    }

    // Compare local HEAD with remote origin/HEAD
    let status_output = std::process::Command::new("git")
        .args(["status", "-uno", "--porcelain"])
        .current_dir(skill_dir)
        .output()
        .map_err(|e| format!("Failed to run git status: {e}"))?;

    let status_text = String::from_utf8_lossy(&status_output.stdout);
    let has_behind = status_text.contains("behind");

    // Also check rev-list count
    let rev_output = std::process::Command::new("git")
        .args(["rev-list", "--count", "HEAD..origin/HEAD"])
        .current_dir(skill_dir)
        .output();

    match rev_output {
        Ok(out) if out.status.success() => {
            let count_str = String::from_utf8_lossy(&out.stdout);
            let count: u32 = count_str.trim().parse().unwrap_or(0);
            Ok(count > 0 || has_behind)
        }
        _ => Ok(has_behind),
    }
}

/// Update a single installed skill's status in DB and return the updated row.
pub fn refresh_single_skill_status(
    conn: &Connection,
    skill_id: &str,
) -> SqliteResult<Option<InstalledSkillRow>> {
    // Get skill info
    let skill = conn.query_row(
        "SELECT i.id, i.skill_id, i.name, i.tool_name, i.directory_id, \
                COALESCE(d.path, ''), i.source_id, i.repo_url, i.installed_at, i.status \
         FROM installed_skills i \
         LEFT JOIN ai_tool_directories d ON i.directory_id = d.id \
         WHERE i.id = ?1",
        params![skill_id],
        |row| {
            Ok(InstalledSkillRow {
                id: row.get(0)?,
                skill_id: row.get(1)?,
                name: row.get(2)?,
                tool_name: row.get(3)?,
                directory_id: row.get(4)?,
                directory_path: row.get(5)?,
                source_id: row.get(6)?,
                repo_url: row.get(7)?,
                installed_at: row.get(8)?,
                status: row.get(9)?,
            })
        },
    );

    match skill {
        Ok(mut s) => {
            let mut new_status = compute_skill_status(&s.name, &s.directory_path).to_string();

            // If ok and has repo_url, also check for updates
            if new_status == "ok" {
                if let Some(ref repo_url) = s.repo_url {
                    if !repo_url.is_empty() {
                        let target = target_path(&s.directory_path, &s.name);
                        if let Ok(has_update) = check_update_available(&target) {
                            if has_update {
                                new_status = "update_available".to_string();
                            }
                        }
                    }
                }
            }

            if new_status != s.status {
                conn.execute(
                    "UPDATE installed_skills SET status = ?1 WHERE id = ?2",
                    params![new_status, s.id],
                )?;
                s.status = new_status;
            }
            Ok(Some(s))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Get a stable timestamp string for log lines.
pub fn _now_ts() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("[{secs}]")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn target_path_sanitizes_name() {
        let p = target_path("/home/user/.cursor/rules", "my-skill_v1.0");
        assert_eq!(p, PathBuf::from("/home/user/.cursor/rules/my-skill_v1.0"));
    }

    #[test]
    fn target_path_replaces_special_chars() {
        let p = target_path("/tmp", "skill@name#with$special");
        let name = p.file_name().unwrap().to_str().unwrap();
        assert!(!name.contains('@'));
        assert!(!name.contains('#'));
        assert!(!name.contains('$'));
    }

    #[test]
    fn build_preview_no_conflict() {
        let preview = build_preview("test-skill", "https://github.com/x/y", "/nonexistent/path");
        assert_eq!(preview.skill_name, "test-skill");
        assert_eq!(preview.repo_url, "https://github.com/x/y");
        assert!(preview.conflict.is_none());
    }

    #[test]
    fn build_preview_detects_conflict() {
        let tmp = std::env::temp_dir().join("skills_pp_test_conflict");
        let _ = fs::create_dir_all(&tmp);
        let skill_dir = tmp.join("my-skill");
        let _ = fs::create_dir_all(&skill_dir);

        let preview = build_preview(
            "my-skill",
            "https://github.com/x/y",
            &tmp.to_string_lossy(),
        );
        assert!(preview.conflict.is_some());
        let conflict = preview.conflict.unwrap();
        assert_eq!(conflict.kind, "existing_dir");

        // cleanup
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn compute_status_missing_when_no_dir() {
        assert_eq!(compute_skill_status("test", ""), "missing");
        assert_eq!(compute_skill_status("test", "/nonexistent/path/12345"), "missing");
    }

    #[test]
    fn compute_status_ok_when_dir_exists() {
        let tmp = std::env::temp_dir().join("skills_pp_test_status_ok");
        let _ = fs::create_dir_all(&tmp);
        let skill_dir = tmp.join("my-skill");
        let _ = fs::create_dir_all(&skill_dir);
        // Create a file so it's non-empty
        let _ = fs::write(skill_dir.join("README.md"), "# test");

        assert_eq!(compute_skill_status("my-skill", &tmp.to_string_lossy()), "ok");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn compute_status_changed_when_empty_dir() {
        let tmp = std::env::temp_dir().join("skills_pp_test_status_changed");
        let _ = fs::create_dir_all(&tmp);
        let skill_dir = tmp.join("empty-skill");
        let _ = fs::create_dir_all(&skill_dir);

        assert_eq!(compute_skill_status("empty-skill", &tmp.to_string_lossy()), "changed");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn verify_install_checks_nonempty_dir() {
        let tmp = std::env::temp_dir().join("skills_pp_test_verify");
        let _ = fs::create_dir_all(&tmp);
        let _ = fs::write(tmp.join("file.txt"), "content");

        assert!(verify_install(&tmp));

        // Empty dir fails
        let empty = std::env::temp_dir().join("skills_pp_test_verify_empty");
        let _ = fs::create_dir_all(&empty);
        assert!(!verify_install(&empty));

        // Nonexistent fails
        assert!(!verify_install(Path::new("/nonexistent/path/12345")));

        let _ = fs::remove_dir_all(&tmp);
        let _ = fs::remove_dir_all(&empty);
    }

    #[test]
    fn remove_skill_dir_cleans_up() {
        let tmp = std::env::temp_dir().join("skills_pp_test_remove");
        let _ = fs::create_dir_all(&tmp);
        let _ = fs::write(tmp.join("file.txt"), "content");

        assert!(remove_skill_dir(&tmp).is_ok());
        assert!(!tmp.exists());
    }

    #[test]
    fn remove_skill_dir_noop_when_missing() {
        assert!(remove_skill_dir(Path::new("/nonexistent/path/12345")).is_ok());
    }
}
