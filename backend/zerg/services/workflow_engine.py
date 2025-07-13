"""
Workflow Execution Engine

Zero runtime parsing. Direct field access. Bulletproof execution.
Once workflows are loaded, it's impossible to have malformed data.

Key principles:
1. Direct field access - node.agent_id.value, not extract_agent_id(node)
2. No runtime parsing - all validation done at load time
3. Clear error messages - impossible states fail at load, not execution
4. Type safety - canonical types prevent invalid workflows
"""

from __future__ import annotations

import logging
import operator
from datetime import datetime
from datetime import timezone
from typing import Annotated
from typing import Any
from typing import Dict
from typing import List
from typing import Union

from langgraph.checkpoint.memory import MemorySaver
from langgraph.graph import END
from langgraph.graph import START
from langgraph.graph import StateGraph

from zerg.crud import crud
from zerg.database import get_session_factory
from zerg.events import EventType
from zerg.events.publisher import publish_event
from zerg.managers.agent_runner import AgentRunner
from zerg.models.models import Agent
from zerg.models.models import NodeExecutionState
from zerg.models.models import Workflow
from zerg.models.models import WorkflowExecution
from zerg.schemas.canonical_serialization import deserialize_workflow_from_database
from zerg.schemas.canonical_types import CanonicalNode
from zerg.schemas.canonical_types import CanonicalWorkflow
from zerg.tools.unified_access import get_tool_resolver

logger = logging.getLogger(__name__)


def merge_dicts(left: Dict[str, Any], right: Dict[str, Any]) -> Dict[str, Any]:
    """Merge two dictionaries for LangGraph state updates."""
    return {**left, **right}


class WorkflowState(dict):
    """State passed between nodes in the LangGraph workflow."""

    execution_id: int
    node_outputs: Annotated[Dict[str, Any], merge_dicts]
    completed_nodes: Annotated[List[str], operator.add]
    error: Union[str, None]


class WorkflowEngine:
    """
    Carmack-style workflow engine using canonical types.

    Zero parsing overhead. Direct field access. Bulletproof execution.
    """

    async def execute_workflow_with_id(
        self, workflow_id: int, execution_id: int, trigger_type: str = "manual", trigger_config: Dict[str, Any] = None
    ) -> None:
        """Execute workflow using an existing execution record."""
        logger.info(
            f"[WorkflowEngine] Execute with existing ID – workflow_id={workflow_id} execution_id={execution_id}"
        )

        session_factory = get_session_factory()

        with session_factory() as db:
            # Get existing execution record
            execution = db.query(WorkflowExecution).filter_by(id=execution_id).first()
            if execution is None:
                raise ValueError(f"Execution {execution_id} not found")

            # Update execution status to running
            execution.status = "running"
            execution.started_at = datetime.now(timezone.utc)
            execution.triggered_by = trigger_type
            db.commit()

            try:
                # Delegate to main execute method but with existing execution ID
                await self._execute_workflow_internal(workflow_id, execution, db)

            except Exception as e:
                # Mark as failed
                execution.status = "failed"
                execution.error = str(e)
                execution.finished_at = datetime.now(timezone.utc)
                db.commit()

                await self._publish_execution_finished(
                    execution_id=execution.id, status="failed", error=str(e), duration_ms=self._duration_ms(execution)
                )

                logger.exception(f"[CanonicalEngine] Execution failed – execution_id={execution.id}")
                raise

    async def execute_workflow(
        self, workflow_id: int, trigger_type: str = "manual", trigger_config: Dict[str, Any] = None
    ) -> int:
        """Execute workflow using canonical types."""

        logger.info(f"[CanonicalEngine] Starting execution – workflow_id={workflow_id} trigger_type={trigger_type}")

        session_factory = get_session_factory()

        with session_factory() as db:
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
                # Execute using unified internal method
                await self._execute_workflow_internal(workflow_id, execution, db)
                return execution.id

            except Exception as e:
                # Mark as failed
                execution.status = "failed"
                execution.error = str(e)
                execution.finished_at = datetime.now(timezone.utc)
                db.commit()

                await self._publish_execution_finished(
                    execution_id=execution.id, status="failed", error=str(e), duration_ms=self._duration_ms(execution)
                )

                logger.exception(f"[CanonicalEngine] Execution failed – execution_id={execution.id}")
                raise

    async def _execute_workflow_internal(self, workflow_id: int, execution: WorkflowExecution, db) -> None:
        """Internal workflow execution - SINGLE transformation point."""

        # Get workflow from database
        workflow_model = db.query(Workflow).filter_by(id=workflow_id, is_active=True).first()
        if workflow_model is None:
            raise ValueError("Workflow not found or inactive")

        # SINGLE TRANSFORMATION POINT - Convert to canonical format
        try:
            canonical_workflow = deserialize_workflow_from_database(
                workflow_model.canvas_data, workflow_model.id, workflow_model.name
            )
            logger.info(f"[CanonicalEngine] Loaded canonical workflow with {len(canonical_workflow.nodes)} nodes")
        except Exception as e:
            logger.error(f"[CanonicalEngine] Failed to load canonical workflow: {e}")
            raise ValueError(f"Invalid workflow data: {e}")

        # Execute using canonical workflow
        await self._execute_canonical_workflow(canonical_workflow, execution, db)

    async def _execute_canonical_workflow(self, workflow: CanonicalWorkflow, execution: WorkflowExecution, db) -> None:
        """Execute canonical workflow with zero parsing overhead."""

        # Handle empty workflows gracefully
        if not workflow.nodes:
            logger.info(f"[CanonicalEngine] Empty workflow {workflow.id} - completing immediately")
            execution.status = "success"
            execution.finished_at = datetime.now(timezone.utc)
            db.commit()

            await publish_event(
                EventType.EXECUTION_FINISHED,
                {
                    "execution_id": execution.id,
                    "workflow_id": workflow.id,
                    "status": "success",
                    "event_type": EventType.EXECUTION_FINISHED,
                },
            )
            return

        # Build LangGraph from canonical workflow
        checkpointer = MemorySaver()
        graph = self._build_canonical_langgraph(workflow, execution.id, checkpointer)

        # Execute the graph
        initial_state = WorkflowState(
            execution_id=execution.id,
            node_outputs={},
            completed_nodes=[],
            error=None,
        )

        config = {"configurable": {"thread_id": f"workflow_{execution.id}"}}

        logger.info(f"[CanonicalEngine] Starting streaming execution – workflow_id={workflow.id}")
        logger.info(f"[CanonicalEngine] Config: {config}")

        # Stream execution for real-time updates
        node_count = 0
        chunk_count = 0

        async for chunk in graph.astream(initial_state, config):
            chunk_count += 1
            logger.info(f"[CanonicalEngine] Received chunk #{chunk_count}: {chunk}")

            if chunk:
                for node_id, state_update in chunk.items():
                    if state_update and hasattr(state_update, "get"):
                        logger.info(f"[CanonicalEngine] Processing state update from node {node_id}: {state_update}")
                        current_completed = len(state_update.get("completed_nodes", []))

                        if current_completed > node_count:
                            newly_completed = current_completed - node_count
                            node_count = current_completed
                            logger.info(
                                f"[CanonicalEngine] Progress: {newly_completed} new nodes "
                                f"completed ({node_count} total)"
                            )

                        # Publish streaming progress
                        await self._publish_streaming_progress(
                            execution_id=execution.id,
                            completed_nodes=state_update.get("completed_nodes", []),
                            node_outputs=state_update.get("node_outputs", {}),
                            error=state_update.get("error"),
                        )

        logger.info(f"[CanonicalEngine] Streaming completed after {chunk_count} chunks")

        # Mark as successful
        execution.status = "success"
        execution.finished_at = datetime.now(timezone.utc)
        db.commit()

        await self._publish_execution_finished(
            execution_id=execution.id, status="success", error=None, duration_ms=self._duration_ms(execution)
        )

        logger.info(f"[CanonicalEngine] Execution completed – execution_id={execution.id}")

    def _build_canonical_langgraph(self, workflow: CanonicalWorkflow, execution_id: int, checkpointer) -> StateGraph:
        """Build LangGraph from canonical workflow with direct field access."""

        graph = StateGraph(WorkflowState)

        logger.info(f"[CanonicalEngine] Building graph for {len(workflow.nodes)} nodes")

        # Add nodes with direct field access - no parsing needed!
        for node in workflow.nodes:
            node_id = node.id.value  # Direct access
            logger.info(f"[CanonicalEngine] Processing node: {node_id} (type={node.node_type.value})")

            # Create node function based on type - direct type checking, no isinstance
            if node.is_agent:
                logger.info(
                    f"[CanonicalEngine] Creating agent node for {node_id} (agent_id={node.agent_id.value})"
                )  # Direct access!
                node_func = self._create_canonical_agent_node(node)
            elif node.is_tool:
                logger.info(
                    f"[CanonicalEngine] Creating tool node for {node_id} (tool={node.tool_name})"
                )  # Direct access!
                node_func = self._create_canonical_tool_node(node)
            elif node.is_trigger:
                logger.info(f"[CanonicalEngine] Creating trigger node for {node_id}")
                node_func = self._create_canonical_trigger_node(node)
            else:
                # This should be impossible with canonical types, but handle gracefully
                logger.warning(f"[CanonicalEngine] Unknown node type for {node_id}: {node.node_type}")
                node_func = self._create_placeholder_node(node)

            graph.add_node(node_id, node_func)

        # Add edges with direct field access
        start_nodes = []
        end_nodes = []

        # Get all node IDs for validation
        node_ids = {node.id.value for node in workflow.nodes}
        logger.info(f"[CanonicalEngine] Valid node IDs: {node_ids}")

        # Find nodes with no incoming/outgoing edges
        target_nodes = {edge.to_node_id.value for edge in workflow.edges}  # Direct access
        source_nodes = {edge.from_node_id.value for edge in workflow.edges}  # Direct access

        for node in workflow.nodes:
            node_id = node.id.value  # Direct access
            if node_id not in target_nodes:
                start_nodes.append(node_id)
            if node_id not in source_nodes:
                end_nodes.append(node_id)

        logger.info(f"[CanonicalEngine] Start nodes: {start_nodes}")
        logger.info(f"[CanonicalEngine] End nodes: {end_nodes}")

        # Connect START to all start nodes
        for start_node in start_nodes:
            graph.add_edge(START, start_node)

        # Add workflow edges with direct field access
        for edge in workflow.edges:
            source = edge.from_node_id.value  # Direct access
            target = edge.to_node_id.value  # Direct access

            if source not in node_ids:
                logger.warning(f"[CanonicalEngine] Skipping edge - source not found: {source}")
                continue
            if target not in node_ids:
                logger.warning(f"[CanonicalEngine] Skipping edge - target not found: {target}")
                continue

            logger.info(f"[CanonicalEngine] Adding edge: {source} -> {target}")
            graph.add_edge(source, target)

        # Connect all end nodes to END
        for end_node in end_nodes:
            graph.add_edge(end_node, END)

        logger.info("[CanonicalEngine] Graph construction complete. Compiling...")
        return graph.compile(checkpointer=checkpointer)

    def _create_canonical_agent_node(self, node: CanonicalNode):
        """Create agent node function using canonical types with direct field access."""

        async def agent_node(state: WorkflowState) -> WorkflowState:
            node_id = node.id.value  # Direct access
            agent_id = node.agent_id.value  # Direct access - no parsing!
            message = node.agent_data.message  # Direct access - no parsing!

            logger.info(f"[AgentNode] Starting execution – node_id={node_id}, agent_id={agent_id}")

            session_factory = get_session_factory()
            with session_factory() as db:
                # Create node execution state
                node_state = NodeExecutionState(
                    workflow_execution_id=state["execution_id"], node_id=node_id, status="running"
                )
                db.add(node_state)
                db.commit()

                await self._publish_node_event(
                    execution_id=state["execution_id"], node_id=node_id, status="running", output=None, error=None
                )

                try:
                    # Get agent - direct field access, no validation needed
                    agent = db.query(Agent).filter_by(id=agent_id).first()
                    if not agent:
                        raise ValueError(f"Agent {agent_id} not found in database")

                    logger.info(f"[AgentNode] Found agent: {agent.name} (id={agent.id})")

                    # Create thread and execute
                    thread = crud.create_thread(
                        db=db,
                        agent_id=agent_id,
                        title=f"Workflow execution {state['execution_id']}",
                    )

                    crud.create_thread_message(
                        db=db,
                        thread_id=thread.id,
                        role="user",
                        content=message,  # Direct field access
                        processed=False,
                    )

                    # Determine tool configuration
                    connected_tools = self._get_connected_tool_names(node_id)

                    if connected_tools:
                        logger.info(f"[AgentNode] Agent connects to tools: {connected_tools}")

                        # Create agent proxy with connected tools
                        current_tools = agent.allowed_tools if agent.allowed_tools else []
                        if isinstance(current_tools, dict):
                            current_tools = []
                        combined_tools = list(set(current_tools + connected_tools))

                        # Simple proxy for tool configuration
                        class AgentProxy:
                            def __init__(self, original_agent, tool_list):
                                for attr in dir(original_agent):
                                    if not attr.startswith("_") and hasattr(original_agent, attr):
                                        try:
                                            setattr(self, attr, getattr(original_agent, attr))
                                        except AttributeError:
                                            pass
                                self.allowed_tools = tool_list

                        agent_for_runner = AgentProxy(agent, combined_tools)
                    else:
                        agent_for_runner = agent

                    # Execute agent
                    runner = AgentRunner(agent_for_runner)
                    created_messages = await runner.run_thread(db, thread)

                    logger.info(f"[AgentNode] Created {len(created_messages)} messages")

                    # Process results
                    assistant_messages = [msg for msg in created_messages if msg.role == "assistant"]

                    if connected_tools and assistant_messages:
                        # Check for tool calls
                        last_message = assistant_messages[-1]
                        if hasattr(last_message, "tool_calls") and last_message.tool_calls:
                            # Output structured tool calls
                            output = {
                                "agent_id": agent_id,
                                "agent_name": agent.name,
                                "message": message,
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
                            # No tool calls made
                            output = {
                                "agent_id": agent_id,
                                "agent_name": agent.name,
                                "message": message,
                                "response": last_message.content if hasattr(last_message, "content") else "No response",
                                "type": "agent",
                            }
                    else:
                        # Normal agent output
                        result = assistant_messages[-1].content if assistant_messages else "No response generated"
                        output = {
                            "agent_id": agent_id,
                            "agent_name": agent.name,
                            "message": message,
                            "response": result,
                            "type": "agent",
                        }

                    # Update node state
                    node_state.status = "success"
                    node_state.output = output
                    db.commit()

                    await self._publish_node_event(
                        execution_id=state["execution_id"], node_id=node_id, status="success", output=output, error=None
                    )

                    return {"node_outputs": {node_id: output}, "completed_nodes": [node_id]}

                except Exception as e:
                    error_msg = str(e)
                    logger.error(f"[AgentNode] Failed: {error_msg}")

                    node_state.status = "failed"
                    node_state.error = error_msg
                    db.commit()

                    await self._publish_node_event(
                        execution_id=state["execution_id"],
                        node_id=node_id,
                        status="failed",
                        output=None,
                        error=error_msg,
                    )

                    raise

        return agent_node

    def _create_canonical_tool_node(self, node: CanonicalNode):
        """Create tool node function using canonical types with direct field access."""

        async def tool_node(state: WorkflowState) -> WorkflowState:
            node_id = node.id.value  # Direct access
            tool_name = node.tool_name  # Direct access - no parsing!
            tool_params = node.tool_data.parameters.copy()  # Direct access - no parsing!

            logger.info(f"[ToolNode] Starting execution – node_id={node_id}, tool={tool_name}")

            # Check for tool calls from connected agents
            connected_agent_ids = self._get_connected_agent_ids(node)

            if connected_agent_ids:
                for agent_id in connected_agent_ids:
                    agent_output = state["node_outputs"].get(agent_id, {})
                    if agent_output.get("type") == "agent_with_tools" and "tool_calls" in agent_output:
                        # Find matching tool call
                        for tool_call in agent_output["tool_calls"]:
                            if tool_call.get("tool") == tool_name:
                                tool_params = tool_call.get("parameters", {})
                                break

            session_factory = get_session_factory()
            with session_factory() as db:
                # Create node execution state
                node_state = NodeExecutionState(
                    workflow_execution_id=state["execution_id"], node_id=node_id, status="running"
                )
                db.add(node_state)
                db.commit()

                await self._publish_node_event(
                    execution_id=state["execution_id"], node_id=node_id, status="running", output=None, error=None
                )

                try:
                    # Execute tool using unified resolver
                    resolver = get_tool_resolver()
                    tool = resolver.get_tool(tool_name)

                    if not tool:
                        available_tools = resolver.get_tool_names()
                        raise ValueError(f"Tool '{tool_name}' not found. Available tools: {available_tools}")

                    result = await tool.ainvoke(tool_params)

                    output = {"tool_name": tool_name, "parameters": tool_params, "result": result, "type": "tool"}

                    # Update node state
                    node_state.status = "success"
                    node_state.output = output
                    db.commit()

                    await self._publish_node_event(
                        execution_id=state["execution_id"], node_id=node_id, status="success", output=output, error=None
                    )

                    return {"node_outputs": {node_id: output}, "completed_nodes": [node_id]}

                except Exception as e:
                    error_msg = str(e)
                    logger.error(f"[ToolNode] Failed: {error_msg}")

                    node_state.status = "failed"
                    node_state.error = error_msg
                    db.commit()

                    await self._publish_node_event(
                        execution_id=state["execution_id"],
                        node_id=node_id,
                        status="failed",
                        output=None,
                        error=error_msg,
                    )

                    raise

        return tool_node

    def _create_canonical_trigger_node(self, node: CanonicalNode):
        """Create trigger node function using canonical types with direct field access."""

        async def trigger_node(state: WorkflowState) -> WorkflowState:
            node_id = node.id.value  # Direct access
            trigger_type = node.trigger_data.trigger_type  # Direct access - no parsing!

            logger.info(f"[TriggerNode] Execution – node_id={node_id}, type={trigger_type}")

            session_factory = get_session_factory()
            with session_factory() as db:
                # Create node execution state
                node_state = NodeExecutionState(
                    workflow_execution_id=state["execution_id"], node_id=node_id, status="running"
                )
                db.add(node_state)
                db.commit()

                await self._publish_node_event(
                    execution_id=state["execution_id"], node_id=node_id, status="running", output=None, error=None
                )

                try:
                    # Simple trigger execution
                    output = {"trigger_type": trigger_type, "status": "triggered", "type": "trigger"}

                    # Update node state
                    node_state.status = "success"
                    node_state.output = output
                    db.commit()

                    await self._publish_node_event(
                        execution_id=state["execution_id"], node_id=node_id, status="success", output=output, error=None
                    )

                    return {"node_outputs": {node_id: output}, "completed_nodes": [node_id]}

                except Exception as e:
                    error_msg = str(e)
                    logger.error(f"[TriggerNode] Failed: {error_msg}")

                    node_state.status = "failed"
                    node_state.error = error_msg
                    db.commit()

                    await self._publish_node_event(
                        execution_id=state["execution_id"],
                        node_id=node_id,
                        status="failed",
                        output=None,
                        error=error_msg,
                    )

                    raise

        return trigger_node

    def _create_placeholder_node(self, node: CanonicalNode):
        """Create placeholder for unknown node types."""

        async def placeholder_node(state: WorkflowState) -> WorkflowState:
            node_id = node.id.value  # Direct access

            logger.warning(f"[PlaceholderNode] Unknown node type: {node.node_type} for {node_id}")

            output = {"result": f"placeholder_executed_{node.node_type.value}", "type": "placeholder"}

            return {"node_outputs": {node_id: output}, "completed_nodes": [node_id]}

        return placeholder_node

    def _get_connected_tool_names(self, node_id: str) -> List[str]:
        """Get names of tools connected to this node."""
        tool_names = []

        if not hasattr(self, "_current_edges") or not self._current_edges:
            return tool_names

        # Find outgoing edges from this node
        for edge in self._current_edges:
            if hasattr(edge, "from_node_id") and str(edge.from_node_id) == str(node_id):
                # Check if target node is a tool
                target_id = str(edge.to_node_id)
                if hasattr(self, "_current_nodes") and self._current_nodes:
                    for node in self._current_nodes:
                        if self._get_node_id(node) == target_id:
                            # Check if this is a tool node and extract tool name
                            tool_name = None
                            if hasattr(node, "node_type"):
                                # Check if node_type is a dict with Tool key
                                if isinstance(node.node_type, dict) and "Tool" in node.node_type:
                                    tool_config = node.node_type["Tool"]
                                    if isinstance(tool_config, dict):
                                        tool_name = tool_config.get("tool_name")
                                # Also check config for tool_name
                                elif hasattr(node, "config") and isinstance(node.config, dict):
                                    tool_name = node.config.get("tool_name")
                            elif hasattr(node, "type") and isinstance(node.type, dict):
                                if "Tool" in node.type:
                                    tool_config = node.type["Tool"]
                                    if isinstance(tool_config, dict):
                                        tool_name = tool_config.get("tool_name")

                            if tool_name:
                                tool_names.append(tool_name)

        return tool_names

    def _get_connected_agent_ids(self, node: CanonicalNode) -> List[str]:
        """Get IDs of agents connected to this node using canonical workflow."""
        # This would need access to the workflow context
        # For now, return empty list as this requires workflow-level context
        return []

    def _has_outgoing_tool_connections(self, node_id: str) -> bool:
        """Check if node has outgoing connections to tool nodes."""
        if not hasattr(self, "_current_edges") or not self._current_edges:
            return False

        # Find outgoing edges from this node
        for edge in self._current_edges:
            if hasattr(edge, "from_node_id") and str(edge.from_node_id) == str(node_id):
                # Check if target node is a tool
                target_id = str(edge.to_node_id)
                if hasattr(self, "_current_nodes") and self._current_nodes:
                    for node in self._current_nodes:
                        if self._get_node_id(node) == target_id:
                            # Check if this is a tool node
                            if hasattr(node, "node_type"):
                                # Check if node_type is a dict with Tool key
                                if isinstance(node.node_type, dict) and "Tool" in node.node_type:
                                    return True
                                # Also check string node_type
                                node_type = str(node.node_type).lower()
                                if "tool" in node_type:
                                    return True
                            elif hasattr(node, "type") and isinstance(node.type, dict):
                                if "Tool" in node.type:
                                    return True
        return False

    def _get_node_id(self, node: Any) -> str:
        """Extract node ID from various node formats (canonical, WorkflowNode, or dict)."""
        if hasattr(node, "id"):
            # Canonical node
            if hasattr(node.id, "value"):
                return node.id.value
            else:
                return str(node.id)
        elif hasattr(node, "node_id"):
            # WorkflowNode from schema
            return str(node.node_id)
        elif isinstance(node, dict) and "id" in node:
            # Dict-based node
            return str(node["id"])
        else:
            raise ValueError(
                f"Cannot extract node ID from {type(node)}. "
                f"Available attributes: {dir(node) if hasattr(node, '__dict__') else 'N/A'}"
            )

    # Event publishing methods (unchanged from original)
    async def _publish_node_event(self, *, execution_id: int, node_id: str, status: str, output: Any, error: str):
        """Publish node state change event."""
        payload = {
            "execution_id": execution_id,
            "node_id": node_id,
            "status": status,
            "output": output,
            "error": error,
            "event_type": EventType.NODE_STATE_CHANGED,
        }

        await publish_event(EventType.NODE_STATE_CHANGED, payload)

    async def _publish_streaming_progress(
        self, *, execution_id: int, completed_nodes: List[str], node_outputs: Dict[str, Any], error: str
    ):
        """Publish streaming progress update."""
        payload = {
            "execution_id": execution_id,
            "completed_nodes": completed_nodes,
            "node_outputs": node_outputs,
            "error": error,
            "event_type": EventType.WORKFLOW_PROGRESS,
        }

        await publish_event(EventType.WORKFLOW_PROGRESS, payload)

    async def _publish_execution_finished(self, *, execution_id: int, status: str, error: str, duration_ms: int):
        """Publish execution finished event."""
        payload = {
            "execution_id": execution_id,
            "status": status,
            "error": error,
            "duration_ms": duration_ms,
            "event_type": EventType.EXECUTION_FINISHED,
        }

        await publish_event(EventType.EXECUTION_FINISHED, payload)

    @staticmethod
    def _duration_ms(execution: WorkflowExecution) -> Union[int, None]:
        if execution.started_at and execution.finished_at:
            delta = execution.finished_at - execution.started_at
            return int(delta.total_seconds() * 1000)
        return None


# Singleton instance
workflow_engine = WorkflowEngine()
