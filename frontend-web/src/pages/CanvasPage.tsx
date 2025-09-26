import React from "react";
import { useQuery } from "@tanstack/react-query";
import { fetchAgents, type AgentSummary } from "../services/api";

export default function CanvasPage() {
  const { data: agents = [] } = useQuery<AgentSummary[]>({
    queryKey: ["agents", { scope: "my" }],
    queryFn: () => fetchAgents({ scope: "my" }),
    refetchInterval: 2000, // Poll every 2 seconds
  });

  return (
    <div className="canvas-page">
      <h1>Canvas (React Prototype)</h1>
      <div
        id="canvas-container"
        data-testid="canvas-container"
        className="canvas-container"
      >
        <div
          id="agent-shelf"
          data-testid="agent-shelf"
          className="agent-shelf"
        >
          <h3>Agents</h3>
          <div className="agent-shelf-content">
            {agents.length === 0 ? (
              <p>No agents available</p>
            ) : (
              agents.map((agent) => (
                <div
                  key={agent.id}
                  className="agent-shelf-item"
                  data-testid={`shelf-agent-${agent.id}`}
                  draggable={true}
                  onDragStart={(e) => {
                    e.dataTransfer.setData('agent-id', String(agent.id));
                    e.dataTransfer.setData('agent-name', agent.name);
                  }}
                >
                  <div className="agent-icon">🤖</div>
                  <div className="agent-name">{agent.name}</div>
                </div>
              ))
            )}
          </div>
        </div>

        <div
          className="canvas-workspace"
          data-testid="canvas-workspace"
          onDrop={(e) => {
            e.preventDefault();
            const agentId = e.dataTransfer.getData('agent-id');
            const agentName = e.dataTransfer.getData('agent-name');
            console.log('Dropped agent:', { agentId, agentName });
          }}
          onDragOver={(e) => e.preventDefault()}
        >
          <h3>Workflow Canvas</h3>
          <div className="canvas-drop-zone">
            <p>Drag agents and tools here to create workflows</p>
          </div>
        </div>

        <div
          id="tool-palette"
          data-testid="tool-palette"
          className="tool-palette"
        >
          <h3>Tools</h3>
          <div className="tool-palette-content">
            <div
              className="tool-palette-item"
              data-testid="tool-http-request"
              draggable={true}
              onDragStart={(e) => {
                e.dataTransfer.setData('tool-type', 'http-request');
                e.dataTransfer.setData('tool-name', 'HTTP Request');
              }}
            >
              <div className="tool-icon">🌐</div>
              <div className="tool-name">HTTP Request</div>
            </div>
            <div
              className="tool-palette-item"
              data-testid="tool-url-fetch"
              draggable={true}
              onDragStart={(e) => {
                e.dataTransfer.setData('tool-type', 'url-fetch');
                e.dataTransfer.setData('tool-name', 'URL Fetch');
              }}
            >
              <div className="tool-icon">📡</div>
              <div className="tool-name">URL Fetch</div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
