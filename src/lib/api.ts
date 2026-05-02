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
