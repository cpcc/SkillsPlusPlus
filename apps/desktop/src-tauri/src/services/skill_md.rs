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

/// 从 SKILL.md frontmatter 中提取的完整元数据。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontmatterMeta {
    pub author: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub updated_at: Option<String>,
}

/// 解析 frontmatter 并同时返回剥离后的正文。
/// 若无有效元数据则返回 `(None, content)`。
pub fn parse_frontmatter_and_strip(content: &str) -> (Option<FrontmatterMeta>, String) {
    let Some(front) = extract_frontmatter(content) else {
        return (None, content.to_string());
    };
    let body = strip_frontmatter(content);

    let author = field(&front, "author")
        .map(|s| s.trim().trim_matches('"').trim().to_string())
        .filter(|s| !s.is_empty());
    let description = field(&front, "description")
        .map(|s| s.trim().trim_matches('"').trim().to_string())
        .filter(|s| !s.is_empty());
    let updated_at = field(&front, "updated_at")
        .or_else(|| field(&front, "updatedAt"))
        .map(|s| s.trim().trim_matches('"').trim().to_string())
        .filter(|s| !s.is_empty());
    let tags = parse_tags(&front);

    if author.is_none() && description.is_none() && updated_at.is_none() && tags.is_empty() {
        (None, body)
    } else {
        (
            Some(FrontmatterMeta {
                author,
                description,
                tags,
                updated_at,
            }),
            body,
        )
    }
}

/// 从 frontmatter 文本中解析 `tags` 字段，支持三种格式：
/// - 内联数组：`tags: [a, b]`
/// - YAML 列表：`tags:\n  - a\n  - b`
/// - 单值：`tags: some-tag`
fn parse_tags(front: &str) -> Vec<String> {
    // 先尝内联数组格式 tags: [a, b, c]
    if let Some(v) = field(front, "tags") {
        let v = v.trim();
        if v.starts_with('[') && v.ends_with(']') {
            let inner = &v[1..v.len() - 1];
            return inner
                .split(',')
                .map(|s| s.trim().trim_matches('"').trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
    }

    // 再尝 YAML 列表格式（跨行）
    let mut in_tags = false;
    let mut tags: Vec<String> = Vec::new();
    for line in front.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("tags:") {
            in_tags = true;
            // tags: 后面跟单值（不能在单行同时是 YAML 列表项）
            let rest = trimmed.strip_prefix("tags:").unwrap_or_default().trim();
            if rest.starts_with('[') || rest.starts_with('#') || rest.is_empty() {
                continue;
            }
            // 单值
            let tag = rest.trim_matches('"').trim().to_string();
            if !tag.is_empty() {
                tags.push(tag);
            }
            break;
        }
        if in_tags {
            if let Some(item) = trimmed.strip_prefix("- ") {
                tags.push(item.trim().trim_matches('"').trim().to_string());
            } else if trimmed.is_empty() || trimmed.starts_with('#') {
                // skip blank/comment lines
            } else {
                // 不是列表项，结束
                break;
            }
        }
    }

    tags
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

    // ─── parse_frontmatter_and_strip tests ──────────────────────────────

    #[test]
    fn pfs_all_fields() {
        let (meta, body) = parse_frontmatter_and_strip(
            "---\nname: my-skill\nauthor: Alice\ndescription: \"Does stuff\"\ntags: [a, b]\nupdated_at: 2024-01-15\n---\n\nBody text\n",
        );
        let m = meta.expect("should have meta");
        assert_eq!(m.author.as_deref(), Some("Alice"));
        assert_eq!(m.description.as_deref(), Some("Does stuff"));
        assert_eq!(m.tags, vec!["a", "b"]);
        assert_eq!(m.updated_at.as_deref(), Some("2024-01-15"));
        assert!(body.contains("Body text"));
        assert!(!body.contains("---"));
    }

    #[test]
    fn pfs_inline_array_tags() {
        let (meta, _) = parse_frontmatter_and_strip(
            "---\ntags: [rust, cli, \"ai\"]\n---\n\nbody\n",
        );
        let tags = meta.unwrap().tags;
        assert_eq!(tags, vec!["rust", "cli", "ai"]);
    }

    #[test]
    fn pfs_yaml_list_tags() {
        let (meta, _) = parse_frontmatter_and_strip(
            "---\ntags:\n  - rust\n  - cli\n  - ai\n---\n\nbody\n",
        );
        let tags = meta.unwrap().tags;
        assert_eq!(tags, vec!["rust", "cli", "ai"]);
    }

    #[test]
    fn pfs_single_value_tag() {
        let (meta, _) = parse_frontmatter_and_strip(
            "---\ntags: utilities\n---\n\nbody\n",
        );
        let tags = meta.unwrap().tags;
        assert_eq!(tags, vec!["utilities"]);
    }

    #[test]
    fn pfs_empty_array_tags_returns_empty_vec() {
        let (meta, _) = parse_frontmatter_and_strip(
            "---\ntags: []\n---\n\nbody\n",
        );
        // tags: [] alone shouldn't create meta — no tracked fields
        assert!(meta.is_none());
    }

    #[test]
    fn pfs_no_frontmatter_returns_none() {
        let (meta, body) = parse_frontmatter_and_strip("# Just a heading\nSome text");
        assert!(meta.is_none());
        assert_eq!(body, "# Just a heading\nSome text");
    }

    #[test]
    fn pfs_only_name_no_tracked_fields() {
        let (meta, _) = parse_frontmatter_and_strip(
            "---\nname: foo\n---\n\nbody\n",
        );
        assert!(meta.is_none(), "name is not a tracked field for FrontmatterMeta");
    }

    #[test]
    fn pfs_quoted_tags() {
        let (meta, _) = parse_frontmatter_and_strip(
            "---\ntags: [\"openai\", \"llm\"]\n---\n\nbody\n",
        );
        let tags = meta.unwrap().tags;
        assert_eq!(tags, vec!["openai", "llm"]);
    }

    #[test]
    fn pfs_body_preserved() {
        let input = "---\ndescription: Test\n---\n\n# Heading\n\nParagraph text\n- item 1\n- item 2\n";
        let (meta, body) = parse_frontmatter_and_strip(input);
        assert!(meta.is_some());
        assert!(body.contains("# Heading"));
        assert!(body.contains("Paragraph text"));
        assert!(!body.contains("---"));
    }

    #[test]
    fn parse_tags_inline_array_with_spaces() {
        let (meta, _) = parse_frontmatter_and_strip(
            "---\ntags: [  rust , cli , ai  ]\n---\n\nbody\n",
        );
        assert_eq!(meta.unwrap().tags, vec!["rust", "cli", "ai"]);
    }
}
