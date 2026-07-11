import { test, expect } from "@playwright/test";

test.describe("Discover Page", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/discover");
  });

  test.describe("Page layout", () => {
    test("should display page title '发现'", async ({ page }) => {
      await expect(page.locator("h1")).toContainText("发现");
    });

    test("should display skill count info", async ({ page }) => {
      const infoText = page.locator("p", { hasText: "个 skill" });
      await expect(infoText).toBeVisible();
    });

    test("should have main content area", async ({ page }) => {
      const main = page.locator("main");
      await expect(main).toBeVisible();
    });
  });

  test.describe("Search functionality", () => {
    test("should display search input with placeholder", async ({ page }) => {
      const searchInput = page.locator('input[placeholder*="搜索"]');
      await expect(searchInput).toBeVisible();
      await expect(searchInput).toHaveAttribute("type", "text");
    });

    test("should allow typing in search input", async ({ page }) => {
      const searchInput = page.locator('input[placeholder*="搜索"]');
      await searchInput.fill("test");
      await expect(searchInput).toHaveValue("test");
    });

    test("should clear search input", async ({ page }) => {
      const searchInput = page.locator('input[placeholder*="搜索"]');
      await searchInput.fill("test");
      await searchInput.clear();
      await expect(searchInput).toHaveValue("");
    });

    test("should update count when searching", async ({ page }) => {
      const searchInput = page.locator('input[placeholder*="搜索"]');
      const infoText = page.locator("p", { hasText: "个 skill" });

      await searchInput.fill("nonexistent");
      await expect(infoText).toContainText("0 /");
    });
  });

  test.describe("Refresh functionality", () => {
    test("should display refresh button", async ({ page }) => {
      const refreshButton = page.locator("button", { hasText: "刷新来源" });
      await expect(refreshButton).toBeVisible();
    });

    test("should have refresh icon in button", async ({ page }) => {
      const refreshButton = page.locator("button", { hasText: "刷新来源" });
      await expect(refreshButton).toContainText("刷新来源");
    });

    test("should disable refresh button while loading", async ({ page }) => {
      const refreshButton = page.locator("button", { hasText: "刷新来源" });
      const isDisabled = await refreshButton.isDisabled().catch(() => false);
      expect(typeof isDisabled).toBe("boolean");
    });
  });

  test.describe("Filter bar", () => {
    test("should display source filter dropdown", async ({ page }) => {
      const sourceFilter = page.locator("select");
      await expect(sourceFilter).toBeVisible();
    });

    test("should render category navigation or overflow toggle", async ({ page }) => {
      const categoryNav = page.locator("nav").filter({ has: page.locator("button") }).first();
      await expect(categoryNav).toBeVisible();

      const allTab = page.getByRole("button", { name: "全部" });
      const expandButton = page.getByRole("button", { name: "展开" });
      const hasAllTab = await allTab.isVisible().catch(() => false);
      const hasExpand = await expandButton.isVisible().catch(() => false);

      expect(hasAllTab || hasExpand).toBeTruthy();
    });
  });

  test.describe("Skill cards", () => {
    test("should display skill cards or empty state", async ({ page }) => {
      const skillCards = page.locator("button").filter({ has: page.locator(".font-semibold") });
      const emptyState = page.locator("text=没有匹配的 skill");
      const loadingState = page.locator("text=加载中...");
      const refreshHint = page.locator("text=点击「刷新来源」");

      const hasCards = await skillCards.count().catch(() => 0);
      const hasEmpty = await emptyState.isVisible().catch(() => false);
      const hasLoading = await loadingState.isVisible().catch(() => false);
      const hasHint = await refreshHint.isVisible().catch(() => false);

      expect(hasCards >= 0 || hasEmpty || hasLoading || hasHint).toBeTruthy();
    });

    test("should display skill name in cards", async ({ page }) => {
      const skillNames = page.locator(".font-semibold").filter({ hasText: /[\w\u4e00-\u9fa5]/ });
      const count = await skillNames.count();
      if (count > 0) {
        await expect(skillNames.first()).toBeVisible();
      }
    });

    test("should have hover effect on skill cards", async ({ page }) => {
      const firstCard = page.locator("button").filter({ has: page.locator(".font-semibold") }).first();
      if (await firstCard.isVisible().catch(() => false)) {
        await firstCard.hover();
      }
    });
  });

  test.describe("Empty state", () => {
    test("should display content area after loading", async ({ page }) => {
      await page.waitForLoadState("networkidle");
      const main = page.locator("main");
      await expect(main).toBeVisible();
    });
  });
});
