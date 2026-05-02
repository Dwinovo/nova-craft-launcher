export function HomePage() {
  return (
    <div>
      <h1>欢迎使用 Nova Craft Launcher</h1>
      <p>面向 LLM Agent 时代的 Minecraft 启动器。</p>
      <p style={{ marginTop: 24, color: "#6e6e76" }}>
        当前阶段：<strong>Sprint 0 — 地基</strong>
      </p>
      <ul style={{ marginTop: 16, lineHeight: 1.8 }}>
        <li>Cargo workspace + 9 个业务 crate 骨架</li>
        <li>
          <code>ncl-core</code>: PathLayout / ProgressSink / 错误 / 配置
        </li>
        <li>
          <code>ncl-net</code>: Downloader + SHA1 + MirrorSource trait
        </li>
        <li>前端 Router + AppShell</li>
      </ul>
      <p style={{ marginTop: 24 }}>
        前往 <a href="/settings">设置</a> 查看当前目录布局。
      </p>
    </div>
  );
}
