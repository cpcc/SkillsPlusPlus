import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { InstallLogPanel } from "../InstallLogPanel";
import type { InstallTaskResult } from "@skills-pp/shared";

function makeTask(overrides: Partial<InstallTaskResult> = {}): InstallTaskResult {
  return {
    id: "task-1",
    skillName: "test-skill",
    toolName: "Cursor",
    directoryId: "dir-1",
    action: "install",
    status: "success",
    logLines: ["Cloning...", "Done."],
    ...overrides,
  };
}

describe("InstallLogPanel", () => {
  it("shows success message", () => {
    render(<InstallLogPanel task={makeTask()} />);
    expect(screen.getByText(/安装成功/)).toBeTruthy();
  });

  it("shows failure message with error", () => {
    render(
      <InstallLogPanel
        task={makeTask({ status: "failed", errorMessage: "Network timeout" })}
      />,
    );
    expect(screen.getByText(/安装失败/)).toBeTruthy();
    expect(screen.getByText("Network timeout")).toBeTruthy();
  });

  it("toggles log visibility on click", async () => {
    const user = userEvent.setup();
    render(<InstallLogPanel task={makeTask()} />);

    // Initially collapsed
    expect(screen.queryByText("Cloning...")).toBeNull();

    // Click to expand
    await user.click(screen.getByText(/详细日志/));
    expect(screen.getByText(/Cloning/)).toBeTruthy();

    // Click to collapse
    await user.click(screen.getByText(/收起/));
    expect(screen.queryByText("Cloning...")).toBeNull();
  });

  it("hides toggle when no log lines", () => {
    render(<InstallLogPanel task={makeTask({ logLines: [] })} />);
    expect(screen.queryByText(/详细日志/)).toBeNull();
  });
});
