"""
Test the AgentManager class.

This module contains tests for the AgentManager class, which handles
all LangGraph-based agent interactions and state management.
"""

from unittest.mock import ANY
from unittest.mock import MagicMock
from unittest.mock import patch

import pytest

from zerg.app.agents import AgentManager
from zerg.app.models.models import Agent
from zerg.app.models.models import Thread
from zerg.app.models.models import ThreadMessage


@pytest.fixture
def mock_llm():
    """Create a mock LLM for testing"""
    mock = MagicMock()
    # Mock the invoke method to return a response with content
    response = MagicMock()
    response.content = "This is a test response"
    mock.invoke.return_value = response
    return mock


@pytest.fixture
def agent_manager(sample_agent: Agent, mock_llm):
    """Create an AgentManager instance with a mock LLM"""
    with patch("zerg.app.agents.ChatOpenAI", return_value=mock_llm):
        agent_manager = AgentManager(sample_agent)
        return agent_manager


def test_agent_manager_init(sample_agent: Agent):
    """Test initializing an AgentManager instance"""
    with patch("zerg.app.agents.ChatOpenAI") as mock_chat_openai:
        agent_manager = AgentManager(sample_agent)
        assert agent_manager.agent_model == sample_agent
        # Check that the LLM was initialized with the correct model
        mock_chat_openai.assert_called_once()
        args, kwargs = mock_chat_openai.call_args
        assert kwargs["model"] == sample_agent.model


def test_build_graph(agent_manager: AgentManager):
    """Test building a LangGraph state machine"""
    with patch("zerg.app.agents.StateGraph") as mock_state_graph:
        # Setup the mock graph builder: get the builder mock from return_value
        mock_builder = mock_state_graph.return_value

        # Call the method
        agent_manager._build_graph()

        # DEBUG: Print the arguments add_edge was called with
        print(f"DEBUG: mock_builder.add_edge called with: {mock_builder.add_edge.call_args_list}")

        # Verify that the chatbot node was added
        mock_builder.add_node.assert_any_call("chatbot", ANY)
        # Verify that the edges were added
        mock_builder.add_edge.assert_any_call("__start__", "chatbot")
        mock_builder.add_edge.assert_any_call("chatbot", "__end__")
        # Verify that compile was called
        mock_builder.compile.assert_called_once()


def test_chatbot_node(agent_manager: AgentManager, mock_llm):
    """Test the chatbot node function"""
    # Create a test state
    state = {
        "messages": [{"role": "user", "content": "Hello, test assistant"}],
        "metadata": {"agent_id": 1, "thread_id": 1},
    }

    # Call the node function
    result = agent_manager._chatbot_node(state)

    # Verify the result
    assert "messages" in result
    assert len(result["messages"]) == 1

    # Verify the LLM was called with the expected arguments
    mock_llm.invoke.assert_called_once()
    args, kwargs = mock_llm.invoke.call_args
    assert args[0] == state["messages"]


def test_get_or_create_thread_new(db_session, agent_manager: AgentManager):
    """Test creating a new thread"""
    with patch("zerg.app.crud.crud") as mock_crud:
        # Mock crud.get_thread to return None (no existing thread)
        mock_crud.get_thread.return_value = None
        # Mock crud.get_active_thread to return None (no active thread)
        mock_crud.get_active_thread.return_value = None

        # Mock creating a new thread
        mock_thread = MagicMock(spec=Thread)
        mock_thread.id = 1
        mock_thread.agent_id = agent_manager.agent_model.id
        mock_crud.create_thread.return_value = mock_thread

        # Call the method to create a new thread
        thread, created = agent_manager.get_or_create_thread(db_session, title="Test Thread")

        # Verify the right methods were called
        mock_crud.get_active_thread.assert_called_once_with(db_session, agent_manager.agent_model.id)
        mock_crud.create_thread.assert_called_once()
        args, kwargs = mock_crud.create_thread.call_args
        assert kwargs["db"] == db_session
        assert kwargs["agent_id"] == agent_manager.agent_model.id
        assert kwargs["title"] == "Test Thread"
        assert kwargs["active"] is True

        # Verify the results
        assert thread == mock_thread
        assert created is True


def test_get_or_create_thread_existing(db_session, agent_manager: AgentManager):
    """Test getting an existing thread"""
    with patch("zerg.app.crud.crud") as mock_crud:
        # Create a mock thread
        mock_thread = MagicMock(spec=Thread)
        mock_thread.id = 1
        mock_thread.agent_id = agent_manager.agent_model.id

        # Mock crud.get_thread to return the mock thread
        mock_crud.get_thread.return_value = mock_thread

        # Call the method to get an existing thread
        thread, created = agent_manager.get_or_create_thread(db_session, thread_id=1)

        # Verify the right methods were called
        mock_crud.get_thread.assert_called_once_with(db_session, 1)

        # Verify the results
        assert thread == mock_thread
        assert created is False


def test_add_system_message(db_session, agent_manager: AgentManager):
    """Test adding a system message to a thread"""
    with patch("zerg.app.crud.crud") as mock_crud:
        # Create a mock thread
        mock_thread = MagicMock(spec=Thread)
        mock_thread.id = 1

        # Mock crud.get_thread_messages to return an empty list (no messages yet)
        mock_crud.get_thread_messages.return_value = []

        # Call the method to add a system message
        agent_manager.add_system_message(db_session, mock_thread)

        # Verify the right methods were called
        mock_crud.get_thread_messages.assert_called_once_with(db_session, mock_thread.id)
        mock_crud.create_thread_message.assert_called_once()
        args, kwargs = mock_crud.create_thread_message.call_args
        assert kwargs["db"] == db_session
        assert kwargs["thread_id"] == mock_thread.id
        assert kwargs["role"] == "system"
        assert kwargs["content"] == agent_manager.agent_model.system_instructions


def test_add_system_message_with_existing_messages(db_session, agent_manager: AgentManager):
    """Test not adding a system message when messages already exist"""
    with patch("zerg.app.crud.crud") as mock_crud:
        # Create a mock thread
        mock_thread = MagicMock(spec=Thread)
        mock_thread.id = 1

        # Mock crud.get_thread_messages to return a list with a message (messages already exist)
        mock_message = MagicMock(spec=ThreadMessage)
        mock_crud.get_thread_messages.return_value = [mock_message]

        # Call the method to add a system message
        agent_manager.add_system_message(db_session, mock_thread)

        # Verify the right methods were called
        mock_crud.get_thread_messages.assert_called_once_with(db_session, mock_thread.id)
        # Verify that create_thread_message was not called
        mock_crud.create_thread_message.assert_not_called()


def test_process_message(db_session, agent_manager: AgentManager, mock_llm):
    """Test processing a message through the LangGraph agent"""
    with patch("zerg.app.crud.crud") as mock_crud, patch.object(agent_manager, "_build_graph") as mock_build_graph:
        # Create mock objects
        mock_thread = MagicMock(spec=Thread)
        mock_thread.id = 1

        # Mock crud.create_thread_message with side effect to return different messages
        mock_user_message = MagicMock(spec=ThreadMessage)
        mock_assistant_message = MagicMock(spec=ThreadMessage)
        mock_crud.create_thread_message.side_effect = [mock_user_message, mock_assistant_message]

        # Mock crud.get_thread_messages to return a list of messages
        mock_messages = [
            MagicMock(spec=ThreadMessage, role="system", content="You are a test assistant", tool_calls=None),
            MagicMock(spec=ThreadMessage, role="user", content="Hello", tool_calls=None),
        ]
        mock_crud.get_thread_messages.return_value = mock_messages

        # Set up the mock graph for non-streaming mode
        mock_graph = MagicMock()
        mock_response = MagicMock()
        mock_response.content = "This is a test response"
        mock_graph.invoke.return_value = {"messages": [mock_response]}
        mock_build_graph.return_value = mock_graph

        # Call the method to process a message (non-streaming)
        response_generator = agent_manager.process_message(
            db=db_session, thread=mock_thread, content="Test message", stream=False
        )

        # Consume the generator to get the response
        response = next(response_generator)

        # Verify the LangGraph processing
        assert response == "This is a test response"

        # Verify the first create_thread_message call (user message)
        assert mock_crud.create_thread_message.call_count >= 1
        first_call_kwargs = mock_crud.create_thread_message.call_args_list[0][1]
        assert first_call_kwargs["db"] == db_session
        assert first_call_kwargs["thread_id"] == mock_thread.id
        assert first_call_kwargs["role"] == "user"
        assert first_call_kwargs["content"] == "Test message"

        # Run the generator to completion to ensure all side effects happen
        try:
            while True:
                next(response_generator)
        except StopIteration:
            pass

        # Now verify the assistant message was created
        assert mock_crud.create_thread_message.call_count >= 2
        second_call_kwargs = mock_crud.create_thread_message.call_args_list[1][1]
        assert second_call_kwargs["db"] == db_session
        assert second_call_kwargs["thread_id"] == mock_thread.id
        assert second_call_kwargs["role"] == "assistant"
        assert second_call_kwargs["content"] == "This is a test response"

        # Verify the thread was updated
        mock_crud.update_thread.assert_called_once_with(db_session, mock_thread.id)


def test_process_message_with_tool_calls(db_session, agent_manager: AgentManager):
    """Test processing a message with tool calls"""
    with patch("zerg.app.crud.crud") as mock_crud, patch.object(agent_manager, "_build_graph") as mock_build_graph:
        # Create mock objects
        mock_thread = MagicMock(spec=Thread)
        mock_thread.id = 1

        # Mock crud.create_thread_message to return different messages on each call
        mock_user_message = MagicMock(spec=ThreadMessage)
        mock_assistant_message = MagicMock(spec=ThreadMessage)
        mock_crud.create_thread_message.side_effect = [mock_user_message, mock_assistant_message]

        # Mock crud.get_thread_messages to return messages with tool calls
        mock_system_message = MagicMock(spec=ThreadMessage)
        mock_system_message.role = "system"
        mock_system_message.content = "You are a test assistant"
        mock_system_message.tool_calls = None

        mock_user_message = MagicMock(spec=ThreadMessage)
        mock_user_message.role = "user"
        mock_user_message.content = "What's the weather?"
        mock_user_message.tool_calls = None

        mock_assistant_message = MagicMock(spec=ThreadMessage)
        mock_assistant_message.role = "assistant"
        mock_assistant_message.content = ""
        mock_assistant_message.tool_calls = [{"type": "function", "function": {"name": "get_weather"}}]
        mock_assistant_message.name = None
        mock_assistant_message.tool_call_id = None

        mock_crud.get_thread_messages.return_value = [mock_system_message, mock_user_message, mock_assistant_message]

        # Set up the mock graph stream
        mock_graph = MagicMock()
        mock_response = MagicMock()
        mock_response.content = "Weather response"
        # In streaming mode, return a message that can be processed
        mock_graph.stream.return_value = [{"chatbot": {"messages": [mock_response]}}]
        mock_build_graph.return_value = mock_graph

        # Call the method to process a message
        response_generator = agent_manager.process_message(
            db=db_session, thread=mock_thread, content="What's the temperature?", stream=True
        )

        # Consume the generator to get the response
        response = next(response_generator)

        # Verify the right methods were called - initially called for user message
        assert mock_crud.create_thread_message.call_count >= 1
        first_call_kwargs = mock_crud.create_thread_message.call_args_list[0][1]
        assert first_call_kwargs["db"] == db_session
        assert first_call_kwargs["thread_id"] == mock_thread.id
        assert first_call_kwargs["role"] == "user"
        assert first_call_kwargs["content"] == "What's the temperature?"

        # Verify the response
        assert response == "Weather response"

        # Run the generator to completion to ensure all side effects happen
        try:
            while True:
                next(response_generator)
        except StopIteration:
            pass

        # Now verify the assistant message was created
        assert mock_crud.create_thread_message.call_count >= 2
        second_call_kwargs = mock_crud.create_thread_message.call_args_list[1][1]
        assert second_call_kwargs["db"] == db_session
        assert second_call_kwargs["thread_id"] == mock_thread.id
        assert second_call_kwargs["role"] == "assistant"
        assert second_call_kwargs["content"] == "Weather response"
