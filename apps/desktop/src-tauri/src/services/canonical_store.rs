//! Canonical store：`~/.agents/skills/<name>/`，与 vercel-labs `npx skills` 互通。

use sha1::{Digest, Sha1};
use std::fs;
use std::path::{Path, PathBuf};

/// `~/.agents/skills/` 绝对路径。
pub fn canonical_root() -> Option<PathBuf> {
    Some(dirs::home_dir()?.join(".agents").join("skills"))
}

/// `~/.agents/skills/<name>/`。
pub fn canonical_path(name: &str) -> Option<PathBuf> {
    Some(canonical_root()?.join(crate::services::skill_md::sanitize_name(name)))
}

/// `~/.agents/.skill-lock.json`。
pub fn lockfile_path() -> Option<PathBuf> {
    Some(dirs::home_dir()?.join(".agents").join(".skill-lock.json"))
}

/// 递归遍历目录，按相对路径 + 文件内容的 SHA1 计算稳定的目录指纹。
/// 空目录或不存在目录返回固定值（与 vercel-labs 一致：无内容 → 已知常量）。
/// 跳过 `.git` 目录，避免 git 元数据干扰。
pub fn compute_folder_hash(path: &Path) -> String {
    if !path.exists() {
        return "0000000000000000000000000000000000000000".to_string();
    }
    let mut hasher = Sha1::new();
    let mut entries: Vec<PathBuf> = walk(path, path);
    entries.sort();
    for rel in entries {
        let rel_str = rel.to_string_lossy();
        hasher.update(rel_str.as_bytes());
        hasher.update(b"\0");
        match fs::read(path.join(&rel)) {
            Ok(bytes) => hasher.update(&bytes),
            Err(_) => hasher.update(b"<unreadable>"),
        }
        hasher.update(b"\0");
    }
    hex::encode(hasher.finalize())
}

fn walk(root: &Path, dir: &Path) -> Vec<PathBuf> {
    let mut out = vec![];
    let Ok(rd) = fs::read_dir(dir) else { return out };
    for entry in rd.flatten() {
        let p = entry.path();
        let name = entry.file_name();
        // 跳过 .git
        if name == ".git" {
            continue;
        }
        let rel = p.strip_prefix(root).unwrap_or(&p).to_path_buf();
        if p.is_dir() {
            out.extend(walk(root, &p));
        } else if p.is_file() || p.is_symlink() {
            out.push(rel);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(p: &Path, body: &[u8]) {
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(p, body).unwrap();
    }

    #[test]
    fn hash_is_stable_for_same_content() {
        let a = std::env::temp_dir().join("cstore_stable_a");
        let b = std::env::temp_dir().join("cstore_stable_b");
        for d in [&a, &b] {
            let _ = fs::remove_dir_all(d);
            write(&d.join("SKILL.md"), b"name: x\n");
            write(&d.join("lib/a.txt"), b"hello");
        }
        assert_eq!(compute_folder_hash(&a), compute_folder_hash(&b));
        let _ = fs::remove_dir_all(&a);
        let _ = fs::remove_dir_all(&b);
    }

    #[test]
    fn hash_changes_on_edit() {
        let a = std::env::temp_dir().join("cstore_edit_a");
        let b = std::env::temp_dir().join("cstore_edit_b");
        let _ = fs::remove_dir_all(&a);
        let _ = fs::remove_dir_all(&b);
        write(&a.join("SKILL.md"), b"name: x\n");
        write(&b.join("SKILL.md"), b"name: y\n");
        assert_ne!(compute_folder_hash(&a), compute_folder_hash(&b));
        let _ = fs::remove_dir_all(&a);
        let _ = fs::remove_dir_all(&b);
    }

    #[test]
    fn hash_ignores_git_dir() {
        let a = std::env::temp_dir().join("cstore_git_a");
        let b = std::env::temp_dir().join("cstore_git_b");
        let _ = fs::remove_dir_all(&a);
        let _ = fs::remove_dir_all(&b);
        write(&a.join("SKILL.md"), b"name: x\n");
        write(&b.join("SKILL.md"), b"name: x\n");
        write(&a.join(".git/HEAD"), b"ref: refs/heads/main\n");
        write(&b.join(".git/refs/heads/main"), b"deadbeef\n");
        assert_eq!(compute_folder_hash(&a), compute_folder_hash(&b));
        let _ = fs::remove_dir_all(&a);
        let _ = fs::remove_dir_all(&b);
    }

    #[test]
    fn canonical_path_under_agents_skills() {
        let p = canonical_path("foo-bar").unwrap();
        let s = p.to_string_lossy();
        assert!(s.ends_with(".agents/skills/foo-bar"));
    }
}
