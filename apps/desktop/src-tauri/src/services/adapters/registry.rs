//! 官方聚合 adapter：从 HuggingFace Dataset 拉 CI 抓取的聚合 JSON。
//!
//! 流程：HF 主站 → hf-mirror（国内 CDN 反代）→ 本地磁盘缓存。
//! 任意一个成功就返回。失败时降级到本地缓存 + `log::warn!`（上层 UI 可加 Toast）。

use crate::models::{InstallStrategy, SkillItem};
use crate::services::source::SourceAdapter;
use serde::Deserialize;
use std::path::PathBuf;
use std::pin::Pin;
use std::time::{Duration, SystemTime};

/// HuggingFace 用户名 / 组织名。
///
/// HF Dataset 仓库：`https://huggingface.co/datasets/<HF_USER>/aiskills-registry`。
/// 由用户在 https://huggingface.co/new-dataset 创建；push 由 CI workflow 通过
/// `HF_USER` / `HF_TOKEN` secret 完成。
pub const HF_USER: &str = "futuregateway";

/// HuggingFace Dataset 仓库名。
const HF_DATASET_REPO: &str = "aiskills-registry";
/// CI 推送的主文件名。
const SKILLS_JSON: &str = "skills.json";

/// 本地缓存有效期：超过这个时间会重新走网络。
const CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);

/// 单 URL 超时。
const FETCH_TIMEOUT: Duration = Duration::from_secs(8);

const ETAG_FILE: &str = "registry.etag";

fn build_urls() -> [String; 2] {
    let path = format!("/datasets/{HF_USER}/{HF_DATASET_REPO}/resolve/main/{SKILLS_JSON}");
    [
        format!("https://huggingface.co{path}"),
        // 国内 CDN 反代
        format!("https://hf-mirror.com{path}"),
    ]
}

/// 本地缓存路径：`<cache_dir>/skillspp/registry.json`。
/// macOS: `~/Library/Caches/skillspp/registry.json`
/// Linux: `~/.cache/skillspp/registry.json`
/// Windows: `%LOCALAPPDATA%\skillspp\registry.json`
fn cache_path() -> Option<PathBuf> {
    dirs::cache_dir().map(|d| d.join("skillspp").join("registry.json"))
}

fn etag_path() -> Option<PathBuf> {
    dirs::cache_dir().map(|d| d.join("skillspp").join(ETAG_FILE))
}

fn read_etag() -> Option<String> {
    let path = etag_path()?;
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn write_etag(value: &str) {
    let Some(path) = etag_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(e) = std::fs::write(&path, value) {
        log::warn!("registry: failed to write etag {:?}: {e}", path);
    }
}

fn cache_is_fresh_at(modified: SystemTime, now: SystemTime) -> bool {
    now.duration_since(modified)
        .map(|age| age < CACHE_TTL)
        .unwrap_or(false)
}

pub struct RegistryAdapter;

#[derive(Debug, Deserialize)]
struct RegistryPayload {
    #[allow(dead_code)]
    version: u32,
    #[allow(dead_code)]
    #[serde(rename = "generatedAt")]
    generated_at: Option<String>,
    skills: Vec<SkillItem>,
}

async fn fetch_registry_with_warnings() -> Result<(Vec<SkillItem>, Vec<String>), String> {
    let mut warnings = vec![];

    // 0) 占位 HF_USER 时不发请求——直接降级本地缓存。
    if HF_USER == "<hf_user>" {
        log::warn!("registry: HF_USER placeholder, falling back to local cache only");
        warnings.push("官方聚合未配置远端数据集，已回退到本地缓存。".to_string());
        return load_local_cache()
            .map(|skills| (skills, warnings))
            .ok_or_else(|| {
                "registry not configured (HF_USER placeholder) and no local cache".to_string()
            });
    }

    if let Some(skills) = load_fresh_local_cache() {
        return Ok((skills, warnings));
    }

    let client = reqwest::Client::builder()
        .user_agent("skills-plus-plus/0.1")
        .timeout(FETCH_TIMEOUT)
        .build()
        .map_err(|e| e.to_string())?;

    let etag = read_etag();

    // 1) 依次尝试 HF 主站 → hf-mirror
    for url in build_urls() {
        let mut req = client.get(&url);
        if let Some(etag) = etag.as_deref() {
            req = req.header(reqwest::header::IF_NONE_MATCH, etag);
        }
        match req.send().await {
            Ok(r) if r.status() == reqwest::StatusCode::NOT_MODIFIED => {
                if let Some(skills) = load_local_cache() {
                    return Ok((skills, warnings));
                }
                log::warn!("registry: {url} returned 304 but local cache missing");
            }
            Ok(r) if r.status().is_success() => {
                let response_etag = r
                    .headers()
                    .get(reqwest::header::ETAG)
                    .and_then(|v| v.to_str().ok())
                    .map(str::to_string);
                let body = match r.bytes().await {
                    Ok(b) => b,
                    Err(e) => {
                        log::warn!("registry: {url} body read failed: {e}");
                        continue;
                    }
                };
                let parsed: RegistryPayload = match serde_json::from_slice(&body) {
                    Ok(p) => p,
                    Err(e) => {
                        log::warn!("registry: {url} parse failed: {e}");
                        continue;
                    }
                };
                if let Some(p) = cache_path() {
                    if let Some(parent) = p.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if let Err(e) = std::fs::write(&p, &body) {
                        log::warn!("registry: failed to write cache {:?}: {e}", p);
                    }
                }
                if let Some(etag) = response_etag.as_deref() {
                    write_etag(etag);
                }
                return Ok((parsed.skills, warnings));
            }
            Ok(r) => {
                log::warn!("registry: {url} returned status {}", r.status());
            }
            Err(e) => {
                log::warn!("registry: {url} fetch failed: {e}");
            }
        }
    }

    warnings.push("官方聚合远端拉取失败，已回退到本地缓存。".to_string());
    load_local_cache()
        .map(|skills| (skills, warnings))
        .ok_or_else(|| "registry: all upstream sources failed and no cache".to_string())
}

impl SourceAdapter for RegistryAdapter {
    fn source_id(&self) -> &'static str { "registry" }
    fn source_name(&self) -> &'static str { "官方聚合" }
    fn base_url(&self) -> &'static str {
        // SourceRegistry 只用它做展示用；动态拼需要 format!，这里给固定串。
        "https://huggingface.co/datasets/<hf_user>/aiskills-registry"
    }
    fn default_install_strategy(&self) -> InstallStrategy { InstallStrategy::Git }

    fn fetch(&self) -> Pin<Box<dyn std::future::Future<Output = Result<Vec<SkillItem>, String>> + Send>> {
        Box::pin(async { fetch_registry_with_warnings().await.map(|(skills, _)| skills) })
    }

    fn fetch_with_warnings(
        &self,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<(Vec<SkillItem>, Vec<String>), String>> + Send>> {
        Box::pin(fetch_registry_with_warnings())
    }
}

fn load_fresh_local_cache() -> Option<Vec<SkillItem>> {
    if !local_cache_is_fresh() {
        return None;
    }
    load_local_cache()
}

/// 读本地缓存。**不**校验 TTL——仅在主备 URL 都失败时兜底用。
fn load_local_cache() -> Option<Vec<SkillItem>> {
    let path = cache_path()?;
    let bytes = std::fs::read(&path).ok()?;
    let parsed: RegistryPayload = serde_json::from_slice(&bytes).ok()?;
    Some(parsed.skills)
}

/// 判断本地缓存是否新鲜（用于 UI 判断是否需要刷新；adapter.fetch() 内部不调用）。
pub fn local_cache_is_fresh() -> bool {
    let Some(path) = cache_path() else { return false };
    let Ok(meta) = std::fs::metadata(&path) else { return false };
    let Ok(modified) = meta.modified() else { return false };
    cache_is_fresh_at(modified, SystemTime::now())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_JSON: &str = r#"{
        "version": 1,
        "generatedAt": "2026-06-19T04:17:00Z",
        "stats": { "total": 1 },
        "skills": [{
            "id": "registry_skills_sh_foo",
            "name": "foo",
            "author": "octocat",
            "description": "demo",
            "tags": ["dev"],
            "category": "开发编程",
            "sourceId": "skills_sh",
            "repoUrl": "https://github.com/octocat/foo",
            "detailUrl": "https://skills.sh/foo",
            "updatedAt": "2026-06-19T00:00:00Z",
            "compatibleTools": ["Claude"],
            "stars": 42,
            "installStrategy": "git",
            "archiveUrl": "https://codeload.github.com/octocat/foo/tar.gz/refs/heads/main"
        }]
    }"#;

    #[test]
    fn parses_registry_payload() {
        let p: RegistryPayload = serde_json::from_str(SAMPLE_JSON).unwrap();
        assert_eq!(p.skills.len(), 1);
        let s = &p.skills[0];
        assert_eq!(s.id, "registry_skills_sh_foo");
        assert_eq!(s.source_id, "skills_sh");
        assert_eq!(s.category.as_deref(), Some("开发编程"));
        assert_eq!(s.stars, Some(42));
        assert!(matches!(s.install_strategy, Some(InstallStrategy::Git)));
    }

    #[test]
    fn build_urls_uses_hf_user_and_mirror() {
        let urls = build_urls();
        assert!(urls[0].starts_with("https://huggingface.co/datasets/"));
        assert!(urls[0].ends_with("/resolve/main/skills.json"));
        assert!(urls[1].starts_with("https://hf-mirror.com/datasets/"));
    }

    #[test]
    fn cache_path_under_skillspp_subdir() {
        let Some(p) = cache_path() else { return };
        assert!(p.ends_with("skillspp/registry.json") || p.ends_with(r"skillspp\registry.json"));
    }

    #[test]
    fn etag_path_under_skillspp_subdir() {
        let Some(p) = etag_path() else { return };
        assert!(p.ends_with("skillspp/registry.etag") || p.ends_with(r"skillspp\registry.etag"));
    }

    #[test]
    fn cache_age_younger_than_ttl_is_fresh() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(7 * 24 * 60 * 60);
        let modified = now - Duration::from_secs(60 * 60);
        assert!(cache_is_fresh_at(modified, now));
    }

    #[test]
    fn cache_age_at_ttl_boundary_is_stale() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(7 * 24 * 60 * 60);
        let modified = now - CACHE_TTL;
        assert!(!cache_is_fresh_at(modified, now));
    }
}
