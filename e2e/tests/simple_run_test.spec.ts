import { test, expect, Page } from './fixtures';

test.describe('Simple Run Button Test', () => {
  test.beforeEach(async ({ page }) => {
    await page.request.post('http://localhost:8001/admin/reset-database');
  });

  test('Run button with just trigger node', async ({ page }) => {
    console.log('🚀 Testing run button with trigger node only');
    
    // Capture browser console logs
    page.on('console', msg => {
      console.log(`BROWSER: ${msg.type()}: ${msg.text()}`);
    });

    // Set up WebSocket monitoring BEFORE navigation
    const wsMessages: any[] = [];
    let wsConnected = false;
    let wsUrl = '';
    
    page.on('websocket', ws => {
      wsUrl = ws.url();
      console.log(`🔌 WebSocket connection attempt: ${wsUrl}`);
      wsConnected = true;
      
      ws.on('framereceived', event => {
        const payload = event.payload?.toString() || '';
        wsMessages.push({ type: 'received', data: payload });
        
        // Log execution_finished messages in full
        if (payload.includes('execution_finished')) {
          console.log('🎯 EXECUTION_FINISHED MESSAGE:', payload);
        } else {
          console.log(`📥 WS received: ${payload.substring(0, 100)}...`);
        }
      });
      
      ws.on('framesent', event => {
        const payload = event.payload?.toString() || '';
        console.log(`📤 WS sent: ${payload.substring(0, 100)}...`);
        wsMessages.push({ type: 'sent', data: payload });
        
        // Log subscription messages specifically
        if (payload.includes('workflow_execution')) {
          console.log('📌 SUBSCRIPTION MESSAGE:', payload);
        }
      });
      
      ws.on('close', () => {
        console.log('❌ WebSocket closed');
      });
    });

    // Navigate to canvas
    await page.goto('/');
    
    // Wait for app initialization
    await page.waitForTimeout(2000);
    console.log(`🔍 WebSocket status after nav: connected=${wsConnected}, url=${wsUrl}`);
    
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
      console.log('📨 WebSocket messages captured:', wsMessages.length);
      wsMessages.forEach((msg, i) => {
        console.log(`  ${i}: ${msg.type} - ${JSON.stringify(msg.data).substring(0, 100)}...`);
      });
      
      // Take screenshot
      await page.screenshot({ path: 'run-button-stuck.png', fullPage: true });
      
      throw new Error(`Run button stuck at: "${stuckText}"`);
    }
  });
});