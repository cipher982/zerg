import { Fragment, useEffect, useMemo, useState, type KeyboardEvent as ReactKeyboardEvent, type MouseEvent as ReactMouseEvent } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "react-router-dom";
import {
  createAgent,
  fetchAgents,
  fetchAgentRuns,
  runAgent,
  type AgentRun,
  type AgentSummary,
} from "../services/api";
import { useWebSocket } from "../lib/useWebSocket";
import { useAuth } from "../lib/auth";
import { EditIcon, MessageCircleIcon, PlayIcon, SettingsIcon, TrashIcon } from "../components/icons";
import AgentSettingsDrawer from "../components/agent-settings/AgentSettingsDrawer";

type Scope = "my" | "all";
type SortKey = "name" | "status" | "last_run" | "next_run" | "success";

type SortConfig = {
  key: SortKey;
  ascending: boolean;
};

type AgentRunsState = Record<number, AgentRun[]>;

type LegacyAgentRow = {
  agent: AgentSummary;
  lastRunDisplay: string;
  nextRunDisplay: string;
};

const STATUS_ORDER: Record<string, number> = {
  running: 0,
  processing: 1,
  idle: 2,
  error: 3,
};

const NBSP = "\u00A0";

const STORAGE_KEY_SORT = "dashboard_sort_key";
const STORAGE_KEY_ASC = "dashboard_sort_asc";

export default function DashboardPage() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { isAuthenticated } = useAuth();
  const [scope, setScope] = useState<Scope>("my");
  const [sortConfig, setSortConfig] = useState<SortConfig>(() => loadSortConfig());
  const [expandedAgentId, setExpandedAgentId] = useState<number | null>(null);
  const [runsByAgent, setRunsByAgent] = useState<AgentRunsState>({});
  const [loadingRunIds, setLoadingRunIds] = useState<Set<number>>(new Set());
  const [expandedRunHistory, setExpandedRunHistory] = useState<Set<number>>(new Set());
  const [pendingRunIds, setPendingRunIds] = useState<Set<number>>(new Set());
  const [settingsAgentId, setSettingsAgentId] = useState<number | null>(null);

  const { data, isLoading, error, refetch } = useQuery<AgentSummary[]>({
    queryKey: ["agents", { scope }],
    queryFn: () => fetchAgents({ scope }),
    refetchInterval: 2000,
  });

  const agents: AgentSummary[] = useMemo(() => data ?? [], [data]);

  useEffect(() => {
    // Persist sort preferences whenever they change.
    persistSortConfig(sortConfig);
  }, [sortConfig]);

  useEffect(() => {
    if (!error) {
      return;
    }

    if (error instanceof Error && error.message.includes("(403)")) {
      setScope("my");
    }
  }, [error]);

  useEffect(() => {
    if (expandedAgentId === null) {
      return;
    }
    if (agents.some((agent) => agent.id === expandedAgentId)) {
      return;
    }
    setExpandedAgentId(null);
  }, [agents, expandedAgentId]);

  useEffect(() => {
    const id = expandedAgentId;
    if (id === null) {
      return;
    }
    if (runsByAgent[id] || loadingRunIds.has(id)) {
      return;
    }

    setLoadingRunIds((prev) => new Set(prev).add(id));

    void fetchAgentRuns(id, 50)
      .then((runs) => {
        setRunsByAgent((prev) => ({ ...prev, [id]: runs }));
      })
      .finally(() => {
        setLoadingRunIds((prev) => {
          const next = new Set(prev);
          next.delete(id);
          return next;
        });
      });
  }, [expandedAgentId, loadingRunIds, runsByAgent]);

  // Use unified WebSocket hook for real-time updates
  // Only connect when authenticated to avoid auth failure spam
  useWebSocket(isAuthenticated, {
    invalidateQueries: [["agents", { scope }]],
    onMessage: () => {
      // Additional refetch for immediate updates
      void refetch();
    },
  });

  const createAgentMutation = useMutation({
    mutationFn: async () => {
      const agentName = `New Agent ${Math.round(Math.random() * 100)}`;
      return createAgent({
        name: agentName,
        system_instructions: "You are a helpful AI assistant.",
        task_instructions: "Complete the given task.",
        model: "gpt-4o",
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["agents"] });
    },
  });

  const sortedRows: LegacyAgentRow[] = useMemo(() => {
    return sortAgents(agents, runsByAgent, sortConfig).map((agent) => ({
      agent,
      lastRunDisplay: formatDateTimeShort(agent.last_run_at ?? null),
      nextRunDisplay: formatDateTimeShort(agent.next_run_at ?? null),
    }));
  }, [agents, runsByAgent, sortConfig]);

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

  const includeOwner = scope === "all";
  const emptyColspan = includeOwner ? 7 : 6;

  return (
    <div id="dashboard-container" className="dashboard-container">
      <div id="dashboard" className="dashboard">
        <div className="dashboard-header">
          <div className="scope-wrapper">
            <span className="scope-text-label" id="scope-text">
              {scope === "all" ? "All agents" : "My agents"}
            </span>
            <label className="scope-toggle">
              <input
                type="checkbox"
                id="dashboard-scope-toggle"
                data-testid="dashboard-scope-toggle"
                checked={scope === "all"}
                onChange={(e) => {
                  const newScope = e.target.checked ? "all" : "my";
                  setScope(newScope);
                }}
              />
              <span className="slider"></span>
            </label>
          </div>
          <div className="button-container">
            <button
              id="create-agent-button"
              type="button"
              className={`create-agent-button${createAgentMutation.isPending ? " loading" : ""}`}
              data-testid="create-agent-btn"
              onClick={() => createAgentMutation.mutate()}
              disabled={createAgentMutation.isPending}
            >
              {createAgentMutation.isPending ? <span className="spinner" /> : "Create Agent"}
            </button>
          </div>
        </div>

        <table id="agents-table" className="agents-table">
          <thead>
            <tr>
              {renderHeaderCell("Name", "name", sortConfig, handleSort)}
              {includeOwner && renderHeaderCell("Owner", "owner", sortConfig, handleSort, false)}
              {renderHeaderCell("Status", "status", sortConfig, handleSort)}
              {renderHeaderCell("Last Run", "last_run", sortConfig, handleSort)}
              {renderHeaderCell("Next Run", "next_run", sortConfig, handleSort)}
              {renderHeaderCell("Success Rate", "success", sortConfig, handleSort)}
              <th
                scope="col"
                className="actions-header"
                data-column="actions"
                onClick={() => handleSort("name")}
                role="button"
                tabIndex={0}
              >
                Actions
              </th>
            </tr>
          </thead>
          <tbody id="agents-table-body">
            {sortedRows.map(({ agent, lastRunDisplay, nextRunDisplay }) => {
              const runs = runsByAgent[agent.id];
              const isExpanded = expandedAgentId === agent.id;
              const isRunHistoryExpanded = expandedRunHistory.has(agent.id);
              const successStats = computeSuccessStats(runs);
              const lastRunIndicator = determineLastRunIndicator(runs);
              const isRunning = agent.status === "running";
              const isPendingRun = pendingRunIds.has(agent.id);

              return (
                <Fragment key={agent.id}>
                  <tr
                    data-agent-id={agent.id}
                    aria-expanded={isExpanded ? "true" : "false"}
                    className={agent.status === "error" ? "error-row" : undefined}
                    tabIndex={0}
                    onClick={() => toggleAgentRow(agent.id)}
                    onKeyDown={(event) => handleRowKeyDown(event, agent.id)}
                  >
                    <td data-label="Name">{agent.name}</td>
                    {includeOwner && (
                      <td className="owner-cell" data-label="Owner">
                        {renderOwnerCell(agent)}
                      </td>
                    )}
                    <td data-label="Status">
                      <span className={`status-indicator status-${agent.status.toLowerCase()}`}>
                        {formatStatus(agent.status)}
                      </span>
                      {agent.last_error && agent.last_error.trim() && (
                        <span className="info-icon" title={agent.last_error}>
                          ℹ
                        </span>
                      )}
                      {lastRunIndicator !== null && (
                        <span
                          className={lastRunIndicator ? "last-run-indicator last-run-success" : "last-run-indicator last-run-failure"}
                        >
                          {lastRunIndicator ? " (Last: ✓)" : " (Last: ✗)"}
                        </span>
                      )}
                    </td>
                    <td data-label="Last Run">{lastRunDisplay}</td>
                    <td data-label="Next Run">{nextRunDisplay}</td>
                    <td data-label="Success Rate">{successStats.display}</td>
                    <td className="actions-cell" data-label="Actions">
                      <div className="actions-cell-inner">
                        <button
                          type="button"
                          className={`action-btn run-btn${isRunning || isPendingRun ? " disabled" : ""}`}
                          data-testid={`run-agent-${agent.id}`}
                          disabled={isRunning || isPendingRun}
                          title={isRunning ? "Agent is already running" : "Run Agent"}
                          aria-label={isRunning ? "Agent is already running" : "Run Agent"}
                          onClick={(event) => handleRunAgent(event, agent.id, agent.status)}
                        >
                          <PlayIcon />
                        </button>
                        <button
                          type="button"
                          className="action-btn edit-btn"
                          data-testid={`edit-agent-${agent.id}`}
                          title="Edit Agent"
                          aria-label="Edit Agent"
                          onClick={(event) => handleEditAgent(event, agent.id)}
                        >
                          <EditIcon />
                        </button>
                        <button
                          type="button"
                          className="action-btn chat-btn"
                          data-testid={`chat-agent-${agent.id}`}
                          title="Chat with Agent"
                          aria-label="Chat with Agent"
                          onClick={(event) => handleChatAgent(event, agent.id, agent.name)}
                        >
                          <MessageCircleIcon />
                        </button>
                        <button
                          type="button"
                          className="action-btn debug-btn"
                          data-testid={`debug-agent-${agent.id}`}
                          title="Debug / Info"
                          aria-label="Debug / Info"
                          onClick={(event) => handleDebugAgent(event, agent.id)}
                        >
                          <SettingsIcon />
                        </button>
                        <button
                          type="button"
                          className="action-btn delete-btn"
                          data-testid={`delete-agent-${agent.id}`}
                          title="Delete Agent"
                          aria-label="Delete Agent"
                          onClick={(event) => handleDeleteAgent(event, agent.id, agent.name)}
                        >
                          <TrashIcon />
                        </button>
                      </div>
                    </td>
                  </tr>
                  {isExpanded && (
                    <tr className="agent-detail-row" key={`detail-${agent.id}`}>
                      <td colSpan={emptyColspan}>
                        <div className="agent-detail-container">
                          {loadingRunIds.has(agent.id) && <span>Loading run history...</span>}
                          {!loadingRunIds.has(agent.id) && runs && runs.length === 0 && (
                            <span>No runs recorded yet.</span>
                          )}
                          {!loadingRunIds.has(agent.id) && runs && runs.length > 0 && (
                            <>
                              <table className="run-history-table">
                                <thead>
                                  <tr>
                                    <th>Status</th>
                                    <th>Started</th>
                                    <th>Duration</th>
                                    <th>Trigger</th>
                                    <th>Tokens</th>
                                    <th>Cost</th>
                                    <th />
                                  </tr>
                                </thead>
                                <tbody>
                                  {runs
                                    .slice(0, isRunHistoryExpanded ? runs.length : Math.min(runs.length, 5))
                                    .map((run) => (
                                      <tr key={run.id}>
                                        <td>{formatRunStatusIcon(run.status)}</td>
                                        <td>{formatDateTimeShort(run.started_at ?? null)}</td>
                                        <td>{formatDuration(run.duration_ms)}</td>
                                        <td>{capitaliseFirst(run.trigger)}</td>
                                        <td>{formatTokens(run.total_tokens)}</td>
                                        <td>{formatCost(run.total_cost_usd)}</td>
                                        <td className="run-kebab-cell">
                                          <span
                                            className="kebab-menu-btn"
                                            role="button"
                                            tabIndex={0}
                                            onClick={(event) => {
                                              event.preventDefault();
                                              event.stopPropagation();
                                              dispatchDashboardEvent("run-actions", agent.id, run.id);
                                            }}
                                            onKeyDown={(event) => {
                                              if (event.key === "Enter" || event.key === " ") {
                                                event.preventDefault();
                                                event.stopPropagation();
                                                dispatchDashboardEvent("run-actions", agent.id, run.id);
                                              }
                                            }}
                                          >
                                            ⋮
                                          </span>
                                        </td>
                                      </tr>
                                    ))}
                                </tbody>
                              </table>
                              {runs.length > 5 && (
                                <a
                                  href="#"
                                  className="run-toggle-link"
                                  aria-expanded={isRunHistoryExpanded ? "true" : "false"}
                                  onClick={(event) => {
                                    event.preventDefault();
                                    toggleRunHistory(agent.id);
                                  }}
                                >
                                  {isRunHistoryExpanded ? "Show less" : `Show all (${runs.length})`}
                                </a>
                              )}
                            </>
                          )}
                        </div>
                      </td>
                    </tr>
                  )}
                </Fragment>
              );
            })}
            {sortedRows.length === 0 && (
              <tr>
                <td colSpan={emptyColspan}>
                  <div className="empty-state">
                    <div className="empty-state-illustration">🤖</div>
                    <p className="empty-state-text">
                      No agents found. Click 'Create Agent' to get started.
                    </p>
                  </div>
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
      {settingsAgentId != null && (
        <AgentSettingsDrawer
          agentId={settingsAgentId}
          isOpen={settingsAgentId != null}
          onClose={() => setSettingsAgentId(null)}
        />
      )}
    </div>
  );

  function toggleAgentRow(agentId: number) {
    setExpandedAgentId((prev) => (prev === agentId ? null : agentId));
  }

  function toggleRunHistory(agentId: number) {
    setExpandedRunHistory((prev) => {
      const next = new Set(prev);
      if (next.has(agentId)) {
        next.delete(agentId);
      } else {
        next.clear();
        next.add(agentId);
      }
      return next;
    });
  }

  function handleSort(key: SortKey) {
    setSortConfig((prev) => {
      if (prev.key === key) {
        return { key, ascending: !prev.ascending };
      }
      return { key, ascending: true };
    });
  }

  function handleRowKeyDown(event: ReactKeyboardEvent<HTMLTableRowElement>, agentId: number) {
    const key = event.key;
    if (key === "Enter") {
      event.preventDefault();
      toggleAgentRow(agentId);
      return;
    }

    if (key !== "ArrowDown" && key !== "ArrowUp") {
      return;
    }

    event.preventDefault();
    const current = event.currentTarget;
    const tbody = current.closest("tbody");
    if (!tbody) {
      return;
    }
    const rows = Array.from(tbody.querySelectorAll<HTMLTableRowElement>("tr[data-agent-id]"));
    const index = rows.indexOf(current);
    if (index === -1) {
      return;
    }
    const nextIndex = key === "ArrowDown" ? Math.min(rows.length - 1, index + 1) : Math.max(0, index - 1);
    rows[nextIndex]?.focus();
  }

  async function handleRunAgent(event: ReactMouseEvent<HTMLButtonElement>, agentId: number, status: string) {
    event.stopPropagation();
    if (status === "running" || pendingRunIds.has(agentId)) {
      return;
    }

    setPendingRunIds((prev) => new Set(prev).add(agentId));
    try {
      await runAgent(agentId);
      await queryClient.invalidateQueries({ queryKey: ["agents"] });
      dispatchDashboardEvent("run", agentId);
    } catch (runError) {
      console.error("Failed to run agent", runError);
    } finally {
      setPendingRunIds((prev) => {
        const next = new Set(prev);
        next.delete(agentId);
        return next;
      });
    }
  }

  function handleEditAgent(event: ReactMouseEvent<HTMLButtonElement>, agentId: number) {
    event.stopPropagation();
    dispatchDashboardEvent("edit", agentId);
  }

  function handleChatAgent(event: ReactMouseEvent<HTMLButtonElement>, agentId: number, agentName: string) {
    event.stopPropagation();
    navigate(`/agent/${agentId}/thread/?name=${encodeURIComponent(agentName)}`);
  }

  function handleDebugAgent(event: ReactMouseEvent<HTMLButtonElement>, agentId: number) {
    event.stopPropagation();
    setSettingsAgentId(agentId);
  }

  function handleDeleteAgent(event: ReactMouseEvent<HTMLButtonElement>, agentId: number, name: string) {
    event.stopPropagation();
    const confirmed = typeof window === "undefined" || window.confirm(`Delete agent ${name}?`);
    if (!confirmed) {
      return;
    }
    dispatchDashboardEvent("delete", agentId);
  }
}

type HeaderRenderer = (
  label: string,
  sortKey: SortKey | "owner",
  sortConfig: SortConfig,
  onSort: (key: SortKey) => void,
  sortable?: boolean
) => JSX.Element;

const renderHeaderCell: HeaderRenderer = (label, sortKey, sortConfig, onSort, sortable = true) => {
  const dataColumn = label.toLowerCase().replace(/\s+/g, "_");
  const effectiveKey = sortKey === "owner" ? "name" : sortKey;
  const isActive = sortable && sortConfig.key === effectiveKey;
  const arrow = sortConfig.ascending ? "▲" : "▼";

  return (
    <th
      scope="col"
      data-column={dataColumn}
      onClick={sortable ? () => onSort(effectiveKey as SortKey) : undefined}
      role={sortable ? "button" : undefined}
      tabIndex={sortable ? 0 : undefined}
    >
      {label}
      {isActive && <span className="sort-indicator">{arrow}</span>}
    </th>
  );
};

function renderOwnerCell(agent: AgentSummary) {
  if (!agent.owner) {
    return <span>-</span>;
  }

  const label = agent.owner.display_name?.trim() || agent.owner.email;
  if (!label) {
    return <span>-</span>;
  }

  return (
    <div className="owner-wrapper">
      {agent.owner.avatar_url && <img src={agent.owner.avatar_url} alt="" className="owner-avatar" aria-hidden="true" />}
      <span>{label}</span>
    </div>
  );
}

function sortAgents(agents: AgentSummary[], runsByAgent: AgentRunsState, sortConfig: SortConfig): AgentSummary[] {
  const sorted = [...agents];
  sorted.sort((left, right) => {
    const comparison = compareAgents(left, right, runsByAgent, sortConfig.key);
    if (comparison !== 0) {
      return sortConfig.ascending ? comparison : -comparison;
    }
    const fallback = left.name.toLowerCase().localeCompare(right.name.toLowerCase());
    return sortConfig.ascending ? fallback : -fallback;
  });
  return sorted;
}

function compareAgents(
  left: AgentSummary,
  right: AgentSummary,
  runsByAgent: AgentRunsState,
  sortKey: SortKey
): number {
  switch (sortKey) {
    case "name":
      return left.name.toLowerCase().localeCompare(right.name.toLowerCase());
    case "status":
      return (STATUS_ORDER[left.status] ?? 99) - (STATUS_ORDER[right.status] ?? 99);
    case "last_run":
      return formatDateTimeShort(left.last_run_at ?? null).localeCompare(
        formatDateTimeShort(right.last_run_at ?? null)
      );
    case "next_run":
      return formatDateTimeShort(left.next_run_at ?? null).localeCompare(
        formatDateTimeShort(right.next_run_at ?? null)
      );
    case "success": {
      const leftStats = computeSuccessStats(runsByAgent[left.id]);
      const rightStats = computeSuccessStats(runsByAgent[right.id]);
      if (leftStats.rate === rightStats.rate) {
        return leftStats.count - rightStats.count;
      }
      return leftStats.rate - rightStats.rate;
    }
    default:
      return 0;
  }
}

function computeSuccessStats(runs?: AgentRun[]): { display: string; rate: number; count: number } {
  if (!runs || runs.length === 0) {
    return { display: "0.0% (0)", rate: 0, count: 0 };
  }

  const successCount = runs.filter((run) => run.status === "success").length;
  const successRate = runs.length === 0 ? 0 : (successCount / runs.length) * 100;
  return {
    display: `${successRate.toFixed(1)}% (${runs.length})`,
    rate: successRate,
    count: runs.length,
  };
}

function determineLastRunIndicator(runs?: AgentRun[]): boolean | null {
  if (!runs || runs.length === 0) {
    return null;
  }
  const status = runs[0]?.status;
  if (status === "success") {
    return true;
  }
  if (status === "failed") {
    return false;
  }
  return null;
}

function formatDateTimeShort(iso: string | null | undefined): string {
  if (!iso) {
    return "-";
  }

  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) {
    return "-";
  }

  const year = date.getUTCFullYear();
  const month = String(date.getUTCMonth() + 1).padStart(2, "0");
  const day = String(date.getUTCDate()).padStart(2, "0");
  const hours = String(date.getUTCHours()).padStart(2, "0");
  const minutes = String(date.getUTCMinutes()).padStart(2, "0");
  return `${year}-${month}-${day}${NBSP}${hours}:${minutes}`;
}

function formatStatus(status: string): string {
  switch (status) {
    case "running":
      return "● Running";
    case "processing":
      return "⏳ Processing";
    case "error":
      return "⚠ Error";
    case "idle":
    default:
      return "○ Idle";
  }
}

function formatDuration(durationMs?: number | null): string {
  if (!durationMs) {
    return "-";
  }
  const secondsTotal = Math.floor(durationMs / 1000);
  const minutes = Math.floor(secondsTotal / 60);
  const seconds = secondsTotal % 60;
  if (minutes > 0) {
    return `${minutes} m ${String(seconds).padStart(2, "0")} s`;
  }
  return `${seconds} s`;
}

function capitaliseFirst(value: string): string {
  if (!value) {
    return "";
  }
  return value.charAt(0).toUpperCase() + value.slice(1);
}

function formatTokens(tokens?: number | null): string {
  if (tokens === null || tokens === undefined) {
    return "—";
  }
  return tokens.toString();
}

function formatCost(cost?: number | null): string {
  if (cost === null || cost === undefined) {
    return "—";
  }
  if (cost >= 0.1) {
    return `$${cost.toFixed(2)}`;
  }
  if (cost >= 0.01) {
    return `$${cost.toFixed(3)}`;
  }
  return `$${cost.toFixed(4)}`;
}

function formatRunStatusIcon(status: AgentRun["status"]): string {
  switch (status) {
    case "running":
      return "▶";
    case "success":
      return "✔";
    case "failed":
      return "✖";
    default:
      return "●";
  }
}

function loadSortConfig(): SortConfig {
  if (typeof window === "undefined") {
    return { key: "name", ascending: true };
  }

  const storedKey = window.localStorage.getItem(STORAGE_KEY_SORT) ?? "name";
  const storedAsc = window.localStorage.getItem(STORAGE_KEY_ASC);

  const keyMap: Record<string, SortKey> = {
    name: "name",
    status: "status",
    last_run: "last_run",
    next_run: "next_run",
    success: "success",
  };

  const key = keyMap[storedKey] ?? "name";
  const ascending = storedAsc === null ? true : storedAsc !== "0";
  return { key, ascending };
}

function persistSortConfig(config: SortConfig) {
  if (typeof window === "undefined") {
    return;
  }
  window.localStorage.setItem(STORAGE_KEY_SORT, config.key);
  window.localStorage.setItem(STORAGE_KEY_ASC, config.ascending ? "1" : "0");
}

type DashboardEventType = "run" | "edit" | "debug" | "delete" | "run-actions";

function dispatchDashboardEvent(type: DashboardEventType, agentId: number, runId?: number) {
  if (typeof window === "undefined") {
    return;
  }
  const event = new CustomEvent("dashboard:event", {
    detail: {
      type,
      agentId,
      runId,
    },
  });
  window.dispatchEvent(event);
}
