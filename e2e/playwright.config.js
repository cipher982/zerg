const config = {
  testDir: './tests',

  use: {
    baseURL: 'http://localhost:8002',
    headless: true,
    viewport: { width: 1280, height: 800 },
    
    // Modern debugging & observability  
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
    
    // Performance & reliability
    navigationTimeout: 30_000,
    actionTimeout: 10_000,
    
    // No longer need special headers - isolation handled at infrastructure level
  },

  // Modern test practices: Proper setup and cleanup
  globalSetup: require.resolve('./test-setup.js'),
  globalTeardown: require.resolve('./test-teardown.js'),

  // Conservative parallelism to avoid overwhelming system
  fullyParallel: true,
  workers: process.env.CI ? 4 : 2, // Reduced from 16 to be less disruptive
  retries: process.env.CI ? 2 : 1,
  
  // Better reporting
  reporter: [
    ['list'],
    ['html', { open: 'never' }], // Generate HTML report but don't auto-open
    ['junit', { outputFile: 'test-results/junit.xml' }]
  ],

  // Test categorization
  projects: [
    {
      name: 'chromium',
      use: { ...require('@playwright/test').devices['Desktop Chrome'] },
    }
  ],

  webServer: {
    // Only start the frontend - backends will be started per-worker by tests
    command: 'cd ../frontend && ./build-only.sh && cd ../e2e && node wasm-server.js',
    port: 8002,
    reuseExistingServer: true,
    timeout: 180_000,
  },
};

module.exports = config;
