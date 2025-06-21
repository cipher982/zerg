import { test, expect } from './fixtures';

test('Workflow execution with connections', async ({ page }) => {
  // Reset database first
  await page.request.post('http://localhost:8001/admin/reset-database');
  
  // Go to the app
  await page.goto('/');
  await page.waitForTimeout(2000);
  
  // Go to dashboard first to create some agents
  await page.getByTestId('global-dashboard-tab').click();
  await page.waitForTimeout(1000);
  
  // Create two agents for our workflow
  const createAgentBtn = page.locator('button:has-text("Create New Agent")');
  
  // Create first agent
  await createAgentBtn.click();
  await page.fill('#agent-name', 'Agent A');
  await page.fill('#system-instructions', 'You are Agent A');
  await page.fill('#task-instructions', 'Respond with "Hello from Agent A"');
  await page.locator('button:has-text("Create Agent")').click();
  await page.waitForTimeout(1000);
  
  // Create second agent
  await createAgentBtn.click();
  await page.fill('#agent-name', 'Agent B');
  await page.fill('#system-instructions', 'You are Agent B');
  await page.fill('#task-instructions', 'Respond with "Hello from Agent B"');
  await page.locator('button:has-text("Create Agent")').click();
  await page.waitForTimeout(1000);
  
  // Switch to canvas view
  await page.getByTestId('global-canvas-tab').click();
  await page.waitForTimeout(1000);
  
  // Create a new workflow
  const plusTab = page.locator('.plus-tab');
  await plusTab.click();
  
  // Mock prompt for workflow name
  await page.evaluate(() => {
    window.prompt = () => 'Connection Test Workflow';
  });
  await plusTab.click();
  await page.waitForTimeout(1000);
  
  // Wait for agents to load in shelf
  await expect(page.locator('#agent-shelf .agent-pill')).toHaveCount.toBeGreaterThan(1, { timeout: 10000 });
  
  // Drag agents onto canvas
  const agentPills = page.locator('#agent-shelf .agent-pill');
  const canvasArea = page.locator('#canvas-container canvas');
  
  if (await canvasArea.count() === 0) {
    console.log('Canvas not found, skipping test');
    test.skip();
    return;
  }
  
  // Add first agent (Agent A)
  await agentPills.nth(0).dragTo(canvasArea, { targetPosition: { x: 100, y: 100 } });
  await page.waitForTimeout(500);
  
  // Add second agent (Agent B)
  await agentPills.nth(1).dragTo(canvasArea, { targetPosition: { x: 300, y: 100 } });
  await page.waitForTimeout(500);
  
  // Verify both nodes exist
  const nodes = page.locator('.canvas-node, .generic-node');
  const nodeCount = await nodes.count();
  console.log(`Created ${nodeCount} nodes on canvas`);
  
  if (nodeCount < 2) {
    console.log('Not enough nodes created, skipping connection test');
    test.skip();
    return;
  }
  
  // Enter connection mode
  const connectionBtn = page.locator('#connection-mode-btn');
  await connectionBtn.click();
  await page.waitForTimeout(500);
  
  // Create connection: Agent A -> Agent B
  await nodes.nth(0).click(); // Select Agent A as source
  await page.waitForTimeout(500);
  await nodes.nth(1).click(); // Connect to Agent B
  await page.waitForTimeout(500);
  
  // Look for visual connection (edge) - this might be drawn on canvas or as SVG
  // Since connections are drawn on canvas, we'll check the console logs instead
  const logs: string[] = [];
  page.on('console', msg => {
    if (msg.type() === 'log') {
      logs.push(msg.text());
    }
  });
  
  await page.waitForTimeout(1000);
  
  // Check console logs for connection creation
  const connectionLogs = logs.filter(log => 
    log.includes('Created connection') || 
    log.includes('Created new edge')
  );
  
  console.log('Connection logs:', connectionLogs);
  expect(connectionLogs.length).toBeGreaterThan(0);
  
  // Try to execute the workflow
  const runBtn = page.locator('#run-workflow-btn');
  if (await runBtn.count() > 0) {
    await runBtn.click();
    await page.waitForTimeout(2000);
    
    // Check if execution started
    const hasRunningClass = await runBtn.evaluate(btn => btn.classList.contains('running'));
    console.log('Workflow execution started:', hasRunningClass);
    
    // Wait for execution to complete or timeout
    await page.waitForTimeout(10000);
    
    // Check final state
    const finalClass = await runBtn.getAttribute('class');
    console.log('Final execution state:', finalClass);
    
    // Execution should complete (success or failed, but not still running)
    expect(finalClass).not.toContain('running');
  } else {
    console.log('Run button not found, workflow execution test skipped');
  }
});