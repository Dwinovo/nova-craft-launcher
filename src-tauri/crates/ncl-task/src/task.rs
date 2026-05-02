use async_trait::async_trait;
use ncl_core::Result;
use std::sync::Arc;

/// 流水线中的最小 unit of work。
///
/// `weight` 用于进度聚合（默认 1）；可以是字节数、文件数、或抽象单位，
/// 只要在同一个 pipeline 内统一即可。
#[async_trait]
pub trait Task: Send + Sync {
    /// 人类可读标签（出现在日志/事件中）
    fn label(&self) -> &str;

    /// 此任务对总进度的贡献量
    fn weight(&self) -> u64 {
        1
    }

    /// 执行任务。失败会终止整条流水线。
    async fn run(&self) -> Result<()>;
}

/// 装箱后的 Task，便于异构集合存储。
pub type BoxTask = Arc<dyn Task>;
