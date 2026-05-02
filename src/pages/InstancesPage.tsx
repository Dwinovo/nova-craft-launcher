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
      .catch(() => setLoading(false));
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
    <div style={{ padding: "32px 40px", maxWidth: 1180, margin: "0 auto" }}>
      <header style={{ marginBottom: 24 }}>
        <h1 className="t-h1" style={{ margin: 0, marginBottom: 6 }}>
          实例
        </h1>
        <p className="t-body muted" style={{ margin: 0 }}>
          挑一个 Minecraft 版本，选加载器，开装。
          {list && (
            <>
              {" 最新 release "}
              <code>{list.latestRelease}</code>
              {" · snapshot "}
              <code>{list.latestSnapshot}</code>
            </>
          )}
        </p>
      </header>

      {loading && <p className="muted">加载版本清单…</p>}

      {list && (
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "minmax(0, 1fr) 320px",
            gap: 24,
            alignItems: "start",
          }}
        >
          {/* ── LEFT: pickers + log ─────────────────────────────────── */}
          <div>
            {/* 实例信息 + 搜索 */}
            <div className="card-base" style={{ marginBottom: 16 }}>
              <div className="eyebrow" style={{ marginBottom: 12 }}>
                实例
              </div>
              <div
                style={{
                  display: "grid",
                  gridTemplateColumns: "1fr 1fr",
                  gap: 12,
                  marginBottom: 12,
                }}
              >
                <div>
                  <div
                    className="t-caption"
                    style={{
                      color: "var(--steel)",
                      marginBottom: 4,
                    }}
                  >
                    实例名
                  </div>
                  <input
                    className="text-input"
                    style={{ width: "100%" }}
                    value={instanceName}
                    onChange={(e) => setInstanceName(e.target.value)}
                  />
                </div>
                <div>
                  <div
                    className="t-caption"
                    style={{
                      color: "var(--steel)",
                      marginBottom: 4,
                    }}
                  >
                    离线用户名
                  </div>
                  <input
                    className="text-input"
                    style={{ width: "100%" }}
                    value={username}
                    onChange={(e) => setUsername(e.target.value)}
                  />
                </div>
              </div>
              <input
                className="search-pill"
                style={{ width: "100%" }}
                placeholder="搜索版本（例：1.21.1）"
                value={filter}
                onChange={(e) => setFilter(e.target.value)}
              />
            </div>

            {/* 版本列表 */}
            <div
              className="card-base"
              style={{
                padding: 0,
                maxHeight: 320,
                overflow: "auto",
                marginBottom: 16,
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

            {/* Loader 选 */}
            <div className="card-base" style={{ marginBottom: 16 }}>
              <div className="eyebrow" style={{ marginBottom: 10 }}>
                Mod 加载器
              </div>
              <div className="row-wrap" style={{ gap: 8 }}>
                {loaderTabs.map((opt) => (
                  <button
                    key={opt.v}
                    className={`pill-tab ${loaderChoice === opt.v ? "active" : ""}`}
                    onClick={() => setLoaderChoice(opt.v)}
                  >
                    {opt.label}
                  </button>
                ))}
              </div>
              {loaderChoice !== "vanilla" && (
                <div style={{ marginTop: 12 }}>
                  {loaderLoading && (
                    <span className="t-caption muted">加载 loader 版本…</span>
                  )}
                  {loaderError && (
                    <span
                      className="t-caption"
                      style={{ color: "var(--error)" }}
                    >
                      错误：{loaderError}
                    </span>
                  )}
                  {loaderVersions && (
                    <select
                      className="text-input"
                      style={{ minWidth: 280 }}
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

            {/* 日志 */}
            {logs.length > 0 && (
              <div>
                <div
                  className="row"
                  style={{
                    justifyContent: "space-between",
                    marginBottom: 8,
                  }}
                >
                  <span className="eyebrow">日志</span>
                  <button
                    className="btn-link"
                    onClick={() => setLogs([])}
                    style={{ fontSize: 12 }}
                  >
                    清空
                  </button>
                </div>
                <div ref={logsRef} className="log-console">
                  {logs.map((l, i) => (
                    <div key={i} className={`log-line ${l.level}`}>
                      <span className="log-source">[{l.source}]</span>{" "}
                      {l.message}
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>

          {/* ── RIGHT: action panel ──────────────────────────────────── */}
          <aside style={{ position: "sticky", top: 32 }}>
            <div className="card-base">
              <div className="eyebrow" style={{ marginBottom: 8 }}>
                即将安装
              </div>
              <div
                className="t-h4"
                style={{ margin: 0, marginBottom: 6, wordBreak: "break-all" }}
              >
                {selected ?? "—"}
              </div>
              <div
                className="t-body-sm muted"
                style={{
                  marginBottom: 16,
                  wordBreak: "break-all",
                }}
              >
                {loaderChoice === "vanilla"
                  ? "原版 (无 mod 加载器)"
                  : `${loaderChoice} ${loaderVersionPick ?? "(选择版本)"}`}
                <br />
                实例：<code>{instanceName}</code>
                <br />
                玩家：<code>{username}</code>
              </div>

              <div className="stack-sm">
                <button
                  onClick={handleInstall}
                  disabled={!selected || installing}
                  className="btn btn-primary"
                  style={{ width: "100%" }}
                >
                  {installing
                    ? "安装中…"
                    : loaderChoice === "vanilla"
                    ? "安装原版"
                    : "安装 + Loader"}
                </button>
                <button
                  onClick={handleLaunch}
                  disabled={!installComplete || processPid !== null}
                  className="btn btn-secondary"
                  style={{ width: "100%" }}
                >
                  {processPid !== null
                    ? `已启动 (PID ${processPid})`
                    : "启动游戏"}
                </button>
              </div>

              {progress && (
                <div style={{ marginTop: 16 }}>
                  <div
                    className="t-caption muted"
                    style={{
                      marginBottom: 6,
                      whiteSpace: "nowrap",
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                    }}
                  >
                    {progress.label || "…"} · {pct}%
                  </div>
                  <div className="progress-track">
                    <div
                      className="progress-fill"
                      style={{ width: `${pct}%` }}
                    />
                  </div>
                </div>
              )}

              {installComplete && !installing && !progress && (
                <div
                  style={{
                    marginTop: 12,
                    padding: 10,
                    background: "var(--tint-mint)",
                    color: "var(--brand-green)",
                    borderRadius: "var(--r-md)",
                    fontSize: 12,
                    fontWeight: 600,
                  }}
                >
                  ✓ 安装完成
                </div>
              )}
            </div>
          </aside>
        </div>
      )}
    </div>
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
        background: selected ? "rgba(86, 69, 212, 0.10)" : "transparent",
        borderLeft: selected
          ? "3px solid var(--primary)"
          : "3px solid transparent",
        display: "flex",
        justifyContent: "space-between",
        alignItems: "center",
        borderBottom: "1px solid var(--hairline-soft)",
      }}
    >
      <span style={{ display: "flex", alignItems: "center", gap: 10 }}>
        <span
          className="t-body-sm-medium"
          style={{ color: selected ? "var(--primary)" : "var(--ink)" }}
        >
          {v.id}
        </span>
        <span className="badge badge-tag-gray">{v.kind}</span>
      </span>
      <span className="t-caption muted">{v.releaseTime.slice(0, 10)}</span>
    </div>
  );
}
