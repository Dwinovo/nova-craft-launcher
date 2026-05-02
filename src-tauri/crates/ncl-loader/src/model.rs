use serde::{Deserialize, Serialize};

/// 三大 mod 加载器枚举。Forge 与 NeoForge 在 1.20.1 之后是分支关系，
/// 但对 NCL 用户而言它们是平行的选项。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LoaderKind {
    Forge,
    Fabric,
    NeoForge,
}

/// 一次 loader 版本查询返回的条目。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoaderVersion {
    /// loader 版本号（例：Fabric 0.16.0、Forge 47.3.0）
    pub version: String,
    /// 是否稳定发布（Fabric 区分 stable / beta）
    pub stable: bool,
}

/// 安装一个 loader 后产生的描述信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoaderInstallResult {
    /// 最终的合并 version_id（即 InstallPaths 与 launch 用的 id）
    pub merged_version_id: String,
    /// 主类（loader 通常会重写）
    pub main_class: String,
}
