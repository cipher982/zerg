import { expect } from '@playwright/test';

export interface CreateAgentRequest {
  name?: string;
  model?: string;
  system_instructions?: string;
  task_instructions?: string;
  temperature?: number;
}

export interface Agent {
  id: string;
  name: string;
  model: string;
  system_instructions: string;
  task_instructions: string;
  temperature: number;
  created_at: string;
  updated_at: string;
}

export interface CreateThreadRequest {
  title?: string;
  agent_id: string;
}

export interface Thread {
  id: string;
  title: string;
  agent_id: string;
  created_at: string;
  updated_at: string;
}

export class ApiClient {
  private baseUrl: string;
  private headers: Record<string, string>;

  constructor(baseUrl: string = 'http://localhost:8001') {  // Use backend port 8001
    this.baseUrl = baseUrl;
    this.headers = {
      'Content-Type': 'application/json',
    };
  }

  setAuthToken(token: string) {
    this.headers['Authorization'] = `Bearer ${token}`;
  }

  private async request(method: string, path: string, body?: any): Promise<any> {
    const url = `${this.baseUrl}${path}`;
    let response;
    let errorText = '';
    
    try {
      response = await fetch(url, {
        method,
        headers: this.headers,
        body: body ? JSON.stringify(body) : undefined,
      });
      
      if (!response.ok) {
        errorText = await response.text();
        throw new Error(`API request failed: ${method} ${path} - ${response.status} ${response.statusText}: ${errorText}`);
      }

      const contentType = response.headers.get('content-type');
      if (contentType && contentType.includes('application/json')) {
        return await response.json();
      }
      return await response.text();
    } catch (error) {
      console.error(`API request error: ${method} ${path}`, {
        error: error instanceof Error ? error.message : String(error),
        responseStatus: response?.status,
        responseText: errorText
      });
      
      // If it's a 500 error, wait a bit and retry once
      if (response?.status === 500) {
        await new Promise(resolve => setTimeout(resolve, 1000));
        return this.request(method, path, body);
      }
      
      throw error;
    }
  }

  async createAgent(data: CreateAgentRequest = {}): Promise<Agent> {
    const agentData = {
      name: data.name || `Test Agent ${Date.now()}`,
      model: data.model || 'gpt-mock',  // Use test-friendly model
      system_instructions: data.system_instructions || 'You are a helpful AI assistant.',
      task_instructions: data.task_instructions || 'Please help the user with their request.',
      ...data
    };

    return await this.request('POST', '/api/agents', agentData);
  }

  async getAgent(id: string): Promise<Agent> {
    return await this.request('GET', `/api/agents/${id}`);
  }

  async updateAgent(id: string, data: Partial<CreateAgentRequest>): Promise<Agent> {
    return await this.request('PUT', `/api/agents/${id}`, data);
  }

  async deleteAgent(id: string): Promise<void> {
    await this.request('DELETE', `/api/agents/${id}`);
  }

  async listAgents(): Promise<Agent[]> {
    return await this.request('GET', '/api/agents');
  }

  async createThread(data: CreateThreadRequest): Promise<Thread> {
    const threadData = {
      title: data.title || `Test Thread ${Date.now()}`,
      ...data
    };

    return await this.request('POST', '/api/threads', threadData);
  }

  async getThread(id: string): Promise<Thread> {
    return await this.request('GET', `/api/threads/${id}`);
  }

  async deleteThread(id: string): Promise<void> {
    await this.request('DELETE', `/api/threads/${id}`);
  }

  async listThreads(agentId?: string): Promise<Thread[]> {
    const url = agentId ? `/api/threads?agent_id=${agentId}` : '/api/threads';
    return await this.request('GET', url);
  }

  async resetDatabase(): Promise<void> {
    try {
      await this.request('POST', '/api/admin/reset-database');
      // Wait for database to be fully reset before proceeding
      await new Promise(resolve => setTimeout(resolve, 1000));
    } catch (error) {
      console.error('Database reset failed, trying fallback cleanup...');
      // If reset fails, try to manually clean up test data
      const agents = await this.listAgents();
      await Promise.all(agents.map(agent => this.deleteAgent(agent.id)));
    }
  }

  async healthCheck(): Promise<any> {
    return await this.request('GET', '/');
  }
}

export const apiClient = new ApiClient();
