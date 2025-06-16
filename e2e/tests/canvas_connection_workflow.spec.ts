import { test, expect, Page } from './fixtures';

async function switchToCanvas(page: Page) {
  await page.goto('/');
  const canvasTab = page.getByTestId('global-canvas-tab');
  if (await canvasTab.count()) {
    await canvasTab.click();
  }
  await page.waitForSelector('#canvas-container', { timeout: 10_000 });
}

async function createWorkflowWithNodes(page: Page, workflowName = 'Connection Test') {
  // Create new workflow via plus button
  const plusTab = page.locator('.plus-tab');
  await expect(plusTab).toBeVisible();
  await plusTab.click();
  
  // Fill in workflow name
  await page.waitForFunction(() => window.prompt !== undefined);
  await page.evaluate((name) => {
    // Mock the prompt to return our workflow name
    window.prompt = () => name;
  }, workflowName);
  
  await plusTab.click(); // Click again to trigger the prompt
  
  // Wait for workflow to be created
  await page.waitForSelector(`[data-id]:has-text("${workflowName}")`, { timeout: 5000 });
  
  // Add two nodes to connect
  const firstPill = page.locator('#agent-shelf .agent-pill').first();
  const secondPill = page.locator('#agent-shelf .agent-pill').nth(1);
  
  if (await firstPill.count() === 0) {
    throw new Error('No agents available for testing');
  }
  
  const canvasArea = page.locator('#canvas-container canvas');
  
  // Add first node
  await firstPill.dragTo(canvasArea, { targetPosition: { x: 100, y: 100 } });
  await expect(page.locator('.canvas-node, .generic-node')).toHaveCount(1, { timeout: 5000 });
  
  // Add second node if available
  if (await secondPill.count() > 0) {
    await secondPill.dragTo(canvasArea, { targetPosition: { x: 300, y: 100 } });
    await expect(page.locator('.canvas-node, .generic-node')).toHaveCount(2, { timeout: 5000 });
  }
  
  return workflowName;
}

test.describe('Canvas Connection Workflow Tests', () => {
  test.beforeEach(async ({ page }) => {
    await switchToCanvas(page);
  });

  test('Toggle connection mode button functionality', async ({ page }) => {
    // Check that connection mode button exists
    const connectionBtn = page.locator('#connection-mode-btn');
    await expect(connectionBtn).toBeVisible();
    
    // Verify initial state (not active)
    expect(await connectionBtn.getAttribute('class')).toBe('toolbar-btn');
    expect(await connectionBtn.getAttribute('title')).toBe('Toggle Connection Mode');
    
    // Click to activate connection mode
    await connectionBtn.click();
    
    // Verify active state
    expect(await connectionBtn.getAttribute('class')).toBe('toolbar-btn active');
    expect(await connectionBtn.getAttribute('title')).toContain('Exit Connection Mode');
    
    // Click again to deactivate
    await connectionBtn.click();
    
    // Verify back to inactive state
    expect(await connectionBtn.getAttribute('class')).toBe('toolbar-btn');
    expect(await connectionBtn.getAttribute('title')).toBe('Toggle Connection Mode');
  });

  test('Create connection between two nodes', async ({ page }) => {
    const workflowName = await createWorkflowWithNodes(page, 'Connection Creation Test');
    
    // Get the two nodes
    const nodes = page.locator('.canvas-node, .generic-node');
    const nodeCount = await nodes.count();
    
    if (nodeCount < 2) {
      test.skip(true, 'Need at least two nodes for connection test');
      return;
    }
    
    const firstNode = nodes.nth(0);
    const secondNode = nodes.nth(1);
    
    // Activate connection mode
    const connectionBtn = page.locator('#connection-mode-btn');
    await connectionBtn.click();
    
    // Verify connection mode is active
    expect(await connectionBtn.getAttribute('class')).toBe('toolbar-btn active');
    
    // Click first node to select as source
    await firstNode.click();
    
    // Wait for visual feedback (node should have selection border)
    await page.waitForTimeout(500); // Allow for visual update
    
    // Click second node to create connection
    await secondNode.click();
    
    // Verify connection was created (should see an edge/path element)
    await expect(page.locator('.canvas-edge, path.edge, svg path')).toHaveCount.toBeGreaterThan(0, { timeout: 5000 });
    
    // Verify connection mode automatically exits after connection creation
    expect(await connectionBtn.getAttribute('class')).toBe('toolbar-btn');
  });

  test('Connection mode visual feedback', async ({ page }) => {
    const workflowName = await createWorkflowWithNodes(page, 'Visual Feedback Test');
    
    const nodes = page.locator('.canvas-node, .generic-node');
    const nodeCount = await nodes.count();
    
    if (nodeCount < 1) {
      test.skip(true, 'Need at least one node for visual feedback test');
      return;
    }
    
    const firstNode = nodes.nth(0);
    
    // Activate connection mode
    const connectionBtn = page.locator('#connection-mode-btn');
    await connectionBtn.click();
    
    // Click node to select it
    await firstNode.click();
    
    // Check that node appears selected (by examining computed styles or canvas rendering)
    // Since we can't easily check canvas rendering in e2e tests, we'll verify the state through console logs
    const logs: string[] = [];
    page.on('console', msg => {
      if (msg.type() === 'log') {
        logs.push(msg.text());
      }
    });
    
    // Wait for selection to be processed
    await page.waitForTimeout(500);
    
    // Verify console logs show node selection
    const selectionLogs = logs.filter(log => log.includes('Selected node') && log.includes('as connection source'));
    expect(selectionLogs.length).toBeGreaterThan(0);
  });

  test('Prevent self-connection', async ({ page }) => {
    const workflowName = await createWorkflowWithNodes(page, 'Self Connection Test');
    
    const nodes = page.locator('.canvas-node, .generic-node');
    const nodeCount = await nodes.count();
    
    if (nodeCount < 1) {
      test.skip(true, 'Need at least one node for self-connection prevention test');
      return;
    }
    
    const firstNode = nodes.nth(0);
    
    // Activate connection mode
    const connectionBtn = page.locator('#connection-mode-btn');
    await connectionBtn.click();
    
    // Click same node twice (select as source, then try to connect to itself)
    await firstNode.click();
    await firstNode.click();
    
    // Listen for console messages
    const logs: string[] = [];
    page.on('console', msg => {
      if (msg.type() === 'log') {
        logs.push(msg.text());
      }
    });
    
    await page.waitForTimeout(500);
    
    // Verify error message about self-connection
    const errorLogs = logs.filter(log => log.includes('Cannot connect node to itself'));
    expect(errorLogs.length).toBeGreaterThan(0);
    
    // Verify no edge was created
    const edgeCount = await page.locator('.canvas-edge, path.edge, svg path').count();
    expect(edgeCount).toBe(0);
  });

  test('Clear selection functionality', async ({ page }) => {
    const workflowName = await createWorkflowWithNodes(page, 'Clear Selection Test');
    
    const nodes = page.locator('.canvas-node, .generic-node');
    const nodeCount = await nodes.count();
    
    if (nodeCount < 1) {
      test.skip(true, 'Need at least one node for clear selection test');
      return;
    }
    
    const firstNode = nodes.nth(0);
    
    // Activate connection mode
    const connectionBtn = page.locator('#connection-mode-btn');
    await connectionBtn.click();
    
    // Select a node
    await firstNode.click();
    
    // Exit connection mode (should clear selection)
    await connectionBtn.click();
    
    // Re-enter connection mode 
    await connectionBtn.click();
    
    // Verify no node is pre-selected (by checking console logs)
    const logs: string[] = [];
    page.on('console', msg => {
      if (msg.type() === 'log') {
        logs.push(msg.text());
      }
    });
    
    // Try to create connection without selecting source first
    if (nodeCount >= 2) {
      const secondNode = nodes.nth(1);
      await secondNode.click();
      
      await page.waitForTimeout(500);
      
      // Should select this node as source (not create connection)
      const selectionLogs = logs.filter(log => log.includes('Selected node') && log.includes('as connection source'));
      expect(selectionLogs.length).toBeGreaterThan(0);
    }
  });

  test('Connection persistence after page reload', async ({ page }) => {
    const workflowName = await createWorkflowWithNodes(page, 'Persistence Test');
    
    const nodes = page.locator('.canvas-node, .generic-node');
    const nodeCount = await nodes.count();
    
    if (nodeCount < 2) {
      test.skip(true, 'Need at least two nodes for persistence test');
      return;
    }
    
    // Create connection
    const connectionBtn = page.locator('#connection-mode-btn');
    await connectionBtn.click();
    
    const firstNode = nodes.nth(0);
    const secondNode = nodes.nth(1);
    
    await firstNode.click();
    await secondNode.click();
    
    // Verify connection exists
    await expect(page.locator('.canvas-edge, path.edge, svg path')).toHaveCount.toBeGreaterThan(0, { timeout: 5000 });
    
    // Reload page
    await page.reload();
    await page.waitForSelector('#canvas-container', { timeout: 10_000 });
    
    // Verify connection persists
    await expect(page.locator('.canvas-edge, path.edge, svg path')).toHaveCount.toBeGreaterThan(0, { timeout: 5000 });
    
    // Verify nodes still exist
    await expect(page.locator('.canvas-node, .generic-node')).toHaveCount(2, { timeout: 5000 });
  });

  test('Multiple connections creation', async ({ page }) => {
    // Create workflow with at least 3 nodes if possible
    const workflowName = await createWorkflowWithNodes(page, 'Multiple Connections Test');
    
    // Add a third node if we have enough agents
    const agents = page.locator('#agent-shelf .agent-pill');
    const agentCount = await agents.count();
    
    if (agentCount >= 3) {
      const thirdPill = agents.nth(2);
      const canvasArea = page.locator('#canvas-container canvas');
      await thirdPill.dragTo(canvasArea, { targetPosition: { x: 500, y: 100 } });
      await expect(page.locator('.canvas-node, .generic-node')).toHaveCount(3, { timeout: 5000 });
    }
    
    const nodes = page.locator('.canvas-node, .generic-node');
    const nodeCount = await nodes.count();
    
    if (nodeCount < 2) {
      test.skip(true, 'Need at least two nodes for multiple connections test');
      return;
    }
    
    const connectionBtn = page.locator('#connection-mode-btn');
    
    // Create first connection: node 0 -> node 1
    await connectionBtn.click();
    await nodes.nth(0).click();
    await nodes.nth(1).click();
    
    // Verify first connection
    await expect(page.locator('.canvas-edge, path.edge, svg path')).toHaveCount.toBeGreaterThan(0, { timeout: 5000 });
    
    if (nodeCount >= 3) {
      // Create second connection: node 1 -> node 2
      await connectionBtn.click();
      await nodes.nth(1).click();
      await nodes.nth(2).click();
      
      // Verify we now have multiple connections
      await expect(page.locator('.canvas-edge, path.edge, svg path')).toHaveCount.toBeGreaterThan(1, { timeout: 5000 });
    }
  });
});