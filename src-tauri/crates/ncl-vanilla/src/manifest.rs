//! Mojang launcher meta 拉取与版本继承解析。
//!
//! 入口：
//! - [`fetch_version_list`] 拉取 `version_manifest_v2.json`
//! - [`fetch_version_detail`] 拉取单版本 JSON
//! - [`resolve_inherits`] 递归处理 `inheritsFrom`，产出 [`ResolvedManifest`]

use ncl_core::errors::{Error, Result};
use ncl_core::model::{RawVersion, ResolvedManifest, VersionList};
use reqwest::Client;
use std::time::Duration;

/// 官方版本清单地址。Sprint 2 起会被 `MirrorPool` 改写。
pub const VERSION_MANIFEST_URL: &str =
    "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json";

/// 创建一个适合拉 manifest 的轻量 client。manifest 是 JSON GET，与 ncl-net 的
/// 文件下载流不冲突。
pub fn default_client() -> Result<Client> {
    Client::builder()
        .user_agent("Nova-Craft-Launcher/0.1.0")
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| Error::Network(e.to_string()))
}

pub async fn fetch_version_list(client: &Client) -> Result<VersionList> {
    fetch_json(client, VERSION_MANIFEST_URL).await
}

pub async fn fetch_version_detail(client: &Client, url: &str) -> Result<RawVersion> {
    fetch_json(client, url).await
}

/// 递归解析 inheritsFrom：找 parent → 拉 parent JSON → child.merge(parent) → 检查 parent.inheritsFrom →
/// 直到链顶（无 inheritsFrom 的 vanilla）。
///
/// 检测循环 inheritsFrom 时返回错误。
pub async fn resolve_inherits(
    client: &Client,
    raw: RawVersion,
    list: &VersionList,
) -> Result<ResolvedManifest> {
    let mut chain = vec![raw.id.clone()];
    let mut current = raw;

    while let Some(parent_id) = current.inherits_from.clone() {
        if chain.iter().any(|id| id == &parent_id) {
            return Err(Error::Config(format!(
                "circular inheritsFrom detected: chain={chain:?}, next={parent_id}"
            )));
        }
        let parent_entry = list
            .versions
            .iter()
            .find(|v| v.id == parent_id)
            .ok_or_else(|| {
                Error::NotFound(format!(
                    "inheritsFrom parent '{parent_id}' not found in version manifest"
                ))
            })?;
        let parent = fetch_version_detail(client, &parent_entry.url).await?;
        chain.push(parent_id);
        current = current.merge_inherits(parent);
    }

    current.into_resolved(chain)
}

async fn fetch_json<T: serde::de::DeserializeOwned>(client: &Client, url: &str) -> Result<T> {
    let resp = client
        .get(url)
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
        .map_err(|e| Error::Network(format!("read body {url}: {e}")))?;
    let parsed: T = serde_json::from_slice(&bytes).map_err(Error::Json)?;
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 真实拉一次 launchermeta，验证 JSON schema 与我们的反序列化一致。
    /// 跑这个测试需要外网；CI 跳过，本地验证用：
    /// `cargo test -p ncl-vanilla -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn live_fetch_version_list_and_detail_1_21_1() {
        let client = default_client().unwrap();
        let list = fetch_version_list(&client).await.expect("fetch list");
        assert!(!list.versions.is_empty());
        let entry = list
            .versions
            .iter()
            .find(|v| v.id == "1.21.1")
            .expect("1.21.1 in manifest");
        let detail = fetch_version_detail(&client, &entry.url)
            .await
            .expect("fetch detail");
        assert_eq!(detail.id, "1.21.1");
        let resolved = resolve_inherits(&client, detail, &list).await.unwrap();
        assert_eq!(resolved.id, "1.21.1");
        assert!(!resolved.libraries.is_empty());
        assert!(resolved.java_version.major_version >= 21);
    }
}
