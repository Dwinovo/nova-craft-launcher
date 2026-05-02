//! 启动参数构建：rules 评估 + ${占位符} 替换 + classpath 组装。

use crate::model::{LaunchInputs, LaunchPlan};
use ncl_core::model::{rules_allow, Argument, ArgumentValue, EvalContext, ResolvedManifest};
use ncl_core::Result;
use ncl_vanilla::library::{library_artifacts, ArtifactKind};
use ncl_vanilla::InstallPaths;
use std::collections::HashMap;

/// 构建一个 [`LaunchPlan`]。这是 Sprint 1.5 的核心入口，纯函数（无 IO）。
///
/// 流程（参考 PCL2 `McLaunchArgumentMain` 的工序）：
/// 1. 收集 manifest.arguments.jvm + 内存参数 + 用户 extra
/// 2. **注入 GC 策略**（依 Java 主版本号选 ZGC Gen / ZGC / G1）
/// 3. **注入 log4j2 CVE-2021-44228 修复**（无害 flag，所有版本都加）
/// 4. **去重**（同 `-D<key>=` 后者覆盖前者；`-Xmx`/`-Xms` 前缀去重；
///    `-XX:+Use*GC` 互斥去重；其他完全相等去重；`--tweakClass` 允许重复）
/// 5. game args 同样替换占位符 + 追加用户 extra
pub fn build_plan(inputs: &LaunchInputs<'_>) -> Result<LaunchPlan> {
    let manifest = inputs.manifest;
    let mut ctx = EvalContext::default();
    ctx.features = inputs.features.clone();

    let classpath = build_classpath(manifest, &ctx, inputs.paths);
    let placeholders = build_placeholders(inputs, &classpath);

    // ---- JVM args (manifest 顺序保持) ----
    let mut jvm = Vec::with_capacity(manifest.arguments.jvm.len() + 8);
    jvm.push(format!("-Xms{}M", inputs.memory.min_mb));
    jvm.push(format!("-Xmx{}M", inputs.memory.max_mb));
    flatten_args(&manifest.arguments.jvm, &ctx, &placeholders, &mut jvm);

    // log4j2 CVE-2021-44228 修复:无害 flag,统一注入(老版本必须,新版无影响)
    jvm.push("-Dlog4j2.formatMsgNoLookups=true".to_string());

    // GC 策略:依 Java 主版本号选最优 GC
    inject_gc(&mut jvm, inputs.java.version_major);

    // 用户 extra 在最后,保留覆盖语义(去重时后者胜)
    jvm.extend(inputs.jvm_args_extra.clone());

    // 去重(参考 PCL2 DeduplicateJavaArguments)
    let jvm = dedupe_jvm_args(jvm);

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

/// 依 Java 主版本号注入最优 GC 策略（参考 PCL2）。
///
/// - Java 21+   → ZGC Generational（`-XX:+UseZGC -XX:+ZGenerational`）
/// - Java 15-20 → ZGC Non-generational（`-XX:+UseZGC`）
/// - 其他       → G1GC（`-XX:+UseG1GC`）
///
/// 注入前会先检查 args 里是否已含 `-XX:+Use*GC`（用户 extra 自定义），
/// 若有则不覆盖；最终去重逻辑会保留首个 GC flag。
fn inject_gc(jvm: &mut Vec<String>, java_major: u8) {
    let already_set = jvm
        .iter()
        .any(|a| a.starts_with("-XX:+Use") && a.ends_with("GC"));
    if already_set {
        return;
    }
    if java_major >= 21 {
        jvm.push("-XX:+UseZGC".to_string());
        jvm.push("-XX:+ZGenerational".to_string());
    } else if java_major >= 15 {
        jvm.push("-XX:+UseZGC".to_string());
    } else {
        jvm.push("-XX:+UseG1GC".to_string());
    }
}

/// JVM 参数去重（保留首次出现的有意义值）。参考 PCL2 `DeduplicateJavaArguments`：
/// - 完全相同的字符串：保留**首次**（manifest 优先于用户 extra）
/// - `-Xmx<value>` / `-Xms<value>` 前缀：保留**首次**（manifest 内存配置优先）
/// - `-XX:+Use*GC`（互斥）：保留**首次**
/// - `-D<key>=<value>`：同 key 保留**首次**
/// - `--tweakClass`：允许重复（每个 tweaker 都要保留）
#[must_use]
pub fn dedupe_jvm_args(args: Vec<String>) -> Vec<String> {
    let mut seen_exact: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut seen_prefix: std::collections::HashSet<&'static str> =
        std::collections::HashSet::new();
    let mut seen_d_keys: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut seen_gc = false;
    let mut out: Vec<String> = Vec::with_capacity(args.len());

    for a in args {
        // --tweakClass 允许重复
        if a.starts_with("--tweakClass") {
            out.push(a);
            continue;
        }

        // -Xmx / -Xms 前缀
        if let Some(prefix) = ["-Xmx", "-Xms", "-Xss"]
            .iter()
            .find(|p| a.starts_with(*p))
        {
            if seen_prefix.contains(*prefix) {
                continue;
            }
            seen_prefix.insert(*prefix);
            out.push(a);
            continue;
        }

        // -XX:+Use*GC 互斥
        if a.starts_with("-XX:+Use") && a.ends_with("GC") {
            if seen_gc {
                continue;
            }
            seen_gc = true;
            out.push(a);
            continue;
        }

        // -D<key>=<value> 同 key 去重
        if let Some(rest) = a.strip_prefix("-D") {
            if let Some(eq) = rest.find('=') {
                let key = rest[..eq].to_string();
                if seen_d_keys.contains(&key) {
                    continue;
                }
                seen_d_keys.insert(key);
                out.push(a);
                continue;
            }
        }

        // 其他完全字符串去重
        if seen_exact.contains(&a) {
            continue;
        }
        seen_exact.insert(a.clone());
        out.push(a);
    }

    out
}

/// 构建 classpath：所有满足规则的非 native library 的 jar 路径，附加 client.jar。
///
/// 但对于 modern Forge / NeoForge（1.17+，main_class 含 `BootstrapLauncher`），
/// vanilla `client.jar` 必须**排除**——libraries 里已有 processor 生成的 patched
/// client (`:client` classifier) 与 client-extra (`:client-extra`)，把 vanilla
/// 也加进去会导致 Java 模块系统检测到两个 module 同时 export 同一个 package
/// (`com.mojang.blaze3d.systems` 等),抛 `ResolutionException` 启动直接挂掉。
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
    if !uses_module_loader(&manifest.main_class) {
        entries.push(paths.client_jar().to_string_lossy().into_owned());
    }
    let sep = if cfg!(windows) { ";" } else { ":" };
    entries.join(sep)
}

/// 检测 main_class 是否属于 Forge / NeoForge 的 BootstrapLauncher 家族
/// （Java 模块系统驱动的加载器）。
fn uses_module_loader(main_class: &str) -> bool {
    let lc = main_class.to_ascii_lowercase();
    lc.contains("bootstraplauncher")
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

    // Forge / NeoForge 占位符:BootstrapLauncher 的 -DignoreList 模板会引用
    // `${MC_JAR_NAME}` 来定位需要从模块路径排除的 vanilla client.jar 文件名。
    // 我们把它指向 paths.client_jar() 实际的文件名 (即 <merged_id>.jar)。
    let mc_jar_name = paths
        .client_jar()
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| format!("{}.jar", manifest.id));
    vars.insert("MC_JAR_NAME".into(), mc_jar_name);

    // 在线认证占位符 (离线模式填空字符串而非保留 `${clientid}` 字面量)
    vars.insert("clientid".into(), String::new());
    vars.insert("auth_xuid".into(), String::new());

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
        // 普通 vanilla launch (mainClass=net.minecraft...): client.jar 在
        assert!(cp.contains("1.21.1.jar"), "classpath was: {cp}");
    }

    #[test]
    fn classpath_excludes_client_jar_for_bootstraplauncher() {
        let layout = PathLayout::with_root("/tmp/ncl".into());
        let paths = InstallPaths::new(&layout, "test", "neoforge-21.1.99");
        let mut manifest = fake_manifest();
        // 模拟 NeoForge / Forge 1.17+ 的 main class
        manifest.main_class = "cpw.mods.bootstraplauncher.BootstrapLauncher".into();
        let ctx = EvalContext::default();
        let cp = build_classpath(&manifest, &ctx, &paths);

        // BootstrapLauncher 风格:vanilla client.jar 不能在 classpath
        // (避免与 libraries 里的 patched client 在 module 层冲突)
        assert!(
            !cp.contains("neoforge-21.1.99.jar"),
            "classpath should NOT contain vanilla client jar for BootstrapLauncher; was: {cp}"
        );
        // 库正常包含
        assert!(cp.contains("lwjgl-3.3.3.jar"), "classpath was: {cp}");
    }

    #[test]
    fn dedupe_keeps_first_xmx() {
        let out = dedupe_jvm_args(vec![
            "-Xmx2G".into(),
            "-Xmx4G".into(),
            "-Xms512M".into(),
        ]);
        assert_eq!(out, vec!["-Xmx2G", "-Xms512M"]);
    }

    #[test]
    fn dedupe_d_keys_first_wins() {
        let out = dedupe_jvm_args(vec![
            "-Dlog4j2.formatMsgNoLookups=true".into(),
            "-Dlog4j2.formatMsgNoLookups=false".into(), // 用户试图覆盖
            "-Djava.library.path=/foo".into(),
        ]);
        assert_eq!(out.len(), 2);
        assert!(out.contains(&"-Dlog4j2.formatMsgNoLookups=true".to_string()));
        assert!(out.contains(&"-Djava.library.path=/foo".to_string()));
    }

    #[test]
    fn dedupe_gc_mutex() {
        let out = dedupe_jvm_args(vec![
            "-XX:+UseZGC".into(),
            "-XX:+UseG1GC".into(), // 互斥,丢弃
            "-XX:+ZGenerational".into(), // 不是 *GC,保留
        ]);
        assert_eq!(out, vec!["-XX:+UseZGC", "-XX:+ZGenerational"]);
    }

    #[test]
    fn dedupe_allows_multiple_tweakclass() {
        let out = dedupe_jvm_args(vec![
            "--tweakClass=foo".into(),
            "--tweakClass=bar".into(),
            "--tweakClass=foo".into(), // 同 tweaker 也保留 (PCL 行为)
        ]);
        assert_eq!(out.len(), 3);
    }

    #[test]
    fn dedupe_exact_string_dedup() {
        let out = dedupe_jvm_args(vec![
            "-XstartOnFirstThread".into(),
            "-XstartOnFirstThread".into(),
            "--add-opens=java.base/java.util.jar=ALL-UNNAMED".into(),
        ]);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn inject_gc_picks_zgc_generational_for_java21() {
        let mut jvm = vec!["-Xmx4G".into()];
        inject_gc(&mut jvm, 21);
        assert!(jvm.contains(&"-XX:+UseZGC".to_string()));
        assert!(jvm.contains(&"-XX:+ZGenerational".to_string()));
    }

    #[test]
    fn inject_gc_picks_zgc_for_java17() {
        let mut jvm = vec![];
        inject_gc(&mut jvm, 17);
        assert!(jvm.contains(&"-XX:+UseZGC".to_string()));
        assert!(!jvm.contains(&"-XX:+ZGenerational".to_string()));
    }

    #[test]
    fn inject_gc_picks_g1_for_java8() {
        let mut jvm = vec![];
        inject_gc(&mut jvm, 8);
        assert!(jvm.contains(&"-XX:+UseG1GC".to_string()));
    }

    #[test]
    fn inject_gc_respects_user_choice() {
        let mut jvm = vec!["-XX:+UseShenandoahGC".to_string()];
        inject_gc(&mut jvm, 21);
        // 用户已自定义,不再注入
        assert!(!jvm.contains(&"-XX:+UseZGC".to_string()));
    }

    #[test]
    fn build_plan_injects_log4j_fix_and_gc() {
        let layout = PathLayout::with_root("/tmp/ncl".into());
        let paths = InstallPaths::new(&layout, "test", "1.21.1");
        let manifest = fake_manifest();
        let java = fake_java(); // major 21
        let account = AccountInfo::offline("Steve");
        let inputs = LaunchInputs {
            manifest: &manifest,
            paths: &paths,
            java: &java,
            account: &account,
            memory: MemorySpec { min_mb: 512, max_mb: 4096 },
            jvm_args_extra: vec![],
            game_args_extra: vec![],
            features: HashMap::new(),
        };
        let plan = build_plan(&inputs).unwrap();
        // log4j 修复
        assert!(plan
            .jvm_args
            .contains(&"-Dlog4j2.formatMsgNoLookups=true".into()));
        // ZGC + Generational (Java 21)
        assert!(plan.jvm_args.contains(&"-XX:+UseZGC".into()));
        assert!(plan.jvm_args.contains(&"-XX:+ZGenerational".into()));
    }

    #[test]
    fn placeholders_include_mc_jar_name_and_offline_auth_blanks() {
        let layout = PathLayout::with_root("/tmp/ncl".into());
        let paths = InstallPaths::new(&layout, "test-instance", "neoforge-21.1.99");

        // 模拟 NeoForge 风格的 manifest:arguments.jvm 含
        // `-DignoreList=...,${MC_JAR_NAME}`,arguments.game 含 `${clientid}`
        let mut manifest = fake_manifest();
        manifest.arguments.jvm = vec![
            Argument::Plain("-DignoreList=client-extra.jar,${MC_JAR_NAME}".into()),
            Argument::Plain("-cp".into()),
            Argument::Plain("${classpath}".into()),
        ];
        manifest.arguments.game = vec![
            Argument::Plain("--clientId".into()),
            Argument::Plain("${clientid}".into()),
            Argument::Plain("--xuid".into()),
            Argument::Plain("${auth_xuid}".into()),
        ];
        manifest.id = "neoforge-21.1.99".into();

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
            jvm_args_extra: vec![],
            game_args_extra: vec![],
            features: HashMap::new(),
        };
        let plan = build_plan(&inputs).unwrap();

        // ${MC_JAR_NAME} 应被替换为 <merged_id>.jar
        let ignore_arg = plan
            .jvm_args
            .iter()
            .find(|a| a.starts_with("-DignoreList="))
            .expect("ignoreList present");
        assert!(
            ignore_arg.contains("neoforge-21.1.99.jar"),
            "ignoreList should resolve MC_JAR_NAME; was: {ignore_arg}"
        );
        assert!(
            !ignore_arg.contains("${"),
            "MC_JAR_NAME should not remain unresolved; was: {ignore_arg}"
        );

        // ${clientid} / ${auth_xuid} 应被替换为空字符串
        let cid_idx = plan
            .game_args
            .iter()
            .position(|a| a == "--clientId")
            .expect("clientId arg");
        assert_eq!(plan.game_args[cid_idx + 1], "");
        let xuid_idx = plan
            .game_args
            .iter()
            .position(|a| a == "--xuid")
            .expect("xuid arg");
        assert_eq!(plan.game_args[xuid_idx + 1], "");
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
            jvm_args_extra: vec!["-XX:+UnlockExperimentalVMOptions".into()],
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

        // 用户 extra 非 GC/内存类参数,应被追加且通过去重保留
        assert!(plan
            .jvm_args
            .contains(&"-XX:+UnlockExperimentalVMOptions".into()));

        // main class
        assert_eq!(plan.main_class, "net.minecraft.client.main.Main");

        // game args 中 username / gameDir 替换
        assert!(plan.game_args.contains(&"PlayerOne".into()));
        assert!(plan.game_args.iter().any(|a| a.contains("test-instance")));
        // user extra
        assert!(plan.game_args.contains(&"--quickPlaySingleplayer".into()));
    }
}
