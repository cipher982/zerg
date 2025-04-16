import logging

import pytest
from apscheduler.triggers.cron import CronTrigger

from zerg.app.services.scheduler_service import SchedulerService


@pytest.fixture(autouse=True)
def patch_session_local(monkeypatch, db_session):
    """
    Patch the SessionLocal in scheduler_service to use the test DB session.
    """
    # Replace SessionLocal to return the test db_session
    monkeypatch.setattr(
        "zerg.app.services.scheduler_service.SessionLocal",
        lambda: db_session,
    )
    # Silence logging
    monkeypatch.setattr(
        "zerg.app.services.scheduler_service.logger",
        logging.getLogger("test_scheduler_service"),
    )
    yield
    # No cleanup needed


@pytest.fixture
def service():
    """Return a fresh SchedulerService instance."""
    return SchedulerService()


@pytest.mark.asyncio
async def test_schedule_agent_adds_job(service):
    """Test that schedule_agent creates a job with the correct ID and function."""
    agent_id = 1
    cron = "0 * * * *"

    await service.schedule_agent(agent_id=agent_id, cron_expression=cron)

    # Check job exists with correct ID
    job = service.scheduler.get_job(f"agent_{agent_id}")
    assert job is not None, "Job should be scheduled"

    # Check it's using the correct function (run_agent_task)
    assert job.func == service.run_agent_task

    # Check it was given the correct agent_id argument
    assert job.args == (agent_id,)


def test_remove_agent_job(service):
    # Add a job first
    service.scheduler.add_job(lambda: None, id="agent_2", trigger=CronTrigger.from_crontab("*/5 * * * *"))
    assert service.scheduler.get_job("agent_2") is not None

    service.remove_agent_job(agent_id=2)
    assert service.scheduler.get_job("agent_2") is None


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
async def test_run_agent_task_invokes_manager(monkeypatch, service, db_session):
    # Create a sample agent
    from zerg.app.crud import crud
    from zerg.app.models.models import Agent

    agent = Agent(
        name="Test",
        system_instructions="sys",
        task_instructions="do something",
        model="m1",
        status="idle",
        run_on_schedule=True,
    )
    db_session.add(agent)
    db_session.commit()
    db_session.refresh(agent)

    # Dummy AgentManager to capture calls
    called = {}

    class DummyManager:
        def __init__(self, agent_model):
            assert agent_model.id == agent.id

        def get_or_create_thread(self, db, title):
            # Create a new thread via CRUD to simulate real behavior
            thread = crud.create_thread(
                db=db,
                agent_id=agent.id,
                title=title,
                active=True,
                agent_state={},
                memory_strategy="buffer",
            )
            called["created_thread"] = True
            return thread, True

        def add_system_message(self, db, thread):
            called["added_system_message"] = True

        async def process_message(self, db, thread, content, stream):
            called["processed"] = (content, stream)

    # Patch AgentManager in the scheduler service module
    monkeypatch.setattr(
        "zerg.app.services.scheduler_service.AgentManager",
        DummyManager,
    )

    # Run the task
    await service.run_agent_task(agent_id=agent.id)

    # Verify DummyManager methods were called
    assert called.get("created_thread", False), "Thread should be created"
    assert called.get("added_system_message", False), "System message should be added"
    assert called.get("processed") == ("do something", False)
