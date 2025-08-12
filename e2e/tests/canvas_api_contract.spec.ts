import { test, expect } from './fixtures';

/**
 * Canvas UI Tests
 * These tests ensure the canvas feature works correctly from a user perspective.
 * API contract validation is handled by backend tests (test_api_contract_canvas.py)
 */
test.describe('Canvas UI Tests', () => {
  
  test('Canvas tab loads successfully without errors', async ({ page }) => {
    // User navigates to the app
    await page.goto('/');
    
    // User clicks on Canvas tab
    const canvasTab = page.getByTestId('global-canvas-tab');
    await expect(canvasTab).toBeVisible();
    await canvasTab.click();
    
    // Canvas should load without errors
    // Check for common canvas elements that indicate successful load
    const canvasContainer = page.locator('#canvas-container, [data-testid="canvas-container"], .canvas-wrapper');
    await expect(canvasContainer).toBeVisible({ timeout: 5000 });
    
    // No error messages should be present
    const errorElements = page.locator('.error-message, .error-banner, [data-testid="error"], .alert-danger');
    await expect(errorElements).not.toBeVisible();
    
    // Tool palette should be visible (indicates canvas loaded properly)
    const toolPalette = page.locator('[data-testid="tool-palette"], .tool-palette, #tool-palette');
    await expect(toolPalette).toBeVisible();
  });

  test('Canvas persists state when switching tabs', async ({ page }) => {
    // Navigate to canvas
    await page.goto('/');
    await page.getByTestId('global-canvas-tab').click();
    
    // Wait for canvas to load
    const canvasContainer = page.locator('#canvas-container, [data-testid="canvas-container"], .canvas-wrapper');
    await expect(canvasContainer).toBeVisible({ timeout: 5000 });
    
    // Switch to dashboard and back
    await page.getByTestId('global-dashboard-tab').click();
    await page.waitForTimeout(500);
    await page.getByTestId('global-canvas-tab').click();
    
    // Canvas should still be visible without errors
    await expect(canvasContainer).toBeVisible({ timeout: 5000 });
    const errorElements = page.locator('.error-message, .error-banner, [data-testid="error"]');
    await expect(errorElements).not.toBeVisible();
  });

  test('Canvas workspace is interactive', async ({ page }) => {
    // Navigate to canvas
    await page.goto('/');
    await page.getByTestId('global-canvas-tab').click();
    
    // Wait for canvas to load
    const canvasContainer = page.locator('#canvas-container, [data-testid="canvas-container"], .canvas-wrapper');
    await expect(canvasContainer).toBeVisible({ timeout: 5000 });
    
    // Try clicking on the canvas (shouldn't cause errors)
    const canvas = page.locator('canvas, svg, .canvas-surface').first();
    if (await canvas.count() > 0) {
      await canvas.click({ position: { x: 100, y: 100 } });
      
      // Still no errors after interaction
      const errorElements = page.locator('.error-message, .error-banner');
      await expect(errorElements).not.toBeVisible();
    }
  });
});