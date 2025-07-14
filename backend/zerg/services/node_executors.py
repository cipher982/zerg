"""
Node execution handlers for workflow engine.

Each node type has a focused, testable executor class.
Supports variable passing between nodes via ${node_id.output_key} syntax.
"""

import logging
import re
from typing import Any
from typing import Dict

from zerg.crud import crud
from zerg.database import get_session_factory
from zerg.managers.agent_runner import AgentRunner
from zerg.models.models import Agent
from zerg.models.models import NodeExecutionState
from zerg.tools.unified_access import get_tool_resolver

logger = logging.getLogger(__name__)


def resolve_variables(data: Any, node_outputs: Dict[str, Any]) -> Any:
    """
    Resolve variable references in node configuration.

    Variables use syntax: ${node_id.output_key} or ${node_id}
    Examples:
      - ${agent-1.result} -> output["result"] from node "agent-1"
      - ${tool-1} -> entire output from node "tool-1"
      - "Process ${agent-1.messages_created} messages" -> string interpolation
    """
    if isinstance(data, str):
        # Find all variable references like ${node_id.key} or ${node_id}
        pattern = r"\${([^}]+)}"

        def replace_var(match):
            var_path = match.group(1)

            if "." in var_path:
                # ${node_id.output_key}
                node_id, output_key = var_path.split(".", 1)
                if node_id in node_outputs:
                    node_output = node_outputs[node_id]
                    if isinstance(node_output, dict) and output_key in node_output:
                        return str(node_output[output_key])
                    else:
                        logger.warning(f"Variable {var_path}: key '{output_key}' not found in node output")
                        return match.group(0)  # Return original if not found
                else:
                    logger.warning(f"Variable {var_path}: node '{node_id}' output not found")
                    return match.group(0)
            else:
                # ${node_id} - entire output
                node_id = var_path
                if node_id in node_outputs:
                    return str(node_outputs[node_id])
                else:
                    logger.warning(f"Variable {var_path}: node '{node_id}' output not found")
                    return match.group(0)

        return re.sub(pattern, replace_var, data)

    elif isinstance(data, dict):
        # Recursively resolve variables in dictionary values
        return {key: resolve_variables(value, node_outputs) for key, value in data.items()}

    elif isinstance(data, list):
        # Recursively resolve variables in list items
        return [resolve_variables(item, node_outputs) for item in data]

    else:
        # Return non-string values as-is
        return data


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
                node_state.output = output
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
                node_state.output = {"status": "failed", "error": error_msg}
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
        # Resolve variables in node configuration
        resolved_config = resolve_variables(self.node.config, state.get("node_outputs", {}))

        agent_id = resolved_config.get("agent_id")
        message = resolved_config.get("message", "Execute this task")

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
        runner = AgentRunner(agent)
        created_messages = await runner.run_thread(db, thread)

        # Return structured workflow output
        return {
            "agent_id": agent_id,
            "thread_id": thread.id,
            "messages_created": len(created_messages),
            "status": "completed",
        }


class ToolNodeExecutor(BaseNodeExecutor):
    """Executes tool nodes."""

    async def _execute_node_logic(self, db, state):
        # Resolve variables in node configuration
        resolved_config = resolve_variables(self.node.config, state.get("node_outputs", {}))

        tool_name = resolved_config.get("tool_name")

        if not tool_name:
            raise ValueError(f"Tool node {self.node_id} missing tool_name in config")

        logger.info(f"[ToolNode] Starting execution – node_id={self.node_id}, tool_name={tool_name}")

        # Get tool resolver and execute
        tool_resolver = get_tool_resolver()
        tool = tool_resolver.get_tool(tool_name)

        if not tool:
            raise ValueError(f"Tool {tool_name} not found")

        static_params = resolved_config.get("static_params", {})
        output = tool.run(static_params)

        # Return structured workflow output
        return {"tool_name": tool_name, "parameters": static_params, "result": output, "status": "completed"}


class TriggerNodeExecutor(BaseNodeExecutor):
    """Executes trigger nodes."""

    async def _execute_node_logic(self, db, state):
        logger.info(f"[TriggerNode] Executing trigger node: {self.node_id}")

        # Trigger nodes just pass through for now
        return {"status": "triggered", "config": self.node.config}


class ConditionalNodeExecutor(BaseNodeExecutor):
    """Executes conditional nodes with if/else logic."""

    async def _execute_node_logic(self, db, state):
        logger.info(f"[ConditionalNode] Executing conditional node: {self.node_id}")

        # Resolve variables in node configuration
        resolved_config = resolve_variables(self.node.config, state.get("node_outputs", {}))

        condition = resolved_config.get("condition", "")
        condition_type = resolved_config.get("condition_type", "expression")

        if not condition:
            raise ValueError(f"Conditional node {self.node_id} missing condition")

        # Evaluate the condition
        condition_result = self._evaluate_condition(condition, condition_type, state.get("node_outputs", {}))

        logger.info(f"[ConditionalNode] Condition '{condition}' evaluated to {condition_result}")

        return {
            "condition": condition,
            "condition_result": condition_result,
            "status": "completed",
            "branch": "true" if condition_result else "false",
        }

    def _evaluate_condition(self, condition: str, condition_type: str, node_outputs: Dict[str, Any]) -> bool:
        """Evaluate a condition and return True/False."""

        if condition_type == "expression":
            # Simple expression evaluation
            # Supports: ==, !=, >, <, >=, <=
            # Examples: "${tool-1.result} > 50", "${agent-1.status} == 'completed'"

            # Split on operators (order matters - check >= before >)
            operators = [">=", "<=", "==", "!=", ">", "<"]

            for op in operators:
                if op in condition:
                    left, right = condition.split(op, 1)
                    left = left.strip()
                    right = right.strip()

                    # Convert to appropriate types for comparison
                    left_val = self._convert_value(left)
                    right_val = self._convert_value(right.strip("'\""))  # Remove quotes

                    # Perform comparison
                    if op == "==":
                        return left_val == right_val
                    elif op == "!=":
                        return left_val != right_val
                    elif op == ">":
                        return float(left_val) > float(right_val)
                    elif op == "<":
                        return float(left_val) < float(right_val)
                    elif op == ">=":
                        return float(left_val) >= float(right_val)
                    elif op == "<=":
                        return float(left_val) <= float(right_val)

            # If no operator found, treat as truthy check
            return bool(self._convert_value(condition))

        elif condition_type == "exists":
            # Check if a node output exists
            # Example: "tool-1.result" checks if tool-1 has a result key
            if "." in condition:
                node_id, key = condition.split(".", 1)
                return (
                    node_id in node_outputs and isinstance(node_outputs[node_id], dict) and key in node_outputs[node_id]
                )
            else:
                return condition in node_outputs

        else:
            raise ValueError(f"Unsupported condition type: {condition_type}")

    def _convert_value(self, value: str):
        """Convert string value to appropriate type (int, float, or string)."""
        value = str(value).strip()

        # Try to convert to number
        try:
            if "." in value:
                return float(value)
            else:
                return int(value)
        except ValueError:
            # Return as string if not a number
            return value


def create_node_executor(node, publish_event_callback) -> BaseNodeExecutor:
    """Factory function to create appropriate node executor."""
    if node.type == "agent":
        return AgentNodeExecutor(node, publish_event_callback)
    elif node.type == "tool":
        return ToolNodeExecutor(node, publish_event_callback)
    elif node.type == "trigger":
        return TriggerNodeExecutor(node, publish_event_callback)
    elif node.type == "conditional":
        return ConditionalNodeExecutor(node, publish_event_callback)
    else:
        # Placeholder for unknown types
        class PlaceholderExecutor(BaseNodeExecutor):
            async def _execute_node_logic(self, db, state):
                logger.warning(f"[PlaceholderNode] Executing placeholder for unknown node type: {self.node_id}")
                return {"status": "skipped", "reason": "unknown_type"}

        return PlaceholderExecutor(node, publish_event_callback)
