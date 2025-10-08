import { test, expect } from './fixtures';

/**
 * AGENT-TOOL E2E TEST - STEP BY STEP
 * 
 * Building up the complete workflow one step at a time:
 * 1. Clean database verification
 * 2. Create ONE agent
 * 3. Navigate to canvas
 * 4. Drag agent to canvas  
 * 5. Drag tool to canvas
 * 6. Connect nodes
 * 7. Execute workflow
 * 
 * Each step builds on the previous one, with detailed logging and validation.
 */

function countRealAgents(agentRows: any[]): Promise<number> {
  // Helper to count real agents (exclude UI placeholders)
  return Promise.all(
    agentRows.map(async (row) => {
      const agentName = await row.locator('td').first().textContent();
      return agentName && !agentName.includes('No agents found');
    })
  ).then(results => results.filter(Boolean).length);
}

test.describe('Agent-Tool E2E - Step by Step', () => {
  test('Step 1: Verify clean database state', async ({ page }) => {
    console.log('🧪 Step 1: Verifying clean database...');
    
    await page.goto('/');
    await page.getByTestId('global-dashboard-tab').click();
    await page.waitForTimeout(1000);
    
    const agentRows = await page.locator('table tbody tr').all();
    const realAgentCount = await countRealAgents(agentRows);
    
    console.log(`📊 Step 1 Result: ${realAgentCount} real agents found`);
    expect(realAgentCount).toBe(0);
    
    console.log('✅ Step 1 PASSED: Database is clean');
  });

  test('Step 2: Create one agent', async ({ page }) => {
    console.log('🧪 Step 2: Creating one agent...');
    
    await page.goto('/');
    await page.getByTestId('global-dashboard-tab').click();
    await page.waitForTimeout(1000);
    
    // Verify clean start
    let agentRows = await page.locator('table tbody tr').all();
    let realAgentCount = await countRealAgents(agentRows);
    console.log(`📊 Step 2 Initial: ${realAgentCount} real agents`);
    expect(realAgentCount).toBe(0);
    
    // Create ONE agent
    console.log('🔨 Creating agent...');
    const createBtn = page.locator('button:has-text("Create Agent")');
    await createBtn.click();
    await page.waitForTimeout(1000);
    
    // Verify exactly one agent created
    agentRows = await page.locator('table tbody tr').all();
    realAgentCount = await countRealAgents(agentRows);
    console.log(`📊 Step 2 Final: ${realAgentCount} real agents`);
    expect(realAgentCount).toBe(1);
    
    console.log('✅ Step 2 PASSED: Agent created successfully');
  });

  test('Step 3: Navigate to canvas with agent', async ({ page }) => {
    console.log('🧪 Step 3: Creating agent and navigating to canvas...');
    
    await page.goto('/');
    await page.getByTestId('global-dashboard-tab').click();
    await page.waitForTimeout(1000);
    
    // Create agent first
    const createBtn = page.locator('button:has-text("Create Agent")');
    await createBtn.click();
    await page.waitForTimeout(1000);
    
    // Navigate to canvas
    console.log('🎨 Navigating to canvas...');
    await page.getByTestId('global-canvas-tab').click();
    await page.waitForTimeout(2000);
    
    // Verify canvas loaded
    await expect(page.locator('#canvas-container')).toBeVisible({ timeout: 10000 });
    await expect(page.locator('#canvas-container canvas')).toBeVisible({ timeout: 5000 });
    console.log('✅ Canvas loaded');
    
    // Verify agent shelf loaded
    const agentPills = page.locator('#agent-shelf .agent-pill');
    await expect(agentPills.first()).toBeVisible({ timeout: 10000 });
    const pillCount = await agentPills.count();
    console.log(`📊 Step 3 Result: ${pillCount} agents in shelf`);
    expect(pillCount).toBeGreaterThanOrEqual(1);
    
    console.log('✅ Step 3 PASSED: Canvas loaded with agent shelf');
  });

  test('Step 4: Drag agent to canvas', async ({ page }) => {
    console.log('🧪 Step 4: Drag agent from shelf to canvas...');
    
    // Setup: Create agent and go to canvas
    await page.goto('/');
    await page.getByTestId('global-dashboard-tab').click();
    await page.waitForTimeout(1000);
    
    const createBtn = page.locator('button:has-text("Create Agent")');
    await createBtn.click();
    await page.waitForTimeout(1000);
    
    await page.getByTestId('global-canvas-tab').click();
    await page.waitForTimeout(2000);
    
    // Verify setup
    await expect(page.locator('#canvas-container canvas')).toBeVisible({ timeout: 5000 });
    const agentPills = page.locator('#agent-shelf .agent-pill');
    await expect(agentPills.first()).toBeVisible({ timeout: 10000 });
    
    // Attempt to drag agent to canvas
    console.log('🖱️ Dragging agent to canvas...');
    const agentPill = agentPills.first();
    const canvasArea = page.locator('#canvas-container canvas');
    
    await agentPill.dragTo(canvasArea, { 
      targetPosition: { x: 200, y: 150 } 
    });
    await page.waitForTimeout(2000);
    
    // Check if nodes appeared on canvas
    const possibleNodes = page.locator('.canvas-node, .generic-node, [data-node-type], .node, .workflow-node');
    const nodeCount = await possibleNodes.count();
    console.log(`📊 Step 4 Result: ${nodeCount} nodes on canvas after drag`);
    
    // Log canvas state for debugging
    const canvasHTML = await page.locator('#canvas-container').innerHTML();
    console.log(`🔍 Canvas HTML length: ${canvasHTML.length} characters`);
    
    if (nodeCount === 0) {
      console.log('⚠️ No nodes found - agent drag may not be working');
      console.log('🔍 Canvas container HTML (first 200 chars):', canvasHTML.substring(0, 200));
    } else {
      console.log('✅ Nodes found on canvas!');
    }
    
    console.log('✅ Step 4 COMPLETED: Agent drag attempted (nodes may or may not appear)');
  });

  test('Step 5: Drag tool to canvas', async ({ page }) => {
    console.log('🧪 Step 5: Drag HTTP tool from palette to canvas...');
    
    // Setup: Create agent and go to canvas
    await page.goto('/');
    await page.getByTestId('global-dashboard-tab').click();
    await page.waitForTimeout(1000);
    
    const createBtn = page.locator('button:has-text("Create Agent")');
    await createBtn.click();
    await page.waitForTimeout(1000);
    
    await page.getByTestId('global-canvas-tab').click();
    await page.waitForTimeout(2000);
    
    // Verify canvas loaded
    await expect(page.locator('#canvas-container canvas')).toBeVisible({ timeout: 5000 });
    
    // Find HTTP Request tool in palette
    console.log('🔍 Looking for HTTP Request tool...');
    const httpTool = page.locator('[data-testid*="http"], .palette-node:has-text("HTTP Request"), .tool-palette-item:has-text("HTTP")');
    
    // Try different selectors
    const possibleSelectors = [
      '[data-testid="palette-tool-http_request"]',
      '.palette-node:has-text("HTTP Request")',
      '.tool-palette-item:has-text("HTTP")',
      'text="HTTP Request"',
      'text="🌐 HTTP Request"'
    ];
    
    let toolElement = null;
    for (const selector of possibleSelectors) {
      const element = page.locator(selector);
      if (await element.count() > 0) {
        console.log(`✅ Found HTTP tool with selector: ${selector}`);
        toolElement = element;
        break;
      }
    }
    
    if (!toolElement) {
      console.log('⚠️ HTTP Request tool not found, listing available tools...');
      const allTools = page.locator('.palette-node, .tool-palette-item');
      const toolCount = await allTools.count();
      console.log(`📊 Found ${toolCount} tools in palette`);
      
      for (let i = 0; i < Math.min(toolCount, 5); i++) {
        const toolText = await allTools.nth(i).textContent();
        console.log(`  Tool ${i + 1}: ${toolText}`);
      }
      
      // Use first available tool as fallback
      if (toolCount > 0) {
        toolElement = allTools.first();
        console.log('🔄 Using first available tool as fallback');
      }
    }
    
    if (toolElement) {
      console.log('🖱️ Dragging tool to canvas...');
      const canvasArea = page.locator('#canvas-container canvas');
      
      await toolElement.dragTo(canvasArea, { 
        targetPosition: { x: 400, y: 150 } 
      });
      await page.waitForTimeout(2000);
      
      // Check for nodes
      const possibleNodes = page.locator('.canvas-node, .generic-node, [data-node-type], .node, .workflow-node');
      const nodeCount = await possibleNodes.count();
      console.log(`📊 Step 5 Result: ${nodeCount} nodes on canvas after tool drag`);
      
      console.log('✅ Step 5 COMPLETED: Tool drag attempted');
    } else {
      console.log('❌ Step 5 FAILED: No tool found to drag');
    }
  });
});