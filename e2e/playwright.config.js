// Simple, clean Playwright configuration - just read ports from .env
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

// Define workers count first so we can use it later
const workers = process.env.CI ? 4 : 2;

const config = {
  testDir: './tests',

  use: {
    baseURL: `http://localhost:${FRONTEND_PORT}`,
    headless: true,
    viewport: { width: 1280, height: 800 },
    
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
    
    navigationTimeout: 30_000,
    actionTimeout: 10_000,
  },

  globalSetup: require.resolve('./test-setup.js'),
  globalTeardown: require.resolve('./test-teardown.js'),

  fullyParallel: true,
  workers: workers,
  retries: process.env.CI ? 2 : 1,
  
  reporter: [
    ['list'],
    ['html', { open: 'never' }],
    ['junit', { outputFile: 'test-results/junit.xml' }]
  ],

  projects: [
    {
      name: 'chromium',
      use: { ...require('@playwright/test').devices['Desktop Chrome'] },
    }
  ],

  webServer: [
    {
      // Start the frontend server (static WASM assets)
      command: `cd ../frontend && ./build-only.sh && cd ../e2e && FRONTEND_PORT=${FRONTEND_PORT} node wasm-server.js`,
      port: FRONTEND_PORT,
      reuseExistingServer: !process.env.CI, // Reuse in dev, fresh in CI
      timeout: 180_000,
    },
    {
      // Start a single backend server; DB isolation happens via X-Test-Worker header
      command: `BACKEND_PORT=${BACKEND_PORT} node spawn-test-backend.js`,
      port: BACKEND_PORT,
      cwd: __dirname,
      reuseExistingServer: false,
      timeout: 60_000,
    },
  ],
};

module.exports = config;
