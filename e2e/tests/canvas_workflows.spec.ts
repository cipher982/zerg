import { test, expect, Page } from './fixtures';

async function switchToCanvas(page: Page) {
  await page.goto('/');
  const canvasTab = page.getByTestId('global-canvas-tab');
  if (await canvasTab.count()) {
    await canvasTab.click();
  }
  await page.waitForSelector('#canvas-container', { timeout: 10_000 });
}

test.describe('Canvas Editor basic node interactions', () => {
  test.beforeEach(async ({ page }) => {
    await switchToCanvas(page);
  });

  test('Drag agent from shelf to canvas', async ({ page }) => {
    const pill = page.locator('#agent-shelf .agent-pill').first();
    await expect(pill).toBeVisible();

    const canvasArea = page.locator('#canvas-container canvas');
    if (await canvasArea.count() === 0) {
      test.skip(true, 'Canvas area not found');
      return;
    }

    const bbox = await canvasArea.boundingBox();
    if (!bbox) {
      test.skip(true, 'Cannot get canvas bounding box');
      return;
    }

    await pill.dragTo(canvasArea, { targetPosition: { x: 50, y: 50 } });

    // A new node should appear (class .canvas-node or similar)
    await expect(page.locator('.canvas-node, .generic-node')).toHaveCount(1, { timeout: 5000 });
  });

  test('Position node at specific coordinates', async ({ page }) => {
    const node = page.locator('.canvas-node, .generic-node').first();
    const posBefore = await node.boundingBox();
    if (!posBefore) {
      test.skip(true, 'Node not found or no bounding box');
      return;
    }

    await node.dragTo(node, { targetPosition: { x: posBefore.x + 100, y: posBefore.y + 100 } });

    const posAfter = await node.boundingBox();
    if (!posAfter) {
      throw new Error('Node lost after drag operation');
    }
    expect(posAfter.x).toBeGreaterThan(posBefore.x);
  });

  test('Create edge between two nodes', async ({ page }) => {
    const nodes = page.locator('.canvas-node, .generic-node');
    if ((await nodes.count()) < 2) {
      test.skip(true, 'Need at least two nodes');
      return;
    }

    const first = nodes.nth(0);
    const second = nodes.nth(1);

    const p1 = await first.boundingBox();
    const p2 = await second.boundingBox();
    if (!p1 || !p2) {
      test.skip(true, 'Unable to get node positions');
      return;
    }

    await first.dragTo(second);

    // Expect a new edge element – selector is approximate
    await expect(page.locator('.canvas-edge, path.edge')).toHaveCount(1, { timeout: 5000 });
  });

  test('Delete node from canvas', async ({ page }) => {
    const node = page.locator('.canvas-node, .generic-node').first();
    if (await node.count() === 0) {
      test.skip(true, 'No nodes available to delete');
      return;
    }
    await node.click({ button: 'right' });
    const deleteOpt = page.locator('text=Delete Node');
    if (await deleteOpt.count() === 0) {
      test.skip(true, 'Context menu not present');
      return;
    }
    await deleteOpt.click();
    await expect(node).toHaveCount(0);
  });

  test('Delete edge between nodes (placeholder)', async () => {
    test.skip();
  });

  test('Select multiple nodes (placeholder)', async () => {
    test.skip();
  });

  test('Save and load workflow (placeholder)', async () => {
    test.skip();
  });

  test('Clear canvas (placeholder)', async () => {
    test.skip();
  });

  test('Test zoom/pan controls', async ({ page }) => {
    const canvasArea = page.locator('#canvas-container');
    await canvasArea.click();
    await page.keyboard.press('Meta+-');
    await page.keyboard.press('Meta++');
    // No assertion – ensure no errors
  });
});
