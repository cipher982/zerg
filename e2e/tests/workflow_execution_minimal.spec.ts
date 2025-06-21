import { test, expect, Page } from './fixtures';

test.describe('Minimal Workflow Execution Test', () => {
  test.beforeEach(async ({ page }) => {
    // Reset database before each test
    await page.request.post('http://localhost:8001/admin/reset-database');
  });

  test('Debug workflow execution with trigger node', async ({ page }) => {
    console.log('🚀 Starting workflow execution debug test');

    // Navigate to canvas view
    await page.goto('/');
    
    // Click canvas tab to ensure we're in canvas view
    const canvasTab = page.getByTestId('global-canvas-tab');
    if (await canvasTab.count() > 0) {
      await canvasTab.click();
      console.log('✅ Clicked canvas tab');
    }

    // Wait for canvas to load
    await page.waitForSelector('#canvas-container', { timeout: 10000 });
    console.log('✅ Canvas container loaded');

    // Wait for trigger node to auto-appear (should be auto-placed)
    const triggerNode = page.locator('.canvas-node').filter({ hasText: 'Start' });
    await expect(triggerNode).toBeVisible({ timeout: 10000 });
    console.log('✅ Trigger node is visible');

    // Verify we have at least one node on canvas
    const nodeCount = await page.locator('.canvas-node').count();
    console.log(`📊 Node count: ${nodeCount}`);
    expect(nodeCount).toBeGreaterThan(0);

    // Look for run button
    const runButton = page.locator('#run-workflow-btn');
    await expect(runButton).toBeVisible({ timeout: 5000 });
    console.log('✅ Run button is visible');

    // Check initial button state
    const initialButtonText = await runButton.textContent();
    console.log(`🔘 Initial button text: "${initialButtonText}"`);

    // Click run button
    console.log('🎯 Clicking run button...');
    await runButton.click();

    // Wait for button text to change to "Starting..."
    await page.waitForFunction(() => {
      const btn = document.querySelector('#run-workflow-btn');
      return btn && btn.textContent?.includes('Starting');
    }, { timeout: 5000 });
    
    const startingButtonText = await runButton.textContent();
    console.log(`🔄 Button text changed to: "${startingButtonText}"`);

    // Wait for WebSocket subscription message in console
    page.on('console', msg => {
      if (msg.text().includes('Sending subscribe request')) {
        console.log(`📡 WebSocket: ${msg.text()}`);
      }
      if (msg.text().includes('TopicManager routing message')) {
        console.log(`📨 Message routing: ${msg.text()}`);
      }
      if (msg.text().includes('No handlers registered')) {
        console.log(`❌ Handler issue: ${msg.text()}`);
      }
    });

    // Wait up to 30 seconds for execution to complete
    console.log('⏳ Waiting for execution to complete...');
    
    try {
      await page.waitForFunction(() => {
        const btn = document.querySelector('#run-workflow-btn');
        const text = btn?.textContent || '';
        return !text.includes('Starting') && !text.includes('Running');
      }, { timeout: 30000 });

      const finalButtonText = await runButton.textContent();
      console.log(`✅ Execution completed - final button text: "${finalButtonText}"`);
      
      // Verify button is no longer in starting state
      expect(finalButtonText).not.toContain('Starting');
      
    } catch (error) {
      const stuckButtonText = await runButton.textContent();
      console.log(`❌ Button stuck at: "${stuckButtonText}"`);
      
      // Get console logs for debugging
      const logs = await page.evaluate(() => {
        return (window as any).console?.logs || [];
      });
      console.log('Console logs:', logs);
      
      throw new Error(`Workflow execution stuck - button text: "${stuckButtonText}"`);
    }
  });

  test('Direct API workflow execution test', async ({ page }) => {
    console.log('🔧 Testing direct API workflow execution');

    // First create a workflow via API
    const createWorkflowResponse = await page.request.post('http://localhost:8001/api/workflows', {
      data: {
        name: 'API Test Workflow',
        canvas_data: {
          nodes: [
            {
              node_id: 'trigger_1',
              node_type: { 'Trigger': { trigger_type: 'Manual', config: {} } },
              text: 'Start',
              x: 100,
              y: 100,
              width: 200,
              height: 80
            }
          ],
          edges: []
        }
      }
    });

    expect(createWorkflowResponse.ok()).toBeTruthy();
    const workflow = await createWorkflowResponse.json();
    console.log(`✅ Created workflow ID: ${workflow.id}`);

    // Execute workflow directly via API
    const executeResponse = await page.request.post(`http://localhost:8001/api/workflow-executions/${workflow.id}/start`);
    
    if (!executeResponse.ok()) {
      const errorText = await executeResponse.text();
      console.log(`❌ API execution failed: ${errorText}`);
      throw new Error(`API execution failed: ${errorText}`);
    }

    const execution = await executeResponse.json();
    console.log(`✅ Started execution ID: ${execution.execution_id}`);

    // Wait for execution to complete
    let completed = false;
    let attempts = 0;
    const maxAttempts = 30;

    while (!completed && attempts < maxAttempts) {
      await page.waitForTimeout(1000);
      attempts++;
      
      const statusResponse = await page.request.get(`http://localhost:8001/api/workflow-executions/${execution.execution_id}/status`);
      if (statusResponse.ok()) {
        const status = await statusResponse.json();
        console.log(`📊 Attempt ${attempts}: Execution status = ${status.status}`);
        
        if (status.status === 'success' || status.status === 'failed') {
          completed = true;
          console.log(`✅ Execution completed with status: ${status.status}`);
          expect(['success', 'failed']).toContain(status.status);
        }
      } else {
        console.log(`📊 Attempt ${attempts}: Status endpoint returned ${statusResponse.status()}`);
      }
    }

    if (!completed) {
      throw new Error(`Execution did not complete after ${maxAttempts} seconds`);
    }
  });
});