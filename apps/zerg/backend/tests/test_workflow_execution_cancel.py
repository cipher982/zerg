"""Tests for the /workflow-executions/{id}/cancel endpoint."""

import asyncio

import pytest
from fastapi.testclient import TestClient

from zerg.models.models import Workflow
from zerg.services.workflow_engine import workflow_engine as workflow_execution_engine

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _insert_workflow(db, *, name: str):
    wf = Workflow(owner_id=1, name=name, canvas={"nodes": [], "edges": []})
    db.add(wf)
    db.commit()
    db.refresh(wf)
    return wf


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


def test_cancel_endpoint_sets_status(client: TestClient, db_session):
    wf = _insert_workflow(db_session, name="wf-cancel")

    # Start execution (async but returns immediately after run due to empty nodes)
    execution_id = asyncio.run(workflow_execution_engine.execute_workflow(wf.id))

    # Cancel should return 409 because already finished
    resp = client.patch(f"/api/workflow-executions/{execution_id}/cancel", json={"reason": "user pressed stop"})
    assert resp.status_code == 409


# TODO: Rewrite this test for LangGraph engine
# Test removed due to incompatibility with new LangGraph workflow engine
# Original test attempted to cancel a running workflow execution
