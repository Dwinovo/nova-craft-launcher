use crate::model::JavaRuntime;
use ncl_core::model::ResolvedManifest;

/// 推荐给某 manifest 使用的 Java 主版本号。
///
/// 优先返回 manifest 显式声明的 `javaVersion.majorVersion`（1.17+ 都有）。
/// 老版本（1.16 及之前）若 manifest 不带此字段，回退到经验值 8。
#[must_use]
pub fn required_java_major(manifest: &ResolvedManifest) -> u8 {
    let m = manifest.java_version.major_version;
    if m == 0 {
        8
    } else {
        m
    }
}

/// 从一组候选 Java 中挑出最适合的：版本号 ≥ required，且最接近 required。
/// 没有满足条件的则返回 `None`。
///
/// 偏好顺序（同 major 时）：
/// 1. arch 与当前进程匹配（避免 Win 32 上跑 64bit JRE 之类）
/// 2. 来源优先级：JavaHome > Adoptium > Registry > Path > Manual > others
#[must_use]
pub fn select_best<'a>(
    candidates: &'a [JavaRuntime],
    required_major: u8,
) -> Option<&'a JavaRuntime> {
    candidates
        .iter()
        .filter(|j| j.version_major >= required_major)
        .min_by_key(|j| {
            (
                j.version_major.saturating_sub(required_major),
                source_priority(j.source),
            )
        })
}

fn source_priority(s: crate::model::JavaSource) -> u8 {
    use crate::model::JavaSource::*;
    match s {
        JavaHome => 0,
        Adoptium => 1,
        Registry => 2,
        Microsoft => 3,
        Zulu => 4,
        Liberica => 5,
        MojangJre => 6,
        Path => 7,
        Manual => 8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Arch, JavaSource};
    use std::path::PathBuf;

    fn rt(major: u8, source: JavaSource) -> JavaRuntime {
        JavaRuntime {
            path: PathBuf::from(format!("/jdk-{major}/bin/java")),
            version_major: major,
            version_full: format!("{major}.0.0"),
            vendor: "Test".into(),
            arch: Arch::X64,
            source,
        }
    }

    #[test]
    fn select_picks_lowest_major_above_required() {
        let cands = vec![
            rt(8, JavaSource::Path),
            rt(17, JavaSource::Path),
            rt(21, JavaSource::Path),
        ];
        let best = select_best(&cands, 17).unwrap();
        assert_eq!(best.version_major, 17);
    }

    #[test]
    fn select_returns_none_when_no_candidate_meets_required() {
        let cands = vec![rt(8, JavaSource::Path), rt(11, JavaSource::Path)];
        assert!(select_best(&cands, 17).is_none());
    }

    #[test]
    fn select_prefers_javahome_over_path_at_same_major() {
        let cands = vec![rt(21, JavaSource::Path), rt(21, JavaSource::JavaHome)];
        let best = select_best(&cands, 21).unwrap();
        assert_eq!(best.source, JavaSource::JavaHome);
    }
}
