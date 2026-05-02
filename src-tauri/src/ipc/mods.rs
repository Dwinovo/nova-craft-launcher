//! Mod 管理 IPC：扫描实例的 mods/ 目录 + 启用/禁用切换。

use crate::AppState;
use ncl_core::{LoaderKind, ModEntry, Side};
use ncl_mod::{scan_mods, set_enabled};
use serde::Serialize;
use std::path::PathBuf;
use tauri::State;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModEntryDto {
    pub file_path: String,
    pub enabled: bool,
    pub loader: String,
    pub mod_id: String,
    pub name: String,
    pub version: String,
    pub mc_version_range: Option<String>,
    pub authors: Vec<String>,
    pub description: Option<String>,
    pub side: String,
    pub dependencies: Vec<DepDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DepDto {
    pub mod_id: String,
    pub version_range: Option<String>,
    pub mandatory: bool,
    pub side: String,
}

impl From<ModEntry> for ModEntryDto {
    fn from(m: ModEntry) -> Self {
        Self {
            file_path: m.file_path.display().to_string(),
            enabled: m.enabled,
            loader: loader_str(m.loader),
            mod_id: m.mod_id,
            name: m.name,
            version: m.version,
            mc_version_range: m.mc_version_range,
            authors: m.authors,
            description: m.description,
            side: side_str(m.side),
            dependencies: m
                .dependencies
                .into_iter()
                .map(|d| DepDto {
                    mod_id: d.mod_id,
                    version_range: d.version_range,
                    mandatory: d.mandatory,
                    side: side_str(d.side),
                })
                .collect(),
        }
    }
}

fn loader_str(k: LoaderKind) -> String {
    match k {
        LoaderKind::Forge => "forge",
        LoaderKind::Fabric => "fabric",
        LoaderKind::NeoForge => "neoforge",
    }
    .into()
}

fn side_str(s: Side) -> String {
    match s {
        Side::Client => "client",
        Side::Server => "server",
        Side::Both => "both",
    }
    .into()
}

/// 扫描实例的 mods/ 目录。
#[tauri::command]
pub async fn mod_scan(
    state: State<'_, AppState>,
    instance_name: String,
) -> Result<Vec<ModEntryDto>, String> {
    let mods_dir = state
        .layout
        .instance(&instance_name)
        .join(".minecraft")
        .join("mods");
    let entries = scan_mods(&mods_dir).map_err(|e| e.to_string())?;
    Ok(entries.into_iter().map(ModEntryDto::from).collect())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToggleResultDto {
    pub from: String,
    pub to: String,
    pub now_enabled: bool,
}

/// 切换 mod 启用状态。`file_path` 必须是当前 mods/ 目录中存在的 jar。
#[tauri::command]
pub async fn mod_set_enabled(
    file_path: String,
    enabled: bool,
) -> Result<ToggleResultDto, String> {
    let path = PathBuf::from(file_path);
    let result = set_enabled(&path, enabled).map_err(|e| e.to_string())?;
    Ok(ToggleResultDto {
        from: result.from.display().to_string(),
        to: result.to.display().to_string(),
        now_enabled: result.now_enabled,
    })
}
