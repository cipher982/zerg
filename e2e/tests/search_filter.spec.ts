import { test, expect } from '@playwright/test';

test.describe('Feature: Search & Filtering', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    // Ensure multiple agents exist
    for (let i = 0; i < 3; i++) {
      await page.click('[data-testid="create-agent-btn"]');
      await page.waitForTimeout(200);
    }
    await page.waitForSelector('[data-agent-id]');
  });

  test('Search agents by name', async ({ page }) => {
    await page.fill('[data-testid="agent-search-input"]', 'Agent');
    const rows = page.locator('[data-agent-id]');
    expect(await rows.count()).toBeGreaterThan(0);
  });

  test('Filter by agent status', async ({ page }) => {
    // Assume a status filter dropdown exists
    await page.selectOption('#status-filter', 'idle');
    const rows = page.locator('[data-agent-id]');
    for (let i = 0; i < await rows.count(); i++) {
      await expect(rows.nth(i).locator('.status-indicator')).toContainText('Idle');
    }
  });

  test('Sort by name ascending and descending', async ({ page }) => {
    await page.click('th[data-column="name"]');
    // Verify ascending order
    // TODO: implement order check
    await page.click('th[data-column="name"]');
    // Verify descending order
  });

  test('Combine search and filters and clear all', async ({ page }) => {
    await page.fill('[data-testid="agent-search-input"]', 'Agent');
    await page.selectOption('#status-filter', 'idle');
    // Expect combined filter
    const rows = page.locator('[data-agent-id]');
    expect(await rows.count()).toBeGreaterThan(0);
    // Clear filters
    await page.click('#clear-filters-btn');
    expect(await rows.count()).toBeGreaterThan(0);
  });
});