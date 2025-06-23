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
        with client.websocket_connect("/ws") as websocket:
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
            try:
                # Collect messages for a short time
                start_time = asyncio.get_event_loop().time()
                while asyncio.get_event_loop().time() - start_time < 2.0:
                    try:
                        data = websocket.receive_json(timeout=0.1)
                        messages.append(data)
                        print(f"Received message: {json.dumps(data, indent=2)}")
                    except Exception:
                        # Timeout is expected, just continue
                        await asyncio.sleep(0.05)
            except Exception as e:
                print(f"Error receiving messages: {e}")

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
