use crate::model::{Arch, JavaRuntime, JavaSource};
use ncl_core::{Error, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// MVP 扫描：JAVA_HOME + PATH。
///
/// Sprint 5 会扩展到注册表 + Adoptium / Microsoft / Zulu / Liberica 默认目录 +
/// Mojang JRE (`.minecraft/runtime`)。
pub async fn scan_all() -> Vec<JavaRuntime> {
    let candidates = collect_candidates();
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut out = Vec::new();

    for (path, source) in candidates {
        let canonical = canonical_or_self(&path);
        if !seen.insert(canonical.clone()) {
            continue;
        }
        match detect_version(&canonical).await {
            Ok(rt) => out.push(JavaRuntime { source, ..rt }),
            Err(e) => {
                tracing::debug!(?canonical, %e, "java probe failed; skipped");
            }
        }
    }
    out.sort_by(|a, b| {
        b.version_major
            .cmp(&a.version_major)
            .then_with(|| a.path.cmp(&b.path))
    });
    out
}

fn collect_candidates() -> Vec<(PathBuf, JavaSource)> {
    let mut v = Vec::new();
    if let Some(jh) = std::env::var_os("JAVA_HOME") {
        let p = PathBuf::from(jh).join("bin").join(java_exe_name());
        if p.is_file() {
            v.push((p, JavaSource::JavaHome));
        }
    }
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            let p = dir.join(java_exe_name());
            if p.is_file() {
                v.push((p, JavaSource::Path));
            }
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

/// 探测 `java_path` 指向的 JRE 信息：执行 `java -version`，从 stderr 解析。
///
/// Java 历史原因：`-version` 输出走 **stderr**，不是 stdout。
pub async fn detect_version(java_path: &Path) -> Result<JavaRuntime> {
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
    // -version 走 stderr
    let stderr = String::from_utf8_lossy(&output.stderr);
    parse_java_version_output(&stderr).map(|info| JavaRuntime {
        path: java_path.to_path_buf(),
        version_major: info.major,
        version_full: info.full,
        vendor: info.vendor,
        arch: info.arch,
        source: JavaSource::Path,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedVersion {
    major: u8,
    full: String,
    vendor: String,
    arch: Arch,
}

/// 解析 `java -version` 的 stderr 输出。
///
/// 典型输入：
/// ```text
/// openjdk version "21.0.2" 2024-01-16 LTS
/// OpenJDK Runtime Environment Temurin-21.0.2+13 (build 21.0.2+13-LTS)
/// OpenJDK 64-Bit Server VM Temurin-21.0.2+13 (build 21.0.2+13-LTS, mixed mode, sharing)
/// ```
fn parse_java_version_output(output: &str) -> Result<ParsedVersion> {
    let lines: Vec<&str> = output.lines().collect();

    // 第一行：`<vendor-prefix> version "<x>"`
    let first = lines
        .first()
        .ok_or_else(|| Error::Task("empty -version output".into()))?;
    let full = extract_quoted(first)
        .ok_or_else(|| Error::Task(format!("cannot parse version line: {first}")))?;
    let major = parse_major(&full);

    // 第二行（可选）："<vendor> Runtime Environment ..."；缺失时回退第一行的开头
    let vendor = lines
        .get(1)
        .and_then(|l| l.split(" Runtime Environment").next())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or_else(|| first.split_whitespace().next())
        .unwrap_or("Unknown")
        .to_string();

    // 第三行（可选）："... <64-Bit|32-Bit> ..."
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
fn parse_major(version: &str) -> u8 {
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
        assert!(
            parsed.vendor.starts_with("OpenJDK"),
            "vendor was {:?}",
            parsed.vendor
        );
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

    /// 真实环境探测当前机器上的 Java（如有）。
    #[tokio::test]
    #[ignore]
    async fn live_scan_returns_at_least_one_jdk() {
        let runtimes = scan_all().await;
        eprintln!("found {} java runtimes:", runtimes.len());
        for rt in &runtimes {
            eprintln!("  {:?}", rt);
        }
    }
}
