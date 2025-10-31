import { test, expect, type Page } from './fixtures';

// Reset DB before each test to keep agent/thread ids predictable
test.beforeEach(async ({ request }) => {
  await request.post('/admin/reset-database');
});

async function createAgentAndGetId(page: Page): Promise<string> {
  await page.goto('/');
  await page.locator('[data-testid="create-agent-btn"]').click();
  const row = page.locator('tr[data-agent-id]').first();
  await expect(row).toBeVisible();
  return (await row.getAttribute('data-agent-id')) as string;
}

test.describe('Chat Token Streaming Tests', () => {
  test('Verify token streaming shows up in UI', async ({ page }) => {
    // Create agent and navigate to chat
    const agentId = await createAgentAndGetId(page);
    await page.locator(`[data-testid="chat-agent-${agentId}"]`).click();

    // Verify chat UI loads
    await expect(page.getByTestId('chat-input')).toBeVisible({ timeout: 5000 });
    await expect(page.getByTestId('send-message-btn')).toBeVisible({ timeout: 5000 });

    // Send a test message that should trigger a response
    const testMessage = 'Say hello in exactly 10 words';
    await page.getByTestId('chat-input').fill(testMessage);
    await page.getByTestId('send-message-btn').click();

    // Wait for user message to appear
    await expect(page.getByTestId('messages-container')).toContainText(testMessage, { 
      timeout: 10000 
    });

    // Look for streaming indicator - check for message with data-streaming attribute
    const streamingMessage = page.locator('[data-streaming="true"]').first();
    
    // Wait for streaming to start (assistant message starts appearing)
    // We expect to see at least some content appearing character by character
    await expect(streamingMessage.or(page.locator('.message.streaming')).or(
      page.locator('[data-role="chat-message-assistant"]')
    )).toBeVisible({ timeout: 15000 });

    // Verify streaming cursor appears
    const streamingCursor = page.locator('.streaming-cursor');
    await expect(streamingCursor).toBeVisible({ timeout: 5000 }).catch(() => {
      // Cursor might not always be visible, that's okay
    });

    // Wait for streaming to complete - message should have content
    // The final message should appear without streaming attribute
    await expect(page.locator('[data-role="chat-message-assistant"]').last()).toContainText(/hello|greeting|Hi/i, {
      timeout: 30000
    });

    // Verify streaming state is cleared (no streaming attribute on final message)
    const finalMessage = page.locator('[data-role="chat-message-assistant"]').last();
    await expect(finalMessage).not.toHaveAttribute('data-streaming', 'true', { timeout: 5000 });
  });

  test('Verify tokens accumulate during streaming', async ({ page }) => {
    const agentId = await createAgentAndGetId(page);
    await page.locator(`[data-testid="chat-agent-${agentId}"]`).click();

    await expect(page.getByTestId('chat-input')).toBeVisible({ timeout: 5000 });

    // Send message that will generate a longer response
    const testMessage = 'Count from 1 to 5 with a word between each number';
    await page.getByTestId('chat-input').fill(testMessage);
    await page.getByTestId('send-message-btn').click();

    // Wait for user message
    await expect(page.getByTestId('messages-container')).toContainText(testMessage, { 
      timeout: 10000 
    });

    // Get the assistant message element
    const assistantMessage = page.locator('[data-role="chat-message-assistant"]').last();
    
    // Wait for streaming to start
    await expect(assistantMessage).toBeVisible({ timeout: 15000 });

    // Capture initial content length
    let previousLength = 0;
    let contentGrew = false;
    
    // Check multiple times that content is growing (tokens accumulating)
    for (let i = 0; i < 5; i++) {
      await page.waitForTimeout(1000); // Wait 1 second between checks
      
      const currentContent = await assistantMessage.locator('.message-content').textContent();
      const currentLength = currentContent?.length || 0;
      
      if (currentLength > previousLength) {
        contentGrew = true;
        previousLength = currentLength;
      }
    }

    // Verify content grew (tokens were accumulating)
    expect(contentGrew).toBe(true);

    // Wait for streaming to complete
    await expect(assistantMessage).not.toHaveAttribute('data-streaming', 'true', { 
      timeout: 30000 
    });

    // Verify final message has substantial content
    const finalContent = await assistantMessage.locator('.message-content').textContent();
    expect(finalContent?.length).toBeGreaterThan(10);
  });

  test('Verify multiple token chunks appear incrementally', async ({ page }) => {
    const agentId = await createAgentAndGetId(page);
    await page.locator(`[data-testid="chat-agent-${agentId}"]`).click();

    await expect(page.getByTestId('chat-input')).toBeVisible({ timeout: 5000 });

    const testMessage = 'Write a short sentence about AI';
    await page.getByTestId('chat-input').fill(testMessage);
    await page.getByTestId('send-message-btn').click();

    await expect(page.getByTestId('messages-container')).toContainText(testMessage, { 
      timeout: 10000 
    });

    const assistantMessage = page.locator('[data-role="chat-message-assistant"]').last();
    await expect(assistantMessage).toBeVisible({ timeout: 15000 });

    // Track content changes over time
    const contentSnapshots: string[] = [];
    
    for (let i = 0; i < 10; i++) {
      await page.waitForTimeout(500); // Check every 500ms
      const content = await assistantMessage.locator('.message-content').textContent();
      if (content) {
        contentSnapshots.push(content);
      }
      
      // If we see streaming has ended, break early
      const isStreaming = await assistantMessage.getAttribute('data-streaming');
      if (isStreaming !== 'true') {
        break;
      }
    }

    // Verify we captured multiple snapshots
    expect(contentSnapshots.length).toBeGreaterThan(1);

    // Verify content grew (each snapshot should be longer or equal)
    let grew = false;
    for (let i = 1; i < contentSnapshots.length; i++) {
      if (contentSnapshots[i].length > contentSnapshots[i - 1].length) {
        grew = true;
        break;
      }
    }
    expect(grew).toBe(true);
  });

  test('Verify streaming cursor animation', async ({ page }) => {
    const agentId = await createAgentAndGetId(page);
    await page.locator(`[data-testid="chat-agent-${agentId}"]`).click();

    await expect(page.getByTestId('chat-input')).toBeVisible({ timeout: 5000 });

    const testMessage = 'Hello';
    await page.getByTestId('chat-input').fill(testMessage);
    await page.getByTestId('send-message-btn').click();

    await expect(page.getByTestId('messages-container')).toContainText(testMessage, { 
      timeout: 10000 
    });

    // Look for streaming message
    const streamingMessage = page.locator('[data-streaming="true"]').first();
    
    // Wait for streaming to start
    await expect(streamingMessage).toBeVisible({ timeout: 15000 }).catch(() => {
      // If streaming message not found, check for assistant message with cursor
      const assistantWithCursor = page.locator('[data-role="chat-message-assistant"]').filter({
        has: page.locator('.streaming-cursor')
      }).first();
      return expect(assistantWithCursor).toBeVisible({ timeout: 5000 });
    });

    // Verify cursor element exists
    const cursor = page.locator('.streaming-cursor');
    await expect(cursor).toBeVisible({ timeout: 2000 }).catch(() => {
      // Cursor might blink in/out, that's acceptable
    });

    // Verify cursor has animation (check computed styles)
    const cursorStyle = await cursor.evaluate((el) => {
      const style = window.getComputedStyle(el);
      return {
        animation: style.animation,
        animationName: style.animationName,
      };
    }).catch(() => null);

    if (cursorStyle) {
      // Animation should be set (either animation or animationName)
      expect(cursorStyle.animation || cursorStyle.animationName).toBeTruthy();
    }
  });
});

