//! Forge loader 接入。
//!
//! 通过 BMCLAPI 拉取版本列表 + installer jar；解压 installer 拿到
//! `install_profile.json` 与 `version.json`；下载所有依赖库；最后跑 processors
//! 队列（每个 processor 是一次 `java -cp ... <Main-Class> <args>` 调用）以
//! 生成 patched client.jar 与 SRG 映射。
//!
//! 子模块：
//! - [`api`] BMCLAPI 客户端（list_versions / installer URL）
//! - [`profile`] install_profile.json schema + 占位符 token 解析
//! - [`processor`] 单 processor 执行（java 进程 + maven 路径解析 + Main-Class 提取）
//! - [`install`] 顶层流程

pub mod api;
pub mod install;
pub mod processor;
pub mod profile;

pub use api::{forge_installer_url, list_versions, ForgeBuildEntry};
pub use install::{install_forge, install_from_installer, ForgeInstallOutput};
pub use profile::{InstallProfile, Processor as ProfileProcessor};
