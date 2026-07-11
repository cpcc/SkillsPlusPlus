use crate::models::{InstallStrategy, SkillItem};
use std::future::Future;
use std::pin::Pin;

pub trait SourceAdapter: Send + Sync {
    fn source_id(&self) -> &'static str;
    fn source_name(&self) -> &'static str;
    fn base_url(&self) -> &'static str;
    /// adapter 默认推荐的安装策略。用户可在 InstallDialog 中覆盖。
    fn default_install_strategy(&self) -> InstallStrategy {
        InstallStrategy::Git
    }
    fn fetch(&self) -> Pin<Box<dyn Future<Output = Result<Vec<SkillItem>, String>> + Send>>;

    fn fetch_with_warnings(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<(Vec<SkillItem>, Vec<String>), String>> + Send>> {
        let fut = self.fetch();
        Box::pin(async move { fut.await.map(|items| (items, vec![])) })
    }
}
