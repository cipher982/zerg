"""Tests for retry/back-off logic in *WorkflowExecutionEngine* v1."""

import asyncio

from sqlalchemy.orm import Session

from zerg.models.models import Workflow
from zerg.services.workflow_engine import workflow_execution_engine

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _insert_workflow(db: Session, *, name: str, canvas_data: dict):
    wf = Workflow(owner_id=1, name=name, canvas_data=canvas_data)  # tests use deterministic *dev@local* id=1
    db.add(wf)
    db.commit()
    db.refresh(wf)
    return wf


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


async def _run_execution(workflow_id: int):
    return await workflow_execution_engine.execute_workflow(workflow_id)


def test_retry_succeeds_within_limit(db_session):  # `db_session` fixture from conftest
    """Node should succeed after configured retries."""

    canvas = {
        "retries": {"default": 2, "backoff": "linear"},
        "nodes": [
            {"id": "n1", "type": "dummy", "simulate_failures": 1},  # fails once, then succeeds
        ],
    }

    wf = _insert_workflow(db_session, name="wf-retry-ok", canvas_data=canvas)

    execution_id = asyncio.run(_run_execution(wf.id))

    # Reload and assert execution success
    db_session.expire_all()
    execution = db_session.get(Workflow, wf.id).executions[-1]
    assert execution.status == "success"
    # Node state should be success too
    node_state = execution.node_states[0]
    assert node_state.status == "success"
    assert node_state.output["attempt"] == 1  # succeeded on 1 retry (attempt index)


def test_retry_exhausted_marks_failed(db_session):
    """Engine must mark execution failed after exhausting retries."""

    canvas = {
        "retries": {"default": 1, "backoff": "linear"},
        "nodes": [
            {"id": "n1", "type": "dummy", "simulate_failures": 5},  # will exceed retries
        ],
    }

    wf = _insert_workflow(db_session, name="wf-retry-fail", canvas_data=canvas)

    execution_id = asyncio.run(_run_execution(wf.id))

    db_session.expire_all()
    execution = db_session.get(Workflow, wf.id).executions[-1]
    assert execution.status == "failed"
    node_state = execution.node_states[0]
    assert node_state.status == "failed"
