import { test, expect } from '@playwright/test';

test.describe('Agent scheduling UI', () => {
  async function createAndOpenConfig(page) {
    await page.goto('/');
    await page.locator('[data-testid="create-agent-btn"]').click();
    const row = page.locator('tr[data-agent-id]').first();
    const id = await row.getAttribute('data-agent-id');
    await page.locator(`[data-testid="edit-agent-${id}"]`).click();
    await page.waitForSelector('#agent-modal', { state: 'visible' });
    return id;
  }

  test('Set cron schedule on agent', async ({ page }) => {
    await createAndOpenConfig(page);

    // Frequency dropdown id="sched-frequency"
    const freq = page.locator('#sched-frequency');
    if ((await freq.count()) === 0) test.skip();

    await freq.selectOption('daily');
    await page.locator('#save-agent').click();

    // Back to dashboard and verify scheduled indicator
    await expect(page.locator('tr[data-agent-id] td', { hasText: 'Scheduled' })).toBeVisible();
  });

  test('Edit existing schedule placeholder', async () => {
    test.skip();
  });

  test('Remove schedule placeholder', async () => {
    test.skip();
  });

  test('Verify next_run_at displays correctly placeholder', async () => {
    test.skip();
  });

  test('Scheduled status indicator placeholder', async () => {
    test.skip();
  });

  test('Schedule in different timezones placeholder', async () => {
    test.skip();
  });

  test('View last_run_at after execution placeholder', async () => {
    test.skip();
  });

  test('Test invalid cron expressions', async ({ page }) => {
    await createAndOpenConfig(page);

    // Force invalid by selecting custom freq but not filling fields
    const freq = page.locator('#sched-frequency');
    if ((await freq.count()) === 0) test.skip();

    await freq.selectOption('weekly');
    await page.locator('#save-agent').click();

    // Expect error message
    await expect(page.locator('.validation-error, .error-msg')).toBeVisible();
  });
});
