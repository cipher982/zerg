"""Ensure that if the backend produces **multiple** assistant messages in one
turn, the WebSocket layer emits separate stream sequences so the frontend can
render multiple bubbles instead of concatenating them.
"""

from types import SimpleNamespace

import pytest
from fastapi.testclient import TestClient
from starlette.testclient import TestClient as StarletteTestClient

from zerg.schemas.ws_messages import MessageType


@pytest.fixture
def two_assistant_rows():
    """Return two dummy ORM-like rows with id/content attributes."""

    r1 = SimpleNamespace(id=101, content="Msg A", role="assistant")
    r2 = SimpleNamespace(id=102, content="Msg B", role="assistant")
    return [r1, r2]


def test_multiple_assistant_messages_emit_multiple_sequences(
    monkeypatch, client: TestClient, sample_agent, two_assistant_rows
):
    """Monkey-patch AgentRunner so we can control its output."""

    # 1. Create thread
    resp = client.post(
        "/api/threads",
        json={"title": "bubble-test", "agent_id": sample_agent.id},
    )
    thread_id = resp.json()["id"]

    # 2. Patch AgentRunner.run_thread to return our two assistant rows.
    from zerg.managers import agent_runner as ar_mod

    async def _fake_run_thread(self, db, thread):  # noqa: D401
        return two_assistant_rows

    monkeypatch.setattr(ar_mod.AgentRunner, "run_thread", _fake_run_thread)

    # 3. Open WS
    ws_client = StarletteTestClient(client.app, backend="asyncio")

    with ws_client.websocket_connect("/api/ws") as ws:
        ws.send_json({"type": "subscribe", "topics": [f"thread:{thread_id}"], "message_id": "sub"})

        # 4. Insert a user message so the /run endpoint has work to do
        client.post(
            f"/api/threads/{thread_id}/messages",
            json={"role": "user", "content": "hi"},
        )

        # 5. Trigger run
        client.post(f"/api/threads/{thread_id}/run")

        # New protocol (2025-04): backend emits **one** stream sequence per
        # agent turn, with multiple *stream_chunk* payloads for each assistant
        # or tool message.  We therefore expect:
        #   start, chunk(A), chunk(B), end

        expected_cycle = [
            MessageType.STREAM_START,
            MessageType.STREAM_CHUNK,
            MessageType.STREAM_CHUNK,
            MessageType.STREAM_END,
        ]

        message_ids = []
        for expected in expected_cycle:
            payload = ws.receive_json()
            assert payload["type"] == expected.value
            if expected == MessageType.STREAM_CHUNK:
                message_ids.append(payload["message_id"])

        # Ensure the two assistant IDs both appeared in order
        assert message_ids == ["101", "102"]
