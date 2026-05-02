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
    <>
      {/* ── Hero: memory recommendation ─────────────────────────────── */}
      <section className="tile tile-parchment" style={{ paddingTop: 48, paddingBottom: 48 }}>
        <div className="container">
          <div
            style={{
              display: "flex",
              alignItems: "flex-end",
              justifyContent: "space-between",
              marginBottom: 24,
              flexWrap: "wrap",
              gap: 12,
            }}
          >
            <div>
              <h1 className="t-display-lg" style={{ margin: 0 }}>
                Java 配置
              </h1>
              <p className="t-lead muted" style={{ margin: "8px 0 0" }}>
                JAVA_HOME · PATH · 注册表 · Adoptium / Microsoft / Zulu / Liberica · Mojang JRE
              </p>
            </div>
            <button onClick={refresh} disabled={loading} className="btn btn-primary">
              {loading ? "扫描中…" : "重新扫描"}
            </button>
          </div>

          {memVanilla && (
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))",
                gap: 16,
              }}
            >
              <MemoryCard
                title="原版"
                subtitle="无 mod"
                rec={memVanilla}
              />
              <MemoryCard
                title="带 Loader"
                subtitle="约 30 mods"
                rec={memLoaderLight}
              />
              <MemoryCard
                title="重度整合包"
                subtitle="约 200 mods"
                rec={memLoaderHeavy}
              />
              <div className="card-utility">
                <div className="t-caption-strong muted">系统总内存</div>
                <div
                  className="t-display-md"
                  style={{ marginTop: 4, color: "var(--primary)" }}
                >
                  {Math.round((memVanilla.systemTotalMb / 1024) * 10) / 10} GB
                </div>
                <div className="t-caption muted" style={{ marginTop: 4 }}>
                  {memVanilla.systemTotalMb} MB
                </div>
              </div>
            </div>
          )}
        </div>
      </section>

      {/* ── Java table ──────────────────────────────────────────────── */}
      <section className="section">
        <div className="container">
          <h2
            className="t-display-md"
            style={{ margin: 0, marginBottom: 24 }}
          >
            已识别 Java
          </h2>

          {error && (
            <p className="t-caption" style={{ color: "#d44" }}>
              错误：{error}
            </p>
          )}

          {list && list.length === 0 && (
            <div className="card-utility">
              <p className="t-body muted" style={{ margin: 0 }}>
                未发现 Java 运行时。请安装 JDK 21（推荐 Adoptium / Microsoft Build of OpenJDK）后重新扫描。
              </p>
            </div>
          )}

          {list && list.length > 0 && (
            <div
              className="card-utility"
              style={{ padding: 0, overflow: "hidden" }}
            >
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
                        <span className="t-body-strong">{j.versionMajor}</span>
                      </td>
                      <td>{j.versionFull}</td>
                      <td>{j.vendor}</td>
                      <td>{j.arch}</td>
                      <td>
                        <span className="badge badge-muted">{j.source}</span>
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
        </div>
      </section>
    </>
  );
}

function MemoryCard(props: {
  title: string;
  subtitle: string;
  rec: MemoryRecommendation | null;
}) {
  const { title, subtitle, rec } = props;
  return (
    <div className="card-utility">
      <div className="t-caption-strong muted">{title}</div>
      <div className="t-caption muted" style={{ marginTop: 2 }}>{subtitle}</div>
      {rec && (
        <>
          <div
            className="t-display-md"
            style={{ marginTop: 12, color: "var(--ink)" }}
          >
            {rec.maxMb}
            <span
              className="t-caption muted"
              style={{ marginLeft: 4 }}
            >
              MB
            </span>
          </div>
          <div className="t-caption muted" style={{ marginTop: 4 }}>
            -Xms {rec.minMb} · -Xmx {rec.maxMb}
          </div>
        </>
      )}
    </div>
  );
}
