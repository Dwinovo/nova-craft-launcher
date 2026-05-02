//! Tauri IPC 命令的薄包装层。
//!
//! 每个命令仅做：参数解析 → 调用业务 crate → 把结果（或 ProgressSink 桥接的事件）
//! 转发给前端。**业务逻辑严禁写在这里**。

pub mod core;
pub mod java;
pub mod launch;
pub mod sink;
pub mod vanilla;
