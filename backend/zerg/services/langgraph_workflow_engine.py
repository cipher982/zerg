"""
LangGraph-based Workflow Execution Engine.

This is a proof-of-concept implementation to replace the custom DAG engine
with LangGraph's StateGraph for better observability and state management.
"""

from __future__ import annotations

import asyncio
import logging
import operator
import os
from datetime import datetime
from datetime import timezone
from typing import Annotated
from typing import Any
from typing import Dict
from typing import List
from typing import Optional
from typing import TypedDict
from typing import Union

from langgraph.checkpoint.memory import MemorySaver

# TODO: Fix imports for proper SQLite checkpointing
# from langgraph.checkpoint.sqlite.aio import AsyncSqliteSaver
from langgraph.graph import END
from langgraph.graph import START
from langgraph.graph import StateGraph

from zerg.crud import crud
from zerg.database import get_session_factory
from zerg.events import EventType
from zerg.events import event_bus
from zerg.managers.agent_runner import AgentRunner
from zerg.models.models import Agent
from zerg.models.models import NodeExecutionState
from zerg.models.models import Workflow
from zerg.models.models import WorkflowExecution
from zerg.schemas.workflow_schema import NodeTypeHelper
from zerg.schemas.workflow_schema import WorkflowCanvas
from zerg.services.canvas_transformer import CanvasTransformer
from zerg.tools.registry import get_registry

logger = logging.getLogger(__name__)

# Configure LangSmith tracing if enabled
if os.getenv("LANGCHAIN_TRACING_V2", "false").lower() == "true":
    os.environ.setdefault("LANGCHAIN_PROJECT", "zerg-workflows")
    logger.info("LangSmith tracing enabled for project: zerg-workflows")


def merge_dicts(left: Dict[str, Any], right: Dict[str, Any]) -> Dict[str, Any]:
    """Merge two dictionaries for LangGraph state updates."""
    return {**left, **right}


class WorkflowState(TypedDict):
    """State passed between nodes in the LangGraph workflow."""

    execution_id: int
    node_outputs: Annotated[Dict[str, Any], merge_dicts]  # Node outputs keyed by node_id
    completed_nodes: Annotated[List[str], operator.add]  # Track completed nodes
    error: Union[str, None]
    # Note: db_session_factory removed - not serializable for checkpointing


class LangGraphWorkflowEngine:
    """LangGraph-based workflow execution engine."""

    async def execute_workflow_with_id(
        self, workflow_id: int, execution_id: int, trigger_type: str = "manual", trigger_config: Dict[str, Any] = None
    ) -> None:
        """Execute workflow using an existing execution record."""

        logger.info(
            "[LangGraphEngine] Starting execution with existing ID – workflow_id=%s execution_id=%s",
            workflow_id,
            execution_id,
        )

        session_factory = get_session_factory()

        with session_factory() as db:
            # Get workflow
            workflow: Union[Workflow, None] = db.query(Workflow).filter_by(id=workflow_id, is_active=True).first()
            if workflow is None:
                raise ValueError("Workflow not found or inactive")

            # Get existing execution record
            execution: Union[WorkflowExecution, None] = db.query(WorkflowExecution).filter_by(id=execution_id).first()
            if execution is None:
                raise ValueError("Execution record not found")

            try:
                # Execute the workflow using the existing execution_id
                await self._execute_workflow_internal(workflow, execution, db)
            except Exception as e:
                logger.error(f"[LangGraphEngine] Execution failed: {e}")
                # Update execution status to failed
                execution.status = "failed"
                execution.finished_at = datetime.now(timezone.utc)
                execution.error = str(e)
                db.commit()

                # Publish failure event
                await event_bus.publish(
                    EventType.EXECUTION_FINISHED,
                    {
                        "execution_id": execution.id,
                        "status": "failed",
                        "error": str(e),
                        "duration_ms": int((execution.finished_at - execution.started_at).total_seconds() * 1000),
                    },
                )
                raise

    async def execute_workflow(
        self, workflow_id: int, trigger_type: str = "manual", trigger_config: Dict[str, Any] = None
    ) -> int:
        """Execute workflow using LangGraph StateGraph."""

        logger.info("[LangGraphEngine] Starting execution – workflow_id=%s trigger_type=%s", workflow_id, trigger_type)

        session_factory = get_session_factory()

        with session_factory() as db:
            # Get workflow
            workflow: Union[Workflow, None] = db.query(Workflow).filter_by(id=workflow_id, is_active=True).first()
            if workflow is None:
                raise ValueError("Workflow not found or inactive")

            # Create execution record
            execution = WorkflowExecution(
                workflow_id=workflow_id,
                status="running",
                started_at=datetime.now(timezone.utc),
                triggered_by=trigger_type,
            )
            db.add(execution)
            db.commit()

            try:
                # Execute the workflow using the execution record
                await self._execute_workflow_internal(workflow, execution, db)
                return execution.id

            except Exception as e:
                # Mark as failed
                execution.status = "failed"
                execution.error = str(e)
                execution.finished_at = datetime.now(timezone.utc)
                db.commit()

                self._publish_execution_finished(
                    execution_id=execution.id, status="failed", error=str(e), duration_ms=self._duration_ms(execution)
                )

                logger.exception("[LangGraphEngine] Execution failed – execution_id=%s", execution.id)
                raise

    async def _execute_workflow_internal(self, workflow: Workflow, execution: WorkflowExecution, db) -> None:
        """Internal method to execute workflow with existing execution record."""
        # Transform canvas data first
        canvas = CanvasTransformer.from_database(workflow.canvas_data)

        # Handle empty workflows gracefully - complete immediately
        if not canvas.nodes:
            logger.info(f"[LangGraphEngine] Empty workflow {workflow.id} - completing immediately")
            execution.status = "success"
            execution.finished_at = datetime.now(timezone.utc)
            db.commit()

            # Publish completion event
            await event_bus.publish(
                EventType.EXECUTION_FINISHED,
                {
                    "execution_id": execution.id,
                    "workflow_id": workflow.id,
                    "status": "success",
                    "event_type": EventType.EXECUTION_FINISHED,
                },
            )
            return

        # Use checkpointer context manager
        # TODO: Fix for proper SQLite checkpointing
        checkpointer = MemorySaver()
        # async with AsyncSqliteSaver.from_conn_string("checkpoints.db") as checkpointer:
        # Build LangGraph from canvas_data
        graph = self._build_langgraph(canvas, execution.id, checkpointer)

        # Execute the graph with checkpointing
        initial_state = WorkflowState(
            execution_id=execution.id,
            node_outputs={},
            completed_nodes=[],
            error=None,
        )

        # Use thread_id for checkpoint isolation
        config = {"configurable": {"thread_id": f"workflow_{execution.id}"}}

        # Stream execution for real-time updates
        final_state = None
        node_count = 0
        chunk_count = 0

        logger.info(f"[LangGraphEngine] Starting streaming execution – workflow_id={workflow.id}")
        logger.info(f"[LangGraphEngine] Initial state: {initial_state}")
        logger.info(f"[LangGraphEngine] LangGraph config: {config}")

        async for chunk in graph.astream(initial_state, config):
            chunk_count += 1
            logger.info(f"[LangGraphEngine] Received chunk #{chunk_count}: {chunk}")

            # Each chunk contains state updates as nodes complete.
            # LangGraph returns chunks in the form:
            # {'node_id': {'completed_nodes': [...], 'node_outputs': {...}}}
            if chunk:
                # Process each node's state update in the chunk
                for node_id, state_update in chunk.items():
                    # State updates should always be dicts from LangGraph, but check defensively
                    if state_update and hasattr(state_update, "get"):
                        logger.info(f"[LangGraphEngine] Processing state update from node {node_id}: {state_update}")
                        final_state = state_update
                        current_completed = len(state_update.get("completed_nodes", []))

                        # Log progress for new completed nodes
                        if current_completed > node_count:
                            newly_completed = current_completed - node_count
                            node_count = current_completed
                            logger.info(
                                f"[LangGraphEngine] Progress update: {newly_completed} new nodes "
                                f"completed ({node_count} total)"
                            )
                            completed_nodes = state_update.get("completed_nodes", [])
                            if newly_completed > 0 and len(completed_nodes) >= newly_completed:
                                newly_completed_nodes = completed_nodes[-newly_completed:]
                                logger.info(f"[LangGraphEngine] Newly completed nodes: {newly_completed_nodes}")

                        # Publish streaming progress event
                        self._publish_streaming_progress(
                            execution_id=execution.id,
                            completed_nodes=state_update.get("completed_nodes", []),
                            node_outputs=state_update.get("node_outputs", {}),
                            error=state_update.get("error"),
                        )
            else:
                logger.warning(f"[LangGraphEngine] Received empty chunk #{chunk_count}")

        logger.info(f"[LangGraphEngine] Streaming completed after {chunk_count} chunks")

        # Log completion summary
        completed_count = len(final_state.get("completed_nodes", [])) if final_state else 0
        logger.info(f"[LangGraphEngine] Streaming execution completed – {completed_count} nodes executed")

        # Mark as successful
        execution.status = "success"
        execution.finished_at = datetime.now(timezone.utc)
        db.commit()

        self._publish_execution_finished(
            execution_id=execution.id, status="success", error=None, duration_ms=self._duration_ms(execution)
        )

        logger.info("[LangGraphEngine] Execution completed – execution_id=%s", execution.id)

    def _build_langgraph(self, canvas: WorkflowCanvas, execution_id: int, checkpointer) -> StateGraph:
        """Convert WorkflowCanvas to LangGraph StateGraph."""

        workflow = StateGraph(WorkflowState)

        nodes = canvas.nodes
        edges = canvas.edges

        logger.info(f"[LangGraphEngine] Canvas nodes: {[n.node_id for n in nodes]}")
        logger.info(f"[LangGraphEngine] Canvas edges: {[(e.from_node_id, e.to_node_id) for e in edges]}")

        # Store for use in node creation functions
        self._current_nodes = nodes
        self._current_edges = edges

        # Filter nodes to only include those reachable from trigger nodes
        connected_nodes = self._get_connected_nodes(nodes, edges)
        logger.info(f"[LangGraphEngine] Filtered to connected nodes: {[n.node_id for n in connected_nodes]}")

        # Add only connected nodes to the graph
        for node in connected_nodes:
            node_id = node.node_id
            node_type_raw = node.node_type
            # Use clean NodeTypeHelper instead of isinstance checks
            node_type, typed_config = NodeTypeHelper.parse_node_type(node_type_raw)

            logger.info(f"[LangGraphEngine] Processing node: {node_id} (type={node_type})")

            # Create node execution function based on type
            if node_type == "tool":
                logger.info(f"[LangGraphEngine] Creating tool node for {node_id}")
                node_func = self._create_tool_node(node)
            elif node_type == "agentidentity" or node_type == "agent":
                # Extract agent_id from typed config or fallback to node config
                agent_id = None
                if typed_config and hasattr(typed_config, "agent_id"):
                    agent_id = typed_config.agent_id
                elif node.config:
                    agent_id = node.config.get("agent_id")
                else:
                    # For AgentIdentity nodes, try to extract from node_type dict
                    if isinstance(node.node_type, dict):
                        for key, value in node.node_type.items():
                            if key.lower() == "agentidentity" and isinstance(value, dict) and "agent_id" in value:
                                agent_id = value["agent_id"]
                                break

                logger.info(f"[LangGraphEngine] Creating agent node for {node_id} (agent_id={agent_id})")
                node_func = self._create_agent_node(node)
            elif node_type == "trigger":
                logger.info(f"[LangGraphEngine] Creating trigger node for {node_id}")
                node_func = self._create_trigger_node(node)
            else:
                logger.info(f"[LangGraphEngine] Creating placeholder node for {node_id} (unknown type: {node_type})")
                node_func = self._create_placeholder_node(node)

            workflow.add_node(node_id, node_func)
            logger.info(f"[LangGraphEngine] Added node {node_id} to LangGraph workflow")

        # Add edges between nodes
        start_nodes = []
        end_nodes = []

        # Get all valid node IDs for validation (only connected nodes)
        valid_node_ids = {node.node_id for node in connected_nodes}

        logger.info(f"[LangGraphEngine] Valid node IDs: {valid_node_ids}")
        logger.info(f"[LangGraphEngine] Edges to process: {edges}")

        # Find nodes with no incoming edges (start nodes) - only from connected nodes
        target_nodes = {edge.to_node_id for edge in edges if edge.to_node_id in valid_node_ids}
        source_nodes = {edge.from_node_id for edge in edges if edge.from_node_id in valid_node_ids}

        for node in connected_nodes:
            node_id = node.node_id
            if node_id not in target_nodes:
                start_nodes.append(node_id)
            if node_id not in source_nodes:
                end_nodes.append(node_id)

        logger.info(f"[LangGraphEngine] Start nodes (no incoming edges): {start_nodes}")
        logger.info(f"[LangGraphEngine] End nodes (no outgoing edges): {end_nodes}")

        # Connect START to all start nodes
        for start_node in start_nodes:
            logger.info(f"[LangGraphEngine] Adding edge: START -> {start_node}")
            workflow.add_edge(START, start_node)

        # Add internal edges - with validation
        for edge in edges:
            source = edge.from_node_id
            target = edge.to_node_id

            logger.info(f"[LangGraphEngine] Processing edge: {source} -> {target}")

            # Validate both source and target exist
            if source not in valid_node_ids:
                logger.warning(f"[LangGraphEngine] Skipping edge - source node not found: {source}")
                continue
            if target not in valid_node_ids:
                logger.warning(f"[LangGraphEngine] Skipping edge - target node not found: {target}")
                continue

            logger.info(f"[LangGraphEngine] Adding valid edge: {source} -> {target}")
            workflow.add_edge(source, target)

        # Connect all end nodes to END
        for end_node in end_nodes:
            logger.info(f"[LangGraphEngine] Adding edge: {end_node} -> END")
            workflow.add_edge(end_node, END)

        logger.info("[LangGraphEngine] Graph construction complete. Compiling with checkpointer...")

        return workflow.compile(checkpointer=checkpointer)

    def _get_connected_nodes(self, nodes: List, edges: List) -> List:
        """Filter nodes to only include those reachable from trigger nodes or all nodes for manual execution."""
        if not nodes:
            return []

        # Find all trigger nodes (these are always included)
        trigger_nodes = []
        non_trigger_nodes = []

        for node in nodes:
            node_type_raw = node.node_type
            # Use clean NodeTypeHelper instead of isinstance checks
            node_type, typed_config = NodeTypeHelper.parse_node_type(node_type_raw)

            if node_type == "trigger":
                trigger_nodes.append(node)
            else:
                non_trigger_nodes.append(node)

        # If no trigger nodes, for manual execution include all nodes
        if not trigger_nodes:
            logger.info("[LangGraphEngine] No trigger nodes found - including all nodes for execution")
            return nodes

        # Build adjacency graph from edges
        graph = {}
        for edge in edges:
            source = edge.from_node_id
            target = edge.to_node_id
            if source and target:
                if source not in graph:
                    graph[source] = []
                graph[source].append(target)

        # Find all nodes reachable from trigger nodes using BFS
        reachable = set()
        queue = []

        # Start with all trigger nodes
        for trigger_node in trigger_nodes:
            trigger_id = trigger_node.node_id
            reachable.add(trigger_id)
            queue.append(trigger_id)

        # BFS to find all reachable nodes
        while queue:
            current = queue.pop(0)
            if current in graph:
                for neighbor in graph[current]:
                    if neighbor not in reachable:
                        reachable.add(neighbor)
                        queue.append(neighbor)

        # Filter original nodes list to only include reachable nodes
        connected_nodes = []
        for node in nodes:
            node_id = node.node_id
            if node_id in reachable:
                connected_nodes.append(node)

        logger.info(f"[LangGraphEngine] Reachable node IDs: {reachable}")
        return connected_nodes

    def _get_node_id(self, node) -> str:
        """Get ID from any node-like object, handling different field names."""
        if hasattr(node, "node_id"):
            return node.node_id
        elif hasattr(node, "id"):
            return node.id
        else:
            raise AttributeError(f"Object {type(node)} has no recognizable ID field")

    def _find_node_by_id(self, node_id: str) -> Optional[Any]:
        """Find a node by its ID, handling different ID field names."""
        for node in self._current_nodes:
            if self._get_node_id(node) == node_id:
                return node
        return None

    def _get_node_type_by_id(self, node_id: str) -> str:
        """Get the type of a node by its ID."""
        node = self._find_node_by_id(node_id)
        if node:
            # Use clean NodeTypeHelper instead of isinstance checks
            node_type, _ = NodeTypeHelper.parse_node_type(node.node_type)
            return node_type
        return "unknown"

    def _has_outgoing_tool_connections(self, node_id: str) -> bool:
        """Check if a node has outgoing connections to tool nodes."""
        outgoing_edges = [edge for edge in self._current_edges if edge.from_node_id == node_id]
        return any(self._get_node_type_by_id(edge.to_node_id) == "tool" for edge in outgoing_edges)

    def _get_connected_tool_names(self, node_id: str) -> List[str]:
        """Get names of tools connected to this node."""
        outgoing_edges = [edge for edge in self._current_edges if edge.from_node_id == node_id]
        tool_names = []

        for edge in outgoing_edges:
            if self._get_node_type_by_id(edge.to_node_id) == "tool":
                # Find the tool node and extract its tool_name
                node = self._find_node_by_id(edge.to_node_id)
                if node and hasattr(node, "node_type") and isinstance(node.node_type, dict):
                    if "Tool" in node.node_type:
                        tool_name = node.node_type["Tool"].get("tool_name")
                        if tool_name:
                            tool_names.append(tool_name)

        return tool_names

    def _has_incoming_agent_connections(self, node_id: str) -> bool:
        """Check if a node has incoming connections from agent nodes."""
        incoming_edges = [edge for edge in self._current_edges if edge.to_node_id == node_id]
        return any(
            self._get_node_type_by_id(edge.from_node_id) in ["agent", "agentidentity"] for edge in incoming_edges
        )

    def _get_connected_agent_ids(self, node_id: str) -> List[str]:
        """Get the IDs of agent nodes connected to this node."""
        incoming_edges = [edge for edge in self._current_edges if edge.to_node_id == node_id]
        return [
            edge.from_node_id
            for edge in incoming_edges
            if self._get_node_type_by_id(edge.from_node_id) in ["agent", "agentidentity"]
        ]

    def _create_tool_node(self, node_config):
        """Create a tool execution node function."""

        async def tool_node(state: WorkflowState) -> WorkflowState:
            node_id = node_config.node_id

            # Extract tool configuration using clean NodeTypeHelper
            node_type, typed_config = NodeTypeHelper.parse_node_type(node_config.node_type)

            if node_type == "tool" and typed_config:
                # Use typed configuration from schema
                tool_name = typed_config.tool_name
                tool_params = typed_config.static_params
            else:
                # Fallback for other formats or missing config
                tool_name = node_config.config.get("tool_name") or node_config.config.get("name", "")
                tool_params = node_config.config.get("parameters", {})

            # Check if this tool node has incoming connections from agents
            connected_agent_ids = self._get_connected_agent_ids(node_id)

            # If connected to agents, look for tool calls in their outputs
            if connected_agent_ids:
                for agent_id in connected_agent_ids:
                    agent_output = state["node_outputs"].get(agent_id, {})
                    if agent_output.get("type") == "agent_with_tools" and "tool_calls" in agent_output:
                        # Find the first tool call that matches this tool
                        matching_tool_call = None
                        for tool_call in agent_output["tool_calls"]:
                            if tool_call.get("tool") == tool_name:
                                matching_tool_call = tool_call
                                break

                        # If we found a matching tool call, use its parameters
                        if matching_tool_call:
                            tool_params = matching_tool_call.get("parameters", {})
                            break

            # Node execution - just store output at end, don't update intermediate state

            # Create node execution state
            session_factory = get_session_factory()
            with session_factory() as db:
                node_state = NodeExecutionState(
                    workflow_execution_id=state["execution_id"], node_id=node_id, status="running"
                )
                db.add(node_state)
                db.commit()

                self._publish_node_event(
                    execution_id=state["execution_id"], node_id=node_id, status="running", output=None, error=None
                )

                try:
                    # Execute tool
                    registry = get_registry()
                    all_tools = {t.name: t for t in registry.all_tools()}
                    tool = all_tools.get(tool_name)

                    if not tool:
                        raise ValueError(f"Tool '{tool_name}' not found")

                    result = await tool.ainvoke(tool_params)

                    # Store result
                    output = {"tool_name": tool_name, "parameters": tool_params, "result": result, "type": "tool"}
                    # Update node state
                    node_state.status = "success"
                    node_state.output = output
                    db.commit()

                    self._publish_node_event(
                        execution_id=state["execution_id"], node_id=node_id, status="success", output=output, error=None
                    )

                    # Return only the changes to state
                    return {"node_outputs": {node_id: output}, "completed_nodes": [node_id]}

                except Exception as e:
                    # Mark as failed
                    node_state.status = "failed"
                    node_state.error = str(e)
                    db.commit()

                    self._publish_node_event(
                        execution_id=state["execution_id"], node_id=node_id, status="failed", output=None, error=str(e)
                    )

                    raise  # Re-raise without modifying state

            # Should not reach here
            return {}

        return tool_node

    def _create_agent_node(self, node_config):
        """Create an agent execution node function."""

        async def agent_node(state: WorkflowState) -> WorkflowState:
            node_id = node_config.node_id

            # Extract agent_id from node configuration
            agent_id = None
            if node_config.config:
                agent_id = node_config.config.get("agent_id")

            # If not found in config, try to extract from node_type dict (AgentIdentity format)
            if agent_id is None and isinstance(node_config.node_type, dict):
                for key, value in node_config.node_type.items():
                    if key.lower() == "agentidentity" and isinstance(value, dict) and "agent_id" in value:
                        agent_id = value["agent_id"]
                        break

            logger.info(
                (
                    f"[AgentNode] Starting execution – node_id={node_id}, agent_id={agent_id}, "
                    f"execution_id={state['execution_id']}"
                )
            )

            # Node execution - just store output at end, don't update intermediate state

            session_factory = get_session_factory()
            with session_factory() as db:
                node_state = NodeExecutionState(
                    workflow_execution_id=state["execution_id"], node_id=node_id, status="running"
                )
                db.add(node_state)
                db.commit()
                logger.info("[AgentNode] Created node_state in DB with status 'running'")

                self._publish_node_event(
                    execution_id=state["execution_id"], node_id=node_id, status="running", output=None, error=None
                )

                try:
                    # Get agent
                    logger.info(f"[AgentNode] Querying database for agent_id={agent_id}")
                    agent = db.query(Agent).filter_by(id=agent_id).first()
                    if not agent:
                        error_msg = f"Agent {agent_id} not found in database"
                        logger.error(f"[AgentNode] {error_msg}")
                        raise ValueError(error_msg)

                    logger.info(f"[AgentNode] Found agent: {agent.name} (id={agent.id})")

                    # Create thread and execute
                    logger.info(f"[AgentNode] Creating thread for agent_id={agent_id}")
                    thread = crud.create_thread(
                        db=db,
                        agent_id=agent_id,
                        title=f"Workflow execution {state['execution_id']}",
                    )
                    logger.info(f"[AgentNode] Created thread with id={thread.id}")

                    # Configure agent with connected tools before running
                    connects_to_tools = self._has_outgoing_tool_connections(node_id)

                    # Get the base message from config and enhance it if tools are connected
                    base_message = node_config.config.get("message", "Execute this task")

                    # If agent connects to tools, enhance the message to encourage tool usage
                    if connects_to_tools:
                        connected_tool_names = self._get_connected_tool_names(node_id)
                        if "http_request" in connected_tool_names:
                            user_message = f"{base_message}. Use the http_request tool to make HTTP requests as needed. For example, you could test connectivity by making a request to https://httpbin.org/get"
                        else:
                            user_message = f"{base_message}. You have access to these tools: {', '.join(connected_tool_names)}. Use them as appropriate for the task."
                    else:
                        user_message = base_message

                    logger.info(f"[AgentNode] Creating user message: '{user_message}'")
                    crud.create_thread_message(
                        db=db, thread_id=thread.id, role="user", content=user_message, processed=False
                    )
                    logger.info("[AgentNode] Created thread message in DB")
                    agent_for_runner = agent  # Default to original agent

                    if connects_to_tools:
                        logger.info(f"[AgentNode] Agent connects to tools: {connected_tool_names}")

                        # Create a simple object that mimics the agent but with modified allowed_tools
                        class AgentProxy:
                            def __init__(self, original_agent, tool_list):
                                # Copy all attributes from original agent
                                for attr in dir(original_agent):
                                    if not attr.startswith("_") and hasattr(original_agent, attr):
                                        try:
                                            setattr(self, attr, getattr(original_agent, attr))
                                        except AttributeError:
                                            pass  # Skip read-only attributes
                                # Override allowed_tools with our list
                                self.allowed_tools = tool_list

                        # Get current tools
                        current_tools = agent.allowed_tools if agent.allowed_tools else []
                        if isinstance(current_tools, dict):
                            current_tools = []  # Reset if it's a dict, we need a list
                        elif current_tools is None:
                            current_tools = []

                        # Combine current tools with connected tools
                        combined_tools = list(set(current_tools + connected_tool_names))

                        # Create agent proxy with combined tools
                        agent_for_runner = AgentProxy(agent, combined_tools)
                        logger.info(f"[AgentNode] Created agent proxy with allowed_tools: {combined_tools}")

                    logger.info(f"[AgentNode] Initializing AgentRunner for agent {agent.id}")
                    runner = AgentRunner(agent_for_runner)
                    logger.info("[AgentNode] AgentRunner initialized, calling run_thread...")

                    created_messages = await runner.run_thread(db, thread)
                    logger.info(
                        f"[AgentNode] AgentRunner.run_thread() completed. Created {len(created_messages)} messages"
                    )

                    for i, msg in enumerate(created_messages):
                        logger.info(
                            (
                                f"[AgentNode] Message {i}: role={msg.role}, "
                                f"content_length={len(msg.content) if hasattr(msg, 'content') else 0}"
                            )
                        )

                    assistant_messages = [msg for msg in created_messages if msg.role == "assistant"]
                    logger.info(f"[AgentNode] Filtered to {len(assistant_messages)} assistant messages")

                    if connects_to_tools and assistant_messages:
                        # Extract tool calls from the last assistant message
                        last_message = assistant_messages[-1]
                        logger.info("[AgentNode] Checking last assistant message for tool calls...")
                        if hasattr(last_message, "tool_calls") and last_message.tool_calls:
                            logger.info(f"[AgentNode] Found {len(last_message.tool_calls)} tool calls")
                            # Output structured tool calls for connected tool nodes
                            output = {
                                "agent_id": agent_id,
                                "agent_name": agent.name,
                                "message": user_message,
                                "tool_calls": [
                                    {
                                        "tool": tc.get("name", ""),
                                        "parameters": tc.get("args", {}),
                                        "id": tc.get("id", ""),
                                    }
                                    for tc in last_message.tool_calls
                                ],
                                "type": "agent_with_tools",
                            }
                        else:
                            logger.info("[AgentNode] No tool calls found, using regular agent response")
                            # Fallback: if agent didn't make tool calls, output response
                            result = (
                                last_message.content if hasattr(last_message, "content") else "No response generated"
                            )
                            output = {
                                "agent_id": agent_id,
                                "agent_name": agent.name,
                                "message": user_message,
                                "response": result,
                                "type": "agent",
                            }
                    else:
                        # Normal agent output when not connected to tools
                        logger.info(
                            "[AgentNode] Using normal agent output (no tool connections or no assistant messages)"
                        )
                        result = assistant_messages[-1].content if assistant_messages else "No response generated"
                        output = {
                            "agent_id": agent_id,
                            "agent_name": agent.name,
                            "message": user_message,
                            "response": result,
                            "type": "agent",
                        }

                    logger.info(f"[AgentNode] Final output: {output}")

                    node_state.status = "success"
                    node_state.output = output
                    db.commit()
                    logger.info("[AgentNode] Updated node_state to 'success' in DB")

                    self._publish_node_event(
                        execution_id=state["execution_id"], node_id=node_id, status="success", output=output, error=None
                    )

                    # Return only the changes to state
                    state_update = {"node_outputs": {node_id: output}, "completed_nodes": [node_id]}
                    logger.info(f"[AgentNode] Returning state update: {state_update}")
                    return state_update

                except Exception as e:
                    error_msg = f"Exception in agent node {node_id} (agent_id={agent_id}): {e}"
                    logger.exception(f"[AgentNode] {error_msg}")

                    node_state.status = "failed"
                    node_state.error = str(e)
                    db.commit()
                    logger.error("[AgentNode] Updated node_state to 'failed' in DB")

                    self._publish_node_event(
                        execution_id=state["execution_id"], node_id=node_id, status="failed", output=None, error=str(e)
                    )

                    logger.error("[AgentNode] Re-raising exception to fail workflow execution")
                    raise  # Re-raise without modifying state

            # Should not reach here
            logger.warning("[AgentNode] Reached unexpected code path, returning empty state")
            return {}

        return agent_node

    def _create_trigger_node(self, node_config):
        """Create a trigger execution node function."""

        async def trigger_node(state: WorkflowState) -> WorkflowState:
            node_id = node_config.node_id
            trigger_type = node_config.config.get("trigger_type", "webhook")

            # Node execution - just store output at end, don't update intermediate state

            # For now, just simulate trigger creation
            output = {"trigger_type": trigger_type, "status": "created", "type": "trigger"}
            state["node_outputs"][node_id] = output

            return state

        return trigger_node

    def _create_placeholder_node(self, node_config):
        """Create a placeholder node for unknown types."""

        async def placeholder_node(state: WorkflowState) -> WorkflowState:
            node_id = node_config.node_id
            node_type = str(node_config.node_type)

            logger.info(f"[LangGraphEngine] *** PLACEHOLDER NODE {node_id} EXECUTING ***")

            # Create node execution state
            session_factory = get_session_factory()
            with session_factory() as db:
                node_state = NodeExecutionState(
                    workflow_execution_id=state["execution_id"], node_id=node_id, status="running"
                )
                db.add(node_state)
                db.commit()

                self._publish_node_event(
                    execution_id=state["execution_id"], node_id=node_id, status="running", output=None, error=None
                )

                try:
                    # Simulate execution
                    await asyncio.sleep(0.1)

                    output = {"result": f"{node_type}_executed", "type": "placeholder"}

                    # Update node state to success
                    node_state.status = "success"
                    node_state.output = output
                    db.commit()

                    self._publish_node_event(
                        execution_id=state["execution_id"], node_id=node_id, status="success", output=output, error=None
                    )

                    # Return only the changes to state
                    return {"node_outputs": {node_id: output}, "completed_nodes": [node_id]}

                except Exception as e:
                    error_msg = str(e)
                    logger.error(f"[LangGraphEngine] Placeholder node {node_id} failed: {error_msg}")

                    # Update node state to failed
                    node_state.status = "failed"
                    node_state.error = error_msg
                    db.commit()

                    self._publish_node_event(
                        execution_id=state["execution_id"],
                        node_id=node_id,
                        status="failed",
                        output=None,
                        error=error_msg,
                    )

                    # Return error state
                    return {"error": error_msg}

        return placeholder_node

    def _publish_node_event(
        self,
        *,
        execution_id: int,
        node_id: str,
        status: str,
        output: Union[Dict[str, Any], None],
        error: Union[str, None],
    ):
        """Publish node state change event."""

        payload = {
            "execution_id": execution_id,
            "node_id": node_id,
            "status": status,
            "output": output,
            "error": error,
            "event_type": EventType.NODE_STATE_CHANGED,
        }

        try:
            loop = asyncio.get_running_loop()
            loop.create_task(event_bus.publish(EventType.NODE_STATE_CHANGED, payload))
        except RuntimeError:
            loop = asyncio.new_event_loop()
            asyncio.set_event_loop(loop)
            try:
                loop.run_until_complete(event_bus.publish(EventType.NODE_STATE_CHANGED, payload))
            finally:
                loop.close()

    def _publish_node_log(
        self,
        *,
        execution_id: int,
        node_id: str,
        stream: str,
        text: str,
    ) -> None:
        """Publish NODE_LOG event carrying a single log line."""

        payload = {
            "execution_id": execution_id,
            "node_id": node_id,
            "stream": stream,
            "text": text,
            "event_type": EventType.NODE_LOG,
        }

        try:
            loop = asyncio.get_running_loop()
            loop.create_task(event_bus.publish(EventType.NODE_LOG, payload))
        except RuntimeError:
            loop = asyncio.new_event_loop()
            asyncio.set_event_loop(loop)
            try:
                loop.run_until_complete(event_bus.publish(EventType.NODE_LOG, payload))
            finally:
                loop.close()

    def _publish_streaming_progress(
        self,
        *,
        execution_id: int,
        completed_nodes: List[str],
        node_outputs: Dict[str, Any],
        error: Union[str, None],
    ):
        """Publish streaming progress update event."""

        payload = {
            "execution_id": execution_id,
            "completed_nodes": completed_nodes,
            "node_outputs": node_outputs,
            "error": error,
            "event_type": EventType.WORKFLOW_PROGRESS,
        }

        try:
            loop = asyncio.get_running_loop()
            loop.create_task(event_bus.publish(EventType.WORKFLOW_PROGRESS, payload))
        except RuntimeError:
            loop = asyncio.new_event_loop()
            asyncio.set_event_loop(loop)
            try:
                loop.run_until_complete(event_bus.publish(EventType.WORKFLOW_PROGRESS, payload))
            finally:
                loop.close()

    def _publish_execution_finished(
        self, *, execution_id: int, status: str, error: Union[str, None], duration_ms: Union[int, None]
    ):
        """Publish execution finished event."""

        logger.info("Publishing execution_finished event for execution %s with status %s", execution_id, status)

        payload = {
            "execution_id": execution_id,
            "status": status,
            "error": error,
            "duration_ms": duration_ms,
            "event_type": EventType.EXECUTION_FINISHED,
        }

        try:
            loop = asyncio.get_running_loop()
            loop.create_task(event_bus.publish(EventType.EXECUTION_FINISHED, payload))
        except RuntimeError:
            loop = asyncio.new_event_loop()
            asyncio.set_event_loop(loop)
            try:
                loop.run_until_complete(event_bus.publish(EventType.EXECUTION_FINISHED, payload))
            finally:
                loop.close()

    @staticmethod
    def _duration_ms(execution: WorkflowExecution) -> Union[int, None]:
        if execution.started_at and execution.finished_at:
            delta = execution.finished_at - execution.started_at
            return int(delta.total_seconds() * 1000)
        return None


# Singleton instance
langgraph_workflow_engine = LangGraphWorkflowEngine()
