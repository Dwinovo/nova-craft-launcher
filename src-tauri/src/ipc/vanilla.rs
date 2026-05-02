//! Vanilla 相关 IPC：版本列表 + 一键安装。

use crate::ipc::sink::TauriEventSink;
use crate::AppState;
use ncl_core::model::VersionListEntry;
use ncl_core::MirrorPolicy;
use ncl_net::{BmclApiSource, Downloader, MirrorPool, OfficialSource, ResilientDownloader};
use ncl_vanilla::{
    default_client, fetch_version_detail, fetch_version_list, install, resolve_inherits,
    InstallPaths,
};
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionEntry {
    pub id: String,
    pub kind: String,
    pub release_time: String,
    pub url: String,
}

impl From<&VersionListEntry> for VersionEntry {
    fn from(e: &VersionListEntry) -> Self {
        let kind = match e.kind {
            ncl_core::model::VersionType::Release => "release",
            ncl_core::model::VersionType::Snapshot => "snapshot",
            ncl_core::model::VersionType::OldBeta => "old_beta",
            ncl_core::model::VersionType::OldAlpha => "old_alpha",
        }
        .to_string();
        Self {
            id: e.id.clone(),
            kind,
            release_time: e.release_time.clone(),
            url: e.url.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionListResponse {
    pub latest_release: String,
    pub latest_snapshot: String,
    pub versions: Vec<VersionEntry>,
}

/// 拉取 Mojang 版本清单。`release_only=true` 时只返回 type=release。
#[tauri::command]
pub async fn vanilla_list_versions(
    state: State<'_, AppState>,
    release_only: Option<bool>,
) -> Result<VersionListResponse, String> {
    let release_only = release_only.unwrap_or(true);
    let policy = state.config.read().await.mirror_policy;
    let pool = build_pool(policy);

    let client = default_client().map_err(|e| e.to_string())?;
    let list = fetch_version_list(&client, Some(&pool))
        .await
        .map_err(|e| e.to_string())?;
    let versions: Vec<VersionEntry> = list
        .versions
        .iter()
        .filter(|v| {
            !release_only || matches!(v.kind, ncl_core::model::VersionType::Release)
        })
        .map(VersionEntry::from)
        .collect();
    Ok(VersionListResponse {
        latest_release: list.latest.release,
        latest_snapshot: list.latest.snapshot,
        versions,
    })
}

/// 启动一次安装任务，立即返回 task_id。实际工作在后台执行，进度通过
/// `ncl://progress` 事件汇报给前端。
#[tauri::command]
pub async fn vanilla_install(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_name: String,
    version_id: String,
) -> Result<String, String> {
    let task_id = uuid::Uuid::new_v4().to_string();
    let layout = state.layout.clone();
    let cfg = state.config.read().await;
    let concurrency = cfg.max_concurrent_downloads;
    let policy = cfg.mirror_policy;
    drop(cfg);

    let app_clone = app.clone();
    tokio::spawn(async move {
        let sink = Arc::new(TauriEventSink::new(app_clone.clone()));
        match run_install(layout, policy, instance_name, version_id, sink, concurrency).await {
            Ok(()) => tracing::info!("vanilla install finished"),
            Err(e) => tracing::error!(%e, "vanilla install failed"),
        }
    });

    Ok(task_id)
}

async fn run_install(
    layout: ncl_core::PathLayout,
    policy: MirrorPolicy,
    instance_name: String,
    version_id: String,
    sink: Arc<dyn ncl_core::progress::ProgressSink>,
    concurrency: usize,
) -> ncl_core::Result<()> {
    let pool = Arc::new(build_pool(policy));
    let client = default_client()?;

    let list = fetch_version_list(&client, Some(&pool)).await?;
    let entry = list
        .versions
        .iter()
        .find(|v| v.id == version_id)
        .ok_or_else(|| {
            ncl_core::Error::NotFound(format!("version '{version_id}' not in manifest"))
        })?;
    let raw = fetch_version_detail(&client, Some(&pool), &entry.url).await?;
    let resolved = resolve_inherits(&client, Some(&pool), raw, &list).await?;

    let downloader = Arc::new(ResilientDownloader::new(
        Downloader::new()?,
        pool.clone(),
        4, // 最多跨镜像重试 4 次
    ));
    let paths = InstallPaths::new(&layout, &instance_name, &resolved.id);
    install(&resolved, &paths, downloader, sink, concurrency).await
}

/// 根据 [`MirrorPolicy`] 构建一个 [`MirrorPool`]。`Auto` / `Bmclapi` 都把
/// `OfficialSource` 留作兜底（避免 BMCLAPI 不接管的 URL 漏掉）。
fn build_pool(policy: MirrorPolicy) -> MirrorPool {
    match policy {
        MirrorPolicy::Auto | MirrorPolicy::Bmclapi => MirrorPool::new(vec![
            Arc::new(BmclApiSource),
            Arc::new(OfficialSource),
        ]),
        MirrorPolicy::Official => MirrorPool::new(vec![Arc::new(OfficialSource)]),
    }
}
