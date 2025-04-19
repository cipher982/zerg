import sys
from unittest.mock import MagicMock
from unittest.mock import patch

import dotenv
import pytest
from fastapi.testclient import TestClient
from sqlalchemy.pool import StaticPool

import zerg.app.database as _db_mod
import zerg.app.routers.websocket as _ws_router
from zerg.app.database import Base
from zerg.app.database import get_db
from zerg.app.database import make_engine
from zerg.app.database import make_sessionmaker
from zerg.app.models.models import Agent
from zerg.app.models.models import AgentMessage
from zerg.app.models.models import Thread
from zerg.app.models.models import ThreadMessage

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

# Mock LangGraph modules
sys.modules["langgraph"] = MagicMock()
sys.modules["langgraph.graph"] = MagicMock()
sys.modules["langgraph.graph.message"] = MagicMock()
sys.modules["langchain_openai"] = MagicMock()

# Import app after all engine setup and mocks are in place
from zerg.main import app  # noqa: E402


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
def test_session_factory():
    """
    Returns a session factory using the test database.
    Used for cases where a service requires a session factory.
    """

    def get_test_session():
        db = TestingSessionLocal()
        try:
            yield db
        finally:
            db.close()

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
    with patch("zerg.app.agents.StateGraph") as mock_state_graph:
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
