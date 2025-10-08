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
    # Persist usage/cost if Runner captured metadata
    total_tokens = getattr(runner, "usage_total_tokens", None)
    total_cost_usd = None
    if (
        getattr(runner, "usage_prompt_tokens", None) is not None
        and getattr(runner, "usage_completion_tokens", None) is not None
    ):
        from zerg.pricing import get_usd_prices_per_1k

        prices = get_usd_prices_per_1k(agent.model)
        if prices is not None:
            in_price, out_price = prices
            total_cost_usd = (
                (runner.usage_prompt_tokens * in_price) + (runner.usage_completion_tokens * out_price)
            ) / 1000.0

    # Mark run as finished (summary auto-extracted)
    finished_run = crud.mark_finished(
        db,
        run_row.id,
        finished_at=end_ts,
        duration_ms=duration_ms,
        total_tokens=total_tokens,
        total_cost_usd=total_cost_usd,
    )

    # Refresh to get the auto-extracted summary
    if finished_run:
        db.refresh(finished_run)

    await event_bus.publish(
        EventType.RUN_UPDATED,
        {
            "event_type": "run_updated",
            "agent_id": agent.id,
            "run_id": run_row.id,
            "status": "success",
            "finished_at": end_ts.isoformat(),
            "duration_ms": duration_ms,
            "summary": finished_run.summary if finished_run else None,
        },
    )

    return created_rows
