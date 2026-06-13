use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AppInfo {
    pub version: String,
    pub db_path: String,
    pub log_path: String,
    pub platform: String,
}
