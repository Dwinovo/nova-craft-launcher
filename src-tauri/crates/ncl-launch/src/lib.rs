//! NCL launch: 启动参数构建、离线 UUID、（Sprint 1.6）进程托管 + 日志捕获。
//!
//! Sprint 1.5 范围：账号 + 参数 (build_plan，纯逻辑，可单测)
//! Sprint 1.6 范围：spawn + stdout/stderr → ProgressSink 转发

pub mod account;
pub mod args;
pub mod crash;
pub mod model;
pub mod spawn;

pub use account::{offline_uuid, AccountInfo};
pub use args::{build_classpath, build_plan, dedupe_jvm_args, flatten_args, substitute};
pub use crash::{collect_latest_crash, CrashCategory, CrashReport};
pub use model::{LaunchInputs, LaunchPlan, MemorySpec};
pub use spawn::{spawn, ProcessHandle};
