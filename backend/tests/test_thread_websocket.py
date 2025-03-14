"""
Test WebSocket functionality for threads.

This module tests the WebSocket endpoints and event broadcasting
for real-time thread updates.
"""

import json
from unittest.mock import AsyncMock
from unittest.mock import MagicMock
from unittest.mock import patch

import pytest
from fastapi import WebSocket

from zerg.app.models.models import Agent
from zerg.app.models.models import Thread
from zerg.app.websocket import EventType
from zerg.app.websocket import broadcast_event


@pytest.fixture
def websocket_mock():
    """Create a mock WebSocket"""
    mock = MagicMock(spec=WebSocket)
    mock.accept = AsyncMock()
    mock.send_json = AsyncMock()
    mock.receive_json = AsyncMock()
    mock.receive_text = AsyncMock()
    mock.close = AsyncMock()
    return mock


# Test WebSocket broadcasting
@pytest.mark.asyncio
async def test_broadcast_event():
    """Test broadcasting an event to connected WebSockets"""
    with patch("zerg.app.websocket.connected_clients", [MagicMock()]) as mock_clients:
        # Get the mock WebSocket
        mock_ws = mock_clients[0]
        mock_ws.send_json = AsyncMock()

        # Test data
        event_type = EventType.CONVERSATION_CREATED
        event_data = {"thread_id": 1, "title": "Test Thread"}

        # Broadcast the event
        await broadcast_event(event_type, event_data)

        # Verify the WebSocket's send_json was called with the correct data
        mock_ws.send_json.assert_called_once()
        args, kwargs = mock_ws.send_json.call_args
        sent_data = args[0]
        assert sent_data["type"] == event_type
        assert sent_data == {"type": event_type, **event_data}


# Test WebSocket connection
@pytest.mark.asyncio
async def test_thread_websocket_connect(websocket_mock, sample_thread: Thread, db_session):
    """Test connecting to the thread WebSocket"""
    # Import here to avoid circular imports during testing
    from zerg.app.routers.threads import thread_websocket

    with patch("zerg.app.routers.threads.crud") as mock_crud:
        # Mock crud.get_thread to return the sample thread
        mock_crud.get_thread.return_value = sample_thread

        # Mock crud.get_agent to return the agent
        mock_agent = MagicMock(spec=Agent)
        mock_agent.id = sample_thread.agent_id
        mock_crud.get_agent.return_value = mock_agent

        # Mock crud.get_thread_messages to return some messages
        mock_messages = [
            MagicMock(
                id=1,
                role="system",
                content="You are a test assistant",
                timestamp=MagicMock(isoformat=lambda: "2023-01-01T00:00:00"),
            ),
            MagicMock(id=2, role="user", content="Hello", timestamp=MagicMock(isoformat=lambda: "2023-01-01T00:01:00")),
        ]
        mock_crud.get_thread_messages.return_value = mock_messages

        # Mock AgentManager
        with patch("zerg.app.routers.threads.AgentManager") as mock_agent_manager_class:
            mock_agent_manager = MagicMock()
            mock_agent_manager_class.return_value = mock_agent_manager

            # Configure the WebSocket mock to receive a ping and then disconnect
            websocket_mock.receive_text = AsyncMock()
            websocket_mock.receive_text.side_effect = [
                json.dumps({"type": "ping", "timestamp": 123456789}),
                Exception("Simulated disconnect"),
            ]

            # Call the WebSocket handler with our mocks
            try:
                await thread_websocket(websocket_mock, sample_thread.id, db_session)
            except Exception as e:
                # Expected exception from the simulated disconnect
                assert str(e) == "Simulated disconnect"

            # Verify the WebSocket was accepted
            websocket_mock.accept.assert_called_once()

            # Verify the thread history was sent
            history_call = None
            for call in websocket_mock.send_json.call_args_list:
                args, kwargs = call
                data = args[0]
                if data.get("type") == "thread_history":
                    history_call = data
                    break

            assert history_call is not None
            assert history_call["thread_id"] == sample_thread.id
            assert len(history_call["messages"]) == 2

            # Verify the pong response was sent
            pong_call = None
            for call in websocket_mock.send_json.call_args_list:
                args, kwargs = call
                data = args[0]
                if data.get("type") == "pong":
                    pong_call = data
                    break

            assert pong_call is not None
            assert pong_call["timestamp"] == 123456789


@pytest.mark.asyncio
async def test_thread_websocket_process_message(websocket_mock, sample_thread: Thread, db_session):
    """Test processing a message through the thread WebSocket"""
    # Import here to avoid circular imports during testing
    from zerg.app.routers.threads import thread_websocket

    with patch("zerg.app.routers.threads.crud") as mock_crud:
        # Mock crud.get_thread to return the sample thread
        mock_crud.get_thread.return_value = sample_thread

        # Mock crud.get_agent to return the agent
        mock_agent = MagicMock(spec=Agent)
        mock_agent.id = sample_thread.agent_id
        mock_crud.get_agent.return_value = mock_agent

        # Mock crud.get_thread_messages to return empty list
        mock_crud.get_thread_messages.return_value = []

        # Mock creating messages
        mock_user_message = MagicMock(id=1)
        mock_crud.create_thread_message.return_value = mock_user_message

        # Mock AgentManager
        with patch("zerg.app.routers.threads.AgentManager") as mock_agent_manager_class:
            mock_agent_manager = MagicMock()
            mock_agent_manager_class.return_value = mock_agent_manager

            # Set up the process_message mock to yield chunks
            def mock_process_message(*args, **kwargs):
                yield "First "
                yield "chunk "
                yield "of "
                yield "response"

            mock_agent_manager.process_message.return_value = mock_process_message()

            # Configure the WebSocket mock to receive a message and then disconnect
            websocket_mock.receive_text = AsyncMock()
            websocket_mock.receive_text.side_effect = [
                json.dumps({"type": "message", "content": "Hello, assistant"}),
                Exception("Simulated disconnect"),
            ]

            # Call the WebSocket handler with our mocks
            try:
                await thread_websocket(websocket_mock, sample_thread.id, db_session)
            except Exception as e:
                # Expected exception from the simulated disconnect
                assert str(e) == "Simulated disconnect"

            # Verify the agent manager's process_message was called
            mock_agent_manager.process_message.assert_called_once()
            args, kwargs = mock_agent_manager.process_message.call_args
            assert kwargs["db"] == db_session
            assert kwargs["thread"] == sample_thread
            assert kwargs["content"] == "Hello, assistant"
            assert kwargs["stream"] is True

            # Verify the messages were sent
            message_received_call = None
            stream_start_call = None
            stream_end_call = None
            chunk_calls = []

            for call in websocket_mock.send_json.call_args_list:
                args, kwargs = call
                data = args[0]

                if data.get("type") == "message_received":
                    message_received_call = data
                elif data.get("type") == "stream_start":
                    stream_start_call = data
                elif data.get("type") == "stream_end":
                    stream_end_call = data
                elif data.get("type") == "stream_chunk":
                    chunk_calls.append(data)

            # Verify all expected messages were sent
            assert message_received_call is not None
            assert stream_start_call is not None
            assert stream_end_call is not None
            assert len(chunk_calls) == 4

            # Verify the chunks
            assert chunk_calls[0]["content"] == "First "
            assert chunk_calls[1]["content"] == "chunk "
            assert chunk_calls[2]["content"] == "of "
            assert chunk_calls[3]["content"] == "response"


@pytest.mark.asyncio
async def test_thread_websocket_error_handling(websocket_mock, db_session):
    """Test error handling in the thread WebSocket"""
    # Import here to avoid circular imports during testing
    from zerg.app.routers.threads import thread_websocket

    with patch("zerg.app.routers.threads.crud") as mock_crud:
        # Mock crud.get_thread to return None (thread not found)
        mock_crud.get_thread.return_value = None

        # Call the WebSocket handler with our mocks
        await thread_websocket(websocket_mock, 999, db_session)

        # Verify the WebSocket was accepted
        websocket_mock.accept.assert_called_once()

        # Verify an error message was sent
        websocket_mock.send_json.assert_called_once()
        args, kwargs = websocket_mock.send_json.call_args
        data = args[0]
        assert "error" in data
        assert data["error"] == "Thread not found"

        # Verify the WebSocket was closed
        websocket_mock.close.assert_called_once()
