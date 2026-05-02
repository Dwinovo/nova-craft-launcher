//! 镜像源 + 评分池：把官方 URL 改写为镜像 URL 并按健康度降级。
//!
//! 设计参考 PCL2：每个 source 维护一个 score (i32)，
//! - 成功 +5，失败 -7，下限 -30、上限 +30
//! - 候选按 score 倒序排序后逐一尝试
//! - `OfficialSource` 始终作为最后兜底（passthrough）

use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;

/// 镜像源 trait。`rewrite` 把官方 URL 改写为镜像 URL；返回 `None`
/// 表示该镜像不接管这个 URL（应继续尝试下一个）。
pub trait MirrorSource: Send + Sync {
    fn name(&self) -> &str;
    fn rewrite(&self, url: &str) -> Option<String>;
}

pub type SharedSource = Arc<dyn MirrorSource>;

/// 官方源：所有 URL 原样透传。
pub struct OfficialSource;

impl MirrorSource for OfficialSource {
    fn name(&self) -> &str {
        "official"
    }
    fn rewrite(&self, url: &str) -> Option<String> {
        Some(url.to_string())
    }
}

/// BMCLAPI 镜像源（[bmclapidoc.bangbang93.com](https://bmclapidoc.bangbang93.com/)）。
/// 仅改写 Mojang 已知前缀；其他 URL 返回 None。
pub struct BmclApiSource;

impl MirrorSource for BmclApiSource {
    fn name(&self) -> &str {
        "bmclapi"
    }
    fn rewrite(&self, url: &str) -> Option<String> {
        const MAPPINGS: &[(&str, &str)] = &[
            (
                "https://piston-meta.mojang.com/",
                "https://bmclapi2.bangbang93.com/",
            ),
            (
                "https://piston-data.mojang.com/",
                "https://bmclapi2.bangbang93.com/",
            ),
            (
                "https://launchermeta.mojang.com/",
                "https://bmclapi2.bangbang93.com/",
            ),
            (
                "https://launcher.mojang.com/",
                "https://bmclapi2.bangbang93.com/",
            ),
            (
                "https://resources.download.minecraft.net/",
                "https://bmclapi2.bangbang93.com/assets/",
            ),
            (
                "https://libraries.minecraft.net/",
                "https://bmclapi2.bangbang93.com/maven/",
            ),
        ];
        for (from, to) in MAPPINGS {
            if let Some(rest) = url.strip_prefix(from) {
                return Some(format!("{to}{rest}"));
            }
        }
        None
    }
}

/// 镜像池：管理多个 source + score 状态。
pub struct MirrorPool {
    entries: Vec<MirrorEntry>,
}

struct MirrorEntry {
    source: SharedSource,
    score: AtomicI32,
}

const SCORE_FLOOR: i32 = -30;
const SCORE_CEIL: i32 = 30;
const SCORE_SUCCESS: i32 = 5;
const SCORE_FAIL: i32 = -7;

impl MirrorPool {
    /// 创建池。建议把镜像源放在前，`OfficialSource` 放最后兜底。
    /// 第一个 source 获得 +5 初始分，使其默认被首选；其余 0。
    #[must_use]
    pub fn new(sources: Vec<SharedSource>) -> Self {
        let entries = sources
            .into_iter()
            .enumerate()
            .map(|(i, source)| MirrorEntry {
                source,
                score: AtomicI32::new(if i == 0 { SCORE_SUCCESS } else { 0 }),
            })
            .collect();
        Self { entries }
    }

    /// 给定原始 URL，返回 `(改写后 URL, source 索引)` 列表，按 score 倒序排序。
    /// 不能改写的 source 自动跳过。
    #[must_use]
    pub fn candidates(&self, original: &str) -> Vec<(String, usize)> {
        let mut candidates: Vec<(String, usize, i32)> = self
            .entries
            .iter()
            .enumerate()
            .filter_map(|(idx, entry)| {
                entry
                    .source
                    .rewrite(original)
                    .map(|u| (u, idx, entry.score.load(Ordering::Relaxed)))
            })
            .collect();
        // 高分在前；分数相同保持原插入顺序（stable sort）
        candidates.sort_by(|a, b| b.2.cmp(&a.2));
        candidates.into_iter().map(|(u, i, _)| (u, i)).collect()
    }

    /// 上报某个 source 的成败，更新分数（最终钳到 [-30, +30]）。
    pub fn record(&self, source_idx: usize, success: bool) {
        if let Some(entry) = self.entries.get(source_idx) {
            let delta = if success { SCORE_SUCCESS } else { SCORE_FAIL };
            let prev = entry.score.load(Ordering::Relaxed);
            let next = (prev + delta).clamp(SCORE_FLOOR, SCORE_CEIL);
            entry.score.store(next, Ordering::Relaxed);
        }
    }

    pub fn name_of(&self, idx: usize) -> Option<&str> {
        self.entries.get(idx).map(|e| e.source.name())
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bmclapi_rewrites_known_prefixes() {
        let s = BmclApiSource;
        assert_eq!(
            s.rewrite("https://launchermeta.mojang.com/mc/game/version_manifest_v2.json"),
            Some(
                "https://bmclapi2.bangbang93.com/mc/game/version_manifest_v2.json".to_string()
            )
        );
        assert_eq!(
            s.rewrite("https://resources.download.minecraft.net/a9/abc"),
            Some("https://bmclapi2.bangbang93.com/assets/a9/abc".to_string())
        );
        assert_eq!(
            s.rewrite("https://libraries.minecraft.net/org/lwjgl/x.jar"),
            Some("https://bmclapi2.bangbang93.com/maven/org/lwjgl/x.jar".to_string())
        );
    }

    #[test]
    fn bmclapi_returns_none_for_unknown() {
        assert_eq!(
            BmclApiSource.rewrite("https://example.com/somewhere"),
            None
        );
    }

    #[test]
    fn official_passthrough_always_some() {
        let s = OfficialSource;
        assert_eq!(
            s.rewrite("https://example.com/x"),
            Some("https://example.com/x".to_string())
        );
    }

    #[test]
    fn pool_candidates_sorted_by_score() {
        let pool = MirrorPool::new(vec![
            Arc::new(BmclApiSource), // initial +5
            Arc::new(OfficialSource), // 0
        ]);
        let cs = pool.candidates("https://launchermeta.mojang.com/x.json");
        assert_eq!(cs.len(), 2);
        assert!(cs[0].0.contains("bmclapi"));
        assert!(cs[1].0.contains("launchermeta"));
    }

    #[test]
    fn pool_skip_source_that_cannot_rewrite() {
        let pool = MirrorPool::new(vec![
            Arc::new(BmclApiSource),
            Arc::new(OfficialSource),
        ]);
        // BMCLAPI 不接管 example.com
        let cs = pool.candidates("https://example.com/x");
        assert_eq!(cs.len(), 1);
        assert!(cs[0].0.contains("example.com"));
    }

    #[test]
    fn record_failure_demotes_then_official_wins() {
        let pool = MirrorPool::new(vec![
            Arc::new(BmclApiSource),
            Arc::new(OfficialSource),
        ]);
        // BMCLAPI 失败 5 次:5 - 7*5 = -30
        for _ in 0..5 {
            pool.record(0, false);
        }
        let cs = pool.candidates("https://launchermeta.mojang.com/x");
        assert_eq!(cs.len(), 2);
        assert!(cs[0].0.contains("launchermeta"), "official should now lead");
    }

    #[test]
    fn score_clamped() {
        let pool = MirrorPool::new(vec![Arc::new(OfficialSource)]);
        for _ in 0..100 {
            pool.record(0, true);
        }
        // 起始 +5,最多累加到 +30
        assert!(pool.entries[0].score.load(Ordering::Relaxed) <= SCORE_CEIL);
        for _ in 0..100 {
            pool.record(0, false);
        }
        assert!(pool.entries[0].score.load(Ordering::Relaxed) >= SCORE_FLOOR);
    }
}
