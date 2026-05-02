import { useEffect, useMemo, useRef, useState } from "react";
import {
  installLoader,
  installVersion,
  launchRun,
  listLoaderVersions,
  listVersions,
  onProcessExit,
  onProgress,
  type LoaderKind,
  type LoaderVersion,
  type ProgressEvent,
  type VersionEntry,
  type VersionListResponse,
} from "../lib/api";

type LogLine = { source: string; level: string; message: string };
type LoaderChoice = "vanilla" | LoaderKind;

const loaderTabs: { v: LoaderChoice; label: string }[] = [
  { v: "vanilla", label: "原版" },
  { v: "fabric", label: "Fabric" },
  { v: "forge", label: "Forge" },
  { v: "neo_forge", label: "NeoForge" },
];

export function InstancesPage() {
  const [list, setList] = useState<VersionListResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [filter, setFilter] = useState("");
  const [selected, setSelected] = useState<string | null>(null);
  const [instanceName, setInstanceName] = useState("test-instance");
  const [username, setUsername] = useState("Player1");

  const [loaderChoice, setLoaderChoice] = useState<LoaderChoice>("vanilla");
  const [loaderVersions, setLoaderVersions] = useState<LoaderVersion[] | null>(null);
  const [loaderVersionPick, setLoaderVersionPick] = useState<string | null>(null);
  const [loaderLoading, setLoaderLoading] = useState(false);
  const [loaderError, setLoaderError] = useState<string | null>(null);

  const [installing, setInstalling] = useState(false);
  const [progress, setProgress] = useState<{
    completed: number;
    total: number;
    label: string;
  } | null>(null);
  const [logs, setLogs] = useState<LogLine[]>([]);
  const [processPid, setProcessPid] = useState<number | null>(null);
  const [installComplete, setInstallComplete] = useState<string | null>(null);

  const logsRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    listVersions(true)
      .then((r) => {
        setList(r);
        setSelected(r.latestRelease);
        setLoading(false);
      })
      .catch((e) => {
        console.error(e);
        setLoading(false);
      });
  }, []);

  useEffect(() => {
    if (loaderChoice === "vanilla" || !selected) {
      setLoaderVersions(null);
      setLoaderVersionPick(null);
      return;
    }
    setLoaderLoading(true);
    setLoaderError(null);
    setLoaderVersions(null);
    setLoaderVersionPick(null);
    listLoaderVersions(loaderChoice, selected)
      .then((versions) => {
        setLoaderVersions(versions);
        const stable = versions.find((v) => v.stable) ?? versions[0];
        setLoaderVersionPick(stable?.version ?? null);
      })
      .catch((e) => setLoaderError(String(e)))
      .finally(() => setLoaderLoading(false));
  }, [loaderChoice, selected]);

  useEffect(() => {
    let unlistenProgress: (() => void) | undefined;
    let unlistenExit: (() => void) | undefined;
    onProgress((ev: ProgressEvent) => {
      if (ev.kind === "task_started") {
        setProgress({ completed: 0, total: ev.total_weight, label: ev.label });
      } else if (ev.kind === "task_progress") {
        setProgress({
          completed: ev.completed,
          total: ev.total,
          label: ev.message ?? "",
        });
      } else if (ev.kind === "task_finished") {
        setInstalling(false);
        setProgress(null);
        if (!ev.success) {
          setLogs((l) => [
            ...l,
            { source: "install", level: "error", message: ev.error ?? "失败" },
          ]);
        }
      } else if (ev.kind === "log") {
        setLogs((l) => [
          ...l,
          { source: ev.source, level: ev.level, message: ev.message },
        ]);
      }
    }).then((u) => (unlistenProgress = u));
    onProcessExit((ev) => {
      setLogs((l) => [
        ...l,
        {
          source: "process",
          level: "info",
          message: `进程 ${ev.pid} 退出，code=${ev.exit_code ?? "(signal)"}`,
        },
      ]);
      setProcessPid(null);
    }).then((u) => (unlistenExit = u));
    return () => {
      unlistenProgress?.();
      unlistenExit?.();
    };
  }, []);

  useEffect(() => {
    logsRef.current?.scrollTo(0, logsRef.current.scrollHeight);
  }, [logs]);

  const filtered = useMemo(() => {
    if (!list) return [];
    const q = filter.trim().toLowerCase();
    if (!q) return list.versions.slice(0, 200);
    return list.versions.filter((v) => v.id.toLowerCase().includes(q)).slice(0, 200);
  }, [list, filter]);

  async function handleInstall() {
    if (!selected || installing) return;
    if (loaderChoice !== "vanilla" && !loaderVersionPick) {
      setLogs((l) => [
        ...l,
        { source: "install", level: "error", message: "请先选择 loader 版本" },
      ]);
      return;
    }
    setLogs([]);
    setInstallComplete(null);
    setInstalling(true);
    setProgress({ completed: 0, total: 1, label: "starting…" });
    try {
      let installedId: string;
      if (loaderChoice === "vanilla") {
        await installVersion(instanceName, selected);
        installedId = selected;
      } else {
        installedId = await installLoader(
          loaderChoice,
          selected,
          loaderVersionPick!,
          instanceName
        );
      }
      setInstallComplete(installedId);
      setLogs((l) => [
        ...l,
        {
          source: "install",
          level: "info",
          message: `已开始安装：${installedId}`,
        },
      ]);
    } catch (e) {
      console.error(e);
      setInstalling(false);
      setProgress(null);
      setLogs((l) => [
        ...l,
        { source: "install", level: "error", message: String(e) },
      ]);
    }
  }

  async function handleLaunch() {
    if (!installComplete) return;
    try {
      const pid = await launchRun({
        instanceName,
        versionId: installComplete,
        username,
        minMemMb: 1024,
        maxMemMb: 4096,
      });
      setProcessPid(pid);
      setLogs((l) => [
        ...l,
        { source: "launch", level: "info", message: `已启动，PID=${pid}` },
      ]);
    } catch (e) {
      setLogs((l) => [
        ...l,
        { source: "launch", level: "error", message: String(e) },
      ]);
    }
  }

  const pct = progress
    ? Math.min(100, Math.round((progress.completed / Math.max(progress.total, 1)) * 100))
    : 0;

  return (
    <section className="section">
      <div className="container">
        {loading && <p className="muted">加载版本清单…</p>}

        {list && (
          <>
            {/* ── Heading ──────────────────────────────────────────── */}
            <div style={{ marginBottom: 32 }}>
              <h1 className="t-display-lg" style={{ margin: 0 }}>
                选个版本，开始游戏
              </h1>
              <p className="t-lead muted" style={{ margin: "8px 0 0" }}>
                最新 release{" "}
                <code className="mono-inline">{list.latestRelease}</code>，
                snapshot{" "}
                <code className="mono-inline">{list.latestSnapshot}</code>
              </p>
            </div>

            {/* ── Search + identity inputs ─────────────────────────── */}
            <div
              className="row-wrap"
              style={{ marginBottom: 24, alignItems: "center" }}
            >
              <input
                className="search-input"
                style={{ flex: "1 1 280px", minWidth: 200 }}
                placeholder="搜索版本（例：1.21.1）"
                value={filter}
                onChange={(e) => setFilter(e.target.value)}
              />
              <input
                className="input-text"
                style={{ width: 180 }}
                placeholder="实例名"
                value={instanceName}
                onChange={(e) => setInstanceName(e.target.value)}
              />
              <input
                className="input-text"
                style={{ width: 160 }}
                placeholder="离线用户名"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
              />
            </div>

            {/* ── Version list ─────────────────────────────────────── */}
            <div
              className="card-utility"
              style={{
                padding: 0,
                maxHeight: 320,
                overflow: "auto",
                marginBottom: 32,
              }}
            >
              {filtered.map((v) => (
                <VersionRow
                  key={v.id}
                  v={v}
                  selected={selected === v.id}
                  onSelect={() => setSelected(v.id)}
                />
              ))}
            </div>

            {/* ── Loader picker ────────────────────────────────────── */}
            <div style={{ marginBottom: 32 }}>
              <div
                className="t-caption-strong"
                style={{
                  textTransform: "uppercase",
                  letterSpacing: 0.6,
                  color: "var(--ink-muted-48)",
                  marginBottom: 12,
                }}
              >
                Mod 加载器
              </div>
              <div className="row-wrap">
                {loaderTabs.map((opt) => (
                  <button
                    key={opt.v}
                    className={`chip ${loaderChoice === opt.v ? "chip-selected" : ""}`}
                    onClick={() => setLoaderChoice(opt.v)}
                  >
                    {opt.label}
                  </button>
                ))}
              </div>
              {loaderChoice !== "vanilla" && (
                <div style={{ marginTop: 16 }}>
                  {loaderLoading && <span className="t-caption muted">加载 loader 版本…</span>}
                  {loaderError && (
                    <span className="t-caption" style={{ color: "#d44" }}>
                      错误：{loaderError}
                    </span>
                  )}
                  {loaderVersions && (
                    <select
                      className="input-text"
                      style={{ minWidth: 260 }}
                      value={loaderVersionPick ?? ""}
                      onChange={(e) => setLoaderVersionPick(e.target.value)}
                    >
                      {loaderVersions.map((lv) => (
                        <option key={lv.version} value={lv.version}>
                          {lv.version} {lv.stable ? "(stable)" : "(beta)"}
                        </option>
                      ))}
                    </select>
                  )}
                </div>
              )}
            </div>

            {/* ── Action buttons ───────────────────────────────────── */}
            <div className="row" style={{ marginBottom: 24, gap: 12 }}>
              <button
                onClick={handleInstall}
                disabled={!selected || installing}
                className="btn btn-primary"
              >
                {installing
                  ? "安装中…"
                  : loaderChoice === "vanilla"
                  ? `安装原版 ${selected ?? ""}`
                  : `安装 ${loaderChoice} ${loaderVersionPick ?? ""}`}
              </button>
              <button
                onClick={handleLaunch}
                disabled={!installComplete || processPid !== null}
                className="btn btn-secondary"
              >
                {processPid !== null ? `已启动 (PID ${processPid})` : "启动游戏"}
              </button>
            </div>

            {/* ── Progress ─────────────────────────────────────────── */}
            {progress && (
              <div style={{ marginBottom: 24 }}>
                <div
                  className="t-caption muted"
                  style={{ marginBottom: 6 }}
                >
                  {progress.label || "进行中…"} · {progress.completed}/{progress.total} ({pct}%)
                </div>
                <div className="progress-track">
                  <div className="progress-fill" style={{ width: `${pct}%` }} />
                </div>
              </div>
            )}

            {/* ── Log console ──────────────────────────────────────── */}
            {logs.length > 0 && (
              <div>
                <div
                  className="t-caption-strong"
                  style={{
                    textTransform: "uppercase",
                    letterSpacing: 0.6,
                    color: "var(--ink-muted-48)",
                    marginBottom: 8,
                  }}
                >
                  日志
                </div>
                <div ref={logsRef} className="log-console">
                  {logs.map((l, i) => (
                    <div key={i} className={`log-line ${l.level}`}>
                      <span className="log-source">[{l.source}]</span> {l.message}
                    </div>
                  ))}
                </div>
              </div>
            )}
          </>
        )}
      </div>
    </section>
  );
}

function VersionRow(props: {
  v: VersionEntry;
  selected: boolean;
  onSelect: () => void;
}) {
  const { v, selected, onSelect } = props;
  return (
    <div
      onClick={onSelect}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => (e.key === "Enter" || e.key === " ") && onSelect()}
      style={{
        padding: "10px 16px",
        cursor: "pointer",
        background: selected ? "var(--primary)" : "transparent",
        color: selected ? "var(--on-primary)" : "var(--ink)",
        display: "flex",
        justifyContent: "space-between",
        alignItems: "center",
        borderBottom: "1px solid var(--divider-soft)",
      }}
    >
      <span>
        <span className="t-body-strong">{v.id}</span>
        <span
          className="t-caption"
          style={{
            marginLeft: 8,
            opacity: selected ? 0.7 : 0.5,
          }}
        >
          {v.kind}
        </span>
      </span>
      <span
        className="t-caption"
        style={{ opacity: selected ? 0.7 : 0.5 }}
      >
        {v.releaseTime.slice(0, 10)}
      </span>
    </div>
  );
}
