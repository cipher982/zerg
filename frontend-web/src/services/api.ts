export type AgentSummary = {
  id: number;
  name: string;
  status: string;
  system_instructions?: string;
  task_instructions?: string;
};

export type Agent = AgentSummary & {
  created_at: string;
  updated_at: string;
  model: string;
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

const API_BASE = "/api";

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const token = localStorage.getItem("zerg_jwt");
  const headers = new Headers(init?.headers);
  headers.set("Content-Type", "application/json");
  if (token) {
    headers.set("Authorization", `Bearer ${token}`);
  }

  // Add test worker header for E2E test isolation
  // This should match the worker ID that Playwright uses
  const testWorkerHeader = (window as any).__TEST_WORKER_ID__;
  if (testWorkerHeader !== undefined) {
    headers.set("X-Test-Worker", String(testWorkerHeader));
  }

  const res = await fetch(`${API_BASE}${path}`, {
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

export async function fetchAgents(): Promise<AgentSummary[]> {
  return request<AgentSummary[]>("/agents?scope=my");
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
