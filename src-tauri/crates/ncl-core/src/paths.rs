use crate::errors::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// 启动器目录布局。
///
/// 检测策略：
/// 1. 程序所在目录的 `./data/` 可创建/可写 → Portable（PCL 风格）
/// 2. 否则 `%APPDATA%/NovaCraftLauncher`（Windows）或
///    `~/.local/share/NovaCraftLauncher`（Linux/macOS）→ AppData 回落
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathLayout {
    pub data_root: PathBuf,
    pub mode: PathMode,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PathMode {
    Portable,
    AppData,
}

impl PathLayout {
    pub fn autodetect() -> Result<Self> {
        let exe_dir = current_exe_dir()?;
        let portable = exe_dir.join("data");
        if can_use_dir(&portable) {
            return Ok(Self {
                data_root: portable,
                mode: PathMode::Portable,
            });
        }
        let appdata = appdata_dir()?;
        if can_use_dir(&appdata) {
            return Ok(Self {
                data_root: appdata,
                mode: PathMode::AppData,
            });
        }
        Err(Error::PathLayout(format!(
            "neither portable ({}) nor appdata ({}) is usable",
            portable.display(),
            appdata.display()
        )))
    }

    /// 测试和单元用例使用的显式构造。
    pub fn with_root(data_root: PathBuf) -> Self {
        Self {
            data_root,
            mode: PathMode::Portable,
        }
    }

    pub fn instances(&self) -> PathBuf {
        self.data_root.join("instances")
    }
    pub fn instance(&self, name: &str) -> PathBuf {
        self.instances().join(name)
    }
    pub fn shared_assets(&self) -> PathBuf {
        self.data_root.join("shared/assets")
    }
    pub fn shared_libraries(&self) -> PathBuf {
        self.data_root.join("shared/libraries")
    }
    pub fn shared_versions(&self) -> PathBuf {
        self.data_root.join("shared/versions")
    }
    pub fn config_file(&self) -> PathBuf {
        self.data_root.join("config.json")
    }
    pub fn logs(&self) -> PathBuf {
        self.data_root.join("logs")
    }
    pub fn cache(&self) -> PathBuf {
        self.data_root.join("cache")
    }

    /// 创建所有标准目录。
    pub fn ensure_all(&self) -> Result<()> {
        for path in [
            self.instances(),
            self.shared_assets(),
            self.shared_libraries(),
            self.shared_versions(),
            self.logs(),
            self.cache(),
        ] {
            fs::create_dir_all(&path).map_err(|e| Error::io(&path, e))?;
        }
        Ok(())
    }
}

fn current_exe_dir() -> Result<PathBuf> {
    let exe = std::env::current_exe()
        .map_err(|e| Error::PathLayout(format!("cannot resolve current exe: {e}")))?;
    exe.parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| Error::PathLayout("current exe has no parent".to_string()))
}

#[cfg(target_os = "windows")]
fn appdata_dir() -> Result<PathBuf> {
    let appdata = std::env::var_os("APPDATA")
        .ok_or_else(|| Error::PathLayout("%APPDATA% not set".to_string()))?;
    Ok(PathBuf::from(appdata).join("NovaCraftLauncher"))
}

#[cfg(not(target_os = "windows"))]
fn appdata_dir() -> Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .ok_or_else(|| Error::PathLayout("$HOME not set".to_string()))?;
    Ok(PathBuf::from(home).join(".local/share/NovaCraftLauncher"))
}

fn can_use_dir(path: &Path) -> bool {
    if path.exists() {
        let probe = path.join(".ncl-write-probe");
        let result = fs::write(&probe, b"").is_ok();
        let _ = fs::remove_file(&probe);
        result
    } else {
        fs::create_dir_all(path).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_subdirs_under_root() {
        let root = PathBuf::from("/tmp/ncl-test");
        let layout = PathLayout::with_root(root.clone());
        assert_eq!(layout.instances(), root.join("instances"));
        assert_eq!(layout.instance("foo"), root.join("instances").join("foo"));
        assert_eq!(layout.shared_assets(), root.join("shared/assets"));
        assert_eq!(layout.config_file(), root.join("config.json"));
    }
}
