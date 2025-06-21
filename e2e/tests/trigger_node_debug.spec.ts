import { test, expect, Page } from './fixtures';

test.describe('Trigger Node Debug', () => {
  test.beforeEach(async ({ page }) => {
    // Reset database before each test
    await page.request.post('http://localhost:8001/admin/reset-database');
  });

  test('Debug trigger node appearance', async ({ page }) => {
    console.log('🔍 Starting trigger node debug test');

    // Navigate to home page first
    await page.goto('/');
    console.log('✅ Navigated to home page');

    // Click canvas tab to go to canvas view
    const canvasTab = page.getByTestId('global-canvas-tab');
    if (await canvasTab.count() > 0) {
      await canvasTab.click();
      console.log('✅ Clicked canvas tab');
    } else {
      console.log('❌ Canvas tab not found');
    }

    // Wait for canvas container to appear
    await page.waitForSelector('#canvas-container', { timeout: 15000 });
    console.log('✅ Canvas container appeared');

    // Wait for canvas element itself
    await page.waitForSelector('#node-canvas', { timeout: 5000 });
    console.log('✅ Canvas element found');

    // Check what elements exist on the page
    const allElements = await page.locator('*').count();
    console.log(`📊 Total elements on page: ${allElements}`);

    // Check for any nodes on canvas
    const allNodes = await page.locator('.canvas-node, .node, [class*="node"]').count();
    console.log(`📊 Nodes found: ${allNodes}`);

    // List all classes that contain "node"
    const nodeClasses = await page.evaluate(() => {
      const allElements = document.querySelectorAll('*');
      const nodeElements = [];
      for (let el of allElements) {
        if (el.className && el.className.toString().toLowerCase().includes('node')) {
          nodeElements.push({
            tag: el.tagName,
            classes: el.className.toString(),
            text: el.textContent?.slice(0, 50) || '',
            id: el.id || ''
          });
        }
      }
      return nodeElements;
    });
    console.log(`📊 Elements with "node" in class:`, nodeClasses);

    // Check console logs from the page
    const logs = [];
    page.on('console', msg => {
      if (msg.text().includes('CANVAS') || msg.text().includes('trigger') || msg.text().includes('node')) {
        logs.push(msg.text());
        console.log(`📝 Browser log: ${msg.text()}`);
      }
    });

    // Wait a bit more to see if trigger node appears
    console.log('⏳ Waiting 5 seconds for trigger node to appear...');
    await page.waitForTimeout(5000);

    // Check again for nodes
    const finalNodeCount = await page.locator('.canvas-node, .node, [class*="node"]').count();
    console.log(`📊 Final node count: ${finalNodeCount}`);

    // Try to find trigger node by text content
    const triggerByText = await page.locator('text="▶ Start"').count();
    console.log(`📊 Elements with "▶ Start" text: ${triggerByText}`);

    const triggerByStartText = await page.locator('text="Start"').count();
    console.log(`📊 Elements with "Start" text: ${triggerByStartText}`);

    // Check if there are any SVG or canvas elements that might contain the nodes
    const svgElements = await page.locator('svg').count();
    const canvasElements = await page.locator('canvas').count();
    console.log(`📊 SVG elements: ${svgElements}, Canvas elements: ${canvasElements}`);

    // If we have a canvas, try to see if nodes are drawn on it
    if (canvasElements > 0) {
      const canvasData = await page.evaluate(() => {
        const canvas = document.querySelector('#node-canvas') as HTMLCanvasElement;
        if (canvas) {
          const ctx = canvas.getContext('2d');
          if (ctx) {
            // Get image data to see if anything is drawn
            const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
            let hasContent = false;
            for (let i = 3; i < imageData.data.length; i += 4) {
              if (imageData.data[i] > 0) { // Check alpha channel
                hasContent = true;
                break;
              }
            }
            return {
              width: canvas.width,
              height: canvas.height,
              hasContent
            };
          }
        }
        return null;
      });
      console.log(`📊 Canvas data:`, canvasData);
    }

    // Print all collected logs
    console.log(`📝 All canvas logs:`, logs);

    // Take a screenshot for debugging
    await page.screenshot({ path: 'trigger-node-debug.png', fullPage: true });
    console.log('📸 Screenshot saved as trigger-node-debug.png');

    // The test doesn't need to pass/fail, it's just for debugging
    console.log('🔍 Debug test completed');
  });
});