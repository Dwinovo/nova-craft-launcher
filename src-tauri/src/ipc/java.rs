//! Java 相关 IPC：扫描本机 Java + 推荐选择。

use ncl_java::{scan_all, JavaRuntime};
use serde::Serialize;

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

/// 扫描本机所有 Java 运行时。MVP：JAVA_HOME + PATH。
#[tauri::command]
pub async fn java_scan() -> Result<Vec<JavaInfo>, String> {
    let runtimes = scan_all().await;
    Ok(runtimes.into_iter().map(JavaInfo::from).collect())
}
