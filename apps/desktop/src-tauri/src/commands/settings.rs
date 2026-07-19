use crate::commands::app::DbState;
use crate::services::mirror;
use crate::services::net_resilient;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tauri::State;

/// 镜像配置（前端 ↔ 后端）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorConfig {
    /// 是否启用镜像 fallback。
    pub enabled: bool,
    /// GitHub 镜像候选前缀列表（按优先级）。空字符串代表直连。
    pub github_mirrors: Vec<String>,
}

impl From<mirror::MirrorConfig> for MirrorConfig {
    fn from(v: mirror::MirrorConfig) -> Self {
        Self {
            enabled: v.enabled,
            github_mirrors: v.github_mirrors,
        }
    }
}

impl From<MirrorConfig> for mirror::MirrorConfig {
    fn from(v: MirrorConfig) -> Self {
        Self {
            enabled: v.enabled,
            github_mirrors: v.github_mirrors,
        }
    }
}

/// 单个镜像的健康检查结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorHealth {
    /// 镜像前缀（空字符串表示直连）。
    pub prefix: String,
    /// 是否可达（任一测试 URL 成功）。
    pub reachable: bool,
    /// 首个成功 URL 的响应时间（毫秒），失败时为 None。
    pub latency_ms: Option<u64>,
    /// 错误信息（仅 unreachable 时有值）。
    pub error: Option<String>,
}

/// 内部实现（与 Tauri `State<DbState>` 解耦），http_bridge 复用。
pub fn get_mirror_config_inner(conn: &Connection) -> Result<MirrorConfig, String> {
    Ok(mirror::load_from_db(conn).into())
}

/// 获取当前镜像配置。
#[tauri::command]
pub fn get_mirror_config(db: State<DbState>) -> Result<MirrorConfig, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    get_mirror_config_inner(&conn)
}

/// 内部实现（与 Tauri `State<DbState>` 解耦），http_bridge 复用。
pub fn set_mirror_config_inner(conn: &Connection, config: MirrorConfig) -> Result<(), String> {
    mirror::save_to_db(conn, &config.clone().into())
        .map_err(|e| e.to_string())?;
    mirror::reload_config(conn);
    Ok(())
}

/// 设置镜像配置并刷新全局状态。
#[tauri::command]
pub fn set_mirror_config(db: State<DbState>, config: MirrorConfig) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    set_mirror_config_inner(&conn, config)
}

/// 单个镜像健康检查：并发试 2 个小 URL，任一成功即认为可达。
async fn check_one_mirror(prefix: String) -> MirrorHealth {
    let test_urls = vec![
        "https://github.com/atom/atom/raw/master/.gitignore",
        "https://raw.githubusercontent.com/vercel/vercel/main/README.md",
    ];
    let wrapped_urls: Vec<String> = test_urls
        .iter()
        .map(|u| mirror::wrap(&prefix, u))
        .collect();

    let mut reachable = false;
    let mut min_latency: Option<u64> = None;
    let mut last_err: Option<String> = None;

    for url in &wrapped_urls {
        let url_start = std::time::Instant::now();
        let opts = net_resilient::FetchOptions {
            urls: vec![url.clone()],
            max_attempts: 1,
            connect_timeout: std::time::Duration::from_secs(3),
            read_timeout: std::time::Duration::from_secs(5),
            ..Default::default()
        };
        match net_resilient::fetch_bytes(&opts).await {
            Ok(_) => {
                reachable = true;
                let latency = url_start.elapsed().as_millis() as u64;
                min_latency = Some(min_latency.map_or(latency, |x| x.min(latency)));
                break;
            }
            Err(e) => {
                last_err = Some(e);
            }
        }
    }

    MirrorHealth {
        prefix,
        reachable,
        latency_ms: min_latency,
        error: if reachable { None } else { last_err },
    }
}

/// 内部实现（无 DB 依赖），http_bridge 复用。
pub async fn check_mirror_health_inner() -> Result<Vec<MirrorHealth>, String> {
    let cfg = mirror::current_config();
    let mut tasks = vec![];

    for prefix in &cfg.github_mirrors {
        let p = prefix.clone();
        tasks.push(tokio::spawn(async move { check_one_mirror(p).await }));
    }

    let mut results = vec![];
    for task in tasks {
        let health = task.await.map_err(|e| format!("join error: {e}"))?;
        results.push(health);
    }
    Ok(results)
}

/// 检查各镜像的可达性（并发测试）。
///
/// 测试用的小文件：
/// - 直连：`https://github.com/atom/atom/raw/master/.gitignore`
/// - 镜像：同样的 URL 通过各镜像前缀包装。
///
/// 返回各镜像的健康状态（reachable / latency_ms / error）。
#[tauri::command]
pub async fn check_mirror_health() -> Result<Vec<MirrorHealth>, String> {
    check_mirror_health_inner().await
}