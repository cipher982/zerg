import socket
import sys
import threading
import time
from unittest.mock import MagicMock

import pytest
import requests
import uvicorn
from fastapi import FastAPI
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker
from sqlalchemy.pool import StaticPool

from zerg.app.database import Base
from zerg.app.database import get_db
from zerg.app.models.models import Agent
from zerg.app.models.models import AgentMessage

# Mock the OpenAI module before importing main app
sys.modules["openai"] = MagicMock()
sys.modules["openai.OpenAI"] = MagicMock()

from zerg.main import app  # noqa: E402

# Create a test database - using in-memory SQLite for tests
SQLALCHEMY_DATABASE_URL = "sqlite:///:memory:"

engine = create_engine(
    SQLALCHEMY_DATABASE_URL,
    connect_args={"check_same_thread": False},
    poolclass=StaticPool,  # Use StaticPool for in-memory database
)

TestingSessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)


@pytest.fixture
def db_session():
    """
    Creates a fresh database for each test, then tears it down after the test is done.
    """
    # Create the tables
    Base.metadata.create_all(bind=engine)

    # Create a session
    db = TestingSessionLocal()
    try:
        yield db
    finally:
        db.close()
        # Drop all tables after the test
        Base.metadata.drop_all(bind=engine)


# Simple test client that doesn't rely on TestClient
class SimpleTestClient:
    def __init__(self, app: FastAPI, base_url: str = None):
        self.app = app
        self.port = self._get_free_port()
        self.base_url = base_url or f"http://localhost:{self.port}"
        self.server_thread = None
        self.should_stop = False

    def _get_free_port(self):
        """Find a free port to use for testing"""
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.bind(("127.0.0.1", 0))
            return s.getsockname()[1]

    def start_server(self):
        def run_server():
            uvicorn.run(self.app, host="127.0.0.1", port=self.port, log_level="error")

        self.server_thread = threading.Thread(target=run_server)
        self.server_thread.daemon = True
        self.server_thread.start()
        time.sleep(1)  # Give the server time to start

    def stop_server(self):
        if self.server_thread:
            self.should_stop = True
            self.server_thread.join(timeout=1)

    def get(self, path, **kwargs):
        return requests.get(f"{self.base_url}{path}", **kwargs)

    def post(self, path, **kwargs):
        return requests.post(f"{self.base_url}{path}", **kwargs)

    def put(self, path, **kwargs):
        return requests.put(f"{self.base_url}{path}", **kwargs)

    def delete(self, path, **kwargs):
        return requests.delete(f"{self.base_url}{path}", **kwargs)


@pytest.fixture
def client(db_session):
    """
    Create a test client with a test database dependency
    """

    # Override the get_db dependency
    def override_get_db():
        try:
            yield db_session
        finally:
            pass

    # Override the dependency
    app.dependency_overrides[get_db] = override_get_db

    # Create a test client
    test_client = SimpleTestClient(app)
    test_client.start_server()
    yield test_client
    test_client.stop_server()

    # Clean up the overrides
    app.dependency_overrides = {}


@pytest.fixture
def sample_agent(db_session):
    """
    Create a sample agent in the database
    """
    agent = Agent(name="Test Agent", instructions="This is a test agent", model="gpt-4o", status="idle")
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
