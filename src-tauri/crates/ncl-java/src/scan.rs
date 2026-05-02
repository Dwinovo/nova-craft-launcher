//! Java 扫描入口。汇集 JAVA_HOME / PATH / well-known dirs / 注册表 /
//! Mojang JRE 等多源候选，dedupe 后逐一探测版本。

use crate::dirs::{scan_mojang_jre, scan_one_root, well_known_roots};
use crate::model::{Arch, JavaRuntime, JavaSource};
use crate::registry::scan_registry;
use crate::release::parse_release_file;
use ncl_core::{Error, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// 扫所有源。`extra_dirs` 通常传 NCL 数据根目录用于扫描 Mojang JRE
/// （`<data_root>/runtime/...`）；可空。
pub async fn scan_all_with(extra_data_roots: &[PathBuf]) -> Vec<JavaRuntime> {
    let candidates = collect_candidates(extra_data_roots);
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut out = Vec::new();

    for (path, source) in candidates {
        let canonical = canonical_or_self(&path);
        if !seen.insert(canonical.clone()) {
            continue;
        }
        match detect_version(&canonical).await {
            Ok(mut rt) => {
                rt.source = source;
                out.push(rt);
            }
            Err(e) => {
                tracing::debug!(?canonical, %e, "java probe failed; skipped");
            }
        }
    }
    out.sort_by(|a, b| {
        b.version_major
            .cmp(&a.version_major)
            .then_with(|| (a.source as u8).cmp(&(b.source as u8)))
            .then_with(|| a.path.cmp(&b.path))
    });
    out
}

/// 兼容 Sprint 1.4 的 API：仅 JAVA_HOME + PATH。等价于 `scan_all_with(&[])` 的子集。
pub async fn scan_all() -> Vec<JavaRuntime> {
    scan_all_with(&[]).await
}

fn collect_candidates(extra_data_roots: &[PathBuf]) -> Vec<(PathBuf, JavaSource)> {
    let mut v = Vec::new();

    // 1. JAVA_HOME
    if let Some(jh) = std::env::var_os("JAVA_HOME") {
        let p = PathBuf::from(jh).join("bin").join(java_exe_name());
        if p.is_file() {
            v.push((p, JavaSource::JavaHome));
        }
    }

    // 2. PATH
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            let p = dir.join(java_exe_name());
            if p.is_file() {
                v.push((p, JavaSource::Path));
            }
        }
    }

    // 3. 厂商默认目录
    for (root, source) in well_known_roots() {
        for exe in scan_one_root(&root) {
            v.push((exe, source));
        }
    }

    // 4. Windows 注册表
    v.extend(scan_registry());

    // 5. Mojang JRE（NCL 数据根目录下 runtime/）
    for data_root in extra_data_roots {
        for exe in scan_mojang_jre(data_root) {
            v.push((exe, JavaSource::MojangJre));
        }
    }

    v
}

#[cfg(target_os = "windows")]
fn java_exe_name() -> &'static str {
    "java.exe"
}

#[cfg(not(target_os = "windows"))]
fn java_exe_name() -> &'static str {
    "java"
}

fn canonical_or_self(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

/// 探测 `java_path` 指向的 JRE 信息。
///
/// 策略：
/// 1. 先尝试读 `<java_home>/release` 文件（快、无需启动 JVM）
/// 2. 失败则回退到执行 `java -version`
pub async fn detect_version(java_path: &Path) -> Result<JavaRuntime> {
    if let Some(rt) = detect_via_release_file(java_path) {
        return Ok(rt);
    }
    detect_via_command(java_path).await
}

fn detect_via_release_file(java_path: &Path) -> Option<JavaRuntime> {
    // java_path = .../bin/java[.exe] → java_home = parent.parent
    let java_home = java_path.parent()?.parent()?;
    let info = parse_release_file(java_home)?;
    let full = info.java_version.clone()?;
    let major = parse_major(&full);
    let arch = info.arch();
    Some(JavaRuntime {
        path: java_path.to_path_buf(),
        version_major: major,
        version_full: full,
        vendor: info.vendor.unwrap_or_else(|| "Unknown".into()),
        arch,
        source: JavaSource::Manual, // 调用方覆盖
    })
}

async fn detect_via_command(java_path: &Path) -> Result<JavaRuntime> {
    let output = tokio::process::Command::new(java_path)
        .arg("-version")
        .output()
        .await
        .map_err(|e| Error::io(java_path, e))?;
    if !output.status.success() {
        return Err(Error::Task(format!(
            "{} -version exited non-zero: {}",
            java_path.display(),
            output.status
        )));
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    parse_java_version_output(&stderr).map(|info| JavaRuntime {
        path: java_path.to_path_buf(),
        version_major: info.major,
        version_full: info.full,
        vendor: info.vendor,
        arch: info.arch,
        source: JavaSource::Manual,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedVersion {
    major: u8,
    full: String,
    vendor: String,
    arch: Arch,
}

fn parse_java_version_output(output: &str) -> Result<ParsedVersion> {
    let lines: Vec<&str> = output.lines().collect();

    let first = lines
        .first()
        .ok_or_else(|| Error::Task("empty -version output".into()))?;
    let full = extract_quoted(first)
        .ok_or_else(|| Error::Task(format!("cannot parse version line: {first}")))?;
    let major = parse_major(&full);

    let vendor = lines
        .get(1)
        .and_then(|l| l.split(" Runtime Environment").next())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or_else(|| first.split_whitespace().next())
        .unwrap_or("Unknown")
        .to_string();

    let arch = lines
        .get(2)
        .map(|l| {
            if l.contains("64-Bit") {
                Arch::X64
            } else if l.contains("32-Bit") {
                Arch::X86
            } else if l.contains("aarch64") {
                Arch::Arm64
            } else {
                Arch::Other
            }
        })
        .unwrap_or(Arch::Other);

    Ok(ParsedVersion {
        major,
        full,
        vendor,
        arch,
    })
}

fn extract_quoted(line: &str) -> Option<String> {
    let start = line.find('"')?;
    let rest = &line[start + 1..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// `1.8.0_312` → 8；`21.0.2` → 21；`9` → 9。
pub(crate) fn parse_major(version: &str) -> u8 {
    let stripped = version.strip_prefix("1.").unwrap_or(version);
    stripped
        .split(|c: char| c == '.' || c == '_' || c == '-' || c == '+')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_major_handles_legacy_and_modern() {
        assert_eq!(parse_major("21.0.2"), 21);
        assert_eq!(parse_major("1.8.0_312"), 8);
        assert_eq!(parse_major("17.0.5"), 17);
        assert_eq!(parse_major("9"), 9);
        assert_eq!(parse_major("11.0.21+9-LTS"), 11);
    }

    #[test]
    fn parse_temurin_jdk21_output() {
        let output = "openjdk version \"21.0.2\" 2024-01-16 LTS\n\
            OpenJDK Runtime Environment Temurin-21.0.2+13 (build 21.0.2+13-LTS)\n\
            OpenJDK 64-Bit Server VM Temurin-21.0.2+13 (build 21.0.2+13-LTS, mixed mode, sharing)";
        let parsed = parse_java_version_output(output).unwrap();
        assert_eq!(parsed.major, 21);
        assert_eq!(parsed.full, "21.0.2");
        assert!(parsed.vendor.starts_with("OpenJDK"));
        assert_eq!(parsed.arch, Arch::X64);
    }

    #[test]
    fn parse_oracle_jdk8_output() {
        let output = "java version \"1.8.0_312\"\n\
            Java(TM) SE Runtime Environment (build 1.8.0_312-b07)\n\
            Java HotSpot(TM) 64-Bit Server VM (build 25.312-b07, mixed mode)";
        let parsed = parse_java_version_output(output).unwrap();
        assert_eq!(parsed.major, 8);
        assert_eq!(parsed.full, "1.8.0_312");
        assert_eq!(parsed.arch, Arch::X64);
    }

    /// 端到端真机扫描：枚举本机所有 Java。
    /// `cargo test -p ncl-java -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn live_scan_all_with_real_machine() {
        let runtimes = scan_all_with(&[]).await;
        eprintln!("found {} java runtimes:", runtimes.len());
        for rt in &runtimes {
            eprintln!(
                "  major={} vendor={} source={:?} path={}",
                rt.version_major,
                rt.vendor,
                rt.source,
                rt.path.display()
            );
        }
    }
}
