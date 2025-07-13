"""
Node execution handlers for workflow engine.

Each node type has a focused, testable executor class.
"""

import logging
from typing import Any
from typing import Dict

from zerg.crud import crud
from zerg.database import get_session_factory
from zerg.managers.agent_runner import AgentRunner
from zerg.models.models import Agent
from zerg.models.models import NodeExecutionState
from zerg.tools.unified_access import get_tool_resolver

logger = logging.getLogger(__name__)


class BaseNodeExecutor:
    """Base class for node executors with common functionality."""

    def __init__(self, node, publish_event_callback):
        self.node = node
        self.node_id = node.id
        self.publish_event = publish_event_callback

    async def execute(self, state: Dict[str, Any]) -> Dict[str, Any]:
        """Execute the node and return updated state."""
        session_factory = get_session_factory()

        with session_factory() as db:
            # Create execution state
            node_state = NodeExecutionState(
                workflow_execution_id=state["execution_id"], node_id=self.node_id, status="running"
            )
            db.add(node_state)
            db.commit()

            await self.publish_event(
                execution_id=state["execution_id"], node_id=self.node_id, status="running", output=None, error=None
            )

            try:
                output = await self._execute_node_logic(db, state)

                node_state.status = "completed"
                node_state.output = output if isinstance(output, dict) else {"result": output}
                db.commit()

                await self.publish_event(
                    execution_id=state["execution_id"],
                    node_id=self.node_id,
                    status="completed",
                    output=str(output),
                    error=None,
                )

                return {
                    **state,
                    "node_outputs": {**state["node_outputs"], self.node_id: output},
                    "completed_nodes": state["completed_nodes"] + [self.node_id],
                }

            except Exception as e:
                error_msg = str(e)
                logger.error(f"[{self.__class__.__name__}] Error in node {self.node_id}: {error_msg}")

                node_state.status = "failed"
                node_state.error = error_msg
                node_state.output = {"error": error_msg}
                db.commit()

                await self.publish_event(
                    execution_id=state["execution_id"],
                    node_id=self.node_id,
                    status="failed",
                    output=None,
                    error=error_msg,
                )

                return {
                    **state,
                    "error": f"{self.node.type} node {self.node_id} failed: {error_msg}",
                    "completed_nodes": state["completed_nodes"] + [self.node_id],
                }

    async def _execute_node_logic(self, db, state):
        """Override this in subclasses."""
        raise NotImplementedError


class AgentNodeExecutor(BaseNodeExecutor):
    """Executes agent nodes."""

    async def _execute_node_logic(self, db, state):
        agent_id = self.node.config.get("agent_id")
        message = self.node.config.get("message", "Execute this task")

        if not agent_id:
            raise ValueError(f"Agent node {self.node_id} missing agent_id in config")

        logger.info(f"[AgentNode] Starting execution – node_id={self.node_id}, agent_id={agent_id}")

        # Get agent
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
            content=message,
            processed=False,
        )

        # Execute via AgentRunner
        runner = AgentRunner()
        output = await runner.process_thread_async(thread.id, agent.allowed_tools)

        return output


class ToolNodeExecutor(BaseNodeExecutor):
    """Executes tool nodes."""

    async def _execute_node_logic(self, db, state):
        tool_name = self.node.config.get("tool_name")

        if not tool_name:
            raise ValueError(f"Tool node {self.node_id} missing tool_name in config")

        logger.info(f"[ToolNode] Starting execution – node_id={self.node_id}, tool_name={tool_name}")

        # Get tool resolver and execute
        tool_resolver = get_tool_resolver()
        tool_class = tool_resolver.get_tool_by_name(tool_name)

        if not tool_class:
            raise ValueError(f"Tool {tool_name} not found")

        tool_instance = tool_class()
        static_params = self.node.config.get("static_params", {})
        output = await tool_instance.execute(**static_params)

        return output


class TriggerNodeExecutor(BaseNodeExecutor):
    """Executes trigger nodes."""

    async def _execute_node_logic(self, db, state):
        logger.info(f"[TriggerNode] Executing trigger node: {self.node_id}")

        # Trigger nodes just pass through for now
        return {"status": "triggered", "config": self.node.config}


def create_node_executor(node, publish_event_callback) -> BaseNodeExecutor:
    """Factory function to create appropriate node executor."""
    if node.type == "agent":
        return AgentNodeExecutor(node, publish_event_callback)
    elif node.type == "tool":
        return ToolNodeExecutor(node, publish_event_callback)
    elif node.type == "trigger":
        return TriggerNodeExecutor(node, publish_event_callback)
    else:
        # Placeholder for unknown types
        class PlaceholderExecutor(BaseNodeExecutor):
            async def _execute_node_logic(self, db, state):
                logger.warning(f"[PlaceholderNode] Executing placeholder for unknown node type: {self.node_id}")
                return {"status": "skipped", "reason": "unknown_type"}

        return PlaceholderExecutor(node, publish_event_callback)
