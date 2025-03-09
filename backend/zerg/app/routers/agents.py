import asyncio
import json
import logging
import os
from typing import List

from fastapi import APIRouter
from fastapi import Depends
from fastapi import HTTPException
from fastapi import WebSocket
from fastapi import status
from openai import OpenAI
from sqlalchemy.orm import Session

from zerg.app.crud import crud
from zerg.app.database import get_db
from zerg.app.schemas.schemas import Agent
from zerg.app.schemas.schemas import AgentCreate
from zerg.app.schemas.schemas import AgentUpdate
from zerg.app.schemas.schemas import MessageCreate
from zerg.app.schemas.schemas import MessageResponse
from zerg.app.websocket import EventType
from zerg.app.websocket import broadcast_event

# Set up logging
logger = logging.getLogger(__name__)

router = APIRouter(
    prefix="/api/agents",
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
    return agents


@router.post("/", response_model=Agent, status_code=status.HTTP_201_CREATED)
@router.post("", response_model=Agent, status_code=status.HTTP_201_CREATED)
def create_agent(agent: AgentCreate, db: Session = Depends(get_db)):
    """Create a new agent"""
    # No default handling, require complete data from API calls
    new_agent = crud.create_agent(
        db=db,
        name=agent.name,
        system_instructions=agent.system_instructions,
        task_instructions=agent.task_instructions,
        model=agent.model,
        schedule=agent.schedule,
        config=agent.config,
    )

    # Schedule broadcast about new agent creation
    asyncio.create_task(
        broadcast_event(
            EventType.AGENT_CREATED, {"agent_id": new_agent.id, "name": new_agent.name, "model": new_agent.model}
        )
    )

    return new_agent


@router.get("/{agent_id}", response_model=Agent)
def read_agent(agent_id: int, db: Session = Depends(get_db)):
    """Get a specific agent by ID"""
    db_agent = crud.get_agent(db, agent_id=agent_id)
    if db_agent is None:
        raise HTTPException(status_code=404, detail="Agent not found")
    return db_agent


@router.put("/{agent_id}", response_model=Agent)
def update_agent(agent_id: int, agent: AgentUpdate, db: Session = Depends(get_db)):
    """Update an agent"""
    # Explicit validation
    if agent_id is None:
        raise HTTPException(status_code=400, detail="Agent ID is required")

    # Get all fields from the update object that are not None
    update_data = {k: v for k, v in agent.dict().items() if v is not None}

    # If we have nothing to update, return error
    if not update_data:
        raise HTTPException(status_code=400, detail="No valid update data provided")

    db_agent = crud.update_agent(
        db,
        agent_id=agent_id,
        name=agent.name,
        system_instructions=agent.system_instructions,
        task_instructions=agent.task_instructions,
        model=agent.model,
        status=agent.status,
        schedule=agent.schedule,
        config=agent.config,
    )
    if db_agent is None:
        raise HTTPException(status_code=404, detail="Agent not found")

    # Schedule broadcast about agent update
    try:
        asyncio.create_task(
            broadcast_event(
                EventType.AGENT_UPDATED, {"agent_id": db_agent.id, "name": db_agent.name, "status": db_agent.status}
            )
        )
    except Exception as e:
        logger.error(f"Error broadcasting agent update: {str(e)}")

    return db_agent


@router.delete("/{agent_id}", status_code=status.HTTP_204_NO_CONTENT)
def delete_agent(agent_id: int, db: Session = Depends(get_db)):
    """Delete an agent"""
    success = crud.delete_agent(db, agent_id=agent_id)
    if not success:
        raise HTTPException(status_code=404, detail="Agent not found")

    # Schedule broadcast about agent deletion
    try:
        asyncio.create_task(broadcast_event(EventType.AGENT_DELETED, {"agent_id": agent_id}))
    except Exception as e:
        logger.error(f"Error broadcasting agent deletion: {str(e)}")

    return None


# Agent messages endpoints
@router.get("/{agent_id}/messages", response_model=List[MessageResponse])
def read_agent_messages(agent_id: int, skip: int = 0, limit: int = 100, db: Session = Depends(get_db)):
    """Get all messages for a specific agent"""
    # First check if the agent exists
    db_agent = crud.get_agent(db, agent_id=agent_id)
    if db_agent is None:
        raise HTTPException(status_code=404, detail="Agent not found")

    # Get messages for the agent
    messages = crud.get_agent_messages(db, agent_id=agent_id, skip=skip, limit=limit)
    return messages


@router.post("/{agent_id}/messages", response_model=MessageResponse, status_code=status.HTTP_201_CREATED)
def create_agent_message(agent_id: int, message: MessageCreate, db: Session = Depends(get_db)):
    """Create a new message for an agent"""
    # First check if the agent exists
    db_agent = crud.get_agent(db, agent_id=agent_id)
    if db_agent is None:
        raise HTTPException(status_code=404, detail="Agent not found")

    # Create the message
    return crud.create_agent_message(db=db, agent_id=agent_id, role=message.role, content=message.content)


@router.post("/{agent_id}/run", response_model=Agent)
def run_agent(agent_id: int, db: Session = Depends(get_db)):
    """Trigger an agent to run"""
    # First check if the agent exists
    db_agent = crud.get_agent(db, agent_id=agent_id)
    if db_agent is None:
        raise HTTPException(status_code=404, detail="Agent not found")

    # Update agent status to "processing"
    db_agent = crud.update_agent(db, agent_id=agent_id, status="processing")

    # Schedule broadcast about agent status change
    asyncio.create_task(
        broadcast_event(
            EventType.AGENT_STATUS_CHANGED,
            {"agent_id": db_agent.id, "name": db_agent.name, "status": "processing", "action": "run"},
        )
    )

    # In a real implementation, you'd queue the agent execution task here
    # For now, we'll just update the status to show the endpoint works

    return db_agent


@router.websocket("/{agent_id}/ws")
async def agent_websocket(websocket: WebSocket, agent_id: int, db: Session = Depends(get_db)):
    """WebSocket endpoint for real-time agent communication"""
    await websocket.accept()

    # Get the agent
    db_agent = crud.get_agent(db, agent_id=agent_id)
    if db_agent is None:
        await websocket.send_json({"type": "error", "error": "Agent not found"})
        await websocket.close()
        return

    try:
        # Update agent status to processing
        crud.update_agent(db, agent_id=agent_id, status="processing")

        # Broadcast status change
        await broadcast_event(
            EventType.AGENT_STATUS_CHANGED,
            {"agent_id": agent_id, "name": db_agent.name, "status": "processing", "action": "websocket_connected"},
        )

        # Get user message
        data = await websocket.receive_text()
        json_data = json.loads(data)

        # Create message in DB
        _ = crud.create_agent_message(db=db, agent_id=agent_id, role="user", content=json_data.get("text", ""))

        # Prepare messages for OpenAI
        messages = []
        if db_agent.system_instructions:
            messages.append({"role": "system", "content": db_agent.system_instructions})

        # Add the user message
        messages.append({"role": "user", "content": json_data.get("text", "")})

        # Stream response from OpenAI
        response = client.chat.completions.create(
            model=db_agent.model or "gpt-4o",
            messages=messages,
            stream=True,
        )

        # Return message_id if provided
        if "message_id" in json_data:
            await websocket.send_json({"type": "message_id", "message_id": json_data["message_id"]})

        # Collection for full response
        full_response = ""

        # Stream the response
        for chunk in response:
            if chunk.choices and chunk.choices[0].delta.content:
                content = chunk.choices[0].delta.content
                full_response += content
                await websocket.send_json({"type": "chunk", "chunk": content})

        # Create assistant message in DB
        _ = crud.create_agent_message(db=db, agent_id=agent_id, role="assistant", content=full_response)

        # Update agent status back to idle
        crud.update_agent(db, agent_id=agent_id, status="idle")

        # Broadcast status change back to idle
        await broadcast_event(
            EventType.AGENT_STATUS_CHANGED,
            {"agent_id": agent_id, "name": db_agent.name, "status": "idle", "action": "processing_complete"},
        )

        # Signal end of response
        await websocket.send_json({"type": "done"})

    except Exception as e:
        logger.error(f"Agent WebSocket error: {str(e)}")

        # Update agent status on error
        crud.update_agent(db, agent_id=agent_id, status="error")

        # Broadcast error status
        await broadcast_event(
            EventType.AGENT_STATUS_CHANGED,
            {
                "agent_id": agent_id,
                "name": db_agent.name if db_agent else "Unknown",
                "status": "error",
                "error": str(e),
                "action": "websocket_error",
            },
        )

        await websocket.send_json({"type": "error", "error": str(e)})

        # Try to close WebSocket gracefully
        try:
            await websocket.close()
        except Exception:
            pass
