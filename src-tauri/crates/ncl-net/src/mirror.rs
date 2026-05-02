use std::sync::Arc;

/// 镜像源 trait。`rewrite` 把官方 URL 改写为镜像 URL；返回 `None`
/// 表示该镜像不接管这个 URL（应继续尝试下一个）。
pub trait MirrorSource: Send + Sync {
    fn name(&self) -> &str;
    fn rewrite(&self, url: &str) -> Option<String>;
}

/// 官方源：所有 URL 原样透传。`MirrorPool` 的 fallback 终点。
pub struct OfficialSource;

impl MirrorSource for OfficialSource {
    fn name(&self) -> &str {
        "official"
    }
    fn rewrite(&self, url: &str) -> Option<String> {
        Some(url.to_string())
    }
}

pub type SharedSource = Arc<dyn MirrorSource>;
