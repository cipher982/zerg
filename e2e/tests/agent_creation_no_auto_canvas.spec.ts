/**
 * E2E Test: Agent Creation Should Not Auto-Populate Canvas
 * 
 * This test verifies that agents created in the dashboard do NOT automatically
 * appear as nodes on the canvas when switching to canvas view. Agents should
 * only appear in the shelf and require manual dragging to the canvas.
 */

import { test, expect } from './fixtures';
import { ApiClient } from './helpers/api-client';

test.describe('Agent Creation Canvas Bug', () => {
  let apiClient: ApiClient;

  test.beforeEach(async ({ page }) => {
    apiClient = new ApiClient();
    
    // Clean slate - delete all existing agents
    const agents = await apiClient.listAgents();
    for (const agent of agents) {
      if (agent.id) {
        await apiClient.deleteAgent(agent.id);
      }
    }

    // Navigate to the application
    await page.goto('/');
    await page.waitForSelector('table'); // Wait for dashboard to load
  });

  test('CRITICAL BUG: Agents created in dashboard should NOT auto-appear as canvas nodes', async ({ page }) => {
    console.log('🧪 Starting agent creation canvas bug test...');

    // Step 1: Create an agent in dashboard
    console.log('📋 Step 1: Creating agent in dashboard...');
    await page.locator('[data-testid="create-agent-btn"]').click();
    await page.waitForTimeout(3000); // Wait for agent creation

    // Step 2: Verify agent exists in dashboard
    console.log('✅ Step 2: Verifying agent exists in dashboard...');
    const agentRows = await page.locator('tr[data-agent-id]').count();
    expect(agentRows).toBeGreaterThan(0);
    console.log(`Found ${agentRows} agent(s) in dashboard`);

    // Step 3: Switch to canvas view
    console.log('🎨 Step 3: Switching to canvas view...');
    await page.locator('[data-testid="global-canvas-tab"]').click();
    await page.waitForTimeout(3000); // Wait for view switch

    // Step 4: Check that NO nodes exist on canvas
    console.log('🚫 Step 4: Verifying NO nodes on canvas...');
    const canvasNodes = await page.locator('#node-canvas .node').count();
    console.log(`Found ${canvasNodes} node(s) on canvas`);
    
    // THE CRITICAL TEST: Canvas should be empty
    expect(canvasNodes).toBe(0);

    // Step 5: Verify agent appears in shelf (for dragging)
    console.log('📦 Step 5: Verifying agent appears in shelf...');
    const shelfAgents = await page.locator('#agent-shelf .agent-pill').count();
    console.log(`Found ${shelfAgents} agent(s) in shelf`);
    expect(shelfAgents).toBeGreaterThan(0);

    console.log('✅ Test PASSED: Agent creation does not auto-populate canvas');
  });

  test('Multiple agents should not auto-populate canvas', async ({ page }) => {
    console.log('🧪 Testing multiple agent creation...');

    // Create 3 agents
    for (let i = 1; i <= 3; i++) {
      console.log(`📋 Creating agent ${i}...`);
      await page.locator('[data-testid="create-agent-btn"]').click();
      await page.waitForTimeout(2000);
    }

    // Verify 3 agents in dashboard
    const agentRows = await page.locator('tr[data-agent-id]').count();
    expect(agentRows).toBe(3);
    console.log(`✅ Created ${agentRows} agents in dashboard`);

    // Switch to canvas
    console.log('🎨 Switching to canvas...');
    await page.locator('[data-testid="global-canvas-tab"]').click();
    await page.waitForTimeout(3000);

    // Canvas should still be empty
    const canvasNodes = await page.locator('#node-canvas .node').count();
    console.log(`Canvas has ${canvasNodes} nodes`);
    expect(canvasNodes).toBe(0);

    // All 3 should be in shelf
    const shelfAgents = await page.locator('#agent-shelf .agent-pill').count();
    console.log(`Shelf has ${shelfAgents} agents`);
    expect(shelfAgents).toBe(3);

    console.log('✅ Multiple agent test PASSED');
  });

  test('Canvas remains empty after multiple view switches', async ({ page }) => {
    console.log('🧪 Testing view switching persistence...');

    // Create agent in dashboard
    await page.locator('[data-testid="create-agent-btn"]').click();
    await page.waitForTimeout(3000);

    // Switch between views multiple times
    for (let i = 1; i <= 3; i++) {
      console.log(`🔀 View switch cycle ${i}...`);
      
      // Go to canvas
      await page.locator('[data-testid="global-canvas-tab"]').click();
      await page.waitForTimeout(2000);
      
      // Verify still empty
      const canvasNodes = await page.locator('#node-canvas .node').count();
      expect(canvasNodes).toBe(0);
      
      // Back to dashboard
      await page.locator('[data-testid="global-dashboard-tab"]').click();
      await page.waitForTimeout(2000);
      
      // Verify agent still there
      const agentRows = await page.locator('tr[data-agent-id]').count();
      expect(agentRows).toBeGreaterThan(0);
    }

    console.log('✅ View switching test PASSED');
  });

  test('Agent drag from shelf to canvas works correctly', async ({ page }) => {
    console.log('🧪 Testing manual drag from shelf to canvas...');

    // Create agent
    await page.locator('[data-testid="create-agent-btn"]').click();
    await page.waitForTimeout(3000);

    // Go to canvas
    await page.locator('[data-testid="global-canvas-tab"]').click();
    await page.waitForTimeout(3000);

    // Verify agent in shelf
    const shelfPill = page.locator('#agent-shelf .agent-pill').first();
    await expect(shelfPill).toBeVisible();

    // Try to drag agent to canvas
    const canvasArea = page.locator('#node-canvas');
    await shelfPill.dragTo(canvasArea, {
      targetPosition: { x: 200, y: 200 }
    });
    await page.waitForTimeout(1000);

    // NOW there should be a canvas node
    const canvasNodes = await page.locator('#node-canvas .node').count();
    console.log(`After drag: ${canvasNodes} node(s) on canvas`);
    expect(canvasNodes).toBe(1);

    console.log('✅ Manual drag test PASSED');
  });
});