use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub db_path: String,
    pub log_path: String,
    pub platform: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryRow {
    pub id: String,
    pub tool_name: String,
    pub path: String,
    pub is_default: bool,
    pub is_detected: bool,
    pub writable: bool,
    pub enabled: bool,
    pub skill_count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub id: String,
    pub tool_name: String,
    pub path: String,
    pub exists: bool,
    pub writable: bool,
    pub skill_count: i64,
}

/// 安装策略（serde 小写以对齐 DB / 前端字面量）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallStrategy {
    Git,
    Copy,
    Archive,
    SkillsCli,
}

impl InstallStrategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            InstallStrategy::Git => "git",
            InstallStrategy::Copy => "copy",
            InstallStrategy::Archive => "archive",
            InstallStrategy::SkillsCli => "skills_cli",
        }
    }

    /// 解析失败时回退为 git（兼容历史数据）。
    pub fn parse(s: &str) -> Self {
        match s {
            "copy" => InstallStrategy::Copy,
            "archive" => InstallStrategy::Archive,
            "skills_cli" => InstallStrategy::SkillsCli,
            _ => InstallStrategy::Git,
        }
    }
}

impl Default for InstallStrategy {
    fn default() -> Self {
        InstallStrategy::Git
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillItem {
    pub id: String,
    pub name: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub source_id: String,
    pub repo_url: Option<String>,
    pub detail_url: String,
    pub updated_at: Option<String>,
    pub compatible_tools: Vec<String>,
    pub stars: Option<i64>,
    /// adapter 声明的默认安装策略。
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub install_strategy: Option<InstallStrategy>,
    /// copy/archive/skills_cli 时使用的归档下载地址。
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub archive_url: Option<String>,
    /// CI 聚合阶段生成的分类（对齐 FilterBar 17 类）。registry 源必有；其它源可能为空。
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub category: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceRow {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshWarning {
    pub source_id: String,
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshSourcesResult {
    pub skills: Vec<SkillItem>,
    pub warnings: Vec<RefreshWarning>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallPreview {
    pub skill_name: String,
    pub repo_url: String,
    pub target_path: String,
    pub strategy: InstallStrategy,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub canonical_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub symlink_path: Option<String>,
    pub conflict: Option<ConflictInfo>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictInfo {
    pub existing_path: String,
    pub kind: String, // "existing_dir" | "existing_file"
}

/// 应用更新检查结果（GitHub Releases latest）。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub release_url: String,
    pub release_notes: String,
    pub published_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledSkillRow {
    pub id: String,
    pub skill_id: Option<String>,
    pub name: String,
    pub tool_name: String,
    pub directory_id: String,
    pub directory_path: String,
    pub source_id: Option<String>,
    pub repo_url: Option<String>,
    pub installed_at: String,
    pub status: String,
    pub install_strategy: InstallStrategy,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub content_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub canonical_path: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
}

// ─── 目录文件树（抽屉） ───────────────────────────────────────────────────────

/// 文件树节点类型。文件 → "file"，目录 → "dir"。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileNodeKind {
    File,
    Dir,
}

/// 抽屉中渲染的一棵目录树节点。
///
/// 设计要点：
/// - `relative_path` 永远用 `/` 分隔，方便前端直接当 key 用。
/// - `absolute_path` 直接给前端使用，不在前端拼路径。
/// - `children` 在文件 / 触达深度或节点数上限时为 `None`。
/// - `truncated=true` 表示当前层因 `max_depth` / `max_nodes` 被截断。
/// - `error` 非空时表示 `read_dir` 失败，行内红色提示。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileTreeNode {
    pub name: String,
    pub relative_path: String,
    pub absolute_path: String,
    pub kind: FileNodeKind,
    pub size: u64,
    /// dir 级：该目录是否含 SKILL.md（任意大小写）。
    pub has_skill_md: bool,
    /// dir 级：`has_skill_md` 或顶层含 `.md`/`.yaml`/`.yml` 文件。
    pub is_skill: bool,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub children: Option<Vec<FileTreeNode>>,
    pub truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error: Option<String>,
}
