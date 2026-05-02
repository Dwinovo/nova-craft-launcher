import { useEffect, useState } from "react";
import {
  getConfig,
  getPaths,
  setConfig,
  type AppConfig,
  type MirrorPolicy,
  type PathInfo,
} from "../lib/api";

export function SettingsPage() {
  const [paths, setPaths] = useState<PathInfo | null>(null);
  const [config, setConfigState] = useState<AppConfig | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    Promise.all([getPaths(), getConfig()])
      .then(([p, c]) => {
        setPaths(p);
        setConfigState(c);
      })
      .catch((e) => setError(String(e)));
  }, []);

  async function updatePolicy(policy: MirrorPolicy) {
    if (!config) return;
    const next = { ...config, mirror_policy: policy };
    setConfigState(next);
    setSaving(true);
    try {
      await setConfig(next);
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  async function updateConcurrency(n: number) {
    if (!config) return;
    const next = { ...config, max_concurrent_downloads: n };
    setConfigState(next);
    setSaving(true);
    try {
      await setConfig(next);
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  const pathRows: [string, string | undefined][] = paths
    ? [
        ["模式", paths.mode],
        ["数据根目录", paths.dataRoot],
        ["实例", paths.instances],
        ["共享 assets", paths.sharedAssets],
        ["共享 libraries", paths.sharedLibraries],
        ["共享 versions", paths.sharedVersions],
        ["配置文件", paths.configFile],
        ["日志", paths.logs],
        ["缓存", paths.cache],
      ]
    : [];

  return (
    <div>
      <h1>设置</h1>
      {error && <p style={{ color: "#d44" }}>错误：{error}</p>}

      {config && (
        <section style={{ marginBottom: 32 }}>
          <h2 style={{ fontSize: 16, marginTop: 24, marginBottom: 8 }}>
            下载策略 {saving && <span style={savingHint}>保存中…</span>}
          </h2>
          <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
            {(
              [
                { v: "auto", label: "自动 (推荐)", note: "BMCLAPI 优先，失败回落官方" },
                { v: "bmclapi", label: "仅 BMCLAPI", note: "官方仅作为 BMCLAPI 不接管的兜底" },
                { v: "official", label: "仅官方源", note: "海外网络或调试时使用" },
              ] as const
            ).map((opt) => (
              <button
                key={opt.v}
                onClick={() => updatePolicy(opt.v)}
                style={mirrorBtnStyle(config.mirror_policy === opt.v)}
                title={opt.note}
              >
                {opt.label}
              </button>
            ))}
          </div>
          <div style={{ marginTop: 12 }}>
            <label style={{ display: "block", fontSize: 13, marginBottom: 4 }}>
              全局并发下载数：<strong>{config.max_concurrent_downloads}</strong>
            </label>
            <input
              type="range"
              min={1}
              max={32}
              value={config.max_concurrent_downloads}
              onChange={(e) => updateConcurrency(Number(e.target.value))}
              style={{ width: 240 }}
            />
          </div>
        </section>
      )}

      <h2 style={{ fontSize: 16, marginTop: 24, marginBottom: 8 }}>
        目录布局
      </h2>
      {!paths && !error && <p>加载中…</p>}
      {paths && (
        <table className="paths-table">
          <tbody>
            {pathRows.map(([k, v]) => (
              <tr key={k}>
                <td className="path-key">{k}</td>
                <td className="path-val">
                  <code>{v}</code>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}

function mirrorBtnStyle(active: boolean): React.CSSProperties {
  return {
    padding: "8px 14px",
    border: active ? "2px solid #396cd8" : "1px solid #cfcfd4",
    background: active ? "#e9efff" : "transparent",
    color: active ? "#1a1a1f" : "#6e6e76",
    borderRadius: 6,
    cursor: "pointer",
    fontSize: 14,
    fontWeight: active ? 600 : 400,
  };
}

const savingHint: React.CSSProperties = {
  fontSize: 12,
  color: "#6e6e76",
  marginLeft: 8,
};
