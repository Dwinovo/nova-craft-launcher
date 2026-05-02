use crate::AppState;
use ncl_core::PathMode;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathInfo {
    pub data_root: String,
    pub mode: PathMode,
    pub instances: String,
    pub shared_assets: String,
    pub shared_libraries: String,
    pub shared_versions: String,
    pub config_file: String,
    pub logs: String,
    pub cache: String,
}

/// 返回当前的目录布局快照，供前端展示与调试。
#[tauri::command]
pub fn core_get_paths(state: State<'_, AppState>) -> Result<PathInfo, String> {
    let layout = &state.layout;
    Ok(PathInfo {
        data_root: layout.data_root.display().to_string(),
        mode: layout.mode,
        instances: layout.instances().display().to_string(),
        shared_assets: layout.shared_assets().display().to_string(),
        shared_libraries: layout.shared_libraries().display().to_string(),
        shared_versions: layout.shared_versions().display().to_string(),
        config_file: layout.config_file().display().to_string(),
        logs: layout.logs().display().to_string(),
        cache: layout.cache().display().to_string(),
    })
}
