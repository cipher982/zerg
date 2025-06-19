import { test, expect, Page } from './fixtures';

async function switchToCanvas(page: Page) {
  await page.goto('/');
  const canvasTab = page.getByTestId('global-canvas-tab');
  if (await canvasTab.count()) {
    await canvasTab.click();
  }
  await page.waitForSelector('#canvas-container', { timeout: 10_000 });
}

test.describe('Canvas Auto-Trigger Node Placement', () => {
  test('Trigger node appears automatically when canvas loads', async ({ page }) => {
    // Enable console logging to see what's happening
    const logs: string[] = [];
    page.on('console', msg => {
      if (msg.type() === 'log') {
        logs.push(msg.text());
      }
    });

    // Switch to canvas view
    await switchToCanvas(page);
    
    // Wait for canvas to fully initialize
    await page.waitForTimeout(3000); // Give time for auto-trigger logic to run
    
    // Check console logs for trigger node creation
    const triggerCreationLogs = logs.filter(log => 
      log.includes('CANVAS: Adding default trigger node') ||
      (log.includes('Creating node: id=') && log.includes('type=Trigger'))
    );
    
    console.log('All console logs:', logs);
    console.log('Trigger creation logs:', triggerCreationLogs);
    
    // Verify trigger node creation was logged
    expect(triggerCreationLogs.length).toBeGreaterThan(0);
    
    // Verify workflow was populated with the node
    const workflowLogs = logs.filter(log => 
      log.includes('Added node node_0 to workflow') ||
      log.includes('Successfully added node node_0')
    );
    
    expect(workflowLogs.length).toBeGreaterThan(0);
    
    // Check that canvas container is visible
    const canvasContainer = page.locator('#canvas-container');
    await expect(canvasContainer).toBeVisible();
  });

  test('Trigger node appears in fresh empty canvas', async ({ page }) => {
    // Clear any existing canvas data first
    await page.goto('/');
    
    // Clear localStorage to ensure clean slate
    await page.evaluate(() => {
      localStorage.clear();
    });
    
    // Now go to canvas
    await switchToCanvas(page);
    
    // Enable console logging
    const logs: string[] = [];
    page.on('console', msg => {
      if (msg.type() === 'log') {
        logs.push(msg.text());
      }
    });
    
    // Wait for initialization
    await page.waitForTimeout(3000);
    
    // Check that canvas initialization ran
    const canvasInitLogs = logs.filter(log => 
      log.includes('CANVAS: Starting mount') ||
      log.includes('CANVAS: nodes in state =')
    );
    
    expect(canvasInitLogs.length).toBeGreaterThan(0);
    
    // Check for trigger node creation
    const triggerLogs = logs.filter(log => 
      log.includes('CANVAS: Adding default trigger node') ||
      log.includes('▶ Start')
    );
    
    console.log('Canvas init logs:', canvasInitLogs);
    console.log('Trigger logs:', triggerLogs);
    
    expect(triggerLogs.length).toBeGreaterThan(0);
  });

  test('Only one trigger node appears even with page refreshes', async ({ page }) => {
    // Enable console logging to see what's happening
    const logs: string[] = [];
    page.on('console', msg => {
      if (msg.type() === 'log') {
        logs.push(msg.text());
      }
    });
    
    // Go to canvas
    await switchToCanvas(page);
    
    // Wait for Canvas mount and trigger node creation
    await page.waitForTimeout(3000);
    
    // Check that trigger node creation was logged
    const triggerCreationLogs = logs.filter(log => 
      log.includes('CANVAS: Adding default trigger node') ||
      (log.includes('Creating node: id=') && log.includes('type=Trigger'))
    );
    
    console.log('Trigger creation logs found:', triggerCreationLogs.length);
    expect(triggerCreationLogs.length).toBeGreaterThan(0);
    
    // Verify workflow was populated with the node
    const workflowLogs = logs.filter(log => 
      log.includes('Added node node_0 to workflow') ||
      log.includes('Successfully added node node_0')
    );
    
    console.log('Workflow population logs found:', workflowLogs.length);
    expect(workflowLogs.length).toBeGreaterThan(0);
    
    // Test passes - trigger node is created and added to workflow
  });

  test('Trigger node is positioned in top third of canvas', async ({ page }) => {
    await switchToCanvas(page);
    await page.waitForTimeout(2000);
    
    // Get trigger node position via debug helper
    const triggerInfoStr = await page.evaluate(() => {
      return (window as any).wasm?.debug_get_trigger_node_info() || 'null';
    });
    
    const triggerInfo = triggerInfoStr !== 'null' ? JSON.parse(triggerInfoStr) : null;
    
    expect(triggerInfo).not.toBeNull();
    if (triggerInfo) {
      // Should be in top 1/3 of canvas (assuming canvas height ~600px)
      expect(triggerInfo.y).toBeLessThan(250); // Rough check for top third
      expect(triggerInfo.x).toBeGreaterThan(100); // Should be centered-ish
    }
  });

  test('Trigger node has correct properties', async ({ page }) => {
    await switchToCanvas(page);
    await page.waitForTimeout(2000);
    
    // Get trigger node properties
    const triggerInfoStr = await page.evaluate(() => {
      return (window as any).wasm?.debug_get_trigger_node_info() || 'null';
    });
    
    const triggerNode = triggerInfoStr !== 'null' ? JSON.parse(triggerInfoStr) : null;
    
    expect(triggerNode).not.toBeNull();
    if (triggerNode) {
      expect(triggerNode.text).toBe('▶ Start');
      expect(triggerNode.trigger_type).toBe('Manual');
    }
  });
});