use super::arguments::Arguments;
use super::assets::AssetIndexInfo;
use super::java::JavaVersionRequirement;
use super::library::{maven_ga, Library};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// `version_manifest_v2.json` 顶层结构。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionList {
    pub latest: LatestVersions,
    pub versions: Vec<VersionListEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LatestVersions {
    pub release: String,
    pub snapshot: String,
}

/// 版本清单中的一项。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionListEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: VersionType,
    pub url: String,
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
    pub sha1: String,
    #[serde(rename = "complianceLevel", default)]
    pub compliance_level: u32,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VersionType {
    Release,
    Snapshot,
    OldBeta,
    OldAlpha,
}

/// 单版本 JSON 的"原始"结构。可能含 `inheritsFrom`（Forge / Fabric / NeoForge 等）。
///
/// 解析后通过 [`merge_inherits`](Self::merge_inherits) 与父版本合并，得到
/// [`ResolvedManifest`]。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RawVersion {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: Option<VersionType>,
    pub main_class: Option<String>,
    pub asset_index: Option<AssetIndexInfo>,
    /// 旧版本只有 "assets" 字符串字段（指向 asset index id）
    pub assets: Option<String>,
    pub downloads: Option<MainDownloads>,
    #[serde(default)]
    pub libraries: Vec<Library>,
    pub arguments: Option<Arguments>,
    /// 1.12 及以前的旧字段，整段命令行文本
    pub minecraft_arguments: Option<String>,
    pub java_version: Option<JavaVersionRequirement>,
    pub inherits_from: Option<String>,
    pub release_time: Option<String>,
    pub time: Option<String>,
    pub minimum_launcher_version: Option<u32>,
}

/// 主下载组（client.jar / server.jar / mappings 等）
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MainDownloads {
    pub client: Option<DownloadInfo>,
    #[serde(rename = "client_mappings")]
    pub client_mappings: Option<DownloadInfo>,
    pub server: Option<DownloadInfo>,
    #[serde(rename = "server_mappings")]
    pub server_mappings: Option<DownloadInfo>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DownloadInfo {
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

/// 合并 inheritsFrom 后的版本清单。所有 Sprint 1 后续逻辑（下载 / 启动参数构建）
/// 都消费这个结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedManifest {
    pub id: String,
    pub kind: VersionType,
    pub main_class: String,
    pub asset_index: AssetIndexInfo,
    pub downloads: MainDownloads,
    pub libraries: Vec<Library>,
    pub arguments: Arguments,
    /// 旧版整段命令行（与新格式 arguments 共存时拼接）
    pub minecraft_arguments: Option<String>,
    pub java_version: JavaVersionRequirement,
    /// 继承链（从 child 到 root：[id, parent_id, ...]）。仅供调试 / 上报。
    pub inherits_chain: Vec<String>,
}

impl RawVersion {
    /// 把 child 的字段合并到 parent 上，child 优先（除 libraries 与 arguments 是追加）。
    ///
    /// 调用约定：`child.merge_inherits(parent)` 返回合并结果。Forge/Fabric 装出来的
    /// version JSON 是 child；它的 `inheritsFrom` 指向 vanilla `parent`。
    #[must_use]
    pub fn merge_inherits(self, parent: RawVersion) -> RawVersion {
        RawVersion {
            id: self.id,
            kind: self.kind.or(parent.kind),
            main_class: self.main_class.or(parent.main_class),
            asset_index: self.asset_index.or(parent.asset_index),
            assets: self.assets.or(parent.assets),
            downloads: self.downloads.or(parent.downloads),
            libraries: merge_libraries(parent.libraries, self.libraries),
            arguments: merge_arguments(parent.arguments, self.arguments),
            minecraft_arguments: self.minecraft_arguments.or(parent.minecraft_arguments),
            java_version: self.java_version.or(parent.java_version),
            inherits_from: parent.inherits_from,
            release_time: self.release_time.or(parent.release_time),
            time: self.time.or(parent.time),
            minimum_launcher_version: self
                .minimum_launcher_version
                .or(parent.minimum_launcher_version),
        }
    }

    /// 把已合并好的 `RawVersion`（无 inheritsFrom）固化为 `ResolvedManifest`。
    /// 缺失关键字段时返回 `Err`。
    pub fn into_resolved(self, inherits_chain: Vec<String>) -> crate::errors::Result<ResolvedManifest> {
        use crate::errors::Error;
        Ok(ResolvedManifest {
            id: self.id,
            kind: self.kind.unwrap_or(VersionType::Release),
            main_class: self
                .main_class
                .ok_or_else(|| Error::Config("manifest missing mainClass".into()))?,
            asset_index: self
                .asset_index
                .ok_or_else(|| Error::Config("manifest missing assetIndex".into()))?,
            downloads: self
                .downloads
                .ok_or_else(|| Error::Config("manifest missing downloads".into()))?,
            libraries: self.libraries,
            arguments: self.arguments.unwrap_or_default(),
            minecraft_arguments: self.minecraft_arguments,
            java_version: self.java_version.unwrap_or_default(),
            inherits_chain,
        })
    }
}

/// 合并 libraries：parent 在前，child 在后。
/// 同 GA（group:artifact）的库优先保留 child（loader 经常重写 vanilla 的库版本）；
/// 完全相同的 GAV+classifier 去重。
fn merge_libraries(parent: Vec<Library>, child: Vec<Library>) -> Vec<Library> {
    // 第一步：按 GA 收集 child 提供的版本，用于覆盖 parent
    let child_gas: HashMap<String, ()> = child
        .iter()
        .map(|l| (maven_ga(&l.name).to_string(), ()))
        .collect();

    let mut result: Vec<Library> = Vec::with_capacity(parent.len() + child.len());
    let mut seen_full: std::collections::HashSet<String> = std::collections::HashSet::new();

    // parent 中 GA 不被 child 覆盖的保留
    for lib in parent {
        let ga = maven_ga(&lib.name).to_string();
        if child_gas.contains_key(&ga) {
            continue;
        }
        if seen_full.insert(lib.name.clone()) {
            result.push(lib);
        }
    }
    for lib in child {
        if seen_full.insert(lib.name.clone()) {
            result.push(lib);
        }
    }
    result
}

fn merge_arguments(
    parent: Option<Arguments>,
    child: Option<Arguments>,
) -> Option<Arguments> {
    match (parent, child) {
        (None, c) => c,
        (p, None) => p,
        (Some(mut p), Some(c)) => {
            p.game.extend(c.game);
            p.jvm.extend(c.jvm);
            Some(p)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::arguments::Argument;

    fn parse_raw(json: &str) -> RawVersion {
        serde_json::from_str(json).expect("fixture should parse")
    }

    /// 极简 vanilla fixture（保留必要字段）
    const VANILLA: &str = r#"{
        "id": "1.21.1",
        "type": "release",
        "mainClass": "net.minecraft.client.main.Main",
        "assetIndex": {
            "id": "16",
            "sha1": "0000000000000000000000000000000000000001",
            "size": 100,
            "totalSize": 1000,
            "url": "https://example.com/16.json"
        },
        "downloads": {
            "client": {
                "sha1": "0000000000000000000000000000000000000002",
                "size": 1234,
                "url": "https://example.com/client.jar"
            }
        },
        "javaVersion": { "component": "java-runtime-delta", "majorVersion": 21 },
        "libraries": [
            {
                "name": "org.lwjgl:lwjgl:3.3.3",
                "downloads": { "artifact": { "path": "p", "sha1": "s", "size": 1, "url": "u" } }
            },
            {
                "name": "ca.weblite:java-objc-bridge:1.1",
                "downloads": { "artifact": { "path": "p", "sha1": "s", "size": 1, "url": "u" } },
                "rules": [{ "action": "allow", "os": { "name": "osx" } }]
            }
        ],
        "arguments": {
            "game": ["--username", "${auth_player_name}"],
            "jvm": ["-Djava.library.path=${natives_directory}"]
        }
    }"#;

    /// Fabric / Forge 风格的 child fixture
    const LOADER_CHILD: &str = r#"{
        "id": "1.21.1-fabric-0.16.0",
        "inheritsFrom": "1.21.1",
        "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient",
        "libraries": [
            {
                "name": "org.lwjgl:lwjgl:3.3.4",
                "downloads": { "artifact": { "path": "p", "sha1": "s", "size": 1, "url": "u" } }
            },
            {
                "name": "net.fabricmc:fabric-loader:0.16.0",
                "downloads": { "artifact": { "path": "p", "sha1": "s", "size": 1, "url": "u" } }
            }
        ],
        "arguments": {
            "jvm": ["-DFabricMcEmu=net.minecraft.client.main.Main"]
        }
    }"#;

    #[test]
    fn child_overrides_main_class() {
        let parent = parse_raw(VANILLA);
        let child = parse_raw(LOADER_CHILD);
        let merged = child.merge_inherits(parent);
        assert_eq!(
            merged.main_class.as_deref(),
            Some("net.fabricmc.loader.impl.launch.knot.KnotClient")
        );
    }

    #[test]
    fn libraries_dedupe_by_ga_child_wins() {
        let parent = parse_raw(VANILLA);
        let child = parse_raw(LOADER_CHILD);
        let merged = child.merge_inherits(parent);

        // 同 GA 的 lwjgl 应只保留 child 版本
        let lwjgls: Vec<&Library> = merged
            .libraries
            .iter()
            .filter(|l| l.name.starts_with("org.lwjgl:lwjgl:"))
            .collect();
        assert_eq!(lwjgls.len(), 1, "lwjgl deduped");
        assert_eq!(lwjgls[0].name, "org.lwjgl:lwjgl:3.3.4", "child wins");

        // parent 独有的库（java-objc-bridge）保留
        assert!(merged
            .libraries
            .iter()
            .any(|l| l.name == "ca.weblite:java-objc-bridge:1.1"));

        // child 独有的（fabric-loader）也在
        assert!(merged
            .libraries
            .iter()
            .any(|l| l.name == "net.fabricmc:fabric-loader:0.16.0"));
    }

    #[test]
    fn arguments_merge_preserves_parent_then_child() {
        let parent = parse_raw(VANILLA);
        let child = parse_raw(LOADER_CHILD);
        let merged = child.merge_inherits(parent);
        let args = merged.arguments.expect("has arguments");

        // parent.jvm + child.jvm: 顺序为 parent 在前
        let jvm_str: Vec<&str> = args
            .jvm
            .iter()
            .filter_map(|a| match a {
                Argument::Plain(s) => Some(s.as_str()),
                Argument::Conditional { .. } => None,
            })
            .collect();
        assert_eq!(
            jvm_str,
            vec![
                "-Djava.library.path=${natives_directory}",
                "-DFabricMcEmu=net.minecraft.client.main.Main",
            ]
        );

        // parent 的 game 参数依然在
        let game_str: Vec<&str> = args
            .game
            .iter()
            .filter_map(|a| match a {
                Argument::Plain(s) => Some(s.as_str()),
                Argument::Conditional { .. } => None,
            })
            .collect();
        assert_eq!(game_str, vec!["--username", "${auth_player_name}"]);
    }

    #[test]
    fn into_resolved_succeeds_after_merge() {
        let parent = parse_raw(VANILLA);
        let child = parse_raw(LOADER_CHILD);
        let merged = child.merge_inherits(parent);
        let resolved = merged
            .into_resolved(vec![
                "1.21.1-fabric-0.16.0".to_string(),
                "1.21.1".to_string(),
            ])
            .expect("resolved");
        assert_eq!(resolved.id, "1.21.1-fabric-0.16.0");
        assert_eq!(resolved.kind, VersionType::Release);
        assert_eq!(resolved.java_version.major_version, 21);
        assert_eq!(resolved.inherits_chain.len(), 2);
    }

    #[test]
    fn into_resolved_fails_without_main_class() {
        let mut raw = parse_raw(VANILLA);
        raw.main_class = None;
        let res = raw.into_resolved(vec!["x".to_string()]);
        assert!(res.is_err());
    }

    #[test]
    fn vanilla_alone_resolves_without_parent() {
        let raw = parse_raw(VANILLA);
        assert!(raw.inherits_from.is_none());
        let resolved = raw.into_resolved(vec!["1.21.1".to_string()]).unwrap();
        assert_eq!(resolved.main_class, "net.minecraft.client.main.Main");
        assert_eq!(resolved.libraries.len(), 2);
    }
}

