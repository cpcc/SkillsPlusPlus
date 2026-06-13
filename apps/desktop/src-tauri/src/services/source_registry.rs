use crate::services::adapters::{GithubAdapter, LobehubAdapter, StubAdapter};
use crate::services::source::SourceAdapter;

pub struct SourceRegistry {
    adapters: Vec<Box<dyn SourceAdapter>>,
}

impl SourceRegistry {
    pub fn new() -> Self {
        SourceRegistry {
            adapters: vec![
                Box::new(GithubAdapter),
                Box::new(LobehubAdapter),
                Box::new(StubAdapter { id: "skillhub", name: "SkillHub.cn", url: "https://skillhub.cn" }),
                Box::new(StubAdapter { id: "clawhub", name: "ClawHub.ai", url: "https://clawhub.ai/skills" }),
                Box::new(StubAdapter { id: "skillsmp", name: "SkillsMP", url: "https://skillsmp.com" }),
            ],
        }
    }

    pub fn get_adapter(&self, source_id: &str) -> Option<&dyn SourceAdapter> {
        self.adapters.iter().find(|a| a.source_id() == source_id).map(|a| a.as_ref())
    }
}

impl Default for SourceRegistry {
    fn default() -> Self { Self::new() }
}
