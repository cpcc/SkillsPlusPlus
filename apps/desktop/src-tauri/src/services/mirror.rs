//! GitHub 镜像管理。
//!
//! 参考 `docs/中国网络抖动解决方案借鉴.md`：国内直连 GitHub 极慢/不可达，
//! `gh-proxy.com` 是当前可用的镜像（`mirror.ghproxy.com` / `ghproxy.net` 等已失效）。
//!
//! 镜像列表做成运行时可配置：默认走 `GITHUB_MIRRORS`（编译期），
//! 启动时 `init_config(&conn)` 从 DB 读取用户自定义列表覆盖之，
//! 设置变更时 `reload_config(&conn)` 刷新。
//! 这样 `candidate_urls` 的纯函数签名不变，所有调用点（包括 trait 方法
//! `GithubAdapter::fetch`）都能拿到最新配置，无需透传 `&Connection`。

use crate::repositories::settings;
use rusqlite::Connection;
use std::sync::{OnceLock, RwLock};

/// 默认 GitHub 镜像候选（按优先级）。
///
/// 空字符串代表直连。海外用户直连即可，国内网络下直连失败会走后面的镜像。
///
/// 注意：镜像可用性会变化（`mirror.ghproxy.com` 已挂）。
/// 这里只放当前实测可用的 `gh-proxy.com`（新加坡 CDN）。
pub const GITHUB_MIRRORS: &[&str] = &[
    "",                     // 直连（先试一次，海外用户无感）
    "https://gh-proxy.com", // 国内镜像（新加坡 CDN，当前可用）
];

/// DB 里的配置 key。
const KEY_GITHUB_MIRRORS: &str = "mirror.github";
const KEY_MIRROR_ENABLED: &str = "mirror.enabled";

/// 运行时可变的镜像配置。
#[derive(Clone, Debug)]
pub struct MirrorConfig {
    /// 是否启用镜像 fallback。`false` 时所有 URL 直连。
    pub enabled: bool,
    /// GitHub 镜像候选前缀列表（按优先级）。空字符串代表直连。
    pub github_mirrors: Vec<String>,
}

impl Default for MirrorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            github_mirrors: GITHUB_MIRRORS.iter().map(|s| s.to_string()).collect(),
        }
    }
}

/// 全局运行时配置。未初始化时返回 `Default`（测试场景友好）。
static CONFIG: OnceLock<RwLock<MirrorConfig>> = OnceLock::new();

/// 启动时调用：从 DB 读配置并初始化全局状态。
/// 如果 DB 中无配置或读取失败，使用编译期默认。
pub fn init_config(conn: &Connection) {
    let cfg = load_from_db(conn);
    // 已初始化则等效于 reload；首次初始化走 set。
    if let Some(lock) = CONFIG.get() {
        if let Ok(mut guard) = lock.write() {
            *guard = cfg;
        }
        return;
    }
    let _ = CONFIG.set(RwLock::new(cfg));
}

/// 设置变更后调用：从 DB 重新读配置刷新全局状态。
/// 若 `init_config` 未执行过，这里会做初始化。
pub fn reload_config(conn: &Connection) {
    init_config(conn);
}

/// 读取当前生效的配置（拷贝）。未 `init_config` 时返回 `Default`。
pub fn current_config() -> MirrorConfig {
    match CONFIG.get() {
        Some(lock) => lock
            .read()
            .map(|g| g.clone())
            .unwrap_or_else(|_| MirrorConfig::default()),
        None => MirrorConfig::default(),
    }
}

/// 直接覆盖全局配置（不写 DB），主要给测试用。
pub fn set_config_for_test(cfg: MirrorConfig) {
    if let Some(lock) = CONFIG.get() {
        if let Ok(mut guard) = lock.write() {
            *guard = cfg;
            return;
        }
    }
    let _ = CONFIG.set(RwLock::new(cfg));
}

/// 从 DB 读配置。任何字段缺失或解析失败都回退到编译期默认。
pub fn load_from_db(conn: &Connection) -> MirrorConfig {
    let enabled = settings::get_str(conn, KEY_MIRROR_ENABLED)
        .ok()
        .flatten()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(true); // DB 无配置 → 默认启用

    let github_mirrors: Vec<String> = settings::get_json::<Vec<String>>(conn, KEY_GITHUB_MIRRORS)
        .ok()
        .flatten()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| GITHUB_MIRRORS.iter().map(|s| s.to_string()).collect());

    MirrorConfig {
        enabled,
        github_mirrors,
    }
}

/// 把配置写回 DB（commands/settings.rs 调用）。
pub fn save_to_db(conn: &Connection, cfg: &MirrorConfig) -> Result<(), rusqlite::Error> {
    settings::set_str(
        conn,
        KEY_MIRROR_ENABLED,
        if cfg.enabled { "1" } else { "0" },
    )?;
    settings::set_json(conn, KEY_GITHUB_MIRRORS, &cfg.github_mirrors)?;
    Ok(())
}

/// 判断 URL 是否属于 GitHub 资源（需要走镜像）。
pub fn is_github_url(url: &str) -> bool {
    url.contains("github.com")
        || url.contains("codeload.github.com")
        || url.contains("raw.githubusercontent.com")
        || url.contains("release-assets.githubusercontent.com")
        || url.contains("api.github.com")
}

/// 把 GitHub 原始 URL 包装成镜像 URL。
/// `mirror_prefix` 为空时返回原 URL。
///
/// 用法：`wrap("https://gh-proxy.com", "https://github.com/owner/repo")`
///       → `https://gh-proxy.com/https://github.com/owner/repo`
pub fn wrap(mirror_prefix: &str, original_url: &str) -> String {
    if mirror_prefix.is_empty() {
        original_url.to_string()
    } else {
        format!("{}/{}", mirror_prefix.trim_end_matches('/'), original_url)
    }
}

/// 生成候选 URL 列表：直连 + 各镜像。
/// 非 GitHub URL 只返回原 URL（不镜像）。
/// 镜像禁用时也只返回原 URL。
pub fn candidate_urls(original_url: &str) -> Vec<String> {
    if !is_github_url(original_url) {
        return vec![original_url.to_string()];
    }
    let cfg = current_config();
    if !cfg.enabled {
        return vec![original_url.to_string()];
    }
    cfg.github_mirrors
        .iter()
        .map(|m| wrap(m, original_url))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE app_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
        )
        .unwrap();
        conn
    }

    #[test]
    fn detects_github_urls() {
        assert!(is_github_url("https://github.com/owner/repo"));
        assert!(is_github_url("https://codeload.github.com/owner/repo/tar.gz/main"));
        assert!(is_github_url("https://raw.githubusercontent.com/owner/repo/main/SKILL.md"));
        assert!(is_github_url("https://api.github.com/search/repositories?q=topic:foo"));
        assert!(is_github_url("https://release-assets.githubusercontent.com/123"));
        assert!(!is_github_url("https://huggingface.co/datasets/foo/bar"));
        assert!(!is_github_url("https://skills.sh/api/search"));
        assert!(!is_github_url("https://hf-mirror.com/foo"));
    }

    #[test]
    fn wrap_empty_prefix_returns_original() {
        assert_eq!(wrap("", "https://github.com/a/b"), "https://github.com/a/b");
    }

    #[test]
    fn wrap_adds_prefix() {
        assert_eq!(
            wrap("https://gh-proxy.com", "https://github.com/a/b"),
            "https://gh-proxy.com/https://github.com/a/b"
        );
        // trim trailing slash
        assert_eq!(
            wrap("https://gh-proxy.com/", "https://github.com/a/b"),
            "https://gh-proxy.com/https://github.com/a/b"
        );
    }

    #[test]
    fn candidate_urls_direct_then_mirrors_for_github() {
        // 测试场景：未 init_config，使用编译期默认。
        set_config_for_test(MirrorConfig::default());
        let urls = candidate_urls("https://github.com/owner/repo");
        assert_eq!(urls.len(), GITHUB_MIRRORS.len());
        assert_eq!(urls[0], "https://github.com/owner/repo");
        assert!(urls.iter().any(|u| u.starts_with("https://gh-proxy.com/")));
    }

    #[test]
    fn candidate_urls_single_for_non_github() {
        set_config_for_test(MirrorConfig::default());
        let urls = candidate_urls("https://huggingface.co/datasets/foo/bar");
        assert_eq!(urls, vec!["https://huggingface.co/datasets/foo/bar"]);
    }

    #[test]
    fn candidate_urls_preserves_codeload_and_raw() {
        set_config_for_test(MirrorConfig::default());
        let urls = candidate_urls("https://codeload.github.com/owner/repo/tar.gz/refs/heads/main");
        assert!(urls[0].contains("codeload.github.com"));
        assert!(urls[1].contains("gh-proxy.com"));
        assert!(urls[1].contains("codeload.github.com"));

        let urls = candidate_urls("https://raw.githubusercontent.com/owner/repo/refs/heads/main/SKILL.md");
        assert!(urls[0].contains("raw.githubusercontent.com"));
        assert!(urls[1].contains("gh-proxy.com"));
        assert!(urls[1].contains("raw.githubusercontent.com"));
    }

    #[test]
    fn candidate_urls_disabled_returns_only_original() {
        set_config_for_test(MirrorConfig {
            enabled: false,
            github_mirrors: vec!["".to_string(), "https://gh-proxy.com".to_string()],
        });
        let urls = candidate_urls("https://github.com/owner/repo");
        assert_eq!(urls, vec!["https://github.com/owner/repo"]);
    }

    #[test]
    fn candidate_urls_uses_custom_mirrors() {
        set_config_for_test(MirrorConfig {
            enabled: true,
            github_mirrors: vec![
                "https://my-mirror.example.com".to_string(),
                "".to_string(),
            ],
        });
        let urls = candidate_urls("https://github.com/owner/repo");
        assert_eq!(urls.len(), 2);
        assert_eq!(urls[0], "https://my-mirror.example.com/https://github.com/owner/repo");
        assert_eq!(urls[1], "https://github.com/owner/repo");
    }

    #[test]
    fn save_to_db_then_load_from_db_roundtrips() {
        let conn = open_test_db();
        let cfg = MirrorConfig {
            enabled: false,
            github_mirrors: vec![
                "https://a.example.com".to_string(),
                "".to_string(),
                "https://b.example.com".to_string(),
            ],
        };
        save_to_db(&conn, &cfg).unwrap();
        let loaded = load_from_db(&conn);
        assert_eq!(loaded.enabled, false);
        assert_eq!(loaded.github_mirrors, cfg.github_mirrors);
    }

    #[test]
    fn load_from_db_uses_defaults_when_empty() {
        let conn = open_test_db();
        let loaded = load_from_db(&conn);
        assert_eq!(loaded.enabled, true);
        assert_eq!(
            loaded.github_mirrors,
            GITHUB_MIRRORS.iter().map(|s| s.to_string()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn load_from_db_handles_partial_config() {
        let conn = open_test_db();
        // 只设了 enabled，没设 mirrors
        settings::set_str(&conn, KEY_MIRROR_ENABLED, "0").unwrap();
        let loaded = load_from_db(&conn);
        assert_eq!(loaded.enabled, false);
        // mirrors 用默认
        assert!(!loaded.github_mirrors.is_empty());
    }

    #[test]
    fn init_then_reload_refreshes_global_state() {
        let conn = open_test_db();
        // 初始：DB 空 → 默认
        init_config(&conn);
        assert!(current_config().enabled);
        assert!(current_config().github_mirrors.iter().any(|m| m == "https://gh-proxy.com"));

        // 写入禁用配置 → reload → 全局生效
        settings::set_str(&conn, KEY_MIRROR_ENABLED, "0").unwrap();
        reload_config(&conn);
        assert!(!current_config().enabled);

        // 清理：恢复默认，避免污染同进程其它测试
        set_config_for_test(MirrorConfig::default());
    }
}
