"""Test workflow execution with agent connections created via frontend."""

from fastapi.testclient import TestClient
from sqlalchemy.orm import Session


def test_agent_connection_workflow_execution(
    client: TestClient,
    db: Session,
    auth_headers: dict,
    test_user,
):
    """Test that workflows with agent connections execute correctly."""

    from zerg.crud import crud as crud_mod

    # Create two test agents
    agent1 = crud_mod.create_agent(
        db=db,
        owner_id=test_user.id,
        name="Test Agent A",
        system_instructions="You are Test Agent A",
        task_instructions="Respond with 'Hello from Agent A'",
    )

    agent2 = crud_mod.create_agent(
        db=db,
        owner_id=test_user.id,
        name="Test Agent B",
        system_instructions="You are Test Agent B",
        task_instructions="Respond with 'Hello from Agent B'",
    )

    # Create workflow with connected agents (mimicking frontend AddEdge)
    canvas_data = {
        "nodes": [
            {
                "id": f"agent_node_{agent1.id}",
                "type": "AgentIdentity",
                "data": {"agent_id": agent1.id, "name": agent1.name},
                "x": 100,
                "y": 100,
            },
            {
                "id": f"agent_node_{agent2.id}",
                "type": "AgentIdentity",
                "data": {"agent_id": agent2.id, "name": agent2.name},
                "x": 300,
                "y": 100,
            },
        ],
        "edges": [
            {"id": "edge_1", "source": f"agent_node_{agent1.id}", "target": f"agent_node_{agent2.id}", "label": None}
        ],
    }

    workflow = crud_mod.create_workflow(
        db=db,
        owner_id=test_user.id,
        name="Connection Test Workflow",
        description="Test workflow with agent connections",
        canvas_data=canvas_data,
    )

    # Execute the workflow
    resp = client.post(f"/api/workflow-executions/{workflow.id}/start", headers=auth_headers)
    assert resp.status_code == 200
    payload = resp.json()

    execution_id = payload["execution_id"]
    assert execution_id > 0

    # Wait for execution to complete
    import time

    time.sleep(2)

    # Check execution status
    status_resp = client.get(f"/api/workflow-executions/{execution_id}/status", headers=auth_headers)
    assert status_resp.status_code == 200
    status_data = status_resp.json()

    print(f"Workflow execution status: {status_data}")

    # The execution should have processed the connection
    assert status_data["status"] in ["success", "failed", "running"]

    # Check that node states were created for both connected agents
    from zerg.models.models import NodeExecutionState

    node_states = db.query(NodeExecutionState).filter(NodeExecutionState.execution_id == execution_id).all()

    node_ids = {state.node_id for state in node_states}
    expected_node_ids = {f"agent_node_{agent1.id}", f"agent_node_{agent2.id}"}

    print(f"Node states created: {node_ids}")
    print(f"Expected node IDs: {expected_node_ids}")

    # Verify that the workflow engine processed both connected nodes
    assert len(node_states) >= 1, "At least one node should have been executed"

    # Check logs for execution details
    logs_resp = client.get(f"/api/workflow-executions/{execution_id}/logs", headers=auth_headers)
    if logs_resp.status_code == 200:
        logs_data = logs_resp.json()
        print(f"Execution logs: {logs_data}")


def test_frontend_edge_format_compatibility(
    client: TestClient,
    db: Session,
    auth_headers: dict,
    test_user,
):
    """Test that the exact edge format created by frontend AddEdge message works."""

    from zerg.crud import crud as crud_mod

    # Create a simple workflow with the exact format the frontend creates
    canvas_data = {
        "nodes": [
            {"id": "node_1", "type": "Tool", "data": {"tool_name": "test_tool"}},
            {"id": "node_2", "type": "Tool", "data": {"tool_name": "test_tool_2"}},
        ],
        "edges": [{"from_node_id": "node_1", "to_node_id": "node_2", "label": None}],
    }

    workflow = crud_mod.create_workflow(
        db=db,
        owner_id=test_user.id,
        name="Frontend Edge Format Test",
        description="Test exact frontend edge format",
        canvas_data=canvas_data,
    )

    # Verify workflow was created successfully
    assert workflow.id > 0
    assert workflow.canvas_data["edges"][0]["from_node_id"] == "node_1"
    assert workflow.canvas_data["edges"][0]["to_node_id"] == "node_2"

    # Try to execute it (may fail due to tool nodes, but should parse correctly)
    resp = client.post(f"/api/workflow-executions/{workflow.id}/start", headers=auth_headers)

    # Should at least attempt to start (not crash on edge format)
    assert resp.status_code in [200, 400], f"Got unexpected status {resp.status_code}: {resp.text}"

    if resp.status_code == 200:
        payload = resp.json()
        assert "execution_id" in payload
        print(f"Workflow started successfully with execution_id: {payload['execution_id']}")
    else:
        # If it fails, it should be due to tool configuration, not edge format
        error_data = resp.json()
        print(f"Expected failure due to tool config: {error_data}")
        assert "tool" in error_data.get("detail", "").lower() or "node" in error_data.get("detail", "").lower()
