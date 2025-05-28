import { test, expect } from '@playwright/test';

test.describe('Feature: Error Handling Scenarios', () => {
  test('Network timeout handling shows error banner', async ({ page }) => {
    // Simulate slow network
    await page.route('**/api/agents', route => setTimeout(() => route.continue(), 6000));
    await page.goto('/');
    // Expect error banner
    await expect(page.locator('.error-banner')).toBeVisible({ timeout: 10000 });
  });

  test('404 error page displays for unknown route', async ({ page }) => {
    await page.goto('/non-existent', { waitUntil: 'networkidle' });
    await expect(page.locator('text="404"')).toBeVisible();
  });

  test('API error messages display to user', async ({ page }) => {
    await page.route('**/api/agents', route => route.fulfill({ status: 500, body: 'Server error' }));
    await page.goto('/');
    await expect(page.locator('.toast-error')).toContainText('Server error');
  });

  test('Session expiry redirects to login', async ({ page }) => {
    // Simulate expired JWT
    await page.addInitScript(() => { localStorage.setItem('jwt', 'expired'); });
    await page.goto('/');
    await expect(page.locator('#login-overlay')).toBeVisible();
  });

  test('Offline mode shows offline banner', async ({ page }) => {
    await page.goto('/');
    await page.setOffline();
    await page.click('[data-testid="create-agent-btn"]');
    await expect(page.locator('.offline-banner')).toBeVisible();
  });
  // TODO: rate limit responses handling
});