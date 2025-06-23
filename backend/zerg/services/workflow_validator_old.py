"""
Workflow Graph Validation Service.

Combines LangGraph's built-in validation with additional checks
for comprehensive workflow validation.
"""

import logging
from dataclasses import dataclass
from typing import Any
from typing import Dict
from typing import List

from langgraph.checkpoint.memory import MemorySaver

from zerg.schemas.workflow_schema import WorkflowCanvas

logger = logging.getLogger(__name__)


@dataclass
class ValidationError:
    """Represents a validation error."""

    code: str
    message: str
    node_id: str = None
    severity: str = "error"  # error, warning


@dataclass
class ValidationResult:
    """Result of workflow validation."""

    is_valid: bool
    errors: List[ValidationError]
    warnings: List[ValidationError]

    def has_errors(self) -> bool:
        return len(self.errors) > 0

    def has_warnings(self) -> bool:
        return len(self.warnings) > 0


class WorkflowValidator:
    """
    Comprehensive workflow validation combining LangGraph validation
    with custom business logic checks.
    """

    def __init__(self):
        self.max_nodes = 100
        self.max_edges = 500
        self.max_depth = 20

    def validate_workflow(self, canvas: WorkflowCanvas) -> ValidationResult:
        """
        Validate workflow using multi-layer approach:
        1. Custom structural validation
        2. LangGraph compilation validation
        3. Business logic validation
        """
        errors = []
        warnings = []

        try:
            # Layer 1: Custom structural validation
            structural_errors = self._validate_structure(canvas)
            errors.extend(structural_errors)

            # Layer 2: LangGraph compilation validation
            # Only proceed if structure is basically valid
            if not structural_errors:
                try:
                    langgraph_errors = self._validate_with_langgraph(canvas)
                    errors.extend(langgraph_errors)
                except Exception as e:
                    errors.append(
                        ValidationError(
                            code="LANGGRAPH_COMPILATION_FAILED", message=f"LangGraph compilation failed: {str(e)}"
                        )
                    )

            # Layer 3: Business logic validation (warnings mostly)
            business_warnings = self._validate_business_logic(canvas)
            warnings.extend(business_warnings)

        except Exception as e:
            logger.exception("Unexpected error during workflow validation")
            errors.append(
                ValidationError(code="VALIDATION_EXCEPTION", message=f"Validation failed with exception: {str(e)}")
            )

        return ValidationResult(is_valid=len(errors) == 0, errors=errors, warnings=warnings)

    def _validate_structure(self, canvas_data: Dict[str, Any]) -> List[ValidationError]:
        """Validate basic structure and limits."""
        errors = []

        # Check required fields
        if not isinstance(canvas_data, dict):
            errors.append(ValidationError(code="INVALID_CANVAS_DATA", message="Canvas data must be a dictionary"))
            return errors

        nodes = canvas_data.get("nodes", [])
        edges = canvas_data.get("edges", [])

        # Check limits
        if len(nodes) > self.max_nodes:
            errors.append(
                ValidationError(
                    code="TOO_MANY_NODES", message=f"Workflow has {len(nodes)} nodes, maximum is {self.max_nodes}"
                )
            )

        if len(edges) > self.max_edges:
            errors.append(
                ValidationError(
                    code="TOO_MANY_EDGES", message=f"Workflow has {len(edges)} edges, maximum is {self.max_edges}"
                )
            )

        # Validate nodes
        node_errors = self._validate_nodes(nodes)
        errors.extend(node_errors)

        # Validate edges
        edge_errors = self._validate_edges(edges, nodes)
        errors.extend(edge_errors)

        return errors

    def _validate_nodes(self, nodes: List[Dict[str, Any]]) -> List[ValidationError]:
        """Validate individual nodes."""
        errors = []
        node_ids = set()

        for node in nodes:
            # Handle both dict and string formats
            if isinstance(node, dict):
                node_id = node.get("node_id") or node.get("id")
            else:
                # If it's a string, skip validation for now
                continue

            # Check for duplicate node IDs
            if node_id in node_ids:
                errors.append(
                    ValidationError(code="DUPLICATE_NODE_ID", message=f"Duplicate node ID: {node_id}", node_id=node_id)
                )
            node_ids.add(node_id)

            # Validate node type and configuration
            node_type = node.get("node_type")
            if not node_type:
                errors.append(
                    ValidationError(
                        code="MISSING_NODE_TYPE", message=f"Node {node_id} missing node_type", node_id=node_id
                    )
                )
                continue

            # Validate based on node type
            if isinstance(node_type, dict):
                type_key = list(node_type.keys())[0].lower() if node_type else "unknown"

                if type_key == "tool":
                    tool_errors = self._validate_tool_node(node_id, node_type["Tool"])
                    errors.extend(tool_errors)
                elif type_key in ["agent", "agentidentity"]:
                    agent_errors = self._validate_agent_node(node_id, node)
                    errors.extend(agent_errors)

        return errors

    def _validate_tool_node(self, node_id: str, tool_config: Dict[str, Any]) -> List[ValidationError]:
        """Validate tool node configuration."""
        errors = []

        tool_name = tool_config.get("tool_name")
        if not tool_name:
            errors.append(
                ValidationError(
                    code="MISSING_TOOL_NAME", message=f"Tool node {node_id} missing tool_name", node_id=node_id
                )
            )

        # Validate against our tool contracts!
        if tool_name:
            from zerg.tools.generated.tool_definitions import list_all_tools

            valid_tools = list_all_tools()
            if tool_name not in valid_tools:
                errors.append(
                    ValidationError(
                        code="INVALID_TOOL_NAME",
                        message=f"Tool node {node_id} uses invalid tool '{tool_name}'. Valid tools: {valid_tools}",
                        node_id=node_id,
                    )
                )

        return errors

    def _validate_agent_node(self, node_id: str, node: Dict[str, Any]) -> List[ValidationError]:
        """Validate agent node configuration."""
        errors = []

        agent_id = node.get("agent_id")
        if not agent_id:
            errors.append(
                ValidationError(
                    code="MISSING_AGENT_ID", message=f"Agent node {node_id} missing agent_id", node_id=node_id
                )
            )

        return errors

    def _validate_edges(self, edges: List[Dict[str, Any]], nodes: List[Dict[str, Any]]) -> List[ValidationError]:
        """Validate edges between nodes."""
        errors = []

        node_ids = {node.get("node_id") or node.get("id") for node in nodes if isinstance(node, dict)}

        for i, edge in enumerate(edges):
            from_id = edge.get("from_node_id") or edge.get("source")
            to_id = edge.get("to_node_id") or edge.get("target")

            if from_id not in node_ids:
                errors.append(
                    ValidationError(
                        code="INVALID_EDGE_SOURCE", message=f"Edge {i} references non-existent source node: {from_id}"
                    )
                )

            if to_id not in node_ids:
                errors.append(
                    ValidationError(
                        code="INVALID_EDGE_TARGET", message=f"Edge {i} references non-existent target node: {to_id}"
                    )
                )

        return errors

    def _validate_with_langgraph(self, canvas_data: Dict[str, Any]) -> List[ValidationError]:
        """Use LangGraph compilation to catch additional issues."""
        errors = []

        try:
            # Import the workflow engine's method to convert canvas_data to LangGraph
            from zerg.services.langgraph_workflow_engine import LangGraphWorkflowEngine

            # Create a dummy engine instance to use its graph building logic
            engine = LangGraphWorkflowEngine()

            # Try to build and compile the graph
            # Handle different LangGraph API versions gracefully
            try:
                engine._build_langgraph(canvas_data, execution_id=0, checkpointer=MemorySaver())
                # If we get here, LangGraph validation passed
            except TypeError as te:
                if "checkpointer" in str(te):
                    # Try without checkpointer for older/different LangGraph versions
                    try:
                        # Build graph directly without using the engine's method
                        from langgraph.graph import StateGraph

                        from zerg.services.langgraph_workflow_engine import WorkflowState

                        workflow = StateGraph(WorkflowState)
                        nodes = canvas_data.get("nodes", [])

                        # Simple validation - just try to add nodes and edges
                        for node in nodes:
                            if isinstance(node, dict):
                                node_id = str(node.get("node_id") or node.get("id", "unknown"))
                                workflow.add_node(node_id, lambda x: x)
                            else:
                                # Skip non-dict nodes in validation
                                continue

                        edges = canvas_data.get("edges", [])
                        for edge in edges:
                            from_id = str(edge.get("from_node_id") or edge.get("source", ""))
                            to_id = str(edge.get("to_node_id") or edge.get("target", ""))
                            if from_id and to_id:
                                workflow.add_edge(from_id, to_id)

                        # Try basic compilation
                        workflow.compile()

                    except Exception as inner_e:
                        raise inner_e
                else:
                    raise te

        except ValueError as e:
            # LangGraph validation errors
            error_msg = str(e)
            if "entrypoint" in error_msg:
                errors.append(
                    ValidationError(
                        code="NO_ENTRYPOINT", message="Workflow must have at least one trigger node or edge from START"
                    )
                )
            elif "unknown node" in error_msg:
                errors.append(
                    ValidationError(
                        code="UNKNOWN_NODE_REFERENCE", message=f"Edge references non-existent node: {error_msg}"
                    )
                )
            else:
                errors.append(ValidationError(code="LANGGRAPH_VALIDATION_ERROR", message=error_msg))
        except Exception as e:
            errors.append(ValidationError(code="LANGGRAPH_BUILD_ERROR", message=f"Failed to build LangGraph: {str(e)}"))

        return errors

    def _validate_business_logic(self, canvas_data: Dict[str, Any]) -> List[ValidationError]:
        """Validate business logic and best practices (mostly warnings)."""
        warnings = []

        nodes = canvas_data.get("nodes", [])
        edges = canvas_data.get("edges", [])

        # Check for orphaned nodes
        orphaned = self._find_orphaned_nodes(nodes, edges)
        for node_id in orphaned:
            warnings.append(
                ValidationError(
                    code="ORPHANED_NODE",
                    message=f"Node {node_id} is not connected to any other nodes",
                    node_id=node_id,
                    severity="warning",
                )
            )

        # Check for potential cycles
        cycles = self._detect_cycles(nodes, edges)
        if cycles:
            warnings.append(
                ValidationError(
                    code="POTENTIAL_CYCLE", message=f"Workflow may contain cycles: {cycles}", severity="warning"
                )
            )

        # Check for missing trigger nodes
        has_trigger = any(
            isinstance(node, dict)
            and isinstance(node.get("node_type"), dict)
            and "trigger" in str(list(node.get("node_type", {}).keys())[0]).lower()
            for node in nodes
            if isinstance(node, dict)
        )
        if not has_trigger:
            warnings.append(
                ValidationError(
                    code="NO_TRIGGER_NODE",
                    message="Workflow has no trigger nodes - it may never execute",
                    severity="warning",
                )
            )

        return warnings

    def _find_orphaned_nodes(self, nodes: List[Dict], edges: List[Dict]) -> List[str]:
        """Find nodes that are not connected to anything."""
        node_ids = {node.get("node_id") or node.get("id") for node in nodes if isinstance(node, dict)}
        connected_ids = set()

        for edge in edges:
            connected_ids.add(edge.get("from_node_id") or edge.get("source"))
            connected_ids.add(edge.get("to_node_id") or edge.get("target"))

        return list(node_ids - connected_ids)

    def _detect_cycles(self, nodes: List[Dict], edges: List[Dict]) -> List[str]:
        """Simple cycle detection - skip for very large graphs to avoid recursion issues."""
        # Skip cycle detection for very large graphs
        if len(nodes) > 500:
            return []

        # Build adjacency list
        graph = {}
        for node in nodes:
            if isinstance(node, dict):
                node_id = node.get("node_id") or node.get("id")
                if node_id:
                    graph[node_id] = []

        for edge in edges:
            # Handle different edge field formats
            from_id = edge.get("from_node_id") or edge.get("source")
            to_id = edge.get("to_node_id") or edge.get("target")
            if from_id and from_id in graph and to_id:
                graph[from_id].append(to_id)

        # Simple cycle detection using node coloring (iterative approach)
        WHITE, GRAY, BLACK = 0, 1, 2
        color = {node_id: WHITE for node_id in graph}
        cycles = []

        def has_cycle_from(start):
            stack = [(start, iter(graph.get(start, [])))]
            path = []

            while stack:
                node, children = stack[-1]
                if color[node] == WHITE:
                    color[node] = GRAY
                    path.append(node)

                try:
                    child = next(children)
                    if child and child in graph:  # Valid child node
                        if color[child] == GRAY:
                            # Found cycle
                            cycle_start = path.index(child)
                            cycle_path = path[cycle_start:] + [child]
                            cycles.append(" -> ".join(cycle_path))
                            return True
                        elif color[child] == WHITE:
                            stack.append((child, iter(graph.get(child, []))))
                except StopIteration:
                    # No more children
                    stack.pop()
                    if path and path[-1] == node:
                        path.pop()
                        color[node] = BLACK

            return False

        for node_id in graph:
            if color[node_id] == WHITE:
                if has_cycle_from(node_id):
                    break  # Found at least one cycle

        return cycles[:3]  # Limit to first 3 cycles found
