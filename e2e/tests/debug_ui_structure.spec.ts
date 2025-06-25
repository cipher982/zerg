import { test, expect } from './fixtures';

/**
 * DEBUG: UI Structure Investigation
 * 
 * Let's see what the actual UI looks like to understand:
 * 1. How agent creation really works
 * 2. What the canvas/shelf structure actually is
 * 3. What tool palette elements exist
 */

test.describe('Debug UI Structure', () => {
  test('Debug: Agent creation flow', async ({ page }) => {
    console.log('🔍 Investigating agent creation...');
    
    await page.goto('/');
    await page.getByTestId('global-dashboard-tab').click();
    await page.waitForTimeout(1000);
    
    // Log initial state
    console.log('📊 BEFORE creating agent:');
    const beforeRows = await page.locator('table tbody tr').all();
    console.log(`  Table rows: ${beforeRows.length}`);
    for (let i = 0; i < beforeRows.length; i++) {
      const text = await beforeRows[i].textContent();
      console.log(`  Row ${i + 1}: ${text?.substring(0, 100)}`);
    }
    
    // Try to create agent
    console.log('🔨 Clicking Create Agent button...');
    const createBtn = page.locator('button:has-text("Create Agent")');
    console.log(`  Create button count: ${await createBtn.count()}`);
    console.log(`  Create button visible: ${await createBtn.isVisible()}`);
    
    await createBtn.click();
    console.log('  ✅ Clicked create button');
    
    // Wait and check what happened
    await page.waitForTimeout(2000);
    
    console.log('📊 AFTER creating agent:');
    const afterRows = await page.locator('table tbody tr').all();
    console.log(`  Table rows: ${afterRows.length}`);
    for (let i = 0; i < afterRows.length; i++) {
      const text = await afterRows[i].textContent();
      console.log(`  Row ${i + 1}: ${text?.substring(0, 100)}`);
    }
    
    // Check network requests
    console.log('📊 Network activity during agent creation should be logged above');
  });

  test('Debug: Canvas and shelf structure', async ({ page }) => {
    console.log('🔍 Investigating canvas structure...');
    
    await page.goto('/');
    await page.getByTestId('global-dashboard-tab').click();
    await page.waitForTimeout(1000);
    
    // Create an agent first
    const createBtn = page.locator('button:has-text("Create Agent")');
    await createBtn.click();
    await page.waitForTimeout(1000);
    
    // Go to canvas
    console.log('🎨 Navigating to canvas...');
    await page.getByTestId('global-canvas-tab').click();
    await page.waitForTimeout(3000); // Give it time to load
    
    // Log canvas structure
    console.log('📊 CANVAS STRUCTURE:');
    const canvasContainer = page.locator('#canvas-container');
    console.log(`  Canvas container exists: ${await canvasContainer.count()}`);
    
    if (await canvasContainer.count() > 0) {
      const canvasHTML = await canvasContainer.innerHTML();
      console.log(`  Canvas HTML length: ${canvasHTML.length}`);
      console.log(`  Canvas HTML sample: ${canvasHTML.substring(0, 500)}`);
    }
    
    // Look for agent shelf with different selectors
    console.log('📊 AGENT SHELF INVESTIGATION:');
    const shelfSelectors = [
      '#agent-shelf',
      '.agent-shelf', 
      '[data-testid="agent-shelf"]',
      '.agent-pill',
      '[class*="agent"]',
      '[class*="shelf"]'
    ];
    
    for (const selector of shelfSelectors) {
      const elements = page.locator(selector);
      const count = await elements.count();
      console.log(`  ${selector}: ${count} elements`);
      
      if (count > 0) {
        for (let i = 0; i < Math.min(count, 3); i++) {
          const text = await elements.nth(i).textContent();
          console.log(`    Element ${i + 1}: ${text?.substring(0, 50)}`);
        }
      }
    }
  });

  test('Debug: Tool palette structure', async ({ page }) => {
    console.log('🔍 Investigating tool palette...');
    
    await page.goto('/');
    await page.getByTestId('global-canvas-tab').click();
    await page.waitForTimeout(3000);
    
    console.log('📊 TOOL PALETTE INVESTIGATION:');
    
    // Look for tool palette with different selectors
    const paletteSelectors = [
      '.palette-node',
      '.tool-palette-item',
      '[data-testid*="palette"]',
      '[data-testid*="tool"]',
      '[draggable="true"]',
      'text="HTTP Request"',
      'text="🌐 HTTP Request"',
      '[class*="tool"]',
      '[class*="palette"]'
    ];
    
    for (const selector of paletteSelectors) {
      const elements = page.locator(selector);
      const count = await elements.count();
      console.log(`  ${selector}: ${count} elements`);
      
      if (count > 0) {
        for (let i = 0; i < Math.min(count, 5); i++) {
          const element = elements.nth(i);
          const text = await element.textContent();
          const visible = await element.isVisible();
          const draggable = await element.getAttribute('draggable');
          console.log(`    Element ${i + 1}: "${text?.substring(0, 30)}" | Visible: ${visible} | Draggable: ${draggable}`);
        }
      }
    }
    
    // Check for HTTP specifically
    console.log('📊 HTTP TOOL SEARCH:');
    const httpSelectors = [
      'text="HTTP"',
      'text="Request"', 
      'text="HTTP Request"',
      '[data-node-type*="http"]',
      '[class*="http"]'
    ];
    
    for (const selector of httpSelectors) {
      const elements = page.locator(selector);
      const count = await elements.count();
      if (count > 0) {
        console.log(`  Found HTTP with ${selector}: ${count} elements`);
      }
    }
  });
});