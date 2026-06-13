// LobeHub adapter — implemented in Task 3
use crate::models::SkillItem;
use crate::services::source::SourceAdapter;
use std::pin::Pin;

pub struct LobehubAdapter;

impl SourceAdapter for LobehubAdapter {
    fn source_id(&self) -> &'static str { "lobehub" }
    fn source_name(&self) -> &'static str { "LobeHub" }
    fn base_url(&self) -> &'static str { "https://lobehub.com/skills" }
    fn fetch(&self) -> Pin<Box<dyn std::future::Future<Output = Result<Vec<SkillItem>, String>> + Send>> {
        Box::pin(async { Ok(vec![]) })
    }
}
