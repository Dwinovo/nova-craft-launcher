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

export function InstancesPage() {
  const [list, setList] = useState<VersionListResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [filter, setFilter] = useState("");
  const [selected, setSelected] = useState<string | null>(null);
  const [instanceName, setInstanceName] = useState("test-instance");
  const [username, setUsername] = useState("Player1");

  // Loader state
  const [loaderChoice, setLoaderChoice] = useState<LoaderChoice>("vanilla");
  const [loaderVersions, setLoaderVersions] = useState<LoaderVersion[] | null>(null);
  const [loaderVersionPick, setLoaderVersionPick] = useState<string | null>(null);
  const [loaderLoading, setLoaderLoading] = useState(false);
  const [loaderError, setLoaderError] = useState<string | null>(null);

  // Install / launch state
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

  // 当 MC 版本或 loader 类型变化时,刷新 loader 版本列表
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
        if (ev.success) {
          setProgress(null);
        } else {
          setProgress(null);
          setLogs((l) => [
            ...l,
            { source: "install", level: "error", message: ev.error ?? "失败" },
          ]);
        }
      } else if (ev.kind === "log") {
        setLogs((l) => [...l, { source: ev.source, level: ev.level, message: ev.message }]);
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
          message: `已开始安装：${installedId}（后续进度见进度条）`,
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
    <div>
      <h1>实例</h1>
      {loading && <p>加载版本清单…</p>}
      {list && (
        <>
          <div style={{ display: "flex", gap: 8, marginBottom: 12, flexWrap: "wrap" }}>
            <input
              placeholder="搜索版本 (例: 1.21.1)"
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              style={inputStyle}
            />
            <input
              placeholder="实例名"
              value={instanceName}
              onChange={(e) => setInstanceName(e.target.value)}
              style={inputStyle}
            />
            <input
              placeholder="离线用户名"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              style={inputStyle}
            />
          </div>
          <p style={{ color: "#6e6e76", fontSize: 12, marginBottom: 8 }}>
            最新 release: <code>{list.latestRelease}</code> · 最新 snapshot:{" "}
            <code>{list.latestSnapshot}</code> · 显示 {filtered.length}/{list.versions.length}
          </p>
          <div style={listStyle}>
            {filtered.map((v) => (
              <VersionRow
                key={v.id}
                v={v}
                selected={selected === v.id}
                onSelect={() => setSelected(v.id)}
              />
            ))}
          </div>

          {/* Loader 选择 */}
          <div style={{ marginTop: 16, padding: 12, background: "#f0f0f4", borderRadius: 6 }}>
            <div style={{ fontSize: 13, marginBottom: 8, fontWeight: 600 }}>
              Mod 加载器
            </div>
            <div style={{ display: "flex", gap: 6, marginBottom: 8 }}>
              {(
                [
                  { v: "vanilla", label: "原版", enabled: true },
                  { v: "fabric", label: "Fabric", enabled: true },
                  { v: "forge", label: "Forge (Sprint 3b)", enabled: false },
                  { v: "neo_forge", label: "NeoForge (Sprint 4)", enabled: false },
                ] as const
              ).map((opt) => (
                <button
                  key={opt.v}
                  onClick={() => opt.enabled && setLoaderChoice(opt.v)}
                  style={loaderTabStyle(loaderChoice === opt.v, !opt.enabled)}
                  disabled={!opt.enabled}
                >
                  {opt.label}
                </button>
              ))}
            </div>
            {loaderChoice !== "vanilla" && (
              <div>
                {loaderLoading && <span style={{ fontSize: 12 }}>加载 loader 版本…</span>}
                {loaderError && (
                  <span style={{ fontSize: 12, color: "#d44" }}>错误：{loaderError}</span>
                )}
                {loaderVersions && (
                  <select
                    value={loaderVersionPick ?? ""}
                    onChange={(e) => setLoaderVersionPick(e.target.value)}
                    style={{ ...inputStyle, minWidth: 220 }}
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

          <div style={{ marginTop: 16, display: "flex", gap: 8, alignItems: "center" }}>
            <button
              onClick={handleInstall}
              disabled={!selected || installing}
              style={btnStyle(!selected || installing)}
            >
              {installing
                ? "安装中…"
                : loaderChoice === "vanilla"
                ? `安装原版 ${selected ?? ""}`
                : `安装 ${loaderChoice} ${loaderVersionPick ?? ""} (${selected ?? ""})`}
            </button>
            <button
              onClick={handleLaunch}
              disabled={!installComplete || processPid !== null}
              style={btnStyle(!installComplete || processPid !== null)}
            >
              {processPid !== null ? `已启动 (PID ${processPid})` : "启动游戏"}
            </button>
          </div>

          {progress && (
            <div style={{ marginTop: 16 }}>
              <div style={{ fontSize: 12, color: "#6e6e76", marginBottom: 4 }}>
                {progress.label} · {progress.completed}/{progress.total} ({pct}%)
              </div>
              <div style={progressBarOuter}>
                <div style={{ ...progressBarInner, width: `${pct}%` }} />
              </div>
            </div>
          )}

          {logs.length > 0 && (
            <div style={{ marginTop: 16 }}>
              <h2 style={{ fontSize: 14, margin: "0 0 6px" }}>日志</h2>
              <div ref={logsRef} style={logBoxStyle}>
                {logs.map((l, i) => (
                  <div key={i} style={logLineStyle(l.level)}>
                    <span style={{ color: "#6e6e76" }}>[{l.source}]</span> {l.message}
                  </div>
                ))}
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
}

function VersionRow(props: { v: VersionEntry; selected: boolean; onSelect: () => void }) {
  const { v, selected, onSelect } = props;
  return (
    <div
      onClick={onSelect}
      style={{
        padding: "6px 12px",
        cursor: "pointer",
        background: selected ? "#396cd8" : "transparent",
        color: selected ? "white" : "inherit",
        borderRadius: 4,
        display: "flex",
        justifyContent: "space-between",
      }}
    >
      <span>
        <code>{v.id}</code> <span style={{ opacity: 0.6, fontSize: 11 }}>({v.kind})</span>
      </span>
      <span style={{ opacity: 0.6, fontSize: 11 }}>{v.releaseTime.slice(0, 10)}</span>
    </div>
  );
}

const inputStyle: React.CSSProperties = {
  padding: "6px 10px",
  border: "1px solid #cfcfd4",
  borderRadius: 6,
  fontSize: 14,
};

const listStyle: React.CSSProperties = {
  maxHeight: 280,
  overflow: "auto",
  border: "1px solid #cfcfd4",
  borderRadius: 6,
  padding: 4,
};

function btnStyle(disabled: boolean): React.CSSProperties {
  return {
    padding: "8px 16px",
    background: disabled ? "#a8a8b0" : "#396cd8",
    color: "white",
    border: "none",
    borderRadius: 6,
    cursor: disabled ? "not-allowed" : "pointer",
    fontSize: 14,
  };
}

function loaderTabStyle(active: boolean, disabled: boolean): React.CSSProperties {
  return {
    padding: "5px 12px",
    background: active ? "#396cd8" : disabled ? "#e0e0e4" : "transparent",
    color: active ? "white" : disabled ? "#a8a8b0" : "#1a1a1f",
    border: active ? "1px solid #396cd8" : "1px solid #cfcfd4",
    borderRadius: 4,
    cursor: disabled ? "not-allowed" : "pointer",
    fontSize: 12,
  };
}

const progressBarOuter: React.CSSProperties = {
  height: 8,
  background: "#e0e0e4",
  borderRadius: 4,
  overflow: "hidden",
};
const progressBarInner: React.CSSProperties = {
  height: "100%",
  background: "#396cd8",
  transition: "width 0.2s",
};

const logBoxStyle: React.CSSProperties = {
  height: 220,
  overflow: "auto",
  background: "#1c1c20",
  color: "#e7e7ea",
  fontFamily: "Cascadia Mono, Consolas, monospace",
  fontSize: 12,
  padding: 8,
  borderRadius: 6,
};

function logLineStyle(level: string): React.CSSProperties {
  let color = "#e7e7ea";
  if (level === "error") color = "#ff6b6b";
  else if (level === "warn") color = "#ffce5b";
  return { color, lineHeight: 1.4 };
}
