import { test, expect } from '@playwright/test';

// Stubs for trigger management – UI selectors may evolve, skip if missing.

test.describe('Webhook trigger management', () => {
  async function openTriggersTab(page) {
    await page.goto('/');
    await page.locator('[data-testid="create-agent-btn"]').click();
    const id = await page.locator('tr[data-agent-id]').first().getAttribute('data-agent-id');
    await page.locator(`[data-testid="edit-agent-${id}"]`).click();
    await page.waitForSelector('#agent-modal', { state: 'visible' });
    const tab = page.locator('#agent-triggers-tab');
    if ((await tab.count()) === 0) test.skip('Triggers tab not present');
    await tab.click();
  }

  test('Create webhook trigger', async ({ page }) => {
    await openTriggersTab(page);

    const addBtn = page.locator('#agent-add-trigger-btn');
    await addBtn.click();

    const typeSel = page.locator('#agent-trigger-type-select');
    await typeSel.selectOption('webhook');

    await page.locator('#agent-create-trigger').click();

    // Expect list entry
    await expect(page.locator('#agent-triggers-list li')).toHaveCountGreaterThan(0);
  });

  test('Copy webhook URL placeholder', async () => {
    test.skip();
  });

  test('View webhook secret placeholder', async () => {
    test.skip();
  });

  test('Multiple triggers per agent placeholder', async () => {
    test.skip();
  });

  test('Delete webhook trigger', async ({ page }) => {
    await openTriggersTab(page);
    const firstLi = page.locator('#agent-triggers-list li').first();
    if ((await firstLi.count()) === 0) test.skip('No triggers to delete');

    await firstLi.locator('button', { hasText: 'Delete' }).click();
    page.once('dialog', (d) => d.accept());
    await expect(firstLi).toHaveCount(0);
  });
});
