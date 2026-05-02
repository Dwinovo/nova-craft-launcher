use serde::{Deserialize, Serialize};

/// version JSON 中的 `javaVersion` 字段（1.17+ 后存在）。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JavaVersionRequirement {
    /// Mojang 内部组件名，如 `java-runtime-delta`、`java-runtime-gamma`。
    /// 本启动器不强制使用 Mojang JRE，主要看 `major_version`。
    #[serde(default)]
    pub component: String,
    #[serde(rename = "majorVersion")]
    pub major_version: u8,
}

impl Default for JavaVersionRequirement {
    fn default() -> Self {
        // 老版本无此字段：经验默认 8（1.16-）。Sprint 1 baseline 1.21.1 一定走显式字段。
        Self {
            component: String::new(),
            major_version: 8,
        }
    }
}

impl JavaVersionRequirement {
    /// 检查给定的 Java 主版本号是否满足需求（==、或允许更高的兼容版本）。
    /// Sprint 5 会扩展到完整兼容矩阵；Sprint 1 仅做精确匹配 + 高版本兜底。
    #[must_use]
    pub fn is_satisfied_by(&self, candidate_major: u8) -> bool {
        candidate_major >= self.major_version
    }
}
