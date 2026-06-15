use crate::models::{DirectoryRow, ScanResult};
use rusqlite::{Connection, Result as SqliteResult};
use std::path::{Path, PathBuf};

// ─── Tool rules ───────────────────────────────────────────────────────────────

struct ToolRule {
    tool_name: &'static str,
    candidate_paths: &'static [&'static str], // relative to home dir
}

const TOOL_RULES: &[ToolRule] = &[
    // 通用共享目录（~/.agents/skills）：被 Amp / Cline / Codex / Cursor /
    // Deep Agents / Gemini CLI / GitHub Copilot / Kimi / OpenCode / Warp / Zed
    // 等几乎所有主流 AI 工具读取，因此以单条 "Agents" 条目展示，避免重复。
    ToolRule {
        tool_name: "Agents",
        candidate_paths: &[".agents/skills"],
    },
    ToolRule {
        tool_name: "Codex",
        candidate_paths: &[".codex/skills"],
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
        candidate_paths: &[".copilot/skills"],
    },
    ToolRule {
        tool_name: "Antigravity",
        candidate_paths: &[".antigravity/skills"],
    },
    // Antigravity CLI（与 Gemini CLI 共享 ~/.gemini 根目录的独立子目录）
    ToolRule {
        tool_name: "Antigravity CLI",
        candidate_paths: &[".gemini/antigravity/skills"],
    },
    ToolRule {
        tool_name: "Amp",
        candidate_paths: &[".config/agents/skills"],
    },
    ToolRule {
        tool_name: "Cline",
        candidate_paths: &[".cline/skills"],
    },
    ToolRule {
        tool_name: "Warp",
        candidate_paths: &[".warp/skills"],
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

    // 老版本把 `~/.agents/skills` 作为 Codex 的第二条候选路径写入，生成
    // 固定 id `codex-1`。新版本把 `.agents/skills` 提升为独立的 "Agents"
    // 条目（id `agents-0`）。需要清理遗留的 `codex-1` 行，但 installed_skills
    // 有 FK 引用 directory_id（ON DELETE NO ACTION），所以必须先把历史安装
    // 记录重新归属到 `agents-0`（同一物理目录，迁移是无副作用的），再删除 codex-1。
    // 必须在 INSERT 循环之后执行，确保 agents-0 已存在以满足 FK 约束。
    conn.execute(
        "UPDATE installed_skills SET directory_id = 'agents-0' WHERE directory_id = 'codex-1'",
        [],
    )?;
    conn.execute("DELETE FROM ai_tool_directories WHERE id = 'codex-1'", [])?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn expand_path_returns_some_for_relative() {
        let result = expand_path(".cursor/rules");
        assert!(result.is_some());
        let path = result.unwrap();
        assert!(path.to_string_lossy().contains(".cursor"));
    }

    #[test]
    fn count_skills_returns_zero_for_nonexistent() {
        assert_eq!(count_skills(Path::new("/nonexistent/path/12345")), 0);
    }

    #[test]
    fn count_skills_counts_dirs_and_md_files() {
        let tmp = std::env::temp_dir().join("skills_pp_test_count");
        let _ = fs::create_dir_all(&tmp);
        let _ = fs::create_dir(tmp.join("skill-a"));
        let _ = fs::create_dir(tmp.join("skill-b"));
        let _ = fs::write(tmp.join("standalone.md"), "# skill");
        let _ = fs::write(tmp.join("ignore.txt"), "not a skill");

        assert_eq!(count_skills(&tmp), 3); // 2 dirs + 1 .md

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn is_writable_true_for_temp_dir() {
        let tmp = std::env::temp_dir().join("skills_pp_test_writable");
        let _ = fs::create_dir_all(&tmp);
        assert!(is_writable(&tmp));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn is_writable_false_for_nonexistent() {
        assert!(!is_writable(Path::new("/nonexistent/path/12345")));
    }

    #[test]
    fn scan_directory_detects_existing() {
        let tmp = std::env::temp_dir().join("skills_pp_test_scan");
        let _ = fs::create_dir_all(&tmp);

        let result = scan_directory("test-0", "TestTool", &tmp.to_string_lossy());
        assert!(result.exists);
        assert!(result.writable);
        assert_eq!(result.tool_name, "TestTool");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn scan_directory_marks_nonexistent() {
        let result = scan_directory("test-0", "TestTool", "/nonexistent/path/12345");
        assert!(!result.exists);
        assert!(!result.writable);
        assert_eq!(result.skill_count, 0);
    }
}
