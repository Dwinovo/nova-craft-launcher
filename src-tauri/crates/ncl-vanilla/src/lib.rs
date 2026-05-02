//! NCL vanilla: Mojang 原版 manifest / asset / library 处理 + 一键安装流水线。

pub mod install;
pub mod library;
pub mod manifest;
pub mod paths;

pub use install::install;
pub use library::{library_artifacts, ArtifactKind, LibraryArtifact};
pub use manifest::{
    default_client, fetch_version_detail, fetch_version_list, resolve_inherits,
    VERSION_MANIFEST_URL,
};
pub use paths::InstallPaths;
