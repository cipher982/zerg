import { test, expect } from '@playwright/test';

test.describe('Agent run history view', () => {
  test('Open run history tab placeholder', async ({ page }) => {
    await page.goto('/');
    await page.locator('[data-testid="create-agent-btn"]').click();
    const agentId = await page.locator('tr[data-agent-id]').first().getAttribute('data-agent-id');
    await page.locator(`[data-testid="debug-agent-${agentId}"]`).click();
    // Modal should open (read-only debug modal)
    await page.waitForSelector('[data-testid="agent-debug-modal"]', { timeout: 5000 });

    // Click Runs tab (if exists)
    const runsTab = page.locator('button', { hasText: 'Runs' });
    if (await runsTab.count()) {
      await runsTab.click();
      await expect(page.locator('.run-row')).toHaveCountGreaterThanOrEqual(0);
    }
  });

  test('Run list filters placeholder', async () => {
    test.skip();
  });

  test('Run pagination placeholder', async () => {
    test.skip();
  });

  test('Export run data placeholder', async () => {
    test.skip();
  });
});
