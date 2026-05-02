//! 一键安装流水线：把 [`ResolvedManifest`] 拍平为 [`Pipeline`]，下载 client.jar /
//! libraries / asset_index / assets，并解压 native 库。
//!
//! 安装产物：
//! - `<root>/shared/versions/<id>/<id>.json` 持久化的 ResolvedManifest（启动时直接读）
//! - `<root>/shared/versions/<id>/<id>.jar` client.jar
//! - `<root>/shared/versions/<id>/natives/` native 解压目录
//! - `<root>/shared/libraries/...` Maven 风格的库
//! - `<root>/shared/assets/indexes/<id>.json` + `objects/<prefix>/<hash>` assets
//!
//! 流水线由四个 stage 组成（顺序执行，stage 内并行）：
//! 1. `prepare` 写入 ResolvedManifest JSON + 拉取 asset_index（同步序列化点）
//! 2. `download core+libs` client.jar 与所有 library / native artifact
//! 3. `download assets` 所有 asset object（数千个小文件）
//! 4. `extract natives` 解压 native jar 到 natives/

use crate::library::{library_artifacts, ArtifactKind};
use crate::paths::InstallPaths;
use async_trait::async_trait;
use ncl_core::model::{AssetIndex, EvalContext, ResolvedManifest};
use ncl_core::progress::ProgressSink;
use ncl_core::{Error, Result};
use ncl_net::Downloader;
use ncl_task::{Pipeline, Stage, Task};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// 执行完整安装。前置：调用方已通过 `manifest::resolve_inherits` 拿到合并后的 manifest。
pub async fn install(
    resolved: &ResolvedManifest,
    paths: &InstallPaths,
    downloader: Arc<Downloader>,
    sink: Arc<dyn ProgressSink>,
    concurrency: usize,
) -> Result<()> {
    let ctx = EvalContext::default();

    // ---- Phase 0：写入 manifest JSON + 拉 asset_index（同步序列化点） ----
    tokio::fs::create_dir_all(&paths.version_dir)
        .await
        .map_err(|e| Error::io(&paths.version_dir, e))?;
    let manifest_json = serde_json::to_vec_pretty(resolved)?;
    tokio::fs::write(paths.version_json(), &manifest_json)
        .await
        .map_err(|e| Error::io(paths.version_json(), e))?;

    let asset_index_path = paths.asset_index_file(&resolved.asset_index.id);
    downloader
        .fetch(
            &resolved.asset_index.url,
            &asset_index_path,
            Some(&resolved.asset_index.sha1),
        )
        .await?;
    let asset_index_bytes = tokio::fs::read(&asset_index_path)
        .await
        .map_err(|e| Error::io(&asset_index_path, e))?;
    let asset_index: AssetIndex = serde_json::from_slice(&asset_index_bytes)?;

    tracing::info!(
        version = %resolved.id,
        libs = resolved.libraries.len(),
        assets = asset_index.objects.len(),
        "vanilla install: pipeline composing"
    );

    // ---- Phase 1：构建 stage A — client.jar + libraries（含 natives） ----
    let mut stage_core = Stage::new("download client + libraries");
    let mut natives_to_extract: Vec<NativeRef> = Vec::new();

    if let Some(client) = &resolved.downloads.client {
        stage_core = stage_core.push(Arc::new(DownloadTask {
            label: format!("client.jar {}", resolved.id),
            url: client.url.clone(),
            target: paths.client_jar(),
            sha1: Some(client.sha1.clone()),
            weight: 50,
            downloader: downloader.clone(),
        }));
    }

    for lib in &resolved.libraries {
        for artifact in library_artifacts(lib, &ctx, &paths.libraries_dir) {
            let label = format!("library {}", lib.name);
            let target = artifact.target.clone();
            stage_core = stage_core.push(Arc::new(DownloadTask {
                label,
                url: artifact.url.clone(),
                target: target.clone(),
                sha1: Some(artifact.sha1.clone()),
                weight: 5,
                downloader: downloader.clone(),
            }));
            if let ArtifactKind::Native { excludes } = artifact.kind {
                natives_to_extract.push(NativeRef {
                    jar: target,
                    excludes,
                });
            }
        }
        if lib.downloads.is_none() && lib.url.is_none() {
            tracing::warn!(name = %lib.name, "library has no downloads and no url; skipped");
        }
    }

    // ---- Phase 2：构建 stage B — assets ----
    let mut stage_assets = Stage::new("download assets");
    for (_name, obj) in &asset_index.objects {
        stage_assets = stage_assets.push(Arc::new(DownloadTask {
            label: format!("asset {}", &obj.hash[..8]),
            url: obj.official_url(),
            target: paths.asset_object(obj),
            sha1: Some(obj.hash.clone()),
            weight: 1,
            downloader: downloader.clone(),
        }));
    }

    // ---- Phase 3：构建 stage C — extract natives ----
    let natives_dir = paths.natives_dir();
    let mut stage_natives = Stage::new("extract natives");
    for nref in natives_to_extract {
        stage_natives = stage_natives.push(Arc::new(ExtractNativesTask {
            jar: nref.jar,
            target_dir: natives_dir.clone(),
            excludes: nref.excludes,
        }));
    }

    // ---- 拼装流水线 ----
    let pipeline = Pipeline::new(format!("install {}", resolved.id), concurrency)
        .add_stage(stage_core)
        .add_stage(stage_assets)
        .add_stage(stage_natives);

    pipeline.execute(sink).await
}

#[derive(Debug, Clone)]
struct NativeRef {
    jar: PathBuf,
    excludes: Vec<String>,
}

/// 单文件下载任务。
struct DownloadTask {
    label: String,
    url: String,
    target: PathBuf,
    sha1: Option<String>,
    weight: u64,
    downloader: Arc<Downloader>,
}

#[async_trait]
impl Task for DownloadTask {
    fn label(&self) -> &str {
        &self.label
    }
    fn weight(&self) -> u64 {
        self.weight
    }
    async fn run(&self) -> Result<()> {
        self.downloader
            .fetch(&self.url, &self.target, self.sha1.as_deref())
            .await
    }
}

/// 解压 native jar 到 target_dir，跳过 `excludes` 列出的路径前缀。
struct ExtractNativesTask {
    jar: PathBuf,
    target_dir: PathBuf,
    excludes: Vec<String>,
}

#[async_trait]
impl Task for ExtractNativesTask {
    fn label(&self) -> &str {
        "extract natives"
    }
    fn weight(&self) -> u64 {
        2
    }
    async fn run(&self) -> Result<()> {
        let jar = self.jar.clone();
        let target = self.target_dir.clone();
        let excludes = self.excludes.clone();
        tokio::task::spawn_blocking(move || extract_zip(&jar, &target, &excludes))
            .await
            .map_err(|e| Error::Task(format!("extract natives spawn: {e}")))?
    }
}

fn extract_zip(jar: &Path, target_dir: &Path, excludes: &[String]) -> Result<()> {
    use std::fs;
    use std::io::copy;

    let file = fs::File::open(jar).map_err(|e| Error::io(jar, e))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| Error::Task(format!("open zip {jar:?}: {e}")))?;

    fs::create_dir_all(target_dir).map_err(|e| Error::io(target_dir, e))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| Error::Task(format!("read entry {i}: {e}")))?;
        let entry_name = entry.name().to_string();
        if entry.is_dir() {
            continue;
        }
        if excludes
            .iter()
            .any(|prefix| entry_name.starts_with(prefix))
        {
            continue;
        }
        let out_path = target_dir.join(&entry_name);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        let mut out_file = fs::File::create(&out_path).map_err(|e| Error::io(&out_path, e))?;
        copy(&mut entry, &mut out_file).map_err(|e| Error::io(&out_path, e))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;
    use zip::write::SimpleFileOptions;

    /// 用 `zip` crate 写一个最小测试 archive，验证解压 + excludes 跳过逻辑。
    #[test]
    fn extract_zip_skips_excluded_prefix() {
        let tmp = std::env::temp_dir().join(format!("ncl-extract-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let zip_path = tmp.join("test.jar");
        let dest = tmp.join("dest");
        {
            let file = std::fs::File::create(&zip_path).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let opts = SimpleFileOptions::default();

            zip.start_file("native.dll", opts).unwrap();
            zip.write_all(b"native bytes").unwrap();

            zip.start_file("META-INF/MANIFEST.MF", opts).unwrap();
            zip.write_all(b"Manifest-Version: 1.0").unwrap();

            zip.finish().unwrap();
        }

        extract_zip(&zip_path, &dest, &["META-INF/".to_string()]).unwrap();
        assert!(dest.join("native.dll").exists());
        assert!(!dest.join("META-INF").exists());

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
