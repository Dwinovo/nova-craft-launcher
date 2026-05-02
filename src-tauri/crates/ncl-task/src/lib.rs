//! NCL task: 多阶段并行任务流水线（PCL2 LoaderCombo 风格简化版）。
//!
//! 数据模型：
//! - [`Task`] trait — 单个 unit of work，有 weight 用于进度聚合
//! - [`Stage`] — 一组可并行执行的 task；同一 stage 内的 task 互不依赖
//! - [`Pipeline`] — 多个 stage 顺序执行；阶段间串行，阶段内并行
//!
//! 进度事件通过 `ncl-core::ProgressSink` 上抛。Sprint 3 起若 Forge installer
//! 需要复杂依赖关系，再升级为完整 DAG。

pub mod pipeline;
pub mod task;

pub use pipeline::{Pipeline, Stage};
pub use task::{BoxTask, Task};
