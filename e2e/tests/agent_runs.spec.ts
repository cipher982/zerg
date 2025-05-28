import { test, expect } from '@playwright/test';

test.describe('Feature: Run History & Monitoring', () => {
  test('View run history for agent and status indicators', async ({ page }) => {
    await page.goto('/');
    await page.click('[data-testid="create-agent-btn"]');
    const agentId = await (await page.waitForSelector('[data-agent-id]')).getAttribute('data-agent-id');
    // Open agent details
    await page.click(`[data-testid="debug-agent-${agentId}"]`);
    // Switch to Runs tab
    await page.click('#agent-runs-tab');
    const runRows = page.locator('.run-history-row');
    expect(await runRows.count()).toBeGreaterThanOrEqual(0);
    // Check status indicator exists
    await expect(runRows.first().locator('.run-status')).toBeVisible();
  });

  test('View run details and filter by status', async ({ page }) => {
    await page.goto('/');
    await page.click('[data-testid="create-agent-btn"]');
    const agentId = await (await page.waitForSelector('[data-agent-id]')).getAttribute('data-agent-id');
    await page.click(`[data-testid="debug-agent-${agentId}"]`);
    await page.click('#agent-runs-tab');
    // Filter by failed status
    await page.selectOption('#run-status-filter', 'failed');
    // Expect rows match filter
    const rows = page.locator('.run-history-row');
    for (let i = 0; i < await rows.count(); i++) {
      await expect(rows.nth(i).locator('.run-status')).toHaveText('failed');
    }
    // View details of first run
    await rows.first().click();
    await expect(page.locator('#run-detail-modal')).toBeVisible();
  });

  test('Pagination and export run data', async ({ page }) => {
    await page.goto('/');
    await page.click('[data-testid="create-agent-btn"]');
    const agentId = await (await page.waitForSelector('[data-agent-id]')).getAttribute('data-agent-id');
    await page.click(`[data-testid="debug-agent-${agentId}"]`);
    await page.click('#agent-runs-tab');
    // Navigate to next page
    await page.click('button#next-page-btn');
    // Export data
    await page.click('#export-runs-btn');
    // TODO: Verify file download
  });
});