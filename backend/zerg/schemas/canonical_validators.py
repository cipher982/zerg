"""
Carmack-Style Input Validation at API Boundary

This module converts raw JSON/dict input into canonical types.
Once data passes through here, it's impossible to have malformed nodes
in the execution engine.

Key principles:
1. Fail fast - reject invalid input immediately
2. Single conversion point - no parsing elsewhere in system
3. Clear error messages - developers know exactly what's wrong
4. Zero runtime overhead after conversion
"""

from __future__ import annotations

import logging
from typing import Any
from typing import Dict
from typing import List

from .canonical_types import CanonicalEdge
from .canonical_types import CanonicalNode
from .canonical_types import CanonicalWorkflow
from .canonical_types import NodeId
from .canonical_types import Position
from .canonical_types import create_agent_node
from .canonical_types import create_tool_node
from .canonical_types import create_trigger_node

logger = logging.getLogger(__name__)


class ValidationError(Exception):
    """Clear validation error with context."""

    def __init__(self, message: str, field_path: str = "", raw_data: Any = None):
        self.message = message
        self.field_path = field_path
        self.raw_data = raw_data
        super().__init__(f"{field_path}: {message}" if field_path else message)


class CanonicalValidator:
    """
    Validates and converts raw input to canonical types.

    This is the ONLY place in the system where we handle messy input data.
    Everything downstream works with clean, validated canonical types.
    """

    @staticmethod
    def validate_workflow(raw_data: Dict[str, Any]) -> CanonicalWorkflow:
        """
        Convert raw workflow data to CanonicalWorkflow.

        This is the main entry point - raw JSON → canonical types.
        """
        try:
            # Validate required fields
            if not isinstance(raw_data, dict):
                raise ValidationError("Workflow data must be a dictionary", raw_data=raw_data)

            workflow_id = CanonicalValidator._validate_workflow_id(raw_data)
            workflow_name = CanonicalValidator._validate_workflow_name(raw_data)
            nodes = CanonicalValidator._validate_nodes(raw_data.get("nodes", []))
            edges = CanonicalValidator._validate_edges(raw_data.get("edges", []))

            return CanonicalWorkflow(id=workflow_id, name=workflow_name, nodes=nodes, edges=edges)

        except ValidationError:
            raise
        except Exception as e:
            raise ValidationError(f"Unexpected error validating workflow: {e}", raw_data=raw_data)

    @staticmethod
    def _validate_workflow_id(raw_data: Dict[str, Any]) -> int:
        """Validate workflow ID field."""
        if "id" not in raw_data:
            raise ValidationError("Missing required field 'id'", field_path="workflow.id")

        workflow_id = raw_data["id"]
        if not isinstance(workflow_id, int) or workflow_id < 0:
            raise ValidationError(
                f"Workflow ID must be non-negative integer, got {type(workflow_id).__name__}: {workflow_id}",
                field_path="workflow.id",
                raw_data=workflow_id,
            )

        return workflow_id

    @staticmethod
    def _validate_workflow_name(raw_data: Dict[str, Any]) -> str:
        """Validate workflow name field."""
        if "name" not in raw_data:
            raise ValidationError("Missing required field 'name'", field_path="workflow.name")

        name = raw_data["name"]
        if not isinstance(name, str) or not name.strip():
            raise ValidationError(
                f"Workflow name must be non-empty string, got {type(name).__name__}: {name}",
                field_path="workflow.name",
                raw_data=name,
            )

        return name.strip()

    @staticmethod
    def _validate_nodes(raw_nodes: List[Any]) -> List[CanonicalNode]:
        """Validate and convert nodes array."""
        if not isinstance(raw_nodes, list):
            raise ValidationError(
                f"Nodes must be array, got {type(raw_nodes).__name__}", field_path="workflow.nodes", raw_data=raw_nodes
            )

        nodes = []
        for i, raw_node in enumerate(raw_nodes):
            try:
                node = CanonicalValidator._validate_single_node(raw_node)
                nodes.append(node)
            except ValidationError as e:
                # Add context about which node failed
                raise ValidationError(
                    e.message, field_path=f"workflow.nodes[{i}].{e.field_path}".rstrip("."), raw_data=e.raw_data
                )

        return nodes

    @staticmethod
    def _validate_single_node(raw_node: Dict[str, Any]) -> CanonicalNode:
        """Validate and convert a single node."""
        if not isinstance(raw_node, dict):
            raise ValidationError(f"Node must be object, got {type(raw_node).__name__}", raw_data=raw_node)

        # Extract and validate common fields
        node_id = CanonicalValidator._validate_node_id(raw_node)
        position = CanonicalValidator._validate_position(raw_node)
        node_type = CanonicalValidator._validate_node_type(raw_node)

        # Create node based on type
        if node_type == "agent" or node_type == "agentidentity":
            return CanonicalValidator._create_agent_node(raw_node, node_id, position)
        elif node_type == "tool":
            return CanonicalValidator._create_tool_node(raw_node, node_id, position)
        elif node_type == "trigger":
            return CanonicalValidator._create_trigger_node(raw_node, node_id, position)
        else:
            raise ValidationError(
                f"Unknown node type: {node_type}. Supported types: agent, tool, trigger",
                field_path="type",
                raw_data=node_type,
            )

    @staticmethod
    def _validate_node_id(raw_node: Dict[str, Any]) -> str:
        """Extract and validate node ID."""
        # Handle both 'id' and 'node_id' for compatibility
        node_id = raw_node.get("id") or raw_node.get("node_id")

        if not node_id:
            raise ValidationError("Missing required field 'id' or 'node_id'", field_path="id")

        if not isinstance(node_id, str) or not node_id.strip():
            raise ValidationError(
                f"Node ID must be non-empty string, got {type(node_id).__name__}: {node_id}",
                field_path="id",
                raw_data=node_id,
            )

        return node_id.strip()

    @staticmethod
    def _validate_position(raw_node: Dict[str, Any]) -> Position:
        """Extract and validate position."""
        position_data = raw_node.get("position", {})

        if not isinstance(position_data, dict):
            raise ValidationError(
                f"Position must be object, got {type(position_data).__name__}",
                field_path="position",
                raw_data=position_data,
            )

        x = position_data.get("x", 0)
        y = position_data.get("y", 0)

        try:
            return Position(float(x), float(y))
        except (TypeError, ValueError) as e:
            raise ValidationError(
                f"Invalid position coordinates: x={x}, y={y}. Error: {e}", field_path="position", raw_data=position_data
            )

    @staticmethod
    def _validate_node_type(raw_node: Dict[str, Any]) -> str:
        """Extract and validate node type."""
        node_type = raw_node.get("type") or raw_node.get("node_type")

        if not node_type:
            raise ValidationError("Missing required field 'type' or 'node_type'", field_path="type")

        # Handle both string and dict formats for compatibility
        if isinstance(node_type, str):
            return node_type.lower()
        elif isinstance(node_type, dict):
            # Handle {"AgentIdentity": {...}} format
            if len(node_type) == 1:
                type_name = list(node_type.keys())[0]
                return type_name.lower()
            else:
                raise ValidationError(
                    f"Dict node type must have exactly one key, got: {list(node_type.keys())}",
                    field_path="type",
                    raw_data=node_type,
                )
        else:
            raise ValidationError(
                f"Node type must be string or object, got {type(node_type).__name__}",
                field_path="type",
                raw_data=node_type,
            )

    @staticmethod
    def _create_agent_node(raw_node: Dict[str, Any], node_id: str, position: Position) -> CanonicalNode:
        """Create agent node from raw data."""
        # Extract agent_id - check multiple possible locations
        agent_id = None

        # Check direct field
        if "agent_id" in raw_node:
            agent_id = raw_node["agent_id"]

        # Check in config
        elif "config" in raw_node and isinstance(raw_node["config"], dict):
            agent_id = raw_node["config"].get("agent_id")

        # Check in node_type dict (legacy format)
        elif "type" in raw_node and isinstance(raw_node["type"], dict):
            type_data = raw_node["type"]
            for key, value in type_data.items():
                if key.lower() in ["agent", "agentidentity"] and isinstance(value, dict):
                    agent_id = value.get("agent_id")
                    break

        if agent_id is None:
            raise ValidationError(
                "Missing required field 'agent_id'. Must be in root, config, or type object", field_path="agent_id"
            )

        if not isinstance(agent_id, int) or agent_id <= 0:
            raise ValidationError(
                f"Agent ID must be positive integer, got {type(agent_id).__name__}: {agent_id}",
                field_path="agent_id",
                raw_data=agent_id,
            )

        # Extract message - check multiple possible locations
        message = ""

        # Check direct field
        if "message" in raw_node:
            message = raw_node["message"]
        # Check in config
        elif "config" in raw_node and isinstance(raw_node["config"], dict):
            message = raw_node["config"].get("message", "")

        if not isinstance(message, str):
            raise ValidationError(
                f"Agent message must be string, got {type(message).__name__}", field_path="message", raw_data=message
            )

        return create_agent_node(node_id, position.x, position.y, agent_id, message)

    @staticmethod
    def _create_tool_node(raw_node: Dict[str, Any], node_id: str, position: Position) -> CanonicalNode:
        """Create tool node from raw data."""
        # Extract tool_name
        tool_name = None

        # Check direct field
        if "tool_name" in raw_node:
            tool_name = raw_node["tool_name"]

        # Check in config
        elif "config" in raw_node and isinstance(raw_node["config"], dict):
            tool_name = raw_node["config"].get("tool_name") or raw_node["config"].get("name")

        # Check in node_type dict
        elif "type" in raw_node and isinstance(raw_node["type"], dict):
            type_data = raw_node["type"]
            for key, value in type_data.items():
                if key.lower() == "tool" and isinstance(value, dict):
                    tool_name = value.get("tool_name")
                    break

        if not tool_name:
            raise ValidationError(
                "Missing required field 'tool_name'. Must be in root, config, or type object", field_path="tool_name"
            )

        if not isinstance(tool_name, str):
            raise ValidationError(
                f"Tool name must be string, got {type(tool_name).__name__}", field_path="tool_name", raw_data=tool_name
            )

        # Extract parameters
        parameters = {}
        if "parameters" in raw_node:
            parameters = raw_node["parameters"]
        elif "config" in raw_node and isinstance(raw_node["config"], dict):
            parameters = raw_node["config"].get("parameters", {})

        if not isinstance(parameters, dict):
            raise ValidationError(
                f"Tool parameters must be object, got {type(parameters).__name__}",
                field_path="parameters",
                raw_data=parameters,
            )

        return create_tool_node(node_id, position.x, position.y, tool_name, parameters)

    @staticmethod
    def _create_trigger_node(raw_node: Dict[str, Any], node_id: str, position: Position) -> CanonicalNode:
        """Create trigger node from raw data."""
        # Extract trigger_type
        trigger_type = None

        # Check direct field
        if "trigger_type" in raw_node:
            trigger_type = raw_node["trigger_type"]

        # Check in config
        elif "config" in raw_node and isinstance(raw_node["config"], dict):
            trigger_type = raw_node["config"].get("trigger_type")

        # Check in node_type dict
        elif "type" in raw_node and isinstance(raw_node["type"], dict):
            type_data = raw_node["type"]
            for key, value in type_data.items():
                if key.lower() == "trigger" and isinstance(value, dict):
                    trigger_type = value.get("trigger_type")
                    break

        if not trigger_type:
            trigger_type = "manual"  # Default trigger type

        if not isinstance(trigger_type, str):
            raise ValidationError(
                f"Trigger type must be string, got {type(trigger_type).__name__}",
                field_path="trigger_type",
                raw_data=trigger_type,
            )

        # Extract config
        config = raw_node.get("config", {})
        if not isinstance(config, dict):
            raise ValidationError(
                f"Trigger config must be object, got {type(config).__name__}", field_path="config", raw_data=config
            )

        return create_trigger_node(node_id, position.x, position.y, trigger_type, config)

    @staticmethod
    def _validate_edges(raw_edges: List[Any]) -> List[CanonicalEdge]:
        """Validate and convert edges array."""
        if not isinstance(raw_edges, list):
            raise ValidationError(
                f"Edges must be array, got {type(raw_edges).__name__}", field_path="workflow.edges", raw_data=raw_edges
            )

        edges = []
        for i, raw_edge in enumerate(raw_edges):
            try:
                edge = CanonicalValidator._validate_single_edge(raw_edge)
                edges.append(edge)
            except ValidationError as e:
                raise ValidationError(
                    e.message, field_path=f"workflow.edges[{i}].{e.field_path}".rstrip("."), raw_data=e.raw_data
                )

        return edges

    @staticmethod
    def _validate_single_edge(raw_edge: Dict[str, Any]) -> CanonicalEdge:
        """Validate and convert a single edge."""
        if not isinstance(raw_edge, dict):
            raise ValidationError(f"Edge must be object, got {type(raw_edge).__name__}", raw_data=raw_edge)

        # Handle multiple field name formats
        from_id = raw_edge.get("from") or raw_edge.get("from_node_id") or raw_edge.get("source")

        to_id = raw_edge.get("to") or raw_edge.get("to_node_id") or raw_edge.get("target")

        if not from_id:
            raise ValidationError("Missing required field: 'from', 'from_node_id', or 'source'", field_path="from")

        if not to_id:
            raise ValidationError("Missing required field: 'to', 'to_node_id', or 'target'", field_path="to")

        if not isinstance(from_id, str) or not from_id.strip():
            raise ValidationError(
                f"Edge source must be non-empty string, got {type(from_id).__name__}: {from_id}",
                field_path="from",
                raw_data=from_id,
            )

        if not isinstance(to_id, str) or not to_id.strip():
            raise ValidationError(
                f"Edge target must be non-empty string, got {type(to_id).__name__}: {to_id}",
                field_path="to",
                raw_data=to_id,
            )

        # Extract config
        config = raw_edge.get("config", {})
        if not isinstance(config, dict):
            config = {}

        return CanonicalEdge(from_node_id=NodeId(from_id.strip()), to_node_id=NodeId(to_id.strip()), config=config)


# ============================================================================
# Convenience Functions for Common Use Cases
# ============================================================================


def validate_workflow_json(json_data: Dict[str, Any]) -> CanonicalWorkflow:
    """
    Main entry point for converting JSON to canonical workflow.

    Use this function in API endpoints to convert incoming JSON
    to validated canonical types.
    """
    return CanonicalValidator.validate_workflow(json_data)


def validate_node_json(json_data: Dict[str, Any]) -> CanonicalNode:
    """
    Validate and convert a single node from JSON.

    Useful for individual node creation/update endpoints.
    """
    return CanonicalValidator._validate_single_node(json_data)


def validate_edge_json(json_data: Dict[str, Any]) -> CanonicalEdge:
    """
    Validate and convert a single edge from JSON.

    Useful for individual edge creation/update endpoints.
    """
    return CanonicalValidator._validate_single_edge(json_data)
