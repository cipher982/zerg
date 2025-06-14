from datetime import datetime

from fastapi import APIRouter
from fastapi import Depends
from fastapi import HTTPException
from fastapi import Response
from pydantic import BaseModel
from pydantic import Field
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


# ---------------------------------------------------------------------------
# Models
# ---------------------------------------------------------------------------


class CancelPayload(BaseModel):
    reason: str = Field(..., max_length=500)


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

    # Currently we await directly so the client receives the *execution_id*.
    # Future background-task refactor will keep the response payload stable
    # (i.e. still return the created execution id).
    execution_id = await workflow_execution_engine.execute_workflow(workflow_id)

    return {"execution_id": execution_id, "status": "running"}


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


# ---------------------------------------------------------------------------
# Cancellation endpoint
# ---------------------------------------------------------------------------


@router.patch("/{execution_id}/cancel", status_code=204)
def cancel_execution(
    *,
    execution_id: int,
    payload: CancelPayload,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """Mark a running workflow execution as *cancelled*.

    The engine cooperatively checks the updated status before starting each
    new node and exits early. If the execution already finished the endpoint
    returns 409.
    """

    execution = crud.get_workflow_execution(db, execution_id)
    if execution is None or execution.workflow.owner_id != current_user.id:
        raise HTTPException(status_code=404, detail="Execution not found")

    if execution.status in {"success", "failed", "cancelled"}:
        raise HTTPException(status_code=409, detail="Execution already finished")

    execution.status = "cancelled"
    execution.cancel_reason = payload.reason
    execution.finished_at = datetime.utcnow()
    db.commit()

    # Emit EXECUTION_FINISHED event with cancelled status so UI updates
    import asyncio

    from zerg.events import EventType  # local import to avoid cycles
    from zerg.events import event_bus  # local import to avoid cycles

    payload_dict = {
        "execution_id": execution.id,
        "status": "cancelled",
        "error": payload.reason,
        "duration_ms": None,
        "event_type": EventType.EXECUTION_FINISHED,
    }

    try:
        asyncio.run(event_bus.publish(EventType.EXECUTION_FINISHED, payload_dict))
    except RuntimeError:
        loop = asyncio.new_event_loop()
        loop.run_until_complete(event_bus.publish(EventType.EXECUTION_FINISHED, payload_dict))
        loop.close()

    return Response(status_code=204)
