use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AppInfo {
    pub version: String,
    pub db_path: String,
    pub log_path: String,
    pub platform: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub struct ScanResult {
    pub id: String,
    pub tool_name: String,
    pub path: String,
    pub exists: bool,
    pub writable: bool,
    pub skill_count: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SourceRow {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
pub struct InstallPreview {
    pub skill_name: String,
    pub repo_url: String,
    pub target_path: String,
    pub conflict: Option<ConflictInfo>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConflictInfo {
    pub existing_path: String,
    pub kind: String, // "existing_dir" | "existing_file"
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
}
