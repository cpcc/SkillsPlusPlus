// GitHub Search adapter — implemented in Task 2
use crate::models::SkillItem;
use crate::services::source::SourceAdapter;
use std::pin::Pin;

pub struct GithubAdapter;

impl SourceAdapter for GithubAdapter {
    fn source_id(&self) -> &'static str { "skills_sh" }
    fn source_name(&self) -> &'static str { "skills.sh" }
    fn base_url(&self) -> &'static str { "https://skills.sh" }
    fn fetch(&self) -> Pin<Box<dyn std::future::Future<Output = Result<Vec<SkillItem>, String>> + Send>> {
        Box::pin(async { Ok(vec![]) })
    }
}
