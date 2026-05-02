use crate::errors::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// 启动器全局配置（存于 [`PathLayout::config_file`](crate::paths::PathLayout::config_file)）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// 镜像源策略
    pub mirror_policy: MirrorPolicy,
    /// 全局并发下载数（信号量上限）
    pub max_concurrent_downloads: usize,
    /// UI 语言
    pub ui_locale: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            mirror_policy: MirrorPolicy::Auto,
            max_concurrent_downloads: 16,
            ui_locale: "zh-CN".to_string(),
        }
    }
}

/// 镜像源选择策略。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MirrorPolicy {
    /// 默认：BMCLAPI 优先，失败回落官方
    Auto,
    /// 仅官方 launchermeta.mojang.com
    Official,
    /// 仅 bmclapi2.bangbang93.com
    Bmclapi,
}

impl AppConfig {
    pub fn load_or_default(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let bytes = fs::read(path).map_err(|e| Error::io(path, e))?;
        let cfg: Self = serde_json::from_slice(&bytes)?;
        Ok(cfg)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        let json = serde_json::to_vec_pretty(self)?;
        fs::write(path, json).map_err(|e| Error::io(path, e))?;
        Ok(())
    }
}
