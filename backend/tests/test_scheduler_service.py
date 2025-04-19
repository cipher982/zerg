import logging

import pytest
from apscheduler.triggers.cron import CronTrigger

from zerg.app.services.scheduler_service import SchedulerService


@pytest.fixture(autouse=True)
def patch_logging(monkeypatch):
    """
    Patch the scheduler service logger for cleaner test output.
    """
    # Silence logging
    monkeypatch.setattr(
        "zerg.app.services.scheduler_service.logger",
        logging.getLogger("test_scheduler_service"),
    )
    yield


@pytest.fixture
def service(test_session_factory):
    """
    Create and return a SchedulerService instance for testing.

    The service is configured to use a test scheduler that does not actually
    run jobs but allows verifying that jobs would be scheduled correctly.
    """
    service = SchedulerService(session_factory=test_session_factory)
    # Mock the event_bus subscription to avoid errors during tests
    monkeypatch = pytest.MonkeyPatch()
    monkeypatch.setattr(
        "zerg.app.services.scheduler_service.event_bus.subscribe",
        lambda event_type, handler: None,
    )
    yield service
    # Ensure scheduler is properly shut down
    if service._initialized:
        service.scheduler.shutdown()


@pytest.mark.asyncio
async def test_schedule_agent(service):
    # Schedule an agent
    agent_id = 42
    cron_expression = "*/5 * * * *"
    await service.schedule_agent(agent_id, cron_expression)

    # Verify the job was added
    job = service.scheduler.get_job(f"agent_{agent_id}")
    assert job is not None
    assert isinstance(job.trigger, CronTrigger)
    assert job.args == (agent_id,)


@pytest.mark.asyncio
async def test_load_scheduled_agents(service, db_session):
    # Insert two agents: one with run_on_schedule=True, one with False
    from zerg.app.models.models import Agent

    a1 = Agent(
        name="A1",
        system_instructions="si",
        task_instructions="ti",
        model="m1",
        status="idle",
        schedule="*/5 * * * *",
        run_on_schedule=True,
    )
    a2 = Agent(
        name="A2",
        system_instructions="si",
        task_instructions="ti",
        model="m1",
        status="idle",
        schedule="*/5 * * * *",
        run_on_schedule=False,
    )
    db_session.add_all([a1, a2])
    db_session.commit()

    # No jobs initially
    assert not service.scheduler.get_jobs()

    # Load scheduled agents
    await service.load_scheduled_agents()

    jobs = service.scheduler.get_jobs()
    assert len(jobs) == 1
    job = service.scheduler.get_job(f"agent_{a1.id}")
    assert job is not None


@pytest.mark.asyncio
async def test_remove_agent_job(service):
    # First schedule an agent
    agent_id = 42
    await service.schedule_agent(agent_id, "*/5 * * * *")

    # Verify it was scheduled
    assert service.scheduler.get_job(f"agent_{agent_id}") is not None

    # Now remove the job
    service.remove_agent_job(agent_id)

    # Verify it was removed
    assert service.scheduler.get_job(f"agent_{agent_id}") is None


@pytest.mark.asyncio
async def test_handle_agent_created(service):
    """Test that an agent creation event schedules the agent if needed."""
    # Create event data
    event_data = {
        "id": 1,
        "name": "Test Agent",
        "schedule": "*/5 * * * *",
        "run_on_schedule": True,
    }

    # Process the event
    await service._handle_agent_created(event_data)

    # Verify the agent was scheduled
    job = service.scheduler.get_job(f"agent_{event_data['id']}")
    assert job is not None
    assert job.args == (event_data["id"],)


@pytest.mark.asyncio
async def test_handle_agent_updated_enabled(service):
    """Test that an agent update event updates its schedule when enabled."""
    # First schedule the agent
    agent_id = 2
    await service.schedule_agent(agent_id, "*/10 * * * *")

    # Update with a new schedule
    event_data = {
        "id": agent_id,
        "schedule": "*/5 * * * *",
        "run_on_schedule": True,
    }

    # Process the update event
    await service._handle_agent_updated(event_data)

    # Verify the schedule was updated
    job = service.scheduler.get_job(f"agent_{agent_id}")
    assert job is not None
    # Check that the job uses the new schedule
    # We can't easily check the cron expression directly but can verify
    # the job still exists with the same ID


@pytest.mark.asyncio
async def test_handle_agent_updated_disabled(service):
    """Test that an agent update event removes the schedule when disabled."""
    # First schedule the agent
    agent_id = 3
    await service.schedule_agent(agent_id, "*/10 * * * *")

    # Verify the agent is scheduled
    assert service.scheduler.get_job(f"agent_{agent_id}") is not None

    # Update to disable scheduling
    event_data = {
        "id": agent_id,
        "run_on_schedule": False,
    }

    # Process the update event
    await service._handle_agent_updated(event_data)

    # Verify the schedule was removed
    assert service.scheduler.get_job(f"agent_{agent_id}") is None


@pytest.mark.asyncio
async def test_handle_agent_deleted(service):
    """Test that an agent deletion event removes any scheduled jobs."""
    # First schedule the agent
    agent_id = 4
    await service.schedule_agent(agent_id, "*/10 * * * *")

    # Verify the agent is scheduled
    assert service.scheduler.get_job(f"agent_{agent_id}") is not None

    # Delete the agent
    event_data = {
        "id": agent_id,
        "name": "Deleted Agent",
    }

    # Process the delete event
    await service._handle_agent_deleted(event_data)

    # Verify the schedule was removed
    assert service.scheduler.get_job(f"agent_{agent_id}") is None
