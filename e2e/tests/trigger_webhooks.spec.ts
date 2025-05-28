import { test, expect } from '@playwright/test';

test.describe('Feature: Trigger Management - Webhooks', () => {
  test('Create webhook trigger', async ({ page }) => {
    await page.goto('/');
    await page.click('[data-testid="create-agent-btn"]');
    const agentId = await (await page.waitForSelector('[data-agent-id]')).getAttribute('data-agent-id');
    await page.click(`[data-testid="edit-agent-${agentId}"]`);
    // Switch to Triggers tab
    await page.click('#agent-triggers-tab');
    // Click add webhook trigger
    await page.click('#add-webhook-btn');
    // Fill trigger details
    await page.fill('#webhook-name', 'Test Webhook');
    await page.click('#save-trigger-btn');
    // Expect trigger in list
    const items = page.locator('#agent-triggers-list li');
    expect(await items.count()).toBeGreaterThan(0);
  });

  test('Copy webhook URL and view secret', async ({ page }) => {
    await page.goto('/');
    // Assume one trigger exists
    await page.click('[data-testid="edit-agent-1"]');
    await page.click('#agent-triggers-tab');
    // Click copy URL
    await page.click('button.copy-webhook-url');
    // TODO: Verify clipboard content
    // View secret
    await page.click('button.view-webhook-secret');
    await expect(page.locator('#webhook-secret')).toBeVisible();
  });

  test('Delete webhook trigger', async ({ page }) => {
    await page.goto('/');
    await page.click('[data-testid="edit-agent-1"]');
    await page.click('#agent-triggers-tab');
    // Delete first trigger
    await page.click('#agent-triggers-list li button.delete-trigger-btn');
    await page.click('button:has-text("Confirm")');
    await expect(page.locator('#agent-triggers-list li')).toHaveCount(0);
  });

  test('Test multiple triggers per agent', async ({ page }) => {
    await page.goto('/');
    await page.click('[data-testid="edit-agent-1"]');
    await page.click('#agent-triggers-tab');
    // Add two triggers
    await page.click('#add-webhook-btn'); await page.click('#save-trigger-btn');
    await page.click('#add-webhook-btn'); await page.click('#save-trigger-btn');
    const count = await page.locator('#agent-triggers-list li').count();
    expect(count).toBeGreaterThanOrEqual(2);
  });
});