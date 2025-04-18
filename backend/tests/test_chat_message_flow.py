import logging

import pytest
import pytest_asyncio

from zerg.app.schemas.ws_messages import MessageType

from .ws_test_client import WebSocketTestClient

# Set up logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


@pytest.fixture
def test_thread(db_session, sample_agent):
    """Create a test thread for WebSocket testing"""
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
    """Create a WebSocket test client for a single connection"""
    base_url = f"ws://localhost:{client.port}"
    logger.info(f"Connecting to WebSocket at {base_url}")

    ws_client = WebSocketTestClient(base_url)
    try:
        await ws_client.connect(path="/api/ws")
        yield ws_client
    finally:
        await ws_client.disconnect()


@pytest.mark.asyncio
class TestChatMessageFlow:
    """Test suite for verifying chat message flow through websockets"""

    async def test_basic_connection(self, ws_client):
        """Verify basic websocket connection works"""
        # Send a ping to check connectivity
        await ws_client.send_json({"type": MessageType.PING})
        response = await ws_client.receive_json()
        assert response["type"] == MessageType.PONG

    async def test_chat_message_flow(self, ws_client, test_thread):
        """Test the complete flow of sending a chat message and receiving response"""
        logger.info("Starting chat message flow test")

        # 1. Subscribe to thread first
        sub_msg = {"type": MessageType.SUBSCRIBE_THREAD, "thread_id": test_thread.id, "message_id": "test-sub-1"}
        await ws_client.send_json(sub_msg)

        # Should receive thread history
        history = await ws_client.receive_json()
        assert history["type"] == MessageType.THREAD_HISTORY
        assert history["thread_id"] == test_thread.id

        # 2. Send test message
        message = {
            "type": MessageType.SEND_MESSAGE,
            "thread_id": test_thread.id,
            "content": "Test message",
            "message_id": "test-msg-1",
        }
        logger.info(f"Sending message: {message}")
        await ws_client.send_json(message)

        # 3. Wait for response with timeout
        try:
            response = await ws_client.receive_json(timeout=5.0)
            logger.info(f"Received response: {response}")
            assert response["type"] == MessageType.THREAD_MESSAGE
            assert "message" in response
            assert "content" in response["message"]
        except TimeoutError:
            logger.error("No response received within timeout")
            raise

    async def test_multiple_messages(self, ws_client, test_thread):
        """Test sending multiple messages in sequence"""
        # First subscribe to thread
        await ws_client.send_json(
            {"type": MessageType.SUBSCRIBE_THREAD, "thread_id": test_thread.id, "message_id": "test-sub-2"}
        )
        await ws_client.receive_json()  # Consume history

        # Send multiple messages
        messages = ["First test message", "Second test message", "Third test message"]

        for idx, content in enumerate(messages):
            message = {
                "type": MessageType.SEND_MESSAGE,
                "thread_id": test_thread.id,
                "content": content,
                "message_id": f"test-msg-{idx+1}",
            }
            logger.info(f"Sending message {idx+1}: {message}")
            await ws_client.send_json(message)

            # Verify response for each message
            response = await ws_client.receive_json(timeout=5.0)
            assert response["type"] == MessageType.THREAD_MESSAGE
            assert response["message"]["content"] == content
