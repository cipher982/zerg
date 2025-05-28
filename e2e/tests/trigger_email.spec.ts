import { test, expect } from '@playwright/test';

test.describe('Feature: Trigger Management - Email', () => {
  test('Create Gmail email trigger', async ({ page }) => {
    await page.goto('/');
    await page.click('[data-testid="create-agent-btn"]');
    const agentId = await (await page.waitForSelector('[data-agent-id]')).getAttribute('data-agent-id');
    await page.click(`[data-testid="edit-agent-${agentId}"]`);
    await page.click('#agent-triggers-tab');
    // Click add email trigger
    await page.click('#add-email-trigger-btn');
    // Fill filter criteria
    await page.fill('#email-filter-subject', 'Test');
    await page.click('#connect-gmail-btn');
    // TODO: Mock Gmail OAuth flow
    await page.click('#save-trigger-btn');
    await expect(page.locator('#agent-triggers-list li')).toHaveCount(1);
  });

  test('Test Gmail connection status', async ({ page }) => {
    await page.goto('/');
    await page.click('[data-testid="edit-agent-1"]');
    await page.click('#agent-triggers-tab');
    // Expect status indicator
    await expect(page.locator('.gmail-status.connected')).toBeVisible();
  });

  test('Remove email trigger and handle disconnected account', async ({ page }) => {
    await page.goto('/');
    await page.click('[data-testid="edit-agent-1"]');
    await page.click('#agent-triggers-tab');
    // Delete email trigger
    await page.click('#agent-triggers-list li button.delete-trigger-btn');
    await page.click('button:has-text("Confirm")');
    await expect(page.locator('#agent-triggers-list li')).toHaveCount(0);
    // Simulate disconnected Gmail
    // TODO: Mock disconnection and verify UI
  });
});