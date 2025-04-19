"""Agent routes module."""

import logging
import os
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


@router.post("/{agent_id}/run", status_code=status.HTTP_201_CREATED)
def run_agent(agent_id: int, db: Session = Depends(get_db)):
    """Run an agent to process its messages"""
    # First check if agent exists
    if not crud.get_agent(db, agent_id=agent_id):
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Agent not found")

    # Get unprocessed messages
    messages = crud.get_agent_messages(db, agent_id=agent_id, processed=False)
    if not messages:
        return {"status": "No unprocessed messages"}

    # Process messages through OpenAI
    try:
        response = client.chat.completions.create(
            model="gpt-4",  # TODO: Get from agent config
            messages=[{"role": msg.role, "content": msg.content} for msg in messages],
        )

        # Create response message
        message = MessageCreate(role="assistant", content=response.choices[0].message.content)
        response_msg = crud.create_agent_message(db=db, agent_id=agent_id, role=message.role, content=message.content)

        # Mark messages as processed
        for msg in messages:
            crud.update_message_processed(db, msg.id, True)

        return response_msg

    except Exception as e:
        logger.error(f"Error running agent {agent_id}: {str(e)}")
        raise HTTPException(status_code=500, detail=str(e))
