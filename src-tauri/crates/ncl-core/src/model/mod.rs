//! 业务数据模型。
//!
//! Sprint 1: vanilla 启动需要的核心结构（version manifest / library / asset /
//! arguments / java requirement）。后续 sprint 添加 mod / loader / launch 配置等。

pub mod arguments;
pub mod assets;
pub mod java;
pub mod library;
pub mod mod_entry;
pub mod version;

pub use arguments::{
    rules_allow, Argument, ArgumentValue, Arguments, EvalContext, OsCondition, OsName, Rule,
    RuleAction,
};
pub use assets::{AssetIndex, AssetIndexInfo, AssetObject};
pub use java::JavaVersionRequirement;
pub use library::{maven_ga, ArtifactInfo, ExtractRules, Library, LibraryDownloads};
pub use mod_entry::{LoaderKind, ModDep, ModEntry, Side};
pub use version::{
    DownloadInfo, LatestVersions, MainDownloads, RawVersion, ResolvedManifest, VersionList,
    VersionListEntry, VersionType,
};
