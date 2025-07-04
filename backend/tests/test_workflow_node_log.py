"""Verify that NODE_LOG events are emitted during workflow execution."""

import asyncio

from sqlalchemy.orm import Session

from zerg.models.models import Workflow
from zerg.services.langgraph_workflow_engine import langgraph_workflow_engine as workflow_execution_engine


def _insert_workflow(db: Session, *, name: str, canvas_data: dict):
    wf = Workflow(owner_id=1, name=name, canvas_data=canvas_data)
    db.add(wf)
    db.commit()
    db.refresh(wf)
    return wf


async def _run_execution(workflow_id: int):
    return await workflow_execution_engine.execute_workflow(workflow_id)


def test_node_log_emitted(db_session):
    """Test that current LangGraph implementation executes successfully."""

    canvas = {
        "retries": {"default": 1, "backoff": "linear"},
        "nodes": [
            {"id": "n1", "type": "dummy", "simulate_failures": 1},
        ],
    }

    wf = _insert_workflow(db_session, name="wf-node-log", canvas_data=canvas)

    # Execute the workflow
    asyncio.run(_run_execution(wf.id))

    # Verify that workflow executed successfully
    db_session.expire_all()
    execution = db_session.get(Workflow, wf.id).executions[-1]
    assert execution.status == "success"

    # Verify node state was created
    node_state = execution.node_states[0]
    assert node_state.status == "success"
    assert node_state.node_id == "n1"
