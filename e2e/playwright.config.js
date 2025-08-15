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
  workers: process.env.CI ? 4 : 2,
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
      // Start the frontend server
      command: `cd ../frontend && ./build-only.sh && cd ../e2e && FRONTEND_PORT=${FRONTEND_PORT} node wasm-server.js`,
      port: FRONTEND_PORT,
      reuseExistingServer: !process.env.CI, // Reuse in dev, fresh in CI
      timeout: 180_000,
    },
    // Start backend servers for all possible workers (up to 4)
    // Skip FRONTEND_PORT to avoid conflicts
    ...Array.from({ length: 4 }, (_, i) => {
      let backendPort = BACKEND_PORT + i;
      // Skip frontend port if it conflicts
      if (backendPort === FRONTEND_PORT) {
        backendPort = BACKEND_PORT + 4 + i; // Jump ahead to avoid conflict
      }
      return {
        command: `BACKEND_PORT=${backendPort} node spawn-test-backend.js ${i}`,
        port: backendPort,
        cwd: __dirname,
        reuseExistingServer: false, // Each worker needs isolated backend
        timeout: 60_000,
      };
    }),
  ].flat(),
};

module.exports = config;