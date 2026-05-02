//! NCL task: DAG 任务编排（PCL2 LoaderCombo 风格）。
//!
//! Sprint 1 起填充：`Task` trait（id / deps / run / weight）+ `TaskGraph`
//! （tokio JoinSet + Semaphore 控并发，进度聚合）。
