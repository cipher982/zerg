import { Page, expect } from '@playwright/test';
import { createApiClient, Agent, CreateAgentRequest } from './api-client';

/**
 * Agent lifecycle helpers for E2E tests
 * Provides consistent patterns for agent creation, management, and cleanup
 */

export interface AgentCreationOptions {
  name?: string;
  model?: string;
  systemInstructions?: string;
  taskInstructions?: string;
  temperature?: number;
  retries?: number;
}

export interface AgentBatchOptions {
  count: number;
  namePrefix?: string;
  model?: string;
  systemInstructions?: string;
  taskInstructions?: string;
}

/**
 * Create a single agent via API with sensible defaults
 */
export async function createAgentViaAPI(
  workerId: string,
  options: AgentCreationOptions = {}
): Promise<Agent> {
  const apiClient = createApiClient(workerId);
  
  const config: CreateAgentRequest = {
    name: options.name || `Test Agent ${workerId}`,
    model: options.model || 'gpt-4o-mini',
    system_instructions: options.systemInstructions || 'You are a test agent for E2E testing',
    task_instructions: options.taskInstructions || 'Perform test tasks as requested',
    temperature: options.temperature || 0.7,
  };
  
  const retries = options.retries || 3;
  let attempts = 0;
  
  while (attempts < retries) {
    try {
      const agent = await apiClient.createAgent(config);
      console.log(`✅ Agent created via API: ${agent.name} (ID: ${agent.id})`);
      return agent;
    } catch (error) {
      attempts++;
      console.warn(`Agent creation attempt ${attempts} failed:`, error);
      
      if (attempts >= retries) {
        throw new Error(`Failed to create agent after ${retries} attempts: ${error}`);
      }
      
      await new Promise(resolve => setTimeout(resolve, 1000));
    }
  }
  
  throw new Error('Unexpected error in agent creation');
}

/**
 * Create multiple agents in batch
 */
export async function createMultipleAgents(
  workerId: string,
  options: AgentBatchOptions
): Promise<Agent[]> {
  const agents: Agent[] = [];
  const namePrefix = options.namePrefix || 'Batch Agent';
  
  for (let i = 0; i < options.count; i++) {
    const agent = await createAgentViaAPI(workerId, {
      name: `${namePrefix} ${i + 1}`,
      model: options.model,
      systemInstructions: options.systemInstructions,
      taskInstructions: options.taskInstructions,
    });
    agents.push(agent);
  }
  
  console.log(`✅ Created ${agents.length} agents in batch`);
  return agents;
}

/**
 * Create agent via UI and return its ID
 * Uses the dashboard create button
 */
export async function createAgentViaUI(page: Page): Promise<string> {
  // Navigate to dashboard if not already there
  try {
    await page.getByTestId('global-dashboard-tab').click();
    await page.waitForTimeout(500);
  } catch (error) {
    // Ignore if already on dashboard or button doesn't exist
  }
  
  // Click create agent button
  await page.locator('[data-testid="create-agent-btn"]').click();
  
  // Wait for new row to be visible and stable
  const newRow = page.locator('tr[data-agent-id]').first();
  await expect(newRow).toBeVisible({ timeout: 5000 });
  
  // Wait for row to be fully rendered
  await page.waitForFunction(
    (selector) => {
      const row = document.querySelector(selector);
      return row && row.clientHeight > 0 && row.clientWidth > 0;
    },
    'tr[data-agent-id]:first-of-type',
    { timeout: 5000 }
  );
  
  const agentId = await newRow.getAttribute('data-agent-id');
  if (!agentId) {
    throw new Error('Failed to get agent ID from newly created agent row');
  }
  
  console.log(`✅ Agent created via UI with ID: ${agentId}`);
  return agentId;
}

/**
 * Get agent by ID with retry logic
 */
export async function getAgentById(workerId: string, agentId: string): Promise<Agent | null> {
  const apiClient = createApiClient(workerId);
  
  try {
    const agents = await apiClient.listAgents();
    return agents.find(agent => agent.id === agentId) || null;
  } catch (error) {
    console.warn(`Failed to get agent ${agentId}:`, error);
    return null;
  }
}

/**
 * Verify agent exists and has expected properties
 */
export async function verifyAgentExists(
  workerId: string, 
  agentId: string, 
  expectedName?: string
): Promise<boolean> {
  const agent = await getAgentById(workerId, agentId);
  
  if (!agent) {
    console.warn(`Agent ${agentId} not found`);
    return false;
  }
  
  if (expectedName && agent.name !== expectedName) {
    console.warn(`Agent ${agentId} has name "${agent.name}", expected "${expectedName}"`);
    return false;
  }
  
  return true;
}

/**
 * Wait for agent to appear in UI
 */
export async function waitForAgentInUI(page: Page, agentId: string, timeout: number = 10000): Promise<void> {
  await expect(page.locator(`tr[data-agent-id="${agentId}"]`)).toBeVisible({ timeout });
}

/**
 * Edit agent via UI modal
 */
export async function editAgentViaUI(
  page: Page, 
  agentId: string, 
  data: {
    name?: string;
    systemInstructions?: string;
    taskInstructions?: string;
    temperature?: number;
    model?: string;
  }
): Promise<void> {
  // Open edit modal
  await page.locator(`[data-testid="edit-agent-${agentId}"]`).click();
  await expect(page.locator('#agent-modal')).toBeVisible({ timeout: 5000 });
  await page.waitForSelector('#agent-name:not([disabled])', { timeout: 5000 });

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
  await expect(page.locator('#agent-modal')).not.toBeVisible({ timeout: 5000 });
  
  console.log(`✅ Agent ${agentId} edited via UI`);
}

/**
 * Delete agent via UI with confirmation
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
    console.log(`✅ Agent ${agentId} deleted via UI`);
  } else {
    // Row should still be present
    await expect(page.locator(`tr[data-agent-id="${agentId}"]`)).toHaveCount(1);
    console.log(`✅ Agent ${agentId} deletion cancelled`);
  }
}

/**
 * Delete agent via API
 */
export async function deleteAgentViaAPI(workerId: string, agentId: string): Promise<void> {
  const apiClient = createApiClient(workerId);
  
  try {
    await apiClient.deleteAgent(agentId);
    console.log(`✅ Agent ${agentId} deleted via API`);
  } catch (error) {
    console.warn(`Failed to delete agent ${agentId}:`, error);
    throw error;
  }
}

/**
 * Cleanup multiple agents
 */
export async function cleanupAgents(workerId: string, agents: Agent[] | string[]): Promise<void> {
  const apiClient = createApiClient(workerId);
  
  for (const agent of agents) {
    const agentId = typeof agent === 'string' ? agent : agent.id;
    
    try {
      await apiClient.deleteAgent(agentId);
      console.log(`✅ Cleaned up agent ${agentId}`);
    } catch (error) {
      console.warn(`Failed to cleanup agent ${agentId}:`, error);
    }
  }
}

/**
 * Get agent count for a worker
 */
export async function getAgentCount(workerId: string): Promise<number> {
  const apiClient = createApiClient(workerId);
  
  try {
    const agents = await apiClient.listAgents();
    return agents.length;
  } catch (error) {
    console.warn(`Failed to get agent count for worker ${workerId}:`, error);
    return 0;
  }
}

/**
 * Navigate to chat for a specific agent
 */
export async function navigateToAgentChat(page: Page, agentId: string): Promise<void> {
  await page.locator(`[data-testid="chat-agent-${agentId}"]`).click();
  
  // Wait for chat interface to load
  try {
    await page.waitForSelector('.chat-input', { timeout: 5000 });
    console.log(`✅ Navigated to chat for agent ${agentId}`);
  } catch (error) {
    console.warn(`Chat UI not fully loaded for agent ${agentId}, continuing...`);
  }
}