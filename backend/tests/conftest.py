import pytest
from app.database import Base
from app.database import get_db
from app.models import Agent
from app.models import AgentMessage
from fastapi.testclient import TestClient
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker
from sqlalchemy.pool import StaticPool

from main import app

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
    with TestClient(app) as test_client:
        yield test_client

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
