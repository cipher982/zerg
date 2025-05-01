"""
Run History Service: Consolidates AgentRun lifecycle logic for thread-based runs.
"""

from datetime import datetime
from datetime import timezone
from typing import Sequence

from sqlalchemy.orm import Session

from zerg.crud import crud
from zerg.events import EventType
from zerg.events.event_bus import event_bus
from zerg.managers.agent_runner import AgentRunner
from zerg.models.models import Agent as AgentModel
from zerg.models.models import Thread as ThreadModel


async def execute_thread_run_with_history(
    db: Session,
    agent: AgentModel,
    thread: ThreadModel,
    runner: AgentRunner,
    trigger: str = "api",
) -> Sequence:
    """
    Execute a single run of the agent on the given thread,
    recording AgentRun rows and publishing RUN events.

    Returns the sequence of created message rows from AgentRunner.run_thread().
    """
    # Create the AgentRun (queued)
    run_row = crud.create_run(
        db,
        agent_id=agent.id,
        thread_id=thread.id,
        trigger=trigger,
        status="queued",
    )
    # Notify queued state
    await event_bus.publish(
        EventType.RUN_CREATED,
        {
            "event_type": "run_created",
            "agent_id": agent.id,
            "run_id": run_row.id,
            "status": run_row.status,
        },
    )

    # Mark running
    start_ts = datetime.now(timezone.utc)
    crud.mark_running(db, run_row.id, started_at=start_ts)
    await event_bus.publish(
        EventType.RUN_UPDATED,
        {
            "event_type": "run_updated",
            "agent_id": agent.id,
            "run_id": run_row.id,
            "status": "running",
            "started_at": start_ts.isoformat(),
        },
    )

    # Execute the agent turn
    try:
        created_rows = await runner.run_thread(db, thread)
    except Exception as exc:
        # Failure path
        end_ts = datetime.now(timezone.utc)
        duration_ms = int((end_ts - start_ts).total_seconds() * 1000)
        crud.mark_failed(db, run_row.id, finished_at=end_ts, duration_ms=duration_ms, error=str(exc))
        await event_bus.publish(
            EventType.RUN_UPDATED,
            {
                "event_type": "run_updated",
                "agent_id": agent.id,
                "run_id": run_row.id,
                "status": "failed",
                "finished_at": end_ts.isoformat(),
                "duration_ms": duration_ms,
                "error": str(exc),
            },
        )
        raise

    # Success path
    end_ts = datetime.now(timezone.utc)
    duration_ms = int((end_ts - start_ts).total_seconds() * 1000)
    crud.mark_finished(db, run_row.id, finished_at=end_ts, duration_ms=duration_ms)
    await event_bus.publish(
        EventType.RUN_UPDATED,
        {
            "event_type": "run_updated",
            "agent_id": agent.id,
            "run_id": run_row.id,
            "status": "success",
            "finished_at": end_ts.isoformat(),
            "duration_ms": duration_ms,
        },
    )

    return created_rows
