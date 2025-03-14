"""
Test the streaming functionality for threads.

This module tests the streaming response endpoints for thread messages.
"""

from unittest.mock import MagicMock
from unittest.mock import patch

from fastapi import status
from fastapi.testclient import TestClient

from zerg.app.models.models import Thread


def test_create_thread_message_streaming(client: TestClient, sample_thread: Thread):
    """Test the POST /api/threads/{thread_id}/messages endpoint with streaming response"""

    # Message data to send
    message_data = {"role": "user", "content": "Hello, test assistant"}

    # Mock the agent_manager.process_message method to yield chunks
    with patch("zerg.app.routers.threads.AgentManager") as mock_agent_manager_class:
        # Configure the mock
        mock_agent_manager = MagicMock()
        mock_agent_manager_class.return_value = mock_agent_manager

        # Set up the process_message mock to yield chunks
        def mock_process_message(*args, **kwargs):
            yield "This "
            yield "is "
            yield "a "
            yield "test "
            yield "response"

        mock_agent_manager.process_message.return_value = mock_process_message()

        # Make the request
        response = client.post(f"/api/threads/{sample_thread.id}/messages", json=message_data)

        # Verify the response
        assert response.status_code == status.HTTP_200_OK
        assert response.headers["content-type"] == "text/plain; charset=utf-8"

        # Check the streaming response content
        content = response.content.decode("utf-8")
        assert content == "This is a test response"

        # Verify that process_message was called with the right arguments
        mock_agent_manager.process_message.assert_called_once()
        args, kwargs = mock_agent_manager.process_message.call_args
        assert kwargs["thread"].id == sample_thread.id
        assert kwargs["content"] == message_data["content"]
        assert kwargs["stream"] is True


def test_create_thread_message_not_found(client: TestClient):
    """Test the POST /api/threads/{thread_id}/messages endpoint with a non-existent ID"""
    message_data = {"role": "user", "content": "Hello, test assistant"}

    response = client.post("/api/threads/999/messages", json=message_data)
    assert response.status_code == status.HTTP_404_NOT_FOUND
    assert "detail" in response.json()
    assert response.json()["detail"] == "Thread not found"


def test_create_thread_message_error(client: TestClient, sample_thread: Thread):
    """Test error handling in the POST /api/threads/{thread_id}/messages endpoint"""
    message_data = {"role": "user", "content": "Hello, test assistant"}

    # Mock the agent_manager.process_message method to raise an exception
    with patch("zerg.app.routers.threads.AgentManager") as mock_agent_manager_class:
        # Configure the mock
        mock_agent_manager = MagicMock()
        mock_agent_manager_class.return_value = mock_agent_manager

        # Set up the process_message mock to raise an exception
        def mock_process_message(*args, **kwargs):
            raise Exception("Test error")

        mock_agent_manager.process_message.side_effect = mock_process_message

        # Make the request
        response = client.post(f"/api/threads/{sample_thread.id}/messages", json=message_data)

        # Verify the response
        assert response.status_code == status.HTTP_200_OK
        assert response.headers["content-type"] == "text/plain; charset=utf-8"

        # Check the response content contains the error
        content = response.content.decode("utf-8")
        assert "Error: Test error" in content
