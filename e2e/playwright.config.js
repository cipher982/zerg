// Playwright configuration for Zerg E2E tests.
//
// Running `npx playwright test` from the `e2e/` directory will:
//   1. Spin-up the backend (port 8001) and the frontend dev server (port 8002)
//      via the `webServer` helper, unless they are already running.
//   2. Execute all specs found in `./tests`.
//
/** @type {import('@playwright/test').PlaywrightTestConfig} */
const config = {
  testDir: './tests',

  use: {
    baseURL: 'http://localhost:8002',
    headless: true,
    viewport: { width: 1280, height: 800 },
    trace: 'on-first-retry',
  },

  // Test configuration
  fullyParallel: true,
  retries: process.env.CI ? 2 : 0,
  reporter: process.env.CI ? 'github' : 'list',

  // Override default worker count.  Playwright defaults to the number of
  // CPU cores but we sometimes want to saturate logical cores / hyper-threads
  // on beefy machines.  Adjust via PW_WORKERS env or fallback to 16.
  workers: process.env.PW_WORKERS ? parseInt(process.env.PW_WORKERS, 10) : 16,

  // Automatically start backend + frontend unless they are already running.
  webServer: [
    {
      command: 'cd ../backend && E2E_LOG_SUPPRESS=1 LOG_LEVEL=WARNING uv run python -m uvicorn zerg.main:app --port 8001 --log-level warning',
      port: 8001,
      reuseExistingServer: true,
      timeout: 120_000,
    },
    {
      command: 'cd ../frontend && ./build-debug.sh > /dev/null 2>&1',
      url: 'http://localhost:8002',
      reuseExistingServer: true,
      timeout: 180_000,
    },
  ],
};

module.exports = config;
