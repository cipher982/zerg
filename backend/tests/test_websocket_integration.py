import asyncio

import pytest
import pytest_asyncio

from zerg.app.models.models import Thread
from zerg.app.schemas.ws_messages import MessageType
from zerg.main import app

from .ws_test_client import WebSocketTestClient
from .ws_test_client import connect_clients
from .ws_test_client import disconnect_clients


def test_websocket_endpoint_exists():
    """Verify that the WebSocket endpoint exists in the app"""
    ws_routes = [
        route for route in app.routes if route.path == "/api/ws" and "websocket" in str(route.endpoint).lower()
    ]
    assert len(ws_routes) > 0, "WebSocket endpoint for /api/ws is not registered"


@pytest.fixture
def test_thread(db_session, sample_agent):
    """Create a test thread for WebSocket testing"""
    thread = Thread(
        agent_id=sample_agent.id,
        title="WebSocket Test Thread",
        active=True,
        memory_strategy="buffer",
    )
    db_session.add(thread)
    db_session.commit()
    db_session.refresh(thread)
    return thread


@pytest_asyncio.fixture
async def ws_client(client):
    """Create a WebSocket test client for a single connection"""
    base_url = f"ws://localhost:{client.port}"
    print(f"Connecting to WebSocket at {base_url}")

    ws_client = WebSocketTestClient(base_url)
    try:
        await ws_client.connect()
        yield ws_client
    finally:
        await ws_client.disconnect()


@pytest.mark.asyncio
class TestWebSocketIntegration:
    """Integration tests for WebSocket functionality"""

    async def test_connect_disconnect(self, ws_client):
        """Test basic connection and disconnection"""
        # Send a ping to check connectivity
        await ws_client.send_json({"type": "ping", "timestamp": 123456789})
        response = await ws_client.receive_json()
        assert response["type"] == "pong"
        assert "timestamp" in response

    async def test_subscribe_thread(self, ws_client, test_thread):
        """Test subscribing to a thread"""
        # Subscribe to thread topic
        await ws_client.send_json(
            {"type": "subscribe", "topics": [f"thread:{test_thread.id}"], "message_id": "test-sub-1"}
        )

        # Should receive thread history
        response = await ws_client.receive_json()
        assert response["type"] == "thread_history"
        assert response["thread_id"] == test_thread.id
        assert "messages" in response

    async def test_subscribe_invalid_thread(self, ws_client):
        """Test subscribing to a non-existent thread"""
        await ws_client.send_json({"type": "subscribe", "topics": ["thread:999999"], "message_id": "test-sub-2"})

        response = await ws_client.receive_json()
        assert response["type"] == "error"
        assert "Thread 999999 not found" in response["error"]

    async def test_send_message(self, ws_client, test_thread):
        """Test sending a message to a thread"""
        # First subscribe to thread
        await ws_client.send_json(
            {"type": MessageType.SUBSCRIBE_THREAD, "thread_id": test_thread.id, "message_id": "test-sub-3"}
        )
        await ws_client.receive_json()  # Consume history

        # Send test message
        test_content = "Hello, WebSocket world!"
        await ws_client.send_json(
            {
                "type": MessageType.SEND_MESSAGE,
                "thread_id": test_thread.id,
                "content": test_content,
                "message_id": "test-msg-1",
            }
        )

        # Should receive a broadcast message
        response = await ws_client.receive_json()
        assert response["type"] == MessageType.THREAD_MESSAGE
        assert response["thread_id"] == test_thread.id
        assert response["message"]["content"] == test_content

    async def test_multiple_clients(self, client, test_thread):
        """Test message broadcasting to multiple clients subscribed to same thread"""
        base_url = f"ws://localhost:{client.port}"
        clients = await connect_clients(base_url, "/api/ws", 2)

        try:
            # Subscribe both clients to the same thread
            for client in clients:
                await client.send_json(
                    {"type": MessageType.SUBSCRIBE_THREAD, "thread_id": test_thread.id, "message_id": "test-sub-4"}
                )
                await client.receive_json()  # Consume history

            # Client 1 sends a message
            test_content = "Broadcast test"
            await clients[0].send_json(
                {
                    "type": MessageType.SEND_MESSAGE,
                    "thread_id": test_thread.id,
                    "content": test_content,
                    "message_id": "test-msg-2",
                }
            )

            # Create a list to store responses from both clients
            responses = []

            # Wait for both clients to receive the message with a timeout
            async def receive_message(client):
                try:
                    return await client.receive_json()
                except Exception as e:
                    return {"error": str(e)}

            # Gather responses from both clients concurrently
            response_tasks = [receive_message(client) for client in clients]
            responses = await asyncio.gather(*response_tasks)

            # Verify both clients received valid messages
            for response in responses:
                assert "error" not in response, f"Error receiving message: {response.get('error')}"
                assert response["type"] == MessageType.THREAD_MESSAGE
                assert response["message"]["content"] == test_content

        finally:
            await disconnect_clients(clients)
