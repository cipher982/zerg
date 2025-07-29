const config = {
  testDir: './tests',

  use: {
    baseURL: 'http://localhost:8002',
    headless: true,
    viewport: { width: 1280, height: 800 },
    trace: 'on-first-retry',
    // No longer need special headers - isolation handled at infrastructure level
  },

  // Modern test practices: Proper setup and cleanup
  globalSetup: require.resolve('./test-setup.js'),
  globalTeardown: require.resolve('./test-teardown.js'),

  fullyParallel: true,
  workers: 16,
  retries: process.env.CI ? 2 : 0,
  reporter: process.env.CI ? 'github' : 'list',

  webServer: {
    // Only start the frontend - backends will be started per-worker by tests
    command: 'cd ../frontend && ./build-only.sh && cd ../e2e && node wasm-server.js',
    port: 8002,
    reuseExistingServer: true,
    timeout: 180_000,
  },
};

module.exports = config;
