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
pub struct InstallTaskRow {
    pub id: String,
    pub skill_id: Option<String>,
    pub skill_name: String,
    pub tool_name: String,
    pub directory_id: String,
    pub action: String,
    pub status: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub error_message: Option<String>,
    pub log_lines: Vec<String>,
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
}
