import { test, expect } from '@playwright/test';

test.describe('Feature: Agent Management', () => {
  test('Create agent with minimal configuration', async ({ page }) => {
    // Navigate to dashboard and create a new agent using default settings
    await page.goto('/');
    await page.click('[data-testid="create-agent-btn"]');
    // Wait for agent row to appear
    await page.waitForSelector('[data-agent-id]', { timeout: 5000 });
    const agentRows = page.locator('[data-agent-id]');
    expect(await agentRows.count()).toBeGreaterThan(0);
  });

  test('Create agent with all fields (instructions, model, temperature)', async ({ page }) => {
    await page.goto('/');
    await page.click('[data-testid="create-agent-btn"]');
    // Wait for the new agent to appear in the table
    const agentRow = await page.waitForSelector('[data-agent-id]', { timeout: 5000 });
    const agentId = await agentRow.getAttribute('data-agent-id');
    // Open the edit modal for the newly created agent
    await page.click(`[data-testid="edit-agent-${agentId}"]`);
    await page.waitForSelector('[data-testid="agent-modal"]', { timeout: 5000 });
    // Fill out all configurable fields
    await page.fill('[data-testid="agent-name-input"]', 'Test Agent Full');
    await page.fill('[data-testid="system-instructions-textarea"]', 'Custom system instructions');
    await page.fill('[data-testid="task-instructions-textarea"]', 'Custom task instructions');
    // TODO: select model and temperature if UI controls are available
    // Save the agent configuration
    await page.click('#save-agent');
    // Ensure the modal closes after saving
    await page.waitForSelector('[data-testid="agent-modal"]', { state: 'hidden', timeout: 5000 });
    // Verify that the agent name was updated in the dashboard
    const nameCell = page.locator(`[data-agent-id="${agentId}"] td`).first();
    await expect(nameCell).toHaveText('Test Agent Full');
  });

  test('Validate required fields show errors', async ({ page }) => {
    await page.goto('/');
    await page.click('[data-testid="create-agent-btn"]');
    const agentRow = await page.waitForSelector('[data-agent-id]', { timeout: 5000 });
    const agentId = await agentRow.getAttribute('data-agent-id');
    await page.click(`[data-testid="edit-agent-${agentId}"]`);
    await page.waitForSelector('[data-testid="agent-modal"]');
    // Clear the required name field to trigger validation
    await page.fill('[data-testid="agent-name-input"]', '');
    // Attempt to save without a name
    await page.click('#save-agent');
    // Expect a validation error message to appear
    await expect(page.locator('.error-message')).toBeVisible();
  });

  test('Edit agent name and verify update', async ({ page }) => {
    await page.goto('/');
    await page.click('[data-testid="create-agent-btn"]');
    const agentRow = await page.waitForSelector('[data-agent-id]');
    const agentId = await agentRow.getAttribute('data-agent-id');
    await page.click(`[data-testid="edit-agent-${agentId}"]`);
    await page.waitForSelector('[data-testid="agent-modal"]');
    // Change the agent name
    await page.fill('[data-testid="agent-name-input"]', 'Renamed Agent');
    await page.click('#save-agent');
    await page.waitForSelector('[data-testid="agent-modal"]', { state: 'hidden' });
    // Verify the updated name in the table
    const updatedCell = page.locator(`[data-agent-id="${agentId}"] td`).first();
    await expect(updatedCell).toHaveText('Renamed Agent');
  });

  // TODO: Implement tests for editing instructions, changing model,
  // deleting agent with confirmation, cancel delete operation,
  // verifying appearance after creation, and status toggle.
});