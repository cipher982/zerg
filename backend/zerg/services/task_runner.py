"""Task Runner helper – execute an agent's *task_instructions* once.

This module provides :pyfunc:`execute_agent_task` which is the single source
of truth for running a **non-interactive** task run ("▶ Play" button,
cron/scheduler run, future webhook trigger).

The helper:

1. Creates a fresh thread seeded with the agent's *system* prompt.
2. Inserts one *user* message containing ``agent.task_instructions``.
3. Delegates to :class:`zerg.managers.agent_runner.AgentRunner` to produce the
   assistant/tool messages (token streaming disabled).
4. Updates the agent's status / timestamps and broadcasts
   :pydata:`zerg.events.EventType.AGENT_UPDATED` so dashboards refresh in
   real-time.

It unifies logic that previously lived in *two* places (agents router &
SchedulerService) and removes the last dependency on the legacy
``AgentManager.execute_task`` code path.
"""

from __future__ import annotations

import logging
from datetime import datetime
from datetime import timezone

from sqlalchemy.orm import Session

from zerg.crud import crud
from zerg.events import EventType
from zerg.events.event_bus import event_bus
from zerg.managers.agent_runner import AgentRunner
from zerg.models.models import Agent as AgentModel
from zerg.models.models import Thread as ThreadModel
from zerg.services.thread_service import ThreadService

logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


async def execute_agent_task(db: Session, agent: AgentModel, *, thread_type: str = "manual") -> ThreadModel:
    """Run *agent.task_instructions* exactly once and return the created thread.

    Parameters
    ----------
    db
        Active SQLAlchemy *Session* bound to the current request / job.
    agent
        ORM row of the agent to run.
    thread_type
        One of ``"manual"`` (▶ Play button) or ``"scheduled"`` (cron).  The
        value is persisted on the thread row for analytics.
    """

    # ------------------------------------------------------------------
    # Validate pre-conditions
    # ------------------------------------------------------------------
    if not agent.task_instructions or str(agent.task_instructions).strip() == "":
        raise ValueError("Agent has no task_instructions defined")

    # ------------------------------------------------------------------
    # Flip agent → running and broadcast immediately so UI shows progress.
    # ------------------------------------------------------------------
    crud.update_agent(db, agent.id, status="running")
    db.commit()

    await event_bus.publish(EventType.AGENT_UPDATED, {"id": agent.id, "status": "running"})

    # ------------------------------------------------------------------
    # Create the new thread + seed messages.
    # ------------------------------------------------------------------
    timestamp_str = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M:%S")
    title = f"Task Run – {timestamp_str}"

    thread = ThreadService.create_thread_with_system_message(
        db,
        agent,
        title=title,
        thread_type=thread_type,
        active=False,  # task runs are not the *active* chat thread
    )

    # Insert the user *task* prompt (unprocessed)
    crud.create_thread_message(
        db=db,
        thread_id=thread.id,
        role="user",
        content=agent.task_instructions,
        processed=False,
    )

    # ------------------------------------------------------------------
    # Delegate to AgentRunner (no token stream).
    # ------------------------------------------------------------------
    runner = AgentRunner(agent)

    try:
        await runner.run_thread(db, thread)

    except Exception as exc:
        # Persist error state & broadcast so dashboards refresh
        crud.update_agent(db, agent.id, status="error", last_error=str(exc))
        db.commit()

        await event_bus.publish(
            EventType.AGENT_UPDATED,
            {"id": agent.id, "status": "error", "last_error": str(exc)},
        )

        logger.exception("Task run failed for agent %s", agent.id)
        raise

    # ------------------------------------------------------------------
    # Success – flip agent back to idle and store timestamps.
    # ------------------------------------------------------------------
    now = datetime.now(timezone.utc)
    crud.update_agent(db, agent.id, status="idle", last_run_at=now, last_error=None)
    db.commit()

    await event_bus.publish(
        EventType.AGENT_UPDATED,
        {
            "id": agent.id,
            "status": "idle",
            "last_run_at": now.isoformat(),
            "thread_id": thread.id,
            "last_error": None,
        },
    )

    return thread
