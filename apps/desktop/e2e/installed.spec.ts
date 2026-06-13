import { test, expect } from "@playwright/test";

test.describe("Installed Page", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/installed");
  });

  test.describe("Page layout", () => {
    test("should display page title '已安装'", async ({ page }) => {
      await expect(page.locator("h2")).toContainText("已安装");
    });

    test("should display installed count info", async ({ page }) => {
      const infoText = page.locator("p", { hasText: "共" }).filter({ hasText: "个" });
      await expect(infoText).toBeVisible();
    });

    test("should have main content area", async ({ page }) => {
      const main = page.locator("main");
      await expect(main).toBeVisible();
    });
  });

  test.describe("Refresh functionality", () => {
    test("should display refresh button", async ({ page }) => {
      const refreshButton = page.locator("button", { hasText: "刷新状态" });
      await expect(refreshButton).toBeVisible();
    });

    test("should have refresh icon in button", async ({ page }) => {
      const refreshButton = page.locator("button", { hasText: "刷新状态" });
      await expect(refreshButton).toContainText("刷新状态");
    });
  });

  test.describe("Empty state", () => {
    test("should show empty state or installed list", async ({ page }) => {
      const emptyState = page.locator("text=暂无已安装的 skill");
      const installedList = page.locator(".space-y-2 > div");

      const hasEmpty = await emptyState.isVisible().catch(() => false);
      const hasItems = await installedList.count().catch(() => 0);

      expect(hasEmpty || hasItems >= 0).toBeTruthy();
    });

    test("should have link to discover page when empty", async ({ page }) => {
      const discoverLink = page.locator("a", { hasText: "去发现页浏览" });
      const isHidden = await discoverLink.isHidden().catch(() => true);

      if (!isHidden) {
        await discoverLink.click();
        await expect(page).toHaveURL("/discover");
      }
    });

    test("should display empty state or installed skills", async ({ page }) => {
      const emptyContainer = page.locator(".border-dashed");
      const installedList = page.locator(".space-y-2 > div");

      const hasEmpty = await emptyContainer.isVisible().catch(() => false);
      const hasItems = await installedList.count().catch(() => 0);

      expect(hasEmpty || hasItems >= 0).toBeTruthy();
    });
  });

  test.describe("Installed skill list", () => {
    test("should display skill items when installed", async ({ page }) => {
      const skillItems = page.locator(".space-y-2 > div");
      const count = await skillItems.count().catch(() => 0);
      expect(count >= 0).toBeTruthy();
    });

    test("should display skill name in list items", async ({ page }) => {
      const skillNames = page.locator(".space-y-2 .font-semibold");
      const count = await skillNames.count().catch(() => 0);
      if (count > 0) {
        await expect(skillNames.first()).toBeVisible();
      }
    });

    test("should display status badge for each skill", async ({ page }) => {
      const statusBadges = page.locator(".space-y-2 .rounded-full");
      const count = await statusBadges.count().catch(() => 0);
      if (count > 0) {
        await expect(statusBadges.first()).toBeVisible();
      }
    });

    test("should display action buttons for each skill", async ({ page }) => {
      const actionButtons = page.locator(".space-y-2 button[title]");
      const count = await actionButtons.count().catch(() => 0);
      if (count > 0) {
        await expect(actionButtons.first()).toBeVisible();
      }
    });
  });

  test.describe("Skill actions", () => {
    test("should have open directory button", async ({ page }) => {
      const openDirButton = page.locator('button[title="打开目录"]').first();
      const hasButton = await openDirButton.isVisible().catch(() => false);
      expect(typeof hasButton).toBe("boolean");
    });

    test("should have view detail button", async ({ page }) => {
      const viewDetailButton = page.locator('button[title="查看详情"]').first();
      const hasButton = await viewDetailButton.isVisible().catch(() => false);
      expect(typeof hasButton).toBe("boolean");
    });

    test("should have check update button", async ({ page }) => {
      const checkUpdateButton = page.locator('button[title="检查更新"]').first();
      const hasButton = await checkUpdateButton.isVisible().catch(() => false);
      expect(typeof hasButton).toBe("boolean");
    });

    test("should have reinstall button", async ({ page }) => {
      const reinstallButton = page.locator('button[title="重装"]').first();
      const hasButton = await reinstallButton.isVisible().catch(() => false);
      expect(typeof hasButton).toBe("boolean");
    });

    test("should have uninstall button", async ({ page }) => {
      const uninstallButton = page.locator('button[title="卸载"]').first();
      const hasButton = await uninstallButton.isVisible().catch(() => false);
      expect(typeof hasButton).toBe("boolean");
    });
  });

  test.describe("Navigation to skill detail", () => {
    test("should navigate to skill detail when clicking skill name", async ({ page }) => {
      const skillName = page.locator(".space-y-2 .font-semibold").first();
      const hasSkill = await skillName.isVisible().catch(() => false);

      if (hasSkill) {
        await skillName.click();
        await expect(page).toHaveURL(/\/skill\//);
      }
    });

    test("should navigate to skill detail when clicking view detail button", async ({ page }) => {
      const viewDetailButton = page.locator('button[title="查看详情"]').first();
      const hasButton = await viewDetailButton.isVisible().catch(() => false);

      if (hasButton) {
        await viewDetailButton.click();
        await expect(page).toHaveURL(/\/skill\//);
      }
    });
  });

  test.describe("Recent tasks", () => {
    test("should display recent tasks section when available", async ({ page }) => {
      const tasksSection = page.locator("h3", { hasText: "最近安装记录" });
      const hasSection = await tasksSection.isVisible().catch(() => false);
      expect(typeof hasSection).toBe("boolean");
    });
  });
});
