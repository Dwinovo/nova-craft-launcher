import { Link } from "react-router-dom";

export function HomePage() {
  return (
    <div style={{ padding: "32px 40px", maxWidth: 1180, margin: "0 auto" }}>
      {/* ── Navy hero panel (Notion signature) ──────────────────────── */}
      <section className="hero-band" style={{ marginBottom: 32 }}>
        <span
          className="hero-dot"
          style={{
            top: 80,
            right: 200,
            width: 8,
            height: 8,
            background: "var(--brand-teal)",
          }}
        />
        <span
          className="hero-dot"
          style={{
            top: 36,
            right: 80,
            width: 6,
            height: 6,
            background: "var(--brand-purple-300)",
          }}
        />
        <span
          className="hero-dot"
          style={{
            bottom: 60,
            left: 120,
            width: 12,
            height: 12,
            background: "var(--brand-orange)",
          }}
        />
        <span
          className="hero-dot"
          style={{
            top: 120,
            left: 64,
            width: 5,
            height: 5,
            background: "var(--brand-green)",
          }}
        />

        <div style={{ position: "relative", zIndex: 1, maxWidth: 720 }}>
          <span
            className="badge badge-purple-solid badge-pill"
            style={{ marginBottom: 16 }}
          >
            ALPHA · Sprint 5 已完成
          </span>
          <h1 className="t-display-lg" style={{ margin: 0, marginBottom: 12 }}>
            一句话，跑起整合包
          </h1>
          <p
            className="t-subtitle on-dark-muted"
            style={{ margin: 0, marginBottom: 24, maxWidth: 560 }}
          >
            原版 · Forge · Fabric · NeoForge 一键安装，BMCLAPI
            镜像加速，Java 自动选，Mod 元数据识别。
          </p>
          <div className="row" style={{ gap: 12 }}>
            <Link to="/instances" className="btn btn-primary">
              开始安装版本
            </Link>
            <Link to="/settings" className="btn btn-secondary-on-dark">
              查看设置
            </Link>
          </div>
        </div>
      </section>

      {/* ── Capability cards (pastel tints,Notion product grid feel) ── */}
      <section style={{ marginBottom: 32 }}>
        <div
          className="row"
          style={{
            justifyContent: "space-between",
            marginBottom: 16,
            alignItems: "baseline",
          }}
        >
          <h2 className="t-h2" style={{ margin: 0 }}>
            能力总览
          </h2>
          <span className="eyebrow">8 sprints · 81 unit tests</span>
        </div>
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fit, minmax(240px, 1fr))",
            gap: 16,
          }}
        >
          <CapabilityCard
            tint="lavender"
            title="多加载器"
            kpi="4"
            note="Forge · Fabric · NeoForge · 原版"
          />
          <CapabilityCard
            tint="sky"
            title="镜像加速"
            kpi="BMCLAPI"
            note="评分降级,自动回落官方源"
          />
          <CapabilityCard
            tint="mint"
            title="Java 检测"
            kpi="8 源"
            note="JAVA_HOME · 注册表 · 厂商目录 · Mojang JRE"
          />
          <CapabilityCard
            tint="peach"
            title="Mod 识别"
            kpi="3 格式"
            note="mods.toml · fabric.mod.json · neoforge.mods.toml"
          />
        </div>
      </section>

      {/* ── Sprint 进度 (utility 卡片网格) ───────────────────────────── */}
      <section>
        <div
          className="row"
          style={{
            justifyContent: "space-between",
            marginBottom: 16,
            alignItems: "baseline",
          }}
        >
          <h2 className="t-h2" style={{ margin: 0 }}>
            阶段 1 已交付
          </h2>
          <Link
            to="/instances"
            className="btn-link"
            style={{ fontWeight: 500 }}
          >
            去装一个版本 →
          </Link>
        </div>
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fit, minmax(260px, 1fr))",
            gap: 12,
          }}
        >
          {[
            ["Sprint 0", "Workspace + 9 业务 crate 骨架"],
            ["Sprint 1", "Vanilla 1.21.1 一键装并启动"],
            ["Sprint 2", "BMCLAPI 镜像池 + 韧性下载"],
            ["Sprint 3a", "Fabric loader 接入"],
            ["Sprint 3b", "Forge installer + processors"],
            ["Sprint 4a", "Mod 元数据 + 启用/禁用"],
            ["Sprint 4b", "NeoForge installer"],
            ["Sprint 5", "Java 全面扫描 + 内存推荐"],
          ].map(([k, v]) => (
            <div key={k} className="card-base" style={{ padding: 16 }}>
              <div className="eyebrow" style={{ marginBottom: 4 }}>
                {k}
              </div>
              <div className="t-body-medium">{v}</div>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}

function CapabilityCard(props: {
  tint: "lavender" | "sky" | "mint" | "peach";
  title: string;
  kpi: string;
  note: string;
}) {
  const { tint, title, kpi, note } = props;
  return (
    <div className={`card-tint-${tint}`}>
      <div className="eyebrow" style={{ marginBottom: 8 }}>
        {title}
      </div>
      <div
        className="t-h2"
        style={{
          margin: 0,
          marginBottom: 8,
          fontSize: 36,
        }}
      >
        {kpi}
      </div>
      <div className="t-body-sm muted-deep">{note}</div>
    </div>
  );
}
