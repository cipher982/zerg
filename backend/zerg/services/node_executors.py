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
from zerg.schemas.node_output import create_agent_envelope
from zerg.schemas.node_output import create_conditional_envelope
from zerg.schemas.node_output import create_tool_envelope
from zerg.schemas.node_output import create_trigger_envelope
from zerg.services.expression_evaluator import ExpressionEvaluationError
from zerg.services.expression_evaluator import ExpressionValidationError
from zerg.services.expression_evaluator import safe_evaluator
from zerg.services.variable_resolver import resolve_variables as enhanced_resolve_variables
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

    def __init__(self, node, publish_event_callback, use_envelope_format=True):
        self.node = node
        self.node_id = node.id
        self.publish_event = publish_event_callback
        self.use_envelope_format = use_envelope_format

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
                # Convert envelope to dict for database storage if needed
                if hasattr(output, "model_dump"):
                    db_output = output.model_dump()
                    # For backward compatibility with tests, add legacy fields at top level for conditional nodes
                    if (
                        self.node.type == "conditional"
                        and isinstance(db_output, dict)
                        and "value" in db_output
                        and isinstance(db_output["value"], dict)
                    ):
                        # Add legacy fields to maintain test compatibility
                        db_output["condition_result"] = db_output["value"].get("result")
                        db_output["branch"] = db_output["value"].get("branch")
                    node_state.output = db_output
                else:
                    node_state.output = output
                db.commit()

                await self.publish_event(
                    execution_id=state["execution_id"],
                    node_id=self.node_id,
                    status="completed",
                    output=str(output),
                    error=None,
                )

                # Store output in state - convert envelope to dict if needed for proper variable resolution
                state_output = output.model_dump() if hasattr(output, "model_dump") else output

                return {
                    **state,
                    "node_outputs": {**state["node_outputs"], self.node_id: state_output},
                    "completed_nodes": state["completed_nodes"] + [self.node_id],
                }

            except Exception as e:
                error_msg = str(e)
                logger.error(f"[{self.__class__.__name__}] Error in node {self.node_id}: {error_msg}")

                node_state.status = "failed"
                node_state.error = error_msg
                error_output = self._create_error_output(error_msg)
                # Convert envelope to dict for database storage if needed
                if hasattr(error_output, "model_dump"):
                    node_state.output = error_output.model_dump()
                else:
                    node_state.output = error_output
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

    def _create_envelope_output(self, value, node_type, **kwargs):
        """Create standardized envelope output based on node type."""
        if not self.use_envelope_format:
            # Return legacy format for backward compatibility
            return value

        # Create envelope based on node type
        if node_type == "tool":
            return create_tool_envelope(value, **kwargs)
        elif node_type == "agent":
            return create_agent_envelope(value, **kwargs)
        elif node_type == "conditional":
            return create_conditional_envelope(value, **kwargs)
        elif node_type == "trigger":
            return create_trigger_envelope(value, **kwargs)
        else:
            # Fallback to tool envelope for unknown types (safest default)
            return create_tool_envelope(value, **kwargs)

    def _create_error_output(self, error_msg):
        """Create standardized error output."""
        if not self.use_envelope_format:
            # Legacy error format
            return {"status": "failed", "error": error_msg}

        # For envelope format, create a proper error envelope based on node type
        # Determine node type from the executor class name
        node_type = "tool"  # Default fallback
        if hasattr(self, "__class__"):
            class_name = self.__class__.__name__
            if "Agent" in class_name:
                node_type = "agent"
            elif "Tool" in class_name:
                node_type = "tool"
            elif "Conditional" in class_name:
                node_type = "conditional"
            elif "Trigger" in class_name:
                node_type = "trigger"

        # Use the specific envelope creator for error cases
        return self._create_envelope_output(
            value=None, node_type=node_type, status="failed", error=error_msg, error_type=type(Exception()).__name__
        )


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

        # Create agent result - either envelope or legacy format
        legacy_output = {
            "agent_id": agent_id,
            "thread_id": thread.id,
            "messages_created": len(created_messages),
            "status": "completed",
        }

        if self.use_envelope_format:
            # Use envelope format: separate agent result from metadata
            return self._create_envelope_output(
                value={
                    "messages": created_messages,
                    "messages_created": len(created_messages),
                },  # The actual agent result
                node_type="agent",
                status="completed",
                agent_id=agent_id,
                agent_name=agent.name,
                thread_id=thread.id,
            )
        else:
            # Return legacy format for backward compatibility
            return legacy_output


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

        # Create tool result - either envelope or legacy format
        legacy_output = {"tool_name": tool_name, "parameters": static_params, "result": output, "status": "completed"}

        if self.use_envelope_format:
            # Use envelope format: separate result value from metadata
            return self._create_envelope_output(
                value=output,  # The actual tool result
                node_type="tool",
                status="completed",
                tool_name=tool_name,
                parameters=static_params,
            )
        else:
            # Return legacy format for backward compatibility
            return legacy_output


class TriggerNodeExecutor(BaseNodeExecutor):
    """Executes trigger nodes."""

    async def _execute_node_logic(self, db, state):
        logger.info(f"[TriggerNode] Executing trigger node: {self.node_id}")

        # Create trigger result - either envelope or legacy format
        legacy_output = {"status": "triggered", "config": self.node.config}

        if self.use_envelope_format:
            # Use envelope format: separate trigger result from metadata
            return self._create_envelope_output(
                value={"triggered": True},  # The actual trigger result
                node_type="trigger",
                status="completed",
                trigger_type="manual",  # Could be enhanced to detect type
                trigger_config=self.node.config,
            )
        else:
            # Return legacy format for backward compatibility
            return legacy_output


class ConditionalNodeExecutor(BaseNodeExecutor):
    """Executes conditional nodes with if/else logic."""

    async def _execute_node_logic(self, db, state):
        logger.info(f"[ConditionalNode] Executing conditional node: {self.node_id}")

        # Use enhanced variable resolution for better type handling
        node_outputs = state.get("node_outputs", {})
        resolved_config = enhanced_resolve_variables(self.node.config, node_outputs)

        condition = resolved_config.get("condition", "")
        condition_type = resolved_config.get("condition_type", "expression")

        if not condition:
            raise ValueError(f"Conditional node {self.node_id} missing condition")

        # Evaluate the condition using the new system
        condition_result = self._evaluate_condition(condition, condition_type, node_outputs)

        logger.info(f"[ConditionalNode] Condition '{condition}' evaluated to {condition_result}")

        # Create conditional result - either envelope or legacy format
        branch = "true" if condition_result else "false"
        legacy_output = {
            "condition": condition,
            "condition_result": condition_result,
            "status": "completed",
            "branch": branch,
        }

        if self.use_envelope_format:
            # Use envelope format: separate conditional result from metadata
            return self._create_envelope_output(
                value={"result": condition_result, "branch": branch},  # The actual conditional result
                node_type="conditional",
                status="completed",
                condition=condition,
                resolved_condition=condition,  # Could be enhanced to show resolved version
                evaluation_method="ast_safe",
            )
        else:
            # Return legacy format for backward compatibility
            return legacy_output

    def _evaluate_condition(self, condition: str, condition_type: str, node_outputs: Dict[str, Any]) -> bool:
        """Evaluate a condition using SafeExpressionEvaluator with proper type handling."""

        if condition_type == "expression":
            try:
                # For expression conditions, we need to resolve variables first to get typed values
                # The condition may contain ${node.field} references that need to be resolved

                # Step 1: Resolve all variables in the condition to get actual values
                resolved_condition = enhanced_resolve_variables(condition, node_outputs)

                # Step 2: If the condition is now a pure variable (single resolved value), handle it
                if not isinstance(resolved_condition, str):
                    # The condition resolved to a single value, treat as truthy check
                    result = bool(resolved_condition)
                    logger.debug(f"Condition '{condition}' resolved to value {resolved_condition}, truthy: {result}")
                    return result

                # Step 3: Extract variables from the resolved condition for SafeExpressionEvaluator
                # We need to build a variables dict for the evaluator
                variables = self._extract_expression_variables(resolved_condition, node_outputs)

                # Step 4: Use SafeExpressionEvaluator for secure, typed evaluation
                result = safe_evaluator.evaluate(resolved_condition, variables)

                # Ensure result is boolean
                result = bool(result)

                logger.debug(f"Expression '{condition}' -> '{resolved_condition}' evaluated to {result}")
                return result

            except (ExpressionEvaluationError, ExpressionValidationError) as e:
                logger.error(f"Failed to evaluate expression condition '{condition}': {e}")
                # For safety, return False on evaluation failure
                return False
            except Exception as e:
                logger.error(f"Unexpected error evaluating condition '{condition}': {e}")
                return False

        elif condition_type == "exists":
            # Check if a node output exists - enhanced with envelope format support
            try:
                # Use enhanced variable resolver to check existence
                resolved_value = enhanced_resolve_variables(f"${{{condition}}}", node_outputs)
                # If resolution succeeded and didn't return the original pattern, the path exists
                return resolved_value != f"${{{condition}}}"
            except Exception:
                # Fallback to legacy existence check
                if "." in condition:
                    node_id, key = condition.split(".", 1)
                    return (
                        node_id in node_outputs
                        and isinstance(node_outputs[node_id], dict)
                        and key in node_outputs[node_id]
                    )
                else:
                    return condition in node_outputs

        else:
            raise ValueError(f"Unsupported condition type: {condition_type}")

    def _extract_expression_variables(self, expression: str, node_outputs: Dict[str, Any]) -> Dict[str, Any]:
        """Extract variables from expression and resolve them to typed values."""
        variables = {}

        # Get variable names from the expression evaluator
        try:
            var_names = safe_evaluator.get_variable_names(expression)

            for var_name in var_names:
                # Try to resolve each variable to a typed value
                try:
                    # Look for the variable in resolved node outputs
                    resolved_value = enhanced_resolve_variables(f"${{{var_name}}}", node_outputs)
                    if resolved_value != f"${{{var_name}}}":
                        # Successfully resolved - use the typed value
                        variables[var_name] = resolved_value
                    else:
                        # Could not resolve - check if it's a literal value in the expression
                        logger.warning(f"Could not resolve variable '{var_name}' in expression '{expression}'")
                except Exception as e:
                    logger.warning(f"Error resolving variable '{var_name}': {e}")

        except Exception as e:
            logger.warning(f"Error extracting variables from expression '{expression}': {e}")

        return variables


def create_node_executor(node, publish_event_callback, use_envelope_format=True) -> BaseNodeExecutor:
    """Factory function to create appropriate node executor."""
    if node.type == "agent":
        return AgentNodeExecutor(node, publish_event_callback, use_envelope_format)
    elif node.type == "tool":
        return ToolNodeExecutor(node, publish_event_callback, use_envelope_format)
    elif node.type == "trigger":
        return TriggerNodeExecutor(node, publish_event_callback, use_envelope_format)
    elif node.type == "conditional":
        return ConditionalNodeExecutor(node, publish_event_callback, use_envelope_format)
    else:
        # Placeholder for unknown types
        class PlaceholderExecutor(BaseNodeExecutor):
            async def _execute_node_logic(self, db, state):
                logger.warning(f"[PlaceholderNode] Executing placeholder for unknown node type: {self.node_id}")
                return {"status": "skipped", "reason": "unknown_type"}

        return PlaceholderExecutor(node, publish_event_callback, use_envelope_format)
