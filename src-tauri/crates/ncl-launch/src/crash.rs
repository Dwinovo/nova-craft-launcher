//! 进程崩溃后，从 `.minecraft/crash-reports/` 收集最新一份报告并提取关键信息。
//!
//! 参考 PCL2 [`ModCrash::CrashAnalyzer`] 的四阶段思路：
//! 1. 收集 — 找最新的 `crash-*.txt`
//! 2. 准备 — 解析 `Description:` 行 + 取头部若干行
//! 3. 分析 — 留给前端 / Sprint 6 后续做模式匹配（OOM / NoClassDef / Mixin 失败等）
//! 4. 输出 — 通过 IPC 事件 `ncl://process_exit` 的 `crash_report` 字段送给前端

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// 一份从 `crash-reports/<file>.txt` 提取出的崩溃概要。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashReport {
    pub file_path: PathBuf,
    /// 文件名中嵌入的时间戳（`crash-<ts>-client.txt` → `<ts>`）
    pub timestamp: Option<String>,
    /// `Description: ...` 行的内容（最有诊断价值的单行）
    pub description: Option<String>,
    /// 头部前 N 行（含 stack trace 顶部），方便前端展示
    pub head_lines: Vec<String>,
    /// PCL 风格的快速诊断分类（OOM / NoClassDef / ModuleResolution / Mixin / Unknown）
    pub category: CrashCategory,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CrashCategory {
    OutOfMemory,
    NoClassDef,
    ModuleResolution,
    Mixin,
    NativeCrash,
    GpuDriver,
    Unknown,
}

const HEAD_LINES_MAX: usize = 80;

/// 在 `game_dir/crash-reports/` 下找最新一份 `crash-*.txt`，解析并返回。
/// 没有该目录或没有文件时返回 `None`。
#[must_use]
pub fn collect_latest_crash(game_dir: &Path) -> Option<CrashReport> {
    let crash_dir = game_dir.join("crash-reports");
    if !crash_dir.exists() {
        return None;
    }
    let path = newest_crash_file(&crash_dir)?;
    let content = fs::read_to_string(&path).ok()?;
    Some(parse_crash_report(path, &content))
}

fn newest_crash_file(dir: &Path) -> Option<PathBuf> {
    let mut newest: Option<(PathBuf, SystemTime)> = None;
    for entry in fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !name.starts_with("crash-") {
            continue;
        }
        if !name.ends_with(".txt") {
            continue;
        }
        let mtime = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        match &newest {
            Some((_, t)) if *t >= mtime => {}
            _ => newest = Some((path, mtime)),
        }
    }
    newest.map(|(p, _)| p)
}

fn parse_crash_report(path: PathBuf, content: &str) -> CrashReport {
    let description = content
        .lines()
        .find_map(|l| l.trim().strip_prefix("Description:").map(|s| s.trim().to_string()));

    let head_lines: Vec<String> = content
        .lines()
        .take(HEAD_LINES_MAX)
        .map(str::to_string)
        .collect();

    let timestamp = path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| {
            // 形如 `crash-2026-05-02_22.13.45-client.txt` → `2026-05-02_22.13.45`
            s.trim_start_matches("crash-")
                .trim_end_matches(".txt")
                .trim_end_matches("-client")
                .trim_end_matches("-server")
                .to_string()
        });

    let category = classify(content);

    CrashReport {
        file_path: path,
        timestamp,
        description,
        head_lines,
        category,
    }
}

fn classify(content: &str) -> CrashCategory {
    // 用前 4KB 做模式匹配避免扫全文
    let head: String = content.chars().take(4096).collect();
    if head.contains("OutOfMemoryError") || head.contains("Java heap space") {
        return CrashCategory::OutOfMemory;
    }
    if head.contains("NoClassDefFoundError") || head.contains("ClassNotFoundException") {
        return CrashCategory::NoClassDef;
    }
    if head.contains("java.lang.module.ResolutionException") {
        return CrashCategory::ModuleResolution;
    }
    if head.contains("MixinTransformerError")
        || head.contains("InvalidMixinException")
        || head.contains("Mixin apply failed")
    {
        return CrashCategory::Mixin;
    }
    if head.contains("# A fatal error has been detected by the Java Runtime")
        || head.contains("EXCEPTION_ACCESS_VIOLATION")
    {
        return CrashCategory::NativeCrash;
    }
    if head.contains("Pixel format not accelerated")
        || head.contains("OpenGL")
            && (head.contains("not supported") || head.contains("Failed to initialize"))
    {
        return CrashCategory::GpuDriver;
    }
    CrashCategory::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_OOM: &str = r#"---- Minecraft Crash Report ----

Time: 2026-05-02 22:13:45
Description: Exception in server tick loop

java.lang.OutOfMemoryError: Java heap space
	at net.minecraft.server.MinecraftServer.run(MinecraftServer.java:111)
"#;

    const SAMPLE_MODULE: &str = r#"---- Minecraft Crash Report ----

Description: Failed to launch

java.lang.module.ResolutionException: Modules a and b export package x to module c
	at java.base/java.lang.module.Resolver.resolveFail(Resolver.java:900)
"#;

    #[test]
    fn parse_extracts_description_and_category_oom() {
        let report = parse_crash_report(PathBuf::from("crash-test-client.txt"), SAMPLE_OOM);
        assert_eq!(
            report.description.as_deref(),
            Some("Exception in server tick loop")
        );
        assert_eq!(report.category, CrashCategory::OutOfMemory);
        assert!(report.head_lines.len() > 0);
    }

    #[test]
    fn classify_module_resolution() {
        let report = parse_crash_report(PathBuf::from("c.txt"), SAMPLE_MODULE);
        assert_eq!(report.category, CrashCategory::ModuleResolution);
    }

    #[test]
    fn classify_falls_back_to_unknown() {
        let report = parse_crash_report(
            PathBuf::from("c.txt"),
            "---- Minecraft Crash Report ----\n\nDescription: Something weird\n",
        );
        assert_eq!(report.category, CrashCategory::Unknown);
        assert_eq!(report.description.as_deref(), Some("Something weird"));
    }

    #[test]
    fn timestamp_extracted_from_filename() {
        let report = parse_crash_report(
            PathBuf::from("crash-2026-05-02_22.13.45-client.txt"),
            SAMPLE_OOM,
        );
        assert_eq!(report.timestamp.as_deref(), Some("2026-05-02_22.13.45"));
    }

    #[test]
    fn collect_returns_none_when_no_crash_dir() {
        let tmp = std::env::temp_dir().join(format!("ncl-crash-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        assert!(collect_latest_crash(&tmp).is_none());
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn collect_picks_newest_crash_file() {
        let tmp = std::env::temp_dir().join(format!(
            "ncl-crash-newest-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let crash_dir = tmp.join("crash-reports");
        fs::create_dir_all(&crash_dir).unwrap();

        let old = crash_dir.join("crash-old-client.txt");
        fs::write(&old, "Description: old\n").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        let new = crash_dir.join("crash-new-client.txt");
        fs::write(&new, SAMPLE_OOM).unwrap();

        let report = collect_latest_crash(&tmp).expect("should find report");
        assert_eq!(report.category, CrashCategory::OutOfMemory);
        assert_eq!(
            report.description.as_deref(),
            Some("Exception in server tick loop")
        );

        let _ = fs::remove_dir_all(&tmp);
    }
}
