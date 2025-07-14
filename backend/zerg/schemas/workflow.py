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

    model_config = {"populate_by_name": True}

    from_: str = Field(..., alias="from")
    to: str
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
            if e.from_ not in node_ids or e.to not in node_ids:
                raise ValueError(f"Edge {e.from_}->{e.to} references unknown node")
        return edges

    class Config:
        extra = "forbid"  # Reject unknown fields for security
