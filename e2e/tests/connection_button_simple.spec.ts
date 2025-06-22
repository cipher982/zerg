import { test, expect } from './fixtures';

test('Connection button exists and is functional', async ({ page }) => {
  // Go to the app
  await page.goto('/');
  
  // Wait for app to load
  await page.waitForTimeout(2000);
  
  // Check if we can find the canvas tab and click it
  const canvasTab = page.getByTestId('global-canvas-tab');
  if (await canvasTab.count() > 0) {
    await canvasTab.click();
    await page.waitForTimeout(1000);
  }
  
  // Look for the connection mode button
  const connectionBtn = page.locator('#connection-mode-btn');
  
  // Test that the button exists
  await expect(connectionBtn).toBeVisible({ timeout: 10000 });
  
  // Test button properties
  const initialClass = await connectionBtn.getAttribute('class');
  const initialTitle = await connectionBtn.getAttribute('title');
  
  console.log('Initial button class:', initialClass);
  console.log('Initial button title:', initialTitle);
  
  // Click the button to toggle connection mode
  await connectionBtn.click();
  await page.waitForTimeout(500);
  
  // Check if button state changed
  const activeClass = await connectionBtn.getAttribute('class');
  const activeTitle = await connectionBtn.getAttribute('title');
  
  console.log('Active button class:', activeClass);
  console.log('Active button title:', activeTitle);
  
  expect(activeClass).toContain('active');
  expect(activeTitle).toContain('Exit Connection Mode');
  
  // Toggle back
  await connectionBtn.click();
  await page.waitForTimeout(500);
  
  const finalClass = await connectionBtn.getAttribute('class');
  expect(finalClass).not.toContain('active');
});