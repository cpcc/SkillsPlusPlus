import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi, beforeEach } from "vitest";
import type { RefreshSourcesResult, SkillItem, SkillSource } from "@skills-pp/shared";
import { ToastProvider } from "../../components/ui/toast";
import DiscoverPage from "./index";

const listSkills = vi.fn<() => Promise<SkillItem[]>>();
const listSources = vi.fn<() => Promise<SkillSource[]>>();
const refreshAllSources = vi.fn<() => Promise<RefreshSourcesResult>>();
const searchOnline = vi.fn<() => Promise<SkillItem[]>>();

vi.mock("../../lib/ipc", () => ({
  ipc: {
    listSkills: () => listSkills(),
    listSources: () => listSources(),
    refreshAllSources: () => refreshAllSources(),
    searchOnline: () => searchOnline(),
  },
}));

function renderDiscover() {
  const qc = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter initialEntries={["/discover"]}>
        <ToastProvider>
          <DiscoverPage />
        </ToastProvider>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("DiscoverPage", () => {
  beforeEach(() => {
    listSkills.mockReset();
    listSources.mockReset();
    refreshAllSources.mockReset();
    searchOnline.mockReset();

    listSources.mockResolvedValue([
      { id: "registry", name: "官方聚合", baseUrl: "https://huggingface.co", enabled: true },
      { id: "skills_sh", name: "skills.sh", baseUrl: "https://skills.sh", enabled: true },
    ]);
    searchOnline.mockResolvedValue([]);
  });

  it("shows a toast when refresh falls back to local registry cache", async () => {
    listSkills.mockResolvedValue([
      {
        id: "registry_demo",
        name: "Registry Demo",
        description: "from registry",
        tags: ["demo"],
        sourceId: "registry",
        detailUrl: "https://example.com/demo",
        category: "开发编程",
      },
    ]);
    refreshAllSources.mockResolvedValue({
      skills: [
        {
          id: "registry_demo",
          name: "Registry Demo",
          description: "from registry",
          tags: ["demo"],
          sourceId: "registry",
          detailUrl: "https://example.com/demo",
          category: "开发编程",
        },
      ],
      warnings: [
        {
          sourceId: "registry",
          message: "官方聚合远端拉取失败，已回退到本地缓存。",
        },
      ],
    });

    renderDiscover();

    const refreshButton = await screen.findByRole("button", { name: "刷新来源" });
    await userEvent.click(refreshButton);

    expect(await screen.findByText("官方聚合已回退到本地缓存")).toBeInTheDocument();
    expect(await screen.findByText("官方聚合远端拉取失败，已回退到本地缓存。"))
      .toBeInTheDocument();
  });

  it("filters cards by category tabs while keeping registry as default source", async () => {
    listSkills.mockResolvedValue([
      {
        id: "registry_dev",
        name: "Dev Tool",
        description: "dev",
        tags: ["rust"],
        sourceId: "registry",
        detailUrl: "https://example.com/dev",
        category: "开发编程",
      },
      {
        id: "registry_life",
        name: "Life Tool",
        description: "life",
        tags: ["travel"],
        sourceId: "registry",
        detailUrl: "https://example.com/life",
        category: "生活服务",
      },
      {
        id: "skills_sh_other",
        name: "Other Source",
        description: "other",
        tags: ["demo"],
        sourceId: "skills_sh",
        detailUrl: "https://example.com/other",
        category: "开发编程",
      },
    ]);
    refreshAllSources.mockResolvedValue({ skills: [], warnings: [] });

    renderDiscover();

    await screen.findByText("Dev Tool");
    expect(screen.getByDisplayValue("官方聚合")).toBeInTheDocument();
    expect(screen.queryByText("Other Source")).not.toBeInTheDocument();

    const expandButton = screen.queryByRole("button", { name: "展开" });
    if (expandButton) {
      await userEvent.click(expandButton);
    }

    await userEvent.click(screen.getByRole("button", { name: "生活服务" }));

    await waitFor(() => {
      expect(screen.getByText("Life Tool")).toBeInTheDocument();
      expect(screen.queryByText("Dev Tool")).not.toBeInTheDocument();
    });
  });
});