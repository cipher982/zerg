from datetime import datetime
from typing import Any
from typing import Dict
from typing import List
from typing import Optional

from sqlalchemy.orm import Session

from zerg.models.models import Agent
from zerg.models.models import AgentMessage
from zerg.models.models import AgentRun
from zerg.models.models import Thread
from zerg.models.models import ThreadMessage
from zerg.models.models import Trigger
from zerg.schemas.schemas import RunStatus
from zerg.schemas.schemas import RunTrigger


# Agent CRUD operations
def get_agents(db: Session, skip: int = 0, limit: int = 100):
    """Get all agents with pagination"""
    return db.query(Agent).offset(skip).limit(limit).all()


def get_agent(db: Session, agent_id: int):
    """Get a single agent by ID"""
    return db.query(Agent).filter(Agent.id == agent_id).first()


def create_agent(
    db: Session,
    name: str,
    system_instructions: str,
    task_instructions: str,
    model: str,
    schedule: Optional[str] = None,
    config: Optional[Dict[str, Any]] = None,
):
    """Create a new agent"""
    db_agent = Agent(
        name=name,
        system_instructions=system_instructions,
        task_instructions=task_instructions,
        model=model,
        status="idle",
        schedule=schedule,
        config=config,
        next_run_at=None,
        last_run_at=None,
    )
    db.add(db_agent)
    db.commit()
    db.refresh(db_agent)
    return db_agent


def update_agent(
    db: Session,
    agent_id: int,
    name: Optional[str] = None,
    system_instructions: Optional[str] = None,
    task_instructions: Optional[str] = None,
    model: Optional[str] = None,
    status: Optional[str] = None,
    schedule: Optional[str] = None,
    config: Optional[Dict[str, Any]] = None,
    next_run_at: Optional[datetime] = None,
    last_run_at: Optional[datetime] = None,
    last_error: Optional[str] = None,
):
    """Update an existing agent"""
    db_agent = db.query(Agent).filter(Agent.id == agent_id).first()
    if db_agent is None:
        return None

    # Update provided fields
    if name is not None:
        db_agent.name = name
    if system_instructions is not None:
        db_agent.system_instructions = system_instructions
    if task_instructions is not None:
        db_agent.task_instructions = task_instructions
    if model is not None:
        db_agent.model = model
    if status is not None:
        db_agent.status = status
    if schedule is not None:
        db_agent.schedule = schedule
    if config is not None:
        db_agent.config = config
    if next_run_at is not None:
        db_agent.next_run_at = next_run_at
    if last_run_at is not None:
        db_agent.last_run_at = last_run_at
    if last_error is not None:
        db_agent.last_error = last_error

    db_agent.updated_at = datetime.now()
    db.commit()
    db.refresh(db_agent)
    return db_agent


def delete_agent(db: Session, agent_id: int):
    """Delete an agent and all its messages"""
    db_agent = db.query(Agent).filter(Agent.id == agent_id).first()
    if db_agent is None:
        return False
    db.delete(db_agent)
    db.commit()
    return True


# ------------------------------------------------------------
# Trigger CRUD operations
# ------------------------------------------------------------


def create_trigger(db: Session, agent_id: int, trigger_type: str = "webhook", secret: Optional[str] = None):
    """Create a new trigger for an agent."""
    from uuid import uuid4

    if secret is None:
        secret = uuid4().hex

    db_trigger = Trigger(agent_id=agent_id, type=trigger_type, secret=secret)
    db.add(db_trigger)
    db.commit()
    db.refresh(db_trigger)
    return db_trigger


def get_trigger(db: Session, trigger_id: int):
    return db.query(Trigger).filter(Trigger.id == trigger_id).first()


def delete_trigger(db: Session, trigger_id: int):
    trg = get_trigger(db, trigger_id)
    if trg is None:
        return False
    db.delete(trg)
    db.commit()
    return True


# Agent Message CRUD operations
def get_agent_messages(db: Session, agent_id: int, skip: int = 0, limit: int = 100):
    """Get all messages for a specific agent"""
    return (
        db.query(AgentMessage)
        .filter(AgentMessage.agent_id == agent_id)
        .order_by(AgentMessage.timestamp)
        .offset(skip)
        .limit(limit)
        .all()
    )


def create_agent_message(db: Session, agent_id: int, role: str, content: str):
    """Create a new message for an agent"""
    db_message = AgentMessage(agent_id=agent_id, role=role, content=content)
    db.add(db_message)
    db.commit()
    db.refresh(db_message)
    return db_message


# Thread CRUD operations
def get_threads(db: Session, agent_id: Optional[int] = None, skip: int = 0, limit: int = 100):
    """Get all threads, filtered by agent_id if provided"""
    query = db.query(Thread)
    if agent_id is not None:
        query = query.filter(Thread.agent_id == agent_id)
    return query.offset(skip).limit(limit).all()


def get_active_thread(db: Session, agent_id: int):
    """Get the active thread for an agent, if it exists"""
    return db.query(Thread).filter(Thread.agent_id == agent_id, Thread.active).first()


def get_thread(db: Session, thread_id: int):
    """Get a specific thread by ID"""
    return db.query(Thread).filter(Thread.id == thread_id).first()


def create_thread(
    db: Session,
    agent_id: int,
    title: str,
    active: bool = True,
    agent_state: Optional[Dict[str, Any]] = None,
    memory_strategy: Optional[str] = "buffer",
    thread_type: Optional[str] = "chat",
):
    """Create a new thread for an agent"""
    # If this is set as active, deactivate any other active threads
    if active:
        db.query(Thread).filter(Thread.agent_id == agent_id, Thread.active).update({"active": False})

    db_thread = Thread(
        agent_id=agent_id,
        title=title,
        active=active,
        agent_state=agent_state,
        memory_strategy=memory_strategy,
        thread_type=thread_type,
    )
    db.add(db_thread)
    db.commit()
    db.refresh(db_thread)
    return db_thread


def update_thread(
    db: Session,
    thread_id: int,
    title: Optional[str] = None,
    active: Optional[bool] = None,
    agent_state: Optional[Dict[str, Any]] = None,
    memory_strategy: Optional[str] = None,
):
    """Update a thread"""
    db_thread = db.query(Thread).filter(Thread.id == thread_id).first()
    if db_thread is None:
        return None

    # Update provided fields
    if title is not None:
        db_thread.title = title
    if active is not None:
        if active:
            # Deactivate other threads for this agent
            db.query(Thread).filter(Thread.agent_id == db_thread.agent_id, Thread.id != thread_id).update(
                {"active": False}
            )
        db_thread.active = active
    if agent_state is not None:
        db_thread.agent_state = agent_state
    if memory_strategy is not None:
        db_thread.memory_strategy = memory_strategy

    db_thread.updated_at = datetime.now()
    db.commit()
    db.refresh(db_thread)
    return db_thread


def delete_thread(db: Session, thread_id: int):
    """Delete a thread and all its messages"""
    db_thread = db.query(Thread).filter(Thread.id == thread_id).first()
    if db_thread is None:
        return False
    db.delete(db_thread)
    db.commit()
    return True


# Thread Message CRUD operations
def get_thread_messages(db: Session, thread_id: int, skip: int = 0, limit: int = 100):
    """Get all messages for a specific thread"""
    # Use the *id* column for deterministic chronological ordering. SQLite
    # timestamps have a resolution of 1 second which can lead to two messages
    # inserted within the same second being returned in undefined order.  The
    # auto-incrementing primary-key is strictly monotonic, therefore provides a
    # stable ordering even when multiple rows share the same timestamp.

    return (
        db.query(ThreadMessage)
        .filter(ThreadMessage.thread_id == thread_id)
        .order_by(ThreadMessage.id)
        .offset(skip)
        .limit(limit)
        .all()
    )


def create_thread_message(
    db: Session,
    thread_id: int,
    role: str,
    content: str,
    tool_calls: Optional[List[Dict[str, Any]]] = None,
    tool_call_id: Optional[str] = None,
    name: Optional[str] = None,
    processed: bool = False,
    parent_id: Optional[int] = None,
):
    """Create a new message for a thread"""
    db_message = ThreadMessage(
        thread_id=thread_id,
        role=role,
        content=content,
        tool_calls=tool_calls,
        tool_call_id=tool_call_id,
        name=name,
        processed=processed,
        parent_id=parent_id,
    )
    db.add(db_message)
    db.commit()
    db.refresh(db_message)
    return db_message


def mark_message_processed(db: Session, message_id: int):
    """Mark a message as processed"""
    db_message = db.query(ThreadMessage).filter(ThreadMessage.id == message_id).first()
    if db_message:
        db_message.processed = True
        db.commit()
        db.refresh(db_message)
        return db_message
    return None


def get_unprocessed_messages(db: Session, thread_id: int):
    """Get unprocessed messages for a thread"""
    return (
        db.query(ThreadMessage)
        .filter(ThreadMessage.thread_id == thread_id, ~ThreadMessage.processed)
        .order_by(ThreadMessage.timestamp)
        .all()
    )


# ---------------------------------------------------------------------------
# AgentRun CRUD helpers (Run History feature)
# ---------------------------------------------------------------------------


def create_run(
    db: Session,
    *,
    agent_id: int,
    thread_id: int,
    trigger: str = "manual",
    status: str = "queued",
) -> AgentRun:
    """Insert a new *AgentRun* row.

    Minimal helper to keep service layers free from SQLAlchemy internals.
    """

    # Validate trigger and status enum values
    try:
        trigger_enum = RunTrigger(trigger)
    except ValueError:
        raise ValueError(f"Invalid run trigger: {trigger}")
    try:
        status_enum = RunStatus(status)
    except ValueError:
        raise ValueError(f"Invalid run status: {status}")
    run_row = AgentRun(
        agent_id=agent_id,
        thread_id=thread_id,
        trigger=trigger_enum,
        status=status_enum,
    )
    db.add(run_row)
    db.commit()
    db.refresh(run_row)
    return run_row


def mark_running(db: Session, run_id: int, *, started_at: Optional[datetime] = None) -> Optional[AgentRun]:
    row = db.query(AgentRun).filter(AgentRun.id == run_id).first()
    if row is None:
        return None

    started_at = started_at or datetime.utcnow()
    # Set to running status
    row.status = RunStatus.running
    row.started_at = started_at
    db.commit()
    db.refresh(row)
    return row


def mark_finished(
    db: Session,
    run_id: int,
    *,
    finished_at: Optional[datetime] = None,
    duration_ms: Optional[int] = None,
    total_tokens: Optional[int] = None,
    total_cost_usd: Optional[float] = None,
) -> Optional[AgentRun]:
    row = db.query(AgentRun).filter(AgentRun.id == run_id).first()
    if row is None:
        return None

    finished_at = finished_at or datetime.utcnow()
    if row.started_at and duration_ms is None:
        duration_ms = int((finished_at - row.started_at).total_seconds() * 1000)

    # Set to success status
    row.status = RunStatus.success
    row.finished_at = finished_at
    row.duration_ms = duration_ms
    row.total_tokens = total_tokens
    row.total_cost_usd = total_cost_usd

    db.commit()
    db.refresh(row)
    return row


def mark_failed(
    db: Session,
    run_id: int,
    *,
    finished_at: Optional[datetime] = None,
    duration_ms: Optional[int] = None,
    error: Optional[str] = None,
) -> Optional[AgentRun]:
    row = db.query(AgentRun).filter(AgentRun.id == run_id).first()
    if row is None:
        return None

    finished_at = finished_at or datetime.utcnow()
    if row.started_at and duration_ms is None:
        duration_ms = int((finished_at - row.started_at).total_seconds() * 1000)

    # Set to failed status
    row.status = RunStatus.failed
    row.finished_at = finished_at
    row.duration_ms = duration_ms
    row.error = error

    db.commit()
    db.refresh(row)
    return row


def list_runs(db: Session, agent_id: int, *, limit: int = 20):
    """Return the most recent runs for *agent_id* ordered DESC by id."""
    return db.query(AgentRun).filter(AgentRun.agent_id == agent_id).order_by(AgentRun.id.desc()).limit(limit).all()
