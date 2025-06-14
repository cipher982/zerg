const config = {
  testDir: './tests',

  use: {
    baseURL: 'http://localhost:8002',
    headless: true,
    viewport: { width: 1280, height: 800 },
    trace: 'on-first-retry',
    extraHTTPHeaders: {
      'X-Test-Worker': process.env.PW_TEST_WORKER_INDEX || '0',
    },
  },

  fullyParallel: true,
  workers: 16,
  retries: process.env.CI ? 2 : 0,
  reporter: process.env.CI ? 'github' : 'list',

  webServer: [
    {
      // Launch the FastAPI backend.  Using the system Python directly avoids
      // the `uv run` wrapper which attempts to access the `.uv_cache` folder
      // and occasionally fails with EPERM inside the sandbox.
      command: 'cd ../backend && TESTING=1 DEV_ADMIN=1 WORKER_ID=$PW_TEST_WORKER_INDEX E2E_LOG_SUPPRESS=1 LOG_LEVEL=WARNING ./.venv/bin/python -m uvicorn zerg.main:app --port 8001 --log-level warning',
      port: 8001,
      reuseExistingServer: false,
      timeout: 120_000,
    },
    {
      command: 'cd ../frontend && ./build-only.sh && cd ../e2e && node wasm-server.js',
      port: 8002,
      reuseExistingServer: false,
      timeout: 180_000,
    },
  ],
};

module.exports = config;
