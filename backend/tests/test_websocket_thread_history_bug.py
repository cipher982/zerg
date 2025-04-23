"""Failing test for thread‑history subscription bug.

The desired behaviour: subscribing to a thread that already contains
messages should yield a ``thread_history`` payload with those messages.

Current behaviour (before the bug fix): an ``error`` payload is returned.
This test therefore fails – confirming the issue is reproducible.
"""

import pytest
from fastapi.testclient import TestClient
from starlette.testclient import WebSocketTestSession

from zerg.app.models.models import Thread


@pytest.fixture
def ws_client(test_client: TestClient) -> WebSocketTestSession:
    """Fresh WebSocket connection for each test."""
    with test_client.websocket_connect("/api/ws") as websocket:
        yield websocket


def test_subscribe_thread_with_existing_messages_returns_history(
    ws_client: WebSocketTestSession,
    sample_thread: Thread,
    sample_thread_messages,  # populated by fixture
):
    """Subscribing should send thread_history including existing messages."""

    # Ensure fixture created at least one message for the thread
    assert sample_thread.messages, "Fixture must create initial messages"

    ws_client.send_json({"type": "subscribe", "topics": [f"thread:{sample_thread.id}"], "message_id": "bug-repro-1"})

    response = ws_client.receive_json()

    # Expect thread_history – this assertion fails with current bug
    assert response["type"] == "thread_history", response
    assert response["thread_id"] == sample_thread.id
    assert len(response["messages"]) == len(sample_thread.messages)
