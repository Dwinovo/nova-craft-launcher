//! Forge / NeoForge install_profile 中的 library 定义。
//!
//! 与 Mojang vanilla version.json 的 [`Library`](ncl_core::Library) 结构一致，
//! 但不带 rules。我们用一个独立类型避免引入 vanilla 的 rules 字段产生 schema 冲突。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProfileLibrary {
    pub name: String,
    #[serde(default)]
    pub downloads: Option<ProfileLibraryDownloads>,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProfileLibraryDownloads {
    #[serde(default)]
    pub artifact: Option<ncl_core::ArtifactInfo>,
}
