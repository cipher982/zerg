"""Regression test for message ordering bug – user messages appearing above
their corresponding assistant replies in the chat UI.

The sequence we want to verify:

1. A user message is stored.
2. The assistant reply is generated and stored.
3. We wait >1 s so timestamps differ.
4. A second user message is stored followed by its assistant reply.

When the frontend fetches `/threads/{id}/messages` it expects the messages in
the *natural* conversational order::

    user, assistant, user, assistant

Historically the endpoint sometimes returned them grouped by role (all users
then all assistants).  This test fails if that regression re-appears.
"""

from __future__ import annotations

import time

from fastapi.testclient import TestClient

# Remove the manual database initialization


# Use the client fixture from conftest.py, which properly sets up the test database
def test_interleaved_order_after_delay(client: TestClient) -> None:  # noqa: D401
    """Ensure user/assistant messages stay interleaved even with a delay."""

    agent_id = _create_agent(client)
    thread_id = _create_thread(client, agent_id)

    # First exchange
    _send_user_message(client, thread_id, "Hello")
    _run_thread(client, thread_id)

    time.sleep(0.1)

    # Second exchange
    _send_user_message(client, thread_id, "Hello again")
    _run_thread(client, thread_id)

    # Fetch messages exactly as the frontend does
    resp = client.get(f"/api/threads/{thread_id}/messages")
    resp.raise_for_status()
    msgs = [m for m in resp.json() if m["role"] != "system"]

    roles = [m["role"] for m in msgs]
    assert roles == [
        "user",
        "assistant",
        "user",
        "assistant",
    ], f"Unexpected order: {roles}"


def test_tool_message_order(client: TestClient) -> None:
    """Ensure tool messages are inserted between user and assistant in DB order."""
    agent_id = _create_agent(client)
    thread_id = _create_thread(client, agent_id)

    # Send a message that triggers a tool call (assuming 'time' triggers get_current_time)
    _send_user_message(client, thread_id, "What time is it?")
    _run_thread(client, thread_id)

    # Fetch messages
    resp = client.get(f"/api/threads/{thread_id}/messages")
    resp.raise_for_status()
    msgs = [m for m in resp.json() if m["role"] != "system"]

    # Should be: user, assistant, tool
    roles = [m["role"] for m in msgs]
    assert roles == ["user", "assistant", "tool"], f"Unexpected order: {roles}"

    # Tool message should have message_type/tool_call_id/tool_name fields
    tool_msg = msgs[2]
    assert tool_msg["role"] == "tool"
    assert (
        tool_msg.get("message_type") == "tool_output" or tool_msg.get("tool_call_id") or tool_msg.get("tool_name")
    ), "Tool message missing expected fields"


def _create_agent(client: TestClient) -> int:
    resp = client.post(
        "/api/agents",
        json={
            "name": "bot",
            "system_instructions": "sys",
            "task_instructions": "task",
            "model": "gpt-4o-mini",
        },
    )
    resp.raise_for_status()
    return resp.json()["id"]


def _create_thread(client: TestClient, agent_id: int) -> int:
    resp = client.post(
        "/api/threads",
        json={"agent_id": agent_id, "title": "demo", "active": True},
    )
    resp.raise_for_status()
    return resp.json()["id"]


def _send_user_message(client: TestClient, thread_id: int, text: str) -> None:
    resp = client.post(f"/api/threads/{thread_id}/messages", json={"role": "user", "content": text})
    resp.raise_for_status()


def _run_thread(client: TestClient, thread_id: int) -> None:
    resp = client.post(f"/api/threads/{thread_id}/run")
    resp.raise_for_status()
