import { test, expect } from './fixtures';

/**
 * DEBUG: Header Transmission Investigation
 * 
 * Let's see if headers are being sent and received properly
 */

test.describe('Debug Header Transmission', () => {
  test('Debug: Header transmission check', async ({ page }) => {
    console.log('🔍 Investigating header transmission...');
    
    await page.goto('/');
    await page.waitForTimeout(1000);

    // Get the worker ID from environment
    const workerId = process.env.PW_TEST_WORKER_INDEX || '0';
    console.log('📊 Worker ID:', workerId);
    
    // Make API request with explicit headers
    const response = await page.request.get('http://localhost:8001/api/agents', {
      headers: {
        'X-Test-Worker': workerId,
        'Content-Type': 'application/json'
      }
    });
    
    console.log('📊 Response status:', response.status());
    console.log('📊 Response headers:', await response.headers());
    
    try {
      const text = await response.text();
      console.log('📊 Response body (first 300 chars):', text.substring(0, 300));
    } catch (error) {
      console.log('❌ Response body error:', error);
    }
    
    console.log('✅ Header transmission test complete');
  });
});