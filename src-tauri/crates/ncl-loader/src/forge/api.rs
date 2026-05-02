//! BMCLAPI 客户端：拉取 Forge 版本列表 + installer 下载 URL。
//!
//! BMCLAPI Forge endpoints (https://bmclapidoc.bangbang93.com/):
//! - `GET /forge/minecraft` — 所有有 Forge 支持的 MC 版本
//! - `GET /forge/minecraft/<mc>` — 该 MC 版本所有 Forge 构建
//!
//! installer 下载使用 maven 路径：
//! `https://maven.minecraftforge.net/net/minecraftforge/forge/<mc>-<ver>/forge-<mc>-<ver>-installer.jar`

use crate::model::LoaderVersion;
use ncl_core::errors::{Error, Result};
use reqwest::Client;
use serde::Deserialize;

const BMCLAPI_FORGE: &str = "https://bmclapi2.bangbang93.com/forge";

#[derive(Debug, Clone, Deserialize)]
pub struct ForgeBuildEntry {
    pub build: u64,
    pub mcversion: String,
    pub version: String,
    #[serde(default)]
    pub modified: Option<String>,
}

/// 列出某 MC 版本的所有 Forge 构建。BMCLAPI 返回按 build 数升序，调用方
/// 期望最新在前——所以这里反转。
pub async fn list_versions(client: &Client, mc_version: &str) -> Result<Vec<LoaderVersion>> {
    let url = format!("{BMCLAPI_FORGE}/minecraft/{mc_version}");
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| Error::Network(format!("GET {url}: {e}")))?;
    if !resp.status().is_success() {
        return Err(Error::Network(format!(
            "GET {url}: HTTP {}",
            resp.status()
        )));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| Error::Network(format!("read {url}: {e}")))?;
    let mut entries: Vec<ForgeBuildEntry> = serde_json::from_slice(&bytes).map_err(Error::Json)?;
    // 新版在前
    entries.sort_by(|a, b| b.build.cmp(&a.build));
    // Forge 没有 stable/beta 区分；统一标 stable
    Ok(entries
        .into_iter()
        .map(|e| LoaderVersion {
            version: e.version,
            stable: true,
        })
        .collect())
}

/// 给定 MC 版本与 Forge 版本号，返回 installer jar 下载 URL（官方 maven）。
/// 这个 URL 会被 BMCLAPI / OfficialSource 通过 `MirrorPool` 改写。
#[must_use]
pub fn forge_installer_url(mc_version: &str, forge_version: &str) -> String {
    let coord = format!("{mc_version}-{forge_version}");
    format!(
        "https://maven.minecraftforge.net/net/minecraftforge/forge/{coord}/forge-{coord}-installer.jar"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installer_url_format() {
        let url = forge_installer_url("1.20.1", "47.3.0");
        assert!(url.ends_with("/forge-1.20.1-47.3.0-installer.jar"));
        assert!(url.contains("maven.minecraftforge.net"));
    }

    /// 真实拉一次 BMCLAPI Forge 列表；CI 跳过。
    #[tokio::test]
    #[ignore]
    async fn live_list_forge_for_1_20_1() {
        let client = Client::new();
        let list = list_versions(&client, "1.20.1").await.expect("list");
        assert!(!list.is_empty(), "Forge 1.20.1 必有版本");
        eprintln!(
            "found {} forge versions for 1.20.1, latest = {}",
            list.len(),
            list[0].version
        );
    }
}
