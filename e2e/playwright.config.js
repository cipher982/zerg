// Load dynamic ports from environment or .env file
const fs = require('fs');
const path = require('path');

// Load .env file from repo root
const envPath = path.resolve(__dirname, '../.env');
let BACKEND_PORT = 8001;
let FRONTEND_PORT = 8002;

if (fs.existsSync(envPath)) {
  const envContent = fs.readFileSync(envPath, 'utf8');
  envContent.split('\n').forEach(line => {
    const [key, value] = line.split('=');
    if (key === 'BACKEND_PORT') BACKEND_PORT = parseInt(value) || 8001;
    if (key === 'FRONTEND_PORT') FRONTEND_PORT = parseInt(value) || 8002;
  });
}

// Allow env vars to override
BACKEND_PORT = process.env.BACKEND_PORT ? parseInt(process.env.BACKEND_PORT) : BACKEND_PORT;
FRONTEND_PORT = process.env.FRONTEND_PORT ? parseInt(process.env.FRONTEND_PORT) : FRONTEND_PORT;

const config = {
  testDir: './tests',

  use: {
    baseURL: `http://localhost:${FRONTEND_PORT}`,
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

  webServer: [
    {
      // Start the frontend server
      command: `cd ../frontend && ./build-only.sh && cd ../e2e && FRONTEND_PORT=${FRONTEND_PORT} node wasm-server.js`,
      port: FRONTEND_PORT,
      reuseExistingServer: true,
      timeout: 180_000,
    },
    // Start backend servers for all possible workers (up to 4)
    ...Array.from({ length: 4 }, (_, i) => ({
      command: `BACKEND_PORT=${BACKEND_PORT} node spawn-test-backend.js ${i}`,
      url: `http://localhost:${BACKEND_PORT + i}/api/agents`,
      cwd: __dirname,
      reuseExistingServer: false,
      timeout: 60_000,
    })),
  ].flat(),
};

module.exports = config;
