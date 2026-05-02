import { Link } from "react-router-dom";

export function HomePage() {
  return (
    <>
      {/* ── Hero tile (light) ───────────────────────────────────────── */}
      <section className="tile tile-light" style={{ paddingBottom: 64 }}>
        <div className="container" style={{ textAlign: "center", paddingTop: 24 }}>
          <h1
            className="t-hero-display"
            style={{ margin: 0, marginBottom: 16 }}
          >
            Nova Craft Launcher
          </h1>
          <p
            className="t-lead muted"
            style={{ maxWidth: 720, margin: "0 auto 32px" }}
          >
            面向 LLM Agent 时代的 Minecraft 启动器。一句自然语言，跑起一套整合包。
          </p>
          <div className="row" style={{ justifyContent: "center", gap: 12 }}>
            <Link to="/instances" className="btn btn-primary">
              开始使用
            </Link>
            <Link to="/settings" className="btn btn-secondary">
              查看设置
            </Link>
          </div>
        </div>
      </section>

      {/* ── Capabilities tile (dark) ────────────────────────────────── */}
      <section className="tile tile-dark">
        <div
          className="container"
          style={{ display: "grid", gap: 48, textAlign: "center" }}
        >
          <h2 className="t-display-lg" style={{ margin: 0 }}>
            原版 · Forge · Fabric · NeoForge
          </h2>
          <p className="t-lead-airy muted-on-dark" style={{ margin: 0 }}>
            BMCLAPI 镜像加速 · 自动选 Java · 离线账号 · Mod 元数据识别
          </p>

          <div
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(auto-fit, minmax(200px, 1fr))",
              gap: 32,
              maxWidth: 880,
              margin: "0 auto",
              textAlign: "left",
            }}
          >
            {[
              { kpi: "4", label: "Mod 加载器" },
              { kpi: "8", label: "Java 厂商扫描" },
              { kpi: "81", label: "单元测试覆盖" },
              { kpi: "0", label: "外部依赖运行时" },
            ].map((s) => (
              <div key={s.label}>
                <div
                  className="t-display-md"
                  style={{ color: "var(--primary-on-dark)", margin: 0 }}
                >
                  {s.kpi}
                </div>
                <div
                  className="t-caption muted-on-dark"
                  style={{ marginTop: 4 }}
                >
                  {s.label}
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* ── Sprint progress tile (parchment) ────────────────────────── */}
      <section className="tile tile-parchment">
        <div className="container">
          <h2
            className="t-display-md"
            style={{ margin: 0, marginBottom: 8, textAlign: "center" }}
          >
            阶段 1 · 已交付
          </h2>
          <p
            className="t-caption muted"
            style={{ textAlign: "center", marginBottom: 40 }}
          >
            按子提交切分，每个 sprint 独立可验证
          </p>
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(auto-fit, minmax(260px, 1fr))",
              gap: 20,
            }}
          >
            {[
              ["Sprint 0", "Workspace + 9 业务 crate 骨架"],
              ["Sprint 1", "Vanilla 1.21.1 一键装并启动"],
              ["Sprint 2", "BMCLAPI 镜像池 + 韧性下载"],
              ["Sprint 3a", "Fabric loader 接入"],
              ["Sprint 3b", "Forge installer + processors 引擎"],
              ["Sprint 4a", "Mod 元数据解析 + 启用/禁用"],
              ["Sprint 4b", "NeoForge installer (复用 Forge 引擎)"],
              ["Sprint 5", "Java 全面扫描 + 内存推荐"],
            ].map(([k, v]) => (
              <div key={k} className="card-utility">
                <div className="t-caption-strong muted">{k}</div>
                <div className="t-body-strong" style={{ marginTop: 4 }}>
                  {v}
                </div>
              </div>
            ))}
          </div>
          <div style={{ textAlign: "center", marginTop: 48 }}>
            <Link to="/instances" className="btn btn-primary">
              去装一个版本
            </Link>
          </div>
        </div>
      </section>
    </>
  );
}
