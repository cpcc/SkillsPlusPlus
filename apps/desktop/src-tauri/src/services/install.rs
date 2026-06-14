use crate::models::{ConflictInfo, InstallPreview, InstallStrategy, InstalledSkillRow};
use crate::services::canonical_store as cstore;
use crate::services::lockfile::{self, LockEntry};
use crate::services::skill_md;
use crate::services::symlink;
use rusqlite::{params, Connection, Result as SqliteResult};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Whether a filesystem entry name is hidden / non-skill (starts with `.`).
fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
}

// ─── Preview ──────────────────────────────────────────────────────────────────

/// Build the target path: `<directory_path>/<skill_name>`
pub fn target_path(directory_path: &str, skill_name: &str) -> PathBuf {
    let safe_name = skill_name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '-' })
        .collect::<String>();
    PathBuf::from(directory_path).join(safe_name)
}

/// 构建 install 预览。
/// - `git`/`copy`/`archive`：target = `<directory_path>/<skill_name>`。
/// - `skills_cli`：target（即 symlink 路径）= `<directory_path>/<skill_name>`，
///   canonical = `~/.agents/skills/<skill_name>/`。
pub fn build_preview(
    skill_name: &str,
    repo_url: &str,
    directory_path: &str,
    strategy: InstallStrategy,
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

    let (canonical_path, symlink_path) = if strategy == InstallStrategy::SkillsCli {
        let canonical = cstore::canonical_path(skill_name)
            .map(|p| p.to_string_lossy().to_string());
        (canonical, Some(tpath.to_string_lossy().to_string()))
    } else {
        (None, None)
    };

    InstallPreview {
        skill_name: skill_name.to_string(),
        repo_url: repo_url.to_string(),
        target_path: tpath.to_string_lossy().to_string(),
        strategy,
        canonical_path,
        symlink_path,
        conflict,
    }
}

// ─── Install ──────────────────────────────────────────────────────────────────

/// 装完后的统一结果。
pub struct InstallOutcome {
    pub log_lines: Vec<String>,
    pub content_hash: String,
    /// skills_cli 策略最终的 canonical 路径（其它策略为 None）。
    pub canonical_path: Option<PathBuf>,
    /// skills_cli 策略最终的 symlink 路径（其它策略为 None）。
    pub symlink_path: Option<PathBuf>,
}

/// 通用 dispatcher：按 strategy 分发到具体安装实现。
///
/// - `repo_url`：git clone 时的远程；copy/archive 时若 `archive_url` 为空则尝试用它。
/// - `archive_url`：copy/archive 的归档下载地址（github 的 codeload tar.gz）。
/// - `target`：对于 git/copy/archive 是落盘目录；对于 skills_cli 是 agent_link_dir（symlink 父目录）。
pub fn install_dispatch(
    strategy: InstallStrategy,
    skill_name: &str,
    repo_url: &str,
    archive_url: Option<&str>,
    target: &Path,
) -> Result<InstallOutcome, String> {
    match strategy {
        InstallStrategy::Git => {
            let lines = git_clone(repo_url, &target.join(skill_name))?;
            let hash = cstore::compute_folder_hash(&target.join(skill_name));
            Ok(InstallOutcome {
                log_lines: lines,
                content_hash: hash,
                canonical_path: None,
                symlink_path: None,
            })
        }
        InstallStrategy::Copy => {
            let url = archive_url.or(Some(repo_url)).unwrap_or("");
            install_copy(url, &target.join(skill_name))?;
            let hash = cstore::compute_folder_hash(&target.join(skill_name));
            Ok(InstallOutcome {
                log_lines: vec![format!("copy install done -> {}", target.join(skill_name).display())],
                content_hash: hash,
                canonical_path: None,
                symlink_path: None,
            })
        }
        InstallStrategy::Archive => {
            let url = archive_url.or(Some(repo_url)).unwrap_or("");
            install_archive(url, &target.join(skill_name))?;
            let hash = cstore::compute_folder_hash(&target.join(skill_name));
            Ok(InstallOutcome {
                log_lines: vec![format!("archive install done -> {}", target.join(skill_name).display())],
                content_hash: hash,
                canonical_path: None,
                symlink_path: None,
            })
        }
        InstallStrategy::SkillsCli => {
            install_skills_cli(skill_name, repo_url, archive_url, target)
        }
    }
}

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

/// HTTP GET tar.gz/zip → 解压 → strip 顶层目录 → 写入 target。
/// 不保留 `.git`。
fn install_copy(url: &str, target: &Path) -> Result<(), String> {
    download_and_extract(url, target, /* strip_git = */ true)
}

fn install_archive(url: &str, target: &Path) -> Result<(), String> {
    download_and_extract(url, target, /* strip_git = */ true)
}

/// 下载 `url`，根据扩展名（.tar.gz/.tgz/.zip）解压到 target。
/// 自动 strip 单一顶层目录（如 `repo-main/`）。
fn download_and_extract(url: &str, target: &Path, strip_git: bool) -> Result<(), String> {
    if url.is_empty() {
        return Err("no archive url available for this strategy".to_string());
    }
    let bytes = blocking_get_bytes(url)?;
    extract_archive_from_bytes(&bytes, target, strip_git)
}

/// 把已下载的归档字节解压到 target（自动 strip 顶层目录、可选去掉 `.git`）。
pub fn extract_archive_from_bytes(bytes: &[u8], target: &Path, strip_git: bool) -> Result<(), String> {
    // 先解压到临时目录，再 strip 顶层目录后整体移动到 target。
    let staging = tempfile::tempdir().map_err(|e| format!("tempdir: {e}"))?;
    if looks_like_targz(bytes) {
        extract_tar_gz(bytes, staging.path())?;
    } else if looks_like_zip(bytes) {
        extract_zip(bytes, staging.path())?;
    } else {
        // 默认按 tar.gz 尝试，失败时给出明确错误。
        extract_tar_gz(bytes, staging.path())
            .map_err(|e| format!("archive format unrecognized; tar.gz parse failed: {e}"))?;
    }

    // 找到真正的 skill 根：若 staging 只有一个目录 → 用它作为内容根；否则用 staging 本身。
    let content_root = single_child_dir(staging.path()).unwrap_or_else(|| staging.path().to_path_buf());

    fs::create_dir_all(target).map_err(|e| format!("mkdir target: {e}"))?;
    move_contents(&content_root, target, strip_git)?;

    // 强制校验：装完必须能找到 SKILL.md（大小写不敏感）。
    if !has_skill_md(target) {
        // 清掉半成品。
        let _ = fs::remove_dir_all(target);
        return Err("installed directory has no SKILL.md".to_string());
    }
    Ok(())
}

fn blocking_get_bytes(url: &str) -> Result<Vec<u8>, String> {
    // 同步阻塞执行 reqwest，避免在非 async 上下文中调用。
    let url = url.to_string();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                let _ = tx.send(Err(format!("build tokio runtime: {e}")));
                return;
            }
        };
        let result: Result<Vec<u8>, String> = rt.block_on(async move {
            let client = reqwest::Client::builder()
                .user_agent("skills-plus-plus/0.1")
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .map_err(|e| e.to_string())?;
            let resp = client
                .get(&url)
                .send()
                .await
                .map_err(|e| format!("http get: {e}"))?;
            if !resp.status().is_success() {
                return Err(format!("http status {}", resp.status()));
            }
            let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
            Ok(bytes.to_vec())
        });
        let _ = tx.send(result);
    });
    rx.recv().map_err(|e| format!("http thread join: {e}"))?
}

fn extract_tar_gz(bytes: &[u8], dest: &Path) -> Result<(), String> {
    let decoder = flate2::read::GzDecoder::new(bytes);
    let mut archive = tar::Archive::new(decoder);
    archive.set_overwrite(true);
    archive.unpack(dest).map_err(|e| format!("tar.gz unpack: {e}"))
}

fn extract_zip(bytes: &[u8], dest: &Path) -> Result<(), String> {
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| format!("zip open: {e}"))?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| format!("zip entry {i}: {e}"))?;
        let outpath = match file.enclosed_name() {
            Some(p) => dest.join(p),
            None => continue,
        };
        if file.is_dir() {
            fs::create_dir_all(&outpath).map_err(|e| format!("mkdir {outpath:?}: {e}"))?;
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent).map_err(|e| format!("mkdir parent: {e}"))?;
            }
            let mut outfile = fs::File::create(&outpath).map_err(|e| format!("create {outpath:?}: {e}"))?;
            std::io::copy(&mut file, &mut outfile).map_err(|e| format!("copy {outpath:?}: {e}"))?;
        }
        // 权限（unix）。
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = file.unix_mode() {
                let _ = fs::set_permissions(&outpath, fs::Permissions::from_mode(mode));
            }
        }
    }
    Ok(())
}

fn single_child_dir(dir: &Path) -> Option<PathBuf> {
    let entries: Vec<_> = fs::read_dir(dir).ok()?.flatten().collect();
    if entries.len() == 1 && entries[0].path().is_dir() {
        Some(entries[0].path())
    } else {
        None
    }
}

/// 把 `src` 的内容（去掉 `.git`）整体移到 `dst`。
fn move_contents(src: &Path, dst: &Path, strip_git: bool) -> Result<(), String> {
    for entry in fs::read_dir(src).map_err(|e| format!("readdir: {e}"))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name();
        if strip_git && name == ".git" {
            continue;
        }
        let from = entry.path();
        let to = dst.join(&name);
        // 跨目录时 rename 可能失败，回退到 copy + remove。
        if fs::rename(&from, &to).is_err() {
            symlink::copy_recursive(&from, &to)?;
            let _ = fs::remove_dir_all(&from);
        }
    }
    Ok(())
}

fn has_skill_md(dir: &Path) -> bool {
    for name in ["SKILL.md", "skill.md", "Skill.md"] {
        if dir.join(name).exists() {
            return true;
        }
    }
    false
}

fn looks_like_targz(bytes: &[u8]) -> bool {
    // gzip magic: 1f 8b
    bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b
}

fn looks_like_zip(bytes: &[u8]) -> bool {
    // zip magic: 50 4b 03 04
    bytes.len() >= 4 && bytes[0] == 0x50 && bytes[1] == 0x4b && bytes[2] == 0x03 && bytes[3] == 0x04
}

/// BFS 搜索目录树，找到第一个包含 SKILL.md 的目录，返回该目录路径。
/// 优先匹配名称与 `skill_name` 最接近的目录（按字符相似度）。
fn find_skill_folder_with_name(base_dir: &Path, skill_name: &str) -> Option<PathBuf> {
    let mut candidates: Vec<(PathBuf, usize)> = Vec::new();
    let mut queue: Vec<PathBuf> = vec![base_dir.to_path_buf()];

    while let Some(dir) = queue.pop() {
        if has_skill_md(&dir) {
            // 计算目录名与 skill_name 的公共前缀长度作为相似度
            let dir_name = dir.file_name().and_then(|s| s.to_str()).unwrap_or("");
            let score = common_prefix_len(&dir_name.to_lowercase(), &skill_name.to_lowercase());
            candidates.push((dir.clone(), score));
        }
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let hidden = path.file_name()
                    .and_then(|s| s.to_str())
                    .map(|n| n.starts_with('.'))
                    .unwrap_or(false);
                if path.is_dir() && !hidden {
                    queue.push(path);
                }
            }
        }
    }

    // 选最高分；平局时取路径最短（最顶层）
    candidates.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.to_string_lossy().len().cmp(&b.0.to_string_lossy().len())));
    candidates.into_iter().next().map(|(p, _)| p)
}

fn common_prefix_len(a: &str, b: &str) -> usize {
    a.chars().zip(b.chars()).take_while(|(ca, cb)| ca == cb).count()
}

/// 递归复制 src 下所有内容到 dst（类似 `cp -r src/* dst/`）。
fn copy_dir_contents(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("mkdir dst: {e}"))?;
    let entries = fs::read_dir(src).map_err(|e| format!("read_dir {src:?}: {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("dir entry: {e}"))?;
        let src_path = entry.path();
        let file_name = entry.file_name();
        let dst_path = dst.join(&file_name);
        if src_path.is_dir() {
            copy_dir_contents(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| format!("copy {src_path:?} -> {dst_path:?}: {e}"))?;
        }
    }
    Ok(())
}

/// skills_cli 安装策略（对齐 vercel-labs `npx skills`）：
/// 1. clone 仓库到临时目录 / download archive 到临时目录
/// 2. 在临时目录中搜索 SKILL.md，找到实际 skill 目录
/// 3. 复制 skill 目录内容到 canonical = `~/.agents/skills/<name>/`
/// 4. 解析 SKILL.md，必要时 rename canonical 到规范名
/// 5. 在 `agent_link_dir/<name>` 创建 symlink → canonical
/// 6. 写 lockfile 条目
/// 7. 返回 outcome（canonical_path / symlink_path / content_hash）
///
/// `agent_link_dir` 通常为 `~/.claude/skills/` 这类 AI 工具目录。
fn install_skills_cli(
    skill_name: &str,
    repo_url: &str,
    archive_url: Option<&str>,
    agent_link_dir: &Path,
) -> Result<InstallOutcome, String> {
    let canonical_root = cstore::canonical_root()
        .ok_or_else(|| "cannot resolve home dir for canonical store".to_string())?;
    fs::create_dir_all(&canonical_root).map_err(|e| format!("mkdir canonical root: {e}"))?;

    let mut canonical = canonical_root.join(skill_md::sanitize_name(skill_name));

    // 1) 拉内容到临时目录，然后找到 SKILL.md 所在目录复制到 canonical。
    if !canonical.exists() || fs::read_dir(&canonical).map(|mut d| d.next().is_none()).unwrap_or(true) {
        let temp_dir = tempfile::tempdir().map_err(|e| format!("create temp dir: {e}"))?;
        let clone_target = temp_dir.path().join("repo");

        // 下载 / clone 到临时目录。
        if let Some(url) = archive_url.filter(|u| !u.is_empty()) {
            download_and_extract(url, &clone_target, /* strip_git = */ true)?;
        } else {
            git_clone(repo_url, &clone_target)?;
            if clone_target.exists() {
                let _ = fs::remove_dir_all(clone_target.join(".git"));
            }
        }

        // 在 clone 中搜索 SKILL.md。
        let skill_folder = find_skill_folder_with_name(&clone_target, skill_name)
            .ok_or_else(|| "no SKILL.md found in repository".to_string())?;

        // 复制 skill 目录内容到 canonical。
        fs::create_dir_all(&canonical).ok();
        copy_dir_contents(&skill_folder, &canonical)?;
    }

    if !has_skill_md(&canonical) {
        let _ = fs::remove_dir_all(&canonical);
        return Err("installed directory has no SKILL.md".to_string());
    }

    // 2) 规范化目录名。
    let name_key;
    if let Some(manifest) = skill_md::parse_skill_md(&canonical) {
        let normalized = skill_md::normalize_skill_dir(&canonical, &manifest);
        canonical = normalized;
        name_key = skill_md::sanitize_name(&manifest.name);
    } else {
        name_key = skill_md::sanitize_name(skill_name);
    }

    // 3) symlink：agent_link_dir/<name> -> canonical
    let link = agent_link_dir.join(&name_key);
    fs::create_dir_all(agent_link_dir).map_err(|e| format!("mkdir agent_link_dir: {e}"))?;
    symlink::create_symlink(&canonical, &link)?;

    // 4) 写 lockfile。
    let hash = cstore::compute_folder_hash(&canonical);
    let now = now_iso8601();
    let source_label = derive_source_label(repo_url);
    let source_type = if archive_url.filter(|u| !u.is_empty()).is_some() { "archive" } else { "github" };
    let entry = LockEntry {
        source: source_label,
        source_type: source_type.to_string(),
        source_url: repo_url.to_string(),
        skill_path: "SKILL.md".to_string(),
        skill_folder_hash: hash.clone(),
        installed_at: now.clone(),
        updated_at: now,
    };
    if let Err(e) = lockfile::upsert_entry(&name_key, entry) {
        log::warn!("lockfile upsert failed for {name_key}: {e}");
    }

    Ok(InstallOutcome {
        log_lines: vec![format!(
            "skills_cli install: {} -> {}",
            link.display(),
            canonical.display()
        )],
        content_hash: hash,
        canonical_path: Some(canonical.clone()),
        symlink_path: Some(link),
    })
}

fn derive_source_label(repo_url: &str) -> String {
    // https://github.com/owner/repo(.git) -> owner/repo
    let trimmed = repo_url
        .trim_end_matches(".git")
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    if let Some(rest) = trimmed.strip_prefix("github.com/") {
        return rest.to_string();
    }
    trimmed.to_string()
}

fn now_iso8601() -> String {
    // 简易 ISO8601（UTC），不引 chrono。
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (y, mo, d, h, mi, s) = epoch_to_ymdhms(secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}.000Z")
}

fn epoch_to_ymdhms(secs: u64) -> (u32, u32, u32, u32, u32, u32) {
    // 足够用的民用日历算法（1970-01-01 起算）。
    let s = (secs % 60) as u32;
    let m = ((secs / 60) % 60) as u32;
    let h = ((secs / 3600) % 24) as u32;
    let mut days = secs / 86400;
    let mut year = 1970u32;
    loop {
        let leap = is_leap(year);
        let yd = if leap { 366 } else { 365 };
        if days < yd { break; }
        days -= yd;
        year += 1;
    }
    let leap = is_leap(year);
    let mdays = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1u32;
    let mut remaining = days as u32;
    for &dm in mdays.iter() {
        if remaining < dm { break; }
        remaining -= dm;
        month += 1;
    }
    let day = remaining + 1;
    (year, month, day, h, m, s)
}

fn is_leap(y: u32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
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
    strategy: InstallStrategy,
    content_hash: Option<&str>,
    canonical_path: Option<&str>,
) -> SqliteResult<()> {
    conn.execute(
        "INSERT OR REPLACE INTO installed_skills \
         (id, skill_id, name, tool_name, directory_id, source_id, repo_url, installed_at, status, \
          install_strategy, content_hash, canonical_path) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now'), 'ok', ?8, ?9, ?10)",
        params![
            id, skill_id, skill_name, tool_name, directory_id, source_id, repo_url,
            strategy.as_str(), content_hash, canonical_path,
        ],
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
                i.source_id, i.repo_url, i.installed_at, i.status, \
                i.install_strategy, i.content_hash, i.canonical_path, \
                i.author, i.description \
         FROM installed_skills i \
         LEFT JOIN ai_tool_directories d ON i.directory_id = d.id \
         ORDER BY i.installed_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        let strategy_s: String = row.get(10)?;
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
            install_strategy: crate::models::InstallStrategy::parse(&strategy_s),
            content_hash: row.get(11)?,
            canonical_path: row.get(12)?,
            author: row.get(13)?,
            description: row.get(14)?,
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

// ─── Import existing local skills ─────────────────────────────────────────────

/// Best-effort: read `<skill_dir>/.git/config` and return the `[remote "origin"]` url.
fn extract_git_origin(skill_dir: &Path) -> Option<String> {
    let config_path = skill_dir.join(".git").join("config");
    let content = std::fs::read_to_string(&config_path).ok()?;
    let mut in_origin = false;
    for raw in content.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            in_origin = line == "[remote \"origin\"]";
            continue;
        }
        if in_origin {
            // match: url = <value>
            if let Some(rest) = line.strip_prefix("url") {
                let rest = rest.trim_start();
                let rest = rest.strip_prefix('=').unwrap_or(rest).trim();
                if !rest.is_empty() {
                    return Some(rest.to_string());
                }
            }
        }
    }
    None
}

/// Locate the SKILL.md inside a skill directory (case-insensitive on the
/// filename). Returns `None` if not found.
fn find_skill_md(skill_dir: &Path) -> Option<PathBuf> {
    for entry in std::fs::read_dir(skill_dir).ok()?.flatten() {
        if let Some(name) = entry.file_name().to_str() {
            if name.eq_ignore_ascii_case("SKILL.md") {
                return Some(entry.path());
            }
        }
    }
    None
}

/// Parse a YAML-frontmatter block at the top of `SKILL.md` (or the entry
/// file itself when `skill_path` is a `.md` file). Returns `(author, description)`.
///
/// Recognises a leading `---` block; for each `key: value` line inside it,
/// extracts `description` (may be quoted / multi-line via `>-` not supported —
/// single-line only) and `author`. Falls back to `description` from the first
/// non-heading paragraph after the frontmatter.
fn parse_skill_meta(skill_path: &Path) -> (Option<String>, Option<String>) {
    let content = match std::fs::read_to_string(skill_path) {
        Ok(c) => c,
        Err(_) => return (None, None),
    };
    let mut lines = content.lines();

    let mut in_fm = false;
    let mut started_fm = false;
    let mut author: Option<String> = None;
    let mut description: Option<String> = None;
    let mut body_first_line: Option<String> = None;

    for line in lines.by_ref() {
        let trimmed = line.trim();
        if !started_fm && trimmed == "---" {
            in_fm = true;
            started_fm = true;
            continue;
        }
        if in_fm {
            if trimmed == "---" || trimmed == "..." {
                in_fm = false;
                continue;
            }
            // key: value
            if let Some((k, v)) = split_yaml_kv(line) {
                let key = k.to_ascii_lowercase();
                match key.as_str() {
                    "author" => {
                        if author.is_none() {
                            author = Some(unquote_yaml(v));
                        }
                    }
                    "description" => {
                        if description.is_none() {
                            description = Some(unquote_yaml(v));
                        }
                    }
                    _ => {}
                }
            }
            continue;
        }
        // Body: capture first non-empty, non-heading line as a description fallback.
        if body_first_line.is_none() && !trimmed.is_empty() {
            if trimmed.starts_with('#') {
                continue;
            }
            body_first_line = Some(trimmed.to_string());
        }
    }

    let description = description.or(body_first_line);
    (author, description)
}

fn split_yaml_kv(line: &str) -> Option<(&str, &str)> {
    let line = line.trim_start_matches(|c: char| c == '\t' || c == ' ');
    let idx = line.find(':')?;
    let key = &line[..idx];
    let mut value = &line[idx + 1..];
    value = value.trim();
    // Skip YAML list / nested keys.
    if value.is_empty() {
        return None;
    }
    Some((key, value))
}

fn unquote_yaml(v: &str) -> String {
    let v = v.trim();
    if v.len() >= 2 {
        let first = v.chars().next().unwrap();
        let last = v.chars().last().unwrap();
        if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
            return v[1..v.len() - 1].to_string();
        }
    }
    v.to_string()
}

/// Format a `SystemTime` as an RFC 3339 string (UTC). Returns None on error.
fn system_time_to_rfc3339(time: SystemTime) -> Option<String> {
    let dur = time.duration_since(UNIX_EPOCH).ok()?;
    let secs = dur.as_secs();
    // Use SQLite datetime via seconds → rfc3339 manually.
    let days = (secs / 86400) as i64;
    let rem = (secs % 86400) as i64;
    let h = rem / 3600;
    let m = (rem % 3600) / 60;
    let s = rem % 60;
    // Days since 1970-01-01 → civil date (Howard Hinnant's algorithm).
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };
    Some(format!(
        "{year:04}-{month:02}-{d:02}T{h:02}:{m:02}:{s:02}Z"
    ))
}

/// Scan all enabled, detected tool directories and register any pre-existing
/// skill folders / files into `installed_skills`. Idempotent: existing records
/// (matched by `(name, directory_id)`) are left untouched.
///
/// Returns the number of newly inserted rows.
pub fn import_existing_skills(conn: &Connection) -> SqliteResult<usize> {
    // Load enabled+detected directories.
    let dirs: Vec<(String, String, String)> = {
        let mut stmt = conn.prepare(
            "SELECT id, tool_name, path FROM ai_tool_directories \
             WHERE enabled = 1 AND is_detected = 1",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    let mut inserted = 0usize;

    for (directory_id, tool_name, dir_path) in dirs {
        let path = PathBuf::from(&dir_path);
        let entries = match std::fs::read_dir(&path) {
            Ok(e) => e,
            Err(_) => continue, // Directory missing / unreadable — skip.
        };

        // Collect candidate (name, is_dir) pairs in this directory.
        let mut candidates: Vec<(String, bool)> = Vec::new();
        for entry in entries.flatten() {
            let file_name = match entry.file_name().to_str() {
                Some(n) => n.to_string(),
                None => continue,
            };
            if is_hidden(&file_name) {
                continue;
            }
            let ft = match entry.file_type() {
                Ok(t) => t,
                Err(_) => continue,
            };
            if ft.is_dir() {
                // Skip empty directories.
                let is_empty = std::fs::read_dir(entry.path())
                    .map(|mut d| d.next().is_none())
                    .unwrap_or(true);
                if is_empty {
                    continue;
                }
                candidates.push((file_name, true));
            } else if ft.is_file() {
                let ext_ok = Path::new(&file_name)
                    .extension()
                    .map(|x| x == "md" || x == "yaml" || x == "yml")
                    .unwrap_or(false);
                if ext_ok {
                    candidates.push((file_name, false));
                }
            }
        }

        for (skill_name, is_dir) in candidates {
            // Already present? (idempotent check — do not overwrite user-installed rows)
            let exists: bool = conn
                .query_row(
                    "SELECT 1 FROM installed_skills WHERE name = ?1 AND directory_id = ?2",
                    params![skill_name, directory_id],
                    |_| Ok(()),
                )
                .is_ok();
            if exists {
                continue;
            }

            let skill_path = path.join(&skill_name);
            let repo_url = if is_dir {
                extract_git_origin(&skill_path)
            } else {
                None
            };

            // Parse author/description from SKILL.md frontmatter (or the file
            // itself when the entry is a standalone `.md`).
            let (author, description) = if is_dir {
                find_skill_md(&skill_path)
                    .map(|p| parse_skill_meta(&p))
                    .unwrap_or((None, None))
            } else {
                parse_skill_meta(&skill_path)
            };

            let installed_at = {
                let mtime_target = if is_dir {
                    skill_path.join(".git")
                } else {
                    skill_path.clone()
                };
                std::fs::metadata(&mtime_target)
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(system_time_to_rfc3339)
            };

            let id = uuid::Uuid::new_v4().to_string();
            let inst_at = installed_at.as_deref();

            conn.execute(
                "INSERT INTO installed_skills \
                 (id, skill_id, name, tool_name, directory_id, source_id, repo_url, installed_at, status, author, description) \
                 VALUES (?1, NULL, ?2, ?3, ?4, NULL, ?5, \
                         COALESCE(?6, datetime('now')), 'ok', ?7, ?8)",
                params![id, skill_name, tool_name, directory_id, repo_url, inst_at, author, description],
            )?;
            inserted += 1;
        }
    }

    Ok(inserted)
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

/// 对非 git 安装的 skill：重新下载到 tmp，算 hash，与 stored_hash 比。
fn hash_differs_for(url: &str, stored_hash: &str) -> Result<bool, String> {
    let tmp = tempfile::tempdir().map_err(|e| format!("tempdir: {e}"))?;
    download_and_extract(url, tmp.path(), /* strip_git = */ true)?;
    let new_hash = cstore::compute_folder_hash(tmp.path());
    Ok(new_hash != stored_hash)
}

/// Update a single installed skill's status in DB and return the updated row.
///
/// 策略分支：
/// - git：保持原有 `check_update_available`（git fetch + rev-list）。
/// - copy / archive：用 content_hash 与远端重新下载比对。
/// - skills_cli：读 lockfile 里的 `skillFolderHash`，重算 canonical 目录 hash；
///   两者不一致 → `changed`；一致但 lockfile 记录与远端比对可省略（暂不下载）。
pub fn refresh_single_skill_status(
    conn: &Connection,
    skill_id: &str,
) -> SqliteResult<Option<InstalledSkillRow>> {
    // Get skill info
    let skill = conn.query_row(
        "SELECT i.id, i.skill_id, i.name, i.tool_name, i.directory_id, \
                COALESCE(d.path, ''), i.source_id, i.repo_url, i.installed_at, i.status, \
                i.install_strategy, i.content_hash, i.canonical_path, \
                i.author, i.description \
         FROM installed_skills i \
         LEFT JOIN ai_tool_directories d ON i.directory_id = d.id \
         WHERE i.id = ?1",
        params![skill_id],
        |row| {
            let strategy_s: String = row.get(10)?;
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
                install_strategy: crate::models::InstallStrategy::parse(&strategy_s),
                content_hash: row.get(11)?,
                canonical_path: row.get(12)?,
                author: row.get(13)?,
                description: row.get(14)?,
            })
        },
    );

    match skill {
        Ok(mut s) => {
            let mut new_status = compute_skill_status(&s.name, &s.directory_path).to_string();

            if new_status == "ok" {
                let target = target_path(&s.directory_path, &s.name);
                match s.install_strategy {
                    InstallStrategy::Git => {
                        if let Some(ref repo_url) = s.repo_url {
                            if !repo_url.is_empty() && target.join(".git").exists() {
                                if let Ok(has_update) = check_update_available(&target) {
                                    if has_update {
                                        new_status = "update_available".to_string();
                                    }
                                }
                            }
                        }
                    }
                    InstallStrategy::Copy | InstallStrategy::Archive => {
                        // 比对 content_hash 与重新下载的 hash。
                        if let Some(stored) = s.content_hash.as_deref() {
                            // 用 repo_url 兜底（archive_url 没存进 installed_skills）。
                            let url = s.repo_url.clone().unwrap_or_default();
                            if !url.is_empty() {
                                match hash_differs_for(&url, stored) {
                                    Ok(true) => new_status = "update_available".to_string(),
                                    Ok(false) => {}
                                    Err(e) => log::warn!("hash diff failed for {}: {e}", s.name),
                                }
                            }
                        }
                    }
                    InstallStrategy::SkillsCli => {
                        // canonical 目录 hash 与 lockfile 记录比对。
                        let canonical = s
                            .canonical_path
                            .clone()
                            .and_then(|p| if p.is_empty() { None } else { Some(PathBuf::from(p)) })
                            .or_else(|| cstore::canonical_path(&s.name));
                        if let Some(canon) = canonical {
                            let actual = cstore::compute_folder_hash(&canon);
                            let lf = lockfile::read_lockfile();
                            let stored = lf
                                .get(&crate::services::skill_md::sanitize_name(&s.name))
                                .map(|e| e.skill_folder_hash.clone());
                            if let Some(stored_hash) = stored {
                                if stored_hash != actual {
                                    new_status = "changed".to_string();
                                }
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
        let preview = build_preview(
            "test-skill",
            "https://github.com/x/y",
            "/nonexistent/path",
            InstallStrategy::Git,
        );
        assert_eq!(preview.skill_name, "test-skill");
        assert_eq!(preview.repo_url, "https://github.com/x/y");
        assert!(preview.conflict.is_none());
        assert!(preview.canonical_path.is_none());
        assert!(preview.symlink_path.is_none());
    }

    #[test]
    fn build_preview_skills_cli_emits_canonical_and_symlink() {
        let preview = build_preview(
            "cli-skill",
            "https://github.com/x/y",
            "/tmp/agent-link-dir",
            InstallStrategy::SkillsCli,
        );
        assert!(preview.canonical_path.as_deref().unwrap().ends_with(".agents/skills/cli-skill"));
        assert!(preview.symlink_path.is_some());
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
            InstallStrategy::Git,
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

    #[test]
    fn extract_archive_strips_top_level_and_requires_skill_md() {
        // 构造一个 tar.gz：顶层目录 repo-main/，内含 SKILL.md。
        let staging = std::env::temp_dir().join("skills_pp_extract_src");
        let out = std::env::temp_dir().join("skills_pp_extract_dst");
        let _ = fs::remove_dir_all(&staging);
        let _ = fs::remove_dir_all(&out);
        fs::create_dir_all(staging.join("repo-main")).unwrap();
        fs::write(staging.join("repo-main/SKILL.md"), "---\nname: demo\n---\nbody").unwrap();
        fs::write(staging.join("repo-main/lib.txt"), "hello").unwrap();

        // 打包：tar.gz
        let mut buf: Vec<u8> = vec![];
        {
            let encoder = flate2::write::GzEncoder::new(&mut buf, flate2::Compression::default());
            let mut tar = tar::Builder::new(encoder);
            tar.append_dir_all("repo-main", staging.join("repo-main")).unwrap();
            tar.finish().unwrap();
        }
        extract_archive_from_bytes(&buf, &out, /* strip_git = */ true).expect("extract ok");
        assert!(out.join("SKILL.md").exists());
        assert!(out.join("lib.txt").exists());
        // 顶层目录应被 strip
        assert!(!out.join("repo-main").exists());

        let _ = fs::remove_dir_all(&staging);
        let _ = fs::remove_dir_all(&out);
    }

    #[test]
    fn extract_archive_rejects_when_no_skill_md() {
        let staging = std::env::temp_dir().join("skills_pp_extract_nomd_src");
        let out = std::env::temp_dir().join("skills_pp_extract_nomd_dst");
        let _ = fs::remove_dir_all(&staging);
        let _ = fs::remove_dir_all(&out);
        fs::create_dir_all(staging.join("repo-main")).unwrap();
        fs::write(staging.join("repo-main/README.md"), "no skill md here").unwrap();

        let mut buf: Vec<u8> = vec![];
        {
            let encoder = flate2::write::GzEncoder::new(&mut buf, flate2::Compression::default());
            let mut tar = tar::Builder::new(encoder);
            tar.append_dir_all("repo-main", staging.join("repo-main")).unwrap();
            tar.finish().unwrap();
        }
        let err = extract_archive_from_bytes(&buf, &out, true).unwrap_err();
        assert!(err.contains("SKILL.md"), "unexpected err: {err}");
        assert!(!out.exists(), "half-baked dir should be removed");

        let _ = fs::remove_dir_all(&staging);
    }

    #[test]
    fn derive_source_label_strips_github_prefix() {
        assert_eq!(derive_source_label("https://github.com/vercel-labs/skills.git"), "vercel-labs/skills");
        assert_eq!(derive_source_label("https://github.com/a/b"), "a/b");
    }

    #[test]
    fn epoch_to_ymdhms_known_value() {
        // 2026-06-13T12:00:00 UTC ≈ 1781352000
        let (y, mo, d, h, mi, s) = epoch_to_ymdhms(1_781_352_000);
        assert_eq!(y, 2026);
        assert_eq!(mo, 6);
        assert_eq!(d, 13);
        assert_eq!((h, mi, s), (12, 0, 0));
    }

    // ─── import_existing_skills tests ────────────────────────────────────────

    fn open_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE ai_tool_directories (
                id TEXT PRIMARY KEY, tool_name TEXT NOT NULL, path TEXT NOT NULL,
                is_default INTEGER NOT NULL DEFAULT 0, is_detected INTEGER NOT NULL DEFAULT 0,
                writable INTEGER NOT NULL DEFAULT 0, enabled INTEGER NOT NULL DEFAULT 1,
                skill_count INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE installed_skills (
                id TEXT PRIMARY KEY, skill_id TEXT, name TEXT NOT NULL, tool_name TEXT NOT NULL,
                directory_id TEXT NOT NULL, source_id TEXT, repo_url TEXT,
                installed_at TEXT NOT NULL DEFAULT (datetime('now')), status TEXT NOT NULL DEFAULT 'ok',
                author TEXT, description TEXT
            );",
        )
        .unwrap();
        conn
    }

    #[test]
    fn is_hidden_detects_dotfiles() {
        assert!(is_hidden(".git"));
        assert!(is_hidden(".DS_Store"));
        assert!(!is_hidden("my-skill"));
        assert!(!is_hidden("README.md"));
    }

    #[test]
    fn import_existing_skills_imports_subdir() {
        let tmp = std::env::temp_dir().join("skills_pp_test_import_subdir");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        // One subdir with content (skill), one empty subdir (skipped), one .md file (skill).
        let skill_a = tmp.join("skill-a");
        fs::create_dir_all(&skill_a).unwrap();
        fs::write(skill_a.join("SKILL.md"), "x").unwrap();
        fs::create_dir_all(tmp.join("empty")).unwrap();
        fs::write(tmp.join("standalone.md"), "y").unwrap();
        // Non-skill file extension ignored.
        fs::write(tmp.join("notes.txt"), "z").unwrap();
        // Hidden file ignored.
        fs::write(tmp.join(".DS_Store"), "").unwrap();

        let conn = open_test_db();
        conn.execute(
            "INSERT INTO ai_tool_directories (id, tool_name, path, is_detected, enabled) \
             VALUES ('d1', 'Claude', ?1, 1, 1)",
            params![tmp.to_string_lossy()],
        )
        .unwrap();

        let n = import_existing_skills(&conn).unwrap();
        assert_eq!(n, 2); // skill-a + standalone.md

        // Idempotent: re-running inserts nothing.
        let n2 = import_existing_skills(&conn).unwrap();
        assert_eq!(n2, 0);

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM installed_skills WHERE directory_id = 'd1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn import_existing_skills_preserves_user_records() {
        let tmp = std::env::temp_dir().join("skills_pp_test_import_idem");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let skill_dir = tmp.join("real-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "x").unwrap();

        let conn = open_test_db();
        conn.execute(
            "INSERT INTO ai_tool_directories (id, tool_name, path, is_detected, enabled) \
             VALUES ('d1', 'Claude', ?1, 1, 1)",
            params![tmp.to_string_lossy()],
        )
        .unwrap();

        // Pre-existing user record (with repo_url, source_id — should NOT be overwritten).
        conn.execute(
            "INSERT INTO installed_skills \
             (id, skill_id, name, tool_name, directory_id, source_id, repo_url, status) \
             VALUES ('u1', 'sk1', 'real-skill', 'Claude', 'd1', 'skills_sh', 'https://github.com/x/y', 'ok')",
            [],
        )
        .unwrap();

        let n = import_existing_skills(&conn).unwrap();
        assert_eq!(n, 0); // existing row skipped

        // Verify the user's repo_url was preserved.
        let repo: Option<String> = conn
            .query_row(
                "SELECT repo_url FROM installed_skills WHERE id = 'u1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(repo.as_deref(), Some("https://github.com/x/y"));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn extract_git_origin_parses_remote() {
        let tmp = std::env::temp_dir().join("skills_pp_test_git_origin");
        let _ = fs::remove_dir_all(&tmp);
        let git_dir = tmp.join(".git");
        fs::create_dir_all(&git_dir).unwrap();
        fs::write(
            git_dir.join("config"),
            "[core]\n\trepositoryformatversion = 0\n\
             [remote \"origin\"]\n\turl = https://github.com/foo/bar.git\n\tfetch = +refs/heads/*:refs/remotes/origin/*\n\
             [remote \"upstream\"]\n\turl = https://github.com/up/u.git\n",
        )
        .unwrap();

        assert_eq!(
            extract_git_origin(&tmp).as_deref(),
            Some("https://github.com/foo/bar.git")
        );

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn extract_git_origin_returns_none_without_git() {
        let tmp = std::env::temp_dir().join("skills_pp_test_git_origin_none");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        assert!(extract_git_origin(&tmp).is_none());
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn import_existing_skills_skips_disabled_directories() {
        let tmp = std::env::temp_dir().join("skills_pp_test_import_disabled");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::create_dir_all(tmp.join("skill-x")).unwrap();
        fs::write(tmp.join("skill-x").join("a.md"), "y").unwrap();

        let conn = open_test_db();
        // enabled = 0 → must be skipped
        conn.execute(
            "INSERT INTO ai_tool_directories (id, tool_name, path, is_detected, enabled) \
             VALUES ('d1', 'Claude', ?1, 1, 0)",
            params![tmp.to_string_lossy()],
        )
        .unwrap();

        let n = import_existing_skills(&conn).unwrap();
        assert_eq!(n, 0);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn parse_skill_meta_reads_frontmatter() {
        let tmp = std::env::temp_dir().join("skills_pp_test_meta_fm");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let md = tmp.join("SKILL.md");
        fs::write(
            &md,
            "---\nname: playwright-cli\ndescription: \"Automate browser stuff.\"\nauthor: jane\n---\n\n# Heading\n\nBody text.\n",
        )
        .unwrap();
        let (author, desc) = parse_skill_meta(&md);
        assert_eq!(author.as_deref(), Some("jane"));
        assert_eq!(desc.as_deref(), Some("Automate browser stuff."));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn parse_skill_meta_falls_back_to_first_paragraph() {
        let tmp = std::env::temp_dir().join("skills_pp_test_meta_fb");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let md = tmp.join("SKILL.md");
        fs::write(&md, "# Title\n\nThis is the first paragraph.\n").unwrap();
        let (author, desc) = parse_skill_meta(&md);
        assert!(author.is_none());
        assert_eq!(desc.as_deref(), Some("This is the first paragraph."));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn import_existing_skills_captures_meta() {
        let tmp = std::env::temp_dir().join("skills_pp_test_import_meta");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let skill_dir = tmp.join("meta-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nauthor: alice\ndescription: Does cool stuff.\n---\n\n# Body\n",
        )
        .unwrap();

        let conn = open_test_db();
        conn.execute(
            "INSERT INTO ai_tool_directories (id, tool_name, path, is_detected, enabled) \
             VALUES ('d1', 'Claude', ?1, 1, 1)",
            params![tmp.to_string_lossy()],
        )
        .unwrap();

        import_existing_skills(&conn).unwrap();
        let (author, desc): (Option<String>, Option<String>) = conn
            .query_row(
                "SELECT author, description FROM installed_skills WHERE directory_id = 'd1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(author.as_deref(), Some("alice"));
        assert_eq!(desc.as_deref(), Some("Does cool stuff."));

        let _ = fs::remove_dir_all(&tmp);
    }
}
