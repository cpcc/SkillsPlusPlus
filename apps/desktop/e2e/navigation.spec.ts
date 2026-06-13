import { test, expect } from "@playwright/test";

test.describe("Navigation", () => {
  test.describe("Default redirect", () => {
    test("should redirect / to /discover", async ({ page }) => {
      await page.goto("/");
      await expect(page).toHaveURL("/discover");
    });
  });

  test.describe("Sidebar navigation", () => {
    test.beforeEach(async ({ page }) => {
      await page.goto("/");
    });

    test("should display sidebar with brand name", async ({ page }) => {
      const brand = page.locator("nav").locator("text=skills++");
      await expect(brand).toBeVisible();
    });

    test("should display all nav items", async ({ page }) => {
      await expect(page.locator('nav a[href="/discover"]')).toBeVisible();
      await expect(page.locator('nav a[href="/installed"]')).toBeVisible();
      await expect(page.locator('nav a[href="/tools"]')).toBeVisible();
      await expect(page.locator('nav a[href="/settings"]')).toBeVisible();
    });

    test("should highlight active nav item", async ({ page }) => {
      const discoverLink = page.locator('nav a[href="/discover"]');
      await expect(discoverLink).toHaveClass(/bg-brand-50/);
    });

    test("should update active state on navigation", async ({ page }) => {
      await page.click('nav a[href="/tools"]');
      const toolsLink = page.locator('nav a[href="/tools"]');
      await expect(toolsLink).toHaveClass(/bg-brand-50/);

      const discoverLink = page.locator('nav a[href="/discover"]');
      await expect(discoverLink).not.toHaveClass(/bg-brand-50/);
    });
  });

  test.describe("Page navigation", () => {
    test("should navigate to discover page", async ({ page }) => {
      await page.goto("/");
      await page.click('a[href="/discover"]');
      await expect(page).toHaveURL("/discover");
      await expect(page.locator("h2")).toContainText("发现");
    });

    test("should navigate to installed page", async ({ page }) => {
      await page.goto("/");
      await page.click('a[href="/installed"]');
      await expect(page).toHaveURL("/installed");
      await expect(page.locator("h2")).toContainText("已安装");
    });

    test("should navigate to tools page", async ({ page }) => {
      await page.goto("/");
      await page.click('a[href="/tools"]');
      await expect(page).toHaveURL("/tools");
      await expect(page.locator("h2")).toContainText("工具与目录");
    });

    test("should navigate to settings page", async ({ page }) => {
      await page.goto("/");
      await page.click('a[href="/settings"]');
      await expect(page).toHaveURL("/settings");
      await expect(page.locator("h2")).toContainText("设置");
    });
  });

  test.describe("Direct URL access", () => {
    test("should load discover page directly", async ({ page }) => {
      await page.goto("/discover");
      await expect(page.locator("h2")).toContainText("发现");
    });

    test("should load installed page directly", async ({ page }) => {
      await page.goto("/installed");
      await expect(page.locator("h2")).toContainText("已安装");
    });

    test("should load tools page directly", async ({ page }) => {
      await page.goto("/tools");
      await expect(page.locator("h2")).toContainText("工具与目录");
    });

    test("should load settings page directly", async ({ page }) => {
      await page.goto("/settings");
      await expect(page.locator("h2")).toContainText("设置");
    });
  });

  test.describe("Back navigation", () => {
    test("should navigate back to discover from installed", async ({ page }) => {
      await page.goto("/installed");
      await page.click('a[href="/discover"]');
      await expect(page).toHaveURL("/discover");
    });

    test("should navigate back to discover from tools", async ({ page }) => {
      await page.goto("/tools");
      await page.click('a[href="/discover"]');
      await expect(page).toHaveURL("/discover");
    });

    test("should navigate back to discover from settings", async ({ page }) => {
      await page.goto("/settings");
      await page.click('a[href="/discover"]');
      await expect(page).toHaveURL("/discover");
    });
  });

  test.describe("Full navigation flow", () => {
    test("should navigate through all pages in sequence", async ({ page }) => {
      await page.goto("/");

      await page.click('a[href="/installed"]');
      await expect(page).toHaveURL("/installed");

      await page.click('a[href="/tools"]');
      await expect(page).toHaveURL("/tools");

      await page.click('a[href="/settings"]');
      await expect(page).toHaveURL("/settings");

      await page.click('a[href="/discover"]');
      await expect(page).toHaveURL("/discover");
    });
  });
});
