//! 单 processor 执行：把 maven 坐标列表解析为 classpath 文件路径，从 jar
//! META-INF/MANIFEST.MF 提取 Main-Class，然后启动 java 进程。

use super::profile::{maven_coord_to_path, resolve_token, DataEntry, Processor};
use ncl_core::{Error, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// 把 processor.classpath 的 maven 坐标列表 + jar 自己（去重）解析为本地路径。
pub fn build_processor_classpath(processor: &Processor, libraries_root: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = Vec::with_capacity(processor.classpath.len() + 1);
    let mut seen = std::collections::HashSet::new();
    let mut push = |coord: &str| {
        let p = maven_coord_to_path(libraries_root, coord);
        if seen.insert(p.clone()) {
            paths.push(p);
        }
    };
    push(&processor.jar);
    for c in &processor.classpath {
        push(c);
    }
    paths
}

/// 把 processor.args 的占位符全部解析为最终字符串。
pub fn resolve_processor_args(
    processor: &Processor,
    data: &HashMap<String, DataEntry>,
    libraries_root: &Path,
) -> Result<Vec<String>> {
    processor
        .args
        .iter()
        .map(|a| resolve_token(a, data, libraries_root))
        .collect()
}

/// 从 jar 的 META-INF/MANIFEST.MF 中读 Main-Class 字段。
pub fn extract_main_class(jar: &Path) -> Result<String> {
    let file = std::fs::File::open(jar).map_err(|e| Error::io(jar, e))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| Error::Task(format!("open jar {jar:?}: {e}")))?;
    let mut entry = archive
        .by_name("META-INF/MANIFEST.MF")
        .map_err(|e| Error::Task(format!("MANIFEST.MF not found in {jar:?}: {e}")))?;
    let mut content = String::new();
    use std::io::Read;
    entry
        .read_to_string(&mut content)
        .map_err(|e| Error::io(jar, e))?;

    parse_manifest_main_class(&content)
        .ok_or_else(|| Error::Config(format!("Main-Class not found in MANIFEST.MF of {jar:?}")))
}

/// MANIFEST.MF 用 RFC822 风格但允许多行折叠（折叠行以一个空格开头）。
/// 简单地把折叠行合并，再找 `Main-Class:` 行。
fn parse_manifest_main_class(content: &str) -> Option<String> {
    let mut joined: Vec<String> = Vec::new();
    for raw in content.lines() {
        if let Some(stripped) = raw.strip_prefix(' ') {
            if let Some(last) = joined.last_mut() {
                last.push_str(stripped);
                continue;
            }
        }
        joined.push(raw.to_string());
    }
    for line in joined {
        if let Some(value) = line.strip_prefix("Main-Class:") {
            return Some(value.trim().to_string());
        }
    }
    None
}

#[cfg(target_os = "windows")]
pub const CLASSPATH_SEPARATOR: &str = ";";

#[cfg(not(target_os = "windows"))]
pub const CLASSPATH_SEPARATOR: &str = ":";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::profile::{DataEntry, Processor};

    #[test]
    fn parse_main_class_simple() {
        let m = "Manifest-Version: 1.0\nMain-Class: net.minecraftforge.installer.SimpleInstaller\n";
        assert_eq!(
            parse_manifest_main_class(m).as_deref(),
            Some("net.minecraftforge.installer.SimpleInstaller")
        );
    }

    #[test]
    fn parse_main_class_with_folded_line() {
        let m = "Manifest-Version: 1.0\nMain-Class: net.minecraftforge.installer\n .SimpleInstaller\n";
        // 折叠行合并
        assert_eq!(
            parse_manifest_main_class(m).as_deref(),
            Some("net.minecraftforge.installer.SimpleInstaller")
        );
    }

    #[test]
    fn parse_main_class_missing() {
        let m = "Manifest-Version: 1.0\nImplementation-Title: foo\n";
        assert!(parse_manifest_main_class(m).is_none());
    }

    #[test]
    fn classpath_dedupes_jar_in_classpath() {
        let p = Processor {
            jar: "a.b:c:1.0".into(),
            classpath: vec!["a.b:c:1.0".into(), "x.y:z:2.0".into()],
            args: vec![],
            outputs: HashMap::new(),
            sides: vec![],
        };
        let paths = build_processor_classpath(&p, Path::new("/L"));
        assert_eq!(paths.len(), 2);
    }

    #[test]
    fn resolve_args_substitutes_data_keys() {
        let mut data = HashMap::new();
        data.insert(
            "MCP".to_string(),
            DataEntry {
                client: "[a.b:c:1.0]".into(),
                server: "[a.b:c:1.0]".into(),
            },
        );
        let p = Processor {
            jar: "x:y:1".into(),
            classpath: vec![],
            args: vec!["--task".into(), "MCP_DATA".into(), "--input".into(), "{MCP}".into()],
            outputs: HashMap::new(),
            sides: vec![],
        };
        let resolved = resolve_processor_args(&p, &data, Path::new("/L")).unwrap();
        assert_eq!(resolved[0], "--task");
        assert_eq!(resolved[1], "MCP_DATA"); // 没有 {} 不替换
        assert_eq!(resolved[2], "--input");
        assert!(resolved[3].ends_with("c-1.0.jar"));
    }
}
