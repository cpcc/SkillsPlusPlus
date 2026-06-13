//! 跨平台 symlink。Windows 在权限不足时降级为递归 copy。

use std::fs;
use std::path::{Path, PathBuf};

/// 创建一个指向 `target` 的符号链接 `link`。
/// - Unix：使用 `std::os::unix::fs::symlink`（建一个文件/目录皆可的 symlink）。
/// - Windows：使用 `symlink_dir`；失败（未启用开发者模式 / 权限不足）时
///   打 warn 日志并降级为递归 copy。
pub fn create_symlink(target: &Path, link: &Path) -> Result<(), String> {
    // 若 link 已存在，先清理：symlink 指向同 target 视为幂等成功。
    if let Ok(meta) = fs::symlink_metadata(link) {
        if meta.file_type().is_symlink() {
            if let Ok(existing) = fs::read_link(link) {
                if existing == target {
                    return Ok(());
                }
            }
            // 指向别处：删除旧链接。
            let _ = fs::remove_file(link).or_else(|_| fs::remove_dir_all(link));
        } else if meta.is_dir() {
            // 已有真实目录：保留并降级为 copy（避免误删用户文件）。
            log::warn!(
                "symlink: link path {:?} is an existing dir; falling back to copy",
                link
            );
            return copy_recursive(target, link);
        } else {
            let _ = fs::remove_file(link);
        }
    }

    if let Some(parent) = link.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create symlink parent: {e}"))?;
    }

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link)
            .map_err(|e| format!("create symlink: {e}"))?;
        return Ok(());
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::symlink_dir;
        match symlink_dir(target, link) {
            Ok(()) => return Ok(()),
            Err(e) => {
                log::warn!(
                    "symlink_dir failed for {:?} -> {:?}: {e}; falling back to copy",
                    link,
                    target
                );
                return copy_recursive(target, link);
            }
        }
    }

    #[cfg(not(any(unix, windows)))]
    {
        log::warn!("symlink unsupported on this platform; falling back to copy");
        copy_recursive(target, link)
    }
}

/// 递归拷贝目录（用于 Windows 降级或显式 copy 策略）。
pub fn copy_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    if !src.exists() {
        return Err(format!("copy_recursive: src {:?} does not exist", src));
    }
    if src.is_dir() {
        fs::create_dir_all(dst).map_err(|e| format!("mkdir {dst:?}: {e}"))?;
        for entry in fs::read_dir(src).map_err(|e| format!("readdir {src:?}: {e}"))? {
            let entry = entry.map_err(|e| e.to_string())?;
            let from = entry.path();
            let to = dst.join(entry.file_name());
            copy_recursive(&from, &to)?;
        }
        Ok(())
    } else {
        fs::copy(src, dst).map_err(|e| format!("copy {src:?} -> {dst:?}: {e}"))?;
        Ok(())
    }
}

pub fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

pub fn read_symlink_target(path: &Path) -> Option<PathBuf> {
    fs::read_link(path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_symlink_and_reads_back() {
        let root = std::env::temp_dir().join("skills_pp_symlink_test");
        let _ = fs::remove_dir_all(&root);
        let target = root.join("target");
        let link = root.join("link");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("f.txt"), b"hi").unwrap();

        create_symlink(&target, &link).expect("symlink or copy");
        // 读回：要么是 symlink 指向 target，要么是 copy 后的目录，二选一。
        let ok = if is_symlink(&link) {
            read_symlink_target(&link).as_deref() == Some(&target)
        } else {
            link.is_dir() && link.join("f.txt").exists()
        };
        assert!(ok);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn copy_recursive_copies_subdirs() {
        let root = std::env::temp_dir().join("skills_pp_copy_recursive");
        let _ = fs::remove_dir_all(&root);
        let src = root.join("src");
        let dst = root.join("dst");
        fs::create_dir_all(src.join("sub")).unwrap();
        fs::write(src.join("a.txt"), b"a").unwrap();
        fs::write(src.join("sub/b.txt"), b"b").unwrap();

        copy_recursive(&src, &dst).unwrap();
        assert!(dst.join("a.txt").exists());
        assert!(dst.join("sub/b.txt").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn symlink_idempotent_same_target() {
        let root = std::env::temp_dir().join("skills_pp_symlink_idem");
        let _ = fs::remove_dir_all(&root);
        let target = root.join("t");
        let link = root.join("l");
        fs::create_dir_all(&target).unwrap();
        create_symlink(&target, &link).unwrap();
        create_symlink(&target, &link).unwrap();
        let _ = fs::remove_dir_all(&root);
    }
}
