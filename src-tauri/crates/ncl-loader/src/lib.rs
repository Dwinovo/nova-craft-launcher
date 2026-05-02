//! NCL loader: ModLoader trait + Forge / Fabric / NeoForge 三套实现。
//!
//! Sprint 3a 已实现：Fabric（最简单，纯 JSON 合并）
//! Sprint 3b 待实现：Forge（需调用 installer CLI 跑 processor）
//! Sprint 4 待实现：NeoForge

pub mod fabric;
pub mod model;

pub use fabric::FabricLoader;
pub use model::{LoaderInstallResult, LoaderKind, LoaderVersion};
