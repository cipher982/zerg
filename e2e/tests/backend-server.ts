/**
 * Backend server management for E2E tests
 * 
 * This module provides utilities to start and stop isolated backend servers
 * for each test worker, ensuring complete test isolation.
 */

import { spawn, ChildProcess } from 'child_process';
import { join } from 'path';
import * as fs from 'fs';
import * as path from 'path';

// Load dynamic backend port from .env
function getBackendPort(): number {
  // Check environment variable first
  if (process.env.BACKEND_PORT) {
    return parseInt(process.env.BACKEND_PORT);
  }
  
  // Load from .env file
  const envPath = path.resolve(__dirname, '../../../.env');
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

interface BackendServer {
  port: number;
  baseUrl: string;
  process?: ChildProcess;
}

const runningServers = new Map<string, BackendServer>();

export async function startBackendServer(workerId: string): Promise<BackendServer> {
  const existingServer = runningServers.get(workerId);
  if (existingServer) {
    return existingServer;
  }

  const basePort = getBackendPort();
  const port = basePort + parseInt(workerId);
  const baseUrl = `http://localhost:${port}`;
  
  console.log(`[backend-server] Starting backend for worker ${workerId} on port ${port}`);
  
  const backendProcess = spawn(
    'uv',
    ['run', 'python', '-m', 'uvicorn', 'test_main:app', `--port=${port}`, '--log-level=warning'],
    {
      env: {
        ...process.env,
        ENVIRONMENT: 'test',
        TEST_WORKER_ID: workerId,
        NODE_ENV: 'test',
        TESTING: '1',  // Enable testing mode for database reset
      },
      cwd: join(__dirname, '..', '..', 'backend'),
      stdio: 'pipe', // Capture output to avoid noise
    }
  );

  const server: BackendServer = {
    port,
    baseUrl,
    process: backendProcess,
  };

  runningServers.set(workerId, server);

  // Handle process events
  backendProcess.on('error', (error) => {
    console.error(`[backend-server] Worker ${workerId} backend error:`, error);
    runningServers.delete(workerId);
  });

  backendProcess.on('close', (code) => {
    console.log(`[backend-server] Worker ${workerId} backend exited with code ${code}`);
    runningServers.delete(workerId);
  });

  // Wait for server to be ready
  await waitForServer(baseUrl);
  
  console.log(`[backend-server] Backend ready for worker ${workerId} at ${baseUrl}`);
  return server;
}

export async function stopBackendServer(workerId: string): Promise<void> {
  const server = runningServers.get(workerId);
  if (server && server.process) {
    console.log(`[backend-server] Stopping backend for worker ${workerId}`);
    server.process.kill('SIGTERM');
    runningServers.delete(workerId);
  }
}

export async function stopAllBackendServers(): Promise<void> {
  console.log(`[backend-server] Stopping all backend servers`);
  const promises = Array.from(runningServers.keys()).map(stopBackendServer);
  await Promise.all(promises);
}

async function waitForServer(baseUrl: string, maxAttempts = 30): Promise<void> {
  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    try {
      const response = await fetch(`${baseUrl}/`, { method: 'GET' });
      if (response.ok) {
        return; // Server is ready
      }
    } catch (error) {
      // Server not ready yet, continue waiting
    }

    if (attempt < maxAttempts) {
      await new Promise(resolve => setTimeout(resolve, 1000)); // Wait 1 second
    }
  }
  
  throw new Error(`Backend server at ${baseUrl} did not start within ${maxAttempts} seconds`);
}

// Clean up on process exit
process.on('exit', () => {
  for (const [workerId, server] of runningServers.entries()) {
    if (server.process) {
      console.log(`[backend-server] Cleaning up backend for worker ${workerId}`);
      server.process.kill('SIGKILL');
    }
  }
});

process.on('SIGTERM', async () => {
  await stopAllBackendServers();
  process.exit(0);
});

process.on('SIGINT', async () => {
  await stopAllBackendServers();
  process.exit(0);
});