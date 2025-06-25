import { test, expect } from './fixtures';

/**
 * BASIC DATABASE ISOLATION TEST
 * 
 * This test verifies that our test database isolation is working properly.
 * Each test should start with a completely clean database.
 */

test.describe('Basic Database Isolation', () => {
  test('Test 1: Should start with empty database', async ({ page }) => {
    console.log('🧪 Test 1: Checking for clean database state...');
    
    await page.goto('/');
    await page.waitForTimeout(1000);
    
    // Navigate to dashboard
    await page.getByTestId('global-dashboard-tab').click();
    await page.waitForTimeout(1000);
    
    // Count existing agents (exclude placeholder messages)
    const agentRows = page.locator('table tbody tr');
    const initialCount = await agentRows.count();
    
    console.log(`📊 Test 1 Initial agent count: ${initialCount}`);
    
    // Check if it's a real agent or placeholder
    let realAgentCount = 0;
    for (let i = 0; i < initialCount; i++) {
      const agentName = await agentRows.nth(i).locator('td').first().textContent();
      if (agentName && !agentName.includes('No agents found')) {
        realAgentCount++;
        console.log(`  Real agent: ${agentName}`);
      } else {
        console.log(`  Placeholder: ${agentName}`);
      }
    }
    
    console.log(`📊 Test 1 Real agent count: ${realAgentCount}`);
    
    // With in-memory DB, should be 0 real agents
    expect(realAgentCount).toBe(0);
    
    console.log('✅ Test 1: Database is clean');
  });

  test('Test 2: Should also start with empty database', async ({ page }) => {
    console.log('🧪 Test 2: Checking for clean database state...');
    
    await page.goto('/');
    await page.waitForTimeout(1000);
    
    // Navigate to dashboard
    await page.getByTestId('global-dashboard-tab').click();
    await page.waitForTimeout(1000);
    
    // Count existing agents (exclude placeholder messages)
    const agentRows = page.locator('table tbody tr');
    const initialCount = await agentRows.count();
    
    console.log(`📊 Test 2 Initial agent count: ${initialCount}`);
    
    // Check if it's a real agent or placeholder
    let realAgentCount = 0;
    for (let i = 0; i < initialCount; i++) {
      const agentName = await agentRows.nth(i).locator('td').first().textContent();
      if (agentName && !agentName.includes('No agents found')) {
        realAgentCount++;
        console.log(`  Real agent: ${agentName}`);
      } else {
        console.log(`  Placeholder: ${agentName}`);
      }
    }
    
    console.log(`📊 Test 2 Real agent count: ${realAgentCount}`);
    
    // Should still be 0 real agents in fresh test
    expect(realAgentCount).toBe(0);
    
    console.log('✅ Test 2: Database is clean');
  });

  test('Test 3: Create agent and verify isolation', async ({ page }) => {
    console.log('🧪 Test 3: Create agent and verify it exists only in this test...');
    
    await page.goto('/');
    await page.waitForTimeout(1000);
    
    // Navigate to dashboard
    await page.getByTestId('global-dashboard-tab').click();
    await page.waitForTimeout(1000);
    
    // Verify clean start (exclude placeholder messages)
    const agentRows = page.locator('table tbody tr');
    const initialCount = await agentRows.count();
    console.log(`📊 Test 3 Initial agent count: ${initialCount}`);
    
    // Check if it's a real agent or placeholder
    let realAgentCount = 0;
    for (let i = 0; i < initialCount; i++) {
      const agentName = await agentRows.nth(i).locator('td').first().textContent();
      if (agentName && !agentName.includes('No agents found')) {
        realAgentCount++;
        console.log(`  Real agent: ${agentName}`);
      } else {
        console.log(`  Placeholder: ${agentName}`);
      }
    }
    
    console.log(`📊 Test 3 Initial real agent count: ${realAgentCount}`);
    expect(realAgentCount).toBe(0);
    
    // Create ONE agent
    const createBtn = page.locator('button:has-text("Create Agent")');
    await createBtn.click();
    await page.waitForTimeout(1000);
    
    // Verify exactly one agent exists
    const finalCount = await agentRows.count();
    console.log(`📊 Test 3 Final agent count: ${finalCount}`);
    expect(finalCount).toBe(1);
    
    console.log('✅ Test 3: Agent created successfully');
  });
});