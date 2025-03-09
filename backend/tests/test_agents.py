from app.models import Agent
from fastapi.testclient import TestClient


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
    assert agents[0]["instructions"] == sample_agent.instructions
    assert agents[0]["model"] == sample_agent.model
    assert agents[0]["status"] == sample_agent.status


def test_create_agent(client: TestClient):
    """Test the POST /api/agents endpoint"""
    agent_data = {"name": "New Test Agent", "instructions": "This is a new test agent", "model": "gpt-4o"}

    response = client.post("/api/agents", json=agent_data)
    assert response.status_code == 201
    created_agent = response.json()
    assert created_agent["name"] == agent_data["name"]
    assert created_agent["instructions"] == agent_data["instructions"]
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
    assert fetched_agent["instructions"] == sample_agent.instructions
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
        "instructions": "Updated instructions",
        "model": "gpt-4o-mini",
        "status": "processing",
    }

    response = client.put(f"/api/agents/{sample_agent.id}", json=update_data)
    assert response.status_code == 200
    updated_agent = response.json()
    assert updated_agent["id"] == sample_agent.id
    assert updated_agent["name"] == update_data["name"]
    assert updated_agent["instructions"] == update_data["instructions"]
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


def test_run_agent(client: TestClient, sample_agent: Agent):
    """Test the POST /api/agents/{agent_id}/run endpoint"""
    response = client.post(f"/api/agents/{sample_agent.id}/run")
    assert response.status_code == 200
    run_agent = response.json()
    assert run_agent["id"] == sample_agent.id
    assert run_agent["status"] == "processing"  # Status should be updated

    # Verify the agent status was updated in the database
    response = client.get(f"/api/agents/{sample_agent.id}")
    assert response.status_code == 200
    fetched_agent = response.json()
    assert fetched_agent["status"] == "processing"


def test_run_agent_not_found(client: TestClient):
    """Test the POST /api/agents/{agent_id}/run endpoint with a non-existent ID"""
    response = client.post("/api/agents/999/run")
    assert response.status_code == 404
    assert "detail" in response.json()
    assert response.json()["detail"] == "Agent not found"
