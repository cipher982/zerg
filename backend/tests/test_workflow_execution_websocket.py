"""Test WebSocket workflow execution subscription behavior."""

import asyncio
import json

import pytest
from fastapi.testclient import TestClient
from sqlalchemy.orm import Session

from zerg.crud import crud
from zerg.main import app


@pytest.mark.asyncio
async def test_workflow_execution_subscription_snapshot(db: Session):
    """Test that subscribing to a finished workflow execution returns a snapshot."""

    # Create a finished workflow execution
    workflow = crud.create_workflow(db, owner_id=1, name="Test Workflow", description="Test", canvas_data={})
    execution = crud.create_workflow_execution(db, workflow_id=workflow.id, status="success", triggered_by="manual")
    # Update it to have finished timestamps
    execution.status = "success"
    execution.error = None
    db.commit()

    with TestClient(app) as client:
        # WebSocket endpoint is exposed under the API prefix ("/api/ws").
        # Using the fully-qualified path avoids accidental breakage if the
        # router is mounted under a prefix in the future.
        with client.websocket_connect("/api/ws") as websocket:
            # Subscribe to the workflow execution
            subscribe_msg = {
                "type": "subscribe",
                "topics": [f"workflow_execution:{execution.id}"],
                "message_id": "test-1",
            }
            websocket.send_json(subscribe_msg)

            # We should receive a snapshot immediately
            # Wait for up to 2 seconds for the message
            messages = []
            # Collect messages for a short window (~2s) – the snapshot payload
            # is sent immediately by the server so a short loop here is
            # sufficient and avoids relying on *blocking* receive() calls.
            end = asyncio.get_event_loop().time() + 2.0
            while asyncio.get_event_loop().time() < end:
                try:
                    data = websocket.receive_json()
                    messages.append(data)
                except Exception:
                    # No message available yet – yield control briefly.
                    await asyncio.sleep(0.05)

            # Check if we got an execution_finished message
            execution_finished_msgs = [
                msg
                for msg in messages
                if msg.get("type") == "execution_finished" or (msg.get("data", {}).get("type") == "execution_finished")
            ]

            assert len(execution_finished_msgs) > 0, f"Expected execution_finished snapshot but got: {messages}"

            # Verify the message content
            msg = execution_finished_msgs[0]
            if "data" in msg:
                data = msg["data"]
            else:
                data = msg

            assert data.get("execution_id") == execution.id or data.get("data", {}).get("execution_id") == execution.id
