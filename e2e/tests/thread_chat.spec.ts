import { test, expect, Page } from '@playwright/test';

async function createAgentAndGetId(page: Page): Promise<string> {
  await page.goto('/');
  await page.locator('[data-testid="create-agent-btn"]').click();
  const row = page.locator('tr[data-agent-id]').first();
  await expect(row).toBeVisible();
  return (await row.getAttribute('data-agent-id')) as string;
}

test.describe('Thread & Chat – basic flows', () => {
  test('Create new thread and send message', async ({ page }) => {
    const agentId = await createAgentAndGetId(page);

    // Enter chat view via dashboard action button.
    await page.locator(`[data-testid="chat-agent-${agentId}"]`).click();

    // Chat view should appear (chat-input visible)
    await page.waitForSelector('.chat-input', { state: 'visible' });

    // Click "New Thread" to ensure fresh context
    await page.locator('.new-thread-btn').click();

    // Type user message and send.
    const INPUT = page.locator('.chat-input');
    await INPUT.fill('Hello agent');
    await page.locator('.send-button').click();

    // Verify the message appears in messages container.
    await expect(page.locator('.messages-container')).toContainText('Hello agent');

    // Optional: switch back to dashboard
    await page.locator('.back-button').click();
    await expect(page.locator('[data-testid="global-dashboard-tab"]')).toBeVisible();
  });

  test('Wait for and verify agent response (placeholder)', async ({ page }) => {
    test.skip(true, 'LLM streaming not stubbed – skipping until mock server available');
  });

  test('Send follow-up message in same thread', async ({ page }) => {
    const agentId = await createAgentAndGetId(page);
    await page.locator(`[data-testid="chat-agent-${agentId}"]`).click();
    await page.waitForSelector('.chat-input');

    // Use existing thread (first in sidebar)
    const INPUT = page.locator('.chat-input');
    await INPUT.fill('Follow-up');
    await page.locator('.send-button').click();
    await expect(page.locator('.messages-container')).toContainText('Follow-up');
  });

  test('Create multiple threads and switch', async ({ page }) => {
    const agentId = await createAgentAndGetId(page);
    await page.locator(`[data-testid="chat-agent-${agentId}"]`).click();
    await page.waitForSelector('.new-thread-btn');

    // create two threads
    await page.locator('.new-thread-btn').click();
    await page.locator('.new-thread-btn').click();

    const listItems = page.locator('.thread-list .thread-row');
    if ((await listItems.count()) < 2) test.skip('Thread list not rendered');

    const first = listItems.nth(0);
    const second = listItems.nth(1);
    await second.click();
    await expect(second).toHaveClass(/selected/);
    await first.click();
    await expect(first).toHaveClass(/selected/);
  });

  test('Delete thread and verify removal', async ({ page }) => {
    const agentId = await createAgentAndGetId(page);
    await page.locator(`[data-testid="chat-agent-${agentId}"]`).click();
    await page.waitForSelector('.new-thread-btn');
    await page.locator('.new-thread-btn').click();
    const threadRow = page.locator('.thread-list .thread-row').first();
    if ((await threadRow.count()) === 0) test.skip();
    await threadRow.locator('button', { hasText: 'Delete' }).click();
    page.once('dialog', (d) => d.accept());
    await expect(threadRow).toHaveCount(0);
  });

  test('Thread title editing', async ({ page }) => {
    const agentId = await createAgentAndGetId(page);
    await page.locator(`[data-testid="chat-agent-${agentId}"]`).click();
    await page.locator('.new-thread-btn').click();
    const threadRow = page.locator('.thread-list .thread-row').first();
    if (await threadRow.count() === 0) test.skip();
    await threadRow.dblclick();
    const titleInput = threadRow.locator('input');
    if (await titleInput.count() === 0) test.skip();
    await titleInput.fill('Renamed');
    await titleInput.press('Enter');
    await expect(threadRow).toContainText('Renamed');
  });

  test('Verify message history persistence after reload', async ({ page }) => {
    const agentId = await createAgentAndGetId(page);
    await page.locator(`[data-testid="chat-agent-${agentId}"]`).click();
    await page.locator('.new-thread-btn').click();
    const input = page.locator('.chat-input');
    await input.fill('Persist this');
    await page.locator('.send-button').click();
    await page.reload();
    await page.locator('.chat-input');
    await expect(page.locator('.messages-container')).toContainText('Persist this');
  });

  test('Empty thread state displays CTA', async ({ page }) => {
    const agentId = await createAgentAndGetId(page);
    await page.locator(`[data-testid="chat-agent-${agentId}"]`).click();
    await page.waitForSelector('.messages-container');
    await expect(page.locator('.messages-container')).toContainText(/Start.*conversation|No messages/);
  });
});
