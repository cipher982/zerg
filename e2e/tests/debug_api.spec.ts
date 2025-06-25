import { test, expect } from './fixtures';

/**
 * DEBUG: API Access Investigation
 * 
 * Let's see if API requests are reaching the backend
 */

test.describe('Debug API Access', () => {
  test('Debug: API endpoint access', async ({ page }) => {
    console.log('🔍 Investigating API access...');
    
    await page.goto('/');
    await page.waitForTimeout(1000);
    
    // Check what the API base URL is in the page
    const apiBaseUrl = await page.evaluate(() => {
      return (window as any).APP_CONFIG?.API_BASE_URL || 'http://localhost:8001';
    });
    console.log('📊 API Base URL:', apiBaseUrl);
    
    // Get the worker ID from environment
    const workerId = process.env.PW_TEST_WORKER_INDEX || '0';
    console.log('📊 Worker ID:', workerId);
    
    // Try to access the API directly using the baseURL from context
    try {
      const response = await page.request.get(`${apiBaseUrl}/api/agents`, {
        headers: {
          'X-Test-Worker': workerId
        }
      });
      console.log('📊 Direct agent API response status:', response.status());
      const text = await response.text();
      console.log('📊 Direct agent API response body (first 200 chars):', text.substring(0, 200));
    } catch (error) {
      console.log('❌ Direct agent API failed:', error);
    }
    
    // Also try a simple root API endpoint
    try {
      const response = await page.request.get(`${apiBaseUrl}/`, {
        headers: {
          'X-Test-Worker': workerId
        }
      });
      console.log('📊 Root API response status:', response.status());
      const text = await response.text();
      console.log('📊 Root API response:', text);
    } catch (error) {
      console.log('❌ Root API failed:', error);
    }
    
    console.log('✅ Debug API test complete');
  });
});