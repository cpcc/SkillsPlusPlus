//! WebDAV HTTP 客户端。
//!
//! 基于 `reqwest` 实现 WebDAV 的 PUT / GET / DELETE / MKCOL / PROPFIND 操作。
//! 用于跨设备同步时上传/下载 `skillspp-sync.json`。
//!
//! ## 远端结构
//! ```text
//! {remotePath}/
//!   skillspp-sync.json    ← 合并后的同步快照
//! ```
//!
//! 参见 `docs/plans/cross-device-sync-2026-07-20.md` Phase 2。

use reqwest::{Client, Method};
use std::time::Duration;

/// WebDAV 客户端配置。
#[derive(Debug, Clone)]
pub struct WebDavConfig {
    /// WebDAV 服务器基础 URL（如 `https://dav.example.com`）。
    pub url: String,
    /// 用户名。
    pub username: String,
    /// 密码（明文，传输走 HTTPS Basic Auth）。
    pub password: String,
    /// 远端存储路径（如 `/skillspp`）。
    pub remote_path: String,
}

/// WebDAV 客户端。
pub struct WebDavClient {
    client: Client,
    base_url: String,
    /// Basic Auth header value (`Basic <base64>`)。
    auth_header: String,
}

impl WebDavClient {
    /// 创建客户端。验证 URL 格式并构建 HTTP client。
    pub fn new(config: &WebDavConfig) -> Result<Self, String> {
        let url = config.url.trim_end_matches('/').to_string();
        if url.is_empty() {
            return Err("WebDAV URL 不能为空".to_string());
        }
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err("WebDAV URL 必须以 http:// 或 https:// 开头".to_string());
        }

        // 安全提醒：HTTP 明文传输密码不安全，建议使用 HTTPS。
        if url.starts_with("http://") {
            log::warn!("webdav: 使用 HTTP（非加密）连接，密码将以明文传输");
        }

        let auth_header = format!(
            "Basic {}",
            base64::encode(format!("{}:{}", config.username, config.password))
        );

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

        Ok(Self {
            client,
            base_url: url,
            auth_header,
        })
    }

    /// 构建远端文件的完整 URL。
    /// `path` 是相对于 `remote_path` 的子路径，如 `skillspp-sync.json`。
    fn full_url(&self, remote_path: &str, path: &str) -> String {
        let base = self.base_url.trim_end_matches('/');
        let dir = remote_path.trim_start_matches('/').trim_end_matches('/');
        let file = path.trim_start_matches('/');
        format!("{base}/{dir}/{file}")
    }

    /// 构建远端目录的完整 URL。
    fn dir_url(&self, remote_path: &str) -> String {
        let base = self.base_url.trim_end_matches('/');
        let dir = remote_path.trim_start_matches('/').trim_end_matches('/');
        format!("{base}/{dir}")
    }

    /// 测试连接：对基础 URL 发 PROPFIND 请求。
    /// 成功返回 `()`，失败返回错误信息。
    pub async fn test_connection(&self, remote_path: &str) -> Result<(), String> {
        let url = self.dir_url(remote_path);
        let resp = self
            .client
            .request(Method::from_bytes(b"PROPFIND").unwrap(), &url)
            .header("Authorization", &self.auth_header)
            .header("Depth", "0")
            .header("Content-Type", "application/xml")
            .body(r#"<?xml version="1.0" encoding="utf-8"?><propfind xmlns="DAV:"><prop><resourcetype/></prop></propfind>"#)
            .send()
            .await
            .map_err(|e| format!("连接失败: {e}"))?;

        let status = resp.status().as_u16();
        match status {
            207 => Ok(()), // Multi-Status → 目录存在
            404 => Ok(()), // 目录不存在也算连接成功（稍后会自动创建）
            401 => Err("用户名或密码错误".to_string()),
            403 => Err("没有权限访问该路径".to_string()),
            s => Err(format!("服务器返回异常状态码: {s}")),
        }
    }

    /// 创建远端目录（MKCOL）。如果目录已存在则忽略。
    pub async fn mkcol(&self, remote_path: &str) -> Result<(), String> {
        let url = self.dir_url(remote_path);
        let resp = self
            .client
            .request(Method::from_bytes(b"MKCOL").unwrap(), &url)
            .header("Authorization", &self.auth_header)
            .send()
            .await
            .map_err(|e| format!("创建目录失败: {e}"))?;

        let status = resp.status().as_u16();
        match status {
            201 | 405 => Ok(()), // 201 Created 或 405 Method Not Allowed（已存在）
            401 => Err("用户名或密码错误".to_string()),
            s => Err(format!("创建目录失败，状态码: {s}")),
        }
    }

    /// 上传文件（PUT）。
    pub async fn upload(&self, remote_path: &str, filename: &str, content: &str) -> Result<(), String> {
        // 先确保目录存在
        self.mkcol(remote_path).await?;

        let url = self.full_url(remote_path, filename);
        let resp = self
            .client
            .put(&url)
            .header("Authorization", &self.auth_header)
            .header("Content-Type", "application/json; charset=utf-8")
            .body(content.to_string())
            .send()
            .await
            .map_err(|e| format!("上传失败: {e}"))?;

        let status = resp.status().as_u16();
        match status {
            200 | 201 | 204 => Ok(()),
            401 => Err("用户名或密码错误".to_string()),
            409 => Err("父目录不存在".to_string()),
            s => Err(format!("上传失败，状态码: {s}")),
        }
    }

    /// 下载文件（GET）。返回 `None` 表示文件不存在（404）。
    pub async fn download(&self, remote_path: &str, filename: &str) -> Result<Option<String>, String> {
        let url = self.full_url(remote_path, filename);
        let resp = self
            .client
            .get(&url)
            .header("Authorization", &self.auth_header)
            .send()
            .await
            .map_err(|e| format!("下载失败: {e}"))?;

        let status = resp.status().as_u16();
        match status {
            200 => {
                let text = resp
                    .text()
                    .await
                    .map_err(|e| format!("读取响应体失败: {e}"))?;
                Ok(Some(text))
            }
            404 => Ok(None),
            401 => Err("用户名或密码错误".to_string()),
            s => Err(format!("下载失败，状态码: {s}")),
        }
    }

    /// 删除文件（DELETE）。文件不存在时返回 Ok。
    pub async fn delete(&self, remote_path: &str, filename: &str) -> Result<(), String> {
        let url = self.full_url(remote_path, filename);
        let resp = self
            .client
            .delete(&url)
            .header("Authorization", &self.auth_header)
            .send()
            .await
            .map_err(|e| format!("删除失败: {e}"))?;

        let status = resp.status().as_u16();
        match status {
            200 | 204 | 404 => Ok(()),
            401 => Err("用户名或密码错误".to_string()),
            s => Err(format!("删除失败，状态码: {s}")),
        }
    }
}

/// base64 编码（不依赖外部 crate，reqwest 已间接引入 base64）。
mod base64 {
    pub fn encode(input: String) -> String {
        const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let bytes = input.as_bytes();
        let mut result = String::with_capacity((bytes.len() + 2) / 3 * 4);
        for chunk in bytes.chunks(3) {
            let b0 = chunk[0] as u32;
            let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
            let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
            let n = (b0 << 16) | (b1 << 8) | b2;

            result.push(CHARS[((n >> 18) & 63) as usize] as char);
            result.push(CHARS[((n >> 12) & 63) as usize] as char);
            if chunk.len() > 1 {
                result.push(CHARS[((n >> 6) & 63) as usize] as char);
            } else {
                result.push('=');
            }
            if chunk.len() > 2 {
                result.push(CHARS[(n & 63) as usize] as char);
            } else {
                result.push('=');
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_encode_basic() {
        assert_eq!(base64::encode("user:pass".to_string()), "dXNlcjpwYXNz");
        assert_eq!(base64::encode("a".to_string()), "YQ==");
        assert_eq!(base64::encode("ab".to_string()), "YWI=");
        assert_eq!(base64::encode("abc".to_string()), "YWJj");
    }

    #[test]
    fn webdav_config_validation() {
        let cfg = WebDavConfig {
            url: "".to_string(),
            username: "u".to_string(),
            password: "p".to_string(),
            remote_path: "/skillspp".to_string(),
        };
        assert!(WebDavClient::new(&cfg).is_err());

        let cfg = WebDavConfig {
            url: "ftp://example.com".to_string(),
            username: "u".to_string(),
            password: "p".to_string(),
            remote_path: "/skillspp".to_string(),
        };
        assert!(WebDavClient::new(&cfg).is_err());
    }

    #[test]
    fn webdav_client_creation() {
        let cfg = WebDavConfig {
            url: "https://dav.example.com".to_string(),
            username: "user".to_string(),
            password: "pass".to_string(),
            remote_path: "/skillspp".to_string(),
        };
        let client = WebDavClient::new(&cfg).unwrap();
        assert_eq!(client.base_url, "https://dav.example.com");
    }

    #[test]
    fn full_url_construction() {
        let cfg = WebDavConfig {
            url: "https://dav.example.com/".to_string(),
            username: "u".to_string(),
            password: "p".to_string(),
            remote_path: "/skillspp/".to_string(),
        };
        let client = WebDavClient::new(&cfg).unwrap();
        assert_eq!(
            client.full_url("/skillspp/", "skillspp-sync.json"),
            "https://dav.example.com/skillspp/skillspp-sync.json"
        );
        assert_eq!(
            client.dir_url("/skillspp/"),
            "https://dav.example.com/skillspp"
        );
    }
}
