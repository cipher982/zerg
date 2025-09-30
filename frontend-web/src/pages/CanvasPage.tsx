import React, { useCallback, useState, useMemo } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  addEdge,
  useNodesState,
  useEdgesState,
  type Node,
  type Edge,
  type Connection,
  type OnConnect,
  type OnNodesChange,
  type OnEdgesChange,
  type NodeTypes,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import "../styles/canvas-react.css";
import toast from "react-hot-toast";
import {
  fetchAgents,
  fetchCurrentWorkflow,
  updateWorkflowCanvas,
  type AgentSummary,
  type Workflow,
  type WorkflowData,
  type WorkflowDataInput,
  type WorkflowNode,
  type WorkflowEdge,
} from "../services/api";

// Custom node component for agents
function AgentNode({ data }: { data: { label: string; agentId?: number } }) {
  return (
    <div className="agent-node">
      <div className="agent-icon">🤖</div>
      <div className="agent-name">{data.label}</div>
    </div>
  );
}

// Custom node component for tools
function ToolNode({ data }: { data: { label: string; toolType?: string } }) {
  const icon = data.toolType === 'http-request' ? '🌐' : data.toolType === 'url-fetch' ? '📡' : '🔧';

  return (
    <div className="tool-node">
      <div className="tool-icon">{icon}</div>
      <div className="tool-name">{data.label}</div>
    </div>
  );
}

// Custom node component for triggers
function TriggerNode({ data }: { data: { label: string } }) {
  return (
    <div className="trigger-node">
      <div className="trigger-icon">⚡</div>
      <div className="trigger-name">{data.label}</div>
    </div>
  );
}

const nodeTypes: NodeTypes = {
  agent: AgentNode,
  tool: ToolNode,
  trigger: TriggerNode,
};

// Convert backend WorkflowData to React Flow format
function convertToReactFlowData(workflowData: WorkflowData): { nodes: Node[]; edges: Edge[] } {
  const nodes: Node[] = workflowData.nodes.map((node: WorkflowNode) => ({
    id: node.id,
    type: node.type,
    position: { x: node.position.x, y: node.position.y },
    data: {
      label: (node.config as any)?.text || `${node.type} node`,
      agentId: (node.config as any)?.agent_id,
      toolType: (node.config as any)?.tool_type,
    },
  }));

  const edges: Edge[] = workflowData.edges.map((edge: WorkflowEdge) => ({
    id: `${edge.from_node_id}-${edge.to_node_id}`,
    source: edge.from_node_id,
    target: edge.to_node_id,
  }));

  return { nodes, edges };
}

// Convert React Flow data to backend WorkflowData format
function convertToBackendFormat(nodes: Node[], edges: Edge[]): WorkflowDataInput {
  const workflowNodes: WorkflowNode[] = nodes.map((node) => ({
    id: node.id,
    type: node.type as "agent" | "tool" | "trigger" | "conditional",
    position: { x: node.position.x, y: node.position.y },
    config: {
      text: node.data.label,
      agent_id: node.data.agentId,
      tool_type: node.data.toolType,
    } as any,
  }));

  const workflowEdges: WorkflowEdge[] = edges.map((edge) => ({
    from_node_id: edge.source,
    to_node_id: edge.target,
    config: {},
  }));

  return {
    nodes: workflowNodes,
    edges: workflowEdges,
  };
}

export default function CanvasPage() {
  const queryClient = useQueryClient();
  const [nodes, setNodes, onNodesChange] = useNodesState<Node>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);
  const [isLoading, setIsLoading] = useState(false);

  // Fetch agents for the shelf
  const { data: agents = [] } = useQuery<AgentSummary[]>({
    queryKey: ["agents", { scope: "my" }],
    queryFn: () => fetchAgents({ scope: "my" }),
    refetchInterval: 2000, // Poll every 2 seconds
  });

  // Fetch current workflow
  const { data: workflow } = useQuery<Workflow>({
    queryKey: ["workflow", "current"],
    queryFn: fetchCurrentWorkflow,
    staleTime: 30000, // Consider data fresh for 30 seconds
  });

  // Initialize nodes and edges from workflow data
  React.useEffect(() => {
    if (workflow?.canvas) {
      const { nodes: flowNodes, edges: flowEdges } = convertToReactFlowData(workflow.canvas);
      setNodes(flowNodes);
      setEdges(flowEdges);
    }
  }, [workflow, setNodes, setEdges]);

  // Save workflow mutation
  const saveWorkflowMutation = useMutation({
    mutationFn: updateWorkflowCanvas,
    onSuccess: () => {
      toast.success("Workflow saved successfully");
      queryClient.invalidateQueries({ queryKey: ["workflow", "current"] });
    },
    onError: (error: any) => {
      console.error("Failed to save workflow:", error);
      toast.error(`Failed to save workflow: ${error.message || "Unknown error"}`);
    },
  });

  // Auto-save workflow when nodes or edges change
  const debouncedSave = useCallback(
    debounce((nodes: Node[], edges: Edge[]) => {
      if (nodes.length > 0 || edges.length > 0) {
        const workflowData = convertToBackendFormat(nodes, edges);
        saveWorkflowMutation.mutate(workflowData);
      }
    }, 1000),
    [saveWorkflowMutation]
  );

  React.useEffect(() => {
    debouncedSave(nodes, edges);
  }, [nodes, edges, debouncedSave]);

  // Handle connection creation
  const onConnect: OnConnect = useCallback(
    (connection: Connection) => {
      setEdges((eds: Edge[]) => addEdge(connection, eds));
    },
    [setEdges]
  );

  // Handle drag and drop from agent shelf
  const onDrop = useCallback(
    (event: React.DragEvent) => {
      event.preventDefault();

      const reactFlowBounds = (event.target as Element)?.closest('.react-flow')?.getBoundingClientRect();
      if (!reactFlowBounds) return;

      const position = {
        x: event.clientX - reactFlowBounds.left,
        y: event.clientY - reactFlowBounds.top,
      };

      const agentId = event.dataTransfer.getData('agent-id');
      const agentName = event.dataTransfer.getData('agent-name');
      const toolType = event.dataTransfer.getData('tool-type');
      const toolName = event.dataTransfer.getData('tool-name');

      if (agentId && agentName) {
        const newNode: Node = {
          id: `agent-${Date.now()}`,
          type: 'agent',
          position,
          data: {
            label: agentName,
            agentId: parseInt(agentId, 10),
          },
        };
        setNodes((nds: Node[]) => [...nds, newNode]);
      } else if (toolType && toolName) {
        const newNode: Node = {
          id: `tool-${Date.now()}`,
          type: 'tool',
          position,
          data: {
            label: toolName,
            toolType,
          },
        };
        setNodes((nds: Node[]) => [...nds, newNode]);
      }
    },
    [setNodes]
  );

  const onDragOver = useCallback((event: React.DragEvent) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
  }, []);

  return (
    <div className="canvas-page">
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

        <div className="main-content-area">
          <div className="canvas-workspace" data-testid="canvas-workspace">
            <div style={{ width: '100%', height: '100%', minHeight: '600px' }}>
              <ReactFlow
                nodes={nodes}
                edges={edges}
                onNodesChange={onNodesChange}
                onEdgesChange={onEdgesChange}
                onConnect={onConnect}
                onDrop={onDrop}
                onDragOver={onDragOver}
                nodeTypes={nodeTypes}
                fitView
              >
                <Background />
                <Controls />
                <MiniMap />
              </ReactFlow>
            </div>
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

      {(isLoading || saveWorkflowMutation.isPending) && (
        <div className="saving-indicator">
          Saving workflow...
        </div>
      )}
    </div>
  );
}

// Simple debounce utility function
function debounce<T extends (...args: any[]) => void>(
  func: T,
  wait: number
): (...args: Parameters<T>) => void {
  let timeout: number | undefined;
  return (...args: Parameters<T>) => {
    clearTimeout(timeout);
    timeout = window.setTimeout(() => func(...args), wait);
  };
}
