/**
 * Visual regression smoke test for /admin/marketplace/orders.html
 * Requires admin login cookie/session to be set (or staging env w/ test admin).
 */
const { test, expect } = require('@playwright/test');

test.describe('Admin / Marketplace / Open Orders', () => {
  test.beforeEach(async ({ page }) => {
    // If a session cookie is provided via env, set it before navigation.
    if (process.env.ADMIN_SESSION_COOKIE) {
      await page.context().addCookies([{
        name: 'session',
        value: process.env.ADMIN_SESSION_COOKIE,
        url: process.env.PLAYWRIGHT_BASE_URL || 'http://localhost:8080',
      }]);
    }
    await page.goto('/admin/marketplace/orders.html');
    await page.waitForSelector('#orders-table tbody tr', { timeout: 10_000 });
  });

  test('renders KPI grid + table + filters', async ({ page }) => {
    await expect(page.locator('.admin-page-title')).toHaveText('Open Orders');
    await expect(page.locator('.admin-kpi-card')).toHaveCount(3);
    await expect(page.locator('#orders-table thead th').first()).toBeVisible();
    await expect(page.locator('.mp-filter-bar')).toBeVisible();
    await expect(page.locator('.admin-tabs .admin-tab')).toHaveCount(6);
  });

  test('command palette opens with Cmd/Ctrl+K', async ({ page }) => {
    const isMac = process.platform === 'darwin';
    await page.keyboard.press(isMac ? 'Meta+K' : 'Control+K');
    await expect(page.locator('#orders-palette')).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(page.locator('#orders-palette')).toBeHidden();
  });

  test('help overlay opens with ?', async ({ page }) => {
    await page.keyboard.press('?');
    await expect(page.locator('#orders-help')).toBeVisible();
    await page.keyboard.press('Escape');
  });

  test('full-page screenshot stable', async ({ page }) => {
    await page.waitForTimeout(500); // allow KPIs/sparkline to settle
    await expect(page).toHaveScreenshot('orders-page.png', { fullPage: true });
  });
});
