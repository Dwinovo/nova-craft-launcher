use crate::checksum;
use ncl_core::errors::{Error, Result};
use reqwest::Client;
use std::fs;
use std::path::Path;
use std::time::Duration;
use tokio::io::AsyncWriteExt;

/// 单文件 HTTP 下载器。Sprint 2 起会被 `MirrorPool` 包装，加上重试与镜像降级。
pub struct Downloader {
    client: Client,
}

impl Downloader {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .user_agent("Nova-Craft-Launcher/0.1.0")
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| Error::Network(e.to_string()))?;
        Ok(Self { client })
    }

    /// 下载 `url` 到 `target`。
    /// - 若 `expected_sha1` 提供且本地文件已存在并校验通过 → 跳过下载。
    /// - 下载完成后若 `expected_sha1` 提供，验证 SHA1。
    ///
    /// HTTP 状态码分类（参考 HMCL `FetchTask` 的策略 + 我们的 429 增强）：
    /// - 2xx → 正常处理
    /// - 4xx 除 429 → 抛 [`Error::HttpClient`],上层应跨镜像
    /// - 429 → 抛 [`Error::HttpRateLimited`] + Retry-After 秒数,上层可选择等待重试
    /// - 5xx / 网络错误 → 抛 [`Error::Network`],上层走重试 + 退避
    pub async fn fetch(
        &self,
        url: &str,
        target: &Path,
        expected_sha1: Option<&str>,
    ) -> Result<()> {
        if let Some(sha) = expected_sha1 {
            if target.exists() && checksum::verify_sha1(target, sha).is_ok() {
                tracing::debug!(target = %target.display(), "sha1 already matches, skipping download");
                return Ok(());
            }
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }

        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| Error::Network(format!("GET {url}: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            let code = status.as_u16();
            // 429 限流:读 Retry-After 头(秒)
            if code == 429 {
                let retry_after_secs = resp
                    .headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.trim().parse::<u64>().ok());
                return Err(Error::HttpRateLimited {
                    url: url.to_string(),
                    retry_after_secs,
                });
            }
            // 4xx (除 429): 客户端错误,跨镜像可能成功
            if status.is_client_error() {
                return Err(Error::HttpClient {
                    url: url.to_string(),
                    status: code,
                });
            }
            // 5xx / 其他: 服务端错误或异常,走 Network 重试
            return Err(Error::Network(format!("GET {url}: HTTP {code}")));
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| Error::Network(format!("read body {url}: {e}")))?;

        let mut file = tokio::fs::File::create(target)
            .await
            .map_err(|e| Error::io(target, e))?;
        file.write_all(&bytes)
            .await
            .map_err(|e| Error::io(target, e))?;
        file.flush().await.map_err(|e| Error::io(target, e))?;
        drop(file);

        if let Some(sha) = expected_sha1 {
            checksum::verify_sha1(target, sha)?;
        }
        Ok(())
    }
}
