import logging

import pytest

from zerg.schemas.ws_messages import MessageType

# Set up logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


@pytest.fixture
def test_thread(db_session, sample_agent):
    """Create a test thread for WebSocket testing"""
    from zerg.models.models import Thread

    thread = Thread(
        agent_id=sample_agent.id,
        title="WebSocket Test Thread",
        active=True,
        memory_strategy="buffer",
    )
    db_session.add(thread)
    db_session.commit()
    db_session.refresh(thread)
    logger.info(f"Created test thread with ID: {thread.id}")
    return thread


@pytest.fixture
def ws_client(client):
    """Create a WebSocket test client for a single connection"""
    logger.info("Connecting to WebSocket at /api/ws")

    with client.websocket_connect("/api/ws") as websocket:
        yield websocket


class TestChatMessageFlow:
    """Test suite for verifying chat message flow through websockets"""

    def test_basic_connection(self, ws_client):
        """Verify basic websocket connection works"""
        # Send a ping to check connectivity
        ws_client.send_json({"type": MessageType.PING})
        raw_response = ws_client.receive_json()
        response = raw_response.get("data", raw_response)
        assert response["type"] == MessageType.PONG

    def test_chat_message_flow(self, ws_client, test_thread):
        """Test the complete flow of sending a chat message and receiving response"""
        logger.info("Starting chat message flow test")

        # 1. Subscribe to thread first
        sub_msg = {
            "type": MessageType.SUBSCRIBE_THREAD,
            "thread_id": test_thread.id,
            "message_id": "test-sub-1",
        }
        logger.info(f"Subscribing to thread: {test_thread.id}")
        ws_client.send_json(sub_msg)

        # 2. Send test message
        message = {
            "type": MessageType.SEND_MESSAGE,
            "thread_id": test_thread.id,
            "content": "Test message",
            "message_id": "test-msg-1",
        }
        logger.info(f"Sending message: {message}")
        ws_client.send_json(message)

        # 3. Wait for response
        raw_response = ws_client.receive_json()
        logger.info(f"Received response: {raw_response}")
        response = raw_response.get("data", raw_response)

        # Check for error on raw envelope layer
        if raw_response["type"] == "error":
            logger.error(f"Error sending message: {raw_response}")
            assert False, f"Error sending message: {raw_response.get('error')}"

        assert response["type"] == MessageType.THREAD_MESSAGE
        assert "message" in response
        assert "content" in response["message"]

    def test_multiple_messages(self, ws_client, test_thread):
        """Test sending multiple messages in sequence"""
        # First subscribe to thread
        logger.info(f"Subscribing to thread: {test_thread.id}")
        ws_client.send_json(
            {
                "type": MessageType.SUBSCRIBE_THREAD,
                "thread_id": test_thread.id,
                "message_id": "test-sub-2",
            }
        )

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
            ws_client.send_json(message)

            # Verify response for each message
            raw = ws_client.receive_json()
            response = raw.get("data", raw)
            logger.info(f"Received response for message {idx+1}: {response}")

            # Check for error
            if response["type"] == "error":
                logger.error(f"Error response for message {idx+1}: {response}")
                assert False, f"Error sending message {idx+1}: {response.get('error')}"

            assert response["type"] == MessageType.THREAD_MESSAGE
            assert response["message"]["content"] == content
