"""
Scheduler Service for managing scheduled agent tasks.

This module provides the SchedulerService class that handles:
- Initializing and managing APScheduler
- Loading and scheduling agents from the database
- Running agent tasks on schedule
"""

import logging

# APScheduler is part of the mandatory backend dependencies; import directly.
from apscheduler.schedulers.asyncio import AsyncIOScheduler
from apscheduler.triggers.cron import CronTrigger

# Legacy AgentManager no longer required – all logic goes through TaskRunner
from zerg.crud import crud
from zerg.database import default_session_factory

# EventBus remains for UI notifications
from zerg.events import EventType
from zerg.events.event_bus import event_bus

# New unified task runner helper
from zerg.services.task_runner import execute_agent_task

logger = logging.getLogger(__name__)


class SchedulerService:
    """Service for managing scheduled agent tasks."""

    def __init__(self, session_factory=None):
        """Initialize the scheduler service."""
        self.scheduler = AsyncIOScheduler()
        self._initialized = False
        self.session_factory = session_factory or default_session_factory

    async def start(self):
        """Start the scheduler if not already running."""
        if not self._initialized:
            # Load all scheduled agents from DB
            await self.load_scheduled_agents()

            # Subscribe to agent events for dynamic scheduling
            await self._subscribe_to_events()

            # Start the scheduler
            self.scheduler.start()
            self._initialized = True
            logger.info("Scheduler service started")

    async def stop(self):
        """Shutdown the scheduler gracefully."""
        if self._initialized:
            self.scheduler.shutdown()
            self._initialized = False
            logger.info("Scheduler service stopped")

    async def _subscribe_to_events(self):
        """Subscribe to agent-related events for dynamic scheduling updates."""
        # Subscribe to agent created events
        event_bus.subscribe(EventType.AGENT_CREATED, self._handle_agent_created)
        # Subscribe to agent updated events
        event_bus.subscribe(EventType.AGENT_UPDATED, self._handle_agent_updated)
        # Subscribe to agent deleted events
        event_bus.subscribe(EventType.AGENT_DELETED, self._handle_agent_deleted)

        # External triggers
        event_bus.subscribe(EventType.TRIGGER_FIRED, self._handle_trigger_fired)

        logger.info("Scheduler subscribed to agent events")

    async def _handle_agent_created(self, data):
        """Handle agent created events by scheduling if needed."""
        if data.get("schedule"):
            agent_id = data.get("id")
            cron_expression = data.get("schedule")
            logger.info(f"Scheduling newly created agent {agent_id}")
            await self.schedule_agent(agent_id, cron_expression)

    async def _handle_agent_updated(self, data):
        """
        Handle agent updated events by updating scheduling accordingly.
        Re-schedule or unschedule the job when the cron expression changes.
        """
        agent_id = data.get("id")
        schedule = data.get("schedule")

        # If we can't determine schedule, load from DB
        if schedule is None:
            db_session = self.session_factory()
            try:
                agent = crud.get_agent(db_session, agent_id)
                if agent:
                    schedule = agent.schedule
            finally:
                db_session.close()

        # Remove any existing job regardless
        self.remove_agent_job(agent_id)

        # Re-schedule if a cron expression is set
        if schedule:
            logger.info(f"Updating schedule for agent {agent_id}")
            await self.schedule_agent(agent_id, schedule)
        else:
            logger.info(f"Agent {agent_id} now has no schedule – unscheduled.")

    async def _handle_agent_deleted(self, data):
        """Handle agent deleted events by removing any scheduled jobs."""
        agent_id = data.get("id")
        if agent_id:
            logger.info(f"Removing schedule for deleted agent {agent_id}")
            self.remove_agent_job(agent_id)

    async def _handle_trigger_fired(self, data):
        """Run the associated agent immediately when a trigger fires."""

        agent_id = data.get("agent_id")
        if agent_id is None:
            logger.warning("trigger_fired event missing agent_id – ignoring")
            return

        logger.info(f"Trigger fired for agent {agent_id}; executing run task now")

        # Execute the agent task immediately (await) so tests can observe the
        # call synchronously; the actual work done inside `run_agent_task` is
        # asynchronous and non‑blocking.  If later we need true fire‑and‑forget
        # behaviour we can switch back to `asyncio.create_task`.
        await self.run_agent_task(agent_id)

    async def load_scheduled_agents(self):
        """Load all agents that define a cron schedule and register them."""

        db_session = self.session_factory()
        try:
            # Get all agents that have a non-null schedule
            agent_rows = (
                db_session.query(crud.Agent.id, crud.Agent.schedule).filter(crud.Agent.schedule.isnot(None)).all()
            )

            for agent_id, cron_expr in agent_rows:
                await self.schedule_agent(agent_id, cron_expr)
                logger.info(f"Scheduled agent {agent_id} with cron: {cron_expr}")

        except Exception as e:
            logger.error(f"Error loading scheduled agents: {e}")
        finally:
            # Intentionally do *not* close the session here. Doing so would
            # detach the ORM objects that the caller (e.g. test cases) may
            # still hold references to, leading to DetachedInstanceError when
            # their attributes are later accessed. The session will be
            # garbage‑collected at the end of the test, and production code
            # runs load_scheduled_agents() only once on startup.
            pass

    async def schedule_agent(self, agent_id: int, cron_expression: str):
        """
        Schedule an agent to run according to its cron expression.

        Args:
            agent_id: The ID of the agent to schedule
            cron_expression: The cron expression defining when to run the agent
        """
        try:
            # Remove any existing jobs for this agent
            self.remove_agent_job(agent_id)

            # Add new job with the cron trigger
            self.scheduler.add_job(
                self.run_agent_task,
                CronTrigger.from_crontab(cron_expression),
                args=[agent_id],
                id=f"agent_{agent_id}",
                replace_existing=True,
            )
            logger.info(f"Added schedule for agent {agent_id}: {cron_expression}")

            # Persist next run time in DB
            job = self.scheduler.get_job(f"agent_{agent_id}")
            # Only persist if we have a valid next run AND the scheduler is
            # already running (during test load_scheduled_agents the scheduler
            # has not started yet and persisting here detaches instances that
            # the tests still hold).
            if self.scheduler.running and job and getattr(job, "next_run_time", None):
                next_run = job.next_run_time

                db_session = self.session_factory()
                try:
                    agent = crud.get_agent(db_session, agent_id)
                    if agent:
                        agent.next_run_at = next_run
                        db_session.commit()
                finally:
                    db_session.close()

        except Exception as e:
            logger.error(f"Error scheduling agent {agent_id}: {e}")

    def remove_agent_job(self, agent_id: int):
        """Remove any existing scheduled jobs for the given agent."""
        job_id = f"agent_{agent_id}"
        if self.scheduler.get_job(job_id):
            self.scheduler.remove_job(job_id)
            logger.info(f"Removed existing schedule for agent {agent_id}")

        # Clear next_run_at in DB as it's no longer scheduled
        db_session = self.session_factory()
        try:
            agent = crud.get_agent(db_session, agent_id)
            if agent:
                agent.next_run_at = None
                db_session.commit()
        finally:
            db_session.close()

    async def run_agent_task(self, agent_id: int):
        """
        Execute an agent's task.

        This is the function that gets called by the scheduler when a job triggers.
        It handles:
        - Getting a DB session
        - Loading the agent
        - Creating a new thread for this run using the execute_task method
        - Running the agent's task instructions
        """
        db_session = self.session_factory()
        try:
            agent = crud.get_agent(db_session, agent_id)
            if agent is None:
                logger.error("Agent %s not found", agent_id)
                return

            # ------------------------------------------------------------------
            # Delegate to shared helper (handles status flips & events).
            # ------------------------------------------------------------------
            logger.info("Running scheduled task for agent %s", agent_id)
            # Use "schedule" to match RunTrigger.schedule and CRUD trigger logic
            thread = await execute_agent_task(db_session, agent, thread_type="schedule")

            # ------------------------------------------------------------------
            # Update *next_run_at* after successful run so dashboards show when
            # the task will fire next.  We do *not* touch last_run_at – helper
            # already set it.
            # ------------------------------------------------------------------
            job = self.scheduler.get_job(f"agent_{agent_id}")
            next_run_time = getattr(job, "next_run_time", None) if job else None
            if next_run_time:
                crud.update_agent(db_session, agent_id, next_run_at=next_run_time)
                db_session.commit()

                await event_bus.publish(
                    EventType.AGENT_UPDATED,
                    {
                        "id": agent_id,
                        "next_run_at": next_run_time.isoformat(),
                        "thread_id": thread.id,
                    },
                )

        except Exception as exc:
            # execute_agent_task already flipped status to *error* and
            # broadcasted so here we just log.
            logger.exception("Scheduled task failed for agent %s: %s", agent_id, exc)
        finally:
            db_session.close()


# Global instance of the scheduler service
scheduler_service = SchedulerService()
