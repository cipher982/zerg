import { test, expect } from '@playwright/test';

test.describe('Feature: Thread & Chat Functionality', () => {
  test('Create new thread from agent dashboard', async ({ page }) => {
    await page.goto('/');
    // Ensure at least one agent exists
    await page.click('[data-testid="create-agent-btn"]');
    const agentRow = await page.waitForSelector('[data-agent-id]', { timeout: 5000 });
    const agentId = await agentRow.getAttribute('data-agent-id');
    // Open chat for the agent
    await page.click(`[data-testid="chat-agent-${agentId}"]`);
    // Wait for chat view
    await page.waitForSelector('#chat-view-container', { timeout: 5000 });
    // A new thread should be created automatically
    const threadTabs = page.locator('.thread-list-item');
    expect(await threadTabs.count()).toBeGreaterThan(0);
  });

  test('Send user message and verify agent response', async ({ page }) => {
    await page.goto('/');
    await page.click('[data-testid="create-agent-btn"]');
    const agentRow = await page.waitForSelector('[data-agent-id]');
    const agentId = await agentRow.getAttribute('data-agent-id');
    await page.click(`[data-testid="chat-agent-${agentId}"]`);
    await page.waitForSelector('#chat-view-container');
    // Send a message
    const input = page.locator('#thread-input');
    await input.fill('Hello agent');
    await input.press('Enter');
    // Wait for user message to appear
    await expect(page.locator('.message.user').last()).toHaveText('Hello agent');
    // Wait for assistant response
    await expect(page.locator('.message.assistant')).toBeVisible({ timeout: 10000 });
  });

  test('Switch between multiple threads', async ({ page }) => {
    await page.goto('/');
    await page.click('[data-testid="create-agent-btn"]');
    const agentRow = await page.waitForSelector('[data-agent-id]');
    const agentId = await agentRow.getAttribute('data-agent-id');
    await page.click(`[data-testid="chat-agent-${agentId}"]`);
    await page.waitForSelector('#chat-view-container');
    // Create second thread
    await page.click('button#create-thread-btn');
    // Ensure two threads exist
    const threadList = page.locator('.thread-list-item');
    expect(await threadList.count()).toBeGreaterThanOrEqual(2);
    // Switch to first thread
    await threadList.nth(0).click();
    // Verify content for thread 1
    await expect(page.locator('#thread-title')).toContainText('Thread');
  });

  test('Delete thread and verify removal', async ({ page }) => {
    await page.goto('/');
    await page.click('[data-testid="create-agent-btn"]');
    const agentRow = await page.waitForSelector('[data-agent-id]');
    const agentId = await agentRow.getAttribute('data-agent-id');
    await page.click(`[data-testid="chat-agent-${agentId}"]`);
    await page.waitForSelector('#chat-view-container');
    // Delete current thread
    await page.click('button#delete-thread-btn');
    // Confirm deletion
    await page.click('button:has-text("Confirm")');
    // Verify no threads present
    await expect(page.locator('.thread-list-item')).toHaveCount(0);
  });
  // TODO: Add tests for thread title editing, message history persistence, empty state
});