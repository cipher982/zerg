/**
 * SIMPLE TEST: Agent Creation Canvas Bug Verification
 * 
 * This test reproduces the exact user scenario:
 * 1. Create agents in dashboard
 * 2. Navigate to canvas
 * 3. Check if agents auto-appear (BUG) or stay in shelf (CORRECT)
 */

import { test, expect } from './fixtures';

test('Simple Bug Test: Dashboard agents should NOT auto-appear on canvas', async ({ page }) => {
  console.log('🚨 TESTING REPORTED BUG: Dashboard agents auto-appearing on canvas');

  // Step 1: Go to dashboard and create an agent
  console.log('📋 Step 1: Creating agent in dashboard...');
  await page.goto('/');
  await page.waitForSelector('table');
  
  // Create agent
  await page.locator('[data-testid="create-agent-btn"]').click();
  await page.waitForTimeout(4000); // Give it time
  
  // Verify agent exists in dashboard  
  const dashboardAgents = await page.locator('tr[data-agent-id]').count();
  console.log(`Dashboard has ${dashboardAgents} agent(s)`);
  expect(dashboardAgents).toBeGreaterThan(0);

  // Step 2: Switch to canvas view
  console.log('🎨 Step 2: Switching to canvas view...');
  await page.locator('[data-testid="global-canvas-tab"]').click();
  await page.waitForTimeout(4000); // Give it time to load and process

  // Step 3: Count canvas nodes - THIS IS THE CRITICAL TEST
  console.log('🔍 Step 3: Checking canvas for auto-created nodes...');
  const canvasNodes = await page.locator('#node-canvas .node').count();
  console.log(`❓ Canvas has ${canvasNodes} node(s)`);

  // Step 4: Count shelf agents
  const shelfAgents = await page.locator('#agent-shelf .agent-pill').count();
  console.log(`📦 Shelf has ${shelfAgents} agent(s)`);

  // THE BUG CHECK: If bug exists, canvasNodes > 0. If fixed, canvasNodes = 0
  if (canvasNodes > 0) {
    console.log('🐛 BUG CONFIRMED: Agents auto-appeared on canvas');
    console.log('❌ This is the reported bug - agents should NOT auto-appear');
    throw new Error(`BUG DETECTED: ${canvasNodes} agents auto-appeared on canvas when they should only be in shelf`);
  } else {
    console.log('✅ NO BUG: Canvas is empty as expected');
    console.log('✅ Agents are correctly in shelf only');
  }

  // Verify agents are in shelf (they should be)
  expect(shelfAgents).toBeGreaterThan(0);
  expect(canvasNodes).toBe(0);

  console.log('🎉 Test Result: NO BUG DETECTED - Behavior is correct');
});

test('Bug Test with Multiple Agents', async ({ page }) => {
  console.log('🚨 TESTING WITH MULTIPLE AGENTS');

  await page.goto('/');
  await page.waitForSelector('table');
  
  // Create 2 agents quickly
  console.log('📋 Creating multiple agents...');
  await page.locator('[data-testid="create-agent-btn"]').click();
  await page.waitForTimeout(2000);
  await page.locator('[data-testid="create-agent-btn"]').click();
  await page.waitForTimeout(2000);
  
  // Switch to canvas
  console.log('🎨 Switching to canvas...');
  await page.locator('[data-testid="global-canvas-tab"]').click();
  await page.waitForTimeout(4000);

  // Check results
  const canvasNodes = await page.locator('#node-canvas .node').count();
  const shelfAgents = await page.locator('#agent-shelf .agent-pill').count();
  
  console.log(`Canvas: ${canvasNodes} nodes, Shelf: ${shelfAgents} agents`);
  
  if (canvasNodes > 0) {
    throw new Error(`BUG: ${canvasNodes} agents auto-appeared on canvas with multiple agents`);
  }
  
  expect(canvasNodes).toBe(0);
  expect(shelfAgents).toBeGreaterThan(0);
  
  console.log('✅ Multiple agents test: NO BUG DETECTED');
});