//! 从 .jar (zip) 内提取 mod 元数据并归一化到 [`ncl_core::ModEntry`]。
//!
//! 支持三种格式（按优先级）：
//! 1. `META-INF/neoforge.mods.toml` — NeoForge 1.20.5+ 引入的新路径
//! 2. `META-INF/mods.toml` — Forge 1.13+ / 早期 NeoForge
//! 3. `fabric.mod.json` — Fabric

use ncl_core::{LoaderKind, ModDep, ModEntry, Side};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("not a recognized mod jar: no mods.toml / fabric.mod.json / neoforge.mods.toml found")]
    NotAMod,
    #[error("toml parse error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("json parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid manifest: {0}")]
    Invalid(String),
}

/// 解析单个 jar，返回 mod 元数据；非 mod jar 返回 `Err(ParseError::NotAMod)`。
///
/// `enabled` 由调用方根据文件扩展名（`.jar` / `.jar.disabled`）填入。
pub fn parse_jar(jar_path: &Path) -> Result<ModEntry, ParseError> {
    let file = File::open(jar_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    // 优先级 1：NeoForge 新路径
    if let Ok(content) = read_entry(&mut archive, "META-INF/neoforge.mods.toml") {
        return parse_forge_like_toml(&content, LoaderKind::NeoForge, jar_path);
    }
    // 优先级 2：Forge / 早期 NeoForge
    if let Ok(content) = read_entry(&mut archive, "META-INF/mods.toml") {
        // 早期 NeoForge 也用这个路径，无法区分。先标 Forge；调用方若已知是
        // NeoForge 实例（version_id 含 neoforge）可后续覆盖。
        return parse_forge_like_toml(&content, LoaderKind::Forge, jar_path);
    }
    // 优先级 3：Fabric
    if let Ok(content) = read_entry(&mut archive, "fabric.mod.json") {
        return parse_fabric_json(&content, jar_path);
    }
    Err(ParseError::NotAMod)
}

fn read_entry(archive: &mut zip::ZipArchive<File>, name: &str) -> Result<String, ParseError> {
    let mut entry = archive.by_name(name)?;
    let mut s = String::new();
    entry.read_to_string(&mut s)?;
    Ok(s)
}

// ─────────────────────────── Forge / NeoForge mods.toml ───────────────────────────

#[derive(Debug, Deserialize)]
struct ModsToml {
    #[serde(default)]
    mods: Vec<ModsTomlMod>,
    #[serde(default)]
    dependencies: HashMap<String, Vec<ModsTomlDep>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModsTomlMod {
    mod_id: String,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    authors: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModsTomlDep {
    mod_id: String,
    #[serde(default = "default_mandatory")]
    mandatory: bool,
    #[serde(default)]
    version_range: Option<String>,
    #[serde(default)]
    side: Option<String>,
}

fn default_mandatory() -> bool {
    true
}

fn parse_forge_like_toml(
    content: &str,
    loader: LoaderKind,
    jar_path: &Path,
) -> Result<ModEntry, ParseError> {
    let parsed: ModsToml = toml::from_str(content)?;
    let primary = parsed
        .mods
        .into_iter()
        .next()
        .ok_or_else(|| ParseError::Invalid("mods.toml 中无 [[mods]] 条目".into()))?;
    let deps_raw = parsed
        .dependencies
        .get(&primary.mod_id)
        .cloned()
        .unwrap_or_default();

    let mc_version_range = deps_raw
        .iter()
        .find(|d| d.mod_id == "minecraft")
        .and_then(|d| d.version_range.clone());

    let dependencies = deps_raw
        .into_iter()
        .filter(|d| d.mod_id != "minecraft" && d.mod_id != "forge" && d.mod_id != "neoforge")
        .map(|d| ModDep {
            mod_id: d.mod_id,
            version_range: d.version_range,
            mandatory: d.mandatory,
            side: parse_side(d.side.as_deref()),
        })
        .collect();

    let authors = primary
        .authors
        .map(|s| {
            s.split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect()
        })
        .unwrap_or_default();

    Ok(make_entry(
        jar_path,
        loader,
        primary.mod_id,
        primary
            .display_name
            .or_else(|| Some(jar_path.file_stem()?.to_string_lossy().into_owned()))
            .unwrap_or_default(),
        primary.version.unwrap_or_else(|| "0.0.0".into()),
        mc_version_range,
        dependencies,
        authors,
        primary.description,
        Side::Both, // mods.toml 没有顶层 side，依赖各自有
    ))
}

// ─────────────────────────── Fabric ───────────────────────────

#[derive(Debug, Deserialize)]
struct FabricModJson {
    id: String,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    authors: Option<serde_json::Value>,
    #[serde(default)]
    depends: Option<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    environment: Option<String>,
}

fn parse_fabric_json(content: &str, jar_path: &Path) -> Result<ModEntry, ParseError> {
    let parsed: FabricModJson = serde_json::from_str(content)?;

    let depends = parsed.depends.unwrap_or_default();
    let mc_version_range = depends.get("minecraft").map(version_value_to_string);

    let dependencies = depends
        .iter()
        .filter(|(k, _)| {
            k.as_str() != "minecraft" && k.as_str() != "fabricloader" && k.as_str() != "java"
        })
        .map(|(k, v)| ModDep {
            mod_id: k.clone(),
            version_range: Some(version_value_to_string(v)),
            mandatory: true,
            side: Side::default(),
        })
        .collect();

    let authors = parsed
        .authors
        .map(|v| extract_fabric_authors(&v))
        .unwrap_or_default();

    let side = match parsed.environment.as_deref() {
        Some("client") => Side::Client,
        Some("server") => Side::Server,
        _ => Side::Both,
    };

    Ok(make_entry(
        jar_path,
        LoaderKind::Fabric,
        parsed.id.clone(),
        parsed.name.unwrap_or(parsed.id),
        parsed.version.unwrap_or_else(|| "0.0.0".into()),
        mc_version_range,
        dependencies,
        authors,
        parsed.description,
        side,
    ))
}

fn extract_fabric_authors(v: &serde_json::Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(arr) = v.as_array() {
        for item in arr {
            match item {
                serde_json::Value::String(s) => out.push(s.clone()),
                serde_json::Value::Object(o) => {
                    if let Some(name) = o.get("name").and_then(|v| v.as_str()) {
                        out.push(name.to_string());
                    }
                }
                _ => {}
            }
        }
    }
    out
}

fn version_value_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|x| x.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        other => other.to_string(),
    }
}

// ─────────────────────────── 共用 ───────────────────────────

#[allow(clippy::too_many_arguments)]
fn make_entry(
    jar_path: &Path,
    loader: LoaderKind,
    mod_id: String,
    name: String,
    version: String,
    mc_version_range: Option<String>,
    dependencies: Vec<ModDep>,
    authors: Vec<String>,
    description: Option<String>,
    side: Side,
) -> ModEntry {
    let enabled = !is_disabled_jar(jar_path);
    ModEntry {
        file_path: jar_path.to_path_buf(),
        enabled,
        loader,
        mod_id,
        name,
        version,
        mc_version_range,
        dependencies,
        authors,
        description,
        side,
    }
}

fn parse_side(s: Option<&str>) -> Side {
    match s.map(|x| x.to_ascii_uppercase()).as_deref() {
        Some("CLIENT") => Side::Client,
        Some("SERVER") => Side::Server,
        _ => Side::Both,
    }
}

fn is_disabled_jar(path: &Path) -> bool {
    matches!(path.extension().and_then(|e| e.to_str()), Some("disabled"))
}

/// 用于在 [`scan`](crate::scan) 中标识可识别的 jar 后缀。
pub(crate) fn is_mod_jar(path: &Path) -> bool {
    let name = match path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n.to_lowercase(),
        None => return false,
    };
    name.ends_with(".jar") || name.ends_with(".jar.disabled")
}

#[allow(dead_code)]
pub(crate) fn enabled_path(disabled: &Path) -> PathBuf {
    let stem = disabled
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .strip_suffix(".disabled")
        .unwrap_or_default();
    disabled.with_file_name(stem)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    fn make_jar(entries: &[(&str, &str)]) -> PathBuf {
        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "ncl-mod-test-{}-{}.jar",
            std::process::id(),
            unique_seq()
        ));
        let file = std::fs::File::create(&path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        for (name, content) in entries {
            zip.start_file(*name, SimpleFileOptions::default()).unwrap();
            zip.write_all(content.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
        path
    }

    fn uuid_like() -> String {
        unique_seq().to_string()
    }

    fn unique_seq() -> u64 {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        nanos.wrapping_add(n)
    }

    const FORGE_TOML: &str = r#"
modLoader = "javafml"
loaderVersion = "[40,)"
license = "MIT"
[[mods]]
modId = "examplemod"
version = "1.2.3"
displayName = "Example Mod"
description = "An example mod"
authors = "dwin, alice"
[[dependencies.examplemod]]
modId = "minecraft"
mandatory = true
versionRange = "[1.21,)"
ordering = "NONE"
side = "BOTH"
[[dependencies.examplemod]]
modId = "jei"
mandatory = false
versionRange = "[15.0,)"
side = "CLIENT"
"#;

    const FABRIC_JSON: &str = r#"{
        "schemaVersion": 1,
        "id": "examplefabric",
        "version": "0.5.0",
        "name": "Example Fabric Mod",
        "description": "Fabric demo",
        "authors": ["dwin", { "name": "alice" }],
        "depends": {
            "minecraft": ">=1.21.1",
            "fabricloader": ">=0.16.0",
            "fabric-api": "*"
        },
        "environment": "client"
    }"#;

    #[test]
    fn parse_forge_mods_toml() {
        let jar = make_jar(&[("META-INF/mods.toml", FORGE_TOML)]);
        let entry = parse_jar(&jar).expect("parse");
        assert_eq!(entry.mod_id, "examplemod");
        assert_eq!(entry.name, "Example Mod");
        assert_eq!(entry.version, "1.2.3");
        assert_eq!(entry.loader, LoaderKind::Forge);
        assert_eq!(entry.mc_version_range.as_deref(), Some("[1.21,)"));
        assert_eq!(entry.authors, vec!["dwin", "alice"]);
        // jei 是 mod 依赖；minecraft 已被过滤
        assert_eq!(entry.dependencies.len(), 1);
        assert_eq!(entry.dependencies[0].mod_id, "jei");
        assert!(!entry.dependencies[0].mandatory);
        assert_eq!(entry.dependencies[0].side, Side::Client);
        let _ = std::fs::remove_file(&jar);
    }

    #[test]
    fn parse_neoforge_mods_toml_priority() {
        // 同时含 mods.toml 和 neoforge.mods.toml 时优先后者
        let jar = make_jar(&[
            ("META-INF/mods.toml", FORGE_TOML),
            ("META-INF/neoforge.mods.toml", FORGE_TOML),
        ]);
        let entry = parse_jar(&jar).expect("parse");
        assert_eq!(entry.loader, LoaderKind::NeoForge);
        let _ = std::fs::remove_file(&jar);
    }

    #[test]
    fn parse_fabric_mod_json() {
        let jar = make_jar(&[("fabric.mod.json", FABRIC_JSON)]);
        let entry = parse_jar(&jar).expect("parse");
        assert_eq!(entry.mod_id, "examplefabric");
        assert_eq!(entry.name, "Example Fabric Mod");
        assert_eq!(entry.version, "0.5.0");
        assert_eq!(entry.loader, LoaderKind::Fabric);
        assert_eq!(entry.mc_version_range.as_deref(), Some(">=1.21.1"));
        assert_eq!(entry.authors, vec!["dwin", "alice"]);
        assert_eq!(entry.side, Side::Client);
        // minecraft / fabricloader 已过滤; fabric-api 保留
        assert_eq!(entry.dependencies.len(), 1);
        assert_eq!(entry.dependencies[0].mod_id, "fabric-api");
        let _ = std::fs::remove_file(&jar);
    }

    #[test]
    fn parse_unrecognized_jar_returns_not_a_mod() {
        let jar = make_jar(&[("META-INF/MANIFEST.MF", "Manifest-Version: 1.0")]);
        match parse_jar(&jar) {
            Err(ParseError::NotAMod) => {}
            other => panic!("expected NotAMod, got {:?}", other),
        }
        let _ = std::fs::remove_file(&jar);
    }

    #[test]
    fn parse_disabled_jar_marks_disabled() {
        let dir = std::env::temp_dir().join(format!("ncl-mod-disabled-{}", uuid_like()));
        std::fs::create_dir_all(&dir).unwrap();
        let jar = dir.join("examplemod-1.0.jar.disabled");
        let _ = std::fs::remove_file(&jar);
        // 重命名一个 jar 进去
        let original = make_jar(&[("META-INF/mods.toml", FORGE_TOML)]);
        std::fs::rename(&original, &jar).unwrap();

        let entry = parse_jar(&jar).expect("parse");
        assert!(!entry.enabled, "disabled jar should be marked disabled");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
