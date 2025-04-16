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
from zerg.app.database import SessionLocal

logger = logging.getLogger(__name__)


class SchedulerService:
    """Service for managing scheduled agent tasks."""

    def __init__(self):
        """Initialize the scheduler service."""
        self.scheduler = AsyncIOScheduler()
        self._initialized = False

    async def start(self):
        """Start the scheduler if not already running."""
        if not self._initialized:
            # Load all scheduled agents from DB
            await self.load_scheduled_agents()
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

    async def load_scheduled_agents(self):
        """Load all agents with run_on_schedule=True from the database and schedule them."""
        db = SessionLocal()
        try:
            # Get all agents with run_on_schedule=True and valid schedule
            agents = (
                db.query(crud.Agent)
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
            db.close()

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
        db = SessionLocal()
        try:
            # Get the agent
            agent = crud.get_agent(db, agent_id)
            if not agent:
                logger.error(f"Agent {agent_id} not found")
                return

            # Create the agent manager
            agent_manager = AgentManager(agent)

            # Get or create a thread for this scheduled run
            thread, created = agent_manager.get_or_create_thread(db, title=f"Scheduled Run - {agent.name}")

            # If it's a new thread, add the system message
            if created:
                agent_manager.add_system_message(db, thread)

            # Run the agent's task instructions
            logger.info(f"Running scheduled task for agent {agent_id}")
            await agent_manager.process_message(
                db=db,
                thread=thread,
                content=agent.task_instructions,
                stream=False,  # Don't stream for scheduled runs
            )

        except Exception as e:
            logger.error(f"Error running scheduled task for agent {agent_id}: {e}")
        finally:
            db.close()


# Global instance of the scheduler service
scheduler_service = SchedulerService()
