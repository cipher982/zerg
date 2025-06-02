// Playwright configuration for Zerg E2E tests.
//
// Running `npx playwright test` from the `e2e/` directory will:
//   1. Spin-up the backend (port 9001) and the frontend dev server (port 9002)
//      via the `webServer` helper, unless they are already running.
//   2. Execute all specs found in `./tests`.
//
// Uses ports 8001/8002 with reuseExistingServer: false to avoid conflicts
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
  fullyParallel: false,
  retries: process.env.CI ? 2 : 0,
  reporter: process.env.CI ? 'github' : 'list',

  // Run tests sequentially to avoid database conflicts
  workers: 1,

  // Automatically start backend + frontend unless they are already running.
  webServer: [
    {
      // Backend on port 8001 (will kill existing server if needed)
      command: 'cd ../backend && mkdir -p .uv_cache .uv_tmp && XDG_CACHE_HOME=$(pwd)/.uv_cache TMPDIR=$(pwd)/.uv_tmp TESTING=1 E2E_LOG_SUPPRESS=1 LOG_LEVEL=WARNING uv run python -m uvicorn zerg.main:app --port 8001 --log-level warning',
      port: 8001,
      reuseExistingServer: false,
      timeout: 120_000,
    },
    {
      // Frontend on port 8002 with WASM-aware server
      command: 'cd ../frontend && ./build-only.sh && cd ../e2e && node wasm-server.js',
      port: 8002,
      reuseExistingServer: false,
      timeout: 180_000,
    },
  ],
};

module.exports = config;
