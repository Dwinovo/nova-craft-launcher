import { useEffect, useState } from "react";
import {
  getConfig,
  getPaths,
  setConfig,
  type AppConfig,
  type MirrorPolicy,
  type PathInfo,
} from "../lib/api";

const mirrorOptions: { v: MirrorPolicy; label: string; note: string }[] = [
  { v: "auto", label: "自动", note: "BMCLAPI 优先，失败回落官方" },
  { v: "bmclapi", label: "仅 BMCLAPI", note: "官方仅作为不接管 URL 的兜底" },
  { v: "official", label: "仅官方源", note: "海外网络或调试时使用" },
];

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

  async function update(next: AppConfig) {
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

  return (
    <section className="section">
      <div className="container">
        <div style={{ marginBottom: 32 }}>
          <h1 className="t-display-lg" style={{ margin: 0 }}>
            设置
          </h1>
          <p className="t-lead muted" style={{ margin: "8px 0 0" }}>
            镜像 · 并发 · 目录布局
          </p>
        </div>

        {error && (
          <p className="t-caption" style={{ color: "#d44" }}>
            错误：{error}
          </p>
        )}

        {/* ── Mirror policy ───────────────────────────────────────── */}
        {config && (
          <section style={{ marginBottom: 48 }}>
            <div
              className="row"
              style={{ marginBottom: 16, justifyContent: "space-between" }}
            >
              <h2 className="t-display-md" style={{ margin: 0 }}>
                下载策略
              </h2>
              {saving && (
                <span className="t-caption muted">保存中…</span>
              )}
            </div>
            <div className="row-wrap" style={{ gap: 12, marginBottom: 24 }}>
              {mirrorOptions.map((opt) => (
                <button
                  key={opt.v}
                  onClick={() => update({ ...config, mirror_policy: opt.v })}
                  className={`chip ${config.mirror_policy === opt.v ? "chip-selected" : ""}`}
                  title={opt.note}
                >
                  {opt.label}
                </button>
              ))}
            </div>
            <p className="t-caption muted" style={{ marginBottom: 24 }}>
              {mirrorOptions.find((o) => o.v === config.mirror_policy)?.note}
            </p>

            <div
              className="card-utility"
              style={{ maxWidth: 480 }}
            >
              <div
                className="row"
                style={{ justifyContent: "space-between", marginBottom: 12 }}
              >
                <span className="t-body-strong">全局并发下载数</span>
                <span
                  className="t-display-md"
                  style={{ margin: 0, color: "var(--primary)" }}
                >
                  {config.max_concurrent_downloads}
                </span>
              </div>
              <input
                type="range"
                min={1}
                max={32}
                value={config.max_concurrent_downloads}
                onChange={(e) =>
                  update({
                    ...config,
                    max_concurrent_downloads: Number(e.target.value),
                  })
                }
                style={{ width: "100%", accentColor: "var(--primary)" }}
              />
              <div
                className="row"
                style={{ justifyContent: "space-between", marginTop: 4 }}
              >
                <span className="t-fine-print muted">1</span>
                <span className="t-fine-print muted">32</span>
              </div>
            </div>
          </section>
        )}

        {/* ── Path layout ─────────────────────────────────────────── */}
        <section>
          <h2
            className="t-display-md"
            style={{ margin: 0, marginBottom: 16 }}
          >
            目录布局
          </h2>
          {!paths && !error && <p className="t-caption muted">加载中…</p>}
          {paths && (
            <div className="card-utility" style={{ padding: 0, overflow: "hidden" }}>
              <table className="data-table">
                <tbody>
                  {[
                    ["模式", paths.mode],
                    ["数据根目录", paths.dataRoot],
                    ["实例", paths.instances],
                    ["共享 assets", paths.sharedAssets],
                    ["共享 libraries", paths.sharedLibraries],
                    ["共享 versions", paths.sharedVersions],
                    ["配置文件", paths.configFile],
                    ["日志", paths.logs],
                    ["缓存", paths.cache],
                  ].map(([k, v]) => (
                    <tr key={k}>
                      <td style={{ width: 160, color: "var(--ink-muted-48)" }}>
                        {k}
                      </td>
                      <td>
                        <code>{v}</code>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </section>
      </div>
    </section>
  );
}
