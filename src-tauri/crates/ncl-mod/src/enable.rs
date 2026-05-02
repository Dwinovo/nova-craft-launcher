//! Mod 启用/禁用切换：通过 `.jar` ↔ `.jar.disabled` 重命名实现。

use ncl_core::{Error, Result};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ToggleResult {
    pub from: PathBuf,
    pub to: PathBuf,
    pub now_enabled: bool,
}

/// 切换 jar 的启用状态。
///
/// - 若 `enabled=true` 且当前是 `.jar.disabled` → 重命名为 `.jar`
/// - 若 `enabled=false` 且当前是 `.jar` → 重命名为 `.jar.disabled`
/// - 状态已符合则 noop
pub fn set_enabled(jar_path: &Path, enabled: bool) -> Result<ToggleResult> {
    let name = jar_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| Error::Config(format!("non-utf8 jar path: {}", jar_path.display())))?;

    let is_disabled = name.ends_with(".jar.disabled");
    let is_jar = name.ends_with(".jar");

    if !is_disabled && !is_jar {
        return Err(Error::Config(format!(
            "not a .jar / .jar.disabled file: {}",
            jar_path.display()
        )));
    }

    let target = if enabled && is_disabled {
        // strip .disabled
        let stem = name.strip_suffix(".disabled").unwrap();
        jar_path.with_file_name(stem)
    } else if !enabled && is_jar {
        // append .disabled
        let mut s = name.to_string();
        s.push_str(".disabled");
        jar_path.with_file_name(s)
    } else {
        // 已是目标状态
        return Ok(ToggleResult {
            from: jar_path.to_path_buf(),
            to: jar_path.to_path_buf(),
            now_enabled: enabled,
        });
    };

    if target.exists() {
        return Err(Error::Config(format!(
            "target {} already exists",
            target.display()
        )));
    }

    std::fs::rename(jar_path, &target).map_err(|e| Error::io(jar_path, e))?;
    Ok(ToggleResult {
        from: jar_path.to_path_buf(),
        to: target,
        now_enabled: enabled,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        let d = std::env::temp_dir().join(format!(
            "ncl-mod-enable-{}-{}-{}",
            std::process::id(),
            nanos,
            n
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn enable_renames_disabled_to_jar() {
        let dir = tmp_dir();
        let disabled = dir.join("foo.jar.disabled");
        std::fs::write(&disabled, b"x").unwrap();
        let result = set_enabled(&disabled, true).unwrap();
        assert!(result.now_enabled);
        assert!(result.to.ends_with("foo.jar"));
        assert!(result.to.exists());
        assert!(!disabled.exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn disable_renames_jar_to_disabled() {
        let dir = tmp_dir();
        let enabled = dir.join("bar.jar");
        std::fs::write(&enabled, b"x").unwrap();
        let result = set_enabled(&enabled, false).unwrap();
        assert!(!result.now_enabled);
        assert!(result.to.ends_with("bar.jar.disabled"));
        assert!(result.to.exists());
        assert!(!enabled.exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn noop_when_already_in_target_state() {
        let dir = tmp_dir();
        let enabled = dir.join("baz.jar");
        std::fs::write(&enabled, b"x").unwrap();
        let result = set_enabled(&enabled, true).unwrap();
        assert!(result.now_enabled);
        assert_eq!(result.from, result.to);
        assert!(enabled.exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rejects_non_jar_files() {
        let dir = tmp_dir();
        let txt = dir.join("readme.txt");
        std::fs::write(&txt, b"x").unwrap();
        assert!(set_enabled(&txt, true).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rejects_when_target_already_exists() {
        let dir = tmp_dir();
        let jar = dir.join("conflict.jar");
        let disabled = dir.join("conflict.jar.disabled");
        std::fs::write(&jar, b"x").unwrap();
        std::fs::write(&disabled, b"y").unwrap();
        // 试图禁用,但 .disabled 已存在 → 应报错避免覆盖
        assert!(set_enabled(&jar, false).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
