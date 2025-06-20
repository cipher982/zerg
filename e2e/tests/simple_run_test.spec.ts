import { test, expect, Page } from './fixtures';

test.describe('Simple Run Button Test', () => {
  test.beforeEach(async ({ page }) => {
    await page.request.post('http://localhost:8001/admin/reset-database');
  });

  test('Run button with just trigger node', async ({ page }) => {
    console.log('🚀 Testing run button with trigger node only');

    // Navigate to canvas
    await page.goto('/');
    const canvasTab = page.getByTestId('global-canvas-tab');
    if (await canvasTab.count() > 0) {
      await canvasTab.click();
      console.log('✅ Clicked canvas tab');
    }

    // Wait for canvas and trigger node
    await page.waitForSelector('#canvas-container', { timeout: 10000 });
    await page.waitForSelector('#node-canvas', { timeout: 5000 });
    
    // Wait for trigger node creation (from logs)
    await page.waitForTimeout(2000);
    console.log('✅ Canvas loaded, trigger node should exist');

    // Find run button
    const runButton = page.locator('#run-workflow-btn');
    await expect(runButton).toBeVisible({ timeout: 5000 });
    
    const initialText = await runButton.textContent();
    console.log(`🔘 Initial run button text: "${initialText}"`);

    // Set up WebSocket message logging
    page.on('websocket', ws => {
      console.log(`🔌 WebSocket connection: ${ws.url()}`);
      ws.on('framereceived', event => {
        console.log(`📥 WebSocket message received: ${event.payload}`);
      });
      ws.on('framesent', event => {
        console.log(`📤 WebSocket message sent: ${event.payload}`);
      });
    });

    // Click run button
    console.log('🎯 Clicking run button...');
    await runButton.click();

    // Wait for button to show "Starting..." 
    try {
      await page.waitForFunction(() => {
        const btn = document.querySelector('#run-workflow-btn');
        return btn && btn.textContent?.includes('Starting');
      }, { timeout: 5000 });
      
      const startingText = await runButton.textContent();
      console.log(`🔄 Button changed to: "${startingText}"`);

      // Now wait for it to complete (should not take long with just trigger)
      await page.waitForFunction(() => {
        const btn = document.querySelector('#run-workflow-btn');
        const text = btn?.textContent || '';
        return !text.includes('Starting') && !text.includes('Running');
      }, { timeout: 15000 });

      const finalText = await runButton.textContent();
      console.log(`✅ Final button text: "${finalText}"`);
      
      // Test passes if button is no longer stuck on "Starting..."
      expect(finalText).not.toContain('Starting');
      expect(finalText).not.toContain('Running');
      
      console.log('🎉 Run button test PASSED!');
      
    } catch (error) {
      const stuckText = await runButton.textContent();
      console.log(`❌ Button stuck at: "${stuckText}"`);
      
      // Capture any console messages about WebSocket
      const logs = await page.evaluate(() => {
        return (window as any).consoleMessages || [];
      });
      console.log('📝 Console logs:', logs);
      
      // Take screenshot
      await page.screenshot({ path: 'run-button-stuck.png', fullPage: true });
      
      throw new Error(`Run button stuck at: "${stuckText}"`);
    }
  });
});