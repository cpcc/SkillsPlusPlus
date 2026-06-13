import { invoke } from "@tauri-apps/api/core";
import type { AppInfo } from "@skills-pp/shared";

export const ipc = {
  getAppInfo: (): Promise<AppInfo> => invoke("get_app_info"),
};
