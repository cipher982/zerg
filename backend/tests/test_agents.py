from unittest.mock import MagicMock
from unittest.mock import patch

from fastapi.testclient import TestClient

from zerg.app.models.models import Agent


def test_read_agents_empty(client: TestClient):
    """Test the GET /api/agents endpoint with an empty database"""
    response = client.get("/api/agents")
    assert response.status_code == 200
    assert response.json() == []


def test_read_agents(client: TestClient, sample_agent: Agent):
    """Test the GET /api/agents endpoint with a sample agent"""
    response = client.get("/api/agents")
    assert response.status_code == 200
    agents = response.json()
    assert len(agents) == 1
    assert agents[0]["id"] == sample_agent.id
    assert agents[0]["name"] == sample_agent.name
    assert agents[0]["system_instructions"] == sample_agent.system_instructions
    assert agents[0]["task_instructions"] == sample_agent.task_instructions
    assert agents[0]["model"] == sample_agent.model
    assert agents[0]["status"] == sample_agent.status


def test_create_agent(client: TestClient):
    """Test the POST /api/agents endpoint"""
    agent_data = {
        "name": "New Test Agent",
        "system_instructions": "System instructions for new test agent",
        "task_instructions": "This is a new test agent",
        "model": "gpt-4o",
    }

    response = client.post("/api/agents", json=agent_data)
    assert response.status_code == 201
    created_agent = response.json()
    assert created_agent["name"] == agent_data["name"]
    assert created_agent["system_instructions"] == agent_data["system_instructions"]
    assert created_agent["task_instructions"] == agent_data["task_instructions"]
    assert created_agent["model"] == agent_data["model"]
    assert created_agent["status"] == "idle"  # Default value
    assert "id" in created_agent

    # Verify the agent was created in the database by fetching it
    response = client.get(f"/api/agents/{created_agent['id']}")
    assert response.status_code == 200
    fetched_agent = response.json()
    assert fetched_agent["id"] == created_agent["id"]
    assert fetched_agent["name"] == agent_data["name"]


def test_read_agent(client: TestClient, sample_agent: Agent):
    """Test the GET /api/agents/{agent_id} endpoint"""
    response = client.get(f"/api/agents/{sample_agent.id}")
    assert response.status_code == 200
    fetched_agent = response.json()
    assert fetched_agent["id"] == sample_agent.id
    assert fetched_agent["name"] == sample_agent.name
    assert fetched_agent["system_instructions"] == sample_agent.system_instructions
    assert fetched_agent["task_instructions"] == sample_agent.task_instructions
    assert fetched_agent["model"] == sample_agent.model
    assert fetched_agent["status"] == sample_agent.status


def test_read_agent_not_found(client: TestClient):
    """Test the GET /api/agents/{agent_id} endpoint with a non-existent ID"""
    response = client.get("/api/agents/999")  # Assuming this ID doesn't exist
    assert response.status_code == 404
    assert "detail" in response.json()
    assert response.json()["detail"] == "Agent not found"


def test_update_agent(client: TestClient, sample_agent: Agent):
    """Test the PUT /api/agents/{agent_id} endpoint"""
    update_data = {
        "name": "Updated Agent Name",
        "system_instructions": "Updated system instructions",
        "task_instructions": "Updated task instructions",
        "model": "gpt-4o-mini",
        "status": "processing",
    }

    response = client.put(f"/api/agents/{sample_agent.id}", json=update_data)
    assert response.status_code == 200
    updated_agent = response.json()
    assert updated_agent["id"] == sample_agent.id
    assert updated_agent["name"] == update_data["name"]
    assert updated_agent["system_instructions"] == update_data["system_instructions"]
    assert updated_agent["task_instructions"] == update_data["task_instructions"]
    assert updated_agent["model"] == update_data["model"]
    assert updated_agent["status"] == update_data["status"]

    # Verify the agent was updated in the database
    response = client.get(f"/api/agents/{sample_agent.id}")
    assert response.status_code == 200
    fetched_agent = response.json()
    assert fetched_agent["name"] == update_data["name"]


def test_update_agent_not_found(client: TestClient):
    """Test the PUT /api/agents/{agent_id} endpoint with a non-existent ID"""
    update_data = {"name": "This agent doesn't exist"}

    response = client.put("/api/agents/999", json=update_data)
    assert response.status_code == 404
    assert "detail" in response.json()
    assert response.json()["detail"] == "Agent not found"


def test_delete_agent(client: TestClient, sample_agent: Agent):
    """Test the DELETE /api/agents/{agent_id} endpoint"""
    response = client.delete(f"/api/agents/{sample_agent.id}")
    assert response.status_code == 204

    # Verify the agent was deleted
    response = client.get(f"/api/agents/{sample_agent.id}")
    assert response.status_code == 404


def test_delete_agent_not_found(client: TestClient):
    """Test the DELETE /api/agents/{agent_id} endpoint with a non-existent ID"""
    response = client.delete("/api/agents/999")
    assert response.status_code == 404
    assert "detail" in response.json()
    assert response.json()["detail"] == "Agent not found"


def test_run_agent(client: TestClient, sample_agent: Agent, db_session):
    """Test running an agent via a thread"""
    # Create a thread for the agent
    thread_data = {
        "title": "Test Thread for Run",
        "agent_id": sample_agent.id,
        "active": True,
        "memory_strategy": "buffer",
    }

    # Create the thread
    response = client.post("/api/threads", json=thread_data)
    assert response.status_code == 201
    thread = response.json()

    # Create a message for the thread
    message_data = {"role": "user", "content": "Hello, test assistant"}
    response = client.post(f"/api/threads/{thread['id']}/messages", json=message_data)
    assert response.status_code == 201

    # Run the thread
    with patch("zerg.app.agents.AgentManager") as mock_agent_manager_class:
        mock_agent_manager = MagicMock()
        mock_agent_manager_class.return_value = mock_agent_manager

        def mock_process_message(*args, **kwargs):
            yield "Test response"

        mock_agent_manager.process_message.return_value = mock_process_message()

        # Run the thread
        response = client.post(f"/api/threads/{thread['id']}/run", json={"content": "Test message"})
        assert response.status_code == 202

        # Verify the agent status in the database is still valid
        response = client.get(f"/api/agents/{sample_agent.id}")
        assert response.status_code == 200
        fetched_agent = response.json()
        assert fetched_agent["id"] == sample_agent.id


def test_run_agent_not_found(client: TestClient):
    """Test running a non-existent agent via a thread"""
    # Create a thread for a non-existent agent
    thread_data = {
        "title": "Test Thread for Non-existent Agent",
        "agent_id": 999,  # Assuming this ID doesn't exist
        "active": True,
        "memory_strategy": "buffer",
    }

    # Try to create the thread
    response = client.post("/api/threads", json=thread_data)
    assert response.status_code == 404
    assert "detail" in response.json()
    assert response.json()["detail"] == "Agent not found"
