"""
Integration test: simulate a frontend using REST for sends and WebSocket for updates.
"""

from fastapi.testclient import TestClient

from zerg.schemas.ws_messages import MessageType


def test_http_rest_and_ws_events(sample_agent, test_client: TestClient):
    # 1. Create a thread via REST
    create_resp = test_client.post(
        "/api/threads",
        json={"title": "IntegrationTest", "agent_id": sample_agent.id},
    )
    assert create_resp.status_code == 201, create_resp.text
    thread = create_resp.json()
    thread_id = thread["id"]

    # 2. Fetch initial history via REST (initial load)
    hist_resp = test_client.get(f"/api/threads/{thread_id}/messages")
    assert hist_resp.status_code == 200, hist_resp.text
    hist = hist_resp.json()
    # Expect at least the system message inserted at thread creation
    assert isinstance(hist, list)
    assert len(hist) >= 1

    # 3. Connect WebSocket for live updates
    with test_client.websocket_connect(f"/api/ws?initial_topics=thread:{thread_id}") as ws:
        # No initial history over WS; proceed to send and receive live events

        # 3. Send a user message via WS (frontend uses WebSocket send)
        send_req = {
            "type": MessageType.SEND_MESSAGE.value,
            "thread_id": thread_id,
            "content": "Hello frontend",
            "message_id": "msg-1",
        }
        ws.send_json(send_req)

        # 4. Expect a THREAD_MESSAGE WS event for our new message
        ev = ws.receive_json()
        assert ev.get("type") == MessageType.THREAD_MESSAGE.value
        assert ev.get("thread_id") == thread_id
        assert ev.get("message", {}).get("content") == "Hello frontend"

        # 5. Trigger the agent run via REST
        run_resp = test_client.post(f"/api/threads/{thread_id}/run")
        assert run_resp.status_code == 202, run_resp.text

        # 6. Expect stream_start
        start = ws.receive_json()
        assert start.get("type") == MessageType.STREAM_START.value
        assert start.get("thread_id") == thread_id

        # 7. Expect stream_chunk
        chunk = ws.receive_json()
        assert chunk.get("type") == MessageType.STREAM_CHUNK.value
        assert chunk.get("thread_id") == thread_id
        assert chunk.get("content")

        # 8. Expect stream_end
        end = ws.receive_json()
        assert end.get("type") == MessageType.STREAM_END.value
        assert end.get("thread_id") == thread_id
