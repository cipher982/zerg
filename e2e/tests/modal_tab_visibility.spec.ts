import { test, expect } from '@playwright/test';

// NB:  This is a *smoke* test – we do not attempt to mock the entire backend.
// The compiled WASM app is expected to be served on http://localhost:8002 by
// `./build.sh --serve` (see README).  CI starts the dev-server in the
// background before invoking `pnpm playwright test`.

test('Agent modal – only one tab-content is visible', async ({ page }) => {
  // 1) Load SPA
  await page.goto('http://localhost:8002');

  // 2) Open *New Agent* modal (button lives in header)
  await page.click('text="New Agent"');

  // The modal DOM is injected lazily – wait until present.
  await page.waitForSelector('#agent-modal', { state: 'visible' });

  // 3) Navigate to Triggers, then back to Main.
  await page.click('#agent-triggers-tab');
  await page.click('#agent-main-tab');

  // 4) Assert exactly one visible .tab-content inside the modal.
  const visibleSections = await page.$$('#agent-modal .tab-content:not([hidden])');
  expect(visibleSections).toHaveLength(1);
});
