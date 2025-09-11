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

    model_config = ConfigDict(extra="forbid")

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
        extra="forbid",
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
    def _validate_triggers_and_manual_unique(cls, nodes):
        """Strict validation for trigger nodes + uniqueness of manual.

        - Require typed trigger meta at config.trigger with shape { type: str, config: { enabled, params, filters } }
        - Forbid legacy flattened keys (trigger_type, enabled, params, filters) at top-level config
        - Enforce at most one manual trigger (lowercase literal)
        """
        manual_count = 0
        for n in nodes:
            if n.type != "trigger":
                continue
            cfg = n.config or {}

            # Fail fast on legacy keys
            legacy_keys = {k for k in cfg.keys() if k in {"trigger_type", "enabled", "params", "filters"}}
            if legacy_keys:
                raise ValueError(f"Node '{n.id}': legacy trigger keys are not allowed: {sorted(legacy_keys)}")

            trig = cfg.get("trigger")
            if not isinstance(trig, dict):
                raise ValueError(f"Node '{n.id}': trigger node missing required config.trigger")

            ttype = trig.get("type")
            if not isinstance(ttype, str):
                raise ValueError(f"Node '{n.id}': config.trigger.type must be a string")
            if ttype != ttype.lower():
                raise ValueError(f"Node '{n.id}': config.trigger.type must be lowercase literal")

            # Uniqueness of manual
            if ttype == "manual":
                manual_count += 1
                if manual_count > 1:
                    raise ValueError("Only one Manual trigger is allowed per workflow")

            tconf = trig.get("config")
            if not isinstance(tconf, dict):
                raise ValueError(f"Node '{n.id}': config.trigger.config must be an object")

            # Optional fields
            if "enabled" in tconf and not isinstance(tconf.get("enabled"), bool):
                raise ValueError(f"Node '{n.id}': config.trigger.config.enabled must be boolean if present")
            if "params" in tconf and not isinstance(tconf.get("params"), dict):
                raise ValueError(f"Node '{n.id}': config.trigger.config.params must be object if present")
            if "filters" in tconf and not isinstance(tconf.get("filters"), list):
                raise ValueError(f"Node '{n.id}': config.trigger.config.filters must be array if present")

        return nodes

    model_config = ConfigDict(extra="forbid")  # Reject unknown fields for security


# ---------------------------------------------------------------------------
# Trigger meta resolver (strict typed-only)
# ---------------------------------------------------------------------------


def resolve_trigger_meta(node: WorkflowNode) -> dict:
    """Return canonical trigger meta for a trigger node (strict, typed-only)."""
    if node.type != "trigger":
        return {}
    cfg: dict = node.config or {}
    trig = cfg.get("trigger")
    if not isinstance(trig, dict):
        raise ValueError(f"Node '{node.id}': trigger node missing required config.trigger")
    if any(k in cfg for k in ("trigger_type", "enabled", "params", "filters")):
        raise ValueError(f"Node '{node.id}': legacy trigger keys present in node.config")
    ttype = trig.get("type")
    if not isinstance(ttype, str) or ttype != ttype.lower():
        raise ValueError(f"Node '{node.id}': config.trigger.type must be lowercase string")
    tconf = trig.get("config") if isinstance(trig.get("config"), dict) else {}
    enabled = bool(tconf.get("enabled", True))
    params = tconf.get("params") if isinstance(tconf.get("params"), dict) else {}
    filters = tconf.get("filters") if isinstance(tconf.get("filters"), list) else []
    return {"type": ttype, "config": {"enabled": enabled, "params": params, "filters": filters}}
