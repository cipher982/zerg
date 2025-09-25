import { useEffect, useMemo } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "react-router-dom";
import { createAgent, fetchAgents, type AgentSummary } from "../services/api";

export default function DashboardPage() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { data, isLoading, error, refetch } = useQuery<AgentSummary[]>({
    queryKey: ["agents"],
    queryFn: fetchAgents,
    refetchInterval: 2000, // Poll every 2 seconds for test environments
  });

  const agents: AgentSummary[] = useMemo(() => data ?? [], [data]);

  const createAgentMutation = useMutation({
    mutationFn: async () => {
      const name = window.prompt("Agent name", "E2E Agent") ?? "E2E Agent";
      const trimmedName = name.trim() || "E2E Agent";
      return createAgent({
        name: trimmedName,
        system_instructions: "You are a helpful assistant.",
        task_instructions: "Respond concisely to user requests.",
        model: "gpt-4o",
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["agents"] });
    },
  });

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }
    const wsBase =
      (window as typeof window & { WS_BASE_URL?: string }).WS_BASE_URL ||
      window.location.origin.replace("http", "ws");
    const wsUrl = new URL("/api/ws", wsBase);

    // Add test worker header to WebSocket URL for E2E test isolation
    const testWorkerHeader = (window as any).__TEST_WORKER_ID__;
    if (testWorkerHeader !== undefined) {
      wsUrl.searchParams.set("worker", String(testWorkerHeader));
    }

    const ws = new WebSocket(wsUrl.toString());
    ws.onmessage = () => {
      // TODO: hydrate from typed events.
      refetch();
    };
    return () => ws.close();
  }, [refetch]);

  if (isLoading) {
    return (
      <div id="dashboard-container" className="dashboard-container">
        <div id="dashboard" className="dashboard">
          <div>Loading agents…</div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div id="dashboard-container" className="dashboard-container">
        <div id="dashboard" className="dashboard">
          <div>Failed to load agents</div>
        </div>
      </div>
    );
  }

  return (
    <div id="dashboard-container" className="dashboard-container">
      <div id="dashboard" className="dashboard">
        <header className="dashboard-header">
          <h1>Agent Dashboard</h1>
          <div className="button-container">
            <button
              id="create-agent-button"
              type="button"
              className="create-agent-button create-agent-btn"
              data-testid="create-agent-btn"
              onClick={() => createAgentMutation.mutate()}
              disabled={createAgentMutation.isPending}
            >
              {createAgentMutation.isPending ? "Creating…" : "Create Agent"}
            </button>
          </div>
        </header>

        <section>
          <table id="agents-table" className="agents-table">
            <thead>
              <tr>
                <th scope="col">Name</th>
                <th scope="col">Status</th>
                <th scope="col" className="actions-header">
                  Actions
                </th>
              </tr>
            </thead>
            <tbody id="agents-table-body">
              {agents.map((agent) => (
                <tr key={agent.id} data-agent-id={agent.id}>
                  <td>{agent.name}</td>
                  <td>{agent.status}</td>
                  <td>
                    <button
                      type="button"
                      data-testid={`chat-agent-${agent.id}`}
                      onClick={() => navigate(`/chat/${agent.id}`)}
                    >
                      Open Chat
                    </button>
                  </td>
                </tr>
              ))}
              {agents.length === 0 && (
                <tr>
                  <td colSpan={3}>No agents yet.</td>
                </tr>
              )}
            </tbody>
          </table>
        </section>
      </div>
    </div>
  );
}
