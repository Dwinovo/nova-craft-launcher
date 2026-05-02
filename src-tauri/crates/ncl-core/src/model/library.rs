use super::arguments::Rule;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Maven 风格的库描述。
///
/// 支持两种 natives 表达方式：
/// - **新格式**（1.19+）：classifier 编码进 `name` 末尾，
///   例如 `org.lwjgl:lwjgl-glfw:3.3.3:natives-windows`。
/// - **旧格式**：`natives` map + `downloads.classifiers`。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Library {
    /// Maven GAV，如 `ca.weblite:java-objc-bridge:1.1` 或带 classifier
    /// `org.lwjgl:lwjgl:3.3.3:natives-windows`。
    pub name: String,

    #[serde(default)]
    pub downloads: Option<LibraryDownloads>,

    /// 旧格式 natives 映射：os → classifier。
    #[serde(default)]
    pub natives: Option<HashMap<String, String>>,

    /// 解压时排除的路径前缀（多用于 natives）。
    #[serde(default)]
    pub extract: Option<ExtractRules>,

    /// 适用规则；空则始终生效。
    #[serde(default)]
    pub rules: Vec<Rule>,

    /// 部分 modloader（特别是 Fabric / Forge）使用此字段指向自定义 maven 仓库。
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LibraryDownloads {
    #[serde(default)]
    pub artifact: Option<ArtifactInfo>,
    /// 旧格式 natives 文件位于此处，key = classifier
    #[serde(default)]
    pub classifiers: Option<HashMap<String, ArtifactInfo>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ArtifactInfo {
    pub path: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExtractRules {
    #[serde(default)]
    pub exclude: Vec<String>,
}

/// 从 maven coord 提取 `group:artifact`。
///
/// 例：
/// - `"ca.weblite:java-objc-bridge:1.1"` → `"ca.weblite:java-objc-bridge"`
/// - `"org.lwjgl:lwjgl:3.3.3:natives-windows"` → `"org.lwjgl:lwjgl"`
#[must_use]
pub fn maven_ga(coord: &str) -> &str {
    let mut indices = coord.match_indices(':');
    let _first = indices.next();
    if let Some((second_pos, _)) = indices.next() {
        &coord[..second_pos]
    } else {
        coord
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maven_ga_strips_version() {
        assert_eq!(maven_ga("a.b:c:1.2.3"), "a.b:c");
        assert_eq!(maven_ga("a.b:c:1.2.3:natives-windows"), "a.b:c");
        assert_eq!(maven_ga("only-one"), "only-one"); // 兜底：没有冒号原样返回
    }
}
