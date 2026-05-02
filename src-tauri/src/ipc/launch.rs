//! 启动相关 IPC：根据已安装的 manifest + 选定 Java 启动 Minecraft 进程。

use crate::ipc::sink::TauriEventSink;
use crate::AppState;
use ncl_core::model::ResolvedManifest;
use ncl_java::{detect_version, required_java_major, scan_all, select_best};
use ncl_launch::{build_plan, spawn, AccountInfo, LaunchInputs, MemorySpec};
use ncl_vanilla::InstallPaths;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchRequest {
    pub instance_name: String,
    pub version_id: String,
    pub username: String,
    /// 可选：用户手动指定 Java 路径；否则自动选 best
    pub java_path: Option<String>,
    /// 可选：内存配置（MB）；缺省 512 / 4096
    pub min_mem_mb: Option<u32>,
    pub max_mem_mb: Option<u32>,
    /// 可选：附加 JVM 参数
    #[serde(default)]
    pub jvm_args_extra: Vec<String>,
    /// 可选：附加游戏参数
    #[serde(default)]
    pub game_args_extra: Vec<String>,
}

/// 启动 Minecraft 进程，返回 PID。stdout/stderr 通过 `ncl://progress` 事件
/// 转发为 `Log` 事件；进程退出会触发 `ncl://process_exit` 事件。
#[tauri::command]
pub async fn launch_run(
    app: AppHandle,
    state: State<'_, AppState>,
    req: LaunchRequest,
) -> Result<u32, String> {
    let layout = state.layout.clone();
    let paths = InstallPaths::new(&layout, &req.instance_name, &req.version_id);

    // 读取已持久化的 ResolvedManifest
    let manifest_path = paths.version_json();
    let manifest_bytes = tokio::fs::read(&manifest_path)
        .await
        .map_err(|e| format!("read {}: {e}", manifest_path.display()))?;
    let manifest: ResolvedManifest =
        serde_json::from_slice(&manifest_bytes).map_err(|e| format!("parse manifest: {e}"))?;

    // 选 Java
    let java = if let Some(p) = req.java_path {
        detect_version(&PathBuf::from(p))
            .await
            .map_err(|e| format!("detect java: {e}"))?
    } else {
        let candidates = scan_all().await;
        let required = required_java_major(&manifest);
        select_best(&candidates, required)
            .ok_or_else(|| {
                format!(
                    "no compatible Java >= {required} found; please install or set JAVA_HOME"
                )
            })?
            .clone()
    };

    let account = AccountInfo::offline(&req.username);
    let memory = MemorySpec {
        min_mb: req.min_mem_mb.unwrap_or(512),
        max_mb: req.max_mem_mb.unwrap_or(4096),
    };
    let inputs = LaunchInputs {
        manifest: &manifest,
        paths: &paths,
        java: &java,
        account: &account,
        memory,
        jvm_args_extra: req.jvm_args_extra,
        game_args_extra: req.game_args_extra,
        features: HashMap::new(),
    };
    let plan = build_plan(&inputs).map_err(|e| format!("build plan: {e}"))?;

    let sink = Arc::new(TauriEventSink::new(app.clone()));
    let handle = spawn(plan, sink).await.map_err(|e| format!("spawn: {e}"))?;
    let pid = handle.pid;

    // 后台等待退出 + 上抛 process_exit
    let app_clone = app.clone();
    tokio::spawn(async move {
        let exit = handle.wait().await;
        let payload = serde_json::json!({
            "pid": pid,
            "exit_code": exit.unwrap_or(None),
        });
        if let Err(e) = app_clone.emit("ncl://process_exit", payload) {
            tracing::warn!(?e, "failed to emit ncl://process_exit");
        }
    });

    Ok(pid)
}
