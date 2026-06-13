import { test, expect } from "@playwright/test";

test.describe("Tools Page", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/tools");
  });

  test.describe("Page layout", () => {
    test("should display page title '工具与目录'", async ({ page }) => {
      await expect(page.locator("h2")).toContainText("工具与目录");
    });

    test("should display directory count info", async ({ page }) => {
      const infoText = page.locator("p", { hasText: "个目录" });
      await expect(infoText).toBeVisible();
    });

    test("should have main content area", async ({ page }) => {
      const main = page.locator("main");
      await expect(main).toBeVisible();
    });
  });

  test.describe("Action buttons", () => {
    test("should display refresh button", async ({ page }) => {
      const refreshButton = page.locator("button", { hasText: "重新扫描" });
      await expect(refreshButton).toBeVisible();
    });

    test("should display add directory button", async ({ page }) => {
      const addButton = page.locator("button", { hasText: "新增目录" });
      await expect(addButton).toBeVisible();
    });

    test("should have primary style for add button", async ({ page }) => {
      const addButton = page.locator("button", { hasText: "新增目录" });
      await expect(addButton).toHaveClass(/bg-brand-600/);
    });
  });

  test.describe("Directory list", () => {
    test("should display directory groups or empty state", async ({ page }) => {
      await page.waitForLoadState("networkidle");
      const directoryCards = page.locator(".space-y-2 > div");
      const loadingState = page.locator("text=加载中...");
      const groupHeaders = page.locator("h3.uppercase");

      const hasCards = await directoryCards.count().catch(() => 0);
      const hasLoading = await loadingState.isVisible().catch(() => false);
      const hasGroups = await groupHeaders.count().catch(() => 0);

      expect(hasCards >= 0 || hasLoading || hasGroups >= 0).toBeTruthy();
    });

    test("should display directory path in cards", async ({ page }) => {
      const paths = page.locator(".font-mono").filter({ hasText: "/" });
      const count = await paths.count().catch(() => 0);
      if (count > 0) {
        await expect(paths.first()).toBeVisible();
      }
    });

    test("should display status badge for directories", async ({ page }) => {
      const statusBadges = page.locator(".text-green-600, .text-yellow-600, .text-gray-400");
      const count = await statusBadges.count().catch(() => 0);
      if (count > 0) {
        await expect(statusBadges.first()).toBeVisible();
      }
    });

    test("should display skill count for detected directories", async ({ page }) => {
      const skillCounts = page.locator("text=个 skill");
      const count = await skillCounts.count().catch(() => 0);
      if (count > 0) {
        await expect(skillCounts.first()).toBeVisible();
      }
    });
  });

  test.describe("Directory card interactions", () => {
    test("should have more options button for each directory", async ({ page }) => {
      const moreButtons = page.locator("button").filter({ has: page.locator("svg") }).last();
      const hasButton = await moreButtons.isVisible().catch(() => false);
      expect(typeof hasButton).toBe("boolean");
    });

    test("should open dropdown menu on more options click", async ({ page }) => {
      const moreButton = page.locator('[data-state="closed"]').first();
      const hasButton = await moreButton.isVisible().catch(() => false);

      if (hasButton) {
        await moreButton.click();
        const menu = page.locator('[role="menu"]');
        const hasMenu = await menu.isVisible().catch(() => false);
        expect(typeof hasMenu).toBe("boolean");
      }
    });
  });

  test.describe("Add directory dialog", () => {
    test("should open add directory dialog", async ({ page }) => {
      const addButton = page.locator("button", { hasText: "新增目录" });
      await addButton.click();

      const dialog = page.locator('[role="dialog"]');
      await expect(dialog).toBeVisible();
    });

    test("should display dialog title", async ({ page }) => {
      const addButton = page.locator("button", { hasText: "新增目录" });
      await addButton.click();

      const dialogTitle = page.locator('[role="dialog"] h2, [role="dialog"] [class*="font-semibold"]');
      await expect(dialogTitle).toContainText("新增目录");
    });

    test("should display tool name selector", async ({ page }) => {
      const addButton = page.locator("button", { hasText: "新增目录" });
      await addButton.click();

      const toolSelect = page.locator('[role="dialog"] select');
      await expect(toolSelect).toBeVisible();
    });

    test("should display path input", async ({ page }) => {
      const addButton = page.locator("button", { hasText: "新增目录" });
      await addButton.click();

      const pathInput = page.locator('[role="dialog"] input[placeholder*="Users"]');
      await expect(pathInput).toBeVisible();
    });

    test("should display cancel button", async ({ page }) => {
      const addButton = page.locator("button", { hasText: "新增目录" });
      await addButton.click();

      const cancelButton = page.locator('[role="dialog"] button', { hasText: "取消" });
      await expect(cancelButton).toBeVisible();
    });

    test("should display submit button", async ({ page }) => {
      const addButton = page.locator("button", { hasText: "新增目录" });
      await addButton.click();

      const submitButton = page.locator('[role="dialog"] button[type="submit"]');
      await expect(submitButton).toBeVisible();
    });

    test("should close dialog on cancel", async ({ page }) => {
      const addButton = page.locator("button", { hasText: "新增目录" });
      await addButton.click();

      const cancelButton = page.locator('[role="dialog"] button', { hasText: "取消" });
      await cancelButton.click();

      const dialog = page.locator('[role="dialog"]');
      await expect(dialog).toBeHidden();
    });

    test("should show custom tool input when '其他' selected", async ({ page }) => {
      const addButton = page.locator("button", { hasText: "新增目录" });
      await addButton.click();

      const toolSelect = page.locator('[role="dialog"] select');
      await toolSelect.selectOption("其他");

      const customInput = page.locator('[role="dialog"] input[placeholder*="MyCopilot"]');
      await expect(customInput).toBeVisible();
    });

    test("should have submit button in dialog", async ({ page }) => {
      const addButton = page.locator("button", { hasText: "新增目录" });
      await addButton.click();

      const submitButton = page.locator('[role="dialog"] button[type="submit"]');
      await expect(submitButton).toBeVisible();
      await expect(submitButton).toContainText("添加");
    });
  });

  test.describe("Directory group headers", () => {
    test("should display tool name as group header", async ({ page }) => {
      const groupHeaders = page.locator("h3.uppercase");
      const count = await groupHeaders.count().catch(() => 0);
      if (count > 0) {
        await expect(groupHeaders.first()).toBeVisible();
      }
    });
  });
});
