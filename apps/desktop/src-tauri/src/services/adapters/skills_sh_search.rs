//! 兜底在线搜索：调 skills.sh `/api/search` 拿官方 70w 条 skill 数据。
//! 本地缓存（GitHub topic 拉取）仅 ~180 条，搜不到时降级到这里。

use crate::models::{InstallStrategy, SkillItem};
use serde::Deserialize;

/// skills.sh `/api/search` 顶层响应。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillsShSearchResponse {
    pub query: String,
    pub search_type: String,
    pub skills: Vec<SearchHit>,
    pub count: i64,
}

/// skills.sh 单条命中。注意：不返回 description/author/tags。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub id: String,
    pub skill_id: String,
    pub name: String,
    pub installs: i64,
    /// `owner/repo` 格式
    pub source: String,
}

/// 调 skills.sh `/api/search?q={query}&limit={n}`，把结果映射为 SkillItem。
/// `limit` 上限 50；`None` 默认 30。
pub async fn search(query: &str, limit: Option<u32>) -> Result<Vec<SkillItem>, String> {
    let limit = limit.unwrap_or(30).min(50);
    let client = reqwest::Client::builder()
        .user_agent("skills-plus-plus/0.1")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!(
        "https://skills.sh/api/search?q={}&limit={limit}",
        urlencoding::encode_query(query),
    );
    let resp = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("skills.sh search request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("skills.sh search returned {}", resp.status()));
    }

    let parsed: SkillsShSearchResponse = resp
        .json()
        .await
        .map_err(|e| format!("skills.sh search parse error: {e}"))?;

    Ok(parsed.skills.into_iter().map(hit_to_skill_item).collect())
}

fn hit_to_skill_item(hit: SearchHit) -> SkillItem {
    // source = "owner/repo"
    let (owner, repo) = hit
        .source
        .split_once('/')
        .map(|(o, r)| (o.to_string(), r.to_string()))
        .unwrap_or((hit.source.clone(), hit.skill_id.clone()));

    let archive_url = Some(crate::services::adapters::github::github_archive_url(
        &owner, &repo,
    ));

    SkillItem {
        id: format!("online_{}", hit.id),
        // skill_id 更短的可读名（如 `last30days`），优于 hit.name
        name: hit.skill_id,
        author: Some(owner),
        description: None,
        tags: vec![],
        source_id: "skills_sh".to_string(),
        repo_url: Some(format!("https://github.com/{}", hit.source)),
        detail_url: format!("https://skills.sh/{}", hit.id),
        updated_at: None,
        compatible_tools: vec![],
        // 复用 stars 字段装 installs（UI 上会显示为数字）
        stars: Some(hit.installs),
        install_strategy: Some(InstallStrategy::Git),
        archive_url,
        category: None,
    }
}

/// 极简 query 编码：空格 → `+`，其他保留字符按字节百分号编码。
/// 不引入 url crate 依赖即可满足 skills.sh 的需求。
mod urlencoding {
    pub fn encode_query(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for &b in s.as_bytes() {
            match b {
                b' ' => out.push('+'),
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    out.push(b as char);
                }
                _ => out.push_str(&format!("%{:02X}", b)),
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_maps_fields_correctly() {
        let hit = SearchHit {
            id: "abc123".to_string(),
            skill_id: "last30days".to_string(),
            name: "Last 30 Days".to_string(),
            installs: 474893,
            source: "mvanhorn/last30days-skill".to_string(),
        };
        let item = hit_to_skill_item(hit);
        assert_eq!(item.id, "online_abc123");
        assert_eq!(item.name, "last30days");
        assert_eq!(item.author.as_deref(), Some("mvanhorn"));
        assert_eq!(
            item.repo_url.as_deref(),
            Some("https://github.com/mvanhorn/last30days-skill"),
        );
        assert_eq!(item.detail_url, "https://skills.sh/abc123");
        assert_eq!(item.stars, Some(474893));
        assert_eq!(
            item.archive_url.as_deref(),
            Some("https://codeload.github.com/mvanhorn/last30days-skill/tar.gz/refs/heads/main"),
        );
        assert_eq!(item.source_id, "skills_sh");
    }

    #[test]
    fn hit_without_slash_in_source_falls_back() {
        let hit = SearchHit {
            id: "x".to_string(),
            skill_id: "foo".to_string(),
            name: "Foo".to_string(),
            installs: 0,
            source: "noslash".to_string(),
        };
        let item = hit_to_skill_item(hit);
        assert_eq!(item.author.as_deref(), Some("noslash"));
        assert!(item.archive_url.unwrap().contains("/noslash/"));
    }

    #[test]
    fn encode_query_handles_spaces_and_unicode() {
        assert_eq!(encode_query_test("hello world"), "hello+world");
        // 中文字符按 UTF-8 字节编码
        let enc = encode_query_test("数据库");
        assert!(enc.starts_with("%E6%95"));
    }

    fn encode_query_test(s: &str) -> String {
        urlencoding::encode_query(s)
    }
}
