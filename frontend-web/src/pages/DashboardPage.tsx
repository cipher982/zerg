import { ChangeEvent, useEffect, useMemo, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "react-router-dom";
import {
  createAgent,
  fetchAgents,
  fetchAgentRuns,
  resetAgent,
  runAgent,
  type AgentRun,
  type AgentSummary,
} from "../services/api";
import "../styles/dashboard.css";

type AgentMetrics = {
  successRate: number;
  runCount: number;
  lastRunStatus: AgentRun["status"] | null;
};

type BulkActionSummary = {
  completed: number;
  failed: number;
  errors: Array<{ id: number; message: string }>;
};

type ActionFeedback = {
  message: string;
  variant: "success" | "error" | "info";
};

const EMPTY_METRICS: AgentMetrics = {
  successRate: 0,
  runCount: 0,
  lastRunStatus: null,
};

export default function DashboardPage() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [scope, setScope] = useState<"my" | "all">("my");
  const [searchTerm, setSearchTerm] = useState("");
  const [selectedAgentIds, setSelectedAgentIds] = useState<Set<number>>(new Set());
  const [actionFeedback, setActionFeedback] = useState<ActionFeedback | null>(null);
  const selectAllRef = useRef<HTMLInputElement | null>(null);

  const { data, isLoading, error, refetch } = useQuery<AgentSummary[]>({
    queryKey: ["agents", { scope }],
    queryFn: () => fetchAgents({ scope }),
    refetchInterval: 2000, // Poll every 2 seconds for test environments
  });

  const agents: AgentSummary[] = useMemo(() => data ?? [], [data]);
  const agentIds = useMemo(() => {
    return agents.map((agent) => agent.id).sort((a, b) => a - b);
  }, [agents]);

  const { data: agentMetrics = {}, isLoading: isMetricsLoading } = useQuery<Record<number, AgentMetrics>>({
    queryKey: ["agent-metrics", scope, agentIds],
    enabled: agentIds.length > 0,
    queryFn: async () => {
      const results = await Promise.all(
        agentIds.map(async (id) => {
          try {
            const runs = await fetchAgentRuns(id, 20);
            return { id, metrics: computeAgentMetrics(runs) };
          } catch (err) {
            console.error(`Failed to fetch runs for agent ${id}`, err);
            return { id, metrics: { ...EMPTY_METRICS } };
          }
        })
      );

      return results.reduce<Record<number, AgentMetrics>>((acc, entry) => {
        acc[entry.id] = entry.metrics;
        return acc;
      }, {});
    },
    staleTime: 5000,
    gcTime: 1000 * 60 * 5,
  });

  const filteredAgents = useMemo(() => {
    if (!searchTerm.trim()) {
      return agents;
    }

    const term = searchTerm.trim().toLowerCase();
    return agents.filter((agent) => {
      const nameMatch = agent.name.toLowerCase().includes(term);
      const ownerLabel = agent.owner?.display_name || agent.owner?.email;
      const ownerMatch = ownerLabel ? ownerLabel.toLowerCase().includes(term) : false;
      const statusMatch = agent.status.toLowerCase().includes(term);
      return nameMatch || ownerMatch || statusMatch;
    });
  }, [agents, searchTerm]);

  const dashboardStats = useMemo(() => {
    const total = agents.length;
    const running = agents.filter((agent) => agent.status === "running" || agent.status === "processing").length;
    const errored = agents.filter((agent) => agent.status === "error").length;

    return {
      total,
      running,
      errored,
    };
  }, [agents]);

  const handleScopeToggle = (event: ChangeEvent<HTMLInputElement>) => {
    setScope(event.target.checked ? "all" : "my");
  };

  const handleSearchChange = (event: ChangeEvent<HTMLInputElement>) => {
    setSearchTerm(event.target.value);
  };

  const clearSearch = () => setSearchTerm("");

  const selectedCount = selectedAgentIds.size;
  const allVisibleSelected =
    filteredAgents.length > 0 && filteredAgents.every((agent) => selectedAgentIds.has(agent.id));

  const handleToggleAll = (checked: boolean) => {
    if (!filteredAgents.length) {
      setSelectedAgentIds(new Set());
      return;
    }
    if (checked) {
      setSelectedAgentIds(new Set(filteredAgents.map((agent) => agent.id)));
    } else {
      setSelectedAgentIds(new Set());
    }
  };

  const handleSelectAgent = (agentId: number, checked: boolean) => {
    setSelectedAgentIds((prev) => {
      const next = new Set(prev);
      if (checked) {
        next.add(agentId);
      } else {
        next.delete(agentId);
      }
      return next;
    });
  };

  const handleBulkCompletion = (action: "rerun" | "reset", summary: BulkActionSummary) => {
    const { completed, failed, errors } = summary;
    if (failed === 0) {
      const verb = action === "rerun" ? "Reran" : "Reset";
      setActionFeedback({
        variant: "success",
        message: `${verb} ${completed} agent${completed === 1 ? "" : "s"}.`,
      });
    } else {
      const verb = action === "rerun" ? "rerunning" : "resetting";
      const details = errors.length
        ? ` Errors: ${errors.map((entry) => `${entry.id}`).join(", ")}.`
        : "";
      setActionFeedback({
        variant: "error",
        message: `Finished ${verb} ${completed} agent${completed === 1 ? "" : "s"}; ${failed} failed.${details}`,
      });
    }
  };

  const runSelectedMutation = useMutation({
    mutationFn: (ids: number[]) => executeBulk(ids, runAgent),
    onSuccess: (summary) => {
      queryClient.invalidateQueries({ queryKey: ["agents"] });
      queryClient.invalidateQueries({ queryKey: ["agent-metrics"] });
      setSelectedAgentIds(new Set());
      handleBulkCompletion("rerun", summary);
    },
    onError: (err: unknown) => {
      setActionFeedback({
        variant: "error",
        message: `Failed to trigger rerun: ${extractErrorMessage(err)}`,
      });
    },
  });

  const resetSelectedMutation = useMutation({
    mutationFn: (ids: number[]) => executeBulk(ids, resetAgent),
    onSuccess: (summary) => {
      queryClient.invalidateQueries({ queryKey: ["agents"] });
      queryClient.invalidateQueries({ queryKey: ["agent-metrics"] });
      setSelectedAgentIds(new Set());
      handleBulkCompletion("reset", summary);
    },
    onError: (err: unknown) => {
      setActionFeedback({
        variant: "error",
        message: `Failed to reset agents: ${extractErrorMessage(err)}`,
      });
    },
  });

  const handleBulkRun = () => {
    if (selectedCount === 0) {
      setActionFeedback({ variant: "info", message: "Select at least one agent first." });
      return;
    }
    setActionFeedback(null);
    runSelectedMutation.mutate(Array.from(selectedAgentIds));
  };

  const handleBulkReset = () => {
    if (selectedCount === 0) {
      setActionFeedback({ variant: "info", message: "Select at least one agent first." });
      return;
    }
    setActionFeedback(null);
    resetSelectedMutation.mutate(Array.from(selectedAgentIds));
  };

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
      queryClient.invalidateQueries({ queryKey: ["agent-metrics"] });
    },
  });

  useEffect(() => {
    if (!error) {
      return;
    }

    if (error instanceof Error && error.message.includes("(403)")) {
      // Fall back to the personal scope if the user cannot view all agents.
      setScope("my");
    }
  }, [error]);

  useEffect(() => {
    setSelectedAgentIds((prev) => {
      if (prev.size === 0) {
        return prev;
      }
      const allowed = new Set(agentIds);
      const next = new Set<number>();
      prev.forEach((id) => {
        if (allowed.has(id)) {
          next.add(id);
        }
      });
      if (next.size === prev.size) {
        return prev;
      }
      return next;
    });
  }, [agentIds]);

  useEffect(() => {
    const checkbox = selectAllRef.current;
    if (!checkbox) {
      return;
    }
    const totalVisible = filteredAgents.length;
    const selectedVisible = filteredAgents.filter((agent) => selectedAgentIds.has(agent.id)).length;
    checkbox.indeterminate = selectedVisible > 0 && selectedVisible < totalVisible;
  }, [filteredAgents, selectedAgentIds]);

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }
    const wsBase =
      (window as typeof window & { WS_BASE_URL?: string }).WS_BASE_URL ||
      window.location.origin.replace("http", "ws");
    const wsUrl = new URL("/api/ws", wsBase);

    // Add test worker header to WebSocket URL for E2E test isolation
    const testWorkerHeader = window.__TEST_WORKER_ID__;
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
    const message = error instanceof Error ? error.message : "Failed to load agents";
    return (
      <div id="dashboard-container" className="dashboard-container">
        <div id="dashboard" className="dashboard">
          <div>{message}</div>
        </div>
      </div>
    );
  }

  const totalColumns = scope === "all" ? 8 : 7;

  return (
    <div id="dashboard-container" className="dashboard-container">
      <div id="dashboard" className="dashboard">
        <header className="dashboard-header">
          <div className="dashboard-header-left">
            <h1>Agent Dashboard</h1>
            <div className="scope-wrapper" role="group" aria-labelledby="dashboard-scope-label">
              <span id="dashboard-scope-label" className="scope-text-label">
                {scope === "all" ? "All agents" : "My agents"}
              </span>
              <label className="scope-toggle">
                <input
                  id="dashboard-scope-toggle"
                  data-testid="dashboard-scope-toggle"
                  type="checkbox"
                  checked={scope === "all"}
                  onChange={handleScopeToggle}
                  aria-label="Toggle between my agents and all agents"
                />
                <span className="slider" aria-hidden="true" />
              </label>
            </div>
          </div>

          <div className="dashboard-header-right">
            <div className="search-container" role="search">
              <span className="search-icon" aria-hidden="true">🔍</span>
              <input
                id="agent-search"
                type="search"
                placeholder="Search agents or owners"
                value={searchTerm}
                onChange={handleSearchChange}
                aria-label="Search agents"
              />
              {searchTerm && (
                <button
                  type="button"
                  className="search-clear"
                  onClick={clearSearch}
                  aria-label="Clear search"
                >
                  ×
                </button>
              )}
            </div>

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
          </div>
        </header>

        <section className="dashboard-stats" aria-label="Agent statistics">
          <div className="dashboard-stat-card">
            <span className="dashboard-stat-label">Total agents</span>
            <span className="dashboard-stat-value">{dashboardStats.total}</span>
          </div>
          <div className="dashboard-stat-card">
            <span className="dashboard-stat-label">Running</span>
            <span className="dashboard-stat-value">{dashboardStats.running}</span>
          </div>
          <div className="dashboard-stat-card">
            <span className="dashboard-stat-label">Errors</span>
            <span className="dashboard-stat-value">{dashboardStats.errored}</span>
          </div>
        </section>

        <section className="dashboard-bulk-actions" aria-label="Bulk actions">
          <div className="bulk-selection-info">
            {selectedCount === 0
              ? "No agents selected"
              : `${selectedCount} agent${selectedCount === 1 ? "" : "s"} selected`}
          </div>
          <div className="bulk-actions-buttons">
            <button
              type="button"
              className="bulk-action-button"
              onClick={handleBulkRun}
              disabled={selectedCount === 0 || runSelectedMutation.isPending}
            >
              {runSelectedMutation.isPending ? "Rerunning…" : "Rerun selected"}
            </button>
            <button
              type="button"
              className="bulk-action-button"
              onClick={handleBulkReset}
              disabled={selectedCount === 0 || resetSelectedMutation.isPending}
            >
              {resetSelectedMutation.isPending ? "Resetting…" : "Reset status"}
            </button>
          </div>
        </section>

        {actionFeedback && (
          <div className={`bulk-feedback bulk-feedback-${actionFeedback.variant}`} role="status">
            {actionFeedback.message}
          </div>
        )}

        <section>
          <table id="agents-table" className="agents-table">
            <thead>
              <tr>
                <th scope="col" className="checkbox-col">
                  <input
                    ref={selectAllRef}
                    type="checkbox"
                    aria-label="Select all agents"
                    checked={filteredAgents.length > 0 && allVisibleSelected}
                    onChange={(event) => handleToggleAll(event.target.checked)}
                  />
                </th>
                <th scope="col">Name</th>
                {scope === "all" && <th scope="col">Owner</th>}
                <th scope="col">Status</th>
                <th scope="col">Last Run</th>
                <th scope="col">Next Run</th>
                <th scope="col">Success Rate</th>
                <th scope="col" className="actions-header">
                  Actions
                </th>
              </tr>
            </thead>
            <tbody id="agents-table-body">
              {filteredAgents.map((agent) => {
                const metrics = agentMetrics[agent.id];
                return (
                  <tr key={agent.id} data-agent-id={agent.id}>
                    <td className="checkbox-cell" data-label="Select">
                      <input
                        type="checkbox"
                        aria-label={`Select agent ${agent.name}`}
                        checked={selectedAgentIds.has(agent.id)}
                        onChange={(event) => handleSelectAgent(agent.id, event.target.checked)}
                      />
                    </td>
                    <td data-label="Name">{agent.name}</td>
                  {scope === "all" && (
                    <td className="owner-cell" data-label="Owner">
                      {agent.owner ? (
                        <span className="owner-wrapper">
                          {agent.owner.avatar_url && (
                            <img
                              src={agent.owner.avatar_url}
                              alt=""
                              className="owner-avatar"
                              aria-hidden="true"
                            />
                          )}
                          <span>
                            {agent.owner.display_name?.trim() || agent.owner.email}
                          </span>
                        </span>
                      ) : (
                        <span>—</span>
                      )}
                    </td>
                  )}
                    <td data-label="Status">
                      <span className={`status-indicator status-${agent.status.toLowerCase()}`}>
                        {formatStatus(agent.status)}
                      </span>
                      {metrics?.lastRunStatus && (
                        <span
                          className={`last-run-indicator ${metrics.lastRunStatus === "success" ? "last-run-success" : metrics.lastRunStatus === "failed" ? "last-run-failure" : ""}`.trim()}
                        >
                          {formatLastRunStatus(metrics.lastRunStatus)}
                        </span>
                      )}
                      {agent.last_error && agent.last_error.trim() && (
                        <span className="last-error-indicator" title={agent.last_error}>
                          ⚠
                        </span>
                      )}
                    </td>
                    <td data-label="Last Run">{formatTimestamp(agent.last_run_at)}</td>
                    <td data-label="Next Run">{formatTimestamp(agent.next_run_at)}</td>
                    <td data-label="Success Rate">
                      {metrics
                        ? formatSuccessRate(metrics)
                        : isMetricsLoading
                        ? "Loading…"
                        : "—"}
                    </td>
                    <td className="actions-cell" data-label="Actions">
                      <button
                        type="button"
                        data-testid={`chat-agent-${agent.id}`}
                        onClick={() => navigate(`/chat/${agent.id}`)}
                      >
                        Open Chat
                      </button>
                    </td>
                  </tr>
                );
              })}
              {agents.length === 0 && (
                <tr>
                  <td colSpan={totalColumns}>No agents yet.</td>
                </tr>
              )}
              {agents.length > 0 && filteredAgents.length === 0 && (
                <tr>
                  <td colSpan={totalColumns}>No agents match your search.</td>
                </tr>
              )}
            </tbody>
          </table>
        </section>
      </div>
    </div>
  );
}

function formatStatus(status: string): string {
  if (!status) {
    return "Unknown";
  }

  return status.charAt(0).toUpperCase() + status.slice(1);
}

function formatTimestamp(timestamp?: string | null): string {
  if (!timestamp) {
    return "—";
  }

  const date = new Date(timestamp);
  if (Number.isNaN(date.getTime())) {
    return "—";
  }

  return date.toLocaleString();
}

function formatSuccessRate(metrics: AgentMetrics): string {
  if (!metrics || metrics.runCount === 0) {
    return "—";
  }
  return `${Math.round(metrics.successRate)}% (${metrics.runCount})`;
}

function formatLastRunStatus(status: AgentRun["status"]): string {
  switch (status) {
    case "success":
      return "Last: ✓";
    case "failed":
      return "Last: ✗";
    case "running":
      return "Last: running";
    case "queued":
      return "Last: queued";
    default:
      return `Last: ${status}`;
  }
}

function computeAgentMetrics(runs: AgentRun[]): AgentMetrics {
  if (!runs || runs.length === 0) {
    return { ...EMPTY_METRICS };
  }

  const runCount = runs.length;
  const successCount = runs.filter((run) => run.status === "success").length;
  const lastRun = runs[0];

  return {
    successRate: runCount > 0 ? (successCount / runCount) * 100 : 0,
    runCount,
    lastRunStatus: lastRun?.status ?? null,
  };
}

async function executeBulk<T>(ids: number[], handler: (id: number) => Promise<T>): Promise<BulkActionSummary> {
  const results = await Promise.allSettled(ids.map((id) => handler(id)));
  const summary: BulkActionSummary = { completed: 0, failed: 0, errors: [] };

  results.forEach((result, index) => {
    const id = ids[index];
    if (result.status === "fulfilled") {
      summary.completed += 1;
    } else {
      summary.failed += 1;
      summary.errors.push({ id, message: extractErrorMessage(result.reason) });
    }
  });

  return summary;
}

function extractErrorMessage(err: unknown): string {
  if (err instanceof Error) {
    return err.message;
  }
  if (typeof err === "string") {
    return err;
  }
  try {
    return JSON.stringify(err);
  } catch {
    return "Unknown error";
  }
}
