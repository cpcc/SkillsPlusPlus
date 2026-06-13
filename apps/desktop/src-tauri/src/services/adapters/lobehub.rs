use crate::models::SkillItem;
use crate::services::source::SourceAdapter;
use serde::Deserialize;
use std::pin::Pin;

pub struct LobehubAdapter;

#[derive(Deserialize)]
struct LobehubIndex {
    plugins: Vec<LobehubPlugin>,
}

#[derive(Deserialize)]
struct LobehubPlugin {
    identifier: String,
    author: Option<String>,
    homepage: Option<String>,
    #[serde(rename = "createdAt")]
    created_at: Option<String>,
    meta: LobehubMeta,
}

#[derive(Deserialize)]
struct LobehubMeta {
    title: String,
    description: Option<String>,
    tags: Option<Vec<String>>,
}

impl SourceAdapter for LobehubAdapter {
    fn source_id(&self) -> &'static str { "lobehub" }
    fn source_name(&self) -> &'static str { "LobeHub" }
    fn base_url(&self) -> &'static str { "https://lobehub.com/skills" }

    fn fetch(&self) -> Pin<Box<dyn std::future::Future<Output = Result<Vec<SkillItem>, String>> + Send>> {
        Box::pin(async {
            let client = reqwest::Client::builder()
                .user_agent("skills-plus-plus/0.1")
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .map_err(|e| e.to_string())?;

            let index: LobehubIndex = client
                .get("https://chat-plugins.lobehub.com/index.json")
                .send()
                .await
                .map_err(|e| e.to_string())?
                .json()
                .await
                .map_err(|e| e.to_string())?;

            let items = index.plugins.into_iter().map(|p| {
                let detail_url = p.homepage.clone()
                    .unwrap_or_else(|| format!("https://lobehub.com/plugins/{}", p.identifier));
                SkillItem {
                    id: format!("lobehub_{}", p.identifier),
                    name: p.meta.title,
                    author: p.author,
                    description: p.meta.description,
                    tags: p.meta.tags.unwrap_or_default(),
                    source_id: "lobehub".to_string(),
                    repo_url: p.homepage.clone(),
                    detail_url,
                    updated_at: p.created_at,
                    compatible_tools: vec!["通用".to_string()],
                    stars: None,
                }
            }).collect();

            Ok(items)
        })
    }
}
