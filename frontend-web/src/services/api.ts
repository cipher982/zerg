export type AgentOwner = {
  id: number;
  email: string;
  display_name?: string | null;
  avatar_url?: string | null;
};

export type AgentSummary = {
  id: number;
  name: string;
  status: string;
  owner_id: number;
  owner?: AgentOwner | null;
  last_run_at?: string | null;
  next_run_at?: string | null;
  last_error?: string | null;
  system_instructions?: string;
  task_instructions?: string;
};

export type Agent = AgentSummary & {
  created_at: string;
  updated_at: string;
  model: string;
};

export type AgentRun = {
  id: number;
  agent_id: number;
  thread_id: number;
  status: "queued" | "running" | "success" | "failed";
  trigger: "manual" | "schedule" | "api";
  started_at?: string | null;
  finished_at?: string | null;
  duration_ms?: number | null;
  total_tokens?: number | null;
  total_cost_usd?: number | null;
  error?: string | null;
};

export type Thread = {
  id: number;
  agent_id: number;
  title: string;
  active: boolean;
  thread_type: string;
  created_at: string;
  updated_at: string;
};

export type ThreadMessage = {
  id: number;
  thread_id: number;
  role: "assistant" | "user" | "system" | "tool";
  content: string;
  created_at: string;
  processed: boolean;
};

const apiBaseOverride = (() => {
  if (typeof window === "undefined") {
    return undefined;
  }
  return (window as typeof window & { API_BASE_URL?: string }).API_BASE_URL;
})();

const API_BASE = apiBaseOverride ?? "/api";

function buildUrl(path: string): string {
  if (API_BASE.startsWith("http")) {
    const base = API_BASE.endsWith("/") ? API_BASE : `${API_BASE}/`;
    return new URL(path.replace(/^\//, ""), base).toString();
  }
  return `${API_BASE}${path}`;
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const token = localStorage.getItem("zerg_jwt");
  const headers = new Headers(init?.headers);
  headers.set("Content-Type", "application/json");
  if (token) {
    headers.set("Authorization", `Bearer ${token}`);
  }

  // Add test worker header for E2E test isolation
  // This should match the worker ID that Playwright uses
  const testWorkerHeader = window.__TEST_WORKER_ID__;
  if (testWorkerHeader !== undefined) {
    headers.set("X-Test-Worker", String(testWorkerHeader));
  }

  const url = buildUrl(path);
  const res = await fetch(url, {
    ...init,
    headers,
    credentials: "include"
  });

  if (!res.ok) {
    const body = await res.text();
    throw new Error(`Request failed (${res.status}): ${body}`);
  }

  return res.json() as Promise<T>;
}

type FetchAgentsParams = {
  scope?: "my" | "all";
  limit?: number;
  skip?: number;
};

export async function fetchAgents(params: FetchAgentsParams = {}): Promise<AgentSummary[]> {
  const scope = params.scope ?? "my";
  const limit = params.limit ?? 100;
  const skip = params.skip ?? 0;
  const searchParams = new URLSearchParams({
    scope,
    limit: String(limit),
    skip: String(skip),
  });

  return request<AgentSummary[]>(`/agents?${searchParams.toString()}`);
}

type CreateAgentPayload = {
  name: string;
  system_instructions: string;
  task_instructions: string;
  model: string;
};

export async function createAgent(payload: CreateAgentPayload): Promise<Agent> {
  return request<Agent>(`/agents`, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function fetchAgent(agentId: number): Promise<Agent> {
  return request<Agent>(`/agents/${agentId}`);
}

export async function fetchThreads(agentId: number): Promise<Thread[]> {
  return request<Thread[]>(`/threads?agent_id=${agentId}`);
}

export async function fetchThreadMessages(threadId: number): Promise<ThreadMessage[]> {
  return request<ThreadMessage[]>(`/threads/${threadId}/messages`);
}

export async function createThread(agentId: number, title: string): Promise<Thread> {
  return request<Thread>(`/threads`, {
    method: "POST",
    body: JSON.stringify({ agent_id: agentId, title, thread_type: "chat" }),
  });
}

export async function postThreadMessage(threadId: number, content: string): Promise<ThreadMessage> {
  return request<ThreadMessage>(`/threads/${threadId}/messages`, {
    method: "POST",
    body: JSON.stringify({ role: "user", content }),
  });
}

export async function runThread(threadId: number): Promise<void> {
  await request(`/threads/${threadId}/run`, {
    method: "POST",
  });
}

export async function fetchAgentRuns(agentId: number, limit = 20): Promise<AgentRun[]> {
  return request<AgentRun[]>(`/agents/${agentId}/runs?limit=${limit}`);
}

type AgentUpdatePayload = Partial<{
  name: string;
  system_instructions: string;
  task_instructions: string;
  model: string;
  status: "idle" | "running" | "error" | "processing";
  schedule: string | null;
  config: Record<string, unknown> | null;
  last_error: string | null;
}>;

export async function updateAgent(agentId: number, payload: AgentUpdatePayload): Promise<Agent> {
  return request<Agent>(`/agents/${agentId}`, {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

export async function resetAgent(agentId: number): Promise<Agent> {
  return updateAgent(agentId, { status: "idle", last_error: "" });
}

export async function runAgent(agentId: number): Promise<void> {
  await request(`/agents/${agentId}/task`, {
    method: "POST",
  });
}
