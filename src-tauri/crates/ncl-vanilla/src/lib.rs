//! NCL vanilla: Mojang 原版 manifest / asset / library 处理。

pub mod manifest;

pub use manifest::{
    default_client, fetch_version_detail, fetch_version_list, resolve_inherits,
    VERSION_MANIFEST_URL,
};
