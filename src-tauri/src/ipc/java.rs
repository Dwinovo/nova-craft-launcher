//! Java 相关 IPC：扫描本机 Java + 内存推荐。

use crate::AppState;
use ncl_java::{recommend_memory, scan_all_with, JavaRuntime, MemoryRecommendation};
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaInfo {
    pub path: String,
    pub version_major: u8,
    pub version_full: String,
    pub vendor: String,
    pub arch: String,
    pub source: String,
}

impl From<JavaRuntime> for JavaInfo {
    fn from(rt: JavaRuntime) -> Self {
        Self {
            path: rt.path.display().to_string(),
            version_major: rt.version_major,
            version_full: rt.version_full,
            vendor: rt.vendor,
            arch: serde_json::to_value(&rt.arch)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_else(|| "other".into()),
            source: serde_json::to_value(&rt.source)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_else(|| "manual".into()),
        }
    }
}

/// 扫描本机所有 Java 运行时（多源：JAVA_HOME / PATH / 厂商默认目录 /
/// Windows 注册表 / Mojang JRE）。
#[tauri::command]
pub async fn java_scan(state: State<'_, AppState>) -> Result<Vec<JavaInfo>, String> {
    let extra = vec![state.layout.data_root.clone()];
    let runtimes = scan_all_with(&extra).await;
    Ok(runtimes.into_iter().map(JavaInfo::from).collect())
}

/// 内存推荐。
#[tauri::command]
pub async fn java_recommend_memory(
    has_loader: bool,
    mod_count: usize,
) -> Result<MemoryRecommendation, String> {
    Ok(recommend_memory(has_loader, mod_count))
}
