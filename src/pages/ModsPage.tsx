import { useEffect, useState } from "react";
import { scanMods, setModEnabled, type ModEntry } from "../lib/api";

const loaderBadgeClass: Record<string, string> = {
  forge: "badge-tag-orange",
  fabric: "badge-tag-purple",
  neoforge: "badge-tag-pink",
};

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

  const enabledCount = mods?.filter((m) => m.enabled).length ?? 0;
  const total = mods?.length ?? 0;

  return (
    <div style={{ padding: "32px 40px", maxWidth: 1180, margin: "0 auto" }}>
      <header style={{ marginBottom: 24 }}>
        <h1 className="t-h1" style={{ margin: 0, marginBottom: 6 }}>
          Mod 管理
        </h1>
        <p className="t-body muted" style={{ margin: 0 }}>
          扫描 <code>.minecraft/mods</code>{" "}
          下的 mods.toml / fabric.mod.json / neoforge.mods.toml
        </p>
      </header>

      {/* ── Toolbar ──────────────────────────────────────────────────── */}
      <div
        className="row"
        style={{ marginBottom: 16, justifyContent: "space-between" }}
      >
        <div className="row" style={{ gap: 12 }}>
          <input
            className="text-input"
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
        {mods && (
          <div className="row" style={{ gap: 8 }}>
            <span className="badge badge-tag-green">{enabledCount} 启用</span>
            <span className="badge badge-tag-gray">
              {total - enabledCount} 禁用
            </span>
          </div>
        )}
      </div>

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

      {mods && mods.length === 0 && (
        <div className="card-feature" style={{ textAlign: "center" }}>
          <p className="t-body muted" style={{ margin: 0 }}>
            mods/ 目录为空或未识别到任何 mod。把 .jar 放到
          </p>
          <p style={{ margin: "8px 0 0" }}>
            <code>./data/instances/{instanceName}/.minecraft/mods/</code>
          </p>
          <p
            className="t-body-sm muted"
            style={{ margin: "8px 0 0" }}
          >
            后重新扫描。
          </p>
        </div>
      )}

      {mods && mods.length > 0 && (
        <div style={{ display: "grid", gap: 12 }}>
          {mods.map((m) => (
            <article
              key={m.filePath}
              className="card-base"
              style={{
                display: "flex",
                gap: 20,
                alignItems: "flex-start",
                opacity: m.enabled ? 1 : 0.55,
                padding: 20,
              }}
            >
              <div style={{ flex: 1, minWidth: 0 }}>
                <div className="row-wrap" style={{ gap: 8, alignItems: "center" }}>
                  <span
                    className="t-h5"
                    style={{ margin: 0, wordBreak: "break-all" }}
                  >
                    {m.name}
                  </span>
                  <span
                    className={`badge ${loaderBadgeClass[m.loader] ?? "badge-tag-gray"}`}
                  >
                    {m.loader}
                  </span>
                  <span className="t-caption muted">v{m.version}</span>
                  {!m.enabled && (
                    <span className="badge badge-tag-gray">禁用</span>
                  )}
                </div>
                <div
                  className="t-caption"
                  style={{
                    marginTop: 6,
                    color: "var(--steel)",
                    wordBreak: "break-word",
                  }}
                >
                  <code>{m.modId}</code>
                  {m.mcVersionRange && (
                    <>
                      {" · "}MC <code>{m.mcVersionRange}</code>
                    </>
                  )}
                  {" · "}side <code>{m.side}</code>
                  {m.authors.length > 0 && <> · {m.authors.join(", ")}</>}
                </div>
                {m.description && (
                  <p
                    className="t-body-sm"
                    style={{
                      marginTop: 8,
                      color: "var(--charcoal)",
                      marginBottom: 0,
                    }}
                  >
                    {m.description}
                  </p>
                )}
                {m.dependencies.length > 0 && (
                  <div
                    className="row-wrap"
                    style={{ marginTop: 10, gap: 6 }}
                  >
                    <span className="eyebrow" style={{ marginRight: 4 }}>
                      依赖
                    </span>
                    {m.dependencies.map((d, i) => (
                      <span
                        key={i}
                        className="badge badge-tag-gray"
                        title={d.versionRange ?? ""}
                      >
                        <code style={{ background: "transparent", padding: 0 }}>
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
  );
}
