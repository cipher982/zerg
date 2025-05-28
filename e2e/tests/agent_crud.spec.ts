import { test, expect, Page } from '@playwright/test';

// Helper to count dashboard agent rows (each <tr data-agent-id="…">)
async function getAgentRowCount(page: Page) {
  return await page.locator('tr[data-agent-id]').count();
}

// Helper that waits for dashboard table to finish any reactive refresh.
// We consider the dashboard ready when the first tbody element exists.
async function waitForDashboardReady(page: Page) {
  await page.goto('/');
  await page.waitForSelector('table');
}

test.describe('Agent CRUD – dashboard interactions', () => {
  test.beforeEach(async ({ page }) => {
    await waitForDashboardReady(page);
  });

  test('Create agent with minimal configuration', async ({ page }) => {
    const before = await getAgentRowCount(page);

    // Click the plus-button in the header (data-testid="create-agent-btn")
    await page.locator('[data-testid="create-agent-btn"]').click();

    // The backend side-effect runs async – wait until a new <tr> appears.
    await expect(page.locator('tr[data-agent-id]')).toHaveCount(before + 1, { timeout: 10_000 });
  });

  test('Verify agent appears in dashboard after creation', async ({ page }) => {
    // The previous test might have created one already but we don’t rely on state.
    await page.locator('[data-testid="create-agent-btn"]').click();

    // Grab the agent name from the toast or directly from the last row.
    // We assume new rows are appended at the top (first row) – adjust if needed.
    const newRow = page.locator('tr[data-agent-id]').first();
    await expect(newRow).toBeVisible();
  });

  test('Edit agent name and instructions', async ({ page }) => {
    // Pick the first agent row and extract its id (data-agent-id attribute).
    const firstRow = page.locator('tr[data-agent-id]').first();
    const agentId = await firstRow.getAttribute('data-agent-id');
    expect(agentId).not.toBeNull();

    // Open modal by clicking the ✎-button (data-testid="edit-agent-{id}")
    await page.locator(`[data-testid="edit-agent-${agentId}"]`).click();

    // Modal should become visible.
    await page.waitForSelector('#agent-modal', { state: 'visible' });

    // Clear and type new name.
    const NAME_INPUT = page.locator('[data-testid="agent-name-input"]');
    await NAME_INPUT.fill('Edited Agent');

    // Update system instructions as well.
    const SYS_TEXTAREA = page.locator('[data-testid="system-instructions-textarea"]');
    await SYS_TEXTAREA.fill('You are a helpful AI assistant.');

    // Save → button id="save-agent".
    await page.locator('#save-agent').click();

    // Modal auto-closes; verify row text updated.
    await expect(page.locator(`tr[data-agent-id="${agentId}"] td`)).toContainText('Edited Agent');
  });

  test('Delete agent with confirmation dialog', async ({ page }) => {
    // Create a dedicated agent to delete so the test is idempotent.
    await page.locator('[data-testid="create-agent-btn"]').click();
    const newRow = page.locator('tr[data-agent-id]').first();
    const agentId = await newRow.getAttribute('data-agent-id');

    // Intercept the confirmation dialog and accept it.
    page.once('dialog', (dialog) => dialog.accept());

    // Click 🗑️ – delete button.
    await page.locator(`[data-testid="delete-agent-${agentId}"]`).click();

    // Row should disappear.
    await expect(page.locator(`tr[data-agent-id="${agentId}"]`)).toHaveCount(0);
  });

  test('Cancel delete operation keeps agent intact', async ({ page }) => {
    // Create agent to attempt deletion.
    await page.locator('[data-testid="create-agent-btn"]').click();
    const newRow = page.locator('tr[data-agent-id]').first();
    const agentId = await newRow.getAttribute('data-agent-id');

    // Dismiss the confirmation dialog.
    page.once('dialog', (dialog) => dialog.dismiss());

    await page.locator(`[data-testid="delete-agent-${agentId}"]`).click();

    // Row should still be present.
    await expect(page.locator(`tr[data-agent-id="${agentId}"]`)).toHaveCount(1);
  });

  test('Change agent model selection', async ({ page }) => {
    // Assumes at least one agent exists
    const firstRow = page.locator('tr[data-agent-id]').first();
    const agentId = await firstRow.getAttribute('data-agent-id');
    await page.locator(`[data-testid="edit-agent-${agentId}"]`).click();

    await page.waitForSelector('#agent-modal', { state: 'visible' });

    // Open model dropdown (id="model-select") – may be inside modal or global.
    const MODEL_SELECT = page.locator('#model-select');
    if (await MODEL_SELECT.count() === 0) {
      test.skip('Model selector not present in UI yet');
    }

    const current = await MODEL_SELECT.inputValue();
    // pick the second option if exists
    const options = MODEL_SELECT.locator('option');
    const count = await options.count();
    if (count < 2) {
      test.skip('Only one model option available');
    }
    const secondVal = await options.nth(1).getAttribute('value');
    await MODEL_SELECT.selectOption(secondVal!);

    await page.locator('#save-agent').click();

    // currently no clear UI indicator – we simply assert modal closed.
    await expect(page.locator('#agent-modal')).toHaveCount(0);
  });

  test('Create agent with all fields (instructions, model, temperature)', async ({ page }) => {
    await page.goto('/');
    await page.locator('[data-testid="create-agent-btn"]').click();

    // Open modal for the new row (topmost)
    const newRow = page.locator('tr[data-agent-id]').first();
    const agentId = await newRow.getAttribute('data-agent-id');
    await page.locator(`[data-testid="edit-agent-${agentId}"]`).click();
    await page.waitForSelector('#agent-modal', { state: 'visible' });

    // Fill name / instructions
    await page.locator('[data-testid="agent-name-input"]').fill('Full Config Agent');
    await page.locator('[data-testid="system-instructions-textarea"]').fill('System text');
    await page.locator('[data-testid="task-instructions-textarea"]').fill('Task text');

    // Temperature slider/input id maybe "temperature-input" – try to set value.
    const tempInput = page.locator('#temperature-input');
    if (await tempInput.count()) {
      await tempInput.fill('0.9');
    }

    // Model select #model-select – choose last option if exists.
    const modelSel = page.locator('#model-select');
    if (await modelSel.count()) {
      const opts = modelSel.locator('option');
      const lastVal = await opts.last().getAttribute('value');
      if (lastVal) await modelSel.selectOption(lastVal);
    }

    await page.locator('#save-agent').click();
    await expect(page.locator(`#dashboard tr[data-agent-id="${agentId}"] td`)).toContainText('Full Config Agent');
  });

  test('Validate required fields show errors', async ({ page }) => {
    // Open create modal by clicking Create without saving behind scenes – assume button toggles modal creation? Missing spec.
    // Fallback: open first agent edit modal and clear name then click save.
    const firstRow = page.locator('tr[data-agent-id]').first();
    const agentId = await firstRow.getAttribute('data-agent-id');
    await page.locator(`[data-testid="edit-agent-${agentId}"]`).click();
    await page.waitForSelector('#agent-modal', { state: 'visible' });

    const NAME_INPUT = page.locator('[data-testid="agent-name-input"]');
    await NAME_INPUT.fill('');
    await page.locator('#save-agent').click();

    // Expect some validation error element – class .error-msg or aria-invalid.
    const ERR = page.locator('.validation-error, .error-msg, [aria-invalid="true"]');
    await expect(ERR.first()).toBeVisible();

    // Close modal to clean state (click X or overlay)
    await page.keyboard.press('Escape');
  });

  test('Agent status toggle (active/inactive)', async ({ page }) => {
    const firstRow = page.locator('tr[data-agent-id]').first();
    const agentId = await firstRow.getAttribute('data-agent-id');

    // locate toggle button maybe data-testid="toggle-agent-{id}".
    const toggleBtn = page.locator(`[data-testid="toggle-agent-${agentId}"]`);
    if (await toggleBtn.count() === 0) {
      test.skip('Toggle status button not implemented yet');
    }

    const beforeText = await firstRow.textContent();
    await toggleBtn.click();
    await page.waitForTimeout(500); // allow update
    const afterText = await firstRow.textContent();
    expect(afterText).not.toEqual(beforeText);
  });
});
