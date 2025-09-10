"""
Single source of truth for workflow data structures.

This module defines the WorkflowData schema used across API, DB, and execution engine.
Replaces the old canonical/transformer/serializer stack.
"""

from typing import Any
from typing import Dict
from typing import List
from typing import Literal

from pydantic import BaseModel
from pydantic import ConfigDict
from pydantic import Field
from pydantic import field_validator


class Position(BaseModel):
    """Node position on canvas."""

    x: float
    y: float


class WorkflowNode(BaseModel):
    """A workflow node (agent, tool, trigger, or conditional)."""

    id: str
    type: Literal["agent", "tool", "trigger", "conditional"]
    position: Position
    config: Dict[str, Any] = Field(default_factory=dict)


class WorkflowEdge(BaseModel):
    """A directed edge connecting two nodes."""

    model_config = ConfigDict(
        populate_by_name=True,
        # Consistent field naming - no more aliasing confusion
        alias_generator=lambda field_name: field_name,
    )

    from_node_id: str  # ✅ Consistent with frontend - no more "from" alias
    to_node_id: str  # ✅ Consistent with frontend - no more confusion
    config: Dict[str, Any] = Field(default_factory=dict)


class WorkflowData(BaseModel):
    """Complete workflow specification."""

    nodes: List[WorkflowNode]
    edges: List[WorkflowEdge]

    @field_validator("edges")
    @classmethod
    def _check_edges(cls, edges, info):
        """Validate edge references to existing nodes."""
        if "nodes" not in info.data:
            return edges

        node_ids = {n.id for n in info.data["nodes"]}
        for e in edges:
            if e.from_node_id not in node_ids or e.to_node_id not in node_ids:
                raise ValueError(f"Edge {e.from_node_id}->{e.to_node_id} references unknown node")
        return edges

    @field_validator("nodes")
    @classmethod
    def _check_manual_trigger_unique(cls, nodes):
        """Frontend invariant: at most one Manual trigger per workflow.
        Enforce here for backend hygiene as well.
        """
        manual_count = 0
        for n in nodes:
            if n.type != "trigger":
                continue
            cfg = n.config or {}
            # Accept both flattened key and (future) typed meta
            ttype = (cfg.get("trigger_type") or cfg.get("trigger", {}).get("type") or "").lower()
            if ttype == "manual":
                manual_count += 1
                if manual_count > 1:
                    raise ValueError("Only one Manual trigger is allowed per workflow")
        return nodes

    model_config = ConfigDict(extra="forbid")  # Reject unknown fields for security
