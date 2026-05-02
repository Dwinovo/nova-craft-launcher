//! Tauri 端的 ProgressSink 实现：把业务层 emit 的事件转发到 webview。

use async_trait::async_trait;
use ncl_core::progress::{ProgressEvent, ProgressSink};
use tauri::{AppHandle, Emitter};

pub struct TauriEventSink {
    app: AppHandle,
}

impl TauriEventSink {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

#[async_trait]
impl ProgressSink for TauriEventSink {
    async fn emit(&self, event: ProgressEvent) {
        if let Err(e) = self.app.emit("ncl://progress", &event) {
            tracing::warn!(?e, "failed to emit ncl://progress");
        }
    }
}
