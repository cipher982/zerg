"""Test single-run guard functionality to prevent concurrent agent runs."""

from fastapi.testclient import TestClient
from sqlalchemy.orm import Session

from zerg.crud import crud


class TestSingleRunGuard:
    """Tests for the single-run guard that prevents concurrent agent execution."""

    def test_acquire_run_lock_success(self, db: Session):
        """Test that acquire_run_lock successfully locks an idle agent."""
        # Create test agent
        agent = crud.create_agent(
            db=db,
            owner_id=1,
            name="Test Agent",
            system_instructions="Test system",
            task_instructions="Test task",
            model="gpt-4",
        )

        # Should successfully acquire lock for idle agent
        assert crud.acquire_run_lock(db, agent.id) is True

        # Verify agent status is now running
        updated_agent = crud.get_agent(db, agent.id)
        assert updated_agent.status == "running"

    def test_acquire_run_lock_already_running(self, db: Session):
        """Test that acquire_run_lock fails when agent is already running."""
        # Create test agent and set to running
        agent = crud.create_agent(
            db=db,
            owner_id=1,
            name="Test Agent",
            system_instructions="Test system",
            task_instructions="Test task",
            model="gpt-4",
        )
        crud.update_agent(db, agent.id, status="running")

        # Should fail to acquire lock for running agent
        assert crud.acquire_run_lock(db, agent.id) is False

        # Verify agent status remains running
        updated_agent = crud.get_agent(db, agent.id)
        assert updated_agent.status == "running"

    def test_api_returns_409_for_concurrent_runs(self, client: TestClient, db: Session):
        """Test that the API returns 409 Conflict for concurrent run attempts."""
        # Create test agent
        agent = crud.create_agent(
            db=db,
            owner_id=1,
            name="Test Agent",
            system_instructions="Test system",
            task_instructions="Test task",
            model="gpt-4",
        )

        # Set agent to running status
        crud.update_agent(db, agent.id, status="running")

        # API call should return 409 Conflict
        response = client.post(f"/api/agents/{agent.id}/task")
        assert response.status_code == 409
        assert "already running" in response.json()["detail"].lower()

    def test_lock_is_released_on_success(self, db: Session):
        """Test that the run lock is properly released when agent completes successfully."""
        # Create test agent
        agent = crud.create_agent(
            db=db,
            owner_id=1,
            name="Test Agent",
            system_instructions="Test system",
            task_instructions="Test task",
            model="gpt-4",
        )

        # Acquire lock
        assert crud.acquire_run_lock(db, agent.id) is True

        # Simulate successful completion by updating to idle
        crud.update_agent(db, agent.id, status="idle")

        # Should be able to acquire lock again
        assert crud.acquire_run_lock(db, agent.id) is True
