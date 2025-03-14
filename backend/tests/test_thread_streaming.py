"""
Test the streaming functionality for threads.

This module tests the streaming response endpoints for thread messages.
"""

from unittest.mock import MagicMock
from unittest.mock import patch

from fastapi import status
from fastapi.testclient import TestClient

from zerg.app.models.models import Thread
from zerg.app.models.models import ThreadMessage


def test_run_thread_streaming(client: TestClient, sample_thread: Thread, db_session):
    """Test the POST /api/threads/{thread_id}/run endpoint with streaming response"""
    # Create an unprocessed message
    message = ThreadMessage(thread_id=sample_thread.id, role="user", content="Hello, test assistant", processed=False)
    db_session.add(message)
    db_session.commit()

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
        response = client.post(f"/api/threads/{sample_thread.id}/run")

        # Verify the response
        assert response.status_code == status.HTTP_200_OK
        assert response.headers["content-type"] == "text/plain; charset=utf-8"

        # Check the streaming response content
        content = response.content.decode("utf-8")
        assert content == "This is a test response"

        # Verify that process_message was called with the right arguments
        mock_agent_manager.process_message.assert_called_once()
        args, kwargs = mock_agent_manager.process_message.call_args
        assert kwargs["db"] is not None
        assert kwargs["thread"] == sample_thread
        assert kwargs["content"] is None  # When running unprocessed messages, content is None
        assert kwargs["stream"] is True


def test_run_thread_no_unprocessed_messages(client: TestClient, sample_thread: Thread):
    """Test the POST /api/threads/{thread_id}/run endpoint with no unprocessed messages"""
    response = client.post(f"/api/threads/{sample_thread.id}/run")
    assert response.status_code == status.HTTP_200_OK
    assert response.json() == {"detail": "No unprocessed messages to run"}


def test_run_thread_not_found(client: TestClient):
    """Test the POST /api/threads/{thread_id}/run endpoint with a non-existent ID"""
    response = client.post("/api/threads/999/run")
    assert response.status_code == status.HTTP_404_NOT_FOUND
    assert "detail" in response.json()
    assert response.json()["detail"] == "Thread not found"


def test_run_thread_error(client: TestClient, sample_thread: Thread, db_session):
    """Test error handling in the POST /api/threads/{thread_id}/run endpoint"""
    # Create an unprocessed message
    message = ThreadMessage(thread_id=sample_thread.id, role="user", content="Hello, test assistant", processed=False)
    db_session.add(message)
    db_session.commit()

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
        response = client.post(f"/api/threads/{sample_thread.id}/run")

        # Verify the response
        assert response.status_code == status.HTTP_200_OK
        assert response.headers["content-type"] == "text/plain; charset=utf-8"

        # Check the response content contains the error
        content = response.content.decode("utf-8")
        assert "Error: Test error" in content
