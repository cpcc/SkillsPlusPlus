import { invoke, isTauri } from "@tauri-apps/api/core";
import type {
  AppInfo, AiToolDirectory, SkillItem, SkillSource,
  InstallPreview, InstalledSkill,
  InstallStrategy, LockEntry, CanonicalSkill, UpdateInfo,
} from "@skills-pp/shared";

/**
 * Base URL of the embedded HTTP bridge (see src-tauri/src/services/http_bridge.rs).
 * Only used when running outside of Tauri (i.e. a plain browser tab pointing at
 * the Vite dev server). Override with `VITE_DEV_HTTP` if needed.
 */
const HTTP_BASE = import.meta.env.VITE_DEV_HTTP ?? "http://127.0.0.1:3030";

/**
 * Invoke a backend command. Uses the native Tauri IPC when running inside the
 * Tauri WebView, and falls back to the HTTP bridge (`POST /invoke/:cmd`) when
 * running in a normal browser. Both paths hit the exact same Rust code, so
 * there is no mocking — browser tabs see real data.
 *
 * Tauri's `invoke` expects camelCase arg keys; we forward the same object to
 * the HTTP bridge, which converts keys to snake_case before deserializing.
 */
async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const hasArgs = args && Object.keys(args).length > 0;
  if (isTauri()) {
    return hasArgs ? invoke<T>(cmd, args) : invoke<T>(cmd);
  }
  const r = await fetch(`${HTTP_BASE}/invoke/${cmd}`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: hasArgs ? JSON.stringify(args) : "{}",
  });
  if (!r.ok) {
    let detail = "";
    try { detail = await r.text(); } catch { /* ignore */ }
    throw new Error(`${cmd} failed (${r.status}): ${detail}`);
  }
  return r.json() as Promise<T>;
}

export const ipc = {
  // App
  getAppInfo: (): Promise<AppInfo> => call("get_app_info"),
  checkAppUpdate: (): Promise<UpdateInfo> => call("check_app_update"),
  /**
   * 在系统浏览器打开 release 页面。
   * Tauri 模式下走 `open_release_url` 命令；浏览器调试模式下直接 window.open。
   */
  openReleaseUrl: async (url: string): Promise<void> => {
    if (isTauri()) {
      await call("open_release_url", { url });
    } else {
      window.open(url, "_blank", "noopener,noreferrer");
    }
  },

  // Directories
  scanDirectories: (): Promise<AiToolDirectory[]> => call("scan_directories"),
  listDirectories: (): Promise<AiToolDirectory[]> => call("list_directories"),
  addDirectory: (toolName: string, path: string): Promise<AiToolDirectory> =>
    call("add_directory", { toolName, path }),
  toggleDirectory: (id: string, enabled: boolean): Promise<void> =>
    call("toggle_directory", { id, enabled }),
  setDefaultDirectory: (id: string): Promise<void> =>
    call("set_default_directory", { id }),
  deleteDirectory: (id: string): Promise<void> =>
    call("delete_directory", { id }),

  // Sources
  listSources: (): Promise<SkillSource[]> => call("list_sources"),
  toggleSource: (id: string, enabled: boolean): Promise<void> =>
    call("toggle_source", { id, enabled }),

  // Skills (discovery)
  listSkills: (): Promise<SkillItem[]> => call("list_skills"),
  refreshSource: (sourceId: string): Promise<SkillItem[]> =>
    call("refresh_source", { sourceId }),
  refreshAllSources: (): Promise<SkillItem[]> => call("refresh_all_sources"),
  getSkill: (id: string): Promise<SkillItem | null> => call("get_skill", { id }),
  fetchSkillMd: (id: string): Promise<string | null> => call("fetch_skill_md", { id }),

  // Install
  previewInstall: (
    skillName: string,
    repoUrl: string,
    directoryId: string,
    strategy: InstallStrategy = "git",
  ): Promise<InstallPreview> =>
    call("preview_install", { skillName, repoUrl, directoryId, strategy }),
  installSkill: (params: {
    skillId?: string; skillName: string; repoUrl: string;
    directoryId: string; overwrite: boolean;
    strategy?: InstallStrategy;
    archiveUrl?: string;
  }): Promise<void> => call("install_skill", params),
  reinstallSkill: (params: {
    skillId?: string; skillName: string; repoUrl: string; directoryId: string;
    strategy?: InstallStrategy;
    archiveUrl?: string;
  }): Promise<void> => call("reinstall_skill", params),
  uninstallSkill: (skillName: string, directoryId: string): Promise<void> =>
    call("uninstall_skill", { skillName, directoryId }),
  listInstalledSkills: (): Promise<InstalledSkill[]> => call("list_installed_skills"),
  checkGitAvailable: (): Promise<boolean> => call("check_git_available"),
  refreshInstalledSkills: (): Promise<InstalledSkill[]> => call("refresh_installed_skills"),
  checkSkillUpdate: (skillId: string): Promise<InstalledSkill> => call("check_skill_update", { skillId }),
  importExistingSkills: (): Promise<number> => call("import_existing_skills"),
  openSkillDir: (path: string): Promise<void> => call("open_skill_dir", { path }),

  // Canonical store / lockfile（与 npx skills 互通）
  readLockfile: (): Promise<Record<string, LockEntry>> => call("read_lockfile"),
  listCanonicalSkills: (): Promise<CanonicalSkill[]> => call("list_canonical_skills"),
};
