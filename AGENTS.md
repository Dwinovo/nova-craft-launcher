# 🚀 Nova Craft Launcher (NCL)

> 面向 AI 编码代理的项目说明书。本文档描述项目目标、架构方向与开发阶段，帮助你在写代码前先理解「我们到底在造什么」。

---

## 📌 项目是什么

**Nova Craft Launcher（NCL）** 是一个**基于大语言模型（LLM Agent）与动态技能注入机制的新一代智能化 Minecraft 启动平台**。

它不仅是一个传统意义上的 Minecraft 客户端版本调度器，更要成为 Minecraft 生态中**首个 Agentic AI 助理与自动化环境部署底座**——把玩家组装多模组环境的认知成本降到最低。

一句话定位：**「让用户用一句自然语言，就能跑起一套完整的多模组 Minecraft 整合包。」**

---

## 💡 为什么要造它

打造 NCL 的核心目的是**解决 Minecraft 生态高度碎片化带来的高门槛**：

1. **无缝跨越繁杂的隔离底座**：社区目前分裂为 Forge / Fabric / NeoForge 等多套技术栈，普通玩家难以理清差异。
2. **终结极其痛苦的依赖地狱**：寻找版本适配的 Mod、排查依赖缺失导致的闪退、调优 Java 内存参数——这些都在劝退新玩家。
3. **用 AI 实现自动化降维打击**：引入具备思考与工具调用能力的 AI，直接解析玩家的自然语言指令，自动寻找最优 Mod 组合、拉取依赖树、完成沙盒配置，**彻底终结「手动查攻略 + 手动下文件」的旧时代**。

---

## 🚀 五阶段执行路线

实现路径分为五个关键阶段，**当前处于第一阶段起步**（Tauri 骨架刚搭好）：

### 阶段 1：核心基建与多协议加载环境
- 实现稳健的客户端底盘
- 完成 Minecraft 原版版本管理
- 原生集成 **Forge / Fabric / NeoForge** 三大模组平台的自动化安装与切换
- 构建完备的 Mod 读取、识别、管理中心

### 阶段 2：CLI 操作流与 Skills 接口暴露
- 把阶段 1 打磨好的装配能力，**剥离为标准化 CLI**
- 同时暴露为可供结构化调用的 **Skills 组件**
- 为后续 AI 原生接管打好基台——所有 GUI 能做的事，都必须能通过 CLI / Skill 完成

### 阶段 3：AI 引擎接入与智能引导配置
- 打通与 LLM 的通信
- 建立针对 Minecraft 模组生态的**专属知识库**与 **Skill 调用清单**
- 让 AI Agent 能借助界面交互帮玩家排错、配置、组装、推荐 Mod 搭配方案

### 阶段 4：知名模组生态专属化支持
- 为 **Create（机械动力）**、**Botania（植物魔法）** 等头部复杂模组定制深度解析 Skill
- 用「0 成本保姆级配置」体验切入社区，建立不可替代性

### 阶段 5：精准营销裂变与项目宣传
- 围绕「一键配置」「零门槛」「AI 前沿体验」做集中宣发
- 技术社区测评 + 游戏解说推荐 + 短视频分发
- 主打标签：**「次世代跨时代一键 AI 启动器」**

---

## 🧭 给 AI 代理的工作准则

当你（AI 代理）在本仓库内写代码时，请遵循以下方向：

1. **CLI 优先、Skill 友好**：任何新功能在设计接口时，**先想「这个能力怎么暴露给 AI 调用」**，再想 GUI 怎么呈现。GUI 是 Skill 的可视化壳，不是核心。
2. **三平台一视同仁**：Forge / Fabric / NeoForge 必须并列支持，避免在抽象层耦合任一加载器的特性。NeoForge 是当前主推方向（用户已在做 1.20.1 Forge → 1.21.1 NeoForge 的迁移实践）。
3. **依赖解析是核心竞争力**：Mod 依赖树、版本兼容性矩阵、Java 运行时参数推断——这些是 NCL 的护城河，不是辅助功能。
4. **为 Agent 留出可观测性**：所有装配、安装、启动过程都应产出**结构化日志**，方便 AI Agent 在阶段 3 接入时直接读取并诊断。
5. **不要过度设计**：当前是阶段 1，先把版本管理 + 三平台加载器跑通。AI 接入相关的抽象不要在没必要时提前引入。

---

## 🛠️ 技术栈（已确定）

- **桌面端框架**：[Tauri 2](https://v2.tauri.app/)（Rust 后端 + Web 前端，体积小、原生性能、跨平台）
- **前端**：React + TypeScript + Vite + react-router-dom
- **包管理器**：pnpm
- **后端语言**：Rust（cargo workspace，阶段 1~2 所有版本管理 / Mod 加载 / CLI 能力的真正核心）
- **应用 Identifier**：`com.dwin.novacraftlauncher`
- **数据存储**：便携式 `./data/`（PCL 风格），不可写时回落 `%APPDATA%/NovaCraftLauncher/`

### 仓库结构
```
nova-craft-launcher/
├── src/                          # React 前端（启动器 GUI）
│   ├── App.tsx
│   ├── components/AppShell.tsx   # 侧栏 + 主区
│   ├── pages/                    # Home / Instances / Mods / Java / Settings
│   └── lib/api.ts                # Tauri invoke 包装
├── src-tauri/                    # Rust workspace
│   ├── Cargo.toml                # workspace 根 + Tauri app 包
│   ├── src/                      # Tauri app（仅 IPC 包装）
│   │   ├── lib.rs
│   │   └── ipc/                  # 各域命令薄包装
│   └── crates/                   # 9 个独立业务 crate
│       ├── ncl-core/             # 类型/错误/PathLayout/ProgressSink ✅
│       ├── ncl-net/              # HTTP/镜像/下载/SHA ✅(部分)
│       ├── ncl-task/             # DAG 任务编排（Sprint 1+）
│       ├── ncl-vanilla/          # Mojang manifest（Sprint 1+）
│       ├── ncl-loader/           # Forge/Fabric/NeoForge（Sprint 3+）
│       ├── ncl-mod/              # mod 元数据（Sprint 4+）
│       ├── ncl-java/             # Java 检测（Sprint 1+/Sprint 5）
│       ├── ncl-launch/           # 启动参数 + 进程（Sprint 1+）
│       └── ncl-cli/              # CLI 入口（阶段 2）
└── AGENTS.md
```

### 常用命令
```bash
pnpm install                          # 安装前端依赖
pnpm tauri dev                        # 启动开发模式（前端 + Rust 后端热重载）
pnpm tauri build                      # 构建生产包

cd src-tauri
cargo check --workspace --all-targets # 全 crate 编译检查
cargo test --workspace                # 全 crate 单测
cargo run --bin ncl                   # 运行 CLI（阶段 2 才完整）
```

---

## 📊 阶段 1 实施进度

实施计划详见 `C:\Users\dwin\.claude\plans\forge-fabric-neoforge-shiny-hearth.md`。

| Sprint | 主题 | 状态 |
|---|---|---|
| **0** | 地基 — workspace + 9 crate + 核心契约 + 前端骨架 + IPC 联通 | ✅ 已完成 |
| 1 | Vanilla 1.21.1 最小启动路径 | ⏳ 待开始 |
| 2 | BMCLAPI 镜像 + 下载稳定性 | ⏳ 待开始 |
| 3 | Forge + Fabric 支持 | ⏳ 待开始 |
| 4 | NeoForge + Mod 管理 | ⏳ 待开始 |
| 5 | Java 全面扫描 + 内存推荐 | ⏳ 待开始 |
| 6 | CLI 骨架 + 打磨（缓冲） | ⏳ 待开始 |

---

## 🔗 相关参考

- **Notion 项目主页**：[Nova Craft Launcher (NCL)](https://www.notion.so/340f684b518a81da90aaf256a2e068c2)
- **关联项目**：DwinOS（个人知识/项目管理系统）、Project Anima（跨维社交共生网络，长期可能与 NCL 在 Minecraft 接入层产生协同）
