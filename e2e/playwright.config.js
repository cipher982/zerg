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

  // Modern test practices: Proper setup and cleanup
  globalSetup: require.resolve('./test-setup.js'),
  globalTeardown: require.resolve('./test-teardown.js'),

  fullyParallel: true,
  workers: 16,
  retries: process.env.CI ? 2 : 0,
  reporter: process.env.CI ? 'github' : 'list',

  webServer: [
    {
      // Launch the FastAPI backend using uv run for proper dependency management
      command: 'cd ../backend && NODE_ENV=test TESTING=1 WORKER_ID=${PW_TEST_WORKER_INDEX:-0} uv run python -m uvicorn zerg.main:app --port 8001 --log-level warning',
      port: 8001,
      reuseExistingServer: true,
      timeout: 120_000,
    },
    {
      command: 'cd ../frontend && ./build-only.sh && cd ../e2e && node wasm-server.js',
      port: 8002,
      reuseExistingServer: true,
      timeout: 180_000,
    },
  ],
};

module.exports = config;
