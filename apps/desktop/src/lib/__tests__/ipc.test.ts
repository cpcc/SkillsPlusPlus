import { describe, it, expect, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue({
    version: "0.1.0",
    dbPath: "/tmp/test.db",
    logPath: "/tmp/test.log",
    platform: "macos",
  }),
}));

describe("ipc", () => {
  it("getAppInfo returns AppInfo shape", async () => {
    const { ipc } = await import("../ipc");
    const result = await ipc.getAppInfo();
    expect(result).toHaveProperty("version");
    expect(result).toHaveProperty("platform");
    expect(result).toHaveProperty("dbPath");
  });
});
