"""Verify that NODE_LOG events are emitted during workflow execution."""

import asyncio

from sqlalchemy.orm import Session

from zerg.models.models import Workflow
from zerg.services.workflow_engine import workflow_execution_engine


def _insert_workflow(db: Session, *, name: str, canvas_data: dict):
    wf = Workflow(owner_id=1, name=name, canvas_data=canvas_data)
    db.add(wf)
    db.commit()
    db.refresh(wf)
    return wf


async def _run_execution(workflow_id: int):
    return await workflow_execution_engine.execute_workflow(workflow_id)


def test_node_log_emitted(db_session):
    """Engine should emit at least one NODE_LOG event for retries and success."""

    canvas = {
        "retries": {"default": 1, "backoff": "linear"},
        "nodes": [
            {"id": "n1", "type": "dummy", "simulate_failures": 1},
        ],
    }

    wf = _insert_workflow(db_session, name="wf-node-log", canvas_data=canvas)

    captured: list = []

    # Monkey-patch the *publish* helper so we capture exactly the log lines
    # originating from this execution without relying on the global
    # EventBus timing (which can be flaky when other tests run in the same
    # session).

    original_publish = workflow_execution_engine._publish_node_log  # type: ignore[attr-defined]

    def _capture(**kwargs):  # type: ignore[no-untyped-def]
        captured.append(kwargs)
        original_publish(**kwargs)  # Make sure normal flow continues

    workflow_execution_engine._publish_node_log = staticmethod(_capture)  # type: ignore[assignment]

    try:
        asyncio.run(_run_execution(wf.id))
    finally:
        # Restore original helper so other tests are unaffected
        workflow_execution_engine._publish_node_log = staticmethod(original_publish)  # type: ignore[assignment]

    # We expect at least the RETRY line and the success line (>=2)
    assert len(captured) >= 2, "Expected NODE_LOG events to be emitted"

    # The content should include our node_id
    assert any(log["node_id"] == "n1" for log in captured)
