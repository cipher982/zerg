"""
Carmack-Style Canonical Workflow Types

Zero runtime parsing. Direct field access. Fail fast at boundaries.
Once data passes validation, it's impossible to have malformed nodes.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from typing import Any
from typing import Dict
from typing import List

# ============================================================================
# Value Objects - Immutable, validated at creation
# ============================================================================


@dataclass(frozen=True)
class NodeId:
    """Strongly-typed node identifier."""

    value: str

    def __post_init__(self):
        if not self.value or not isinstance(self.value, str):
            raise ValueError(f"NodeId must be non-empty string, got: {self.value}")
        if len(self.value) > 100:  # Reasonable limit
            raise ValueError(f"NodeId too long: {len(self.value)} chars")


@dataclass(frozen=True)
class AgentId:
    """Strongly-typed agent identifier."""

    value: int

    def __post_init__(self):
        if not isinstance(self.value, int) or self.value <= 0:
            raise ValueError(f"AgentId must be positive integer, got: {self.value}")


@dataclass(frozen=True)
class Position:
    """Canvas position coordinates."""

    x: float
    y: float

    def __post_init__(self):
        if not isinstance(self.x, (int, float)) or not isinstance(self.y, (int, float)):
            raise ValueError(f"Position coordinates must be numeric, got: x={self.x}, y={self.y}")


# ============================================================================
# Node Types - Composition over inheritance, direct field access
# ============================================================================


class NodeType(Enum):
    """Enumerated node types for type safety."""

    AGENT = "agent"
    TOOL = "tool"
    TRIGGER = "trigger"


@dataclass(frozen=True)
class AgentNodeData:
    """Agent-specific data fields."""

    agent_id: AgentId
    message: str = ""

    def __post_init__(self):
        if not isinstance(self.message, str):
            raise ValueError(f"Agent message must be string, got: {type(self.message)}")


@dataclass(frozen=True)
class ToolNodeData:
    """Tool-specific data fields."""

    tool_name: str
    parameters: Dict[str, Any]

    def __post_init__(self):
        if not self.tool_name or not isinstance(self.tool_name, str):
            raise ValueError(f"Tool name must be non-empty string, got: {self.tool_name}")
        if not isinstance(self.parameters, dict):
            raise ValueError(f"Tool parameters must be dict, got: {type(self.parameters)}")


@dataclass(frozen=True)
class TriggerNodeData:
    """Trigger-specific data fields."""

    trigger_type: str
    config: Dict[str, Any]

    def __post_init__(self):
        if not self.trigger_type or not isinstance(self.trigger_type, str):
            raise ValueError(f"Trigger type must be non-empty string, got: {self.trigger_type}")
        if not isinstance(self.config, dict):
            raise ValueError(f"Trigger config must be dict, got: {type(self.config)}")


# ============================================================================
# Canonical Node - Single representation, no runtime parsing needed
# ============================================================================


@dataclass(frozen=True)
class CanonicalNode:
    """
    Single canonical node representation.

    Key insight: Use composition, not inheritance. All node types share
    common fields (id, position) but have different data payloads.

    This eliminates runtime type checking and parsing - you access
    node.agent_data.agent_id directly, no extraction needed.
    """

    id: NodeId
    position: Position
    node_type: NodeType

    # Union of possible data payloads - only one will be set
    agent_data: AgentNodeData | None = None
    tool_data: ToolNodeData | None = None
    trigger_data: TriggerNodeData | None = None

    def __post_init__(self):
        """Validate data consistency at creation time."""
        data_fields = [self.agent_data, self.tool_data, self.trigger_data]
        non_null_count = sum(1 for field in data_fields if field is not None)

        if non_null_count != 1:
            raise ValueError(f"Exactly one data field must be set, got {non_null_count}")

        # Validate node_type matches data field
        if self.node_type == NodeType.AGENT and not self.agent_data:
            raise ValueError("Agent node must have agent_data")
        elif self.node_type == NodeType.TOOL and not self.tool_data:
            raise ValueError("Tool node must have tool_data")
        elif self.node_type == NodeType.TRIGGER and not self.trigger_data:
            raise ValueError("Trigger node must have trigger_data")

    # Direct field access methods - no parsing, no runtime errors
    @property
    def agent_id(self) -> AgentId:
        """Direct access to agent_id - zero parsing overhead."""
        if not self.agent_data:
            raise ValueError(f"Node {self.id.value} is not an agent node")
        return self.agent_data.agent_id

    @property
    def tool_name(self) -> str:
        """Direct access to tool_name - zero parsing overhead."""
        if not self.tool_data:
            raise ValueError(f"Node {self.id.value} is not a tool node")
        return self.tool_data.tool_name

    @property
    def is_agent(self) -> bool:
        """Type check without isinstance()."""
        return self.node_type == NodeType.AGENT

    @property
    def is_tool(self) -> bool:
        """Type check without isinstance()."""
        return self.node_type == NodeType.TOOL

    @property
    def is_trigger(self) -> bool:
        """Type check without isinstance()."""
        return self.node_type == NodeType.TRIGGER


# ============================================================================
# Workflow Edge - Simple, direct representation
# ============================================================================


@dataclass(frozen=True)
class CanonicalEdge:
    """Workflow edge with direct field access."""

    from_node_id: NodeId
    to_node_id: NodeId
    config: Dict[str, Any] = None

    def __post_init__(self):
        if self.config is None:
            # Use object.__setattr__ to bypass frozen dataclass
            object.__setattr__(self, "config", {})


# ============================================================================
# Complete Workflow - Zero transformation needed at runtime
# ============================================================================


@dataclass(frozen=True)
class CanonicalWorkflow:
    """
    Complete workflow representation.

    Once created, this contains everything needed for execution
    with zero parsing or transformation required.
    """

    id: int
    name: str
    nodes: List[CanonicalNode]
    edges: List[CanonicalEdge]

    def __post_init__(self):
        if not isinstance(self.id, int) or self.id <= 0:
            raise ValueError(f"Workflow ID must be positive integer, got: {self.id}")
        if not isinstance(self.name, str) or not self.name:
            raise ValueError(f"Workflow name must be non-empty string, got: {self.name}")
        if not isinstance(self.nodes, list):
            raise ValueError(f"Nodes must be list, got: {type(self.nodes)}")
        if not isinstance(self.edges, list):
            raise ValueError(f"Edges must be list, got: {type(self.edges)}")

    def get_node_by_id(self, node_id: NodeId) -> CanonicalNode:
        """Direct node lookup - O(n) but simple and reliable."""
        for node in self.nodes:
            if node.id.value == node_id.value:
                return node
        raise ValueError(f"Node not found: {node_id.value}")

    def get_agent_nodes(self) -> List[CanonicalNode]:
        """Get all agent nodes - direct filtering, no parsing."""
        return [node for node in self.nodes if node.is_agent]

    def get_tool_nodes(self) -> List[CanonicalNode]:
        """Get all tool nodes - direct filtering, no parsing."""
        return [node for node in self.nodes if node.is_tool]

    def get_outgoing_edges(self, node_id: NodeId) -> List[CanonicalEdge]:
        """Get edges from a node - direct filtering."""
        return [edge for edge in self.edges if edge.from_node_id.value == node_id.value]

    def get_incoming_edges(self, node_id: NodeId) -> List[CanonicalEdge]:
        """Get edges to a node - direct filtering."""
        return [edge for edge in self.edges if edge.to_node_id.value == node_id.value]


# ============================================================================
# Factory Functions - Clean creation interface
# ============================================================================


def create_agent_node(
    node_id: str, position_x: float, position_y: float, agent_id: int, message: str = ""
) -> CanonicalNode:
    """Factory for agent nodes - validation happens here."""
    return CanonicalNode(
        id=NodeId(node_id),
        position=Position(position_x, position_y),
        node_type=NodeType.AGENT,
        agent_data=AgentNodeData(agent_id=AgentId(agent_id), message=message),
    )


def create_tool_node(
    node_id: str, position_x: float, position_y: float, tool_name: str, parameters: Dict[str, Any] = None
) -> CanonicalNode:
    """Factory for tool nodes - validation happens here."""
    return CanonicalNode(
        id=NodeId(node_id),
        position=Position(position_x, position_y),
        node_type=NodeType.TOOL,
        tool_data=ToolNodeData(tool_name=tool_name, parameters=parameters or {}),
    )


def create_trigger_node(
    node_id: str, position_x: float, position_y: float, trigger_type: str, config: Dict[str, Any] = None
) -> CanonicalNode:
    """Factory for trigger nodes - validation happens here."""
    return CanonicalNode(
        id=NodeId(node_id),
        position=Position(position_x, position_y),
        node_type=NodeType.TRIGGER,
        trigger_data=TriggerNodeData(trigger_type=trigger_type, config=config or {}),
    )
