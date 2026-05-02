import { useEffect, useState } from "react";
import {
  recommendMemory,
  scanJava,
  type JavaInfo,
  type MemoryRecommendation,
} from "../lib/api";

export function JavaPage() {
  const [list, setList] = useState<JavaInfo[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [memVanilla, setMemVanilla] = useState<MemoryRecommendation | null>(null);
  const [memLoaderLight, setMemLoaderLight] = useState<MemoryRecommendation | null>(null);
  const [memLoaderHeavy, setMemLoaderHeavy] = useState<MemoryRecommendation | null>(null);

  async function refresh() {
    setLoading(true);
    setError(null);
    try {
      const [data, mv, ml, mh] = await Promise.all([
        scanJava(),
        recommendMemory(false, 0),
        recommendMemory(true, 30),
        recommendMemory(true, 200),
      ]);
      setList(data);
      setMemVanilla(mv);
      setMemLoaderLight(ml);
      setMemLoaderHeavy(mh);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    refresh();
  }, []);

  return (
    <div style={{ padding: "32px 40px", maxWidth: 1180, margin: "0 auto" }}>
      <header
        className="row"
        style={{ justifyContent: "space-between", marginBottom: 24 }}
      >
        <div>
          <h1 className="t-h1" style={{ margin: 0, marginBottom: 6 }}>
            Java 配置
          </h1>
          <p className="t-body muted" style={{ margin: 0 }}>
            JAVA_HOME · PATH · 注册表 · Adoptium / Microsoft / Zulu / Liberica · Mojang JRE
          </p>
        </div>
        <button
          onClick={refresh}
          disabled={loading}
          className="btn btn-primary"
        >
          {loading ? "扫描中…" : "重新扫描"}
        </button>
      </header>

      {/* ── 内存推荐 (4 张柔色卡片) ─────────────────────────────────── */}
      {memVanilla && (
        <section style={{ marginBottom: 32 }}>
          <div
            className="row"
            style={{
              justifyContent: "space-between",
              marginBottom: 12,
              alignItems: "baseline",
            }}
          >
            <h2 className="t-h3" style={{ margin: 0 }}>
              内存推荐
            </h2>
            <span className="eyebrow">
              系统总内存 {Math.round((memVanilla.systemTotalMb / 1024) * 10) / 10} GB
            </span>
          </div>
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))",
              gap: 12,
            }}
          >
            <MemCard
              tint="lavender"
              title="原版"
              subtitle="无 mod"
              rec={memVanilla}
            />
            <MemCard
              tint="sky"
              title="带 Loader"
              subtitle="约 30 mods"
              rec={memLoaderLight}
            />
            <MemCard
              tint="peach"
              title="重度整合包"
              subtitle="约 200 mods"
              rec={memLoaderHeavy}
            />
            <div
              className="card-tint-mint"
              style={{ display: "flex", flexDirection: "column", gap: 6 }}
            >
              <div className="eyebrow">系统总内存</div>
              <div
                style={{
                  fontSize: 28,
                  fontWeight: 600,
                  color: "var(--brand-green)",
                }}
              >
                {Math.round((memVanilla.systemTotalMb / 1024) * 10) / 10} GB
              </div>
              <div className="t-caption muted-deep">
                {memVanilla.systemTotalMb} MB · 上限 50%
              </div>
            </div>
          </div>
        </section>
      )}

      {/* ── 已识别 Java ─────────────────────────────────────────────── */}
      <section>
        <h2 className="t-h3" style={{ margin: 0, marginBottom: 12 }}>
          已识别 Java
        </h2>

        {error && (
          <div
            className="card-base"
            style={{
              background: "var(--tint-rose)",
              borderColor: "var(--brand-pink-deep)",
              color: "var(--brand-pink-deep)",
              marginBottom: 16,
            }}
          >
            错误：{error}
          </div>
        )}

        {list && list.length === 0 && (
          <div className="card-feature">
            <p className="t-body muted" style={{ margin: 0 }}>
              未发现 Java 运行时。请安装 JDK 21（推荐 Adoptium / Microsoft Build of OpenJDK）后重新扫描。
            </p>
          </div>
        )}

        {list && list.length > 0 && (
          <div className="card-base" style={{ padding: 0, overflow: "hidden" }}>
            <table className="data-table">
              <thead>
                <tr>
                  <th>主版本</th>
                  <th>完整版本</th>
                  <th>厂商</th>
                  <th>架构</th>
                  <th>来源</th>
                  <th>路径</th>
                </tr>
              </thead>
              <tbody>
                {list.map((j) => (
                  <tr key={j.path}>
                    <td>
                      <span
                        className="badge badge-tag-purple"
                        style={{ fontSize: 13, padding: "3px 10px" }}
                      >
                        {j.versionMajor}
                      </span>
                    </td>
                    <td>{j.versionFull}</td>
                    <td>{j.vendor}</td>
                    <td>{j.arch}</td>
                    <td>
                      <span className="badge badge-tag-gray">{j.source}</span>
                    </td>
                    <td>
                      <code>{j.path}</code>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>
    </div>
  );
}

function MemCard(props: {
  tint: "lavender" | "sky" | "peach";
  title: string;
  subtitle: string;
  rec: MemoryRecommendation | null;
}) {
  const { tint, title, subtitle, rec } = props;
  return (
    <div className={`card-tint-${tint}`} style={{ display: "flex", flexDirection: "column", gap: 6 }}>
      <div className="eyebrow">{title}</div>
      <div className="t-caption muted-deep">{subtitle}</div>
      {rec && (
        <>
          <div
            style={{
              fontSize: 28,
              fontWeight: 600,
              color: "var(--charcoal)",
              marginTop: 4,
            }}
          >
            {rec.maxMb}
            <span
              style={{
                fontSize: 14,
                fontWeight: 500,
                color: "var(--slate)",
                marginLeft: 4,
              }}
            >
              MB
            </span>
          </div>
          <div className="t-caption muted-deep">
            -Xms {rec.minMb} · -Xmx {rec.maxMb}
          </div>
        </>
      )}
    </div>
  );
}
