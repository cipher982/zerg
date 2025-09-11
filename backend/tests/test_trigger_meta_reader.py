"""Unit tests for backend trigger meta resolver (strict typed-only)."""

from zerg.schemas.workflow import Position
from zerg.schemas.workflow import WorkflowNode
from zerg.schemas.workflow import resolve_trigger_meta


def _node(cfg: dict) -> WorkflowNode:
    return WorkflowNode(id="n1", type="trigger", position=Position(x=0.0, y=0.0), config=cfg)


def test_resolve_trigger_meta_typed_only():
    cfg = {"trigger": {"type": "email", "config": {"enabled": True, "params": {"foo": "bar"}, "filters": []}}}
    meta = resolve_trigger_meta(_node(cfg))
    assert meta["type"] == "email"
    assert meta["config"]["enabled"] is True
    assert meta["config"]["params"]["foo"] == "bar"


def test_resolve_trigger_meta_rejects_legacy():
    cfg = {"trigger_type": "schedule", "enabled": False}
    try:
        resolve_trigger_meta(_node(cfg))
        assert False, "expected resolver to raise on legacy keys"
    except Exception:
        pass
