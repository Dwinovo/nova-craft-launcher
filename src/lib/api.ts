import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

// ────────── Path layout ──────────
export type PathMode = "portable" | "app_data";

export interface PathInfo {
  dataRoot: string;
  mode: PathMode;
  instances: string;
  sharedAssets: string;
  sharedLibraries: string;
  sharedVersions: string;
  configFile: string;
  logs: string;
  cache: string;
}

export async function getPaths(): Promise<PathInfo> {
  return invoke<PathInfo>("core_get_paths");
}

// ────────── App config ──────────
export type MirrorPolicy = "auto" | "official" | "bmclapi";

export interface AppConfig {
  mirror_policy: MirrorPolicy;
  max_concurrent_downloads: number;
  ui_locale: string;
}

export async function getConfig(): Promise<AppConfig> {
  return invoke<AppConfig>("core_get_config");
}

export async function setConfig(config: AppConfig): Promise<void> {
  return invoke("core_set_config", { config });
}

// ────────── Vanilla versions / install ──────────
export type VersionKind = "release" | "snapshot" | "old_beta" | "old_alpha";

export interface VersionEntry {
  id: string;
  kind: VersionKind;
  releaseTime: string;
  url: string;
}

export interface VersionListResponse {
  latestRelease: string;
  latestSnapshot: string;
  versions: VersionEntry[];
}

export async function listVersions(releaseOnly = true): Promise<VersionListResponse> {
  return invoke<VersionListResponse>("vanilla_list_versions", { releaseOnly });
}

/** 启动一次安装；返回 task_id，进度通过 progress 事件汇报。 */
export async function installVersion(
  instanceName: string,
  versionId: string
): Promise<string> {
  return invoke<string>("vanilla_install", { instanceName, versionId });
}

// ────────── Loader (Fabric / Forge / NeoForge) ──────────
export type LoaderKind = "fabric" | "forge" | "neo_forge";

export interface LoaderVersion {
  version: string;
  stable: boolean;
}

export async function listLoaderVersions(
  kind: LoaderKind,
  mcVersion: string
): Promise<LoaderVersion[]> {
  return invoke<LoaderVersion[]>("loader_list_versions", { kind, mcVersion });
}

/** 启动 loader 安装；返回 mergedVersionId（前端用此 id 后续传给 launchRun）。 */
export async function installLoader(
  kind: LoaderKind,
  mcVersion: string,
  loaderVersion: string,
  instanceName: string
): Promise<string> {
  return invoke<string>("loader_install", {
    kind,
    mcVersion,
    loaderVersion,
    instanceName,
  });
}

// ────────── Java ──────────
export interface JavaInfo {
  path: string;
  versionMajor: number;
  versionFull: string;
  vendor: string;
  arch: string;
  source: string;
}

export async function scanJava(): Promise<JavaInfo[]> {
  return invoke<JavaInfo[]>("java_scan");
}

// ────────── Launch ──────────
export interface LaunchRequest {
  instanceName: string;
  versionId: string;
  username: string;
  javaPath?: string;
  minMemMb?: number;
  maxMemMb?: number;
  jvmArgsExtra?: string[];
  gameArgsExtra?: string[];
}

export async function launchRun(req: LaunchRequest): Promise<number> {
  return invoke<number>("launch_run", { req });
}

// ────────── Progress events ──────────
export type ProgressEvent =
  | {
      kind: "task_started";
      task_id: string;
      label: string;
      total_weight: number;
    }
  | {
      kind: "task_progress";
      task_id: string;
      completed: number;
      total: number;
      message: string | null;
    }
  | {
      kind: "task_finished";
      task_id: string;
      success: boolean;
      error: string | null;
    }
  | {
      kind: "log";
      source: string;
      level: "trace" | "debug" | "info" | "warn" | "error";
      message: string;
    };

export function onProgress(handler: (ev: ProgressEvent) => void): Promise<UnlistenFn> {
  return listen<ProgressEvent>("ncl://progress", (ev) => handler(ev.payload));
}

export interface ProcessExitEvent {
  pid: number;
  exit_code: number | null;
}

export function onProcessExit(
  handler: (ev: ProcessExitEvent) => void
): Promise<UnlistenFn> {
  return listen<ProcessExitEvent>("ncl://process_exit", (ev) => handler(ev.payload));
}
