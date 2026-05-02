import { useEffect, useState } from "react";
import { scanMods, setModEnabled, type ModEntry } from "../lib/api";

export function ModsPage() {
  const [instanceName, setInstanceName] = useState("test-instance");
  const [mods, setMods] = useState<ModEntry[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function refresh() {
    setLoading(true);
    setError(null);
    try {
      const m = await scanMods(instanceName);
      setMods(m);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function toggle(mod: ModEntry) {
    try {
      await setModEnabled(mod.filePath, !mod.enabled);
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <section className="section">
      <div className="container">
        <div style={{ marginBottom: 32 }}>
          <h1 className="t-display-lg" style={{ margin: 0 }}>
            Mod 管理
          </h1>
          <p className="t-lead muted" style={{ margin: "8px 0 0" }}>
            扫描 <code className="mono-inline">.minecraft/mods</code> 下的
            mods.toml / fabric.mod.json / neoforge.mods.toml
          </p>
        </div>

        <div className="row" style={{ marginBottom: 24, gap: 12 }}>
          <input
            className="input-text"
            style={{ width: 240 }}
            placeholder="实例名"
            value={instanceName}
            onChange={(e) => setInstanceName(e.target.value)}
          />
          <button
            onClick={refresh}
            disabled={loading}
            className="btn btn-primary"
          >
            {loading ? "扫描中…" : "扫描"}
          </button>
        </div>

        {error && (
          <p className="t-caption" style={{ color: "#d44", marginBottom: 16 }}>
            错误：{error}
          </p>
        )}

        {mods && mods.length === 0 && (
          <div className="card-utility" style={{ textAlign: "center" }}>
            <p className="t-body muted">
              mods/ 目录为空或未识别到任何 mod。把 .jar 放到
              <br />
              <code className="mono-inline" style={{ marginTop: 8, display: "inline-block" }}>
                ./data/instances/{instanceName}/.minecraft/mods/
              </code>
              <br />
              后重新扫描。
            </p>
          </div>
        )}

        {mods && mods.length > 0 && (
          <div style={{ display: "grid", gap: 16 }}>
            {mods.map((m) => (
              <article
                key={m.filePath}
                className="card-utility"
                style={{
                  display: "flex",
                  gap: 20,
                  alignItems: "flex-start",
                  opacity: m.enabled ? 1 : 0.55,
                }}
              >
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div className="row-wrap" style={{ alignItems: "center", gap: 10 }}>
                    <span className="t-body-strong" style={{ wordBreak: "break-all" }}>
                      {m.name}
                    </span>
                    <span className={`badge badge-loader-${m.loader}`}>
                      {m.loader}
                    </span>
                    <span className="t-caption muted">v{m.version}</span>
                    {!m.enabled && <span className="badge badge-muted">禁用</span>}
                  </div>
                  <div
                    className="t-caption muted"
                    style={{ marginTop: 6, wordBreak: "break-word" }}
                  >
                    <code className="mono-inline">{m.modId}</code>
                    {m.mcVersionRange && (
                      <>
                        {" · "}MC <code className="mono-inline">{m.mcVersionRange}</code>
                      </>
                    )}
                    {" · "}side{" "}
                    <code className="mono-inline">{m.side}</code>
                    {m.authors.length > 0 && <> · {m.authors.join(", ")}</>}
                  </div>
                  {m.description && (
                    <p
                      className="t-caption"
                      style={{ marginTop: 8, color: "var(--ink-muted-80)" }}
                    >
                      {m.description}
                    </p>
                  )}
                  {m.dependencies.length > 0 && (
                    <div className="row-wrap" style={{ marginTop: 8, gap: 6 }}>
                      <span className="t-caption muted">依赖：</span>
                      {m.dependencies.map((d, i) => (
                        <span
                          key={i}
                          className="t-caption"
                          style={{
                            background: "var(--canvas-parchment)",
                            padding: "2px 8px",
                            borderRadius: "var(--r-pill)",
                            color: "var(--ink-muted-80)",
                          }}
                        >
                          <code className="mono-inline" style={{ background: "transparent", padding: 0 }}>
                            {d.modId}
                          </code>
                          {d.versionRange && ` ${d.versionRange}`}
                          {!d.mandatory && " · 可选"}
                        </span>
                      ))}
                    </div>
                  )}
                </div>
                <button
                  onClick={() => toggle(m)}
                  className={m.enabled ? "btn btn-secondary" : "btn btn-primary"}
                  style={{ minWidth: 76 }}
                >
                  {m.enabled ? "禁用" : "启用"}
                </button>
              </article>
            ))}
          </div>
        )}
      </div>
    </section>
  );
}
