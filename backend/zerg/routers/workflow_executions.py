from fastapi import APIRouter
from fastapi import Depends
from fastapi import HTTPException
from sqlalchemy.orm import Session

from zerg.crud import crud
from zerg.database import get_db
from zerg.dependencies.auth import get_current_user
from zerg.models.models import User
from zerg.services.workflow_engine import workflow_execution_engine

router = APIRouter(
    prefix="/workflow-executions",
    tags=["workflow-executions"],
    dependencies=[Depends(get_current_user)],
)


@router.post("/{workflow_id}/start")
async def start_workflow_execution(
    workflow_id: int,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """
    Start a new execution of a workflow.
    """
    workflow = crud.get_workflow(db, workflow_id)
    if not workflow or workflow.owner_id != current_user.id:
        raise HTTPException(status_code=404, detail="Workflow not found")

    # This will eventually be a background task
    await workflow_execution_engine.execute_workflow(workflow_id)

    return {"message": "Workflow execution started"}


@router.get("/{execution_id}/status")
def get_execution_status(
    execution_id: int,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """
    Get the status of a workflow execution.
    """
    execution = crud.get_workflow_execution(db, execution_id)
    if not execution or execution.workflow.owner_id != current_user.id:
        raise HTTPException(status_code=404, detail="Execution not found")
    return {"status": execution.status}


@router.get("/{execution_id}/logs")
def get_execution_logs(
    execution_id: int,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """
    Get the logs of a workflow execution.
    """
    execution = crud.get_workflow_execution(db, execution_id)
    if not execution or execution.workflow.owner_id != current_user.id:
        raise HTTPException(status_code=404, detail="Execution not found")
    return {"logs": execution.log}


@router.get("/history/{workflow_id}")
def get_execution_history(
    workflow_id: int,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """
    Get the execution history of a workflow.
    """
    workflow = crud.get_workflow(db, workflow_id)
    if not workflow or workflow.owner_id != current_user.id:
        raise HTTPException(status_code=404, detail="Workflow not found")
    return crud.get_workflow_executions(db, workflow_id)


@router.get("/{execution_id}/export")
def export_execution_data(
    execution_id: int,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """
    Export the data of a workflow execution.
    """
    execution = crud.get_workflow_execution(db, execution_id)
    if not execution or execution.workflow.owner_id != current_user.id:
        raise HTTPException(status_code=404, detail="Execution not found")
    return execution
