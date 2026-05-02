//! Forge installer 顶层流程。
//!
//! 步骤：
//! 1. 下载 installer.jar 到 cache/
//! 2. 解压 install_profile.json + version.json
//! 3. 把 installer 内嵌的 maven/ 目录释放到 shared/libraries/（avoid 重新下载）
//! 4. 把 installer 的 data/ 目录释放到一个临时工作目录（client.lzma 等被 processor 读取）
//! 5. 解析 install_profile，下载所有需要的库
//! 6. 跑 vanilla install pipeline 拿到 vanilla client.jar / libraries / assets
//! 7. 顺序执行 install_profile.processors[*]（client side）
//! 8. 验证每个 processor 的 outputs SHA1
//! 9. 写入合并 manifest 到 versions/<merged_id>/<merged_id>.json
//! 10. 把 patched client jar 复制为 versions/<merged_id>/<merged_id>.jar（Forge processor
//!     输出的就是 patched client）

use super::api::forge_installer_url;
use super::processor::{
    build_processor_classpath, extract_main_class, resolve_processor_args, CLASSPATH_SEPARATOR,
};
use super::profile::{resolve_token, InstallProfile};
use crate::profile_lib::ProfileLibrary;
use ncl_core::progress::{LogLevel, ProgressEvent, ProgressSink};
use ncl_core::{Error, RawVersion, ResolvedManifest, Result};
use ncl_java::JavaRuntime;
use ncl_net::ResilientDownloader;
use ncl_vanilla::{install as vanilla_install, InstallPaths};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct ForgeInstallOutput {
    pub merged_version_id: String,
    pub resolved_manifest: ResolvedManifest,
    /// patched client.jar 落点（即 versions/<id>/<id>.jar）
    pub patched_client_jar: PathBuf,
}

/// 安装 Forge 整个流程的高层入口。前置：调用方已通过 vanilla manifest 列表
/// 拿到了对应 MC 版本（用于 inheritsFrom 解析）。
#[allow(clippy::too_many_arguments)]
pub async fn install_forge(
    mc_version: &str,
    forge_version: &str,
    instance_name: &str,
    layout: &ncl_core::PathLayout,
    vanilla_list: &ncl_core::VersionList,
    java: &JavaRuntime,
    downloader: Arc<ResilientDownloader>,
    sink: Arc<dyn ProgressSink>,
    concurrency: usize,
) -> Result<ForgeInstallOutput> {
    // 路径准备
    let merged_id = format!("{mc_version}-forge-{forge_version}");
    let install_paths = InstallPaths::new(layout, instance_name, &merged_id);
    let cache_dir = layout.cache().join("forge").join(&merged_id);
    tokio::fs::create_dir_all(&cache_dir)
        .await
        .map_err(|e| Error::io(&cache_dir, e))?;

    // 1. 下载 installer
    let installer_url = forge_installer_url(mc_version, forge_version);
    let installer_path = cache_dir.join("installer.jar");
    log_info(&sink, "forge.install", "下载 Forge installer…").await;
    downloader.fetch(&installer_url, &installer_path, None).await?;

    // 2 + 3 + 4. 解压 installer
    log_info(&sink, "forge.install", "解压 installer…").await;
    let unpacked = unpack_installer(&installer_path, &cache_dir, &install_paths.libraries_dir)?;

    // 5. 解析 install_profile + version_json
    let profile: InstallProfile = serde_json::from_str(&unpacked.install_profile_json)?;
    let version_raw: RawVersion = serde_json::from_str(&unpacked.version_json)?;

    // 6. 解析 inheritsFrom 链（合并 vanilla manifest）
    let mut chain = vec![version_raw.id.clone()];
    let mut current = version_raw;
    while let Some(parent_id) = current.inherits_from.clone() {
        let parent_entry = vanilla_list
            .versions
            .iter()
            .find(|v| v.id == parent_id)
            .ok_or_else(|| Error::NotFound(format!("parent '{parent_id}' not in manifest")))?;
        let pool_client = ncl_vanilla::default_client()?;
        let parent_raw = ncl_vanilla::fetch_version_detail(&pool_client, None, &parent_entry.url).await?;
        chain.push(parent_id);
        current = current.merge_inherits(parent_raw);
    }
    let resolved = current.into_resolved(chain)?;

    // 7. 跑 vanilla install pipeline 下载基础 libraries / assets / client.jar
    log_info(&sink, "forge.install", "下载 vanilla 基础库与 assets…").await;
    vanilla_install(&resolved, &install_paths, downloader.clone(), sink.clone(), concurrency).await?;

    // 8. 下载 install_profile.libraries（Forge processor 自己依赖的库）
    log_info(&sink, "forge.install", "下载 Forge 安装阶段依赖库…").await;
    download_profile_libraries(&profile.libraries, &install_paths.libraries_dir, &downloader)
        .await?;

    // 9. 准备 client.jar 入参（processor 引用 {MINECRAFT_JAR} 时需要）
    let mut processor_data = profile.data.clone();
    processor_data.insert(
        "SIDE".to_string(),
        super::profile::DataEntry {
            client: "client".into(),
            server: "server".into(),
        },
    );
    processor_data.insert(
        "MINECRAFT_JAR".to_string(),
        super::profile::DataEntry {
            client: install_paths.client_jar().to_string_lossy().into_owned(),
            server: String::new(),
        },
    );
    // BINPATCH 路径在 install_profile.data 中是相对路径如 "/data/client.lzma"；
    // 我们把它们指向 cache_dir 内已解压的实际文件
    if let Some(bp) = profile.data.get("BINPATCH") {
        let client_lzma = cache_dir.join(bp.client.trim_start_matches('/'));
        let server_lzma = cache_dir.join(bp.server.trim_start_matches('/'));
        processor_data.insert(
            "BINPATCH".to_string(),
            super::profile::DataEntry {
                client: client_lzma.to_string_lossy().into_owned(),
                server: server_lzma.to_string_lossy().into_owned(),
            },
        );
    }
    // INSTALLER 通常指 installer jar 自身
    processor_data.insert(
        "INSTALLER".to_string(),
        super::profile::DataEntry {
            client: installer_path.to_string_lossy().into_owned(),
            server: installer_path.to_string_lossy().into_owned(),
        },
    );
    // ROOT 是数据根目录
    processor_data.insert(
        "ROOT".to_string(),
        super::profile::DataEntry {
            client: layout.data_root.to_string_lossy().into_owned(),
            server: layout.data_root.to_string_lossy().into_owned(),
        },
    );

    // 10. 顺序跑所有 client-side processors
    let total = profile.processors.iter().filter(|p| p.applies_to_client()).count();
    log_info(
        &sink,
        "forge.install",
        &format!("执行 {total} 个 processors（每个会启动一次 java）…"),
    )
    .await;

    for (idx, processor) in profile
        .processors
        .iter()
        .filter(|p| p.applies_to_client())
        .enumerate()
    {
        let label = format!("processor {}/{total}: {}", idx + 1, processor.jar);
        log_info(&sink, "forge.processor", &label).await;
        run_one_processor(
            processor,
            &processor_data,
            &install_paths.libraries_dir,
            java,
            &cache_dir,
            sink.clone(),
        )
        .await?;
        verify_processor_outputs(processor, &processor_data, &install_paths.libraries_dir)?;
    }

    // 11. 写入合并 manifest（覆盖 vanilla install 已写入的，因为我们要确保 main_class 等取自 forge）
    let manifest_json = serde_json::to_vec_pretty(&resolved)?;
    tokio::fs::write(install_paths.version_json(), &manifest_json)
        .await
        .map_err(|e| Error::io(install_paths.version_json(), e))?;

    Ok(ForgeInstallOutput {
        merged_version_id: merged_id,
        resolved_manifest: resolved,
        patched_client_jar: install_paths.client_jar(),
    })
}

async fn run_one_processor(
    processor: &super::profile::Processor,
    data: &std::collections::HashMap<String, super::profile::DataEntry>,
    libraries_root: &Path,
    java: &JavaRuntime,
    work_dir: &Path,
    sink: Arc<dyn ProgressSink>,
) -> Result<()> {
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::process::Command;

    // 解析 classpath + main class
    let cp_paths = build_processor_classpath(processor, libraries_root);
    let cp_str = cp_paths
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(CLASSPATH_SEPARATOR);

    // jar 本身的 Main-Class
    let processor_jar = super::profile::maven_coord_to_path(libraries_root, &processor.jar);
    let main_class = extract_main_class(&processor_jar)?;

    // 解析 args
    let args = resolve_processor_args(processor, data, libraries_root)?;

    let mut cmd = Command::new(&java.path);
    cmd.arg("-cp");
    cmd.arg(&cp_str);
    cmd.arg(&main_class);
    for a in &args {
        cmd.arg(a);
    }
    cmd.current_dir(work_dir);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW

    let mut child = cmd.spawn().map_err(|e| Error::io(&java.path, e))?;

    if let Some(stdout) = child.stdout.take() {
        let sink2 = sink.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                sink2
                    .emit(ProgressEvent::Log {
                        source: "forge.processor.stdout".into(),
                        level: LogLevel::Debug,
                        message: line,
                    })
                    .await;
            }
        });
    }
    if let Some(stderr) = child.stderr.take() {
        let sink2 = sink.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                sink2
                    .emit(ProgressEvent::Log {
                        source: "forge.processor.stderr".into(),
                        level: LogLevel::Warn,
                        message: line,
                    })
                    .await;
            }
        });
    }

    let status = child
        .wait()
        .await
        .map_err(|e| Error::Task(format!("processor wait: {e}")))?;
    if !status.success() {
        return Err(Error::Task(format!(
            "processor failed with exit {:?}: jar={} main={}",
            status.code(),
            processor.jar,
            main_class
        )));
    }
    Ok(())
}

fn verify_processor_outputs(
    processor: &super::profile::Processor,
    data: &std::collections::HashMap<String, super::profile::DataEntry>,
    libraries_root: &Path,
) -> Result<()> {
    for (path_token, expected_sha) in &processor.outputs {
        let resolved_path = resolve_token(path_token, data, libraries_root)?;
        let actual_sha = ncl_net::sha1_file(Path::new(&resolved_path))?;
        let resolved_sha = resolve_token(expected_sha, data, libraries_root)?;
        if !actual_sha.eq_ignore_ascii_case(&resolved_sha) {
            return Err(Error::ChecksumMismatch {
                expected: resolved_sha,
                actual: actual_sha,
            });
        }
    }
    Ok(())
}

#[derive(Debug)]
struct UnpackedInstaller {
    install_profile_json: String,
    version_json: String,
}

/// 解压 installer jar：
/// - 提取 install_profile.json + version.json 内容（返回字符串）
/// - 把 maven/ 路径下的所有文件复制到 libraries_root（保留路径）
/// - 把 data/ 路径下的所有文件复制到 cache_dir/data/（processor 通过 BINPATCH 引用）
fn unpack_installer(
    jar_path: &Path,
    cache_dir: &Path,
    libraries_root: &Path,
) -> Result<UnpackedInstaller> {
    use std::io::Read;

    let file = std::fs::File::open(jar_path).map_err(|e| Error::io(jar_path, e))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| Error::Task(format!("open installer: {e}")))?;

    let mut install_profile_json = None;
    let mut version_json = None;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| Error::Task(format!("read zip entry: {e}")))?;
        let name = entry.name().to_string();
        if entry.is_dir() {
            continue;
        }
        if name == "install_profile.json" {
            let mut s = String::new();
            entry.read_to_string(&mut s).map_err(|e| Error::io(jar_path, e))?;
            install_profile_json = Some(s);
        } else if name == "version.json" {
            let mut s = String::new();
            entry.read_to_string(&mut s).map_err(|e| Error::io(jar_path, e))?;
            version_json = Some(s);
        } else if let Some(rest) = name.strip_prefix("maven/") {
            let target = libraries_root.join(rest);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
            }
            // 不覆盖已有
            if !target.exists() {
                let mut out = std::fs::File::create(&target).map_err(|e| Error::io(&target, e))?;
                std::io::copy(&mut entry, &mut out).map_err(|e| Error::io(&target, e))?;
            }
        } else if name.starts_with("data/") {
            let target = cache_dir.join(&name);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
            }
            let mut out = std::fs::File::create(&target).map_err(|e| Error::io(&target, e))?;
            std::io::copy(&mut entry, &mut out).map_err(|e| Error::io(&target, e))?;
        }
    }

    Ok(UnpackedInstaller {
        install_profile_json: install_profile_json
            .ok_or_else(|| Error::NotFound("install_profile.json".into()))?,
        version_json: version_json.ok_or_else(|| Error::NotFound("version.json".into()))?,
    })
}

async fn download_profile_libraries(
    libs: &[ProfileLibrary],
    libraries_root: &Path,
    downloader: &ResilientDownloader,
) -> Result<()> {
    for lib in libs {
        if let Some(downloads) = &lib.downloads {
            if let Some(art) = &downloads.artifact {
                let target = libraries_root.join(&art.path);
                downloader.fetch(&art.url, &target, Some(&art.sha1)).await?;
            }
        }
    }
    Ok(())
}

async fn log_info(sink: &Arc<dyn ProgressSink>, source: &str, message: &str) {
    sink.emit(ProgressEvent::Log {
        source: source.to_string(),
        level: LogLevel::Info,
        message: message.to_string(),
    })
    .await;
}
