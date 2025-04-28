"""End-to-end smoke test for the new *AgentRunner* execution path.

The goal is to verify that an HTTP request to ``POST /api/threads/{id}/run``
causes the expected *stream_start → stream_chunk → stream_end* sequence to be
delivered over the topic-based WebSocket channel for that thread.

This provides coverage for checklist item **0.1 – Add smoke test** in
``agent_refactor.md``.
"""

import logging
from typing import Dict

# We need *two* independent TestClient instances – one for REST calls and one
# for the WebSocket – otherwise the internal synchronous test transport in
# Starlette will shut down the WS when we issue an HTTP request (see
# https://github.com/encode/starlette/issues/2196).  Therefore we import the
# WebSocket session separately.
from fastapi.testclient import TestClient
from starlette.testclient import TestClient as StarletteTestClient
from starlette.testclient import WebSocketTestSession

# Pydantic models (for MessageType enum)
from zerg.schemas.ws_messages import MessageType

logger = logging.getLogger(__name__)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


from typing import Iterable  # noqa: E402
from typing import Union  # noqa: E402


def _recv_until(
    ws: WebSocketTestSession,
    wanted_types: Iterable[Union[str, MessageType]],
    *,
    max_messages: int = 10,
) -> Dict:
    """Read WS frames until one of *wanted_types* is encountered.

    Raises ``AssertionError`` if we don't see the desired type within
    *max_messages* frames – this prevents an infinite loop in case of a bug
    where the backend never sends the expected message.
    """

    def _to_type_string(t):
        # Accept raw string (already lower-case) or MessageType enum; return
        # the exact lowercase value that appears in outbound JSON.
        from zerg.schemas.ws_messages import MessageType

        if isinstance(t, MessageType):
            return t.value  # e.g. 'stream_start'
        return str(t)

    wanted_as_str = {_to_type_string(t) for t in wanted_types}

    for _ in range(max_messages):
        payload = ws.receive_json()
        mtype = payload.get("type")

        logger.info("WS recv: %s", payload)

        if mtype in wanted_as_str:
            return payload

    assert False, f"Did not receive any of {wanted_as_str} in {max_messages} frames"


# ---------------------------------------------------------------------------
# Smoke test
# ---------------------------------------------------------------------------


def test_thread_run_emits_stream_messages(client: TestClient, sample_agent):
    """Full round-trip: REST + WS → stream messages arrive in order."""

    # ------------------------------------------------------------------
    # 1. Create a thread for the sample agent via REST
    # ------------------------------------------------------------------
    create_thread_resp = client.post(
        "/api/threads",
        json={
            "title": "SmokeTest Thread",
            "agent_id": sample_agent.id,
        },
    )
    assert create_thread_resp.status_code == 201, create_thread_resp.text
    thread_id = create_thread_resp.json()["id"]

    # ------------------------------------------------------------------
    # 2. Open WebSocket and subscribe to the thread topic BEFORE we post the
    #    user message so that we capture every event.
    # ------------------------------------------------------------------
    # Use a *separate* TestClient instance for the WebSocket to avoid the
    # connection being closed when we later perform HTTP requests with the
    # original ``client``.  Both clients share the same underlying FastAPI
    # *app* object so they operate on the same in-memory database.

    websocket_client = StarletteTestClient(client.app, backend="asyncio")

    with websocket_client.websocket_connect("/api/ws") as ws:
        # Subscribe
        ws.send_json(
            {
                "type": "subscribe",
                "topics": [f"thread:{thread_id}"],
                "message_id": "sub-1",
            }
        )

        # First payload must be thread_history
        history = ws.receive_json()
        assert history["type"] == MessageType.THREAD_HISTORY.value
        assert history["thread_id"] == thread_id

        # ------------------------------------------------------------------
        # 3. Create an unprocessed *user* message via REST
        # ------------------------------------------------------------------
        create_msg_resp = client.post(
            f"/api/threads/{thread_id}/messages",
            json={"role": "user", "content": "Hello Agent"},
        )
        assert create_msg_resp.status_code == 201

        # *Optionally* consume the live broadcast for that user message, if
        # the router ever adds one in the future.  We do not fail if it is
        # absent – we simply ignore unknown message types until we see the
        # expected *stream_start* later.

        # ------------------------------------------------------------------
        # 4. Trigger the agent run via REST (synchronous call)
        # ------------------------------------------------------------------
        run_resp = client.post(f"/api/threads/{thread_id}/run")
        assert run_resp.status_code == 202

        # ------------------------------------------------------------------
        # 5. Verify stream sequence: start → chunk → end
        # ------------------------------------------------------------------
        start_msg = _recv_until(ws, {MessageType.STREAM_START})
        assert start_msg["thread_id"] == thread_id

        chunk_msg = _recv_until(ws, {MessageType.STREAM_CHUNK})
        assert chunk_msg["thread_id"] == thread_id
        assert chunk_msg["chunk_type"] == "assistant_message"
        assert chunk_msg["content"] == "stub-response"
        # Each assistant message must carry a unique DB message_id so the
        # frontend starts a new chat bubble.
        assert chunk_msg["message_id"] is not None

        end_msg = _recv_until(ws, {MessageType.STREAM_END})
        assert end_msg["thread_id"] == thread_id

        # If we reach here without assertion errors the smoke test passed.
