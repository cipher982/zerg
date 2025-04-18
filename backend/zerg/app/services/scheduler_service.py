"""
Scheduler Service for managing scheduled agent tasks.

This module provides the SchedulerService class that handles:
- Initializing and managing APScheduler
- Loading and scheduling agents from the database
- Running agent tasks on schedule
"""

import logging

from apscheduler.schedulers.asyncio import AsyncIOScheduler
from apscheduler.triggers.cron import CronTrigger

from zerg.app.agents import AgentManager
from zerg.app.crud import crud
from zerg.app.database import default_session_factory
from zerg.app.events import EventType
from zerg.app.events.event_bus import event_bus

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

        logger.info("Scheduler subscribed to agent events")

    async def _handle_agent_created(self, data):
        """Handle agent created events by scheduling if needed."""
        if data.get("run_on_schedule") and data.get("schedule"):
            agent_id = data.get("id")
            cron_expression = data.get("schedule")
            logger.info(f"Scheduling newly created agent {agent_id}")
            await self.schedule_agent(agent_id, cron_expression)

    async def _handle_agent_updated(self, data):
        """
        Handle agent updated events by updating scheduling accordingly.
        This covers cases where run_on_schedule is toggled or the schedule is changed.
        """
        agent_id = data.get("id")
        run_on_schedule = data.get("run_on_schedule")
        schedule = data.get("schedule")

        # If we can't determine the scheduling state, fetch the agent
        if run_on_schedule is None or schedule is None:
            db_session = self.session_factory()
            try:
                agent = crud.get_agent(db_session, agent_id)
                if agent:
                    run_on_schedule = agent.run_on_schedule
                    schedule = agent.schedule
            finally:
                db_session.close()

        # Remove any existing job
        self.remove_agent_job(agent_id)

        # Add new job if needed
        if run_on_schedule and schedule:
            logger.info(f"Updating schedule for agent {agent_id}")
            await self.schedule_agent(agent_id, schedule)
        else:
            logger.info(f"Agent {agent_id} updated, not scheduled to run")

    async def _handle_agent_deleted(self, data):
        """Handle agent deleted events by removing any scheduled jobs."""
        agent_id = data.get("id")
        if agent_id:
            logger.info(f"Removing schedule for deleted agent {agent_id}")
            self.remove_agent_job(agent_id)

    async def load_scheduled_agents(self):
        """Load all agents with run_on_schedule=True from the database and schedule them."""
        db_session = self.session_factory()
        try:
            # Get all agents with run_on_schedule=True and valid schedule
            agents = (
                db_session.query(crud.Agent)
                .filter(
                    crud.Agent.run_on_schedule == True,  # noqa: E712
                    crud.Agent.schedule.isnot(None),
                )
                .all()
            )

            for agent in agents:
                await self.schedule_agent(agent.id, agent.schedule)
                logger.info(f"Scheduled agent {agent.id} with cron: {agent.schedule}")

        except Exception as e:
            logger.error(f"Error loading scheduled agents: {e}")
        finally:
            db_session.close()

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

        except Exception as e:
            logger.error(f"Error scheduling agent {agent_id}: {e}")

    def remove_agent_job(self, agent_id: int):
        """Remove any existing scheduled jobs for the given agent."""
        job_id = f"agent_{agent_id}"
        if self.scheduler.get_job(job_id):
            self.scheduler.remove_job(job_id)
            logger.info(f"Removed existing schedule for agent {agent_id}")

    async def run_agent_task(self, agent_id: int):
        """
        Execute an agent's task.

        This is the function that gets called by the scheduler when a job triggers.
        It handles:
        - Getting a DB session
        - Loading the agent
        - Creating or getting a thread
        - Running the agent's task instructions
        """
        db_session = self.session_factory()
        try:
            # Get the agent
            agent = crud.get_agent(db_session, agent_id)
            if not agent:
                logger.error(f"Agent {agent_id} not found")
                return

            # Create the agent manager
            agent_manager = AgentManager(agent)

            # Get or create a thread for this scheduled run
            thread, created = agent_manager.get_or_create_thread(db_session, title=f"Scheduled Run - {agent.name}")

            # If it's a new thread, add the system message
            if created:
                agent_manager.add_system_message(db_session, thread)

            # Run the agent's task instructions
            logger.info(f"Running scheduled task for agent {agent_id}")
            await agent_manager.process_message(
                db=db_session,
                thread=thread,
                content=agent.task_instructions,
                stream=False,  # Don't stream for scheduled runs
            )

        except Exception as e:
            logger.error(f"Error running scheduled task for agent {agent_id}: {e}")
        finally:
            db_session.close()


# Global instance of the scheduler service
scheduler_service = SchedulerService()
