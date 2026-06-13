use crate::models::SkillItem;
use crate::services::source::SourceAdapter;
use std::pin::Pin;

pub struct StubAdapter {
    pub id: &'static str,
    pub name: &'static str,
    pub url: &'static str,
}

impl SourceAdapter for StubAdapter {
    fn source_id(&self) -> &'static str { self.id }
    fn source_name(&self) -> &'static str { self.name }
    fn base_url(&self) -> &'static str { self.url }
    fn fetch(&self) -> Pin<Box<dyn std::future::Future<Output = Result<Vec<SkillItem>, String>> + Send>> {
        Box::pin(async { Ok(vec![]) })
    }
}
