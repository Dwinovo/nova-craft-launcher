use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// 业务层向外汇报的进度/日志事件。前端用 serde 反序列化，
/// CLI 直接打印 Display。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProgressEvent {
    TaskStarted {
        task_id: String,
        label: String,
        total_weight: u64,
    },
    TaskProgress {
        task_id: String,
        completed: u64,
        total: u64,
        message: Option<String>,
    },
    TaskFinished {
        task_id: String,
        success: bool,
        error: Option<String>,
    },
    Log {
        source: String,
        level: LogLevel,
        message: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// 进度事件汇报器。CLI / Tauri / 测试各自实现一份。
#[async_trait]
pub trait ProgressSink: Send + Sync + 'static {
    async fn emit(&self, event: ProgressEvent);
}

/// 基于 tokio unbounded mpsc 的实现。Tauri 端把 receiver 接到 `app.emit()`，
/// CLI 端把 receiver 接到 stdout。
pub struct ChannelSink {
    sender: mpsc::UnboundedSender<ProgressEvent>,
}

impl ChannelSink {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<ProgressEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Self { sender: tx }, rx)
    }
}

#[async_trait]
impl ProgressSink for ChannelSink {
    async fn emit(&self, event: ProgressEvent) {
        if self.sender.send(event).is_err() {
            tracing::debug!("ProgressSink dropped: receiver closed");
        }
    }
}

/// 静默 sink，测试或不关心进度时使用。
pub struct NullSink;

#[async_trait]
impl ProgressSink for NullSink {
    async fn emit(&self, _event: ProgressEvent) {}
}
