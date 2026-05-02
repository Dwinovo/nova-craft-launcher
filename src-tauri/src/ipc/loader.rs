//! Mod loader 相关 IPC：版本列表 + 一键安装（Sprint 3a 仅 Fabric）。

use crate::ipc::sink::TauriEventSink;
use crate::AppState;
use ncl_core::progress::ProgressSink;
use ncl_core::MirrorPolicy;
use ncl_loader::{FabricLoader, LoaderKind, LoaderVersion};
use ncl_net::{BmclApiSource, Downloader, MirrorPool, OfficialSource, ResilientDownloader};
use ncl_vanilla::{
    default_client, fetch_version_list, install as vanilla_install, resolve_inherits,
    InstallPaths,
};
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoaderVersionDto {
    pub version: String,
    pub stable: bool,
}

impl From<LoaderVersion> for LoaderVersionDto {
    fn from(v: LoaderVersion) -> Self {
        Self {
            version: v.version,
            stable: v.stable,
        }
    }
}

/// 列出某 MC 版本可用的 loader 版本。Sprint 3a 只支持 Fabric。
#[tauri::command]
pub async fn loader_list_versions(
    kind: LoaderKind,
    mc_version: String,
) -> Result<Vec<LoaderVersionDto>, String> {
    let client = default_client().map_err(|e| e.to_string())?;
    match kind {
        LoaderKind::Fabric => FabricLoader::list_versions(&client, &mc_version)
            .await
            .map(|v| v.into_iter().map(LoaderVersionDto::from).collect())
            .map_err(|e| e.to_string()),
        LoaderKind::Forge => Err("Forge 加载器尚未实现 (Sprint 3b)".to_string()),
        LoaderKind::NeoForge => Err("NeoForge 加载器尚未实现 (Sprint 4)".to_string()),
    }
}

/// 启动 loader 安装：前台拉 profile + 解析 inherits 立即返回 `merged_version_id`
/// （前端用此 id 后续传给 `launch_run`），后台跑下载流水线，进度通过
/// `ncl://progress` 事件汇报。
#[tauri::command]
pub async fn loader_install(
    app: AppHandle,
    state: State<'_, AppState>,
    kind: LoaderKind,
    mc_version: String,
    loader_version: String,
    instance_name: String,
) -> Result<String, String> {
    let layout = state.layout.clone();
    let cfg = state.config.read().await;
    let concurrency = cfg.max_concurrent_downloads;
    let policy = cfg.mirror_policy;
    drop(cfg);

    let pool = Arc::new(build_pool(policy));
    let client = default_client().map_err(|e| e.to_string())?;

    // 前台：拉 profile + manifest 列表 + 合并 inherits → 拿到 merged_version_id
    let raw = match kind {
        LoaderKind::Fabric => FabricLoader::fetch_profile(&client, &mc_version, &loader_version)
            .await
            .map_err(|e| format!("fetch fabric profile: {e}"))?,
        LoaderKind::Forge => {
            return Err("Forge installer 尚未实现 (Sprint 3b)".to_string())
        }
        LoaderKind::NeoForge => {
            return Err("NeoForge installer 尚未实现 (Sprint 4)".to_string())
        }
    };
    let list = fetch_version_list(&client, Some(&pool))
        .await
        .map_err(|e| format!("fetch vanilla manifest: {e}"))?;
    let resolved = resolve_inherits(&client, Some(&pool), raw, &list)
        .await
        .map_err(|e| format!("resolve inherits: {e}"))?;
    let merged_id = resolved.id.clone();

    // 后台：执行下载流水线
    let app_clone = app.clone();
    tokio::spawn(async move {
        let sink: Arc<dyn ProgressSink> = Arc::new(TauriEventSink::new(app_clone));
        let downloader = match Downloader::new() {
            Ok(d) => Arc::new(ResilientDownloader::new(d, pool.clone(), 4)),
            Err(e) => {
                tracing::error!(%e, "build downloader failed");
                return;
            }
        };
        let paths = InstallPaths::new(&layout, &instance_name, &resolved.id);
        match vanilla_install(&resolved, &paths, downloader, sink, concurrency).await {
            Ok(()) => tracing::info!(version = %resolved.id, "loader install finished"),
            Err(e) => tracing::error!(%e, "loader install failed"),
        }
    });

    Ok(merged_id)
}

fn build_pool(policy: MirrorPolicy) -> MirrorPool {
    match policy {
        MirrorPolicy::Auto | MirrorPolicy::Bmclapi => MirrorPool::new(vec![
            Arc::new(BmclApiSource),
            Arc::new(OfficialSource),
        ]),
        MirrorPolicy::Official => MirrorPool::new(vec![Arc::new(OfficialSource)]),
    }
}
