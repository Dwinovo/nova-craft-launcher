//! NCL loader: ModLoader trait + Forge / Fabric / NeoForge 三套实现。
//!
//! Sprint 3a 已实现：Fabric（最简单，纯 JSON 合并）
//! Sprint 3b 已实现：Forge（installer + processors 引擎）
//! Sprint 4b 待实现：NeoForge（与 Forge 共享 processor 引擎，Maven 不同）

pub mod fabric;
pub mod forge;
pub mod model;
pub mod profile_lib;

pub use fabric::FabricLoader;
pub use forge::install_forge;
pub use model::{LoaderInstallResult, LoaderKind, LoaderVersion};
