#!/usr/bin/env node

/**
 * Spawn isolated test backend for E2E tests
 * 
 * This script spawns a dedicated backend server for each Playwright worker,
 * ensuring complete test isolation without shared state.
 */

const { spawn } = require('child_process');
const { join } = require('path');

// Get worker ID from command line argument
const workerId = process.argv[2];
if (!workerId) {
    console.error('Usage: node spawn-test-backend.js <worker_id>');
    process.exit(1);
}

const port = 8000 + parseInt(workerId);
console.log(`[spawn-backend] Starting isolated backend for worker ${workerId} on port ${port}`);

// Spawn the test backend
const backend = spawn('uv', ['run', 'python', '-m', 'uvicorn', 'test_main:app', `--port=${port}`, '--log-level=warning'], {
    env: {
        ...process.env,
        ENVIRONMENT: 'test',
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