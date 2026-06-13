use crate::models::{InstallStrategy, SkillItem};
use crate::services::source::SourceAdapter;
use std::pin::Pin;

pub struct StubAdapter {
    pub id: &'static str,
    pub name: &'static str,
    pub url: &'static str,
    pub strategy: InstallStrategy,
}

impl Default for StubAdapter {
    fn default() -> Self {
        StubAdapter {
            id: "stub",
            name: "Stub",
            url: "",
            strategy: InstallStrategy::Git,
        }
    }
}

impl SourceAdapter for StubAdapter {
    fn source_id(&self) -> &'static str { self.id }
    fn source_name(&self) -> &'static str { self.name }
    fn base_url(&self) -> &'static str { self.url }
    fn default_install_strategy(&self) -> InstallStrategy { self.strategy }
    fn fetch(&self) -> Pin<Box<dyn std::future::Future<Output = Result<Vec<SkillItem>, String>> + Send>> {
        Box::pin(async { Ok(vec![]) })
    }
}
