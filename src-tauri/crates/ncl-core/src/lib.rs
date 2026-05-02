//! NCL core: 通用类型、错误、路径布局、进度事件契约。
//!
//! 所有业务 crate 的依赖源头。**禁止**依赖 tauri 运行时——
//! UI 与 CLI 通过 `ProgressSink` trait 拿到事件流。

pub mod config;
pub mod errors;
pub mod model;
pub mod paths;
pub mod progress;

pub use config::{AppConfig, MirrorPolicy};
pub use errors::{Error, Result};
pub use paths::{PathLayout, PathMode};
pub use progress::{ChannelSink, LogLevel, NullSink, ProgressEvent, ProgressSink};

// 业务数据模型
pub use model::{
    maven_ga, rules_allow, Argument, ArgumentValue, Arguments, ArtifactInfo, AssetIndex,
    AssetIndexInfo, AssetObject, DownloadInfo, EvalContext, ExtractRules, JavaVersionRequirement,
    LatestVersions, Library, LibraryDownloads, MainDownloads, OsCondition, OsName, RawVersion,
    ResolvedManifest, Rule, RuleAction, VersionList, VersionListEntry, VersionType,
};
