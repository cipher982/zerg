from typing import Any
from typing import Dict
from typing import List

from fastapi import APIRouter
from fastapi import Depends
from fastapi import HTTPException
from fastapi import Response
from fastapi import status
from pydantic import BaseModel
from sqlalchemy.orm import Session

from zerg.crud import crud
from zerg.database import get_db
from zerg.dependencies.auth import get_current_user
from zerg.models.models import User

# Canvas layout helper models reused from graph router to avoid duplication.
from zerg.routers.graph_layout import LayoutUpdate
from zerg.schemas.schemas import Workflow
from zerg.schemas.schemas import WorkflowBase
from zerg.schemas.schemas import WorkflowCreate
from zerg.services.canvas_transformer import CanvasTransformer
from zerg.services.workflow_validator import WorkflowValidator

router = APIRouter(
    prefix="/workflows",
    tags=["workflows"],
    dependencies=[Depends(get_current_user)],
)


class CanvasDataUpdate(BaseModel):
    """Schema for updating workflow canvas data (nodes and edges)"""

    canvas_data: Dict[str, Any]


class ValidationResponse(BaseModel):
    """Response for workflow validation."""

    is_valid: bool
    errors: List[Dict[str, Any]]
    warnings: List[Dict[str, Any]]


@router.post("/validate", response_model=ValidationResponse)
def validate_workflow(
    *,
    payload: CanvasDataUpdate,
    current_user: User = Depends(get_current_user),
):
    """
    Validate workflow canvas data without saving.
    """
    # Transform frontend data to canonical format
    canvas = CanvasTransformer.from_frontend(payload.canvas_data)

    validator = WorkflowValidator()
    result = validator.validate_workflow(canvas)

    return ValidationResponse(
        is_valid=result.is_valid,
        errors=[
            {"code": error.code, "message": error.message, "node_id": error.node_id, "severity": error.severity}
            for error in result.errors
        ],
        warnings=[
            {"code": warning.code, "message": warning.message, "node_id": warning.node_id, "severity": warning.severity}
            for warning in result.warnings
        ],
    )


@router.post("/", response_model=Workflow)
def create_workflow(
    *,
    db: Session = Depends(get_db),
    workflow_in: WorkflowCreate,
    current_user: User = Depends(get_current_user),
):
    """
    Create new workflow.

    Note: Validation is not enforced during creation to allow
    progressive workflow building. Use /validate endpoint to
    check validity before execution.
    """
    # Transform frontend data to canonical format
    canvas = CanvasTransformer.from_frontend(workflow_in.canvas_data)

    # Store canonical format in database
    canonical_canvas_data = CanvasTransformer.to_database(canvas)

    workflow = crud.create_workflow(
        db=db,
        owner_id=current_user.id,
        name=workflow_in.name,
        description=workflow_in.description,
        canvas_data=canonical_canvas_data,
    )
    return workflow


@router.get("/current", response_model=Workflow)
def get_current_workflow(
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """
    Get the user's current working workflow.
    Creates a default workflow if none exists.
    """
    # Get most recent workflow
    workflows = crud.get_workflows(db, owner_id=current_user.id, skip=0, limit=1)

    if workflows:
        return workflows[0]

    # Create default workflow if none exists
    workflow = crud.create_workflow(
        db=db,
        owner_id=current_user.id,
        name="My Workflow",
        description="",
        canvas_data={"nodes": [], "edges": []},
    )
    return workflow


@router.patch("/current/canvas-data", response_model=Workflow)
def update_current_workflow_canvas_data(
    *,
    db: Session = Depends(get_db),
    payload: CanvasDataUpdate,
    current_user: User = Depends(get_current_user),
):
    """
    Update the canvas_data for the user's current workflow.
    Creates a default workflow if none exists.

    Note: Validation is not enforced during editing to allow
    progressive workflow building. Use /validate endpoint to
    check validity before execution.
    """
    # Get most recent workflow
    workflows = crud.get_workflows(db, owner_id=current_user.id, skip=0, limit=1)

    if workflows:
        workflow = workflows[0]
    else:
        # Create default workflow if none exists
        workflow = crud.create_workflow(
            db=db,
            owner_id=current_user.id,
            name="My Workflow",
            description="",
            canvas_data={"nodes": [], "edges": []},
        )

    # Transform frontend data to canonical format
    canvas = CanvasTransformer.from_frontend(payload.canvas_data)

    # Store canonical format in database
    workflow.canvas_data = CanvasTransformer.to_database(canvas)
    db.commit()
    db.refresh(workflow)

    return workflow


@router.get("/", response_model=List[Workflow])
def read_workflows(
    db: Session = Depends(get_db),
    skip: int = 0,
    limit: int = 100,
    current_user: User = Depends(get_current_user),
):
    """Return all workflows owned by current user."""

    return crud.get_workflows(db, owner_id=current_user.id, skip=skip, limit=limit)


# Rename workflow
@router.patch("/{workflow_id}", response_model=Workflow)
def rename_workflow(
    *,
    db: Session = Depends(get_db),
    workflow_id: int,
    payload: WorkflowBase,
    current_user: User = Depends(get_current_user),
):
    wf = crud.get_workflow(db, workflow_id)
    if wf is None or wf.owner_id != current_user.id:
        raise HTTPException(status_code=404, detail="workflow not found")

    wf.name = payload.name
    wf.description = payload.description
    db.commit()
    db.refresh(wf)
    return wf


# Delete workflow (soft delete by flag)
@router.delete("/{workflow_id}", status_code=status.HTTP_204_NO_CONTENT)
def delete_workflow(
    *,
    db: Session = Depends(get_db),
    workflow_id: int,
    current_user: User = Depends(get_current_user),
):
    wf = crud.get_workflow(db, workflow_id)
    if wf is None or wf.owner_id != current_user.id:
        raise HTTPException(status_code=404, detail="workflow not found")

    wf.is_active = False
    db.commit()
    return Response(status_code=status.HTTP_204_NO_CONTENT)


# ---------------------------------------------------------------------------
# Canvas layout scoped to a *specific* workflow (Phase-B)
# ---------------------------------------------------------------------------


@router.put("/{workflow_id}/layout", status_code=status.HTTP_204_NO_CONTENT)
def put_workflow_layout(
    *,
    db: Session = Depends(get_db),
    workflow_id: int,
    payload: LayoutUpdate,
    current_user: User = Depends(get_current_user),
):
    """Persist the canvas layout for **workflow_id** owned by *current_user*.

    The endpoint completely replaces the stored layout – callers should send
    the full `nodes` + `viewport` payload (same schema as `/api/graph/layout`).
    """

    wf = crud.get_workflow(db, workflow_id)
    if wf is None or wf.owner_id != current_user.id:
        raise HTTPException(status_code=404, detail="workflow not found")

    nodes_dict = {k: v.dict() for k, v in payload.nodes.items()}
    viewport_dict = payload.viewport.dict() if payload.viewport is not None else None

    crud.upsert_canvas_layout(
        db,
        current_user.id,
        nodes_dict,
        viewport_dict,
        workflow_id,
    )

    return Response(status_code=status.HTTP_204_NO_CONTENT)


@router.get("/{workflow_id}/layout")
def get_workflow_layout(
    *,
    db: Session = Depends(get_db),
    workflow_id: int,
    current_user: User = Depends(get_current_user),
):
    """Return the stored canvas layout for the workflow or **204** when empty."""

    wf = crud.get_workflow(db, workflow_id)
    if wf is None or wf.owner_id != current_user.id:
        raise HTTPException(status_code=404, detail="workflow not found")

    layout = crud.get_canvas_layout(db, current_user.id, workflow_id)
    if layout is None:
        return Response(status_code=status.HTTP_204_NO_CONTENT)

    return {"nodes": layout.nodes_json, "viewport": layout.viewport}
