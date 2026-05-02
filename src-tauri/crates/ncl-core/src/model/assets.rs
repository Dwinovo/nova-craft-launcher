use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// version JSON 中的 `assetIndex` 字段——指向 asset index 文件的元信息。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssetIndexInfo {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    #[serde(default)]
    pub total_size: u64,
    pub url: String,
}

/// 下载到的 asset index JSON 内容（`indexes/<id>.json`）。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssetIndex {
    pub objects: BTreeMap<String, AssetObject>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}

impl AssetObject {
    /// asset 文件相对路径：`<hash[0..2]>/<hash>`。
    #[must_use]
    pub fn relative_path(&self) -> String {
        format!("{}/{}", &self.hash[..2], self.hash)
    }

    /// 官方下载 URL。
    #[must_use]
    pub fn official_url(&self) -> String {
        format!(
            "https://resources.download.minecraft.net/{}",
            self.relative_path()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_path_and_url() {
        let obj = AssetObject {
            hash: "a9993e364706816aba3e25717850c26c9cd0d89d".to_string(),
            size: 3,
        };
        assert_eq!(
            obj.relative_path(),
            "a9/a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        assert_eq!(
            obj.official_url(),
            "https://resources.download.minecraft.net/a9/a9993e364706816aba3e25717850c26c9cd0d89d"
        );
    }
}
