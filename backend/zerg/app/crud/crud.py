from datetime import datetime
from typing import Any
from typing import Dict
from typing import List
from typing import Optional

from sqlalchemy.orm import Session

from zerg.app.models.models import Agent
from zerg.app.models.models import AgentMessage
from zerg.app.models.models import Thread
from zerg.app.models.models import ThreadMessage
from zerg.app.models.models import Trigger


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
    run_on_schedule: Optional[bool] = False,
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
        run_on_schedule=run_on_schedule if run_on_schedule is not None else False,
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
    run_on_schedule: Optional[bool] = None,
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
    if run_on_schedule is not None:
        db_agent.run_on_schedule = run_on_schedule
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
    return (
        db.query(ThreadMessage)
        .filter(ThreadMessage.thread_id == thread_id)
        .order_by(ThreadMessage.timestamp)
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
