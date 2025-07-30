import { test, expect } from './fixtures';

/**
 * Canvas API Contract Tests
 * These tests ensure frontend-backend API contracts are maintained
 * and prevent endpoint mismatches that cause runtime 404 errors.
 * 
 * This specifically tests the fix for the bug where frontend called
 * /api/workflows/current/canvas-data but backend only had /canvas
 */
test.describe('Canvas API Contract Tests', () => {
  
  test('Canvas endpoint exists and does not return 404', async ({ page }) => {
    // Test that the correct endpoint exists
    const response = await page.evaluate(async () => {
      try {
        const response = await fetch('/api/workflows/current/canvas', {
          method: 'PATCH',
          headers: {
            'Content-Type': 'application/json'
          },
          body: JSON.stringify({ canvas: { nodes: [], edges: [] } })
        });
        
        return {
          status: response.status,
          url: response.url
        };
      } catch (error) {
        return {
          error: error.message,
          status: 0
        };
      }
    });

    // The main assertion: should NOT return 404 (this was the bug)
    expect(response.status).not.toBe(404);
    // Could be 401 (auth required), 400 (validation), etc. but not 404
  });

  test('Wrong canvas-data endpoint returns 404', async ({ page }) => {
    // Test that the old incorrect endpoint returns 404
    const response = await page.evaluate(async () => {
      try {
        const response = await fetch('/api/workflows/current/canvas-data', {
          method: 'PATCH',
          headers: {
            'Content-Type': 'application/json'
          },
          body: JSON.stringify({ canvas_data: { nodes: [], edges: [] } })
        });
        
        return {
          status: response.status
        };
      } catch (error) {
        return {
          error: error.message,
          status: 0
        };
      }
    });

    // This SHOULD return 404 to confirm the wrong endpoint doesn't exist
    expect(response.status).toBe(404);
  });

  test('Canvas tab loads without network errors', async ({ page }) => {
    // This test ensures the frontend can switch to canvas without 404s
    await page.goto('/');
    
    // Monitor for any 404 errors specifically
    const errors: string[] = [];
    page.on('response', (response) => {
      if (response.status() === 404 && response.url().includes('canvas')) {
        errors.push(`404 error on ${response.url()}`);
      }
    });

    // Navigate to canvas tab
    const canvasTab = page.getByTestId('global-canvas-tab');
    if (await canvasTab.count() > 0) {
      await canvasTab.click();
      await page.waitForTimeout(2000); // Wait for any network requests
    }

    // Should not have any canvas-related 404 errors
    expect(errors).toHaveLength(0);
  });
});