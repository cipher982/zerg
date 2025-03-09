from app import crud
from app.models import Agent
from sqlalchemy.orm import Session


def test_get_agents(db_session: Session, sample_agent: Agent):
    """Test getting all agents"""
    # Create a few more agents
    for i in range(3):
        agent = Agent(name=f"Test Agent {i}", instructions=f"Instructions for agent {i}", model="gpt-4o", status="idle")
        db_session.add(agent)

    db_session.commit()

    # Get all agents
    agents = crud.get_agents(db_session)
    assert len(agents) == 4  # 3 new agents + 1 sample agent

    # Test pagination
    agents_page1 = crud.get_agents(db_session, skip=0, limit=2)
    agents_page2 = crud.get_agents(db_session, skip=2, limit=2)

    assert len(agents_page1) == 2
    assert len(agents_page2) == 2
    assert agents_page1[0].id != agents_page2[0].id  # Should be different agents


def test_get_agent(db_session: Session, sample_agent: Agent):
    """Test getting a single agent by ID"""
    agent = crud.get_agent(db_session, sample_agent.id)
    assert agent is not None
    assert agent.id == sample_agent.id
    assert agent.name == sample_agent.name

    # Test getting a non-existent agent
    non_existent_agent = crud.get_agent(db_session, 999)  # Assuming this ID doesn't exist
    assert non_existent_agent is None


def test_create_agent(db_session: Session):
    """Test creating a new agent"""
    agent = crud.create_agent(
        db_session,
        name="New CRUD Agent",
        instructions="Testing CRUD operations",
        model="gpt-4o-mini",
        schedule="0 12 * * *",  # Noon every day
        config={"test": True},
    )

    assert agent.id is not None
    assert agent.name == "New CRUD Agent"
    assert agent.instructions == "Testing CRUD operations"
    assert agent.model == "gpt-4o-mini"
    assert agent.status == "idle"  # Default value
    assert agent.schedule == "0 12 * * *"
    assert agent.config == {"test": True}

    # Verify the agent was added to the database
    db_agent = crud.get_agent(db_session, agent.id)
    assert db_agent is not None
    assert db_agent.id == agent.id
    assert db_agent.name == agent.name


def test_update_agent(db_session: Session, sample_agent: Agent):
    """Test updating an existing agent"""
    # Update some fields
    updated_agent = crud.update_agent(
        db_session, sample_agent.id, name="Updated CRUD Agent", status="processing", model="gpt-4o-mini"
    )

    assert updated_agent is not None
    assert updated_agent.id == sample_agent.id
    assert updated_agent.name == "Updated CRUD Agent"
    assert updated_agent.status == "processing"
    assert updated_agent.model == "gpt-4o-mini"
    assert updated_agent.instructions == sample_agent.instructions  # Should be unchanged

    # Verify the changes were saved to the database
    db_agent = crud.get_agent(db_session, sample_agent.id)
    assert db_agent.name == "Updated CRUD Agent"
    assert db_agent.status == "processing"

    # Test updating a non-existent agent
    non_existent_update = crud.update_agent(db_session, 999, name="This doesn't exist")
    assert non_existent_update is None


def test_delete_agent(db_session: Session, sample_agent: Agent):
    """Test deleting an agent"""
    # First verify the agent exists
    agent = crud.get_agent(db_session, sample_agent.id)
    assert agent is not None

    # Delete the agent
    success = crud.delete_agent(db_session, sample_agent.id)
    assert success is True

    # Verify the agent is gone
    deleted_agent = crud.get_agent(db_session, sample_agent.id)
    assert deleted_agent is None

    # Test deleting a non-existent agent
    success = crud.delete_agent(db_session, 999)  # Assuming this ID doesn't exist
    assert success is False


def test_get_agent_messages(db_session: Session, sample_agent: Agent, sample_messages):
    """Test getting messages for an agent"""
    messages = crud.get_agent_messages(db_session, sample_agent.id)
    assert len(messages) == 3  # From the sample_messages fixture

    # Test pagination
    messages_page = crud.get_agent_messages(db_session, sample_agent.id, skip=1, limit=1)
    assert len(messages_page) == 1

    # Test getting messages for a non-existent agent
    non_existent_messages = crud.get_agent_messages(db_session, 999)
    assert len(non_existent_messages) == 0


def test_create_agent_message(db_session: Session, sample_agent: Agent):
    """Test creating a message for an agent"""
    message = crud.create_agent_message(
        db_session, agent_id=sample_agent.id, role="user", content="Testing CRUD message creation"
    )

    assert message.id is not None
    assert message.agent_id == sample_agent.id
    assert message.role == "user"
    assert message.content == "Testing CRUD message creation"
    assert message.timestamp is not None

    # Verify the message is in the database
    messages = crud.get_agent_messages(db_session, sample_agent.id)
    message_ids = [m.id for m in messages]
    assert message.id in message_ids
