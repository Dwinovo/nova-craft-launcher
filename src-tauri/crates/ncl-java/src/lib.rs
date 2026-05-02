//! NCL java: Java 运行时检测、版本兼容矩阵、内存推荐。
//!
//! Sprint 5（已实现）：
//! - 多源扫描：JAVA_HOME / PATH / Adoptium / Microsoft / Zulu / Liberica /
//!   Oracle / Windows 注册表 / Mojang JRE
//! - 版本判定优先读 `release` 文件（无需启动 JVM），失败回退 `java -version`
//! - 内存推荐：基于系统总内存 + loader + mod 数量

pub mod compat;
pub mod dirs;
pub mod memory;
pub mod model;
pub mod registry;
pub mod release;
pub mod scan;

pub use compat::{required_java_major, select_best};
pub use memory::{recommend_memory, MemoryRecommendation};
pub use model::{Arch, JavaRuntime, JavaSource};
pub use release::{parse_release_content, parse_release_file, ReleaseInfo};
pub use scan::{detect_version, scan_all, scan_all_with};
