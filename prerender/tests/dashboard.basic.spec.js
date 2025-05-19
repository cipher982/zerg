// Very first Playwright sanity-check.

import { test, expect } from '@playwright/test';

test('Dashboard tab renders', async ({ page }) => {
  // Load root – webServer helper ensures the SPA is available.
  await page.goto('/');

  // The top navigation tab should read “Agent Dashboard”.
  await expect(page.locator('#global-dashboard-tab')).toHaveText('Agent Dashboard');
});
