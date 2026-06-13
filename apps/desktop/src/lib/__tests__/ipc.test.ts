import { describe, it, expect, vi, beforeEach } from "vitest";

const mockInvoke = vi.fn();
const mockIsTauri = vi.fn(() => true);

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
  isTauri: () => mockIsTauri(),
}));

describe("ipc", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("getAppInfo calls get_app_info", async () => {
    mockInvoke.mockResolvedValue({ version: "0.1.0", dbPath: "/tmp", logPath: "/tmp", platform: "macos" });
    const { ipc } = await import("../ipc");
    const result = await ipc.getAppInfo();
    expect(mockInvoke).toHaveBeenCalledWith("get_app_info");
    expect(result).toHaveProperty("version", "0.1.0");
  });

  it("scanDirectories calls scan_directories", async () => {
    mockInvoke.mockResolvedValue([]);
    const { ipc } = await import("../ipc");
    await ipc.scanDirectories();
    expect(mockInvoke).toHaveBeenCalledWith("scan_directories");
  });

  it("listDirectories calls list_directories", async () => {
    mockInvoke.mockResolvedValue([]);
    const { ipc } = await import("../ipc");
    await ipc.listDirectories();
    expect(mockInvoke).toHaveBeenCalledWith("list_directories");
  });

  it("addDirectory passes toolName and path", async () => {
    mockInvoke.mockResolvedValue({ id: "dir-1" });
    const { ipc } = await import("../ipc");
    await ipc.addDirectory("Cursor", "/tmp/cursor");
    expect(mockInvoke).toHaveBeenCalledWith("add_directory", { toolName: "Cursor", path: "/tmp/cursor" });
  });

  it("toggleDirectory passes id and enabled", async () => {
    mockInvoke.mockResolvedValue(undefined);
    const { ipc } = await import("../ipc");
    await ipc.toggleDirectory("dir-1", false);
    expect(mockInvoke).toHaveBeenCalledWith("toggle_directory", { id: "dir-1", enabled: false });
  });

  it("listSources calls list_sources", async () => {
    mockInvoke.mockResolvedValue([]);
    const { ipc } = await import("../ipc");
    await ipc.listSources();
    expect(mockInvoke).toHaveBeenCalledWith("list_sources");
  });

  it("listSkills calls list_skills", async () => {
    mockInvoke.mockResolvedValue([]);
    const { ipc } = await import("../ipc");
    await ipc.listSkills();
    expect(mockInvoke).toHaveBeenCalledWith("list_skills");
  });

  it("getSkill passes id", async () => {
    mockInvoke.mockResolvedValue(null);
    const { ipc } = await import("../ipc");
    await ipc.getSkill("skill-1");
    expect(mockInvoke).toHaveBeenCalledWith("get_skill", { id: "skill-1" });
  });

  it("previewInstall passes all params", async () => {
    mockInvoke.mockResolvedValue({ skillName: "test", repoUrl: "url", targetPath: "/tmp/test", strategy: "git" });
    const { ipc } = await import("../ipc");
    await ipc.previewInstall("test", "url", "dir-1", "copy");
    expect(mockInvoke).toHaveBeenCalledWith("preview_install", {
      skillName: "test", repoUrl: "url", directoryId: "dir-1", strategy: "copy",
    });
  });

  it("installSkill passes params correctly", async () => {
    mockInvoke.mockResolvedValue({ id: "task-1", status: "success" });
    const { ipc } = await import("../ipc");
    await ipc.installSkill({
      skillName: "test", repoUrl: "url", directoryId: "dir-1", overwrite: false, strategy: "copy",
    });
    expect(mockInvoke).toHaveBeenCalledWith("install_skill", {
      skillName: "test", repoUrl: "url", directoryId: "dir-1", overwrite: false, strategy: "copy",
    });
  });

  it("uninstallSkill passes skillName and directoryId", async () => {
    mockInvoke.mockResolvedValue({ id: "task-2", status: "success" });
    const { ipc } = await import("../ipc");
    await ipc.uninstallSkill("test", "dir-1");
    expect(mockInvoke).toHaveBeenCalledWith("uninstall_skill", {
      skillName: "test", directoryId: "dir-1",
    });
  });

  it("refreshInstalledSkills calls refresh_installed_skills", async () => {
    mockInvoke.mockResolvedValue([]);
    const { ipc } = await import("../ipc");
    await ipc.refreshInstalledSkills();
    expect(mockInvoke).toHaveBeenCalledWith("refresh_installed_skills");
  });

  it("checkSkillUpdate passes skillId", async () => {
    mockInvoke.mockResolvedValue({ id: "s1", status: "ok" });
    const { ipc } = await import("../ipc");
    await ipc.checkSkillUpdate("s1");
    expect(mockInvoke).toHaveBeenCalledWith("check_skill_update", { skillId: "s1" });
  });

  it("checkGitAvailable calls check_git_available", async () => {
    mockInvoke.mockResolvedValue(true);
    const { ipc } = await import("../ipc");
    const result = await ipc.checkGitAvailable();
    expect(mockInvoke).toHaveBeenCalledWith("check_git_available");
    expect(result).toBe(true);
  });
});
