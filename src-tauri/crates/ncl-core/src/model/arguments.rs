use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// version JSON 的 `arguments` 节（1.13+ 新格式）。
///
/// 旧版本（1.12 及以前）使用 `minecraftArguments` 字符串字段，由
/// [`super::version::ResolvedManifest::minecraft_arguments`] 单独承载。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<Argument>,
    #[serde(default)]
    pub jvm: Vec<Argument>,
}

/// 参数列表中的一项。可能是普通字符串，也可能是带 rules 的条件项。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Argument {
    Plain(String),
    Conditional {
        rules: Vec<Rule>,
        value: ArgumentValue,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ArgumentValue {
    Single(String),
    Multi(Vec<String>),
}

/// 适用性规则。同时被 library 和 arguments 复用——library 中只有 `os`，
/// arguments 中可能含 `features`。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Rule {
    pub action: RuleAction,
    #[serde(default)]
    pub os: Option<OsCondition>,
    #[serde(default)]
    pub features: Option<HashMap<String, bool>>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RuleAction {
    Allow,
    Disallow,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OsCondition {
    #[serde(default)]
    pub name: Option<OsName>,
    #[serde(default)]
    pub arch: Option<String>,
    /// 内核版本正则（Mojang 用过 `^10\\.` 之类）
    #[serde(default)]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OsName {
    Linux,
    Windows,
    Osx,
}

impl OsName {
    /// 当前运行环境的 OS。Mojang 使用 `osx`，不是 `macos`。
    #[must_use]
    pub fn current() -> Self {
        if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::Osx
        } else {
            Self::Linux
        }
    }
}

/// 求值上下文：当前 OS 信息 + 启动 features。
#[derive(Debug, Clone, Default)]
pub struct EvalContext {
    pub os: OsContext,
    pub features: HashMap<String, bool>,
}

#[derive(Debug, Clone)]
pub struct OsContext {
    pub name: OsName,
    pub arch: String,
    pub version: String,
}

impl Default for OsContext {
    fn default() -> Self {
        Self {
            name: OsName::current(),
            arch: std::env::consts::ARCH.to_string(),
            version: std::env::consts::OS.to_string(), // 不重要，rules 极少 match version
        }
    }
}

impl EvalContext {
    #[must_use]
    pub fn with_feature(mut self, key: &str, on: bool) -> Self {
        self.features.insert(key.to_string(), on);
        self
    }
}

impl Rule {
    /// 当前规则的 condition 是否满足上下文（不考虑 action）。
    #[must_use]
    pub fn matches(&self, ctx: &EvalContext) -> bool {
        if let Some(os) = &self.os {
            if let Some(name) = os.name {
                if name != ctx.os.name {
                    return false;
                }
            }
            if let Some(arch) = &os.arch {
                if arch != &ctx.os.arch {
                    return false;
                }
            }
            // 暂不实现 version regex 匹配（Mojang 真实规则极少用，1.21 已不见）
        }
        if let Some(features) = &self.features {
            for (k, v) in features {
                let actual = ctx.features.get(k).copied().unwrap_or(false);
                if actual != *v {
                    return false;
                }
            }
        }
        true
    }
}

/// 给定一组规则与上下文，返回最终是否允许。
///
/// Mojang 算法：`allowed = rules.is_empty()`；遍历规则，匹配则把 allowed 置为
/// 该规则的 action（allow=true / disallow=false）。最后一条匹配规则决定结果。
#[must_use]
pub fn rules_allow(rules: &[Rule], ctx: &EvalContext) -> bool {
    let mut allowed = rules.is_empty();
    for rule in rules {
        if rule.matches(ctx) {
            allowed = matches!(rule.action, RuleAction::Allow);
        }
    }
    allowed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_linux() -> EvalContext {
        EvalContext {
            os: OsContext {
                name: OsName::Linux,
                arch: "x86_64".into(),
                version: "linux".into(),
            },
            features: HashMap::new(),
        }
    }

    fn ctx_windows() -> EvalContext {
        EvalContext {
            os: OsContext {
                name: OsName::Windows,
                arch: "x86_64".into(),
                version: "windows".into(),
            },
            features: HashMap::new(),
        }
    }

    fn rule(action: RuleAction, os: Option<OsName>) -> Rule {
        Rule {
            action,
            os: os.map(|n| OsCondition {
                name: Some(n),
                arch: None,
                version: None,
            }),
            features: None,
        }
    }

    #[test]
    fn empty_rules_allow() {
        assert!(rules_allow(&[], &ctx_linux()));
    }

    #[test]
    fn allow_only_when_os_matches() {
        let rs = vec![rule(RuleAction::Allow, Some(OsName::Linux))];
        assert!(rules_allow(&rs, &ctx_linux()));
        assert!(!rules_allow(&rs, &ctx_windows()));
    }

    #[test]
    fn allow_then_disallow_overrides() {
        let rs = vec![
            rule(RuleAction::Allow, None),
            rule(RuleAction::Disallow, Some(OsName::Linux)),
        ];
        assert!(!rules_allow(&rs, &ctx_linux()));
        assert!(rules_allow(&rs, &ctx_windows()));
    }

    #[test]
    fn feature_gate() {
        let rule = Rule {
            action: RuleAction::Allow,
            os: None,
            features: Some(HashMap::from([("is_demo_user".to_string(), true)])),
        };
        let ctx_no = ctx_linux();
        let ctx_yes = ctx_linux().with_feature("is_demo_user", true);
        assert!(!rules_allow(&[rule.clone()], &ctx_no));
        assert!(rules_allow(&[rule], &ctx_yes));
    }
}
