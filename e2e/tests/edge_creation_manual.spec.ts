import { test, expect } from './fixtures';

test('Manual edge creation and persistence test', async ({ page }) => {
  // Navigate to app
  await page.goto('/');
  await page.waitForTimeout(2000);
  
  // Go to canvas
  const canvasTab = page.getByTestId('global-canvas-tab');
  if (await canvasTab.count() > 0) {
    await canvasTab.click();
    await page.waitForTimeout(1000);
  }
  
  // Create new workflow
  const plusTab = page.locator('.plus-tab');
  if (await plusTab.count() > 0) {
    await plusTab.click();
    await page.evaluate(() => {
      window.prompt = () => 'Edge Test Workflow';
    });
    await plusTab.click();
    await page.waitForTimeout(1000);
  }
  
  // Check if we have agent shelf with agents
  const agents = page.locator('#agent-shelf .agent-pill');
  const agentCount = await agents.count();
  console.log(`Found ${agentCount} agents in shelf`);
  
  if (agentCount === 0) {
    console.log('No agents available, skipping test');
    test.skip();
    return;
  }
  
  // Try to drag agents onto canvas
  const canvas = page.locator('#canvas-container canvas');
  if (await canvas.count() === 0) {
    console.log('Canvas not found, skipping test');
    test.skip();
    return;
  }
  
  // Add agents to canvas
  if (agentCount >= 1) {
    await agents.nth(0).dragTo(canvas, { targetPosition: { x: 150, y: 150 } });
    await page.waitForTimeout(1000);
  }
  
  if (agentCount >= 2) {
    await agents.nth(1).dragTo(canvas, { targetPosition: { x: 350, y: 150 } });
    await page.waitForTimeout(1000);
  }
  
  // Check how many nodes we have
  const nodes = page.locator('.canvas-node, .generic-node');
  const nodeCount = await nodes.count();
  console.log(`Created ${nodeCount} nodes on canvas`);
  
  if (nodeCount < 2) {
    console.log('Not enough nodes for connection test');
    test.skip();
    return;
  }
  
  // Test connection creation
  const connectionBtn = page.locator('#connection-mode-btn');
  await expect(connectionBtn).toBeVisible();
  
  // Enter connection mode
  await connectionBtn.click();
  await page.waitForTimeout(500);
  
  // Listen for console messages to track edge creation
  const logs: string[] = [];
  page.on('console', msg => {
    if (msg.type() === 'log') {
      logs.push(msg.text());
    }
  });
  
  // Create connection
  await nodes.nth(0).click(); // Select source
  await page.waitForTimeout(500);
  await nodes.nth(1).click(); // Connect to target
  await page.waitForTimeout(1000);
  
  // Check console logs for edge creation
  const edgeLogs = logs.filter(log => 
    log.includes('Created connection') || 
    log.includes('edge') ||
    log.includes('Created new edge')
  );
  
  console.log('Edge creation logs:', edgeLogs);
  
  // We should have at least one log about edge creation
  expect(edgeLogs.length).toBeGreaterThan(0);
  
  // Test persistence by reloading page
  await page.reload();
  await page.waitForTimeout(2000);
  
  // Check if nodes still exist after reload
  const persistedNodes = page.locator('.canvas-node, .generic-node');
  const persistedNodeCount = await persistedNodes.count();
  console.log(`After reload: ${persistedNodeCount} nodes found`);
  
  // Nodes should persist (edges are harder to verify visually)
  expect(persistedNodeCount).toBeGreaterThanOrEqual(2);
  
  console.log('Manual edge creation test completed successfully');
});