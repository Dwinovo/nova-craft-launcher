//! 启动参数构建：rules 评估 + ${占位符} 替换 + classpath 组装。

use crate::model::{LaunchInputs, LaunchPlan};
use ncl_core::model::{rules_allow, Argument, ArgumentValue, EvalContext, ResolvedManifest};
use ncl_core::Result;
use ncl_vanilla::library::{library_artifacts, ArtifactKind};
use ncl_vanilla::InstallPaths;
use std::collections::HashMap;

/// 构建一个 [`LaunchPlan`]。这是 Sprint 1.5 的核心入口，纯函数（无 IO）。
pub fn build_plan(inputs: &LaunchInputs<'_>) -> Result<LaunchPlan> {
    let manifest = inputs.manifest;
    let mut ctx = EvalContext::default();
    ctx.features = inputs.features.clone();

    let classpath = build_classpath(manifest, &ctx, inputs.paths);
    let placeholders = build_placeholders(inputs, &classpath);

    // ---- JVM args ----
    let mut jvm = Vec::with_capacity(manifest.arguments.jvm.len() + 4);
    jvm.push(format!("-Xms{}M", inputs.memory.min_mb));
    jvm.push(format!("-Xmx{}M", inputs.memory.max_mb));
    flatten_args(&manifest.arguments.jvm, &ctx, &placeholders, &mut jvm);
    jvm.extend(inputs.jvm_args_extra.clone());

    // ---- Game args ----
    let mut game = Vec::with_capacity(manifest.arguments.game.len() + 8);
    if let Some(legacy) = &manifest.minecraft_arguments {
        // 老版本（1.12 及以前）的整段命令行字符串，按空格切分后逐 token 替换。
        for tok in legacy.split_whitespace() {
            game.push(substitute(tok, &placeholders));
        }
    }
    flatten_args(&manifest.arguments.game, &ctx, &placeholders, &mut game);
    game.extend(inputs.game_args_extra.clone());

    Ok(LaunchPlan {
        java_path: inputs.java.path.clone(),
        jvm_args: jvm,
        main_class: manifest.main_class.clone(),
        game_args: game,
        working_dir: inputs.paths.game_dir.clone(),
    })
}

/// 构建 classpath：所有满足规则的非 native library 的 jar 路径，附加 client.jar。
#[must_use]
pub fn build_classpath(
    manifest: &ResolvedManifest,
    ctx: &EvalContext,
    paths: &InstallPaths,
) -> String {
    let mut entries: Vec<String> = Vec::new();
    for lib in &manifest.libraries {
        for art in library_artifacts(lib, ctx, &paths.libraries_dir) {
            if matches!(art.kind, ArtifactKind::Library) {
                entries.push(art.target.to_string_lossy().into_owned());
            }
        }
    }
    entries.push(paths.client_jar().to_string_lossy().into_owned());
    let sep = if cfg!(windows) { ";" } else { ":" };
    entries.join(sep)
}

/// 把 `[Argument]` 列表压平为字符串序列（评估 rules + 替换占位符）。
pub fn flatten_args(
    args: &[Argument],
    ctx: &EvalContext,
    vars: &HashMap<String, String>,
    out: &mut Vec<String>,
) {
    for arg in args {
        match arg {
            Argument::Plain(s) => out.push(substitute(s, vars)),
            Argument::Conditional { rules, value } => {
                if rules_allow(rules, ctx) {
                    match value {
                        ArgumentValue::Single(s) => out.push(substitute(s, vars)),
                        ArgumentValue::Multi(ss) => {
                            for s in ss {
                                out.push(substitute(s, vars));
                            }
                        }
                    }
                }
            }
        }
    }
}

/// 把字符串内的 `${key}` 替换为 `vars[key]`。
/// 未识别的占位符保持原样（便于调试）。
#[must_use]
pub fn substitute(input: &str, vars: &HashMap<String, String>) -> String {
    let mut out = input.to_string();
    for (k, v) in vars {
        let pattern = format!("${{{k}}}");
        if out.contains(&pattern) {
            out = out.replace(&pattern, v);
        }
    }
    out
}

fn build_placeholders(inputs: &LaunchInputs<'_>, classpath: &str) -> HashMap<String, String> {
    let manifest = inputs.manifest;
    let paths = inputs.paths;
    let account = inputs.account;

    let mut vars = HashMap::new();

    // 账号
    vars.insert("auth_player_name".into(), account.username.clone());
    vars.insert("auth_uuid".into(), account.uuid.replace('-', ""));
    vars.insert("auth_access_token".into(), account.access_token.clone());
    vars.insert("auth_session".into(), format!("token:{}:0", account.access_token));
    vars.insert("user_type".into(), account.user_type.clone());
    vars.insert("user_properties".into(), "{}".into());
    // 老 1.7- 字段
    vars.insert("auth_player_uuid".into(), account.uuid.clone());

    // 版本
    vars.insert("version_name".into(), manifest.id.clone());
    vars.insert(
        "version_type".into(),
        format!("{:?}", manifest.kind).to_lowercase(),
    );

    // 目录
    vars.insert(
        "game_directory".into(),
        paths.game_dir.to_string_lossy().into_owned(),
    );
    vars.insert(
        "assets_root".into(),
        paths.assets_dir.to_string_lossy().into_owned(),
    );
    vars.insert(
        "game_assets".into(),
        paths.assets_dir.to_string_lossy().into_owned(),
    );
    vars.insert("assets_index_name".into(), manifest.asset_index.id.clone());
    vars.insert(
        "natives_directory".into(),
        paths.natives_dir().to_string_lossy().into_owned(),
    );
    vars.insert(
        "library_directory".into(),
        paths.libraries_dir.to_string_lossy().into_owned(),
    );

    // classpath
    vars.insert("classpath".into(), classpath.to_string());
    vars.insert(
        "classpath_separator".into(),
        if cfg!(windows) { ";" } else { ":" }.into(),
    );

    // 启动器自身标识
    vars.insert("launcher_name".into(), "Nova-Craft-Launcher".into());
    vars.insert("launcher_version".into(), env!("CARGO_PKG_VERSION").into());

    vars
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::AccountInfo;
    use crate::model::{LaunchInputs, MemorySpec};
    use ncl_core::model::{
        ArtifactInfo, AssetIndexInfo, DownloadInfo, JavaVersionRequirement, Library,
        LibraryDownloads, MainDownloads, OsCondition, OsName, ResolvedManifest, Rule,
        RuleAction, VersionType,
    };
    use ncl_core::PathLayout;
    use ncl_java::{Arch, JavaRuntime, JavaSource};
    use ncl_vanilla::InstallPaths;
    use std::path::PathBuf;

    fn fake_manifest() -> ResolvedManifest {
        ResolvedManifest {
            id: "1.21.1".into(),
            kind: VersionType::Release,
            main_class: "net.minecraft.client.main.Main".into(),
            asset_index: AssetIndexInfo {
                id: "16".into(),
                sha1: "0".into(),
                size: 1,
                total_size: 1,
                url: "u".into(),
            },
            downloads: MainDownloads {
                client: Some(DownloadInfo {
                    sha1: "0".into(),
                    size: 1,
                    url: "u".into(),
                }),
                client_mappings: None,
                server: None,
                server_mappings: None,
            },
            libraries: vec![
                Library {
                    name: "org.lwjgl:lwjgl:3.3.3".into(),
                    downloads: Some(LibraryDownloads {
                        artifact: Some(ArtifactInfo {
                            path: "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3.jar".into(),
                            sha1: "0".into(),
                            size: 1,
                            url: "u".into(),
                        }),
                        classifiers: None,
                    }),
                    natives: None,
                    extract: None,
                    rules: vec![],
                    url: None,
                },
                Library {
                    name: "org.lwjgl:lwjgl:3.3.3:natives-windows".into(),
                    downloads: Some(LibraryDownloads {
                        artifact: Some(ArtifactInfo {
                            path: "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-windows.jar".into(),
                            sha1: "0".into(),
                            size: 1,
                            url: "u".into(),
                        }),
                        classifiers: None,
                    }),
                    natives: None,
                    extract: None,
                    rules: vec![Rule {
                        action: RuleAction::Allow,
                        os: Some(OsCondition {
                            name: Some(OsName::Windows),
                            arch: None,
                            version: None,
                        }),
                        features: None,
                    }],
                    url: None,
                },
            ],
            arguments: ncl_core::model::Arguments {
                game: vec![
                    Argument::Plain("--username".into()),
                    Argument::Plain("${auth_player_name}".into()),
                    Argument::Plain("--version".into()),
                    Argument::Plain("${version_name}".into()),
                    Argument::Plain("--gameDir".into()),
                    Argument::Plain("${game_directory}".into()),
                ],
                jvm: vec![
                    Argument::Plain("-Djava.library.path=${natives_directory}".into()),
                    Argument::Plain("-cp".into()),
                    Argument::Plain("${classpath}".into()),
                ],
            },
            minecraft_arguments: None,
            java_version: JavaVersionRequirement {
                component: "java-runtime-delta".into(),
                major_version: 21,
            },
            inherits_chain: vec!["1.21.1".into()],
        }
    }

    fn fake_java() -> JavaRuntime {
        JavaRuntime {
            path: PathBuf::from("/usr/bin/java"),
            version_major: 21,
            version_full: "21.0.2".into(),
            vendor: "Test".into(),
            arch: Arch::X64,
            source: JavaSource::JavaHome,
        }
    }

    #[test]
    fn substitute_replaces_known_keys() {
        let mut vars = HashMap::new();
        vars.insert("name".into(), "Steve".into());
        vars.insert("ver".into(), "1.21.1".into());
        let out = substitute("hello ${name} v${ver}!", &vars);
        assert_eq!(out, "hello Steve v1.21.1!");
    }

    #[test]
    fn substitute_keeps_unknown_placeholders() {
        let vars = HashMap::new();
        let out = substitute("hello ${unknown}", &vars);
        assert_eq!(out, "hello ${unknown}");
    }

    #[test]
    fn flatten_args_filters_by_rules() {
        let ctx = EvalContext::default(); // 当前 OS
        let vars = HashMap::new();
        let mut out = Vec::new();
        let win_only = Argument::Conditional {
            rules: vec![Rule {
                action: RuleAction::Allow,
                os: Some(OsCondition {
                    name: Some(OsName::Windows),
                    arch: None,
                    version: None,
                }),
                features: None,
            }],
            value: ArgumentValue::Single("--windows-only".into()),
        };
        flatten_args(&[win_only], &ctx, &vars, &mut out);
        if cfg!(windows) {
            assert_eq!(out, vec!["--windows-only"]);
        } else {
            assert!(out.is_empty(), "non-windows should skip");
        }
    }

    #[test]
    fn flatten_args_handles_multi_value() {
        let ctx = EvalContext::default();
        let vars = HashMap::new();
        let multi = Argument::Conditional {
            rules: vec![],
            value: ArgumentValue::Multi(vec!["-a".into(), "-b".into(), "-c".into()]),
        };
        let mut out = Vec::new();
        flatten_args(&[multi], &ctx, &vars, &mut out);
        assert_eq!(out, vec!["-a", "-b", "-c"]);
    }

    #[test]
    fn classpath_skips_natives_includes_client_jar() {
        let layout = PathLayout::with_root("/tmp/ncl".into());
        let paths = InstallPaths::new(&layout, "test", "1.21.1");
        let manifest = fake_manifest();
        let ctx = EvalContext::default();
        let cp = build_classpath(&manifest, &ctx, &paths);

        // 主 lwjgl jar 必须在
        assert!(cp.contains("lwjgl-3.3.3.jar"), "classpath was: {cp}");
        // 新格式 native jar 不能在 classpath
        assert!(!cp.contains("natives-windows"), "classpath was: {cp}");
        // client.jar 在
        assert!(cp.contains("1.21.1.jar"), "classpath was: {cp}");
    }

    #[test]
    fn build_plan_substitutes_placeholders() {
        let layout = PathLayout::with_root("/tmp/ncl".into());
        let paths = InstallPaths::new(&layout, "test-instance", "1.21.1");
        let manifest = fake_manifest();
        let java = fake_java();
        let account = AccountInfo::offline("PlayerOne");
        let inputs = LaunchInputs {
            manifest: &manifest,
            paths: &paths,
            java: &java,
            account: &account,
            memory: MemorySpec {
                min_mb: 512,
                max_mb: 4096,
            },
            jvm_args_extra: vec!["-XX:+UseG1GC".into()],
            game_args_extra: vec!["--quickPlaySingleplayer".into(), "MyWorld".into()],
            features: HashMap::new(),
        };
        let plan = build_plan(&inputs).unwrap();

        // 内存
        assert!(plan.jvm_args.contains(&"-Xms512M".into()));
        assert!(plan.jvm_args.contains(&"-Xmx4096M".into()));

        // 占位符替换：natives_directory 应被展开为绝对路径，且不再含 ${}
        assert!(plan
            .jvm_args
            .iter()
            .any(|a| a.contains("-Djava.library.path=") && !a.contains("${")));

        // -cp 后跟一段 classpath
        let cp_idx = plan
            .jvm_args
            .iter()
            .position(|a| a == "-cp")
            .expect("-cp present");
        let cp = &plan.jvm_args[cp_idx + 1];
        assert!(cp.contains("lwjgl-3.3.3.jar"));
        assert!(cp.contains("1.21.1.jar"));

        // 用户 extra 也应被追加在尾部
        assert!(plan.jvm_args.contains(&"-XX:+UseG1GC".into()));

        // main class
        assert_eq!(plan.main_class, "net.minecraft.client.main.Main");

        // game args 中 username / gameDir 替换
        assert!(plan.game_args.contains(&"PlayerOne".into()));
        assert!(plan.game_args.iter().any(|a| a.contains("test-instance")));
        // user extra
        assert!(plan.game_args.contains(&"--quickPlaySingleplayer".into()));
    }
}
