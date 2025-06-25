import { test, expect } from './fixtures';

/**
 * MINIMAL DATABASE TEST
 * Just check what's in the database immediately on startup
 */

test('Minimal: Check immediate database state', async ({ page }) => {
  console.log('🔬 Minimal test: Just checking raw database state...');
  
  // Don't wait, just go straight to the dashboard
  await page.goto('/');
  
  // Navigate to dashboard immediately
  await page.getByTestId('global-dashboard-tab').click();
  await page.waitForTimeout(500);
  
  // Check what's there
  const agentRows = page.locator('table tbody tr');
  const count = await agentRows.count();
  
  console.log(`🔍 Raw database state: ${count} agents`);
  
  // If there are agents, log what they are
  if (count > 0) {
    for (let i = 0; i < count; i++) {
      const agentName = await agentRows.nth(i).locator('td').first().textContent();
      console.log(`  Agent ${i + 1}: ${agentName}`);
    }
  }
  
  // Just log, don't assert for now
  console.log('✅ Minimal test complete - logged database state');
});