use crate::models::{AppInfo, UpdateInfo};
use crate::services::{mirror, net_resilient};
use rusqlite::Connection;
use std::sync::{Arc, Mutex, OnceLock};
use tauri::{Manager, State};

/// GitHub Releases latest 接口。修改时同步 release.yml 与 git remote。
const RELEASES_API: &str = "https://api.github.com/repos/cpcc/SkillsPlusPlus/releases/latest";

pub struct DbState(pub Arc<Mutex<Connection>>);

/// 安装互斥锁：防止用户连点导致并发安装写同一目标目录。
/// 用 OnceLock 全局单例（不依赖 Tauri State，方便任何地方用）。
static INSTALL_LOCK: OnceLock<Arc<tokio::sync::Mutex<()>>> = OnceLock::new();

/// 拿到安装锁的函数（返回 guard，drop guard 后自动释放）。
/// guard 是 Send，可在 Tauri async 命令中使用。
pub async fn acquire_install_lock() -> tokio::sync::MutexGuard<'static, ()> {
    INSTALL_LOCK
        .get_or_init(|| Arc::new(tokio::sync::Mutex::new(())))
        .lock()
        .await
}

pub fn get_app_info_inner(
    conn: &Connection,
    version: String,
    db_path: String,
) -> Result<AppInfo, String> {
    // verify DB is accessible
    let _: i64 = conn
        .query_row("SELECT COUNT(*) FROM app_settings", [], |row| row.get(0))
        .unwrap_or(0);

    Ok(AppInfo {
        version,
        db_path,
        log_path: String::from("(see app data dir)"),
        platform: std::env::consts::OS.to_string(),
    })
}

#[tauri::command]
pub fn get_app_info(app: tauri::AppHandle, db: State<DbState>) -> Result<AppInfo, String> {
    let version = app.package_info().version.to_string();
    let db_path = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("skills_pp.db")
        .to_string_lossy()
        .to_string();
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    get_app_info_inner(&conn, version, db_path)
}

/// 将 "0.1.2" 解析为 (0, 1, 2)；非法片段视为 0。
fn parse_version(s: &str) -> (u32, u32, u32) {
    let s = s.trim().trim_start_matches('v').trim();
    let mut it = s.split('.').map(|p| p.parse::<u32>().unwrap_or(0));
    (
        it.next().unwrap_or(0),
        it.next().unwrap_or(0),
        it.next().unwrap_or(0),
    )
}

/// 调用 GitHub Releases `/releases/latest`，与当前版本比较，返回更新信息。
///
/// 失败（网络错误、4xx/5xx、JSON 解析失败）会向上抛出字符串错误，前端静默回退到
/// "无更新"显示，避免在版本号位置弹出红色错误。
#[tauri::command]
pub async fn check_app_update(app: tauri::AppHandle) -> Result<UpdateInfo, String> {
    let current = app.package_info().version.to_string();
    check_app_update_inner(current).await
}

/// 内部实现（与 Tauri `AppHandle` 解耦），http_bridge 复用。
pub async fn check_app_update_inner(current_version: String) -> Result<UpdateInfo, String> {
    // 走公共网络韧性工具：直连 → 镜像 → curl 兜底。
    // 参考 `docs/中国网络抖动解决方案借鉴.md`：国内直连 api.github.com 不稳定。
    let urls = mirror::candidate_urls(RELEASES_API);
    let opts = net_resilient::FetchOptions {
        urls,
        max_attempts: 2,
        headers: vec![
            net_resilient::Header::new("Accept", "application/vnd.github+json"),
            net_resilient::Header::new("User-Agent", format!("SkillsPlusPlus/{}", current_version)),
        ],
        ..Default::default()
    };
    let resp: serde_json::Value = net_resilient::fetch_json(&opts)
        .await
        .map(|(v, _etag)| v)?;

    let tag = resp["tag_name"].as_str().unwrap_or("").trim();
    let latest = tag.trim_start_matches('v').to_string();
    let release_url = resp["html_url"]
        .as_str()
        .unwrap_or("https://github.com/cpcc/SkillsPlusPlus/releases")
        .to_string();
    let release_notes = resp["body"].as_str().unwrap_or("").to_string();
    let published_at = resp["published_at"].as_str().unwrap_or("").to_string();

    let has_update = parse_version(&latest) > parse_version(&current_version);

    Ok(UpdateInfo {
        has_update,
        current_version: current_version,
        latest_version: latest,
        release_url,
        release_notes,
        published_at,
    })
}

/// 通过 tauri-plugin-opener 在系统浏览器打开 release 页面。
/// URL 不受 `opener:allow-open-path` scope 限制，默认即允许。
#[tauri::command]
pub fn open_release_url(app: tauri::AppHandle, url: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}
