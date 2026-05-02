//! 内存推荐：基于系统总内存 + MC 版本 + mod 数量推荐 -Xms / -Xmx。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryRecommendation {
    pub min_mb: u32,
    pub max_mb: u32,
    /// 检测到的系统总内存（MB）
    pub system_total_mb: u64,
}

/// 推荐内存配置。
///
/// 算法（参考 PCL/HMCL 经验值）：
/// - 基础：vanilla 1024 MB；带 loader 2048 MB
/// - 每个 mod 加 8 MB
/// - 上限：系统总内存的 50%
/// - 下限：1024 MB
/// - `-Xms` 取 `-Xmx / 4`（标准 G1GC 推荐）
#[must_use]
pub fn recommend_memory(has_loader: bool, mod_count: usize) -> MemoryRecommendation {
    let total = read_system_total_mb();
    let base: u32 = if has_loader { 2048 } else { 1024 };
    let mods_extra: u32 = (mod_count as u32).saturating_mul(8);
    let suggested = base.saturating_add(mods_extra);
    let cap_by_system = ((total / 2).min(u64::from(u32::MAX))) as u32;
    let max_mb = suggested.min(cap_by_system).max(1024);
    let min_mb = (max_mb / 4).max(256);
    MemoryRecommendation {
        min_mb,
        max_mb,
        system_total_mb: total,
    }
}

fn read_system_total_mb() -> u64 {
    use sysinfo::System;
    let mut sys = System::new();
    sys.refresh_memory();
    sys.total_memory() / (1024 * 1024)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recommendation_in_reasonable_range() {
        let rec = recommend_memory(false, 0);
        assert!(rec.max_mb >= 1024);
        assert!(rec.min_mb >= 256);
        assert!(rec.min_mb <= rec.max_mb);
    }

    #[test]
    fn loader_with_many_mods_increases_memory() {
        let plain = recommend_memory(false, 0);
        let modded = recommend_memory(true, 100);
        assert!(modded.max_mb >= plain.max_mb);
    }
}
