import { test, expect } from '@playwright/test';

test.describe('Feature: Real-time Updates', () => {
  test('Dashboard updates live on agent creation', async ({ page }) => {
    await page.goto('/');
    // Open second tab
    const page2 = await page.context().newPage();
    await page2.goto('/');
    // Create agent in page1
    await page.click('[data-testid="create-agent-btn"]');
    // page2 should see the new agent without reload
    const newAgent = page2.locator('[data-agent-id]').first();
    await expect(newAgent).toBeVisible({ timeout: 5000 });
  });

  test('WebSocket connection is established', async ({ page }) => {
    await page.goto('/');
    // Expose ws status
    const connected = await page.evaluate(() => window['wsClient']?.isConnected);
    expect(connected).toBe(true);
  });

  test('Message streaming displays tokens', async ({ page }) => {
    await page.goto('/');
    await page.click('[data-testid="create-agent-btn"]');
    const agentId = await (await page.waitForSelector('[data-agent-id]')).getAttribute('data-agent-id');
    await page.click(`[data-testid="chat-agent-${agentId}"]`);
    await page.waitForSelector('#chat-view-container');
    await page.fill('#thread-input', 'Streaming test');
    await page.press('#thread-input', 'Enter');
    // Verify token spans appear
    await expect(page.locator('.message.assistant_token')).toBeVisible({ timeout: 10000 });
  });

  test('Connection recovers after disconnect', async ({ page }) => {
    await page.goto('/');
    // Simulate disconnect
    await page.evaluate(() => window['wsClient']?.socket.close());
    // Wait for reconnection
    await page.waitForFunction(() => window['wsClient']?.isConnected === true, null, { timeout: 10000 });
    expect(await page.evaluate(() => window['wsClient']?.isConnected)).toBe(true);
  });
  // TODO: Multi-tab sync beyond creation and presence indicators
});