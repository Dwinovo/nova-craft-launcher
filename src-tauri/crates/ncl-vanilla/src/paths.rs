use ncl_core::model::AssetObject;
use ncl_core::PathLayout;
use std::path::PathBuf;

/// 单次安装/启动涉及的所有路径。从 [`PathLayout`] + 实例名 + 版本 id 推导出来。
///
/// 设计：assets / libraries / versions 三个目录在所有实例间 **共享**
/// （位于 `data_root/shared/`），仅 `.minecraft/`（save / mods / config / options.txt）
/// 按实例隔离。这样磁盘占用最小，启动时通过 `--gameDir` 隔离运行。
#[derive(Debug, Clone)]
pub struct InstallPaths {
    pub instance_name: String,
    pub version_id: String,
    /// `<root>/instances/<name>/.minecraft/`
    pub game_dir: PathBuf,
    /// `<root>/shared/versions/<version_id>/`
    pub version_dir: PathBuf,
    /// `<root>/shared/libraries/`
    pub libraries_dir: PathBuf,
    /// `<root>/shared/assets/`
    pub assets_dir: PathBuf,
}

impl InstallPaths {
    pub fn new(layout: &PathLayout, instance_name: impl Into<String>, version_id: impl Into<String>) -> Self {
        let instance_name = instance_name.into();
        let version_id = version_id.into();
        let game_dir = layout.instance(&instance_name).join(".minecraft");
        let version_dir = layout.shared_versions().join(&version_id);
        Self {
            instance_name,
            version_id,
            game_dir,
            version_dir,
            libraries_dir: layout.shared_libraries(),
            assets_dir: layout.shared_assets(),
        }
    }

    /// `versions/<id>/<id>.jar`（client.jar 落点）
    pub fn client_jar(&self) -> PathBuf {
        self.version_dir.join(format!("{}.jar", self.version_id))
    }

    /// `versions/<id>/<id>.json`（合并后的 ResolvedManifest 持久化）
    pub fn version_json(&self) -> PathBuf {
        self.version_dir.join(format!("{}.json", self.version_id))
    }

    /// `versions/<id>/natives/`（native 库解压目标）
    pub fn natives_dir(&self) -> PathBuf {
        self.version_dir.join("natives")
    }

    /// `assets/indexes/<id>.json`
    pub fn asset_index_file(&self, index_id: &str) -> PathBuf {
        self.assets_dir.join("indexes").join(format!("{index_id}.json"))
    }

    /// `assets/objects/<2-prefix>/<hash>`
    pub fn asset_object(&self, obj: &AssetObject) -> PathBuf {
        self.assets_dir.join("objects").join(obj.relative_path())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_are_correctly_derived() {
        let layout = PathLayout::with_root("/tmp/ncl".into());
        let paths = InstallPaths::new(&layout, "my-instance", "1.21.1");
        assert!(paths.game_dir.ends_with("instances/my-instance/.minecraft"));
        assert!(paths.version_dir.ends_with("shared/versions/1.21.1"));
        assert!(paths.client_jar().ends_with("shared/versions/1.21.1/1.21.1.jar"));
        assert!(paths.version_json().ends_with("shared/versions/1.21.1/1.21.1.json"));
        assert!(paths.natives_dir().ends_with("shared/versions/1.21.1/natives"));
        assert!(paths
            .asset_index_file("16")
            .ends_with("shared/assets/indexes/16.json"));
    }
}
