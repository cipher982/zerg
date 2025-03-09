from datetime import datetime
from typing import Any
from typing import Dict
from typing import Optional

from sqlalchemy.orm import Session

from . import models


# Agent CRUD operations
def get_agents(db: Session, skip: int = 0, limit: int = 100):
    """Get all agents with pagination"""
    return db.query(models.Agent).offset(skip).limit(limit).all()


def get_agent(db: Session, agent_id: int):
    """Get a single agent by ID"""
    return db.query(models.Agent).filter(models.Agent.id == agent_id).first()


def create_agent(
    db: Session,
    name: str,
    instructions: str,
    model: str = "gpt-4o",
    schedule: Optional[str] = None,
    config: Optional[Dict[str, Any]] = None,
):
    """Create a new agent"""
    db_agent = models.Agent(
        name=name, instructions=instructions, model=model, status="idle", schedule=schedule, config=config
    )
    db.add(db_agent)
    db.commit()
    db.refresh(db_agent)
    return db_agent


def update_agent(
    db: Session,
    agent_id: int,
    name: Optional[str] = None,
    instructions: Optional[str] = None,
    model: Optional[str] = None,
    status: Optional[str] = None,
    schedule: Optional[str] = None,
    config: Optional[Dict[str, Any]] = None,
):
    """Update an existing agent"""
    db_agent = db.query(models.Agent).filter(models.Agent.id == agent_id).first()
    if db_agent is None:
        return None

    # Update provided fields
    if name is not None:
        db_agent.name = name
    if instructions is not None:
        db_agent.instructions = instructions
    if model is not None:
        db_agent.model = model
    if status is not None:
        db_agent.status = status
    if schedule is not None:
        db_agent.schedule = schedule
    if config is not None:
        db_agent.config = config

    db_agent.updated_at = datetime.now()
    db.commit()
    db.refresh(db_agent)
    return db_agent


def delete_agent(db: Session, agent_id: int):
    """Delete an agent and all its messages"""
    db_agent = db.query(models.Agent).filter(models.Agent.id == agent_id).first()
    if db_agent is None:
        return False
    db.delete(db_agent)
    db.commit()
    return True


# Agent Message CRUD operations
def get_agent_messages(db: Session, agent_id: int, skip: int = 0, limit: int = 100):
    """Get all messages for a specific agent"""
    return (
        db.query(models.AgentMessage)
        .filter(models.AgentMessage.agent_id == agent_id)
        .order_by(models.AgentMessage.timestamp)
        .offset(skip)
        .limit(limit)
        .all()
    )


def create_agent_message(db: Session, agent_id: int, role: str, content: str):
    """Create a new message for an agent"""
    db_message = models.AgentMessage(agent_id=agent_id, role=role, content=content)
    db.add(db_message)
    db.commit()
    db.refresh(db_message)
    return db_message
