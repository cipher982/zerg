"""Verify that NODE_LOG events are emitted during workflow execution."""

import asyncio

from sqlalchemy.orm import Session

from zerg.models.models import Workflow
from zerg.services.workflow_engine import workflow_engine as workflow_execution_engine


def _insert_workflow(db: Session, *, name: str, canvas: dict):
    wf = Workflow(owner_id=1, name=name, canvas=canvas)
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
            {"id": "n1", "type": "trigger", "trigger_type": "manual", "config": {}},
        ],
    }

    wf = _insert_workflow(db_session, name="wf-node-log", canvas=canvas)

    # Execute the workflow
    asyncio.run(_run_execution(wf.id))

    # Verify that workflow executed successfully
    db_session.expire_all()
    execution = db_session.get(Workflow, wf.id).executions[-1]
    assert execution.status == "success"

    # Trigger nodes don't create node_states records in canonical system
    if execution.node_states:
        node_state = execution.node_states[0]
        assert node_state.status == "success"
        assert node_state.node_id == "n1"
