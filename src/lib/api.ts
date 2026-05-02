import { invoke } from "@tauri-apps/api/core";

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
