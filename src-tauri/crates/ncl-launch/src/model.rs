use crate::account::AccountInfo;
use ncl_core::model::ResolvedManifest;
use ncl_java::JavaRuntime;
use ncl_vanilla::InstallPaths;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// JVM 内存配置。
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MemorySpec {
    pub min_mb: u32,
    pub max_mb: u32,
}

impl Default for MemorySpec {
    fn default() -> Self {
        Self {
            min_mb: 512,
            max_mb: 2048,
        }
    }
}

/// 构建启动计划所需的全部输入。
///
/// 借用语义：所有引用类型，调用方持有所有权——build_plan 是纯函数，不持有状态。
pub struct LaunchInputs<'a> {
    pub manifest: &'a ResolvedManifest,
    pub paths: &'a InstallPaths,
    pub java: &'a JavaRuntime,
    pub account: &'a AccountInfo,
    pub memory: MemorySpec,
    pub jvm_args_extra: Vec<String>,
    pub game_args_extra: Vec<String>,
    /// 启动 features（影响 arguments rules 评估）。
    /// 例如 `is_demo_user` / `has_custom_resolution` 等。
    pub features: HashMap<String, bool>,
}

/// 启动计划：已组装好的命令行 + 工作目录。Sprint 1.6 的 `spawn` 函数消费它。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchPlan {
    pub java_path: PathBuf,
    pub jvm_args: Vec<String>,
    pub main_class: String,
    pub game_args: Vec<String>,
    pub working_dir: PathBuf,
}

impl LaunchPlan {
    /// 完整命令行序列（不含 java 路径），便于日志展示与 dry-run 验证。
    #[must_use]
    pub fn full_args(&self) -> Vec<String> {
        let mut out = self.jvm_args.clone();
        out.push(self.main_class.clone());
        out.extend(self.game_args.clone());
        out
    }
}
