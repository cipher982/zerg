import { test, expect } from '@playwright/test';

test.describe('Feature: Canvas Editor Workflows', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    // Navigate to Canvas view
    await page.click('[data-testid="global-canvas-tab"]');
    await page.waitForSelector('#canvas-container');
  });

  test('Drag agent from shelf to canvas', async ({ page }) => {
    // Locate first agent in shelf
    const agentIcon = page.locator('.canvas-shelf .agent-icon').first();
    const box = await agentIcon.boundingBox();
    const canvas = page.locator('#canvas-container');
    const canvasBox = await canvas.boundingBox();
    if (box && canvasBox) {
      await page.mouse.move(box.x + box.width/2, box.y + box.height/2);
      await page.mouse.down();
      await page.mouse.move(canvasBox.x + 100, canvasBox.y + 100);
      await page.mouse.up();
      // Expect a node element on canvas
      await expect(page.locator('.canvas-node')).toHaveCount(1);
    }
  });

  test('Create and delete a node', async ({ page }) => {
    // Add node
    const shelfIcon = page.locator('.canvas-shelf .node-icon').first();
    await shelfIcon.dragTo(page.locator('#canvas-container'));
    await expect(page.locator('.canvas-node')).toHaveCount(1);
    // Delete node
    await page.click('.canvas-node');
    await page.keyboard.press('Delete');
    await expect(page.locator('.canvas-node')).toHaveCount(0);
  });

  test('Draw and remove edge between nodes', async ({ page }) => {
    // Add two nodes
    const icons = await page.locator('.canvas-shelf .node-icon');
    await icons.nth(0).dragTo(page.locator('#canvas-container'), { targetPosition: { x: 100, y: 100 }});
    await icons.nth(1).dragTo(page.locator('#canvas-container'), { targetPosition: { x: 300, y: 100 }});
    // Connect nodes: simulate drag from port
    await page.mouse.move(100 + 50, 100 + 20);
    await page.mouse.down();
    await page.mouse.move(300 + 10, 100 + 20);
    await page.mouse.up();
    await expect(page.locator('.canvas-edge')).toHaveCount(1);
    // Delete edge
    await page.click('.canvas-edge');
    await page.keyboard.press('Delete');
    await expect(page.locator('.canvas-edge')).toHaveCount(0);
  });

  test('Select multiple nodes', async ({ page }) => {
    // Add nodes at different positions
    const icon = page.locator('.canvas-shelf .node-icon').first();
    await icon.dragTo(page.locator('#canvas-container'), { targetPosition: { x: 100, y: 100 }});
    await icon.dragTo(page.locator('#canvas-container'), { targetPosition: { x: 200, y: 200 }});
    // Drag selection box
    await page.mouse.move(90, 90);
    await page.mouse.down();
    await page.mouse.move(210, 210);
    await page.mouse.up();
    expect(await page.locator('.canvas-node.selected').count()).toBe(2);
  });

  test('Save and load workflow state', async ({ page }) => {
    // Assume a Save button exists
    await page.click('#save-workflow-btn');
    // Reload page
    await page.reload();
    await page.click('[data-testid="global-canvas-tab"]');
    // Assume Load button or auto-load
    await page.click('#load-workflow-btn');
    // Verify nodes persist
    await expect(page.locator('.canvas-node')).toHaveCountGreaterThan(0);
  });

  test('Zoom and pan controls work', async ({ page }) => {
    // Zoom in
    await page.click('#zoom-in-btn');
    // Pan
    await page.mouse.move(400, 300);
    await page.mouse.down();
    await page.mouse.move(350, 250);
    await page.mouse.up();
    // TODO: verify canvas transform attributes
  });
});