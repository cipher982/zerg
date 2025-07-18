import { Page } from '@playwright/test';
import { createApiClient } from './api-client';

/**
 * Database management helpers for E2E tests
 * Provides consistent patterns for database reset and cleanup
 */

export interface DatabaseResetOptions {
  retries?: number;
  timeout?: number;
  skipVerification?: boolean;
}

/**
 * Reset database for a specific worker with retry and verification
 */
export async function resetDatabaseForWorker(
  workerId: string, 
  options: DatabaseResetOptions = {}
): Promise<void> {
  const { retries = 3, timeout = 5000, skipVerification = false } = options;
  const apiClient = createApiClient(workerId);
  
  let attempts = 0;
  
  while (attempts < retries) {
    try {
      // Reset the database
      await apiClient.resetDatabase();
      
      if (!skipVerification) {
        // Verify reset was successful
        const agents = await apiClient.listAgents();
        if (agents.length === 0) {
          return; // Success
        }
        console.warn(`Database reset attempt ${attempts + 1}: Found ${agents.length} remaining agents`);
      } else {
        return; // Skip verification, assume success
      }
    } catch (error) {
      console.warn(`Database reset attempt ${attempts + 1} failed:`, error);
    }
    
    attempts++;
    if (attempts < retries) {
      await new Promise(resolve => setTimeout(resolve, 1000));
    }
  }
  
  throw new Error(`Failed to reset database after ${retries} attempts`);
}

/**
 * Reset database using page.request (for use in beforeEach hooks)
 */
export async function resetDatabaseViaRequest(
  page: Page,
  options: DatabaseResetOptions = {}
): Promise<void> {
  const { retries = 3 } = options;
  let attempts = 0;
  
  while (attempts < retries) {
    try {
      const response = await page.request.post('http://localhost:8001/api/admin/reset-database');
      if (response.ok()) {
        return;
      }
      console.warn(`Database reset attempt ${attempts + 1}: HTTP ${response.status()}`);
    } catch (error) {
      console.warn(`Database reset attempt ${attempts + 1} failed:`, error);
    }
    
    attempts++;
    if (attempts < retries) {
      await new Promise(resolve => setTimeout(resolve, 1000));
    }
  }
  
  throw new Error(`Failed to reset database via request after ${retries} attempts`);
}

/**
 * Ensure database is clean before starting a test
 * Automatically handles worker ID from test context
 */
export async function ensureCleanDatabase(page: Page, workerId: string): Promise<void> {
  try {
    await resetDatabaseForWorker(workerId, { retries: 2, skipVerification: false });
    console.log(`✅ Database reset successful for worker ${workerId}`);
  } catch (error) {
    console.warn(`⚠️  Database reset failed for worker ${workerId}:`, error);
    // Don't throw - let test proceed in case it's a transient issue
  }
}

/**
 * Create a beforeEach hook that resets the database
 * Usage: test.beforeEach(createDatabaseResetHook());
 */
export function createDatabaseResetHook(options: DatabaseResetOptions = {}) {
  return async ({ page }: { page: Page }) => {
    await resetDatabaseViaRequest(page, options);
  };
}

/**
 * Verify database is actually empty (useful for debugging isolation issues)
 */
export async function verifyDatabaseEmpty(workerId: string): Promise<boolean> {
  try {
    const apiClient = createApiClient(workerId);
    const agents = await apiClient.listAgents();
    return agents.length === 0;
  } catch (error) {
    console.warn(`Failed to verify database state for worker ${workerId}:`, error);
    return false;
  }
}

/**
 * Get database statistics for debugging
 */
export async function getDatabaseStats(workerId: string): Promise<{
  agentCount: number;
  workerId: string;
  timestamp: string;
}> {
  const apiClient = createApiClient(workerId);
  const agents = await apiClient.listAgents();
  
  return {
    agentCount: agents.length,
    workerId,
    timestamp: new Date().toISOString()
  };
}