import { invoke } from "@tauri-apps/api/core";
import type { AppInfo, AiToolDirectory } from "@skills-pp/shared";

export const ipc = {
  getAppInfo: (): Promise<AppInfo> => invoke("get_app_info"),

  scanDirectories: (): Promise<AiToolDirectory[]> =>
    invoke("scan_directories"),

  listDirectories: (): Promise<AiToolDirectory[]> =>
    invoke("list_directories"),

  addDirectory: (toolName: string, path: string): Promise<AiToolDirectory> =>
    invoke("add_directory", { toolName, path }),

  toggleDirectory: (id: string, enabled: boolean): Promise<void> =>
    invoke("toggle_directory", { id, enabled }),

  setDefaultDirectory: (id: string): Promise<void> =>
    invoke("set_default_directory", { id }),

  deleteDirectory: (id: string): Promise<void> =>
    invoke("delete_directory", { id }),
};
