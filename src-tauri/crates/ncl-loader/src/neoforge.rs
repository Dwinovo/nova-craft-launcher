//! NeoForge loader 接入。
//!
//! 与 Forge 共用 installer/processor 引擎（参见 [`crate::forge::install_from_installer`]），
//! 仅版本列表 API 与 maven 路径不同。
//!
//! - 列表（BMCLAPI 代理）: `GET /neoforge/list/<mc>`
//! - installer JAR: `https://maven.neoforged.net/releases/net/neoforged/neoforge/<ver>/neoforge-<ver>-installer.jar`

use crate::forge::{install_from_installer, ForgeInstallOutput};
use crate::model::LoaderVersion;
use ncl_core::errors::{Error, Result};
use ncl_java::JavaRuntime;
use ncl_net::ResilientDownloader;
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;

const BMCLAPI_NEOFORGE_LIST: &str = "https://bmclapi2.bangbang93.com/neoforge/list";

#[derive(Debug, Clone, Deserialize)]
struct NeoForgeEntry {
    version: String,
}

/// 列出某 MC 版本的 NeoForge 版本（新版在前）。
///
/// 注意：NeoForge 在 MC 1.20.1 之前不存在（1.20.2+ 才独立维护）。
pub async fn list_versions(client: &Client, mc_version: &str) -> Result<Vec<LoaderVersion>> {
    let url = format!("{BMCLAPI_NEOFORGE_LIST}/{mc_version}");
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
    let mut entries: Vec<NeoForgeEntry> = serde_json::from_slice(&bytes).map_err(Error::Json)?;
    // BMCLAPI 默认排序不稳定；按 build 数倒序（NeoForge 版本最后一段为 build）
    entries.sort_by(|a, b| b.version.cmp(&a.version));
    Ok(entries
        .into_iter()
        .map(|e| LoaderVersion {
            version: e.version,
            stable: true,
        })
        .collect())
}

/// 给定 NeoForge 版本号返回 installer jar URL。
#[must_use]
pub fn neoforge_installer_url(neoforge_version: &str) -> String {
    format!(
        "https://maven.neoforged.net/releases/net/neoforged/neoforge/{neoforge_version}/neoforge-{neoforge_version}-installer.jar"
    )
}

/// 安装 NeoForge。复用 [`install_from_installer`] 的处理逻辑。
#[allow(clippy::too_many_arguments)]
pub async fn install_neoforge(
    mc_version: &str,
    neoforge_version: &str,
    instance_name: &str,
    layout: &ncl_core::PathLayout,
    vanilla_list: &ncl_core::VersionList,
    java: &JavaRuntime,
    downloader: Arc<ResilientDownloader>,
    sink: Arc<dyn ncl_core::progress::ProgressSink>,
    concurrency: usize,
) -> Result<ForgeInstallOutput> {
    let merged_id = format!("{mc_version}-neoforge-{neoforge_version}");
    let installer_url = neoforge_installer_url(neoforge_version);
    install_from_installer(
        &installer_url,
        &merged_id,
        "neoforge",
        instance_name,
        layout,
        vanilla_list,
        java,
        downloader,
        sink,
        concurrency,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installer_url_format() {
        let url = neoforge_installer_url("21.1.69");
        assert!(url.contains("maven.neoforged.net"));
        assert!(url.ends_with("/neoforge-21.1.69-installer.jar"));
    }

    #[tokio::test]
    #[ignore]
    async fn live_list_neoforge_for_1_21_1() {
        let client = Client::new();
        let list = list_versions(&client, "1.21.1").await.expect("list");
        assert!(!list.is_empty());
        eprintln!(
            "found {} neoforge versions for 1.21.1, latest = {}",
            list.len(),
            list[0].version
        );
    }
}
