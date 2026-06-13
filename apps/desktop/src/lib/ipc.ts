import { invoke } from "@tauri-apps/api/core";
import type { AppInfo, AiToolDirectory, SkillItem, SkillSource } from "@skills-pp/shared";

export const ipc = {
  // App
  getAppInfo: (): Promise<AppInfo> => invoke("get_app_info"),

  // Directories
  scanDirectories: (): Promise<AiToolDirectory[]> => invoke("scan_directories"),
  listDirectories: (): Promise<AiToolDirectory[]> => invoke("list_directories"),
  addDirectory: (toolName: string, path: string): Promise<AiToolDirectory> =>
    invoke("add_directory", { toolName, path }),
  toggleDirectory: (id: string, enabled: boolean): Promise<void> =>
    invoke("toggle_directory", { id, enabled }),
  setDefaultDirectory: (id: string): Promise<void> =>
    invoke("set_default_directory", { id }),
  deleteDirectory: (id: string): Promise<void> =>
    invoke("delete_directory", { id }),

  // Sources
  listSources: (): Promise<SkillSource[]> => invoke("list_sources"),
  toggleSource: (id: string, enabled: boolean): Promise<void> =>
    invoke("toggle_source", { id, enabled }),

  // Skills
  listSkills: (): Promise<SkillItem[]> => invoke("list_skills"),
  refreshSource: (sourceId: string): Promise<SkillItem[]> =>
    invoke("refresh_source", { sourceId }),
  refreshAllSources: (): Promise<SkillItem[]> => invoke("refresh_all_sources"),
  getSkill: (id: string): Promise<SkillItem | null> =>
    invoke("get_skill", { id }),
};
