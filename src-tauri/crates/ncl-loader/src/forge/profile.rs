//! Forge `install_profile.json` 数据结构 + 占位符 token 解析。
//!
//! Install profile 的 `data` 字段把抽象 KEY 映射到 client/server 各自的具体值。
//! 而 processors 的 `args` 与 outputs 用 `[maven:coord]` 引用 maven artifact、
//! 用 `{KEY}` 引用 data 字段。我们提供 token 解析函数。

use ncl_core::errors::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// `install_profile.json` 顶层结构（Forge 1.13+）。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InstallProfile {
    /// 通常为 1
    #[serde(default)]
    pub spec: i32,
    pub profile: String,
    pub version: String,
    /// 基础 MC 版本
    pub minecraft: String,
    /// 指向 installer 内嵌 version.json 的相对路径，通常 "/version.json"
    #[serde(default)]
    pub json: Option<String>,
    /// 安装到 launcher_profiles 中的 path 字段（NCL 不用）
    #[serde(default)]
    pub path: Option<String>,
    /// data[KEY][side] 映射。常见 KEY：MAPPINGS / MOJMAPS / MERGED_MAPPINGS /
    /// MC_SLIM / MC_EXTRA / MC_SRG / PATCHED / MCP_VERSION / BINPATCH / SIDE
    #[serde(default)]
    pub data: HashMap<String, DataEntry>,
    /// 必须按顺序执行的 processor 队列
    #[serde(default)]
    pub processors: Vec<Processor>,
    /// 仅安装时下载的库（不进入运行时 classpath；与 version.json 的 libraries 区分）
    #[serde(default)]
    pub libraries: Vec<crate::profile_lib::ProfileLibrary>,
}

/// `data` 字段的单条目；不同 side 不同值。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DataEntry {
    pub client: String,
    pub server: String,
}

/// 单个 processor 描述。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Processor {
    /// processor jar 的 maven 坐标。其 META-INF/MANIFEST.MF 的 Main-Class 即入口
    pub jar: String,
    /// 完整 classpath（maven 坐标列表）；jar 自身一般也在这里
    #[serde(default)]
    pub classpath: Vec<String>,
    /// 命令行参数；其中可能含 `[maven:coord]` 与 `{KEY}` 占位符
    #[serde(default)]
    pub args: Vec<String>,
    /// 期望的输出文件 path → SHA1，供安装后验证
    #[serde(default)]
    pub outputs: HashMap<String, String>,
    /// 仅在指定 sides 上执行；缺省同时跑 client + server
    #[serde(default)]
    pub sides: Vec<String>,
}

impl Processor {
    /// 是否在 client 安装中执行。
    #[must_use]
    pub fn applies_to_client(&self) -> bool {
        self.sides.is_empty() || self.sides.iter().any(|s| s == "client")
    }
}

/// 把单个 token 解析为最终值。
///
/// - `[group:artifact:version[:classifier][@ext]]` → 转为 `libraries_root/<maven path>`
/// - `{KEY}` → 查 `data[KEY].client` 字段，递归解析（值仍是 token 时再 resolve）
/// - 其他保持原样
pub fn resolve_token(
    raw: &str,
    data: &HashMap<String, DataEntry>,
    libraries_root: &Path,
) -> Result<String> {
    resolve_with_depth(raw, data, libraries_root, 8)
}

fn resolve_with_depth(
    raw: &str,
    data: &HashMap<String, DataEntry>,
    libraries_root: &Path,
    depth: u32,
) -> Result<String> {
    if depth == 0 {
        return Err(Error::Config(format!(
            "token resolve depth exceeded: {raw}"
        )));
    }

    if let Some(inner) = strip_brackets(raw) {
        // [maven:coord] — 转 maven 路径，不再递归（叶子）
        let path = maven_coord_to_path(libraries_root, inner);
        return Ok(path.to_string_lossy().into_owned());
    }

    if let Some(key) = strip_braces(raw) {
        // {KEY} — 查 data[KEY].client，再 resolve
        let entry = data
            .get(key)
            .ok_or_else(|| Error::Config(format!("install_profile.data missing key: {key}")))?;
        let inner = entry.client.clone();
        return resolve_with_depth(&inner, data, libraries_root, depth - 1);
    }

    // 内嵌占位符（`prefix${something}` 这种 Forge 不用，但兼容性留着）
    Ok(raw.to_string())
}

fn strip_brackets(s: &str) -> Option<&str> {
    s.strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
}

fn strip_braces(s: &str) -> Option<&str> {
    s.strip_prefix('{').and_then(|rest| rest.strip_suffix('}'))
}

/// Maven 坐标 → 本地文件路径。
///
/// 输入：`group:artifact:version[:classifier][@ext]`。
/// 输出：`libraries_root/<group_with_slashes>/artifact/version/artifact-version[-classifier].<ext>`。
pub fn maven_coord_to_path(libraries_root: &Path, coord: &str) -> PathBuf {
    // 拆 @ext
    let (gav, ext) = match coord.split_once('@') {
        Some((g, e)) => (g, e),
        None => (coord, "jar"),
    };
    let parts: Vec<&str> = gav.split(':').collect();
    if parts.len() < 3 {
        return libraries_root.join(coord);
    }
    let group = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];
    let classifier = parts.get(3).copied().unwrap_or("");
    let filename = if classifier.is_empty() {
        format!("{artifact}-{version}.{ext}")
    } else {
        format!("{artifact}-{version}-{classifier}.{ext}")
    };
    libraries_root
        .join(group)
        .join(artifact)
        .join(version)
        .join(filename)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> HashMap<String, DataEntry> {
        let mut d = HashMap::new();
        d.insert(
            "MC_SLIM".to_string(),
            DataEntry {
                client: "[net.minecraft:client:1.20.1:slim]".into(),
                server: "[net.minecraft:server:1.20.1:slim]".into(),
            },
        );
        d.insert(
            "MCP_VERSION".to_string(),
            DataEntry {
                client: "20230612.114412".into(),
                server: "20230612.114412".into(),
            },
        );
        d
    }

    #[test]
    fn resolve_maven_token_to_path() {
        let root = Path::new("/libs");
        let r = resolve_token("[net.minecraftforge:forge:1.20.1-47.3.0:client]", &HashMap::new(), root)
            .unwrap();
        // Path 在 Windows 下 / 与 \ 混合,只校验文件名 + 关键路径段
        assert!(r.contains("forge-1.20.1-47.3.0-client.jar"), "path was: {r}");
        assert!(r.contains("1.20.1-47.3.0"), "path was: {r}");
    }

    #[test]
    fn resolve_maven_with_extension() {
        let root = Path::new("/libs");
        let r = resolve_token(
            "[de.oceanlabs.mcp:mcp_config:1.20.1-20230612.114412@zip]",
            &HashMap::new(),
            root,
        )
        .unwrap();
        assert!(r.ends_with(".zip"), "path was: {r}");
    }

    #[test]
    fn resolve_brace_key_recursively() {
        let root = Path::new("/libs");
        // {MC_SLIM} → "[net.minecraft:client:1.20.1:slim]" → maven path
        let r = resolve_token("{MC_SLIM}", &data(), root).unwrap();
        assert!(r.contains("client-1.20.1-slim.jar"), "path was: {r}");
    }

    #[test]
    fn resolve_brace_key_string_value() {
        let r = resolve_token("{MCP_VERSION}", &data(), Path::new("/libs")).unwrap();
        assert_eq!(r, "20230612.114412");
    }

    #[test]
    fn resolve_unknown_brace_key_errors() {
        let r = resolve_token("{NOT_THERE}", &data(), Path::new("/libs"));
        assert!(r.is_err());
    }

    #[test]
    fn resolve_plain_string_passthrough() {
        let r = resolve_token("--task", &HashMap::new(), Path::new("/libs")).unwrap();
        assert_eq!(r, "--task");
    }

    #[test]
    fn maven_coord_basic() {
        let p = maven_coord_to_path(Path::new("/L"), "a.b:c:1.0");
        // 校验文件名 + 中间组件存在(避免 Windows / Unix path sep 差异)
        assert_eq!(
            p.file_name().and_then(|n| n.to_str()),
            Some("c-1.0.jar")
        );
        let s = p.to_string_lossy();
        assert!(s.contains("c") && s.contains("1.0"), "path was: {s}");
    }

    #[test]
    fn maven_coord_with_classifier() {
        let p = maven_coord_to_path(Path::new("/L"), "a.b:c:1.0:natives-windows");
        assert_eq!(
            p.file_name().and_then(|n| n.to_str()),
            Some("c-1.0-natives-windows.jar")
        );
    }

    #[test]
    fn applies_to_client_default_true() {
        let p = Processor {
            jar: "x:y:1".into(),
            classpath: vec![],
            args: vec![],
            outputs: HashMap::new(),
            sides: vec![],
        };
        assert!(p.applies_to_client());
    }

    #[test]
    fn applies_to_client_only_server_skip() {
        let p = Processor {
            jar: "x:y:1".into(),
            classpath: vec![],
            args: vec![],
            outputs: HashMap::new(),
            sides: vec!["server".into()],
        };
        assert!(!p.applies_to_client());
    }
}
