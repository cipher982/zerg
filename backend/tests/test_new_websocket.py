"""Tests for the new WebSocket implementation."""

import pytest
from fastapi.testclient import TestClient
from starlette.testclient import WebSocketTestSession

from zerg.app.events import EventType
from zerg.app.events import event_bus
from zerg.app.models import Agent
from zerg.app.models import Thread
from zerg.app.websocket.new_manager import TopicConnectionManager
from zerg.main import app  # Import the FastAPI app from correct location


@pytest.fixture
def topic_manager():
    """Create a fresh topic manager for each test."""
    return TopicConnectionManager()


@pytest.fixture
def ws_client_v2(test_client: TestClient) -> WebSocketTestSession:
    """Create a WebSocket test client for v2 endpoint."""
    with test_client.websocket_connect("/api/ws/v2") as websocket:
        yield websocket


@pytest.mark.asyncio
class TestTopicBasedWebSocket:
    """Test suite for topic-based WebSocket functionality."""

    def test_basic_connection(self, ws_client_v2: WebSocketTestSession):
        """Test basic WebSocket connection and ping/pong."""
        # Send a ping
        ws_client_v2.send_json({"type": "ping", "timestamp": 123456789, "message_id": "test-ping-1"})

        # Expect pong response
        response = ws_client_v2.receive_json()
        assert response["type"] == "pong"
        assert "timestamp" in response

    async def test_subscribe_to_thread(self, ws_client_v2, sample_thread: Thread):
        """Test subscribing to a thread topic."""
        # Subscribe to thread
        ws_client_v2.send_json(
            {"type": "subscribe", "topics": [f"thread:{sample_thread.id}"], "message_id": "test-sub-1"}
        )

        # Should receive thread history
        response = ws_client_v2.receive_json()
        assert response["type"] == "thread_history"
        assert response["thread_id"] == sample_thread.id
        assert "messages" in response

        # Publish a thread event
        await event_bus.publish(
            EventType.THREAD_MESSAGE_CREATED,
            {"thread_id": sample_thread.id, "message": {"content": "Test message", "role": "user"}},
        )

        # Should receive the event
        response = ws_client_v2.receive_json()
        assert response["type"] == "thread_event"
        assert response["data"]["thread_id"] == sample_thread.id

    async def test_subscribe_to_agent(self, ws_client_v2, sample_agent: Agent):
        """Test subscribing to an agent topic."""
        # Subscribe to agent
        ws_client_v2.send_json(
            {"type": "subscribe", "topics": [f"agent:{sample_agent.id}"], "message_id": "test-sub-2"}
        )

        # Should receive current agent state
        response = ws_client_v2.receive_json()
        assert response["type"] == "agent_state"
        assert response["data"]["id"] == sample_agent.id
        assert response["data"]["name"] == sample_agent.name

        # Publish an agent event
        await event_bus.publish(
            EventType.AGENT_UPDATED, {"id": sample_agent.id, "name": sample_agent.name, "status": "processing"}
        )

        # Should receive the event
        response = ws_client_v2.receive_json()
        assert response["type"] == "agent_event"
        assert response["data"]["id"] == sample_agent.id
        assert response["data"]["status"] == "processing"

    async def test_subscribe_to_multiple_topics(self, ws_client_v2, sample_thread: Thread, sample_agent: Agent):
        """Test subscribing to multiple topics in one request."""
        # Subscribe to both thread and agent
        ws_client_v2.send_json(
            {
                "type": "subscribe",
                "topics": [f"thread:{sample_thread.id}", f"agent:{sample_agent.id}"],
                "message_id": "test-sub-3",
            }
        )

        # Should receive thread history and agent state
        responses = [ws_client_v2.receive_json(), ws_client_v2.receive_json()]

        # Verify we got both types of responses (order may vary)
        response_types = {r["type"] for r in responses}
        assert "thread_history" in response_types
        assert "agent_state" in response_types

    async def test_unsubscribe(
        self, ws_client_v2: WebSocketTestSession, sample_thread: Thread, topic_manager: TopicConnectionManager
    ):
        """Test unsubscribing from a topic."""
        # First subscribe
        ws_client_v2.send_json(
            {"type": "subscribe", "topics": [f"thread:{sample_thread.id}"], "message_id": "test-sub-4"}
        )
        ws_client_v2.receive_json()  # Consume history response

        # Then unsubscribe
        ws_client_v2.send_json(
            {"type": "unsubscribe", "topics": [f"thread:{sample_thread.id}"], "message_id": "test-unsub-1"}
        )

        # Wait for unsubscribe confirmation
        response = ws_client_v2.receive_json()
        assert response["type"] == "unsubscribe_success"

        # Verify the topic subscription was removed
        topic = f"thread:{sample_thread.id}"
        assert (
            topic not in topic_manager.topic_subscriptions or not topic_manager.topic_subscriptions[topic]
        ), "Topic should have no subscribers after unsubscribe"

    async def test_invalid_topic_format(self, ws_client_v2):
        """Test handling of invalid topic formats."""
        # Try to subscribe to invalid topic
        ws_client_v2.send_json(
            {"type": "subscribe", "topics": ["invalid:123", "also:invalid"], "message_id": "test-invalid-1"}
        )

        # Should receive error for each invalid topic
        response = ws_client_v2.receive_json()
        assert response["type"] == "error"
        assert "Invalid topic format" in response["error"]

    async def test_nonexistent_resources(self, ws_client_v2):
        """Test subscribing to non-existent thread/agent."""
        # Try to subscribe to non-existent resources
        ws_client_v2.send_json(
            {"type": "subscribe", "topics": ["thread:99999", "agent:99999"], "message_id": "test-invalid-2"}
        )

        # Should receive error messages
        response1 = ws_client_v2.receive_json()
        response2 = ws_client_v2.receive_json()

        assert response1["type"] == "error"
        assert response2["type"] == "error"
        assert any("Thread 99999 not found" in r["error"] for r in [response1, response2])
        assert any("Agent 99999 not found" in r["error"] for r in [response1, response2])

    def test_initial_topics_parameter(self, sample_thread: Thread):
        """Test connecting with initial topics parameter."""
        # Create a client with initial topics
        with TestClient(app).websocket_connect(f"/api/ws/v2?initial_topics=thread:{sample_thread.id}") as websocket:
            # Should automatically receive thread history
            response = websocket.receive_json()
            assert response["type"] == "thread_history"
            assert response["thread_id"] == sample_thread.id
