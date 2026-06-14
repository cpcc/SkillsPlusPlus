//! SKILL.md frontmatter 解析（零依赖的最小 YAML 子集）与目录规范化。

use std::path::{Path, PathBuf};

/// 从 SKILL.md frontmatter 中提取的元数据。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillManifest {
    pub name: String,
    pub description: Option<String>,
}

/// 读取 `<dir>/SKILL.md`（大小写不敏感：优先 SKILL.md，回退 skill.md），
/// 取首对 `---` 之间的 frontmatter，按 `key: value` 解析 `name` / `description`。
/// 没有 SKILL.md 或 frontmatter 缺 `name` 时返回 None。
pub fn parse_skill_md(dir: &Path) -> Option<SkillManifest> {
    let content = read_skill_md(dir)?;
    let front = extract_frontmatter(&content)?;
    let name = field(&front, "name")?.trim().trim_matches('"').trim().to_string();
    if name.is_empty() {
        return None;
    }
    let description = field(&front, "description")
        .map(|s| s.trim().trim_matches('"').trim().to_string())
        .filter(|s| !s.is_empty());
    Some(SkillManifest { name, description })
}

fn read_skill_md(dir: &Path) -> Option<String> {
    for name in ["SKILL.md", "skill.md", "Skill.md"] {
        let p = dir.join(name);
        if let Ok(c) = std::fs::read_to_string(&p) {
            return Some(c);
        }
    }
    None
}

/// 剥离 YAML frontmatter（首对 `---`），返回正文部分。
/// 若无 frontmatter 则返回原文。
pub fn strip_frontmatter(content: &str) -> String {
    let trimmed = content.trim_start_matches(['\u{feff}', '\n', '\r', ' ', '\t']);
    let Some(first_line) = trimmed.lines().next() else {
        return content.to_string();
    };
    if first_line.trim() != "---" {
        return content.to_string();
    }
    let rest = &trimmed[first_line.len()..];
    let Some(end) = rest.find("\n---") else {
        return content.to_string();
    };
    // 跳到 closing `---` 之后
    let after = &rest[end + 4..];
    after.strip_prefix('\n').unwrap_or(after).to_string()
}

/// 取首对 `---` 围起的 frontmatter 文本（不含围栏）。
fn extract_frontmatter(content: &str) -> Option<String> {
    let trimmed = content.trim_start_matches(['\u{feff}', '\n', '\r', ' ', '\t']);
    let first_line = trimmed.lines().next()?;
    if first_line.trim() != "---" {
        return None;
    }
    let rest = &trimmed[first_line.len()..];
    let end = rest.find("\n---")?;
    let body = &rest[..end];
    // 去掉 body 首行换行
    let body = body.strip_prefix('\n').unwrap_or(body);
    Some(body.to_string())
}

/// 在 frontmatter 文本中按行查找 `key:` 行（精确 key 匹配，忽略大小写），
/// 返回冒号右侧的原始 value 文本。
fn field(front: &str, key: &str) -> Option<String> {
    for line in front.lines() {
        let line = line.trim_end();
        if line.is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once(':') else { continue };
        if k.trim() == key {
            return Some(v.to_string());
        }
    }
    None
}

/// 把目录名规范化为 manifest.name（若不一致则 rename）。
/// 返回最终的目录路径；目录不存在或 rename 失败时回退原路径。
pub fn normalize_skill_dir(dir: &Path, manifest: &SkillManifest) -> PathBuf {
    let safe = sanitize_name(&manifest.name);
    let current_name = dir.file_name().and_then(|s| s.to_str()).unwrap_or("");
    if safe.is_empty() || safe == current_name {
        return dir.to_path_buf();
    }
    let parent = match dir.parent() {
        Some(p) => p,
        None => return dir.to_path_buf(),
    };
    let new_path = parent.join(&safe);
    if new_path.exists() {
        // 目标已存在（可能为旧规范目录或冲突）—— 不强行覆盖，保留原目录。
        return dir.to_path_buf();
    }
    match std::fs::rename(dir, &new_path) {
        Ok(_) => new_path,
        Err(e) => {
            log::warn!("normalize_skill_dir: rename {:?} -> {:?} failed: {e}", dir, new_path);
            dir.to_path_buf()
        }
    }
}

/// 将 skill name 中除 `[A-Za-z0-9._-]` 外的字符替换为 `-`。
pub fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(dir: &Path, body: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("SKILL.md"), body).unwrap();
    }

    #[test]
    fn parses_frontmatter() {
        let tmp = std::env::temp_dir().join("skill_md_parse_frontmatter");
        let _ = fs::remove_dir_all(&tmp);
        write(
            &tmp,
            "---\nname: foo-bar\ndescription: \"A skill\"\nlicense: MIT\n---\n\nbody\n",
        );
        let m = parse_skill_md(&tmp).expect("manifest");
        assert_eq!(m.name, "foo-bar");
        assert_eq!(m.description.as_deref(), Some("A skill"));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn no_frontmatter_returns_none() {
        let tmp = std::env::temp_dir().join("skill_md_no_frontmatter");
        let _ = fs::remove_dir_all(&tmp);
        write(&tmp, "# Just a doc\nno frontmatter here");
        assert!(parse_skill_md(&tmp).is_none());
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn frontmatter_without_name_returns_none() {
        let tmp = std::env::temp_dir().join("skill_md_no_name");
        let _ = fs::remove_dir_all(&tmp);
        write(&tmp, "---\ndescription: x\n---\n");
        assert!(parse_skill_md(&tmp).is_none());
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn missing_skill_md_returns_none() {
        let tmp = std::env::temp_dir().join("skill_md_missing");
        let _ = fs::remove_dir_all(&tmp);
        let _ = fs::create_dir_all(&tmp);
        assert!(parse_skill_md(&tmp).is_none());
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn normalizes_dir_name() {
        let parent = std::env::temp_dir().join("skill_md_normalize_parent");
        let _ = fs::remove_dir_all(&parent);
        fs::create_dir_all(&parent).unwrap();
        let dir = parent.join("Repo-Name");
        write(&dir, "---\nname: my-cool-skill\n---\n");
        let m = parse_skill_md(&dir).unwrap();
        let final_path = normalize_skill_dir(&dir, &m);
        assert_eq!(final_path.file_name().unwrap().to_str().unwrap(), "my-cool-skill");
        assert!(final_path.exists());
        let _ = fs::remove_dir_all(&parent);
    }

    #[test]
    fn sanitize_replaces_special() {
        assert_eq!(sanitize_name("Foo Bar"), "Foo-Bar");
        assert_eq!(sanitize_name("a/b@c"), "a-b-c");
    }
}
