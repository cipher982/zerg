from typing import List

from fastapi import APIRouter
from fastapi import Depends
from fastapi import HTTPException
from sqlalchemy.orm import Session

from zerg.database import get_db
from zerg.dependencies.auth import get_current_user
from zerg.models.models import User
from zerg.schemas.schemas import Workflow
from zerg.schemas.schemas import WorkflowCreate

router = APIRouter(
    prefix="/workflows",
    tags=["workflows"],
    dependencies=[Depends(get_current_user)],
)

from zerg.crud import crud


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
    workflow = crud.create_workflow(
        db=db,
        owner_id=current_user.id,
        name=workflow_in.name,
        description=workflow_in.description,
        canvas_data=workflow_in.canvas_data,
    )
    return workflow


@router.get("/", response_model=List[Workflow])
def read_workflows(
    db: Session = Depends(get_db),
    skip: int = 0,
    limit: int = 100,
    current_user: User = Depends(get_current_user),
):
    """
    Retrieve workflows.
    """
    # TODO: Implement workflow retrieval logic
    raise HTTPException(status_code=501, detail="Not implemented")
