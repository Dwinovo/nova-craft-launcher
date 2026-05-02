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
        if !resp.status().is_success() {
            return Err(Error::Network(format!(
                "GET {url}: HTTP {}",
                resp.status()
            )));
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
