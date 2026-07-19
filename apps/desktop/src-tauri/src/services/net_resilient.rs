//! 公共网络韧性工具：多 URL + 重试 + curl 兜底。
//!
//! 抽自 `services/adapters/registry.rs` 的成熟模式，推广到所有 GitHub 网络入口点。
//! 参考：`docs/中国网络抖动解决方案借鉴.md`（gh-proxy.com 镜像方案）。
//!
//! 典型用法：
//! ```
//! use crate::services::mirror;
//! use crate::services::net_resilient;
//!
//! let urls = mirror::candidate_urls("https://codeload.github.com/owner/repo/tar.gz/main");
//! let opts = net_resilient::FetchOptions {
//!     urls,
//!     max_attempts: 2,
//!     ..Default::default()
//! };
//! let fetched = net_resilient::fetch_bytes_blocking(&opts)?;
//! ```

use std::time::Duration;

pub const USER_AGENT: &str = "skills-plus-plus/0.1";

/// 默认网络参数（与 registry.rs 对齐）。
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
pub const READ_TIMEOUT: Duration = Duration::from_secs(45);
pub const CURL_MAX_TIME: Duration = Duration::from_secs(90);

/// HTTP header（name, value）。
#[derive(Clone, Debug)]
pub struct Header {
    pub name: String,
    pub value: String,
}

impl Header {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self { name: name.into(), value: value.into() }
    }
}

#[derive(Clone, Debug)]
pub struct FetchOptions {
    /// 候选 URL 列表（直连 + 镜像）。会按顺序尝试，每个 URL 重试 `max_attempts` 次。
    pub urls: Vec<String>,
    /// 每 URL 最大重试次数。
    pub max_attempts: usize,
    pub connect_timeout: Duration,
    pub read_timeout: Duration,
    pub user_agent: String,
    pub etag: Option<String>,
    /// 额外 HTTP headers（如 GitHub API 的 Accept / X-GitHub-Api-Version）。
    pub headers: Vec<Header>,
}

impl Default for FetchOptions {
    fn default() -> Self {
        Self {
            urls: vec![],
            max_attempts: 1,
            connect_timeout: CONNECT_TIMEOUT,
            read_timeout: READ_TIMEOUT,
            user_agent: USER_AGENT.to_string(),
            etag: None,
            headers: vec![],
        }
    }
}

impl FetchOptions {
    /// 单 URL 直连，1 次尝试。
    pub fn single(url: impl Into<String>) -> Self {
        Self {
            urls: vec![url.into()],
            ..Default::default()
        }
    }

    /// 多 URL（直连 + 镜像），每 URL 重试次数。
    pub fn with_mirrors(urls: Vec<String>, max_attempts: usize) -> Self {
        Self {
            urls,
            max_attempts,
            ..Default::default()
        }
    }
}

/// 抓取结果。
#[derive(Debug)]
pub struct FetchedBytes {
    pub bytes: Vec<u8>,
    pub etag: Option<String>,
    pub source_url: String,
}

/// async 版本：reqwest 依次试每个 URL × max_attempts；全失败 → 系统 curl 兜底。
///
/// 返回 `Ok(FetchedBytes)` 表示成功抓到。
/// 返回 `Err` 表示所有候选 URL + curl 兜底都失败。
pub async fn fetch_bytes(opts: &FetchOptions) -> Result<FetchedBytes, String> {
    if opts.urls.is_empty() {
        return Err("no urls to fetch".to_string());
    }

    let client = reqwest::Client::builder()
        .user_agent(&opts.user_agent)
        .connect_timeout(opts.connect_timeout)
        .read_timeout(opts.read_timeout)
        .build()
        .map_err(|e| format!("build reqwest client: {e}"))?;

    let mut last_err = String::new();

    for url in &opts.urls {
        for attempt in 1..=opts.max_attempts {
            let mut req = client.get(url);
            if let Some(etag) = opts.etag.as_deref().filter(|v| !v.is_empty()) {
                req = req.header(reqwest::header::IF_NONE_MATCH, etag);
            }
            for h in &opts.headers {
                req = req.header(&h.name, &h.value);
            }
            match req.send().await {
                Ok(r) if r.status() == reqwest::StatusCode::NOT_MODIFIED => {
                    // 304：调用方应自己处理本地缓存
                    last_err = format!("{} returned 304", url);
                    break;
                }
                Ok(r) if r.status().is_success() => {
                    let response_etag = r
                        .headers()
                        .get(reqwest::header::ETAG)
                        .and_then(|v| v.to_str().ok())
                        .map(str::to_string);
                    let bytes = match r.bytes().await {
                        Ok(b) => b.to_vec(),
                        Err(e) => {
                            last_err = format!(
                                "{} body read failed on attempt {}: {}",
                                url, attempt, e
                            );
                            if attempt < opts.max_attempts {
                                continue;
                            }
                            break;
                        }
                    };
                    return Ok(FetchedBytes {
                        bytes,
                        etag: response_etag,
                        source_url: url.clone(),
                    });
                }
                Ok(r) => {
                    last_err = format!(
                        "{} returned status {} on attempt {}",
                        url,
                        r.status(),
                        attempt
                    );
                    if attempt < opts.max_attempts {
                        continue;
                    }
                    break;
                }
                Err(e) => {
                    last_err = format!("{} fetch failed on attempt {}: {}", url, attempt, e);
                    if attempt < opts.max_attempts {
                        continue;
                    }
                }
            }
        }
    }

    // reqwest 全失败 → 系统 curl 兜底（按同顺序）
    for url in &opts.urls {
        match fetch_bytes_via_curl(url, &opts.user_agent, opts.etag.as_deref()) {
            Ok(Some((bytes, etag))) => {
                return Ok(FetchedBytes {
                    bytes,
                    etag,
                    source_url: url.clone(),
                });
            }
            Ok(None) => {
                log::warn!("net_resilient: curl returned empty for {}", url);
            }
            Err(e) => {
                log::warn!("net_resilient: {}", e);
            }
        }
    }

    Err(if last_err.is_empty() {
        "all fetch attempts failed".to_string()
    } else {
        last_err
    })
}

/// 同步阻塞版本：给非 async 上下文用（install.rs 等）。
pub fn fetch_bytes_blocking(opts: &FetchOptions) -> Result<FetchedBytes, String> {
    let opts = FetchOptions {
        urls: opts.urls.clone(),
        max_attempts: opts.max_attempts,
        connect_timeout: opts.connect_timeout,
        read_timeout: opts.read_timeout,
        user_agent: opts.user_agent.clone(),
        etag: opts.etag.clone(),
        headers: opts.headers.clone(),
    };
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                let _ = tx.send(Err(format!("build tokio runtime: {e}")));
                return;
            }
        };
        let result = rt.block_on(fetch_bytes(&opts));
        let _ = tx.send(result);
    });
    rx.recv().map_err(|e| format!("http thread join: {e}"))?
}

/// async 抓取文本（给 source.rs / github.rs / app.rs 用）。
///
/// 返回 `(text, etag)`。
pub async fn fetch_text(opts: &FetchOptions) -> Result<(String, Option<String>), String> {
    let fetched = fetch_bytes(opts).await?;
    let text = String::from_utf8(fetched.bytes)
        .map_err(|e| format!("{} response not utf-8: {}", fetched.source_url, e))?;
    Ok((text, fetched.etag))
}

/// async 抓取 JSON（给 github.rs / app.rs 用）。
pub async fn fetch_json<T: serde::de::DeserializeOwned>(
    opts: &FetchOptions,
) -> Result<(T, Option<String>), String> {
    let fetched = fetch_bytes(opts).await?;
    let parsed: T = serde_json::from_slice(&fetched.bytes).map_err(|e| {
        format!(
            "{} json parse failed: {}",
            fetched.source_url, e
        )
    })?;
    Ok((parsed, fetched.etag))
}

/// 系统 curl 兜底（与 registry.rs::fetch_registry_via_curl 同模式）。
/// 返回 `Ok(None)` 表示 304（空 body）。
fn fetch_bytes_via_curl(
    url: &str,
    user_agent: &str,
    etag: Option<&str>,
) -> Result<Option<(Vec<u8>, Option<String>)>, String> {
    let mut cmd = std::process::Command::new("curl");
    cmd.args([
        "-L".to_string(),
        "--fail".to_string(),
        "--silent".to_string(),
        "--show-error".to_string(),
        "--connect-timeout".to_string(),
        CONNECT_TIMEOUT.as_secs().to_string(),
        "--max-time".to_string(),
        CURL_MAX_TIME.as_secs().to_string(),
        "--user-agent".to_string(),
        user_agent.to_string(),
        // dump headers to stdout too (to extract etag)
        "-D".to_string(),
        "-".to_string(),
    ]);
    if let Some(etag) = etag.filter(|v| !v.is_empty()) {
        cmd.args(["-H".to_string(), format!("If-None-Match: {}", etag)]);
    }
    cmd.arg(url);

    let output = cmd
        .output()
        .map_err(|e| format!("curl failed for {}: {}", url, e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = stderr.trim();
        return Err(format!(
            "curl failed for {}: {}",
            url,
            if detail.is_empty() {
                "unknown curl error"
            } else {
                detail
            }
        ));
    }

    if output.stdout.is_empty() {
        return Ok(None);
    }

    // 分离 header 和 body：curl -D - 会把响应 header 放在 stdout 前面，用 \r\n\r\n 分隔
    let (headers, body) = split_curl_output(&output.stdout);
    let etag = extract_etag(&headers);
    Ok(Some((body, etag)))
}

/// 分离 curl 输出中的 header 和 body。
/// `curl -D -` 会把响应 header 放在 stdout 前面，header 和 body 之间是 `\r\n\r\n`。
/// HTTP 重定向会有多组 header，取最后一个分隔之后的内容作为 body。
fn split_curl_output(stdout: &[u8]) -> (String, Vec<u8>) {
    let sep = b"\r\n\r\n";
    let mut last_sep_pos = None;
    let mut pos = 0;
    while pos + sep.len() <= stdout.len() {
        if &stdout[pos..pos + sep.len()] == sep {
            last_sep_pos = Some(pos);
        }
        pos += 1;
    }
    match last_sep_pos {
        Some(p) => {
            let headers = String::from_utf8_lossy(&stdout[..p]).to_string();
            let body = stdout[p + sep.len()..].to_vec();
            (headers, body)
        }
        None => (String::new(), stdout.to_vec()),
    }
}

fn extract_etag(headers: &str) -> Option<String> {
    for line in headers.lines() {
        let line = line.trim();
        // HTTP header 大小写不敏感，curl 输出原样保留服务端大小写
        let lower = line.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("etag:") {
            let v = rest.trim().trim_matches('"');
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_options_have_sane_values() {
        let opts = FetchOptions::default();
        assert!(opts.urls.is_empty());
        assert_eq!(opts.max_attempts, 1);
        assert_eq!(opts.connect_timeout, CONNECT_TIMEOUT);
        assert_eq!(opts.read_timeout, READ_TIMEOUT);
        assert_eq!(opts.user_agent, USER_AGENT);
        assert!(opts.etag.is_none());
    }

    #[test]
    fn single_url_constructor() {
        let opts = FetchOptions::single("https://example.com/foo");
        assert_eq!(opts.urls, vec!["https://example.com/foo"]);
        assert_eq!(opts.max_attempts, 1);
    }

    #[test]
    fn with_mirrors_constructor() {
        let opts = FetchOptions::with_mirrors(
            vec!["https://a.com".to_string(), "https://b.com".to_string()],
            3,
        );
        assert_eq!(opts.urls.len(), 2);
        assert_eq!(opts.max_attempts, 3);
    }

    #[test]
    fn fetch_bytes_empty_urls_returns_error() {
        let opts = FetchOptions::default();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(fetch_bytes(&opts));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no urls"));
    }

    #[test]
    fn split_curl_output_separates_headers_and_body() {
        let input = b"HTTP/2 200\r\netag: \"abc123\"\r\ncontent-type: text/plain\r\n\r\nhello world";
        let (headers, body) = split_curl_output(input);
        assert!(headers.contains("etag"));
        assert!(headers.contains("content-type"));
        assert_eq!(body, b"hello world");
    }

    #[test]
    fn split_curl_output_no_headers_returns_all_as_body() {
        let input = b"just body no headers";
        let (headers, body) = split_curl_output(input);
        assert!(headers.is_empty());
        assert_eq!(body, b"just body no headers");
    }

    #[test]
    fn split_curl_output_handles_multiple_header_blocks() {
        // 重定向场景：两组 header + body
        let input = b"HTTP/2 302\r\nlocation: /foo\r\n\r\nHTTP/2 200\r\netag: \"x\"\r\n\r\nbody";
        let (headers, body) = split_curl_output(input);
        assert!(headers.contains("etag"));
        assert!(headers.contains("location"));
        assert_eq!(body, b"body");
    }

    #[test]
    fn extract_etag_case_insensitive() {
        assert_eq!(extract_etag("ETag: \"abc\""), Some("abc".to_string()));
        assert_eq!(extract_etag("etag: \"abc\""), Some("abc".to_string()));
        assert_eq!(extract_etag("etag: abc"), Some("abc".to_string()));
        assert_eq!(extract_etag("content-type: foo"), None);
        assert_eq!(extract_etag(""), None);
    }

    #[test]
    fn extract_etag_from_multiline_headers() {
        let headers = "HTTP/2 200\r\ncontent-type: application/json\r\netag: \"deadbeef\"\r\nx-foo: bar";
        assert_eq!(extract_etag(headers), Some("deadbeef".to_string()));
    }
}
