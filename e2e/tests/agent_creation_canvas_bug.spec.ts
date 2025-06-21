import { test, expect, Page } from './fixtures';
import { resetDatabase, waitForDashboardReady, createAgentViaUI, getAgentRowCount } from './helpers/test-helpers';

/**
 * E2E Test: Agent creation should NOT automatically add agents to canvas
 * 
 * Bug report: When users create agents in the dashboard, those agents are 
 * automatically appearing on the canvas when they switch to the canvas view.
 * This is undesired behavior - agents should only appear on canvas when 
 * explicitly dragged there by the user.
 */

async function switchToDashboard(page: Page) {
  const dashboardTab = page.getByTestId('global-dashboard-tab');
  await dashboardTab.click();
  await page.waitForSelector('#dashboard', { timeout: 10_000 });
  // Wait for dashboard to stabilize
  await page.waitForTimeout(500);
}

async function switchToCanvas(page: Page) {
  const canvasTab = page.getByTestId('global-canvas-tab');
  await canvasTab.click();
  await page.waitForSelector('#canvas-container', { timeout: 10_000 });
  
  // Wait for canvas and agent shelf to be fully initialized
  await page.waitForSelector('#agent-shelf', { timeout: 5000 });
  await page.waitForTimeout(1000);
}

async function getCanvasNodeCount(page: Page): Promise<number> {
  // Wait for canvas to stabilize after view switch
  await page.waitForTimeout(500);
  return await page.locator('.canvas-node, .generic-node').count();
}

async function waitForAgentShelfPills(page: Page, expectedCount: number, timeout: number = 5000): Promise<void> {
  await page.waitForFunction(
    (count) => {
      const pills = document.querySelectorAll('#agent-shelf .agent-pill');
      return pills.length === count;
    },
    expectedCount,
    { timeout }
  );
}

test.describe('Agent Creation Canvas Bug', () => {
  test.beforeEach(async ({ page }) => {
    // Reset database to ensure clean state
    await resetDatabase();
    await waitForDashboardReady(page);
  });

  test.afterEach(async ({ page }) => {
    // Clean up after each test
    await resetDatabase();
  });

  test('Agents created in dashboard should NOT automatically appear on canvas', async ({ page }) => {
    // Step 1: Start on dashboard and verify it's empty
    await switchToDashboard(page);
    const initialAgentCount = await getAgentRowCount(page);
    expect(initialAgentCount).toBe(0);

    // Step 2: Switch to canvas and verify it's empty (no nodes, no agent pills)
    await switchToCanvas(page);
    const initialCanvasNodes = await getCanvasNodeCount(page);
    expect(initialCanvasNodes).toBe(0);
    
    // Verify no agent pills in shelf (since no agents exist)
    const agentShelf = page.locator('#agent-shelf');
    await expect(agentShelf).toBeVisible();
    const initialPills = agentShelf.locator('.agent-pill');
    await expect(initialPills).toHaveCount(0);

    // Step 3: Switch back to dashboard and create a new agent
    await switchToDashboard(page);
    const agentId = await createAgentViaUI(page);
    
    // Verify agent was created in dashboard
    const agentCountAfterCreation = await getAgentRowCount(page);
    expect(agentCountAfterCreation).toBe(1);
    
    // Verify the specific agent row exists
    await expect(page.locator(`tr[data-agent-id="${agentId}"]`)).toBeVisible();

    // Step 4: Switch to canvas and verify the agent does NOT automatically appear as a node
    await switchToCanvas(page);
    
    // Wait for agent shelf to update with the new agent
    await waitForAgentShelfPills(page, 1);
    
    const canvasNodesAfterAgentCreation = await getCanvasNodeCount(page);
    
    // This is the key assertion - canvas should still be empty (no nodes)
    expect(canvasNodesAfterAgentCreation).toBe(0);
    
    // Additional verification: check that no agent nodes are present
    await expect(page.locator('.canvas-node')).toHaveCount(0);
    await expect(page.locator('.generic-node')).toHaveCount(0);
    
    // But verify agent shelf now shows the agent as available for dragging
    const agentPills = agentShelf.locator('.agent-pill');
    await expect(agentPills).toHaveCount(1);
  });

  test('Multiple agents created in dashboard should NOT appear on canvas', async ({ page }) => {
    // Start on dashboard
    await switchToDashboard(page);
    
    // Create multiple agents
    const agent1Id = await createAgentViaUI(page);
    const agent2Id = await createAgentViaUI(page);
    const agent3Id = await createAgentViaUI(page);
    
    // Verify all agents were created
    expect(await getAgentRowCount(page)).toBe(3);
    await expect(page.locator(`tr[data-agent-id="${agent1Id}"]`)).toBeVisible();
    await expect(page.locator(`tr[data-agent-id="${agent2Id}"]`)).toBeVisible();
    await expect(page.locator(`tr[data-agent-id="${agent3Id}"]`)).toBeVisible();

    // Switch to canvas
    await switchToCanvas(page);
    
    // Canvas should still be empty
    expect(await getCanvasNodeCount(page)).toBe(0);
    
    // But agent shelf should show all 3 agents as available
    const agentShelf = page.locator('#agent-shelf');
    await expect(agentShelf).toBeVisible();
    const agentPills = agentShelf.locator('.agent-pill');
    await expect(agentPills).toHaveCount(3);
  });

  test('Agents only appear on canvas when explicitly dragged from shelf', async ({ page }) => {
    // Create an agent in dashboard
    await switchToDashboard(page);
    const agentId = await createAgentViaUI(page);
    expect(await getAgentRowCount(page)).toBe(1);

    // Switch to canvas - should be empty
    await switchToCanvas(page);
    expect(await getCanvasNodeCount(page)).toBe(0);

    // Now explicitly drag agent from shelf to canvas
    const agentPill = page.locator('#agent-shelf .agent-pill').first();
    await expect(agentPill).toBeVisible();
    
    const canvasArea = page.locator('#canvas-container canvas');
    await expect(canvasArea).toBeVisible();
    
    // Perform the drag operation
    await agentPill.dragTo(canvasArea, { 
      targetPosition: { x: 200, y: 200 } 
    });
    
    // Now canvas should have exactly one node
    await expect(page.locator('.canvas-node, .generic-node')).toHaveCount(1, { timeout: 5000 });
    expect(await getCanvasNodeCount(page)).toBe(1);
  });

  test('Canvas state persists correctly after view switches', async ({ page }) => {
    // Create agent in dashboard
    await switchToDashboard(page);
    const agentId = await createAgentViaUI(page);
    
    // Switch to canvas and drag agent
    await switchToCanvas(page);
    const agentPill = page.locator('#agent-shelf .agent-pill').first();
    const canvasArea = page.locator('#canvas-container canvas');
    await agentPill.dragTo(canvasArea, { targetPosition: { x: 150, y: 150 } });
    
    // Verify node was added
    await expect(page.locator('.canvas-node, .generic-node')).toHaveCount(1, { timeout: 5000 });
    
    // Switch back to dashboard, then back to canvas
    await switchToDashboard(page);
    await switchToCanvas(page);
    
    // Canvas should still have the one node we dragged (persistence)
    expect(await getCanvasNodeCount(page)).toBe(1);
    
    // Create another agent in dashboard
    await switchToDashboard(page);
    await createAgentViaUI(page);
    expect(await getAgentRowCount(page)).toBe(2);
    
    // Switch back to canvas
    await switchToCanvas(page);
    
    // Should still have only the one node we explicitly added
    // The newly created agent should NOT have auto-appeared
    expect(await getCanvasNodeCount(page)).toBe(1);
    
    // But shelf should show both agents available
    const agentPills = page.locator('#agent-shelf .agent-pill');
    await expect(agentPills).toHaveCount(2, { timeout: 5000 });
  });

  test('Agent deletion in dashboard removes from shelf but not canvas nodes', async ({ page }) => {
    // Create two agents in dashboard
    await switchToDashboard(page);
    const agent1Id = await createAgentViaUI(page);
    const agent2Id = await createAgentViaUI(page);
    expect(await getAgentRowCount(page)).toBe(2);
    
    // Switch to canvas and drag both agents
    await switchToCanvas(page);
    const agentPills = page.locator('#agent-shelf .agent-pill');
    const canvasArea = page.locator('#canvas-container canvas');
    
    await agentPills.first().dragTo(canvasArea, { targetPosition: { x: 100, y: 100 } });
    await agentPills.nth(1).dragTo(canvasArea, { targetPosition: { x: 300, y: 100 } });
    
    // Verify both nodes are on canvas
    await expect(page.locator('.canvas-node, .generic-node')).toHaveCount(2, { timeout: 5000 });
    
    // Delete one agent from dashboard
    await switchToDashboard(page);
    
    // Set up dialog handler to confirm deletion
    page.once('dialog', (dialog) => dialog.accept());
    await page.locator(`[data-testid="delete-agent-${agent1Id}"]`).click();
    
    // Verify agent was deleted from dashboard
    await expect(page.locator(`tr[data-agent-id="${agent1Id}"]`)).toHaveCount(0);
    expect(await getAgentRowCount(page)).toBe(1);
    
    // Switch back to canvas
    await switchToCanvas(page);
    
    // Canvas nodes should persist even though source agent was deleted
    // This tests that canvas maintains node state independently of agent existence
    expect(await getCanvasNodeCount(page)).toBe(2);
    
    // But shelf should only show the remaining agent
    await expect(page.locator('#agent-shelf .agent-pill')).toHaveCount(1, { timeout: 5000 });
  });
});