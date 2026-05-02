//! Library artifact 解析：把 `Library` 数据按 OS 规则展开成可下载的 artifact 列表，
//! 同时识别 native 库（决定哪些 jar 后续需要解压到 natives/ 目录）。

use ncl_core::model::{rules_allow, EvalContext, Library, OsName};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct LibraryArtifact {
    pub url: String,
    pub target: PathBuf,
    pub sha1: String,
    pub size: u64,
    pub kind: ArtifactKind,
}

#[derive(Debug, Clone)]
pub enum ArtifactKind {
    /// 普通 library，添加进 classpath
    Library,
    /// native 库，下载后需解压到 natives/ 目录；excludes 是解压时跳过的路径前缀
    Native { excludes: Vec<String> },
}

/// 把单个 [`Library`] 按 [`EvalContext`] 展开为待下载的 artifact 列表。
///
/// 处理两种 native 表达：
/// - **新格式**（1.19+）：classifier 编进 `library.name`，
///   例如 `org.lwjgl:lwjgl:3.3.3:natives-windows`。
///   `library.downloads.artifact` 是 native jar 本身。
/// - **旧格式**：`library.natives` 映射 OS → classifier，
///   `library.downloads.classifiers[<classifier>]` 是 native jar。
///   主 artifact（如有）依然按 library 处理。
///
/// `library.rules` 的求值在此处统一进行。规则不允许时返回空 vec。
pub fn library_artifacts(
    library: &Library,
    ctx: &EvalContext,
    libraries_root: &Path,
) -> Vec<LibraryArtifact> {
    let mut out = Vec::new();
    if !rules_allow(&library.rules, ctx) {
        return out;
    }

    let is_new_native = is_new_format_native(&library.name);
    let excludes = library
        .extract
        .as_ref()
        .map(|e| e.exclude.clone())
        .unwrap_or_default();

    // 主 artifact
    if let Some(artifact) = library
        .downloads
        .as_ref()
        .and_then(|d| d.artifact.as_ref())
    {
        // 空 URL 通常是 Forge/NeoForge installer 的 processor 产物
        // (forge:<ver>:client / :server / :slim / :srg 等):version.json 里
        // 列出 path + sha1 + size 但 url 为空,因为文件由 binarypatcher /
        // jarsplitter 等 processor 阶段生成。这里跳过下载,留给 processor 阶段。
        if artifact.url.is_empty() {
            tracing::debug!(
                name = %library.name,
                path = %artifact.path,
                "library has empty url; skipping download (processor will produce it)"
            );
        } else {
            let kind = if is_new_native {
                ArtifactKind::Native {
                    excludes: excludes.clone(),
                }
            } else {
                ArtifactKind::Library
            };
            out.push(LibraryArtifact {
                url: artifact.url.clone(),
                target: libraries_root.join(&artifact.path),
                sha1: artifact.sha1.clone(),
                size: artifact.size,
                kind,
            });
        }
    }

    // 旧格式 natives：通过 natives map + classifiers 选择
    if let (Some(natives_map), Some(classifiers)) = (
        library.natives.as_ref(),
        library
            .downloads
            .as_ref()
            .and_then(|d| d.classifiers.as_ref()),
    ) {
        let os_key = match ctx.os.name {
            OsName::Windows => "windows",
            OsName::Linux => "linux",
            OsName::Osx => "osx",
        };
        if let Some(classifier_template) = natives_map.get(os_key) {
            // 处理 ${arch} 占位符
            let arch_str = if cfg!(target_pointer_width = "64") {
                "64"
            } else {
                "32"
            };
            let classifier = classifier_template.replace("${arch}", arch_str);
            if let Some(art) = classifiers.get(&classifier) {
                out.push(LibraryArtifact {
                    url: art.url.clone(),
                    target: libraries_root.join(&art.path),
                    sha1: art.sha1.clone(),
                    size: art.size,
                    kind: ArtifactKind::Native { excludes },
                });
            }
        }
    }

    out
}

/// 检测 `library.name` 是否为新格式 native（含 `natives-<os>` classifier）。
#[must_use]
fn is_new_format_native(name: &str) -> bool {
    let parts: Vec<&str> = name.split(':').collect();
    parts.len() >= 4 && parts[3].starts_with("natives-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ncl_core::model::{ArtifactInfo, ExtractRules, Library, LibraryDownloads, OsName};
    use std::collections::HashMap;

    fn linux_ctx() -> EvalContext {
        let mut ctx = EvalContext::default();
        ctx.os.name = OsName::Linux;
        ctx
    }

    fn windows_ctx() -> EvalContext {
        let mut ctx = EvalContext::default();
        ctx.os.name = OsName::Windows;
        ctx
    }

    fn library_new_format_native() -> Library {
        Library {
            name: "org.lwjgl:lwjgl:3.3.3:natives-windows".into(),
            downloads: Some(LibraryDownloads {
                artifact: Some(ArtifactInfo {
                    path: "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-windows.jar".into(),
                    sha1: "abc".into(),
                    size: 100,
                    url: "https://example.com/lwjgl-natives-win.jar".into(),
                }),
                classifiers: None,
            }),
            natives: None,
            extract: Some(ExtractRules {
                exclude: vec!["META-INF/".into()],
            }),
            rules: vec![],
            url: None,
        }
    }

    fn library_old_format_native() -> Library {
        let mut natives = HashMap::new();
        natives.insert("windows".to_string(), "natives-windows".to_string());
        natives.insert("linux".to_string(), "natives-linux".to_string());

        let mut classifiers = HashMap::new();
        classifiers.insert(
            "natives-windows".to_string(),
            ArtifactInfo {
                path: "old-style-windows.jar".into(),
                sha1: "win".into(),
                size: 1,
                url: "win-url".into(),
            },
        );
        classifiers.insert(
            "natives-linux".to_string(),
            ArtifactInfo {
                path: "old-style-linux.jar".into(),
                sha1: "linux".into(),
                size: 1,
                url: "linux-url".into(),
            },
        );

        Library {
            name: "old:lib:1.0".into(),
            downloads: Some(LibraryDownloads {
                artifact: None,
                classifiers: Some(classifiers),
            }),
            natives: Some(natives),
            extract: None,
            rules: vec![],
            url: None,
        }
    }

    #[test]
    fn new_format_native_marked_as_native() {
        let arts =
            library_artifacts(&library_new_format_native(), &windows_ctx(), Path::new("/libs"));
        assert_eq!(arts.len(), 1);
        assert!(matches!(arts[0].kind, ArtifactKind::Native { .. }));
        assert!(arts[0]
            .target
            .ends_with("org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-windows.jar"));
    }

    #[test]
    fn old_format_picks_correct_classifier_per_os() {
        let lib = library_old_format_native();
        let win = library_artifacts(&lib, &windows_ctx(), Path::new("/libs"));
        assert_eq!(win.len(), 1);
        assert_eq!(win[0].sha1, "win");
        assert!(matches!(win[0].kind, ArtifactKind::Native { .. }));

        let lnx = library_artifacts(&lib, &linux_ctx(), Path::new("/libs"));
        assert_eq!(lnx.len(), 1);
        assert_eq!(lnx[0].sha1, "linux");
    }

    #[test]
    fn rules_disallowed_returns_empty() {
        use ncl_core::model::{OsCondition, Rule, RuleAction};
        let mut lib = library_new_format_native();
        // 仅 osx 允许；windows 上跑应排除
        lib.rules = vec![Rule {
            action: RuleAction::Allow,
            os: Some(OsCondition {
                name: Some(OsName::Osx),
                arch: None,
                version: None,
            }),
            features: None,
        }];
        let arts = library_artifacts(&lib, &windows_ctx(), Path::new("/libs"));
        assert!(arts.is_empty());
    }

    #[test]
    fn plain_library_yields_one_artifact() {
        let lib = Library {
            name: "ca.weblite:java-objc-bridge:1.1".into(),
            downloads: Some(LibraryDownloads {
                artifact: Some(ArtifactInfo {
                    path: "ca/weblite/java-objc-bridge/1.1/java-objc-bridge-1.1.jar".into(),
                    sha1: "deadbeef".into(),
                    size: 42,
                    url: "https://example.com/x.jar".into(),
                }),
                classifiers: None,
            }),
            natives: None,
            extract: None,
            rules: vec![],
            url: None,
        };
        let arts = library_artifacts(&lib, &linux_ctx(), Path::new("/libs"));
        assert_eq!(arts.len(), 1);
        assert!(matches!(arts[0].kind, ArtifactKind::Library));
    }
}
