//! NCL net: HTTP 客户端、镜像源 trait、镜像池、韧性下载器、SHA 校验。

pub mod checksum;
pub mod download;
pub mod mirror;
pub mod resilient;

pub use checksum::{sha1_file, verify_sha1};
pub use download::Downloader;
pub use mirror::{BmclApiSource, MirrorPool, MirrorSource, OfficialSource, SharedSource};
pub use resilient::ResilientDownloader;
