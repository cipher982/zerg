import { test, expect, Page } from './fixtures';

test.describe('Canvas Workflow Execution', () => {
  test.beforeEach(async ({ page }) => {
    await page.request.post('http://localhost:8001/admin/reset-database');
  });

  test('Canvas workflow execution with agents', async ({ page }) => {
    console.log('🚀 Starting canvas workflow execution test');

    // Create an agent first via API
    const agentResponse = await page.request.post('http://localhost:8001/api/agents/', {
      data: {
        name: 'Test Agent E2E',
        system_instructions: 'You are a test agent.',
        task_instructions: 'Execute the given task.',
        model: 'gpt-4o-mini'
      }
    });
    const agent = await agentResponse.json();
    console.log(`✅ Created agent ID: ${agent.id}`);

    // Navigate to canvas
    await page.goto('/');
    const canvasTab = page.getByTestId('global-canvas-tab');
    if (await canvasTab.count() > 0) {
      await canvasTab.click();
      console.log('✅ Clicked canvas tab');
    }

    // Wait for canvas to load
    await page.waitForSelector('#canvas-container', { timeout: 10000 });
    await page.waitForSelector('#node-canvas', { timeout: 5000 });
    console.log('✅ Canvas loaded');

    // Wait for trigger node to be created (check console logs)
    await page.waitForFunction(() => {
      return window.console.logs?.some(log => 
        log.includes('CANVAS: Trigger node created successfully')
      ) || document.querySelector('#node-canvas')?.getContext('2d') !== null;
    }, { timeout: 10000 });
    console.log('✅ Trigger node should be created');

    // Verify canvas has content
    const hasCanvasContent = await page.evaluate(() => {
      const canvas = document.querySelector('#node-canvas') as HTMLCanvasElement;
      if (canvas) {
        const ctx = canvas.getContext('2d');
        if (ctx) {
          const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
          for (let i = 3; i < imageData.data.length; i += 4) {
            if (imageData.data[i] > 0) return true;
          }
        }
      }
      return false;
    });
    console.log(`📊 Canvas has content: ${hasCanvasContent}`);
    expect(hasCanvasContent).toBeTruthy();

    // Add an agent to the canvas by dragging from agent shelf
    const agentShelf = page.locator('#agent-shelf');
    await expect(agentShelf).toBeVisible({ timeout: 5000 });
    
    const agentPill = page.locator('#agent-shelf .agent-pill').first();
    await expect(agentPill).toBeVisible({ timeout: 5000 });
    console.log('✅ Agent shelf and agent pill visible');

    // Drag agent to canvas
    const canvas = page.locator('#node-canvas');
    await agentPill.dragTo(canvas, { targetPosition: { x: 400, y: 300 } });
    console.log('✅ Dragged agent to canvas');

    // Wait a moment for the drag to be processed
    await page.waitForTimeout(1000);

    // Try to connect the nodes by clicking on connection handles
    // This is tricky with canvas - let's try clicking on the canvas where nodes should be
    
    // First, let's see if we can trigger a connection by clicking on node areas
    await canvas.click({ position: { x: 410, y: 177 } }); // Trigger node position from logs
    await page.waitForTimeout(500);
    await canvas.click({ position: { x: 400, y: 300 } }); // Agent node position
    console.log('✅ Attempted to connect nodes');

    // Wait for any connection to be established
    await page.waitForTimeout(2000);

    // Look for the run button
    const runButton = page.locator('#run-workflow-btn');
    await expect(runButton).toBeVisible({ timeout: 5000 });
    console.log('✅ Run button found');

    // Check initial button state
    const initialButtonText = await runButton.textContent();
    console.log(`🔘 Initial button text: "${initialButtonText}"`);

    // Click the run button
    await runButton.click();
    console.log('🎯 Clicked run button');

    // Wait for button to show "Starting..." state
    await page.waitForFunction(() => {
      const btn = document.querySelector('#run-workflow-btn');
      return btn && btn.textContent?.includes('Starting');
    }, { timeout: 5000 });
    
    const startingText = await runButton.textContent();
    console.log(`🔄 Button shows: "${startingText}"`);

    // Monitor WebSocket messages and button state
    let executionFinished = false;
    const messagePromise = page.waitForEvent('websocket', { timeout: 30000 });
    
    // Wait for execution to complete (button should change from "Starting...")
    try {
      await page.waitForFunction(() => {
        const btn = document.querySelector('#run-workflow-btn');
        const text = btn?.textContent || '';
        return !text.includes('Starting') && !text.includes('Running');
      }, { timeout: 30000 });

      const finalText = await runButton.textContent();
      console.log(`✅ Execution completed - final button text: "${finalText}"`);
      
      // Verify button is no longer in starting/running state
      expect(finalText).not.toContain('Starting');
      expect(finalText).not.toContain('Running');
      
      console.log('🎉 Workflow execution completed successfully!');
      
    } catch (error) {
      const stuckText = await runButton.textContent();
      console.log(`❌ Button stuck at: "${stuckText}"`);
      
      // Take screenshot for debugging
      await page.screenshot({ path: 'workflow-execution-stuck.png', fullPage: true });
      
      throw new Error(`Workflow execution failed - button stuck at: "${stuckText}"`);
    }
  });
});