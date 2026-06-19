use crate::models::{DirectoryRow, FileNodeKind, FileTreeNode, ScanResult};
use rusqlite::{Connection, Result as SqliteResult};
use std::collections::HashSet;
use std::fs;
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

// ─── Directory tree walker (drawer) ───────────────────────────────────────────

/// Names that should be hidden in the drawer tree (dotfiles plus a few
/// OS-generated files that would only add noise).
fn is_hidden_name(name: &str) -> bool {
    name.starts_with('.')
}

/// Whether a directory entry contains a `SKILL.md` (any case) at its top level.
fn dir_has_skill_md(dir: &Path) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        let name = entry.file_name();
        let Some(name_str) = name.to_str() else { continue };
        if name_str.eq_ignore_ascii_case("SKILL.md") {
            return true;
        }
    }
    false
}

/// Whether a directory looks like a standalone skill folder:
/// has SKILL.md OR contains a top-level .md/.yaml/.yml file.
fn dir_is_skill(dir: &Path, has_skill_md: bool) -> bool {
    if has_skill_md {
        return true;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_file() {
            if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                let ext_lower = ext.to_ascii_lowercase();
                if ext_lower == "md" || ext_lower == "yaml" || ext_lower == "yml" {
                    return true;
                }
            }
        }
    }
    false
}

/// Recursively walk `root` to build a [`FileTreeNode`] tree.
///
/// - `max_depth`: 0 means "only the root node itself, children=None".
///   1 = root + its direct children, etc.
/// - `max_nodes`: total nodes (including the root) beyond which we stop
///   descending further; remaining siblings on the current level still
///   get their own node but the walker stops recursing into directories.
///
/// Symlink loops are prevented via a `HashSet<PathBuf>` of canonical paths
/// already visited. Files are never followed; symlinked directories are
/// deliberately not descended into (we use `file_type()` rather than
/// `metadata()` so symlinks remain classified as symlinks → treated as
/// files in the resulting tree, which is safer).
pub fn walk_directory_tree(
    root: &Path,
    max_depth: u32,
    max_nodes: usize,
) -> Result<FileTreeNode, String> {
    let meta = fs::metadata(root).map_err(|e| e.to_string())?;
    let mut visited: HashSet<PathBuf> = HashSet::new();
    if let Ok(canon) = fs::canonicalize(root) {
        visited.insert(canon);
    }
    let mut node_count: usize = 0;
    walk_node(root, "", max_depth, max_nodes, &mut visited, &mut node_count, meta.len())
        .map(|mut n| {
            // Root is always reported as a directory regardless of file-type
            // quirks — the caller passed us a directory path.
            n.kind = FileNodeKind::Dir;
            n
        })
}

fn walk_node(
    path: &Path,
    relative: &str,
    depth_remaining: u32,
    max_nodes: usize,
    visited: &mut HashSet<PathBuf>,
    node_count: &mut usize,
    size: u64,
) -> Result<FileTreeNode, String> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    let abs = path.to_string_lossy().to_string();
    let rel = relative.to_string();

    // For root, rel is "". Children get joined as "parent/child".
    let is_dir = fs::metadata(path).map(|m| m.is_dir()).unwrap_or(false);

    if !is_dir {
        return Ok(FileTreeNode {
            name,
            relative_path: rel,
            absolute_path: abs,
            kind: FileNodeKind::File,
            size,
            has_skill_md: false,
            is_skill: false,
            children: None,
            truncated: false,
            error: None,
        });
    }

    let has_skill_md = dir_has_skill_md(path);
    let is_skill = dir_is_skill(path, has_skill_md);

    // Leaf if depth exhausted or node budget exhausted.
    let depth_hit = depth_remaining == 0;
    let budget_hit = *node_count >= max_nodes;
    if depth_hit || budget_hit {
        return Ok(FileTreeNode {
            name,
            relative_path: rel,
            absolute_path: abs,
            kind: FileNodeKind::Dir,
            size,
            has_skill_md,
            is_skill,
            children: None,
            truncated: true,
            error: None,
        });
    }

    let read = fs::read_dir(path);
    let mut children: Vec<FileTreeNode> = Vec::new();
    let entries = match read {
        Ok(e) => e,
        Err(e) => {
            return Ok(FileTreeNode {
                name,
                relative_path: rel,
                absolute_path: abs,
                kind: FileNodeKind::Dir,
                size,
                has_skill_md,
                is_skill,
                children: Some(Vec::new()),
                truncated: false,
                error: Some(e.to_string()),
            });
        }
    };
    // Collect + sort for stable UI ordering.
    let mut collected: Vec<PathBuf> = Vec::new();
    for entry in entries.flatten() {
        // Skip hidden (dotfiles) — includes .git, .DS_Store, etc.
        let fname = entry.file_name();
        let Some(fname_str) = fname.to_str() else { continue };
        if is_hidden_name(fname_str) {
            continue;
        }
        // Use file_type() so symlinks are NOT followed (prevents cycles).
        // We'll classify them as files since symlink → is_file() via metadata
        // would follow; here we deliberately drop symlink→dir from descent.
        let ft = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        if ft.is_dir() {
            collected.push(entry.path());
        } else {
            // Includes symlinks (ft.is_symlink()) — treated as files.
            collected.push(entry.path());
        }
    }
    collected.sort_by(|a, b| {
        let ad = fs::metadata(a).map(|m| m.is_dir()).unwrap_or(false);
        let bd = fs::metadata(b).map(|m| m.is_dir()).unwrap_or(false);
        match (ad, bd) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_lowercase()
                .cmp(
                    &b.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_lowercase(),
                ),
        }
    });

    for child_path in collected {
        if *node_count >= max_nodes {
            // Budget hit: leave remaining children on this level out, mark
            // node truncated so UI shows "…".
            // We've already pushed some children; mark current node truncated
            // by tracking via a flag on the *parent* — but to keep types
            // simple, we instead mark the last partial child? Cleaner: set
            // truncated=true on the parent after the loop, break here.
            // (Handled below via `truncated` flag returned from this fn.)
            break;
        }
        let child_meta = match fs::metadata(&child_path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        // Cycle protection: skip already-visited canonical dirs.
        if child_meta.is_dir() {
            if let Ok(canon) = fs::canonicalize(&child_path) {
                if !visited.insert(canon) {
                    continue;
                }
            }
        }
        *node_count += 1;
        let child_name = child_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let child_rel = if rel.is_empty() {
            child_name.clone()
        } else {
            format!("{rel}/{child_name}")
        };
        match walk_node(
            &child_path,
            &child_rel,
            depth_remaining.saturating_sub(1),
            max_nodes,
            visited,
            node_count,
            child_meta.len(),
        ) {
            Ok(n) => children.push(n),
            Err(_) => continue,
        }
    }

    let truncated = *node_count >= max_nodes;
    Ok(FileTreeNode {
        name,
        relative_path: rel,
        absolute_path: abs,
        kind: FileNodeKind::Dir,
        size,
        has_skill_md,
        is_skill,
        children: Some(children),
        truncated,
        error: None,
    })
}

// ─── Text file reader (drawer detail page) ────────────────────────────────────

const TEXT_EXTENSIONS: &[&str] = &[
    "md", "mdx", "yaml", "yml", "txt", "json", "toml", "js", "ts", "tsx", "py", "sh", "rs",
];

const DEFAULT_MAX_TEXT_BYTES: usize = 256 * 1024;

/// Read a text file (whitelisted extensions only) into a String.
/// Returns `Ok(None)` for disallowed extensions; truncates at `max_bytes`.
pub fn read_text_file(path: &Path, max_bytes: usize) -> Result<Option<String>, String> {
    let allowed = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            let lower = e.to_ascii_lowercase();
            TEXT_EXTENSIONS.iter().any(|&allowed| allowed == lower)
        })
        .unwrap_or(false);
    if !allowed {
        return Ok(None);
    }

    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => return Err(e.to_string()),
    };
    let cap = max_bytes.min(DEFAULT_MAX_TEXT_BYTES);
    let truncated = if bytes.len() > cap {
        &bytes[..cap]
    } else {
        &bytes[..]
    };
    Ok(Some(String::from_utf8_lossy(truncated).into_owned()))
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

    // ─── walk_directory_tree tests ─────────────────────────────────────────

    fn make_dir(p: &Path) {
        let _ = fs::create_dir_all(p);
    }

    fn write(p: &Path, contents: &str) {
        if let Some(parent) = p.parent() {
            make_dir(parent);
        }
        let _ = fs::write(p, contents);
    }

    fn unique_tmp(label: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "skills_pp_tree_{}_{}",
            label,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&p);
        make_dir(&p);
        p
    }

    #[test]
    fn walk_returns_error_for_missing_root() {
        let r = walk_directory_tree(Path::new("/nonexistent/zzz/123"), 4, 100);
        assert!(r.is_err());
    }

    #[test]
    fn walk_skips_hidden_and_dotfiles() {
        let tmp = unique_tmp("hidden");
        make_dir(&tmp.join("skill-a"));
        write(&tmp.join("skill-a").join("SKILL.md"), "# a");
        make_dir(&tmp.join(".git"));
        write(&tmp.join(".git").join("config"), "[gc]");
        write(&tmp.join(".DS_Store"), "x");

        let node = walk_directory_tree(&tmp, 4, 100).unwrap();
        let children = node.children.as_ref().unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].name, "skill-a");
        assert!(children[0].has_skill_md);
        assert!(children[0].is_skill);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn walk_depth_zero_yields_leaf_root() {
        let tmp = unique_tmp("depth0");
        make_dir(&tmp.join("sub"));

        let node = walk_directory_tree(&tmp, 0, 100).unwrap();
        assert!(node.children.is_none());
        assert!(node.truncated);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn walk_depth_one_lists_direct_children_only() {
        let tmp = unique_tmp("depth1");
        make_dir(&tmp.join("a"));
        make_dir(&tmp.join("a").join("deep"));
        write(&tmp.join("a").join("deep").join("x.txt"), "x");

        let node = walk_directory_tree(&tmp, 1, 100).unwrap();
        let a = node.children.as_ref().unwrap().iter().find(|c| c.name == "a").unwrap();
        // depth=1 → root + its direct children only; "a"'s children truncated.
        assert!(a.children.is_none());
        assert!(a.truncated);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn walk_node_budget_truncates() {
        let tmp = unique_tmp("budget");
        // Create 5 subdirs.
        for i in 0..5 {
            make_dir(&tmp.join(format!("d{i}")));
        }
        // max_nodes=2 → walker processes 2 children (each counts against
        // the budget when entered) before breaking; remaining 3 are dropped.
        let node = walk_directory_tree(&tmp, 4, 2).unwrap();
        assert!(node.truncated);
        assert_eq!(node.children.as_ref().unwrap().len(), 2);
        // The second child was pushed as a budget-truncated leaf.
        let second = &node.children.as_ref().unwrap()[1];
        assert!(second.truncated);
        assert!(second.children.is_none());

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn walk_nested_skill_detection() {
        let tmp = unique_tmp("nested");
        let skill_dir = tmp.join("my-skill");
        make_dir(&skill_dir);
        write(&skill_dir.join("SKILL.md"), "# my skill");
        make_dir(&skill_dir.join("scripts"));
        write(&skill_dir.join("scripts").join("a.sh"), "echo hi");

        let node = walk_directory_tree(&tmp, 4, 100).unwrap();
        let s = node.children.as_ref().unwrap().iter().find(|c| c.name == "my-skill").unwrap();
        assert!(s.has_skill_md);
        assert!(s.is_skill);
        let scripts = s.children.as_ref().unwrap().iter().find(|c| c.name == "scripts").unwrap();
        assert_eq!(scripts.kind, FileNodeKind::Dir);
        let a = scripts.children.as_ref().unwrap().iter().find(|c| c.name == "a.sh").unwrap();
        assert_eq!(a.kind, FileNodeKind::File);
        assert_eq!(a.size, 7); // "echo hi"

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn walk_skill_via_yaml_top_level() {
        let tmp = unique_tmp("yaml");
        make_dir(&tmp.join("cfg"));
        write(&tmp.join("cfg").join("skill.yaml"), "name: cfg");

        let node = walk_directory_tree(&tmp, 4, 100).unwrap();
        let c = node.children.as_ref().unwrap().iter().find(|x| x.name == "cfg").unwrap();
        assert!(!c.has_skill_md);
        assert!(c.is_skill);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn walk_relative_paths_use_forward_slash() {
        let tmp = unique_tmp("relpath");
        make_dir(&tmp.join("a"));
        write(&tmp.join("a").join("b.md"), "x");

        let node = walk_directory_tree(&tmp, 4, 100).unwrap();
        let a = node.children.as_ref().unwrap().iter().find(|c| c.name == "a").unwrap();
        let b = a.children.as_ref().unwrap().iter().find(|c| c.name == "b.md").unwrap();
        assert_eq!(b.relative_path, "a/b.md");
        // root's relative_path is "" (empty).
        assert_eq!(node.relative_path, "");

        let _ = fs::remove_dir_all(&tmp);
    }

    // ─── read_text_file tests ──────────────────────────────────────────────

    #[test]
    fn read_text_file_rejects_disallowed_extension() {
        let tmp = unique_tmp("reject");
        let p = tmp.join("binary.exe");
        write(&p, "MZ");
        let r = read_text_file(&p, 1024).unwrap();
        assert!(r.is_none());

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn read_text_file_returns_content_for_md() {
        let tmp = unique_tmp("md");
        let p = tmp.join("SKILL.md");
        write(&p, "# hi\n");
        let r = read_text_file(&p, 1024).unwrap();
        assert_eq!(r.as_deref(), Some("# hi\n"));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn read_text_file_truncates_large() {
        let tmp = unique_tmp("big");
        let p = tmp.join("big.txt");
        write(&p, &"a".repeat(5000));
        let r = read_text_file(&p, 100).unwrap();
        assert_eq!(r.unwrap().len(), 100);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn read_text_file_capped_at_default_max() {
        let tmp = unique_tmp("cap256k");
        let p = tmp.join("big.md");
        // Larger than default 256KB cap; caller passes a huge max but we cap.
        write(&p, &"b".repeat(DEFAULT_MAX_TEXT_BYTES + 1000));
        let r = read_text_file(&p, usize::MAX).unwrap();
        assert_eq!(r.unwrap().len(), DEFAULT_MAX_TEXT_BYTES);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn read_text_file_returns_err_for_missing() {
        let r = read_text_file(Path::new("/nonexistent/file.md"), 1024);
        assert!(r.is_err());
    }
}
