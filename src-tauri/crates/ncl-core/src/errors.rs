use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("config error: {0}")]
    Config(String),

    #[error("path layout: {0}")]
    PathLayout(String),

    #[error("network: {0}")]
    Network(String),

    /// HTTP 4xx 客户端错误（除 429）。表示请求本身有问题（URL 不存在 / 权限不足），
    /// 同镜像重试**无意义**——上层应直接换下一个镜像。
    #[error("HTTP {status} at {url}")]
    HttpClient { url: String, status: u16 },

    /// HTTP 429 限流。`retry_after_secs` 来自服务器 `Retry-After` 头（如有）。
    /// 上层应在该镜像本地等待 `retry_after_secs`（或固定值兜底）后**再试同镜像一次**。
    #[error("HTTP 429 rate limited at {url} (retry after {retry_after_secs:?}s)")]
    HttpRateLimited {
        url: String,
        retry_after_secs: Option<u64>,
    },

    #[error("checksum mismatch (expected={expected}, actual={actual})")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("task: {0}")]
    Task(String),

    #[error("not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Error::Io {
            path: path.into(),
            source,
        }
    }
}
