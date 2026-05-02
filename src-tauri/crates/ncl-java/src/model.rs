use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 一个本机识别到的 Java 运行时。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaRuntime {
    /// `java` / `java.exe` 的完整路径
    pub path: PathBuf,
    /// 主版本号：8 / 11 / 17 / 21 ...
    pub version_major: u8,
    /// 完整版本字符串，如 `21.0.2` 或 `1.8.0_312`
    pub version_full: String,
    /// 厂商：OpenJDK / Adoptium / Microsoft / Zulu / Oracle / GraalVM 等
    pub vendor: String,
    pub arch: Arch,
    pub source: JavaSource,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Arch {
    X64,
    X86,
    Arm64,
    Other,
}

/// 此 Java 是怎么被发现的。Sprint 5 会补 Registry / Adoptium / MojangJre 等。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JavaSource {
    JavaHome,
    Path,
    Manual,
    Registry,
    Adoptium,
    Microsoft,
    Zulu,
    Liberica,
    MojangJre,
}
