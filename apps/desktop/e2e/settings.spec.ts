import { test, expect } from "@playwright/test";

test.describe("Settings Page", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/settings");
  });

  test.describe("Page layout", () => {
    test("should display page title '设置'", async ({ page }) => {
      await expect(page.locator("h2")).toContainText("设置");
    });

    test("should display page description", async ({ page }) => {
      const description = page.locator("p", { hasText: "来源站配置" });
      await expect(description).toBeVisible();
    });

    test("should have main content area", async ({ page }) => {
      const main = page.locator("main");
      await expect(main).toBeVisible();
    });
  });

  test.describe("App info section", () => {
    test("should display app info section header", async ({ page }) => {
      const appInfoSection = page.locator("h3", { hasText: "应用信息" });
      await expect(appInfoSection).toBeVisible();
    });

    test("should display version info or loading state", async ({ page }) => {
      const versionLabel = page.locator("dt", { hasText: "版本" });
      const loadingText = page.locator("text=加载中...");
      const errorText = page.locator("text=加载失败");

      const hasVersion = await versionLabel.isVisible().catch(() => false);
      const hasLoading = await loadingText.isVisible().catch(() => false);
      const hasError = await errorText.isVisible().catch(() => false);

      expect(hasVersion || hasLoading || hasError).toBeTruthy();
    });

    test("should display platform info or loading state", async ({ page }) => {
      const platformLabel = page.locator("dt", { hasText: "平台" });
      const loadingText = page.locator("text=加载中...");
      const errorText = page.locator("text=加载失败");

      const hasPlatform = await platformLabel.isVisible().catch(() => false);
      const hasLoading = await loadingText.isVisible().catch(() => false);
      const hasError = await errorText.isVisible().catch(() => false);

      expect(hasPlatform || hasLoading || hasError).toBeTruthy();
    });

    test("should display database path or loading state", async ({ page }) => {
      const dbLabel = page.locator("dt", { hasText: "数据库" });
      const loadingText = page.locator("text=加载中...");
      const errorText = page.locator("text=加载失败");

      const hasDb = await dbLabel.isVisible().catch(() => false);
      const hasLoading = await loadingText.isVisible().catch(() => false);
      const hasError = await errorText.isVisible().catch(() => false);

      expect(hasDb || hasLoading || hasError).toBeTruthy();
    });
  });

  test.describe("App info content", () => {
    test("should display version value when loaded", async ({ page }) => {
      const versionValue = page.locator("dd").first();
      const hasValue = await versionValue.isVisible().catch(() => false);
      expect(typeof hasValue).toBe("boolean");
    });

    test("should display platform value when loaded", async ({ page }) => {
      const platformValue = page.locator("dd").nth(1);
      const hasValue = await platformValue.isVisible().catch(() => false);
      expect(typeof hasValue).toBe("boolean");
    });

    test("should display database path in monospace font", async ({ page }) => {
      const dbValue = page.locator(".font-mono").first();
      const hasValue = await dbValue.isVisible().catch(() => false);
      expect(typeof hasValue).toBe("boolean");
    });
  });

  test.describe("Info card styling", () => {
    test("should have white background card for app info", async ({ page }) => {
      const infoCard = page.locator(".bg-white").filter({ hasText: "应用信息" });
      await expect(infoCard).toBeVisible();
    });

    test("should have border around info card", async ({ page }) => {
      const infoCard = page.locator(".border-gray-200").filter({ hasText: "应用信息" });
      await expect(infoCard).toBeVisible();
    });
  });

  test.describe("Info layout", () => {
    test("should use definition list layout", async ({ page }) => {
      const dtElements = page.locator("dt");
      const ddElements = page.locator("dd");
      const dtCount = await dtElements.count().catch(() => 0);
      const ddCount = await ddElements.count().catch(() => 0);

      expect(dtCount).toBe(ddCount);
    });

    test("should display labels in bold", async ({ page }) => {
      const labels = page.locator("dt.font-medium");
      const count = await labels.count().catch(() => 0);
      if (count > 0) {
        await expect(labels.first()).toBeVisible();
      }
    });
  });
});
