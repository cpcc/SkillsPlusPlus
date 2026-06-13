use crate::models::SkillItem;
use std::future::Future;
use std::pin::Pin;

pub trait SourceAdapter: Send + Sync {
    fn source_id(&self) -> &'static str;
    fn source_name(&self) -> &'static str;
    fn base_url(&self) -> &'static str;
    fn fetch(&self) -> Pin<Box<dyn Future<Output = Result<Vec<SkillItem>, String>> + Send>>;
}
