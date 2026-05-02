//! 已知 Java 厂商的默认安装目录扫描。
//!
//! 思路：每个候选根目录下找形如 `*/bin/java(.exe)` 的可执行文件。
//! 不递归全盘扫描——只扫指定的厂商根。

use crate::model::JavaSource;
use std::path::PathBuf;

/// 列出当前平台所有可能的厂商根 + 对应 source 标记。
pub fn well_known_roots() -> Vec<(PathBuf, JavaSource)> {
    let mut out: Vec<(PathBuf, JavaSource)> = Vec::new();

    #[cfg(target_os = "windows")]
    {
        let candidates: &[(&str, JavaSource)] = &[
            (r"C:\Program Files\Eclipse Adoptium", JavaSource::Adoptium),
            (
                r"C:\Program Files (x86)\Eclipse Adoptium",
                JavaSource::Adoptium,
            ),
            (r"C:\Program Files\Eclipse Foundation", JavaSource::Adoptium),
            (r"C:\Program Files\Microsoft", JavaSource::Microsoft),
            (r"C:\Program Files\Zulu", JavaSource::Zulu),
            (r"C:\Program Files (x86)\Zulu", JavaSource::Zulu),
            (r"C:\Program Files\BellSoft", JavaSource::Liberica),
            (r"C:\Program Files\Java", JavaSource::Registry),
            (r"C:\Program Files (x86)\Java", JavaSource::Registry),
        ];
        for (path, source) in candidates {
            out.push((PathBuf::from(path), *source));
        }
    }

    #[cfg(target_os = "linux")]
    {
        for path in [
            "/usr/lib/jvm",
            "/usr/java",
            "/opt",
        ] {
            out.push((PathBuf::from(path), JavaSource::Registry));
        }
    }

    #[cfg(target_os = "macos")]
    {
        out.push((
            PathBuf::from("/Library/Java/JavaVirtualMachines"),
            JavaSource::Registry,
        ));
    }

    out
}

/// 在 root 下找所有 `*/bin/java(.exe)` 候选；非递归（只下钻一层子目录）。
pub fn scan_one_root(root: &std::path::Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let exe = exe_name();
    let Ok(entries) = std::fs::read_dir(root) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        // 直接形态：root/<jdk-21.0.2>/bin/java.exe
        let direct = path.join("bin").join(exe);
        if direct.is_file() {
            out.push(direct);
            continue;
        }
        // Microsoft 形态：root/jdk-21.0.x/bin/java.exe（同上）
        // Mojang 形态：root/<component>/<os>/<arch>/jre.bundle/Contents/Home/bin/java
        // 暂只处理 direct + 一层常见嵌套
        if let Ok(sub) = std::fs::read_dir(&path) {
            for sub_entry in sub.flatten() {
                let sub_path = sub_entry.path();
                if !sub_path.is_dir() {
                    continue;
                }
                let nested = sub_path.join("bin").join(exe);
                if nested.is_file() {
                    out.push(nested);
                }
            }
        }
    }
    out
}

#[cfg(target_os = "windows")]
fn exe_name() -> &'static str {
    "java.exe"
}

#[cfg(not(target_os = "windows"))]
fn exe_name() -> &'static str {
    "java"
}

/// 扫描 Mojang JRE：`<data_root>/runtime/<component>/<os>/<arch>/<bundle>/bin/java`。
/// 此函数接受 NCL 数据根目录，不依赖 PathLayout（避免循环依赖）。
pub fn scan_mojang_jre(data_root: &std::path::Path) -> Vec<PathBuf> {
    let runtime = data_root.join("runtime");
    if !runtime.exists() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let exe = exe_name();
    walk_for_java(&runtime, &mut out, exe, 5); // 限制深度避免无限递归
    out
}

fn walk_for_java(
    dir: &std::path::Path,
    out: &mut Vec<PathBuf>,
    exe: &str,
    depth: u32,
) {
    if depth == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.file_name().and_then(|n| n.to_str()) == Some(exe) {
            // 父目录应该叫 bin
            if path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                == Some("bin")
            {
                out.push(path);
            }
        } else if path.is_dir() {
            walk_for_java(&path, out, exe, depth - 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn well_known_roots_non_empty() {
        let roots = well_known_roots();
        assert!(!roots.is_empty());
    }

    #[test]
    fn scan_nonexistent_returns_empty() {
        let out = scan_one_root(std::path::Path::new("/this/does/not/exist/ever-12345"));
        assert!(out.is_empty());
    }

    #[test]
    fn scan_mojang_jre_no_runtime_returns_empty() {
        let tmp = std::env::temp_dir().join(format!("ncl-mojang-jre-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let out = scan_mojang_jre(&tmp);
        assert!(out.is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
