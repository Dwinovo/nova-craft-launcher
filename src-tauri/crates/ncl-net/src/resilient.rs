//! 韧性下载器：在 [`Downloader`] 上叠加 [`MirrorPool`]、智能 HTTP 状态码处理、
//! 自动重试、指数退避。
//!
//! 状态码处理（参考 HMCL `FetchTask` + 我们的 429 改进）：
//! - **4xx 除 429**：[`Error::HttpClient`] → 不扣镜像分（不是镜像问题），
//!   立即跳到下一镜像（同镜像重试无意义）
//! - **429 限流**：[`Error::HttpRateLimited`] → 读 `Retry-After` 头本地等待
//!   ≤30s，**同镜像再试一次**；仍失败则换镜像（HMCL 没做这个，我们做）
//! - **5xx / 网络错误**：[`Error::Network`] → 扣镜像分 -7 + 指数退避后换镜像

use crate::download::Downloader;
use crate::mirror::MirrorPool;
use ncl_core::{Error, Result};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

const RATE_LIMIT_MAX_WAIT: u64 = 30; // 429 时本地等待上限,避免 retry-after 过长卡死

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
        let total_attempts = self.max_retries.min(candidates.len());

        for (attempt, (mirror_url, source_idx)) in candidates.iter().enumerate() {
            if attempt >= self.max_retries {
                break;
            }
            let source_name = self.pool.name_of(*source_idx).unwrap_or("?");
            tracing::debug!(
                source = source_name,
                url = %mirror_url,
                attempt = attempt + 1,
                "ResilientDownloader: trying"
            );

            match self.inner.fetch(mirror_url, target, expected_sha1).await {
                Ok(()) => {
                    self.pool.record(*source_idx, true);
                    return Ok(());
                }
                Err(e) => match &e {
                    // 4xx (除 429):该镜像没这个文件,不扣分,直接换下一镜像
                    Error::HttpClient { status, .. } => {
                        tracing::debug!(
                            source = source_name,
                            url = %mirror_url,
                            status,
                            "client error; switching mirror without scoring"
                        );
                        last_err = Some(e);
                        // 不 sleep,立即下一轮
                    }
                    // 429:本地等待 retry-after 后**同镜像重试一次**,仍失败再换
                    Error::HttpRateLimited {
                        retry_after_secs, ..
                    } => {
                        let wait = retry_after_secs.unwrap_or(5).min(RATE_LIMIT_MAX_WAIT);
                        tracing::warn!(
                            source = source_name,
                            url = %mirror_url,
                            wait_secs = wait,
                            "rate limited; waiting then retrying same mirror once"
                        );
                        tokio::time::sleep(Duration::from_secs(wait)).await;
                        match self.inner.fetch(mirror_url, target, expected_sha1).await {
                            Ok(()) => {
                                // 单次重试成功:不算成功上分(避免限流期反向激励)
                                return Ok(());
                            }
                            Err(retry_err) => {
                                self.pool.record(*source_idx, false);
                                last_err = Some(retry_err);
                            }
                        }
                    }
                    // 5xx / 网络错误:扣分 + 指数退避换镜像
                    _ => {
                        tracing::warn!(
                            source = source_name,
                            url = %mirror_url,
                            %e,
                            "mirror failed (server/network); scoring -7"
                        );
                        self.pool.record(*source_idx, false);
                        last_err = Some(e);
                        if attempt + 1 < total_attempts {
                            let delay_ms = 200u64.saturating_mul(1 << attempt as u32);
                            tokio::time::sleep(Duration::from_millis(delay_ms.min(4000))).await;
                        }
                    }
                },
            }
        }
        Err(last_err.unwrap_or_else(|| Error::Network("all mirrors failed".into())))
    }
}
