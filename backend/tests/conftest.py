import os

# Set *before* any project imports so backend skips background services
os.environ["TESTING"] = "1"

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

# Disable LangSmith tracing for all tests
os.environ["LANGCHAIN_TRACING_V2"] = "false"
os.environ["LANGCHAIN_ENDPOINT"] = ""

# ---------------------------------------------------------------------------
# Stub *cryptography* so zerg.utils.crypto can import Fernet without the real
# wheel present.  We only need the API surface used in tests: ``encrypt`` and
# ``decrypt`` should round-trip the plaintext.
# ---------------------------------------------------------------------------

if "cryptography" not in sys.modules:  # guard in case real package installed
    import types as _types

    _crypto_mod = _types.ModuleType("cryptography")
    _fernet_mod = _types.ModuleType("cryptography.fernet")

    class _FakeFernet:  # noqa: D401 – minimal stub
        def __init__(self, _key):
            self._key = _key

        # Keep encryption a **no-op** inside the unit-test environment so
        # assertions that inspect raw values (e.g. auth tokens) still match.
        def encrypt(self, data: bytes):  # noqa: D401 – mimic API
            return data  # type: ignore[return-value]

        def decrypt(self, token: bytes):  # noqa: D401
            return token

    _fernet_mod.Fernet = _FakeFernet  # type: ignore[attr-defined]
    sys.modules["cryptography"] = _crypto_mod
    sys.modules["cryptography.fernet"] = _fernet_mod

os.environ["LANGCHAIN_API_KEY"] = ""

# ---------------------------------------------------------------------------
# Crypto – provide deterministic Fernet key for the test suite so refresh
# tokens are encrypted at rest while keeping decryption reproducible.
# ---------------------------------------------------------------------------
# Key generated via ``cryptography.fernet.Fernet.generate_key()`` once and
# hard-coded here.  The value is **public** and only used in CI/dev tests –
# production deployments must override via environment variable.

os.environ.setdefault(
    "FERNET_SECRET",
    "Mj7MFJspDPjiFBGHZJ5hnx70XAFJ_En6ofIEhn3BoXw=",
)

# Mock the LangSmith client to prevent any actual API calls
mock_langsmith = MagicMock()
mock_langsmith_client = MagicMock()
mock_langsmith.Client.return_value = mock_langsmith_client
sys.modules["langsmith"] = mock_langsmith
sys.modules["langsmith.client"] = MagicMock()

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
# Ensure websocket router uses the in-memory DB
_ws_router.get_session_factory = lambda: TestingSessionLocal  # type: ignore[attr-defined]
_db_mod.default_engine = test_engine
# Ensure helper *get_session_factory()* returns the test sessionmaker so any
# late-resolving imports (e.g. GmailProvider) use the in-memory SQLite
# connection rather than creating a brand new on-disk engine.

_db_mod.get_session_factory = lambda: TestingSessionLocal  # type: ignore[assignment]
# Ensure *EmailTriggerService* uses the same in-memory session factory
try:
    from zerg.services.email_trigger_service import email_trigger_service as _ets  # noqa: WPS433 – test-time import

    _ets._session_factory = TestingSessionLocal  # type: ignore[attr-defined]
except ImportError:
    # In some collector runs the service may not be imported yet; tests that
    # need it will import later and patch manually via the fixture.
    pass

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
# The real modules are imported so we can selectively patch.
import langchain_openai  # noqa: E402
import langgraph  # noqa: E402
import langgraph.graph  # noqa: E402
import langgraph.graph.message  # noqa: E402

# ---------------------------------------------------------------------------
# Async-friendly stub for ChatOpenAI
# ---------------------------------------------------------------------------
from langchain_core.messages import AIMessage  # noqa: E402


class _StubLlm:
    """Stub LLM that returns deterministic response for both sync and async APIs."""

    def invoke(self, _messages):  # noqa: D401 – sync path used in production
        return AIMessage(content="stub-response")

    # Async variants retained for completeness (never awaited in current tests)
    async def ainvoke(self, _messages):  # noqa: D401 – preferred async method
        return AIMessage(content="stub-response")

    async def invoke_async(self, _messages):  # noqa: D401 – legacy async method
        return AIMessage(content="stub-response")


class _StubChatOpenAI:
    """Replacement for ChatOpenAI constructor used in tests."""

    def __init__(self, *args, **kwargs):  # noqa: D401 – signature irrelevant
        pass

    def bind_tools(self, _tools):  # noqa: D401
        return _StubLlm()


# Then patch specific classes or functions rather than entire modules
# ----------------------------------------------------------------------
# LangGraph / LLM stubs
# ----------------------------------------------------------------------


class _FakeRunnable:
    """Minimal runnable that returns a deterministic assistant reply."""

    def invoke(self, _state):  # noqa: D401 – interface mimic
        return {"messages": [AIMessage(content="stub-response")]}


class _FakeStateGraph(MagicMock):
    """Replacement for ``langgraph.graph.StateGraph`` used in tests.

    We inherit from MagicMock so existing attribute access patterns in the
    agent definition code still work (``add_node``, ``add_edge``, etc.), but
    we override ``compile`` to return our deterministic runnable instead of a
    plain MagicMock.  This removes the need for *any* fallback logic in
    production code.
    """

    def compile(self):  # noqa: D401 – mimic real API
        return _FakeRunnable()


# Patch the modules
# We now run the *real* LangGraph StateGraph implementation so the agent
# definition code executes unmodified.  We only patch *ChatOpenAI* because it
# performs an external network call.  ``add_messages`` is a pure helper; no
# need to mock it.

# Remove previous mocks if they were set by earlier lines in this file.
try:
    import importlib

    # Re-import the real sub-module to ensure we restored original symbols.
    importlib.reload(langgraph.graph)
    importlib.reload(langgraph.func)
    importlib.reload(langgraph.graph.message)
except Exception:  # pragma: no cover – defensive; tests still proceed
    pass

# Don't stub LangGraph StateGraph - let it execute real workflows
# We'll stub only the LLM calls within AgentRunner instead

# Patch ChatOpenAI so no external network call happens
# Replace with async-friendly stub so that code under test can *await* LLM responses.

langchain_openai.ChatOpenAI = _StubChatOpenAI  # type: ignore[attr-defined]

# Ensure already-imported agent definition module uses the stub as well.  If
# zerg.agents_def.zerg_react_agent was imported *before* this patch it will
# still hold a reference to the real ChatOpenAI class.  We overwrite it here
# defensively so no test ever triggers an external network call.

import sys as _sys

_zr_module = _sys.modules.get("zerg.agents_def.zerg_react_agent")
if _zr_module is not None:  # pragma: no cover – depends on import order
    _zr_module.ChatOpenAI = _StubChatOpenAI  # type: ignore[attr-defined]

# Don't mock AgentRunner globally - let individual tests mock it if needed

# Import app after all engine setup and mocks are in place
from zerg.main import app  # noqa: E402


@pytest.fixture(scope="session", autouse=True)
def disable_langsmith_tracing():
    """
    Fixture to disable LangSmith tracing for all tests.
    This is more robust than setting environment variables.
    """
    # First try patching internal classes that control tracing
    with patch("langsmith.client.Client") as mock_client, patch(
        "langsmith._internal._background_thread.tracing_control_thread_func"
    ) as mock_thread:
        # Disable tracing in the client
        mock_client_instance = MagicMock()
        mock_client_instance.sync_trace.return_value = MagicMock()
        mock_client_instance.trace.return_value = MagicMock()
        mock_client.return_value = mock_client_instance

        # Disable the background thread
        mock_thread.return_value = None

        # Also disable the tracing wrapper API
        with patch("langsmith.wrappers.wrap_openai") as mock_wrap:
            mock_wrap.return_value = lambda *args, **kwargs: args[0]
            yield


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
def client(db_session, auth_headers):
    """
    Create a FastAPI TestClient with the test database dependency and auth headers.
    """

    def override_get_db():
        try:
            yield db_session
        finally:
            pass

    app.dependency_overrides[get_db] = override_get_db

    with TestClient(app, backend="asyncio") as client:
        client.headers = auth_headers
        yield client

    app.dependency_overrides = {}


@pytest.fixture
def unauthenticated_client(db_session):
    """
    Create a FastAPI TestClient without authentication headers.
    """

    def override_get_db():
        try:
            yield db_session
        finally:
            pass

    app.dependency_overrides[get_db] = override_get_db

    with TestClient(app, backend="asyncio") as client:
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

    with TestClient(app, backend="asyncio") as client:
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


# ---------------------------------------------------------------------------
# Fixtures – generic user + agent helpers
# ---------------------------------------------------------------------------


@pytest.fixture
def _dev_user(db_session):
    """Return the deterministic *dev@local* user used when AUTH is disabled."""

    from zerg.crud import crud as _crud

    user = _crud.get_user_by_email(db_session, "dev@local")
    if user is None:
        user = _crud.create_user(db_session, email="dev@local", provider=None, role="USER")
    return user


@pytest.fixture
def sample_agent(db_session, _dev_user):
    """
    Create a sample agent in the database
    """
    agent = Agent(
        owner_id=_dev_user.id,
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


# ---------------------------------------------------------------------------
# HTTP helper fixtures
# ---------------------------------------------------------------------------


@pytest.fixture()
def auth_headers():  # noqa: D401 – pytest fixture naming
    """Return minimal **empty** headers dict for tests that inject auth.

    Most API endpoints depend on :pyfunc:`zerg.dependencies.auth.get_current_user`
    which, in *test mode*, bypasses JWT validation entirely (``AUTH_DISABLED``
    is implicitly enabled via the ``TESTING=1`` env flag set at the top of
    this file).  The tests for the MCP server router still expect a
    ``headers`` fixture so we provide a **no-op** implementation – an empty
    dict is sufficient because the middleware does not look at the headers
    in this scenario.
    """

    # Provide a dummy bearer token so routes that still check for the header
    # despite *AUTH_DISABLED* being in effect treat the request as
    # *authenticated*.  The token is **not** validated in dev-mode so the
    # value is irrelevant.

    return {"Authorization": "Bearer test-token"}


# Alias *db* fixture used in a handful of newer test files.  Internally we
# already expose a fully configured ``db_session`` fixture that yields a
# transactional SQLAlchemy session bound to the in-memory SQLite engine, so
# we simply return that reference for backwards compatibility.


@pytest.fixture()
def db(db_session):  # noqa: D401 – passthrough alias
    """Backward-compatibility shim – provide ``db`` as alias for *db_session*."""

    return db_session


# ---------------------------------------------------------------------------
# *test_user* – alias for legacy fixture name expected by a handful of new
# test modules.  Reuses the deterministic ``_dev_user`` helper.
# ---------------------------------------------------------------------------


@pytest.fixture()
def test_user(_dev_user):  # noqa: D401 – passthrough alias
    """Provide ``test_user`` as backwards-compatible alias for *_dev_user*."""

    return _dev_user


@pytest.fixture
def other_user(db_session):
    """Create a second, distinct user for isolation tests."""
    from zerg.crud import crud as _crud

    user = _crud.get_user_by_email(db_session, "other@local")
    if user is None:
        user = _crud.create_user(db_session, email="other@local", provider=None, role="USER")
    return user


@pytest.fixture
def mock_langgraph_state_graph():
    """
    Mock the StateGraph class from LangGraph directly.

    Note: Previously mocked from zerg.agents which has been removed.
    Now mocks langgraph.graph.StateGraph directly.
    """
    with patch("langgraph.graph.StateGraph") as mock_state_graph:
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


# ---------------------------------------------------------------------------
# Cleanup: stop EmailTriggerService poll loop so pytest can exit immediately
# ---------------------------------------------------------------------------


@pytest.fixture(autouse=True, scope="session")
def _shutdown_email_trigger_service():
    """Ensure background poller is stopped at the end of the test session."""

    from zerg.services.email_trigger_service import email_trigger_service  # noqa: WPS433

    yield  # run tests

    # Cancel poll loop if still running (ignore event-loop already closed)
    try:
        loop = asyncio.get_event_loop()
        if loop.is_running():
            loop.run_until_complete(email_trigger_service.stop())
    except RuntimeError:
        # event-loop already closed by pytest – no action needed
        pass


# ---------------------------------------------------------------------------
# Tool registry cleanup (autouse for every test)
# ---------------------------------------------------------------------------


@pytest.fixture(autouse=True)
def _cleanup_tool_registry():  # noqa: D401 – internal helper
    """Clear runtime-registered tools before & after each test.

    Several tests register mock tools on-the-fly.  Without cleanup later
    tests would see duplicates which either raise validation errors or –
    worse – make failures flaky and hard to trace.
    """

    from zerg.tools.registry import get_registry

    reg = get_registry()
    reg.clear_runtime_tools()
    yield
    reg.clear_runtime_tools()
