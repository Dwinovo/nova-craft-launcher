//! NCL mod: 解析 jar 内的 mod 元数据（Forge / Fabric / NeoForge）+ 启用/禁用切换。
//!
//! Sprint 4a 范围：
//! - 三种 manifest 格式解析：META-INF/mods.toml / fabric.mod.json /
//!   META-INF/neoforge.mods.toml
//! - 扫描目录得到 ModEntry 列表
//! - `.jar` ↔ `.jar.disabled` 重命名（PCL/HMCL/Prism 通用约定）

pub mod enable;
pub mod parse;
pub mod scan;

pub use enable::{set_enabled, ToggleResult};
pub use parse::{parse_jar, ParseError};
pub use scan::scan_mods;
