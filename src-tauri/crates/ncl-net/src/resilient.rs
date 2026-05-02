//! 韧性下载器：在 [`Downloader`] 上叠加 [`MirrorPool`]、自动重试、指数退避。

use crate::download::Downloader;
use crate::mirror::MirrorPool;
use ncl_core::{Error, Result};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

/// 包装 `Downloader` 的韧性下载器。每次 `fetch`：
/// 1. 询问 `MirrorPool` 拿到按 score 倒序排序的候选 URL
/// 2. 顺序尝试，单次失败 → 指数退避后切下一镜像
/// 3. 任何镜像成功立即返回；上报成败让 pool 调整分数
/// 4. 最多尝试 `max_retries` 次（含跨镜像）
pub struct ResilientDownloader {
    inner: Downloader,
    pool: Arc<MirrorPool>,
    max_retries: usize,
}

impl ResilientDownloader {
    pub fn new(downloader: Downloader, pool: Arc<MirrorPool>, max_retries: usize) -> Self {
        Self {
            inner: downloader,
            pool,
            max_retries: max_retries.max(1),
        }
    }

    /// 下载 `original_url` 到 `target`。SHA1 命中时跳过。
    pub async fn fetch(
        &self,
        original_url: &str,
        target: &Path,
        expected_sha1: Option<&str>,
    ) -> Result<()> {
        let candidates = self.pool.candidates(original_url);
        if candidates.is_empty() {
            return Err(Error::Network(format!(
                "no mirror can serve {original_url}"
            )));
        }

        let mut last_err: Option<Error> = None;
        for (attempt, (mirror_url, source_idx)) in candidates.iter().enumerate() {
            if attempt >= self.max_retries {
                break;
            }
            tracing::debug!(
                source = self.pool.name_of(*source_idx).unwrap_or("?"),
                url = %mirror_url,
                attempt = attempt + 1,
                "ResilientDownloader: trying"
            );
            match self
                .inner
                .fetch(mirror_url, target, expected_sha1)
                .await
            {
                Ok(()) => {
                    self.pool.record(*source_idx, true);
                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!(
                        source = self.pool.name_of(*source_idx).unwrap_or("?"),
                        url = %mirror_url,
                        %e,
                        "mirror failed"
                    );
                    self.pool.record(*source_idx, false);
                    last_err = Some(e);
                    if attempt + 1 < candidates.len() && attempt + 1 < self.max_retries {
                        // 指数退避：200ms / 400ms / 800ms / ...，最大 4s
                        let delay_ms = 200u64.saturating_mul(1 << attempt as u32);
                        tokio::time::sleep(Duration::from_millis(delay_ms.min(4000))).await;
                    }
                }
            }
        }
        Err(last_err.unwrap_or_else(|| Error::Network("all mirrors failed".into())))
    }
}
