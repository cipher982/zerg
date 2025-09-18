export type AgentSummary = {
  id: number;
  name: string;
  status: string;
};

const API_BASE = "/api";

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const token = localStorage.getItem("zerg_jwt");
  const headers = new Headers(init?.headers);
  headers.set("Content-Type", "application/json");
  if (token) {
    headers.set("Authorization", `Bearer ${token}`);
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
  const data = await request<{ items: AgentSummary[] }>("/agents");
  return data.items;
}
