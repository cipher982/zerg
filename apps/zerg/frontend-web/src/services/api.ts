import type { components, operations } from "../generated/openapi-types";

export class ApiError extends Error {
  readonly status: number;
  readonly url: string;
  readonly body: unknown;

  constructor({ url, status, body }: { url: string; status: number; body: unknown }) {
    super(`Request to ${url} failed with status ${status}`);
    this.name = "ApiError";
    this.status = status;
    this.url = url;
    this.body = body;
  }
}

type Schemas = components["schemas"];
type Operations = operations;

type JsonResponse<Op extends { responses: Record<number | string, unknown> }, Code extends keyof Op["responses"]> =
  Op["responses"][Code] extends { content: { "application/json": infer Result } }
    ? Result
    : never;

export type Agent = Schemas["Agent"];
export type AgentSummary = Agent;
export type AgentRun = Schemas["AgentRunOut"];
export type Thread = Schemas["Thread"];
export type ThreadMessage = Schemas["ThreadMessageResponse"] & { created_at?: string };
export type ThreadUpdatePayload = Schemas["ThreadUpdate"];
export type Workflow = Schemas["Workflow"];
export type WorkflowData = Schemas["WorkflowData-Output"];
export type WorkflowDataInput = Schemas["WorkflowData-Input"];
export type WorkflowNode = Schemas["WorkflowNode"];
export type WorkflowEdge = Schemas["WorkflowEdge"];

// Workflow Execution Types
export interface WorkflowExecution {
  id: number;
  workflow_id: number;
  phase: 'waiting' | 'running' | 'finished' | 'cancelled';
  result?: unknown;
  log?: string;
  started_at?: string;
  finished_at?: string;
  triggered_by?: string;
}

export interface ExecutionStatus {
  execution_id: number;
  phase: string;
  result?: unknown;
}

export interface ExecutionLogs {
  logs: string;
}

export interface ContainerPolicy {
  enabled: boolean;
  default_image: string | null;
  network_enabled: boolean;
  user_id: number | null;
  memory_limit: string | null;
  cpus: string | null;
  timeout_secs: number;
  seccomp_profile: string | null;
}

export interface AvailableToolsResponse {
  builtin: string[];
  mcp: Record<string, string[]>;
}

export type McpServerAddRequest = components["schemas"]["MCPServerAddRequest"];
export type McpServerResponse = components["schemas"]["MCPServerResponse"];
export type McpTestConnectionResponse = components["schemas"]["MCPTestConnectionResponse"];

type AgentCreate = Schemas["AgentCreate"];
type AgentUpdate = Schemas["AgentUpdate"];
type ThreadCreate = Schemas["ThreadCreate"];
type ThreadMessageCreate = Schemas["ThreadMessageCreate"];
type WorkflowCreate = Schemas["WorkflowCreate"];
type CanvasUpdate = Schemas["CanvasUpdate"];

export type AgentCreatePayload = Pick<AgentCreate, "name" | "system_instructions" | "task_instructions" | "model"> &
  Partial<Omit<AgentCreate, "name" | "system_instructions" | "task_instructions" | "model">>;

export type AgentUpdatePayload = AgentUpdate;

type AgentsResponse = JsonResponse<Operations["read_agents_api_agents_get"], 200>;
type AgentResponse = JsonResponse<Operations["read_agent_api_agents__agent_id__get"], 200>;
type ThreadsResponse = JsonResponse<Operations["read_threads_api_threads_get"], 200>;
type ThreadMessagesResponse = JsonResponse<Operations["read_thread_messages_api_threads__thread_id__messages_get"], 200>;
type AgentRunsListResponse = JsonResponse<Operations["list_agent_runs_api_agents__agent_id__runs_get"], 200>;
type CreatedThreadResponse = JsonResponse<Operations["create_thread_api_threads_post"], 201>;
type CreatedThreadMessageResponse = JsonResponse<Operations["create_thread_message_api_threads__thread_id__messages_post"], 201>;
type CreatedAgentResponse = JsonResponse<Operations["create_agent_api_agents_post"], 201>;
type UpdatedAgentResponse = JsonResponse<Operations["update_agent_api_agents__agent_id__put"], 200>;
type WorkflowsResponse = JsonResponse<Operations["read_workflows_api_workflows__get"], 200>;
type WorkflowResponse = JsonResponse<Operations["get_current_workflow_api_workflows_current_get"], 200>;
type CreatedWorkflowResponse = JsonResponse<Operations["create_workflow_api_workflows__post"], 201>;
type UpdatedWorkflowCanvasResponse = JsonResponse<Operations["update_current_workflow_canvas_api_workflows_current_canvas_patch"], 200>;

type FetchAgentsParams = {
  scope?: "my" | "all";
  limit?: number;
  skip?: number;
};

type ApiBaseConfig = {
  absolute?: string;
  relative: string;
};

const apiBaseOverride =
  typeof window !== "undefined"
    ? (window as typeof window & { API_BASE_URL?: string }).API_BASE_URL
    : undefined;

function normalizePathname(pathname: string): string {
  const trimmed = pathname.replace(/\/+$/, "");
  if (!trimmed) {
    return "/api";
  }
  if (/\/api(\/|$)/.test(trimmed)) {
    return trimmed;
  }
  return `${trimmed}/api`.replace(/\/+/g, "/");
}

function normalizeRelativeBase(path: string): string {
  const trimmed = path.trim();
  const prefixed = trimmed.startsWith("/") ? trimmed : `/${trimmed}`;
  const withoutTrailing = prefixed.replace(/\/+$/, "");
  if (!withoutTrailing) {
    return "/api";
  }
  if (/\/api(\/|$)/.test(withoutTrailing)) {
    return withoutTrailing;
  }
  return `${withoutTrailing}/api`.replace(/\/+/g, "/");
}

function computeApiBase(override?: string): ApiBaseConfig {
  if (!override) {
    return { relative: "/api" };
  }

  if (!override.startsWith("http")) {
    return { relative: normalizeRelativeBase(override) };
  }

  try {
    const url = new URL(override);
    const normalizedPath = normalizePathname(url.pathname || "/");
    const absolute = `${url.origin}${normalizedPath.endsWith("/") ? normalizedPath : `${normalizedPath}/`}`;
    return {
      absolute,
      relative: normalizedPath,
    };
  } catch (error) {
    console.error("Failed to parse API_BASE_URL", error);
    return { relative: "/api" };
  }
}

const API_BASE = computeApiBase(apiBaseOverride);

function buildUrl(path: string): string {
  const normalizedPath = path.startsWith("/") ? path.slice(1) : path;

  if (API_BASE.absolute) {
    return new URL(normalizedPath, API_BASE.absolute).toString();
  }

  const prefix = API_BASE.relative.endsWith("/") ? API_BASE.relative.slice(0, -1) : API_BASE.relative;
  return `${prefix}/${normalizedPath}`;
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const url = buildUrl(path);
  const headers = new Headers(init?.headers);

  if (!headers.has("Content-Type") && init?.body && !(init.body instanceof FormData)) {
    headers.set("Content-Type", "application/json");
  }

  const token = typeof window !== "undefined" ? window.localStorage.getItem("zerg_jwt") : null;
  if (token) {
    headers.set("Authorization", `Bearer ${token}`);
  }

  const testWorkerHeader = typeof window !== "undefined" ? window.__TEST_WORKER_ID__ : undefined;
  if (testWorkerHeader !== undefined) {
    headers.set("X-Test-Worker", String(testWorkerHeader));
  }

  const response = await fetch(url, {
    ...init,
    headers,
  });

  const hasBody = response.status !== 204 && response.status !== 205;
  const contentType = response.headers.get("content-type") ?? "";
  const expectsJson = contentType.includes("application/json");
  let data: unknown = undefined;

  if (hasBody) {
    try {
      if (expectsJson) {
        data = await response.json();
      } else {
        const text = await response.text();
        data = text.length > 0 ? text : undefined;
      }
    } catch (error) {
      if (!response.ok) {
        throw new ApiError({ url, status: response.status, body: data });
      }
      throw error instanceof Error ? error : new Error("Failed to parse response body");
    }
  }

  if (!response.ok) {
    throw new ApiError({ url, status: response.status, body: data });
  }

  return data as T;
}

export async function fetchAgents(params: FetchAgentsParams = {}): Promise<AgentsResponse> {
  const scope = params.scope ?? "my";
  const limit = params.limit ?? 100;
  const skip = params.skip ?? 0;
  const searchParams = new URLSearchParams({
    scope,
    limit: String(limit),
    skip: String(skip),
  });

  return request<AgentsResponse>(`/agents?${searchParams.toString()}`);
}

export async function createAgent(payload: AgentCreatePayload): Promise<CreatedAgentResponse> {
  return request<CreatedAgentResponse>(`/agents`, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function fetchAgent(agentId: number): Promise<AgentResponse> {
  return request<AgentResponse>(`/agents/${agentId}`);
}

export async function fetchThreads(agentId: number): Promise<ThreadsResponse> {
  return request<ThreadsResponse>(`/threads?agent_id=${agentId}`);
}

export async function fetchThreadMessages(threadId: number): Promise<ThreadMessagesResponse> {
  return request<ThreadMessagesResponse>(`/threads/${threadId}/messages`);
}

export async function createThread(agentId: number, title: string): Promise<CreatedThreadResponse> {
  const payload: ThreadCreate = {
    agent_id: agentId,
    title,
    thread_type: "chat",
    memory_strategy: "buffer",
    active: true,
  };
  return request<CreatedThreadResponse>(`/threads`, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function postThreadMessage(threadId: number, content: string): Promise<CreatedThreadMessageResponse> {
  const payload: ThreadMessageCreate = {
    role: "user",
    content,
  };
  return request<CreatedThreadMessageResponse>(`/threads/${threadId}/messages`, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function runThread(threadId: number): Promise<void> {
  await request<void>(`/threads/${threadId}/run`, {
    method: "POST",
  });
}

export async function updateThread(threadId: number, payload: ThreadUpdatePayload): Promise<Thread> {
  return request<Thread>(`/threads/${threadId}`, {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

export async function fetchAgentRuns(agentId: number, limit = 20): Promise<AgentRunsListResponse> {
  return request<AgentRunsListResponse>(`/agents/${agentId}/runs?limit=${limit}`);
}

export async function updateAgent(agentId: number, payload: AgentUpdatePayload): Promise<UpdatedAgentResponse> {
  return request<UpdatedAgentResponse>(`/agents/${agentId}`, {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

export async function resetAgent(agentId: number): Promise<UpdatedAgentResponse> {
  return updateAgent(agentId, { status: "idle", last_error: "" });
}

export async function runAgent(agentId: number): Promise<void> {
  await request<void>(`/agents/${agentId}/task`, {
    method: "POST",
  });
}

export async function fetchContainerPolicy(): Promise<ContainerPolicy> {
  return request<ContainerPolicy>(`/config/container-policy`);
}

export async function fetchAvailableTools(agentId: number): Promise<AvailableToolsResponse> {
  return request<AvailableToolsResponse>(`/agents/${agentId}/mcp-servers/available-tools`);
}

export async function fetchMcpServers(agentId: number): Promise<McpServerResponse[]> {
  return request<McpServerResponse[]>(`/agents/${agentId}/mcp-servers/`);
}

export async function addMcpServer(agentId: number, payload: McpServerAddRequest): Promise<Agent> {
  return request<Agent>(`/agents/${agentId}/mcp-servers/`, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function removeMcpServer(agentId: number, serverName: string): Promise<void> {
  await request<void>(`/agents/${agentId}/mcp-servers/${encodeURIComponent(serverName)}`, {
    method: "DELETE",
  });
}

export async function testMcpServer(agentId: number, payload: McpServerAddRequest): Promise<McpTestConnectionResponse> {
  return request<McpTestConnectionResponse>(`/agents/${agentId}/mcp-servers/test`, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

// Workflow API functions
export async function fetchWorkflows(): Promise<WorkflowsResponse> {
  return request<WorkflowsResponse>(`/workflows`);
}

export async function fetchCurrentWorkflow(): Promise<WorkflowResponse> {
  return request<WorkflowResponse>(`/workflows/current`);
}

export async function createWorkflow(name: string, description?: string, canvas?: WorkflowDataInput): Promise<CreatedWorkflowResponse> {
  const payload: WorkflowCreate = {
    name,
    description: description || "",
    canvas: canvas || { nodes: [], edges: [] },
  };
  return request<CreatedWorkflowResponse>(`/workflows`, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function updateWorkflowCanvas(canvas: WorkflowDataInput): Promise<UpdatedWorkflowCanvasResponse> {
  const payload: CanvasUpdate = {
    canvas,
  };
  return request<UpdatedWorkflowCanvasResponse>(`/workflows/current/canvas`, {
    method: "PATCH",
    body: JSON.stringify(payload),
  });
}

// Workflow Execution API functions
export async function reserveWorkflowExecution(workflowId: number): Promise<ExecutionStatus> {
  return request<ExecutionStatus>(`/workflow-executions/by-workflow/${workflowId}/reserve`, {
    method: "POST",
  });
}

export async function startWorkflowExecution(workflowId: number): Promise<ExecutionStatus> {
  return request<ExecutionStatus>(`/workflow-executions/by-workflow/${workflowId}/start`, {
    method: "POST",
  });
}

export async function startReservedExecution(executionId: number): Promise<ExecutionStatus> {
  return request<ExecutionStatus>(`/workflow-executions/executions/${executionId}/start`, {
    method: "POST",
  });
}

export async function getExecutionStatus(executionId: number): Promise<ExecutionStatus> {
  return request<ExecutionStatus>(`/workflow-executions/${executionId}/status`);
}

export async function getExecutionLogs(executionId: number): Promise<ExecutionLogs> {
  return request<ExecutionLogs>(`/workflow-executions/${executionId}/logs`);
}

export async function cancelExecution(executionId: number, reason: string): Promise<void> {
  return request<void>(`/workflow-executions/${executionId}/cancel`, {
    method: "PATCH",
    body: JSON.stringify({ reason }),
  });
}

export async function getExecutionHistory(workflowId: number): Promise<WorkflowExecution[]> {
  return request<WorkflowExecution[]>(`/workflow-executions/history/${workflowId}`);
}
