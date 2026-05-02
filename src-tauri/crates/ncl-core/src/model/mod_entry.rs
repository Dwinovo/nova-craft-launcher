use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Mod 加载器枚举（同时被 `ncl-loader` 复用）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LoaderKind {
    Forge,
    Fabric,
    NeoForge,
}

/// Mod 运行端。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Client,
    Server,
    #[default]
    Both,
}

/// 单个已识别的 Mod 条目。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModEntry {
    pub file_path: PathBuf,
    pub enabled: bool,
    /// 解析时识别到的目标 loader；当 jar 同时包含多种 manifest 时取首个识别到的。
    pub loader: LoaderKind,
    /// mod 标识（modid / id）
    pub mod_id: String,
    pub name: String,
    pub version: String,
    /// Mod 对 MC 版本的兼容范围（loader 各自语义；先存原始字符串，Sprint 4 后续做语义比对）
    pub mc_version_range: Option<String>,
    pub dependencies: Vec<ModDep>,
    pub authors: Vec<String>,
    pub description: Option<String>,
    pub side: Side,
}

/// 一个 mod 声明的依赖项。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModDep {
    pub mod_id: String,
    /// 版本范围（loader 各自语义：Forge `[1.0,2.0)`、Fabric `>=1.0 <2.0`）；
    /// 此处保存原始字符串。
    pub version_range: Option<String>,
    pub mandatory: bool,
    #[serde(default)]
    pub side: Side,
}
