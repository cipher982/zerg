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
from zerg.schemas.workflow import WorkflowData

router = APIRouter(
    prefix="/workflows",
    tags=["workflows"],
    dependencies=[Depends(get_current_user)],
)


class CanvasUpdate(BaseModel):
    """Schema for updating workflow canvas (nodes and edges)"""

    canvas: WorkflowData


class ValidationResponse(BaseModel):
    """Response for workflow validation."""

    is_valid: bool
    errors: List[Dict[str, Any]]
    warnings: List[Dict[str, Any]]


@router.post("/validate", response_model=ValidationResponse)
def validate_workflow(
    *,
    payload: CanvasUpdate,
    current_user: User = Depends(get_current_user),
):
    """
    Validate workflow canvas data without saving.
    """
    try:
        # Direct pydantic validation of WorkflowData
        payload.canvas  # This triggers validation

        # If we get here, validation passed
        return ValidationResponse(is_valid=True, errors=[], warnings=[])

    except Exception as e:
        # Validation error
        return ValidationResponse(
            is_valid=False,
            errors=[{"code": "VALIDATION_ERROR", "message": str(e), "node_id": "", "severity": "error"}],
            warnings=[],
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
    """
    try:
        # WorkflowData validation happens automatically via pydantic
        canvas_dict = workflow_in.canvas.model_dump(by_alias=True)
    except Exception as e:
        raise HTTPException(status_code=400, detail=f"Invalid workflow data: {e}")

    workflow = crud.create_workflow(
        db=db,
        owner_id=current_user.id,
        name=workflow_in.name,
        description=workflow_in.description,
        canvas=canvas_dict,
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
        canvas={"nodes": [], "edges": []},
    )
    return workflow


@router.patch("/current/canvas", response_model=Workflow)
def update_current_workflow_canvas(
    *,
    db: Session = Depends(get_db),
    payload: CanvasUpdate,
    current_user: User = Depends(get_current_user),
):
    """
    Update the canvas for the user's current workflow.
    Creates a default workflow if none exists.
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
            canvas={"nodes": [], "edges": []},
        )

    # Validate and store directly
    try:
        canvas_dict = payload.canvas.model_dump(by_alias=True)
        workflow.canvas = canvas_dict
    except Exception as e:
        raise HTTPException(status_code=400, detail=f"Invalid workflow data: {e}")

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
