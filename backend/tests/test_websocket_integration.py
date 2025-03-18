import pytest
import pytest_asyncio

from zerg.app.schemas.ws_messages import MessageType
from zerg.app.schemas.ws_messages import PingMessage
from zerg.app.schemas.ws_messages import SendMessageRequest
from zerg.app.schemas.ws_messages import SubscribeThreadMessage
from zerg.main import app

from .ws_test_client import WebSocketTestClient
from .ws_test_client import connect_clients
from .ws_test_client import disconnect_clients


# First verify that our WebSocket endpoint is registered
def test_websocket_endpoint_exists():
    """Verify that the WebSocket endpoint exists in the app"""
    ws_routes = [
        route for route in app.routes if route.path == "/api/ws" and "websocket" in str(route.endpoint).lower()
    ]
    assert len(ws_routes) > 0, "WebSocket endpoint for /api/ws is not registered"


@pytest.fixture
def test_thread(db_session, sample_agent):
    """
    Create a test thread for WebSocket testing
    """
    from zerg.app.models.models import Thread

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
    """
    Create a WebSocket test client for a single connection
    """
    # Check that the server is running
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

    # Removed skip decorator to test basic connection
    async def test_connect_disconnect(self, ws_client):
        """Test basic connection and disconnection"""
        # Connection is established by the fixture
        # Send a ping to check connectivity
        await ws_client.send_json({"type": MessageType.PING})
        response = await ws_client.receive_json()
        assert response["type"] == MessageType.PONG
        # Disconnection is handled by the fixture

    # Removed skip decorator from this test to try it first
    async def test_ping_pong(self, ws_client):
        """Test ping/pong heartbeat functionality"""
        timestamp = 123456789
        ping_msg = PingMessage(timestamp=timestamp).model_dump()
        await ws_client.send_json(ping_msg)

        response = await ws_client.receive_json()
        assert response["type"] == MessageType.PONG
        assert "timestamp" in response

    # Removed skip decorator to test thread subscription
    async def test_subscribe_valid_thread(self, ws_client, test_thread):
        """Test subscribing to a valid thread"""
        sub_msg = SubscribeThreadMessage(thread_id=test_thread.id).model_dump()
        await ws_client.send_json(sub_msg)

        response = await ws_client.receive_json()
        assert response["type"] == MessageType.THREAD_HISTORY
        assert response["thread_id"] == test_thread.id
        assert "messages" in response

    # Removed skip decorator to test error handling for invalid thread
    async def test_subscribe_invalid_thread(self, ws_client):
        """Test subscribing to a non-existent thread"""
        sub_msg = SubscribeThreadMessage(thread_id=999999).model_dump()
        await ws_client.send_json(sub_msg)

        response = await ws_client.receive_json()
        assert response["type"] == MessageType.ERROR
        assert "thread" in response["error"].lower()

    # Removed skip decorator to test message sending
    async def test_send_message(self, ws_client, test_thread):
        """Test sending a message to a thread"""
        # First subscribe to the thread
        sub_msg = SubscribeThreadMessage(thread_id=test_thread.id).model_dump()
        await ws_client.send_json(sub_msg)
        await ws_client.receive_json()  # Consume the THREAD_HISTORY response

        # Now send a message
        test_content = "Hello, WebSocket world!"
        send_msg = SendMessageRequest(thread_id=test_thread.id, content=test_content).model_dump()
        await ws_client.send_json(send_msg)

        # We should receive a broadcast message with our content
        response = await ws_client.receive_json()
        assert response["type"] == MessageType.THREAD_MESSAGE
        assert response["thread_id"] == test_thread.id
        assert response["message"]["content"] == test_content

    # Removed skip decorator to test multiple clients
    async def test_multiple_clients(self, client, test_thread):
        """Test message broadcasting to multiple clients subscribed to same thread"""
        # Connect two clients
        base_url = f"ws://localhost:{client.port}"
        clients = await connect_clients(base_url, "/api/ws", 2)

        try:
            # Subscribe both clients to the same thread
            sub_msg = SubscribeThreadMessage(thread_id=test_thread.id).model_dump()
            await clients[0].send_json(sub_msg)
            await clients[1].send_json(sub_msg)

            # Consume THREAD_HISTORY responses
            await clients[0].receive_json()
            await clients[1].receive_json()

            # Client 1 sends a message
            test_content = "Broadcast test"
            send_msg = SendMessageRequest(thread_id=test_thread.id, content=test_content).model_dump()
            await clients[0].send_json(send_msg)

            # Both clients should receive the broadcast
            response1 = await clients[0].receive_json()
            response2 = await clients[1].receive_json()

            assert response1["type"] == MessageType.THREAD_MESSAGE
            assert response2["type"] == MessageType.THREAD_MESSAGE
            assert response1["message"]["content"] == test_content
            assert response2["message"]["content"] == test_content

        finally:
            # Clean up connections
            await disconnect_clients(clients)
