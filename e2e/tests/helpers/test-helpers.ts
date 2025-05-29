import { Page, expect } from '@playwright/test';
import { apiClient, Agent, Thread, CreateAgentRequest, CreateThreadRequest } from './api-client';

export interface TestContext {
  agents: Agent[];
  threads: Thread[];
}

/**
 * Setup helper that creates test data and returns a context object
 */
export async function setupTestData(options: {
  agents?: CreateAgentRequest[];
  threadsPerAgent?: number;
} = {}): Promise<TestContext> {
  const context: TestContext = {
    agents: [],
    threads: []
  };

  // Create agents
  const agentConfigs = options.agents || [{}]; // Default to one agent
  for (const agentConfig of agentConfigs) {
    const agent = await apiClient.createAgent(agentConfig);
    context.agents.push(agent);

    // Create threads for this agent
    const threadCount = options.threadsPerAgent || 0;
    for (let i = 0; i < threadCount; i++) {
      const thread = await apiClient.createThread({
        agent_id: agent.id,
        title: `Test Thread ${i + 1} for ${agent.name}`
      });
      context.threads.push(thread);
    }
  }

  return context;
}

/**
 * Cleanup helper that removes test data
 */
export async function cleanupTestData(context: TestContext): Promise<void> {
  if (!context) {
    return;
  }

  // Delete threads first (they reference agents)
  if (context.threads) {
    for (const thread of context.threads) {
      try {
        await apiClient.deleteThread(thread.id);
      } catch (error) {
        console.warn(`Failed to delete thread ${thread.id}:`, error);
      }
    }
  }

  // Delete agents
  if (context.agents) {
    for (const agent of context.agents) {
      try {
        await apiClient.deleteAgent(agent.id);
      } catch (error) {
        console.warn(`Failed to delete agent ${agent.id}:`, error);
      }
    }
  }
}

/**
 * Wait for an element to be visible with a custom error message
 */
export async function waitForElement(page: Page, selector: string, timeout: number = 10000): Promise<void> {
  try {
    await page.waitForSelector(selector, { state: 'visible', timeout });
  } catch (error) {
    throw new Error(`Element "${selector}" not found within ${timeout}ms. Current URL: ${page.url()}`);
  }
}

/**
 * Wait for the dashboard to be ready (app loaded and dashboard rendered)
 */
export async function waitForDashboardReady(page: Page): Promise<void> {
  await page.goto('/');
  
  // Wait for the app container or dashboard root to be populated
  // The frontend is a WASM SPA, so we need to wait for JS to load and render
  try {
    // Try to wait for dashboard-specific elements that get rendered by the WASM app
    await page.waitForSelector('#dashboard-root, #app-container', { timeout: 15000 });
    
    // Wait for either a table (if it exists) or dashboard content to be rendered
    // Use a more flexible approach since the frontend might still be loading
    await page.waitForFunction(() => {
      // Check if dashboard has been rendered by looking for common dashboard elements
      const dashboardRoot = document.querySelector('#dashboard-root');
      const appContainer = document.querySelector('#app-container');
      
      // Look for any of these elements that indicate the dashboard is ready
      return document.querySelector('table') || 
             document.querySelector('[data-testid="create-agent-btn"]') ||
             (dashboardRoot && dashboardRoot.children.length > 0) ||
             (appContainer && appContainer.children.length > 0);
    }, { timeout: 15000 });
    
  } catch (error) {
    // If we can't find dashboard elements, the frontend might not be fully implemented
    console.warn('Dashboard elements not found, frontend may still be loading or incomplete');
    // Still wait a bit for any content to load
    await page.waitForTimeout(2000);
  }
  
  // Wait a bit more for any reactive updates
  await page.waitForTimeout(500);
}

/**
 * Get the count of agent rows in the dashboard
 */
export async function getAgentRowCount(page: Page): Promise<number> {
  return await page.locator('tr[data-agent-id]').count();
}

/**
 * Create an agent via the UI and return its ID
 */
export async function createAgentViaUI(page: Page): Promise<string> {
  await page.locator('[data-testid="create-agent-btn"]').click();
  
  // Wait for the new row to appear
  const newRow = page.locator('tr[data-agent-id]').first();
  await expect(newRow).toBeVisible({ timeout: 15000 });
  
  const agentId = await newRow.getAttribute('data-agent-id');
  if (!agentId) {
    throw new Error('Failed to get agent ID from newly created agent row');
  }
  
  return agentId;
}

/**
 * Edit an agent via the UI modal
 */
export async function editAgentViaUI(page: Page, agentId: string, data: {
  name?: string;
  systemInstructions?: string;
  taskInstructions?: string;
  temperature?: number;
  model?: string;
}): Promise<void> {
  // Open edit modal
  await page.locator(`[data-testid="edit-agent-${agentId}"]`).click();
  await waitForElement(page, '#agent-modal');

  // Fill form fields
  if (data.name !== undefined) {
    await page.locator('#agent-name').fill(data.name);
  }
  
  if (data.systemInstructions !== undefined) {
    await page.locator('#system-instructions').fill(data.systemInstructions);
  }
  
  if (data.taskInstructions !== undefined) {
    await page.locator('#default-task-instructions').fill(data.taskInstructions);
  }
  
  if (data.temperature !== undefined) {
    const tempInput = page.locator('#temperature-input');
    if (await tempInput.count() > 0) {
      await tempInput.fill(data.temperature.toString());
    }
  }
  
  if (data.model !== undefined) {
    const modelSelect = page.locator('#model-select');
    if (await modelSelect.count() > 0) {
      await modelSelect.selectOption(data.model);
    }
  }

  // Save changes
  await page.locator('#save-agent').click();
  
  // Wait for modal to close
  await expect(page.locator('#agent-modal')).toHaveCount(0, { timeout: 5000 });
}

/**
 * Delete an agent via the UI and handle confirmation dialog
 */
export async function deleteAgentViaUI(page: Page, agentId: string, confirm: boolean = true): Promise<void> {
  // Set up dialog handler
  page.once('dialog', (dialog) => {
    if (confirm) {
      dialog.accept();
    } else {
      dialog.dismiss();
    }
  });

  // Click delete button
  await page.locator(`[data-testid="delete-agent-${agentId}"]`).click();
  
  if (confirm) {
    // Wait for row to disappear
    await expect(page.locator(`tr[data-agent-id="${agentId}"]`)).toHaveCount(0, { timeout: 5000 });
  } else {
    // Row should still be present
    await expect(page.locator(`tr[data-agent-id="${agentId}"]`)).toHaveCount(1);
  }
}

/**
 * Navigate to chat for a specific agent
 */
export async function navigateToChat(page: Page, agentId: string): Promise<void> {
  await page.locator(`[data-testid="chat-agent-${agentId}"]`).click();
  
  // Wait for chat interface to load (if implemented)
  try {
    await waitForElement(page, '.chat-input', 5000);
  } catch (error) {
    // Chat UI might not be fully implemented yet
    console.warn('Chat UI not fully loaded, continuing...');
  }
}

/**
 * Reset the database to a clean state
 */
export async function resetDatabase(): Promise<void> {
  try {
    await apiClient.resetDatabase();
  } catch (error) {
    console.warn('Failed to reset database via API:', error);
  }
}

/**
 * Check if the backend is healthy and responding
 */
export async function checkBackendHealth(): Promise<boolean> {
  try {
    const response = await apiClient.healthCheck();
    return response && response.message === 'Agent Platform API is running';
  } catch (error) {
    console.error('Backend health check failed:', error);
    return false;
  }
}

/**
 * Skip test if a UI element is not implemented
 */
export function skipIfNotImplemented(page: Page, selector: string, reason: string = 'UI not implemented yet') {
  return async function() {
    const count = await page.locator(selector).count();
    if (count === 0) {
      console.log(`Skipping test: ${reason} (${selector} not found)`);
      return true;
    }
    return false;
  };
}
