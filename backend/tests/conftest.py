import asyncio
import sys
from unittest.mock import MagicMock
from unittest.mock import patch

import dotenv
import pytest
from fastapi.testclient import TestClient
from sqlalchemy.pool import StaticPool

import zerg.database as _db_mod
import zerg.routers.websocket as _ws_router
from zerg.database import Base
from zerg.database import get_db
from zerg.database import make_engine
from zerg.database import make_sessionmaker
from zerg.events import EventType
from zerg.events import event_bus
from zerg.models.models import Agent
from zerg.models.models import AgentMessage
from zerg.models.models import Thread
from zerg.models.models import ThreadMessage
from zerg.services.scheduler_service import scheduler_service
from zerg.websocket.manager import topic_manager

dotenv.load_dotenv()


# Create a test database - using in-memory SQLite for tests
SQLALCHEMY_DATABASE_URL = "sqlite:///:memory:"

# Create test engine and session factory
test_engine = make_engine(
    SQLALCHEMY_DATABASE_URL,
    connect_args={"check_same_thread": False},
    poolclass=StaticPool,  # Use StaticPool for in-memory database
)

TestingSessionLocal = make_sessionmaker(test_engine)

# Override default_session_factory to use test sessions for WebSocket
_db_mod.default_session_factory = TestingSessionLocal
_ws_router.default_session_factory = TestingSessionLocal

# Mock the OpenAI module before importing main app
mock_openai = MagicMock()
mock_client = MagicMock()
mock_chat = MagicMock()
mock_completions = MagicMock()

# Configure the mock to return a string for the content
mock_message = MagicMock()
mock_message.content = "This is a mock response from the LLM"
mock_choice = MagicMock()
mock_choice.message = mock_message
mock_choices = [mock_choice]
mock_response = MagicMock()
mock_response.choices = mock_choices

mock_completions.create.return_value = mock_response
mock_chat.completions = mock_completions
mock_client.chat = mock_chat
mock_openai.return_value = mock_client

sys.modules["openai"] = MagicMock()
sys.modules["openai.OpenAI"] = mock_openai

# Don't completely mock langgraph, just patch specific functionality
# This allows importing langgraph and accessing attributes like __version__
# while still mocking functionality used in tests
# First import the real modules to preserve their behavior
import langchain_openai  # noqa: E402
import langgraph  # noqa: E402
import langgraph.graph  # noqa: E402
import langgraph.graph.message  # noqa: E402

# Then patch specific classes or functions rather than entire modules
langgraph.graph.StateGraph = MagicMock()
langgraph.func = MagicMock()
langgraph.graph.message.add_messages = MagicMock()
langchain_openai.ChatOpenAI = MagicMock()

# Import app after all engine setup and mocks are in place
from zerg.main import app  # noqa: E402


@pytest.fixture(scope="session", autouse=True)
def cleanup_global_resources(request):
    """
    Ensure global resources like topic_manager are cleaned up after the session.
    This is crucial because topic_manager subscribes to event_bus at import time.
    """
    yield  # Run all tests

    # Teardown logic after all tests in the session have run
    print("\nPerforming session cleanup...")

    # 1. Clear topic_manager state
    #    Resetting internal dicts to break potential reference cycles
    #    and ensure no lingering client data.
    topic_manager.active_connections.clear()
    topic_manager.topic_subscriptions.clear()
    topic_manager.client_topics.clear()
    print("Cleared topic_manager state.")

    # 2. Unsubscribe topic_manager handlers from event_bus
    #    This is important to prevent errors if event_bus tries to call
    #    handlers on a potentially partially garbage-collected topic_manager.
    #    Assuming event_bus.unsubscribe is synchronous.
    try:
        event_bus.unsubscribe(EventType.AGENT_CREATED, topic_manager._handle_agent_event)
        event_bus.unsubscribe(EventType.AGENT_UPDATED, topic_manager._handle_agent_event)
        event_bus.unsubscribe(EventType.AGENT_DELETED, topic_manager._handle_agent_event)
        event_bus.unsubscribe(EventType.THREAD_CREATED, topic_manager._handle_thread_event)
        event_bus.unsubscribe(EventType.THREAD_UPDATED, topic_manager._handle_thread_event)
        event_bus.unsubscribe(EventType.THREAD_DELETED, topic_manager._handle_thread_event)
        event_bus.unsubscribe(EventType.THREAD_MESSAGE_CREATED, topic_manager._handle_thread_event)
        print("Unsubscribed topic_manager from event_bus.")
    except Exception as e:
        print(f"Error during topic_manager unsubscribe: {e}")

    # 3. Explicitly stop the scheduler service
    try:
        # Need to run the async stop method
        async def _stop_scheduler():
            await scheduler_service.stop()

        # Use asyncio.run ONLY if no other loop is running (which should be true at session end)
        # If this causes issues, might need pytest-asyncio's loop access.
        if scheduler_service._initialized:
            asyncio.run(_stop_scheduler())
            print("Stopped scheduler service.")
        else:
            print("Scheduler service was not initialized, skipping stop.")
    except Exception as e:
        print(f"Error stopping scheduler service during cleanup: {e}")

    # 4. Optionally, clear event_bus subscribers if necessary (use with caution)
    # event_bus._subscribers.clear()
    # print("Cleared event_bus subscribers.")

    print("Session cleanup complete.")


@pytest.fixture
def db_session():
    """
    Creates a fresh database for each test, then tears it down after the test is done.
    """
    # Create the tables
    Base.metadata.create_all(bind=test_engine)

    # Create a session
    db = TestingSessionLocal()
    try:
        yield db
    finally:
        db.close()
        # Drop all tables after the test
        Base.metadata.drop_all(bind=test_engine)


@pytest.fixture
def client(db_session):
    """
    Create a FastAPI TestClient with the test database dependency.
    """

    def override_get_db():
        try:
            yield db_session
        finally:
            pass

    app.dependency_overrides[get_db] = override_get_db

    client = TestClient(app, backend="asyncio")
    yield client

    app.dependency_overrides = {}


@pytest.fixture
def test_client(db_session):
    """
    Create a FastAPI TestClient with WebSocket support
    """

    def override_get_db():
        try:
            yield db_session
        finally:
            pass

    app.dependency_overrides[get_db] = override_get_db

    client = TestClient(app, backend="asyncio")
    yield client

    app.dependency_overrides = {}


@pytest.fixture
def test_session_factory(db_session):
    """
    Returns a session factory using the test database.
    Used for cases where a service requires a session factory.
    Ensures all database operations in a test use the same connection.
    """

    def get_test_session():
        return db_session

    return get_test_session


@pytest.fixture
def sample_agent(db_session):
    """
    Create a sample agent in the database
    """
    agent = Agent(
        name="Test Agent",
        system_instructions="System instructions for test agent",
        task_instructions="This is a test agent",
        model="gpt-4o",
        status="idle",
    )
    db_session.add(agent)
    db_session.commit()
    db_session.refresh(agent)
    return agent


@pytest.fixture
def sample_messages(db_session, sample_agent):
    """
    Create sample messages for the sample agent
    """
    messages = [
        AgentMessage(agent_id=sample_agent.id, role="system", content="You are a test assistant"),
        AgentMessage(agent_id=sample_agent.id, role="user", content="Hello, test assistant"),
        AgentMessage(agent_id=sample_agent.id, role="assistant", content="Hello, I'm the test assistant"),
    ]

    for message in messages:
        db_session.add(message)

    db_session.commit()

    return messages


@pytest.fixture
def sample_thread(db_session, sample_agent):
    """
    Create a sample thread in the database
    """
    thread = Thread(
        agent_id=sample_agent.id,
        title="Test Thread",
        active=True,
        agent_state={"test_key": "test_value"},
        memory_strategy="buffer",
    )
    db_session.add(thread)
    db_session.commit()
    db_session.refresh(thread)
    return thread


@pytest.fixture
def sample_thread_messages(db_session, sample_thread):
    """
    Create sample messages for the sample thread
    """
    messages = [
        ThreadMessage(
            thread_id=sample_thread.id,
            role="system",
            content="You are a test assistant",
        ),
        ThreadMessage(
            thread_id=sample_thread.id,
            role="user",
            content="Hello, test assistant",
        ),
        ThreadMessage(
            thread_id=sample_thread.id,
            role="assistant",
            content="Hello, I'm the test assistant",
        ),
    ]

    for message in messages:
        db_session.add(message)

    db_session.commit()
    return messages


@pytest.fixture
def mock_langgraph_state_graph():
    """
    Mock the StateGraph class from LangGraph
    """
    with patch("zerg.agents.StateGraph") as mock_state_graph:
        # Create a mock graph
        mock_graph = MagicMock()
        mock_state_graph.return_value = mock_graph

        # Mock the compile method to return a graph instance
        compiled_graph = MagicMock()
        mock_graph.compile.return_value = compiled_graph

        yield mock_state_graph


@pytest.fixture
def mock_langchain_openai():
    """
    Mock the LangChain OpenAI integration
    """
    with patch("langchain_openai.ChatOpenAI") as mock_chat_openai:
        mock_chat = MagicMock()
        mock_chat_openai.return_value = mock_chat
        yield mock_chat_openai
