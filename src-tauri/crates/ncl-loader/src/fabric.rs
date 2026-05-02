//! Fabric loader 接入：[Fabric Meta API](https://meta.fabricmc.net/) 直接吐出
//! 含 `inheritsFrom` 的 version JSON，套用 ncl-vanilla 现有 inherits 合并 +
//! install 流水线即可。
//!
//! 关键 endpoint：
//! - `GET /v2/versions/loader/<game>` — 列出某 MC 版本可用的 loader 版本
//! - `GET /v2/versions/loader/<game>/<loader>/profile/json` — 拿到 ready-to-use
//!   的 version JSON（含 inheritsFrom = `<game>`，libraries = fabric-loader +
//!   intermediary + asm 等）

use crate::model::{LoaderInstallResult, LoaderVersion};
use ncl_core::model::RawVersion;
use ncl_core::{Error, Result};
use reqwest::Client;
use serde::Deserialize;

const FABRIC_META: &str = "https://meta.fabricmc.net";

pub struct FabricLoader;

impl FabricLoader {
    /// 拉取某 Minecraft 版本可用的 fabric-loader 版本列表（按 Fabric 默认顺序，新版在前）。
    pub async fn list_versions(client: &Client, mc_version: &str) -> Result<Vec<LoaderVersion>> {
        let url = format!("{FABRIC_META}/v2/versions/loader/{mc_version}");
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
        let raw: Vec<FabricLoaderEntry> = serde_json::from_slice(&bytes).map_err(Error::Json)?;
        Ok(raw
            .into_iter()
            .map(|e| LoaderVersion {
                version: e.loader.version,
                stable: e.loader.stable,
            })
            .collect())
    }

    /// 拉取 Fabric profile JSON（已是 NCL 期望的 RawVersion 形态，
    /// 含 `inheritsFrom = <mc_version>`）。
    pub async fn fetch_profile(
        client: &Client,
        mc_version: &str,
        loader_version: &str,
    ) -> Result<RawVersion> {
        let url = format!(
            "{FABRIC_META}/v2/versions/loader/{mc_version}/{loader_version}/profile/json"
        );
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
        serde_json::from_slice(&bytes).map_err(Error::Json)
    }

    /// 一站式：拉 profile → 解 inherits → install → 返回最终 version_id。
    ///
    /// 这个方法不直接做安装（避免 ncl-loader 依赖 ResilientDownloader 等），
    /// 而是返回拉取并合并好的 RawVersion，由调用方自行走 install 流水线。
    pub fn synthesize_result(profile: &RawVersion) -> LoaderInstallResult {
        LoaderInstallResult {
            merged_version_id: profile.id.clone(),
            main_class: profile
                .main_class
                .clone()
                .unwrap_or_else(|| "net.fabricmc.loader.impl.launch.knot.KnotClient".into()),
        }
    }
}

#[derive(Debug, Deserialize)]
struct FabricLoaderEntry {
    loader: FabricLoaderInfo,
}

#[derive(Debug, Deserialize)]
struct FabricLoaderInfo {
    version: String,
    stable: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 真实拉一次 Fabric Meta API 验证 schema 兼容；CI 跳过。
    /// 本地：`cargo test -p ncl-loader -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn live_fabric_list_versions_for_1_21_1() {
        let client = Client::new();
        let list = FabricLoader::list_versions(&client, "1.21.1")
            .await
            .expect("list");
        assert!(!list.is_empty(), "Fabric 必有 1.21.1 loader 版本");
        eprintln!("found {} fabric loader versions", list.len());
        for v in list.iter().take(3) {
            eprintln!("  {} stable={}", v.version, v.stable);
        }
    }

    #[tokio::test]
    #[ignore]
    async fn live_fabric_fetch_profile() {
        let client = Client::new();
        let list = FabricLoader::list_versions(&client, "1.21.1")
            .await
            .unwrap();
        let stable = list.iter().find(|v| v.stable).expect("stable");
        let profile = FabricLoader::fetch_profile(&client, "1.21.1", &stable.version)
            .await
            .expect("profile");
        assert!(profile.inherits_from.as_deref() == Some("1.21.1"));
        assert!(profile.id.contains("fabric"));
        assert!(profile.main_class.is_some());
        let result = FabricLoader::synthesize_result(&profile);
        eprintln!("fabric result: {result:?}");
    }
}
