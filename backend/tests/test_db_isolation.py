"""
Database isolation test to ensure fixture-created objects are accessible via API.

This test runs early to detect if the API and test fixtures are using different database connections.
"""

import pytest
from fastapi.testclient import TestClient
from sqlalchemy.orm import Session

from zerg.app.crud import crud

# Import the SQLAlchemy Agent model, not the Pydantic schema
from zerg.app.models.models import Agent


@pytest.mark.db_isolation
@pytest.mark.first  # Make this run first to catch issues early
def test_api_db_isolation(client: TestClient, db_session: Session):
    """
    Test that objects created in test fixtures are accessible via API endpoints.

    This verifies that API routes and test fixtures use the same database connection.
    If this test fails, it likely means there's an issue with database isolation or
    early binding of SQLAlchemy metadata.
    """
    # Create an agent directly using the test session and SQLAlchemy model
    direct_agent = Agent(
        name="DB Isolation Test Agent",
        system_instructions="System instructions for isolation test",
        task_instructions="Task instructions for isolation test",
        model="gpt-4o",
        status="idle",
    )
    db_session.add(direct_agent)
    db_session.commit()
    db_session.refresh(direct_agent)

    # Verify we can get this agent via direct session access
    agent_db = crud.get_agent(db_session, direct_agent.id)
    assert agent_db is not None, "Agent should exist in test database session"

    # Now try to access it via the API
    response = client.get(f"/api/agents/{direct_agent.id}")

    # If this fails, it means the API is using a different database than our test
    assert response.status_code == 200, (
        f"API returned {response.status_code}, expected 200. "
        f"This suggests the API and test fixtures are using DIFFERENT databases. "
        f"Check that SQLAlchemy metadata is not bound too early in the application startup."
    )

    # Verify the content matches what we created
    api_agent = response.json()
    assert api_agent["id"] == direct_agent.id
    assert api_agent["name"] == direct_agent.name

    # Cleanup - although pytest will rollback the transaction anyway
    db_session.delete(direct_agent)
    db_session.commit()


@pytest.mark.db_isolation
def test_non_existent_agent_404(client: TestClient):
    """
    Test that non-existent resources return 404, not connection errors.

    This ensures that API routes work correctly for negative cases too.
    """
    # Try to access a non-existent agent ID (using a very high number to avoid conflicts)
    response = client.get("/api/agents/999999")

    # Should get a 404, not a connection error or 500
    assert response.status_code == 404, (
        f"API returned {response.status_code}, expected 404 for non-existent resource. "
        f"This suggests API database access is not working correctly."
    )

    # Verify the error message is as expected
    error = response.json()
    assert "detail" in error
    assert "not found" in error["detail"].lower()
