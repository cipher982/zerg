import { test, expect } from '@playwright/test';

test.describe('Feature: Agent Scheduling', () => {
  test('Set and display cron schedule on agent', async ({ page }) => {
    await page.goto('/');
    // Create agent and open config modal
    await page.click('[data-testid="create-agent-btn"]');
    const agentRow = await page.waitForSelector('[data-agent-id]');
    const agentId = await agentRow.getAttribute('data-agent-id');
    await page.click(`[data-testid="edit-agent-${agentId}"]`);
    await page.waitForSelector('#agent-modal');
    // Set schedule to every hour
    await page.selectOption('#sched-frequency', 'hourly');
    // Save changes
    await page.click('#save-agent');
    await page.waitForSelector('#agent-modal', { state: 'hidden' });
    // Verify status indicator in dashboard
    const status = page.locator(`[data-agent-id="${agentId}"] .status-indicator`);
    await expect(status).toContainText('⏱');
  });

  test('Edit and remove schedule', async ({ page }) => {
    await page.goto('/');
    await page.click('[data-testid="create-agent-btn"]');
    const agentRow = await page.waitForSelector('[data-agent-id]');
    const agentId = await agentRow.getAttribute('data-agent-id');
    await page.click(`[data-testid="edit-agent-${agentId}"]`);
    await page.waitForSelector('#agent-modal');
    // Edit schedule: set daily
    await page.selectOption('#sched-frequency', 'daily');
    await page.click('#save-agent');
    await page.waitForSelector('#agent-modal', { state: 'hidden' });
    // Remove schedule
    await page.click(`[data-testid="edit-agent-${agentId}"]`);
    await page.waitForSelector('#agent-modal');
    await page.selectOption('#sched-frequency', 'none');
    await page.click('#save-agent');
    await page.waitForSelector('#agent-modal', { state: 'hidden' });
    // Status indicator should no longer show scheduled
    const status = page.locator(`[data-agent-id="${agentId}"] .status-indicator`);
    await expect(status).not.toContainText('⏱');
  });

  test('Show error for invalid cron expressions', async ({ page }) => {
    await page.goto('/');
    await page.click('[data-testid="create-agent-btn"]');
    const agentRow = await page.waitForSelector('[data-agent-id]');
    const agentId = await agentRow.getAttribute('data-agent-id');
    await page.click(`[data-testid="edit-agent-${agentId}"]`);
    await page.waitForSelector('#agent-modal');
    // Manually input invalid cron
    await page.fill('#schedule-controls input[type=text]', 'invalid');
    await page.click('#save-agent');
    // Expect validation error
    await expect(page.locator('.error-message')).toBeVisible();
  });
  // TODO: Test timezones and verify next_run_at and last_run_at displays
});