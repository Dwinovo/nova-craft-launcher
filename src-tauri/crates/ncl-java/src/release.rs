//! 解析 JDK 安装目录下的 `release` 文件——比执行 `java -version` 快得多，
//! 且不需要启动 JVM。
//!
//! `release` 文件格式（key="value"）：
//! ```text
//! IMPLEMENTOR="Eclipse Adoptium"
//! IMPLEMENTOR_VERSION="Temurin-21.0.2+13"
//! JAVA_VERSION="21.0.2"
//! OS_ARCH="x86_64"
//! ```

use crate::model::Arch;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct ReleaseInfo {
    pub java_version: Option<String>,
    pub vendor: Option<String>,
    pub os_arch: Option<String>,
}

impl ReleaseInfo {
    pub fn arch(&self) -> Arch {
        match self.os_arch.as_deref() {
            Some("x86_64") | Some("amd64") => Arch::X64,
            Some("x86") | Some("i386") | Some("i686") => Arch::X86,
            Some("aarch64") | Some("arm64") => Arch::Arm64,
            _ => Arch::Other,
        }
    }
}

/// 解析 JDK home 目录下的 `release` 文件。
/// `java_home` 是 `bin/` 的上一级（即 JAVA_HOME 自身）。
pub fn parse_release_file(java_home: &Path) -> Option<ReleaseInfo> {
    let path = java_home.join("release");
    let content = std::fs::read_to_string(&path).ok()?;
    Some(parse_release_content(&content))
}

pub fn parse_release_content(content: &str) -> ReleaseInfo {
    let mut info = ReleaseInfo::default();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let v = v.trim().trim_matches('"').to_string();
        match k.trim() {
            "JAVA_VERSION" => info.java_version = Some(v),
            "IMPLEMENTOR" => info.vendor = Some(v),
            "OS_ARCH" => info.os_arch = Some(v),
            _ => {}
        }
    }
    info
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_temurin_release_file() {
        let content = r#"
IMPLEMENTOR="Eclipse Adoptium"
IMPLEMENTOR_VERSION="Temurin-21.0.2+13"
JAVA_VERSION="21.0.2"
JAVA_VERSION_DATE="2024-01-16"
LIBC="default"
MODULES="java.base java.logging"
OS_ARCH="x86_64"
OS_NAME="Windows"
SOURCE=".:foo"
"#;
        let info = parse_release_content(content);
        assert_eq!(info.java_version.as_deref(), Some("21.0.2"));
        assert_eq!(info.vendor.as_deref(), Some("Eclipse Adoptium"));
        assert_eq!(info.os_arch.as_deref(), Some("x86_64"));
        assert_eq!(info.arch(), Arch::X64);
    }

    #[test]
    fn parse_oracle_release_file() {
        let content = r#"
JAVA_VERSION="1.8.0_312"
OS_NAME="Windows"
OS_VERSION="5.2"
OS_ARCH="amd64"
"#;
        let info = parse_release_content(content);
        assert_eq!(info.java_version.as_deref(), Some("1.8.0_312"));
        assert_eq!(info.arch(), Arch::X64);
    }

    #[test]
    fn parse_arm64_release_file() {
        let content = r#"
JAVA_VERSION="17.0.6"
OS_ARCH="aarch64"
"#;
        let info = parse_release_content(content);
        assert_eq!(info.arch(), Arch::Arm64);
    }
}
