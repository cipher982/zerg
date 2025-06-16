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
from typing import TypedDict
from typing import Union

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
    db_session_factory: Any  # Pass session factory through state


class LangGraphWorkflowEngine:
    """LangGraph-based workflow execution engine."""

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
                # Build LangGraph from canvas_data
                graph = self._build_langgraph(workflow.canvas_data, execution.id)

                # Execute the graph
                initial_state = WorkflowState(
                    execution_id=execution.id,
                    node_outputs={},
                    completed_nodes=[],
                    error=None,
                    db_session_factory=session_factory,
                )

                final_state = await graph.ainvoke(initial_state)

                # Log completion summary
                completed_count = len(final_state.get("completed_nodes", []))
                logger.info(f"[LangGraphEngine] Workflow completed – {completed_count} nodes executed")

                # Mark as successful
                execution.status = "success"
                execution.finished_at = datetime.now(timezone.utc)
                db.commit()

                self._publish_execution_finished(
                    execution_id=execution.id, status="success", error=None, duration_ms=self._duration_ms(execution)
                )

                logger.info("[LangGraphEngine] Execution completed – execution_id=%s", execution.id)
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

    def _build_langgraph(self, canvas_data: Dict[str, Any], execution_id: int) -> StateGraph:
        """Convert canvas_data to LangGraph StateGraph."""

        workflow = StateGraph(WorkflowState)

        nodes = canvas_data.get("nodes", [])
        edges = canvas_data.get("edges", [])

        # Add all nodes to the graph
        for node in nodes:
            node_id = str(node.get("id", "unknown"))
            node_type = str(node.get("type", "unknown")).lower()

            # Create node execution function based on type
            if node_type == "tool":
                node_func = self._create_tool_node(node)
            elif node_type == "agent":
                node_func = self._create_agent_node(node)
            elif node_type == "trigger":
                node_func = self._create_trigger_node(node)
            else:
                node_func = self._create_placeholder_node(node)

            workflow.add_node(node_id, node_func)

        # Add edges between nodes
        start_nodes = []
        end_nodes = []

        # Find nodes with no incoming edges (start nodes)
        target_nodes = {str(edge.get("target", "")) for edge in edges}
        source_nodes = {str(edge.get("source", "")) for edge in edges}

        for node in nodes:
            node_id = str(node.get("id", "unknown"))
            if node_id not in target_nodes:
                start_nodes.append(node_id)
            if node_id not in source_nodes:
                end_nodes.append(node_id)

        # Connect START to all start nodes
        for start_node in start_nodes:
            workflow.add_edge(START, start_node)

        # Add internal edges
        for edge in edges:
            source = str(edge.get("source", ""))
            target = str(edge.get("target", ""))
            if source and target:
                workflow.add_edge(source, target)

        # Connect all end nodes to END
        for end_node in end_nodes:
            workflow.add_edge(end_node, END)

        return workflow.compile()

    def _create_tool_node(self, node_config: Dict[str, Any]):
        """Create a tool execution node function."""

        async def tool_node(state: WorkflowState) -> WorkflowState:
            node_id = str(node_config.get("id", "unknown"))
            tool_name = node_config.get("tool_name") or node_config.get("name", "")
            tool_params = node_config.get("parameters", {})

            # Node execution - just store output at end, don't update intermediate state

            # Create node execution state
            with state["db_session_factory"]() as db:
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

    def _create_agent_node(self, node_config: Dict[str, Any]):
        """Create an agent execution node function."""

        async def agent_node(state: WorkflowState) -> WorkflowState:
            node_id = str(node_config.get("id", "unknown"))
            agent_id = node_config.get("agent_id")

            # Node execution - just store output at end, don't update intermediate state

            with state["db_session_factory"]() as db:
                node_state = NodeExecutionState(
                    workflow_execution_id=state["execution_id"], node_id=node_id, status="running"
                )
                db.add(node_state)
                db.commit()

                self._publish_node_event(
                    execution_id=state["execution_id"], node_id=node_id, status="running", output=None, error=None
                )

                try:
                    # Get agent
                    agent = db.query(Agent).filter_by(id=agent_id).first()
                    if not agent:
                        raise ValueError(f"Agent {agent_id} not found")

                    # Create thread and execute
                    thread = crud.create_thread(
                        db=db,
                        user_id=1,  # TODO: Get from context
                        thread_type="workflow",
                        title=f"Workflow execution {state['execution_id']}",
                    )

                    user_message = node_config.get("message", "Execute this task")
                    crud.create_message(db=db, thread_id=thread.id, role="user", content=user_message, processed=False)

                    runner = AgentRunner(agent)
                    created_messages = await runner.run_thread(db, thread)

                    assistant_messages = [msg for msg in created_messages if msg.role == "assistant"]
                    result = assistant_messages[-1].content if assistant_messages else "No response generated"

                    output = {
                        "agent_id": agent_id,
                        "agent_name": agent.name,
                        "message": user_message,
                        "response": result,
                        "type": "agent",
                    }

                    node_state.status = "success"
                    node_state.output = output
                    db.commit()

                    self._publish_node_event(
                        execution_id=state["execution_id"], node_id=node_id, status="success", output=output, error=None
                    )

                    # Return only the changes to state
                    return {"node_outputs": {node_id: output}, "completed_nodes": [node_id]}

                except Exception as e:
                    node_state.status = "failed"
                    node_state.error = str(e)
                    db.commit()

                    self._publish_node_event(
                        execution_id=state["execution_id"], node_id=node_id, status="failed", output=None, error=str(e)
                    )

                    raise  # Re-raise without modifying state

            # Should not reach here
            return {}

        return agent_node

    def _create_trigger_node(self, node_config: Dict[str, Any]):
        """Create a trigger execution node function."""

        async def trigger_node(state: WorkflowState) -> WorkflowState:
            node_id = str(node_config.get("id", "unknown"))
            trigger_type = node_config.get("trigger_type", "webhook")

            # Node execution - just store output at end, don't update intermediate state

            # For now, just simulate trigger creation
            output = {"trigger_type": trigger_type, "status": "created", "type": "trigger"}
            state["node_outputs"][node_id] = output

            return state

        return trigger_node

    def _create_placeholder_node(self, node_config: Dict[str, Any]):
        """Create a placeholder node for unknown types."""

        async def placeholder_node(state: WorkflowState) -> WorkflowState:
            node_id = str(node_config.get("id", "unknown"))
            node_type = str(node_config.get("type", "unknown"))

            # Node execution - just store output at end, don't update intermediate state

            # Simulate execution
            await asyncio.sleep(0.1)

            output = {"result": f"{node_type}_executed", "type": "placeholder"}

            # Return only the changes to state
            return {"node_outputs": {node_id: output}, "completed_nodes": [node_id]}

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

    def _publish_execution_finished(
        self, *, execution_id: int, status: str, error: Union[str, None], duration_ms: Union[int, None]
    ):
        """Publish execution finished event."""

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
