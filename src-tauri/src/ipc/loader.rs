//! Mod loader 相关 IPC：版本列表 + 一键安装。
//! Sprint 3a Fabric / 3b Forge / 4b NeoForge。

use crate::ipc::sink::TauriEventSink;
use crate::AppState;
use ncl_core::progress::ProgressSink;
use ncl_core::MirrorPolicy;
use ncl_java::{scan_all_with, select_best, JavaRuntime};
use ncl_loader::forge::{install_forge, list_versions as list_forge_versions};
use ncl_loader::neoforge::{install_neoforge, list_versions as list_neoforge_versions};
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
        LoaderKind::Forge => list_forge_versions(&client, &mc_version)
            .await
            .map(|v| v.into_iter().map(LoaderVersionDto::from).collect())
            .map_err(|e| e.to_string()),
        LoaderKind::NeoForge => list_neoforge_versions(&client, &mc_version)
            .await
            .map(|v| v.into_iter().map(LoaderVersionDto::from).collect())
            .map_err(|e| e.to_string()),
    }
}

#[tauri::command]
pub async fn loader_install(
    app: AppHandle,
    state: State<'_, AppState>,
    kind: LoaderKind,
    mc_version: String,
    loader_version: String,
    instance_name: String,
) -> Result<String, String> {
    match kind {
        LoaderKind::Fabric => {
            install_fabric_dispatch(app, state, mc_version, loader_version, instance_name).await
        }
        LoaderKind::Forge => {
            install_installer_dispatch(
                app,
                state,
                LoaderKind::Forge,
                mc_version,
                loader_version,
                instance_name,
            )
            .await
        }
        LoaderKind::NeoForge => {
            install_installer_dispatch(
                app,
                state,
                LoaderKind::NeoForge,
                mc_version,
                loader_version,
                instance_name,
            )
            .await
        }
    }
}

async fn install_fabric_dispatch(
    app: AppHandle,
    state: State<'_, AppState>,
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

    let raw = FabricLoader::fetch_profile(&client, &mc_version, &loader_version)
        .await
        .map_err(|e| format!("fetch fabric profile: {e}"))?;
    let list = fetch_version_list(&client, Some(&pool))
        .await
        .map_err(|e| format!("fetch vanilla manifest: {e}"))?;
    let resolved = resolve_inherits(&client, Some(&pool), raw, &list)
        .await
        .map_err(|e| format!("resolve inherits: {e}"))?;
    let merged_id = resolved.id.clone();

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
            Ok(()) => tracing::info!(version = %resolved.id, "fabric install finished"),
            Err(e) => tracing::error!(%e, "fabric install failed"),
        }
    });

    Ok(merged_id)
}

async fn install_installer_dispatch(
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

    // 拉 vanilla manifest 列表
    let list = fetch_version_list(&client, Some(&pool))
        .await
        .map_err(|e| format!("fetch vanilla manifest: {e}"))?;

    // 选 Java
    let required_major = required_java_for_mc(&mc_version);
    let extra = vec![layout.data_root.clone()];
    let candidates = scan_all_with(&extra).await;
    let java: JavaRuntime = select_best(&candidates, required_major)
        .ok_or_else(|| {
            format!("未发现满足 Java {required_major}+ 的 JRE。请安装 JDK {required_major} 后重试。")
        })?
        .clone();

    let merged_id = match kind {
        LoaderKind::Forge => format!("{mc_version}-forge-{loader_version}"),
        LoaderKind::NeoForge => format!("{mc_version}-neoforge-{loader_version}"),
        LoaderKind::Fabric => unreachable!("handled by install_fabric_dispatch"),
    };
    let merged_id_for_task = merged_id.clone();

    let app_clone = app.clone();
    tokio::spawn(async move {
        let sink: Arc<dyn ProgressSink> = Arc::new(TauriEventSink::new(app_clone.clone()));
        let downloader = match Downloader::new() {
            Ok(d) => Arc::new(ResilientDownloader::new(d, pool.clone(), 4)),
            Err(e) => {
                tracing::error!(%e, "build downloader failed");
                return;
            }
        };

        let result = match kind {
            LoaderKind::Forge => {
                install_forge(
                    &mc_version,
                    &loader_version,
                    &instance_name,
                    &layout,
                    &list,
                    &java,
                    downloader,
                    sink.clone(),
                    concurrency,
                )
                .await
            }
            LoaderKind::NeoForge => {
                install_neoforge(
                    &mc_version,
                    &loader_version,
                    &instance_name,
                    &layout,
                    &list,
                    &java,
                    downloader,
                    sink.clone(),
                    concurrency,
                )
                .await
            }
            LoaderKind::Fabric => unreachable!(),
        };

        match result {
            Ok(out) => tracing::info!(merged = %out.merged_version_id, "loader install finished"),
            Err(e) => {
                tracing::error!(%e, "loader install failed");
                let _ = sink
                    .emit(ncl_core::progress::ProgressEvent::TaskFinished {
                        task_id: format!("loader-install-{}", merged_id_for_task),
                        success: false,
                        error: Some(e.to_string()),
                    })
                    .await;
            }
        }
    });

    Ok(merged_id)
}

fn required_java_for_mc(mc_version: &str) -> u8 {
    match mc_version {
        v if v.starts_with("1.16") => 8,
        v if v.starts_with("1.17") => 16,
        v if v.starts_with("1.20.5")
            || v.starts_with("1.20.6")
            || v.starts_with("1.21") =>
        {
            21
        }
        _ => 17,
    }
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
