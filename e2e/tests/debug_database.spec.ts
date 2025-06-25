import { test, expect } from './fixtures';

/**
 * DEBUG: Database Tables Investigation
 * 
 * Let's see what tables are actually being created
 */

test.describe('Debug Database Tables', () => {
  test('Debug: Check database tables', async ({ page }) => {
    console.log('🔍 Investigating database tables...');
    
    await page.goto('/');
    await page.waitForTimeout(1000);
    
    // Try to trigger database usage
    await page.getByTestId('global-dashboard-tab').click();
    await page.waitForTimeout(2000);
    
    // Check if we can see the tables via admin endpoint
    try {
      const response = await page.goto('/api/admin/database-info');
      const text = await response?.text();
      console.log('📊 Database info response:', text);
    } catch (error) {
      console.log('❌ Database info failed:', error);
    }
    
    // Also check the agent creation endpoint directly
    try {
      const response = await page.request.post('/api/agents', {
        data: {
          name: 'Test Agent',
          system_instructions: 'You are a test agent'
        }
      });
      console.log('📊 Agent creation response:', response.status());
      const text = await response.text();
      console.log('📊 Agent creation response body:', text);
    } catch (error) {
      console.log('❌ Agent creation failed:', error);
    }
    
    console.log('✅ Debug database test complete');
  });
});