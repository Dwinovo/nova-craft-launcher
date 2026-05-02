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
    <div>
      <h1>Mod 管理</h1>
      <div style={{ display: "flex", gap: 8, marginBottom: 16, alignItems: "center" }}>
        <input
          placeholder="实例名"
          value={instanceName}
          onChange={(e) => setInstanceName(e.target.value)}
          style={inputStyle}
        />
        <button onClick={refresh} disabled={loading} style={btnStyle(loading)}>
          {loading ? "扫描中…" : "扫描 mods/"}
        </button>
      </div>
      {error && <p style={{ color: "#d44" }}>错误：{error}</p>}
      {mods && mods.length === 0 && (
        <p style={{ color: "#6e6e76" }}>
          mods/ 目录为空或未识别到任何 mod。请把 .jar 放到{" "}
          <code>./data/instances/{instanceName}/.minecraft/mods/</code>
        </p>
      )}
      {mods && mods.length > 0 && (
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {mods.map((m) => (
            <div key={m.filePath} style={modCardStyle(m.enabled)}>
              <div style={{ flex: 1 }}>
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <strong style={{ fontSize: 15 }}>{m.name}</strong>
                  <span style={badgeStyle(m.loader)}>{m.loader}</span>
                  <span style={{ fontSize: 12, color: "#6e6e76" }}>v{m.version}</span>
                  {!m.enabled && <span style={disabledBadge}>禁用</span>}
                </div>
                <div style={{ fontSize: 12, color: "#6e6e76", marginTop: 2 }}>
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
                  <div style={{ fontSize: 12, marginTop: 4 }}>{m.description}</div>
                )}
                {m.dependencies.length > 0 && (
                  <div style={{ fontSize: 11, color: "#6e6e76", marginTop: 4 }}>
                    依赖：
                    {m.dependencies.map((d, i) => (
                      <span key={i} style={{ marginRight: 8 }}>
                        <code>{d.modId}</code>
                        {d.versionRange && ` ${d.versionRange}`}
                        {!d.mandatory && " (可选)"}
                      </span>
                    ))}
                  </div>
                )}
              </div>
              <button onClick={() => toggle(m)} style={toggleBtnStyle(m.enabled)}>
                {m.enabled ? "禁用" : "启用"}
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

const inputStyle: React.CSSProperties = {
  padding: "6px 10px",
  border: "1px solid #cfcfd4",
  borderRadius: 6,
  fontSize: 14,
};

function btnStyle(disabled: boolean): React.CSSProperties {
  return {
    padding: "6px 14px",
    background: disabled ? "#a8a8b0" : "#396cd8",
    color: "white",
    border: "none",
    borderRadius: 6,
    cursor: disabled ? "not-allowed" : "pointer",
    fontSize: 13,
  };
}

function modCardStyle(enabled: boolean): React.CSSProperties {
  return {
    display: "flex",
    alignItems: "flex-start",
    gap: 12,
    padding: "10px 12px",
    border: "1px solid #cfcfd4",
    borderRadius: 6,
    background: enabled ? "white" : "#f0f0f4",
    opacity: enabled ? 1 : 0.7,
  };
}

function badgeStyle(loader: string): React.CSSProperties {
  const colors: Record<string, string> = {
    forge: "#9d2424",
    fabric: "#8a6d3b",
    neoforge: "#d97706",
  };
  return {
    display: "inline-block",
    padding: "1px 6px",
    fontSize: 10,
    fontWeight: 600,
    color: "white",
    background: colors[loader] ?? "#6e6e76",
    borderRadius: 3,
    textTransform: "uppercase",
  };
}

const disabledBadge: React.CSSProperties = {
  display: "inline-block",
  padding: "1px 6px",
  fontSize: 10,
  fontWeight: 600,
  color: "#6e6e76",
  border: "1px solid #cfcfd4",
  borderRadius: 3,
};

function toggleBtnStyle(enabled: boolean): React.CSSProperties {
  return {
    padding: "5px 12px",
    background: enabled ? "#dc3545" : "#28a745",
    color: "white",
    border: "none",
    borderRadius: 4,
    cursor: "pointer",
    fontSize: 12,
    minWidth: 56,
  };
}
