import React from "react";
import { describe, beforeAll, afterAll, beforeEach, test, expect, vi } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";
import DashboardPage from "../DashboardPage";
import {
  fetchAgents,
  createAgent,
  fetchAgentRuns,
  resetAgent,
  runAgent,
  type AgentSummary,
  type AgentRun,
} from "../../services/api";

vi.mock("../../services/api", () => ({
  fetchAgents: vi.fn(),
  createAgent: vi.fn(),
  fetchAgentRuns: vi.fn(),
  resetAgent: vi.fn(),
  runAgent: vi.fn(),
}));

type MockWebSocketInstance = {
  onmessage: ((event: MessageEvent) => void) | null;
  close: () => void;
};

function buildAgent(
  overrides: Partial<AgentSummary> & Pick<AgentSummary, "id" | "name" | "status" | "owner_id">
): AgentSummary {
  const now = new Date().toISOString();
  return {
    id: overrides.id,
    name: overrides.name,
    status: overrides.status,
    owner_id: overrides.owner_id,
    owner: overrides.owner ?? null,
    system_instructions: overrides.system_instructions ?? "",
    task_instructions: overrides.task_instructions ?? "",
    model: overrides.model ?? "gpt-4o",
    schedule: overrides.schedule ?? null,
    config: overrides.config ?? null,
    last_error: overrides.last_error ?? null,
    allowed_tools: overrides.allowed_tools ?? [],
    created_at: overrides.created_at ?? now,
    updated_at: overrides.updated_at ?? now,
    messages: overrides.messages ?? [],
    next_run_at: overrides.next_run_at ?? null,
    last_run_at: overrides.last_run_at ?? null,
  };
}

describe("DashboardPage", () => {
  const fetchAgentsMock = fetchAgents as unknown as vi.MockedFunction<typeof fetchAgents>;
  const createAgentMock = createAgent as unknown as vi.MockedFunction<typeof createAgent>;
  const fetchAgentRunsMock = fetchAgentRuns as unknown as vi.MockedFunction<typeof fetchAgentRuns>;
  const resetAgentMock = resetAgent as unknown as vi.MockedFunction<typeof resetAgent>;
  const runAgentMock = runAgent as unknown as vi.MockedFunction<typeof runAgent>;
  const mockSockets: MockWebSocketInstance[] = [];

  beforeAll(() => {
    class MockWebSocket {
      public onmessage: ((event: MessageEvent) => void) | null = null;

      constructor() {
        mockSockets.push(this);
      }

      close() {
        // no-op
      }
    }

    vi.stubGlobal("WebSocket", MockWebSocket as unknown as typeof WebSocket);
  });

  afterAll(() => {
    vi.unstubAllGlobals();
  });

  beforeEach(() => {
    mockSockets.length = 0;
    fetchAgentsMock.mockReset();
    createAgentMock.mockReset();
    fetchAgentRunsMock.mockReset();
    resetAgentMock.mockReset();
    runAgentMock.mockReset();
    fetchAgentRunsMock.mockResolvedValue([]);
    resetAgentMock.mockResolvedValue({} as Awaited<ReturnType<typeof resetAgent>>);
    runAgentMock.mockResolvedValue(undefined);
  });

  function renderDashboard(initialAgents: AgentSummary[], runsByAgent?: Record<number, AgentRun[]>) {
    fetchAgentsMock.mockResolvedValue(initialAgents);
    const runsLookup = runsByAgent ?? {};
    fetchAgentRunsMock.mockImplementation(async (agentId: number) => runsLookup[agentId] ?? []);

    const queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false, cacheTime: 0 },
      },
    });

    return render(
      <QueryClientProvider client={queryClient}>
        <MemoryRouter>
          <DashboardPage />
        </MemoryRouter>
      </QueryClientProvider>
    );
  }

  test("renders agents, metrics, and dashboard stats", async () => {
    const agents: AgentSummary[] = [
      buildAgent({
        id: 1,
        name: "Alpha",
        status: "running",
        owner_id: 7,
        owner: { id: 7, email: "alpha@example.com", display_name: "Ada", is_active: true, created_at: new Date().toISOString(), avatar_url: null, prefs: {} },
        last_run_at: "2025-09-24T10:00:00.000Z",
        next_run_at: "2025-09-24T12:00:00.000Z",
      }),
      buildAgent({
        id: 2,
        name: "Beta",
        status: "error",
        owner_id: 9,
        owner: { id: 9, email: "beta@example.com", display_name: null, is_active: true, created_at: new Date().toISOString(), avatar_url: null, prefs: {} },
        last_run_at: null,
        next_run_at: null,
        last_error: "Failed to execute",
      }),
    ];

    const runsByAgent: Record<number, AgentRun[]> = {
      1: [
        {
          id: 11,
          agent_id: 1,
          thread_id: 101,
          status: "success",
          trigger: "manual",
          started_at: "2025-09-24T09:55:00.000Z",
          finished_at: "2025-09-24T09:56:00.000Z",
          duration_ms: 60000,
          total_tokens: null,
          total_cost_usd: null,
          error: null,
        },
        {
          id: 12,
          agent_id: 1,
          thread_id: 102,
          status: "success",
          trigger: "manual",
          started_at: "2025-09-24T09:00:00.000Z",
          finished_at: "2025-09-24T09:01:00.000Z",
          duration_ms: 60000,
          total_tokens: null,
          total_cost_usd: null,
          error: null,
        },
      ],
      2: [
        {
          id: 21,
          agent_id: 2,
          thread_id: 201,
          status: "failed",
          trigger: "manual",
          started_at: "2025-09-24T08:30:00.000Z",
          finished_at: "2025-09-24T08:31:00.000Z",
          duration_ms: 60000,
          total_tokens: null,
          total_cost_usd: null,
          error: "Failed",
        },
      ],
    };

    renderDashboard(agents, runsByAgent);

    const totalStat = await screen.findByText("Total agents");
    expect(totalStat).toBeInTheDocument();

    const totalCard = totalStat.closest(".dashboard-stat-card");
    expect(totalCard).not.toBeNull();
    expect(totalCard && within(totalCard).getByText("2")).toBeInTheDocument();

    const statsSection = screen.getByLabelText("Agent statistics");
    expect(within(statsSection).getByText("Running")).toBeInTheDocument();
    expect(within(statsSection).getByText("Errors")).toBeInTheDocument();

    expect(await screen.findByText("100% (2)")).toBeInTheDocument();
    expect(screen.getByText("0% (1)")).toBeInTheDocument();
    expect(screen.getByText("Last: ✓")).toBeInTheDocument();
    expect(screen.getByText("Last: ✗")).toBeInTheDocument();

    const betaRow = await screen.findByText("Beta");
    expect(betaRow).toBeInTheDocument();
    expect(fetchAgentsMock).toHaveBeenCalledWith({ scope: "my" });
    expect(fetchAgentRunsMock).toHaveBeenCalledWith(1, 20);

    const searchInput = screen.getByLabelText("Search agents");
    await userEvent.type(searchInput, "Alpha");
    expect(screen.getByText("Alpha")).toBeInTheDocument();
    expect(screen.queryByText("Beta")).not.toBeInTheDocument();

    await userEvent.clear(searchInput);
    await userEvent.type(searchInput, "Gamma");
    expect(screen.getByText("No agents match your search.")).toBeInTheDocument();
  });

  test("toggling scope requests all agents", async () => {
    const agents: AgentSummary[] = [
      buildAgent({
        id: 1,
        name: "Alpha",
        status: "running",
        owner_id: 7,
        owner: { id: 7, email: "alpha@example.com", display_name: "Ada", is_active: true, created_at: new Date().toISOString(), avatar_url: null, prefs: {} },
      }),
    ];

    renderDashboard(agents);

    const scopeToggle = await screen.findByLabelText("Toggle between my agents and all agents");
    await userEvent.click(scopeToggle);

    expect(fetchAgentsMock).toHaveBeenCalledWith({ scope: "my" });
    expect(fetchAgentsMock).toHaveBeenLastCalledWith({ scope: "all" });
    expect(fetchAgentRunsMock).toHaveBeenCalledWith(1, 20);
  });
});
