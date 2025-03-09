from typing import List

from fastapi import APIRouter
from fastapi import Depends
from fastapi import HTTPException
from fastapi import status
from sqlalchemy.orm import Session

from zerg.app import crud
from zerg.app import schemas
from zerg.app.database import get_db

router = APIRouter(
    prefix="/api/agents",
    tags=["agents"],
)


@router.get("/", response_model=List[schemas.Agent])
def read_agents(skip: int = 0, limit: int = 100, db: Session = Depends(get_db)):
    """Get all agents"""
    agents = crud.get_agents(db, skip=skip, limit=limit)
    return agents


@router.post("/", response_model=schemas.Agent, status_code=status.HTTP_201_CREATED)
def create_agent(agent: schemas.AgentCreate, db: Session = Depends(get_db)):
    """Create a new agent"""
    return crud.create_agent(
        db=db,
        name=agent.name,
        instructions=agent.instructions,
        model=agent.model,
        schedule=agent.schedule,
        config=agent.config,
    )


@router.get("/{agent_id}", response_model=schemas.Agent)
def read_agent(agent_id: int, db: Session = Depends(get_db)):
    """Get a specific agent by ID"""
    db_agent = crud.get_agent(db, agent_id=agent_id)
    if db_agent is None:
        raise HTTPException(status_code=404, detail="Agent not found")
    return db_agent


@router.put("/{agent_id}", response_model=schemas.Agent)
def update_agent(agent_id: int, agent: schemas.AgentUpdate, db: Session = Depends(get_db)):
    """Update an agent"""
    db_agent = crud.update_agent(
        db=db,
        agent_id=agent_id,
        name=agent.name,
        instructions=agent.instructions,
        model=agent.model,
        status=agent.status,
        schedule=agent.schedule,
        config=agent.config,
    )
    if db_agent is None:
        raise HTTPException(status_code=404, detail="Agent not found")
    return db_agent


@router.delete("/{agent_id}", status_code=status.HTTP_204_NO_CONTENT)
def delete_agent(agent_id: int, db: Session = Depends(get_db)):
    """Delete an agent"""
    success = crud.delete_agent(db, agent_id=agent_id)
    if not success:
        raise HTTPException(status_code=404, detail="Agent not found")
    return None


# Agent messages endpoints
@router.get("/{agent_id}/messages", response_model=List[schemas.MessageResponse])
def read_agent_messages(agent_id: int, skip: int = 0, limit: int = 100, db: Session = Depends(get_db)):
    """Get all messages for a specific agent"""
    # First check if the agent exists
    db_agent = crud.get_agent(db, agent_id=agent_id)
    if db_agent is None:
        raise HTTPException(status_code=404, detail="Agent not found")

    # Get messages for the agent
    messages = crud.get_agent_messages(db, agent_id=agent_id, skip=skip, limit=limit)
    return messages


@router.post("/{agent_id}/messages", response_model=schemas.MessageResponse, status_code=status.HTTP_201_CREATED)
def create_agent_message(agent_id: int, message: schemas.MessageCreate, db: Session = Depends(get_db)):
    """Create a new message for an agent"""
    # First check if the agent exists
    db_agent = crud.get_agent(db, agent_id=agent_id)
    if db_agent is None:
        raise HTTPException(status_code=404, detail="Agent not found")

    # Create the message
    return crud.create_agent_message(db=db, agent_id=agent_id, role=message.role, content=message.content)


@router.post("/{agent_id}/run", response_model=schemas.Agent)
def run_agent(agent_id: int, db: Session = Depends(get_db)):
    """Trigger an agent to run"""
    # First check if the agent exists
    db_agent = crud.get_agent(db, agent_id=agent_id)
    if db_agent is None:
        raise HTTPException(status_code=404, detail="Agent not found")

    # Update agent status to "processing"
    db_agent = crud.update_agent(db, agent_id=agent_id, status="processing")

    # In a real implementation, you'd queue the agent execution task here
    # For now, we'll just update the status to show the endpoint works

    return db_agent
