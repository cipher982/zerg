"""Agent routes module."""

import logging
import os
from datetime import datetime
from typing import List

from dotenv import load_dotenv
from fastapi import APIRouter
from fastapi import Depends
from fastapi import HTTPException
from fastapi import status
from openai import OpenAI
from sqlalchemy.orm import Session

from zerg.app.crud import crud
from zerg.app.database import get_db
from zerg.app.events import EventType
from zerg.app.events.decorators import publish_event
from zerg.app.schemas.schemas import Agent
from zerg.app.schemas.schemas import AgentCreate
from zerg.app.schemas.schemas import AgentUpdate
from zerg.app.schemas.schemas import MessageCreate
from zerg.app.schemas.schemas import MessageResponse

load_dotenv()

# Set up logging
logger = logging.getLogger(__name__)

router = APIRouter(
    tags=["agents"],
)

# Initialize OpenAI client
client = OpenAI(
    api_key=os.environ.get("OPENAI_API_KEY"),
    # Don't pass any other parameters that might cause compatibility issues
)


@router.get("/", response_model=List[Agent])
@router.get("", response_model=List[Agent])
def read_agents(skip: int = 0, limit: int = 100, db: Session = Depends(get_db)):
    """Get all agents"""
    agents = crud.get_agents(db, skip=skip, limit=limit)
    # Return empty list instead of exception for no agents
    if not agents:
        return []
    return agents


@router.post("/", response_model=Agent, status_code=status.HTTP_201_CREATED)
@router.post("", response_model=Agent, status_code=status.HTTP_201_CREATED)
@publish_event(EventType.AGENT_CREATED)
async def create_agent(agent: AgentCreate, db: Session = Depends(get_db)):
    """Create a new agent"""
    return crud.create_agent(
        db=db,
        name=agent.name,
        system_instructions=agent.system_instructions,
        task_instructions=agent.task_instructions,
        model=agent.model,
        schedule=agent.schedule,
        config=agent.config,
        run_on_schedule=agent.run_on_schedule,
    )


@router.get("/{agent_id}", response_model=Agent)
def read_agent(agent_id: int, db: Session = Depends(get_db)):
    """Get a specific agent by ID"""
    db_agent = crud.get_agent(db, agent_id=agent_id)
    if db_agent is None:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Agent not found")
    return db_agent


@router.put("/{agent_id}", response_model=Agent)
@publish_event(EventType.AGENT_UPDATED)
async def update_agent(agent_id: int, agent: AgentUpdate, db: Session = Depends(get_db)):
    """Update an agent"""
    db_agent = crud.update_agent(
        db=db,
        agent_id=agent_id,
        name=agent.name,
        system_instructions=agent.system_instructions,
        task_instructions=agent.task_instructions,
        model=agent.model,
        status=agent.status,
        schedule=agent.schedule,
        config=agent.config,
        run_on_schedule=agent.run_on_schedule,
    )
    if db_agent is None:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Agent not found")
    return db_agent


@router.delete("/{agent_id}", status_code=status.HTTP_204_NO_CONTENT)
@publish_event(EventType.AGENT_DELETED)
async def delete_agent(agent_id: int, db: Session = Depends(get_db)):
    """Delete an agent"""
    # Get the agent first to include in the event data
    agent = crud.get_agent(db, agent_id=agent_id)
    if agent is None:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Agent not found")

    # Delete the agent
    if not crud.delete_agent(db, agent_id=agent_id):
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Agent not found")

    # Return the agent data for the event
    return agent


# Agent messages endpoints
@router.get("/{agent_id}/messages", response_model=List[MessageResponse])
def read_agent_messages(agent_id: int, skip: int = 0, limit: int = 100, db: Session = Depends(get_db)):
    """Get all messages for an agent"""
    # First check if agent exists
    if not crud.get_agent(db, agent_id=agent_id):
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Agent not found")

    messages = crud.get_agent_messages(db, agent_id=agent_id, skip=skip, limit=limit)
    if not messages:
        return []
    return messages


@router.post("/{agent_id}/messages", response_model=MessageResponse, status_code=status.HTTP_201_CREATED)
def create_agent_message(agent_id: int, message: MessageCreate, db: Session = Depends(get_db)):
    """Create a new message for an agent"""
    # First check if agent exists
    if not crud.get_agent(db, agent_id=agent_id):
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Agent not found")

    return crud.create_agent_message(db=db, agent_id=agent_id, role=message.role, content=message.content)


# ---------------------------------------------------------------------------
# Manual "▶ Play" endpoint
# ---------------------------------------------------------------------------


@router.post("/{agent_id}/task", status_code=status.HTTP_202_ACCEPTED)
async def run_agent_task(agent_id: int, db: Session = Depends(get_db)):
    """Run the agent's main task (task_instructions) in a new thread, matching scheduled run behavior."""
    from zerg.app.agents import AgentManager

    agent = crud.get_agent(db, agent_id=agent_id)
    if not agent:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Agent not found")

    agent_manager = AgentManager(agent)

    # ------------------------------------------------------------------
    # 1. Mark the agent as *running* and broadcast the change so that the
    #    dashboard can flip the status badge immediately.
    # ------------------------------------------------------------------
    from zerg.app.events.event_bus import EventType
    from zerg.app.events.event_bus import event_bus

    crud.update_agent(db, agent_id, status="running")
    # We need to commit before publishing so that other DB readers (e.g.
    # websocket handlers that fetch agent state) observe the change.
    db.commit()

    # Fire-and‑forget publish (no await-able subscribers need the result
    # synchronously).  Because we are inside an *async* route we can await.
    await event_bus.publish(EventType.AGENT_UPDATED, {"id": agent_id, "status": "running"})

    # ------------------------------------------------------------------
    # 2. Kick off the actual task run inside its own thread context.
    # ------------------------------------------------------------------
    timestamp = datetime.utcnow().strftime("%Y-%m-%d %H:%M:%S")
    thread_title = f"Manual Task Run - {timestamp}"
    thread, created = agent_manager.get_or_create_thread(db, title=thread_title)
    if created:
        agent_manager.add_system_message(db, thread)
    # Run the agent's task instructions (non-streaming)
    try:
        result_chunks = agent_manager.process_message(
            db=db,
            thread=thread,
            content=agent.task_instructions,
            stream=False,
        )
        # process_message yields the result, so get the first (and only) chunk
        _ = next(result_chunks, "")  # materialise generator so errors propagate

        # ------------------------------------------------------------------
        # 3. Mark the agent back to *idle*, persist last_run_at, and publish
        #    another update so the dashboard can refresh.
        # ------------------------------------------------------------------
        now = datetime.utcnow()
        crud.update_agent(db, agent_id, status="idle", last_run_at=now)
        db.commit()

        await event_bus.publish(
            EventType.AGENT_UPDATED,
            {"id": agent_id, "status": "idle", "last_run_at": now.isoformat(), "thread_id": thread.id},
        )

        # The response message is already created in process_message; return
        # the thread so the caller (frontend) can open it if desired.
        return {"thread_id": thread.id}
    except Exception as e:
        logger.error(f"Error running agent task for agent {agent_id}: {str(e)}")
        raise HTTPException(status_code=500, detail=str(e))
