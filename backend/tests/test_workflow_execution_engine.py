"""Tests for the *very first* linear WorkflowExecutionEngine implementation."""

from fastapi.testclient import TestClient
from sqlalchemy.orm import Session

# mypy: ignore-errors – tests are not typed strictly

# Helper -----------------------------------------------------------------


def _create_linear_workflow(db: Session, crud_module, owner_id: int, num_nodes: int = 3):
    """Create a dummy linear workflow with *num_nodes* trigger nodes."""

    canvas = {
        "nodes": [
            {
                "id": f"node_{i}",
                "type": "trigger",
                "position": {"x": i * 100, "y": 100},
                "config": {"trigger_type": "manual"},
            }
            for i in range(num_nodes)
        ],
        "edges": [
            {"from_node_id": f"node_{i}", "to_node_id": f"node_{i + 1}", "config": {}} for i in range(num_nodes - 1)
        ],
    }

    return crud_module.create_workflow(
        db=db,
        owner_id=owner_id,
        name="Engine Test Workflow",
        description="workflow for engine unit tests",
        canvas=canvas,
    )


def test_linear_execution_success(
    client: TestClient,
    db: Session,
    auth_headers: dict,
    test_user,
):
    """Ensure that the engine creates execution + node state rows and finishes successfully."""

    # ------------------------------------------------------------------
    # 1. Arrange – create workflow with 3 dummy nodes
    # ------------------------------------------------------------------
    from zerg.crud import crud as crud_mod

    workflow = _create_linear_workflow(db, crud_mod, owner_id=test_user.id)

    # ------------------------------------------------------------------
    # 2. Act – start execution via API (better surface coverage than direct call)
    # ------------------------------------------------------------------
    resp = client.post(f"/api/workflow-executions/{workflow.id}/start", headers=auth_headers)
    assert resp.status_code == 200
    payload = resp.json()

    execution_id = payload["execution_id"]
    assert execution_id > 0

    # ------------------------------------------------------------------
    # 3. Assert – fetch status + logs, expect success
    # ------------------------------------------------------------------
    status_resp = client.get(f"/api/workflow-executions/{execution_id}/status", headers=auth_headers)
    assert status_resp.status_code == 200
    status_data = status_resp.json()
    assert status_data["phase"] == "finished"
    assert status_data["result"] == "success"

    # Node states exist and are completed
    from zerg.models.models import NodeExecutionState

    node_states = db.query(NodeExecutionState).filter_by(workflow_execution_id=execution_id).all()
    assert len(node_states) == 3
    assert all(ns.phase == "finished" and ns.result == "success" for ns in node_states)
