import { useEffect, useMemo } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "react-router-dom";
import { createAgent, fetchAgents, type AgentSummary } from "../services/api";

export default function DashboardPage() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ["agents"],
    queryFn: fetchAgents,
    refetchInterval: 2000, // Poll every 2 seconds for test environments
  });

  const agents: AgentSummary[] = useMemo(() => {
    return data ?? [];
  }, [data]);

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
    // Add test worker header to WebSocket URL for E2E test isolation
    const testWorkerHeader = (window as any).__TEST_WORKER_ID__;
    let wsUrl = `${window.location.origin.replace("http", "ws")}/api/ws`;
    if (testWorkerHeader !== undefined) {
      wsUrl += `?worker=${testWorkerHeader}`;
    }

    const ws = new WebSocket(wsUrl);
    ws.onmessage = () => {
      // TODO: hydrate from typed events.
      refetch();
    };
    return () => ws.close();
  }, [refetch]);

  if (isLoading) {
    return <div>Loading agents…</div>;
  }

  if (error) {
    return <div>Failed to load agents</div>;
  }

  return (
    <div>
      <header>
        <h1>Agent Dashboard (React Prototype)</h1>
        <button
          type="button"
          onClick={() => createAgentMutation.mutate()}
          disabled={createAgentMutation.isPending}
          data-testid="create-agent-btn"
        >
          {createAgentMutation.isPending ? "Creating…" : "Create Agent"}
        </button>
      </header>
      <section>
        <table className="agent-table">
          <thead>
            <tr>
              <th>Name</th>
              <th>Status</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
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
          </tbody>
        </table>
      </section>
    </div>
  );
}
