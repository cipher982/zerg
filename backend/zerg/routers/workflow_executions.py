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
from zerg.services.workflow_scheduler import workflow_scheduler

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


class ScheduleWorkflowPayload(BaseModel):
    cron_expression: str = Field(..., min_length=1, max_length=100)
    trigger_config: dict = Field(default_factory=dict)


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


# ---------------------------------------------------------------------------
# Workflow Scheduling endpoints
# ---------------------------------------------------------------------------


@router.post("/{workflow_id}/schedule")
async def schedule_workflow(
    workflow_id: int,
    payload: ScheduleWorkflowPayload,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """
    Schedule a workflow to run on a cron schedule.
    """
    workflow = crud.get_workflow(db, workflow_id)
    if not workflow or workflow.owner_id != current_user.id:
        raise HTTPException(status_code=404, detail="Workflow not found")

    success = await workflow_scheduler.schedule_workflow(
        workflow_id=workflow_id,
        cron_expression=payload.cron_expression,
        trigger_config=payload.trigger_config,
    )

    if not success:
        raise HTTPException(status_code=400, detail="Failed to schedule workflow")

    return {"status": "scheduled", "cron_expression": payload.cron_expression}


@router.delete("/{workflow_id}/schedule")
def unschedule_workflow(
    workflow_id: int,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """
    Remove the schedule for a workflow.
    """
    workflow = crud.get_workflow(db, workflow_id)
    if not workflow or workflow.owner_id != current_user.id:
        raise HTTPException(status_code=404, detail="Workflow not found")

    success = workflow_scheduler.unschedule_workflow(workflow_id)

    if not success:
        raise HTTPException(status_code=404, detail="Workflow not scheduled")

    return {"status": "unscheduled"}


@router.get("/{workflow_id}/schedule")
def get_workflow_schedule(
    workflow_id: int,
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """
    Get the current schedule for a workflow.
    """
    workflow = crud.get_workflow(db, workflow_id)
    if not workflow or workflow.owner_id != current_user.id:
        raise HTTPException(status_code=404, detail="Workflow not found")

    scheduled_workflows = workflow_scheduler.get_scheduled_workflows()

    if workflow_id in scheduled_workflows:
        schedule_info = scheduled_workflows[workflow_id]
        return {
            "scheduled": True,
            "next_run_time": schedule_info["next_run_time"].isoformat() if schedule_info["next_run_time"] else None,
            "trigger": schedule_info["trigger"],
        }
    else:
        return {"scheduled": False}


@router.get("/scheduled")
def list_scheduled_workflows(
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user),
):
    """
    List all scheduled workflows for the current user.
    """
    scheduled_workflows = workflow_scheduler.get_scheduled_workflows()

    # Filter to only workflows owned by current user
    user_scheduled = []
    for workflow_id, schedule_info in scheduled_workflows.items():
        workflow = crud.get_workflow(db, workflow_id)
        if workflow and workflow.owner_id == current_user.id:
            user_scheduled.append(
                {
                    "workflow_id": workflow_id,
                    "workflow_name": workflow.name,
                    "next_run_time": schedule_info["next_run_time"].isoformat()
                    if schedule_info["next_run_time"]
                    else None,
                    "trigger": schedule_info["trigger"],
                }
            )

    return {"scheduled_workflows": user_scheduled}
