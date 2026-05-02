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
    <div style={{ padding: "32px 40px", maxWidth: 1180, margin: "0 auto" }}>
      <header style={{ marginBottom: 24 }}>
        <h1 className="t-h1" style={{ margin: 0, marginBottom: 6 }}>
          设置
        </h1>
        <p className="t-body muted" style={{ margin: 0 }}>
          镜像 · 并发 · 目录布局
        </p>
      </header>

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

      {/* ── 下载策略 ─────────────────────────────────────────────────── */}
      {config && (
        <section className="card-base" style={{ marginBottom: 16 }}>
          <div
            className="row"
            style={{
              justifyContent: "space-between",
              marginBottom: 12,
              alignItems: "baseline",
            }}
          >
            <h2 className="t-h3" style={{ margin: 0 }}>
              下载策略
            </h2>
            {saving && <span className="t-caption muted">保存中…</span>}
          </div>
          <div className="row-wrap" style={{ gap: 8, marginBottom: 12 }}>
            {mirrorOptions.map((opt) => (
              <button
                key={opt.v}
                onClick={() => update({ ...config, mirror_policy: opt.v })}
                className={`pill-tab ${config.mirror_policy === opt.v ? "active" : ""}`}
                title={opt.note}
              >
                {opt.label}
              </button>
            ))}
          </div>
          <p className="t-body-sm muted" style={{ margin: 0 }}>
            {mirrorOptions.find((o) => o.v === config.mirror_policy)?.note}
          </p>
        </section>
      )}

      {/* ── 并发数 ───────────────────────────────────────────────────── */}
      {config && (
        <section
          className="card-base"
          style={{ marginBottom: 16, maxWidth: 480 }}
        >
          <div
            className="row"
            style={{
              justifyContent: "space-between",
              alignItems: "baseline",
              marginBottom: 8,
            }}
          >
            <h3 className="t-h4" style={{ margin: 0 }}>
              全局并发下载
            </h3>
            <span
              style={{
                fontSize: 24,
                fontWeight: 600,
                color: "var(--primary)",
              }}
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
            style={{ width: "100%" }}
          />
          <div
            className="row"
            style={{
              justifyContent: "space-between",
              marginTop: 4,
              color: "var(--steel)",
              fontSize: 11,
            }}
          >
            <span>1</span>
            <span>16 (默认)</span>
            <span>32</span>
          </div>
        </section>
      )}

      {/* ── 目录布局 ─────────────────────────────────────────────────── */}
      <section>
        <h2 className="t-h3" style={{ margin: 0, marginBottom: 12 }}>
          目录布局
        </h2>
        {!paths && !error && <p className="t-caption muted">加载中…</p>}
        {paths && (
          <div className="card-base" style={{ padding: 0, overflow: "hidden" }}>
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
                    <td
                      style={{
                        width: 160,
                        color: "var(--steel)",
                        fontWeight: 500,
                      }}
                    >
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
  );
}
