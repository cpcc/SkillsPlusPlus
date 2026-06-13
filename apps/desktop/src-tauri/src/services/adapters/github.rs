use crate::models::SkillItem;
use crate::services::source::SourceAdapter;
use serde::Deserialize;
use std::collections::HashSet;
use std::pin::Pin;

pub struct GithubAdapter;

#[derive(Deserialize)]
struct GithubSearchResponse {
    items: Vec<GithubRepo>,
}

#[derive(Deserialize)]
struct GithubRepo {
    id: u64,
    name: String,
    description: Option<String>,
    html_url: String,
    stargazers_count: i64,
    updated_at: String,
    topics: Vec<String>,
    owner: GithubOwner,
}

#[derive(Deserialize)]
struct GithubOwner {
    login: String,
}

const TOPICS: &[&str] = &[
    "claude-skill",
    "codex-skill",
    "copilot-skill",
    "gemini-skill",
    "opencode-skill",
    "ai-skill",
];

fn infer_tools(topics: &[String]) -> Vec<String> {
    let t = topics.join(" ").to_lowercase();
    let mut tools = vec![];
    if t.contains("claude") { tools.push("Claude".to_string()); }
    if t.contains("codex") { tools.push("Codex".to_string()); }
    if t.contains("copilot") { tools.push("GitHub Copilot".to_string()); }
    if t.contains("gemini") { tools.push("Gemini CLI".to_string()); }
    if t.contains("cursor") { tools.push("Cursor".to_string()); }
    if t.contains("opencode") { tools.push("OpenCode".to_string()); }
    if tools.is_empty() { tools.push("通用".to_string()); }
    tools
}

impl SourceAdapter for GithubAdapter {
    fn source_id(&self) -> &'static str { "skills_sh" }
    fn source_name(&self) -> &'static str { "skills.sh" }
    fn base_url(&self) -> &'static str { "https://skills.sh" }

    fn fetch(&self) -> Pin<Box<dyn std::future::Future<Output = Result<Vec<SkillItem>, String>> + Send>> {
        Box::pin(async {
            let client = reqwest::Client::builder()
                .user_agent("skills-plus-plus/0.1")
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .map_err(|e| e.to_string())?;

            let mut all_items: Vec<SkillItem> = vec![];
            let mut seen: HashSet<u64> = HashSet::new();

            for topic in TOPICS {
                let url = format!(
                    "https://api.github.com/search/repositories?q=topic:{topic}&sort=stars&order=desc&per_page=30"
                );
                let result = client
                    .get(&url)
                    .header("Accept", "application/vnd.github+json")
                    .header("X-GitHub-Api-Version", "2022-11-28")
                    .send()
                    .await;

                let resp: GithubSearchResponse = match result {
                    Ok(r) => match r.json().await {
                        Ok(d) => d,
                        Err(e) => { log::warn!("GitHub parse error for topic {topic}: {e}"); continue; }
                    },
                    Err(e) => { log::warn!("GitHub fetch error for topic {topic}: {e}"); continue; }
                };

                for repo in resp.items {
                    if !seen.insert(repo.id) { continue; }
                    let compatible_tools = infer_tools(&repo.topics);
                    let tags: Vec<String> = repo.topics
                        .iter()
                        .filter(|t| !t.ends_with("-skill"))
                        .cloned()
                        .collect();
                    all_items.push(SkillItem {
                        id: format!("skills_sh_{}", repo.id),
                        name: repo.name,
                        author: Some(repo.owner.login),
                        description: repo.description,
                        tags: if tags.is_empty() { vec!["skill".to_string()] } else { tags },
                        source_id: "skills_sh".to_string(),
                        repo_url: Some(repo.html_url.clone()),
                        detail_url: repo.html_url,
                        updated_at: Some(repo.updated_at),
                        compatible_tools,
                        stars: Some(repo.stargazers_count),
                    });
                }

                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            }

            Ok(all_items)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_tools_detects_claude() {
        let tools = infer_tools(&["claude-skill".to_string(), "coding".to_string()]);
        assert!(tools.contains(&"Claude".to_string()));
    }

    #[test]
    fn infer_tools_detects_multiple() {
        let tools = infer_tools(&[
            "claude-skill".to_string(),
            "copilot-skill".to_string(),
        ]);
        assert!(tools.contains(&"Claude".to_string()));
        assert!(tools.contains(&"GitHub Copilot".to_string()));
    }

    #[test]
    fn infer_tools_defaults_to_generic() {
        let tools = infer_tools(&["coding".to_string(), "utility".to_string()]);
        assert_eq!(tools, vec!["通用".to_string()]);
    }

    #[test]
    fn infer_tools_detects_gemini() {
        let tools = infer_tools(&["gemini-skill".to_string()]);
        assert!(tools.contains(&"Gemini CLI".to_string()));
    }
}
