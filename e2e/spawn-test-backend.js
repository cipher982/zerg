#!/usr/bin/env node

/**
 * Spawn isolated test backend for E2E tests
 * 
 * This script spawns a dedicated backend server for each Playwright worker,
 * ensuring complete test isolation without shared state.
 */

const { spawn } = require('child_process');
const { join } = require('path');
const fs = require('fs');
const path = require('path');

// Load dynamic port from .env file
function getBackendPort() {
    // Check environment variable first
    if (process.env.BACKEND_PORT) {
        return parseInt(process.env.BACKEND_PORT);
    }
    
    // Load from .env file
    const envPath = path.resolve(__dirname, '../.env');
    if (fs.existsSync(envPath)) {
        const envContent = fs.readFileSync(envPath, 'utf8');
        const lines = envContent.split('\n');
        for (const line of lines) {
            const [key, value] = line.split('=');
            if (key === 'BACKEND_PORT') {
                return parseInt(value) || 8001;
            }
        }
    }
    
    return 8001; // Default fallback
}

// Get worker ID from command line argument
const workerId = process.argv[2];
if (!workerId) {
    console.error('Usage: node spawn-test-backend.js <worker_id>');
    process.exit(1);
}

const basePort = getBackendPort();
const port = basePort + parseInt(workerId);
console.log(`[spawn-backend] Starting isolated backend for worker ${workerId} on port ${port} (base: ${basePort})`);

// Spawn the test backend with E2E configuration
const backend = spawn('uv', ['run', 'python', '-m', 'uvicorn', 'test_main:app', `--port=${port}`, '--log-level=warning'], {
    env: {
        ...process.env,
        ENVIRONMENT: 'test:e2e',  // Use E2E test config for real models
        TEST_WORKER_ID: workerId,
        NODE_ENV: 'test',
    },
    cwd: join(__dirname, '..', 'backend'),
    stdio: 'inherit'
});

// Handle backend process events
backend.on('error', (error) => {
    console.error(`[spawn-backend] Worker ${workerId} backend error:`, error);
    process.exit(1);
});

backend.on('close', (code) => {
    console.log(`[spawn-backend] Worker ${workerId} backend exited with code ${code}`);
    process.exit(code);
});

// Forward signals to backend process
process.on('SIGTERM', () => {
    console.log(`[spawn-backend] Worker ${workerId} received SIGTERM, shutting down backend`);
    backend.kill('SIGTERM');
});

process.on('SIGINT', () => {
    console.log(`[spawn-backend] Worker ${workerId} received SIGINT, shutting down backend`);
    backend.kill('SIGINT');
});

// Keep the spawner running
process.stdin.resume();