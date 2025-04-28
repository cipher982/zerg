"""Regression test for issue #XYZ – assistant replies must arrive as **separate**
`stream_chunk` messages across consecutive runs so the frontend does not
concatenate them into a single chat bubble.

We send two user messages in a row and verify that the backend emits two
`stream_chunk` payloads with **different** `message_id` values.
"""

from fastapi.testclient import TestClient
from starlette.testclient import TestClient as StarletteTestClient
from starlette.testclient import WebSocketTestSession

from zerg.schemas.ws_messages import MessageType


def _recv_until(ws: WebSocketTestSession, wanted_type: MessageType):
    """Helper to receive until we encounter a payload of *wanted_type*."""

    while True:
        data = ws.receive_json()
        if data.get("type") == wanted_type.value:
            return data


def test_two_turns_emit_distinct_assistant_messages(client: TestClient, sample_agent):
    """Two turns ⇒ two different assistant message IDs in the stream."""

    # Create thread
    resp = client.post(
        "/api/threads",
        json={"title": "multi-turn", "agent_id": sample_agent.id},
    )
    thread_id = resp.json()["id"]

    # Separate WS client
    ws_client = StarletteTestClient(client.app, backend="asyncio")

    with ws_client.websocket_connect("/api/ws") as ws:
        ws.send_json({"type": "subscribe", "topics": [f"thread:{thread_id}"], "message_id": "sub"})
        _ = ws.receive_json()  # thread_history

        assistant_ids = []

        for turn in range(2):
            # Post user message
            client.post(
                f"/api/threads/{thread_id}/messages",
                json={"role": "user", "content": f"hello {turn}"},
            )

            # Trigger run
            client.post(f"/api/threads/{thread_id}/run")

            _recv_until(ws, MessageType.STREAM_START)
            chunk = _recv_until(ws, MessageType.STREAM_CHUNK)
            _recv_until(ws, MessageType.STREAM_END)

            assistant_ids.append(chunk["message_id"])

        # The two assistant message IDs must be present and distinct
        assert all(assistant_ids)
        assert len(set(assistant_ids)) == 2
