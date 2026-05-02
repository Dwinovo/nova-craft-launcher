//! NCL java: Java 运行时检测、版本兼容矩阵、内存推荐。
//!
//! Sprint 1 提供 MVP：
//! - 仅扫 `JAVA_HOME` + `PATH`（不读注册表 / Adoptium 默认目录）
//! - 通过 `java -version` 解析输出获取版本与 vendor
//! - 兼容矩阵基于 manifest 的 `javaVersion.majorVersion` 字段（最权威）
//!
//! Sprint 5 会补全：注册表 / Adoptium / Microsoft / Zulu / Liberica /
//! `.minecraft/runtime` Mojang JRE / `release` 文件解析 / 内存推荐

pub mod compat;
pub mod model;
pub mod scan;

pub use compat::{required_java_major, select_best};
pub use model::{Arch, JavaRuntime, JavaSource};
pub use scan::{detect_version, scan_all};
