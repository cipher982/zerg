import pytest
from fastapi.testclient import TestClient
from sqlalchemy.orm import Session

from zerg.crud import crud
from zerg.models.models import User
from zerg.schemas.schemas import WorkflowCreate

# Test data
TEST_WORKFLOW_NAME = "Test Workflow"
TEST_WORKFLOW_DESCRIPTION = "This is a test workflow."
TEST_CANVAS_DATA = {"nodes": [], "edges": []}


def create_test_workflow(db: Session, owner_id: int):
    """Helper function to create a workflow for testing."""
    return crud.create_workflow(
        db=db,
        owner_id=owner_id,
        name=TEST_WORKFLOW_NAME,
        description=TEST_WORKFLOW_DESCRIPTION,
        canvas_data=TEST_CANVAS_DATA,
    )


def test_create_workflow_success(client: TestClient, test_user: User, db: Session, auth_headers: dict):
    """Test successful creation of a workflow."""
    payload = {
        "name": "New Workflow",
        "description": "A fresh new workflow.",
        "canvas_data": {"nodes": ["node1"], "edges": []},
    }
    response = client.post("/api/workflows/", headers=auth_headers, json=payload)
    assert response.status_code == 200
    data = response.json()
    assert data["name"] == payload["name"]
    assert data["description"] == payload["description"]
    assert data["canvas_data"] == payload["canvas_data"]
    assert data["owner_id"] == test_user.id
    assert "id" in data


def test_create_workflow_unauthenticated(client: TestClient, auth_headers: dict):
    """Test that unauthenticated users cannot create workflows."""
    payload = {
        "name": "Unauthorized Workflow",
        "description": "This should not be created.",
        "canvas_data": {},
    }
    # Remove auth header to simulate unauthenticated request
    auth_headers.pop("Authorization", None)
    response = client.post("/api/workflows/", headers=auth_headers, json=payload)
    assert response.status_code == 401


def test_read_workflows_success(client: TestClient, test_user: User, db: Session, auth_headers: dict):
    """Test reading workflows owned by the current user."""
    # Create a workflow for the user
    create_test_workflow(db, test_user.id)

    response = client.get("/api/workflows/", headers=auth_headers)
    assert response.status_code == 200
    data = response.json()
    assert len(data) > 0
    assert data[0]["name"] == TEST_WORKFLOW_NAME
    assert data[0]["owner_id"] == test_user.id


def test_read_workflows_isolation(
    client: TestClient, test_user: User, other_user: User, db: Session, auth_headers: dict
):
    """Test that a user cannot see workflows owned by another user."""
    # Create a workflow for the other user
    create_test_workflow(db, other_user.id)

    # Authenticate as the primary test user
    response = client.get("/api/workflows/", headers=auth_headers)
    assert response.status_code == 200
    data = response.json()
    # The user should not see the other user's workflow
    assert all(item["owner_id"] != other_user.id for item in data)


def test_rename_workflow_success(client: TestClient, test_user: User, db: Session, auth_headers: dict):
    """Test successfully renaming a workflow."""
    wf = create_test_workflow(db, test_user.id)
    payload = {"name": "Updated Workflow Name", "description": "Updated description."}
    response = client.patch(f"/api/workflows/{wf.id}", headers=auth_headers, json=payload)
    assert response.status_code == 200
    data = response.json()
    assert data["name"] == payload["name"]
    assert data["description"] == payload["description"]
    assert data["id"] == wf.id


def test_rename_workflow_not_found(client: TestClient, test_user: User, auth_headers: dict):
    """Test renaming a workflow that does not exist."""
    payload = {"name": "Doesn't Matter", "description": "Doesn't Matter"}
    response = client.patch("/api/workflows/99999", headers=auth_headers, json=payload)
    assert response.status_code == 404


def test_rename_workflow_unauthorized(
    client: TestClient, test_user: User, other_user: User, db: Session, auth_headers: dict
):
    """Test that a user cannot rename a workflow they do not own."""
    wf = create_test_workflow(db, other_user.id)
    payload = {"name": "Unauthorized Update", "description": "This should fail."}
    response = client.patch(f"/api/workflows/{wf.id}", headers=auth_headers, json=payload)
    assert response.status_code == 404  # API returns 404 for not found or not owned


def test_delete_workflow_success(client: TestClient, test_user: User, db: Session, auth_headers: dict):
    """Test successfully soft-deleting a workflow."""
    wf = create_test_workflow(db, test_user.id)
    response = client.delete(f"/api/workflows/{wf.id}", headers=auth_headers)
    assert response.status_code == 204

    # Verify the workflow is marked as inactive
    db.refresh(wf)
    assert wf.is_active is False


def test_delete_workflow_not_found(client: TestClient, test_user: User, auth_headers: dict):
    """Test deleting a workflow that does not exist."""
    response = client.delete("/api/workflows/99999", headers=auth_headers)
    assert response.status_code == 404


def test_delete_workflow_unauthorized(
    client: TestClient, test_user: User, other_user: User, db: Session, auth_headers: dict
):
    """Test that a user cannot delete a workflow they do not own."""
    wf = create_test_workflow(db, other_user.id)
    response = client.delete(f"/api/workflows/{wf.id}", headers=auth_headers)
    assert response.status_code == 404
