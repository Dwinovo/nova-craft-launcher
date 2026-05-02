use serde::{Deserialize, Serialize};

// LoaderKind 移动到 ncl-core::model，本 crate 仅 re-export。
pub use ncl_core::model::LoaderKind;

/// 一次 loader 版本查询返回的条目。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoaderVersion {
    pub version: String,
    pub stable: bool,
}

/// 安装一个 loader 后产生的描述信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoaderInstallResult {
    pub merged_version_id: String,
    pub main_class: String,
}
