//! 扫描实例的 mods/ 目录，解析所有 jar 元数据。

use crate::parse::{is_mod_jar, parse_jar, ParseError};
use ncl_core::{Error, ModEntry, Result};
use std::path::Path;

/// 扫描 `mods_dir` 中所有 .jar / .jar.disabled，返回成功解析的 ModEntry。
/// 解析失败的文件会被忽略（仅 trace 日志）。
pub fn scan_mods(mods_dir: &Path) -> Result<Vec<ModEntry>> {
    if !mods_dir.exists() {
        return Ok(Vec::new());
    }
    let entries = std::fs::read_dir(mods_dir).map_err(|e| Error::io(mods_dir, e))?;
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !is_mod_jar(&path) {
            continue;
        }
        match parse_jar(&path) {
            Ok(m) => out.push(m),
            Err(ParseError::NotAMod) => {
                tracing::debug!(?path, "not a recognized mod jar; skipped");
            }
            Err(e) => {
                tracing::warn!(?path, %e, "mod parse error; skipped");
            }
        }
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(out)
}
