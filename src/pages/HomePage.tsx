export function HomePage() {
  return (
    <div>
      <h1>欢迎使用 Nova Craft Launcher</h1>
      <p>面向 LLM Agent 时代的 Minecraft 启动器。</p>
      <p style={{ marginTop: 24, color: "#6e6e76" }}>
        当前阶段：<strong>Sprint 1 — Vanilla 1.21.1 最小启动路径</strong>
      </p>
      <ul style={{ marginTop: 16, lineHeight: 1.8 }}>
        <li>
          Mojang manifest 数据模型 + inheritsFrom 合并（39+ 单测覆盖）
        </li>
        <li>多阶段并行下载流水线（client.jar / libraries / assets / natives）</li>
        <li>Java 检测（JAVA_HOME + PATH）+ 兼容矩阵</li>
        <li>启动参数构建（rules 评估 + 占位符替换 + classpath）</li>
        <li>离线账号（OfflinePlayer:&lt;name&gt; v3 UUID）</li>
      </ul>
      <p style={{ marginTop: 24 }}>
        去 <a href="/instances">实例</a> 一键安装并启动 1.21.1 vanilla。
      </p>
    </div>
  );
}
