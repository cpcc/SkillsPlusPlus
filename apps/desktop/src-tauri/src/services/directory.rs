use crate::models::{DirectoryRow, ScanResult};
use rusqlite::{Connection, Result as SqliteResult};
use std::path::{Path, PathBuf};

// ─── Tool rules ───────────────────────────────────────────────────────────────

struct ToolRule {
    tool_name: &'static str,
    candidate_paths: &'static [&'static str], // relative to home dir
}

const TOOL_RULES: &[ToolRule] = &[
    ToolRule {
        tool_name: "Codex",
        candidate_paths: &[".codex/skills", ".agents/skills"],
    },
    ToolRule {
        tool_name: "Claude",
        candidate_paths: &[".claude/skills"],
    },
    ToolRule {
        tool_name: "Cursor",
        candidate_paths: &[".cursor/rules", ".cursor/skills"],
    },
    ToolRule {
        tool_name: "OpenCode",
        candidate_paths: &[".opencode/skills"],
    },
    ToolRule {
        tool_name: "GitHub Copilot",
        candidate_paths: &[
            ".copilot/installed-plugins/superpowers-marketplace/superpowers/skills",
        ],
    },
    ToolRule {
        tool_name: "Antigravity",
        candidate_paths: &[".antigravity/skills"],
    },
    ToolRule {
        tool_name: "Gemini CLI",
        candidate_paths: &[".gemini/skills"],
    },
    ToolRule {
        tool_name: "Kimi Code CLI",
        candidate_paths: &[".kimi/skills"],
    },
    ToolRule {
        tool_name: "OpenClaw",
        candidate_paths: &[".openclaw/skills"],
    },
    ToolRule {
        tool_name: "CodeBuddy",
        candidate_paths: &[".codebuddy/skills"],
    },
];

// ─── Path expansion ───────────────────────────────────────────────────────────

/// Expand a path relative to the user's home directory.
pub fn expand_path(relative: &str) -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    Some(home.join(relative))
}

/// Count skill entries (subdirectories or recognised skill files) in a directory.
pub fn count_skills(path: &Path) -> i64 {
    if !path.exists() {
        return 0;
    }
    std::fs::read_dir(path)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    let p = e.path();
                    p.is_dir()
                        || p.extension()
                            .map(|x| x == "md" || x == "yaml" || x == "yml")
                            .unwrap_or(false)
                })
                .count() as i64
        })
        .unwrap_or(0)
}

/// Check if a directory is writable by creating and removing a temp probe file.
pub fn is_writable(path: &Path) -> bool {
    if !path.exists() {
        return false;
    }
    let test = path.join(".skills_pp_write_test");
    match std::fs::write(&test, b"") {
        Ok(_) => {
            let _ = std::fs::remove_file(&test);
            true
        }
        Err(_) => false,
    }
}

// ─── Scan ─────────────────────────────────────────────────────────────────────

pub fn scan_directory(id: &str, tool_name: &str, path_str: &str) -> ScanResult {
    let path = PathBuf::from(path_str);
    let exists = path.exists();
    let writable = if exists { is_writable(&path) } else { false };
    let skill_count = if exists { count_skills(&path) } else { 0 };

    ScanResult {
        id: id.to_string(),
        tool_name: tool_name.to_string(),
        path: path_str.to_string(),
        exists,
        writable,
        skill_count,
    }
}

// ─── Seed default directories ─────────────────────────────────────────────────

/// Insert built-in tool directories into DB if they don't already exist.
pub fn seed_default_directories(conn: &Connection) -> SqliteResult<()> {
    for rule in TOOL_RULES {
        for (i, relative_path) in rule.candidate_paths.iter().enumerate() {
            let Some(abs_path) = expand_path(relative_path) else {
                continue;
            };
            let path_str = abs_path.to_string_lossy().to_string();
            let id = format!(
                "{}-{}",
                rule.tool_name.to_lowercase().replace(' ', "-"),
                i
            );
            conn.execute(
                "INSERT OR IGNORE INTO ai_tool_directories \
                 (id, tool_name, path, is_default, is_detected, writable, enabled, skill_count) \
                 VALUES (?1, ?2, ?3, ?4, 0, 0, 1, 0)",
                rusqlite::params![
                    id,
                    rule.tool_name,
                    path_str,
                    if i == 0 { 1i64 } else { 0i64 },
                ],
            )?;
        }
    }
    Ok(())
}

// ─── Repository helpers ───────────────────────────────────────────────────────

pub fn list_directories(conn: &Connection) -> SqliteResult<Vec<DirectoryRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, tool_name, path, is_default, is_detected, writable, enabled, skill_count \
         FROM ai_tool_directories ORDER BY tool_name, is_default DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(DirectoryRow {
            id: row.get(0)?,
            tool_name: row.get(1)?,
            path: row.get(2)?,
            is_default: row.get::<_, i64>(3)? != 0,
            is_detected: row.get::<_, i64>(4)? != 0,
            writable: row.get::<_, i64>(5)? != 0,
            enabled: row.get::<_, i64>(6)? != 0,
            skill_count: row.get(7)?,
        })
    })?;
    rows.collect()
}

pub fn update_scan_result(conn: &Connection, result: &ScanResult) -> SqliteResult<()> {
    conn.execute(
        "UPDATE ai_tool_directories \
         SET is_detected = ?1, writable = ?2, skill_count = ?3 \
         WHERE id = ?4",
        rusqlite::params![
            result.exists as i64,
            result.writable as i64,
            result.skill_count,
            result.id,
        ],
    )?;
    Ok(())
}
