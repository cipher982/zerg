import { test, expect } from './fixtures';
import { 
  setupTestData, 
  cleanupTestData, 
  waitForDashboardReady, 
  getAgentRowCount,
  createAgentViaUI,
  editAgentViaUI,
  deleteAgentViaUI,
  checkBackendHealth,
  resetDatabase,
  TestContext
} from './helpers/test-helpers';
import { apiClient } from './helpers/api-client';

test.describe('Smoke Tests - Basic Infrastructure', () => {
  let testContext: TestContext;

  test.beforeEach(async () => {
    // Reset database and verify clean state
    await resetDatabase();
    const isHealthy = await checkBackendHealth();
    expect(isHealthy).toBe(true);
    
    // Verify empty state
    testContext = { agents: [], threads: [] };
    const agentCount = await apiClient.listAgents();
    expect(agentCount).toHaveLength(0);
  });

  test.afterEach(async () => {
    // Clean up any test data
    await cleanupTestData(testContext);
  });

  test('Backend API is responding correctly', async () => {
    const health = await checkBackendHealth();
    expect(health).toBe(true);
  });

  test('Dashboard loads with clean database', async ({ page }) => {
    await waitForDashboardReady(page);
    
    // Verify UI matches empty database state
    await expect(page.locator('table')).toBeVisible();
    const agentCount = await getAgentRowCount(page);
    expect(agentCount).toBe(0);
    // Check for the empty state message in the table
    await expect(page.locator('table')).toContainText("No agents found. Click 'Create New Agent' to get started.");
    
    // Dashboard should still show the table structure
    await expect(page.locator('table')).toBeVisible();
  });

  test('Can create agent via API and see it in UI', async ({ page }) => {
    // Create agent via API
    testContext = await setupTestData({
      agents: [{
        name: 'API Test Agent',
        model: 'gpt-mock',
        system_instructions: 'You are a test agent created via API.'
      }]
    });

    expect(testContext.agents).toHaveLength(1);
    const agent = testContext.agents[0];

    // Navigate to dashboard and verify agent appears
    await waitForDashboardReady(page);
    
    const agentCount = await getAgentRowCount(page);
    expect(agentCount).toBe(1);

    // Verify the agent row contains expected data
    const agentRow = page.locator(`tr[data-agent-id="${agent.id}"]`);
    await expect(agentRow).toBeVisible();
    await expect(agentRow).toContainText('API Test Agent');
  });

  test('Can create agent via UI', async ({ page }) => {
    await waitForDashboardReady(page);
    
    const initialCount = await getAgentRowCount(page);
    
    // Create agent via UI
    const agentId = await createAgentViaUI(page);
    
    // Add to context for cleanup
    testContext.agents.push({
      id: agentId,
      name: `Test Agent ${Date.now()}`,
      model: 'gpt-mock',
      system_instructions: '',
      task_instructions: '',
      temperature: 0.7,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    });

    // Verify count increased
    const finalCount = await getAgentRowCount(page);
    expect(finalCount).toBe(initialCount + 1);

    // Verify the new agent row is present
    const agentRow = page.locator(`tr[data-agent-id="${agentId}"]`);
    await expect(agentRow).toBeVisible();
  });

  test('Can edit agent via UI', async ({ page }) => {
    // Create agent via API first
    testContext = await setupTestData({
      agents: [{
        name: 'Original Name',
        system_instructions: 'Original instructions'
      }]
    });

    const agent = testContext.agents[0];
    
    await waitForDashboardReady(page);

    // Edit the agent via UI
    await editAgentViaUI(page, agent.id, {
      name: 'Updated Name',
      systemInstructions: 'Updated instructions'
    });

    // Verify the changes are visible in the UI
    const agentRow = page.locator(`tr[data-agent-id="${agent.id}"]`);
    await expect(agentRow).toContainText('Updated Name', { timeout: 5_000 });
  });

  test('Can delete agent via UI', async ({ page }) => {
    // Create agent via API first
    testContext = await setupTestData({
      agents: [{ name: 'To Be Deleted' }]
    });

    const agent = testContext.agents[0];
    
    await waitForDashboardReady(page);

    const initialCount = await getAgentRowCount(page);
    expect(initialCount).toBe(1);

    // Delete the agent via UI
    await deleteAgentViaUI(page, agent.id, true);

    // Verify count decreased
    const finalCount = await getAgentRowCount(page);
    expect(finalCount).toBe(0);

    // Remove from context since it's already deleted
    testContext.agents = [];
  });

  test('Cancel delete keeps agent intact', async ({ page }) => {
    // Create agent via API first
    testContext = await setupTestData({
      agents: [{ name: 'Should Not Be Deleted' }]
    });

    const agent = testContext.agents[0];
    
    await waitForDashboardReady(page);

    // Attempt to delete but cancel
    await deleteAgentViaUI(page, agent.id, false);

    // Verify agent is still there
    const agentRow = page.locator(`tr[data-agent-id="${agent.id}"]`);
    await expect(agentRow).toBeVisible();
    await expect(agentRow).toContainText('Should Not Be Deleted');
  });

  test('Multiple agents can be created and managed', async ({ page }) => {
    // Create multiple agents via API
    testContext = await setupTestData({
      agents: [
        { name: 'Agent 1', model: 'gpt-mock' },
        { name: 'Agent 2', model: 'gpt-4o-mini' },
        { name: 'Agent 3', system_instructions: 'Special instructions' }
      ]
    });

    await waitForDashboardReady(page);

    // Verify all agents appear
    const agentCount = await getAgentRowCount(page);
    expect(agentCount).toBe(3);

    // Verify each agent is visible
    for (const agent of testContext.agents) {
      const agentRow = page.locator(`tr[data-agent-id="${agent.id}"]`);
      await expect(agentRow).toBeVisible();
      await expect(agentRow).toContainText(agent.name);
    }
  });

  test('Dashboard handles empty state correctly', async ({ page }) => {
    // Start with completely clean database
    await waitForDashboardReady(page);

    // Should show table structure but no agent rows
    await expect(page.locator('table')).toBeVisible();
    
    const agentCount = await getAgentRowCount(page);
    expect(agentCount).toBe(0);

    // Create button should still be present and functional
    await expect(page.locator('[data-testid="create-agent-btn"]')).toBeVisible();
  });
});
