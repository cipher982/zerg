"""Agent routes module."""

import logging
import os
from typing import List

# from zerg.app.websocket import EventType
# from zerg.app.websocket import broadcast_event
# dotenv
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
@publish_event(EventType.AGENT_CREATED)
async def create_agent(agent: AgentCreate, db: Session = Depends(get_db)):
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
    return new_agent


@router.get("/{agent_id}", response_model=Agent)
def read_agent(agent_id: int, db: Session = Depends(get_db)):
    """Get a specific agent by ID"""
    db_agent = crud.get_agent(db, agent_id=agent_id)
    if db_agent is None:
        raise HTTPException(status_code=404, detail="Agent not found")
    return db_agent


@router.put("/{agent_id}", response_model=Agent)
@publish_event(EventType.AGENT_UPDATED)
async def update_agent(agent_id: int, agent: AgentUpdate, db: Session = Depends(get_db)):
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

    return db_agent


@router.delete("/{agent_id}", status_code=status.HTTP_204_NO_CONTENT)
@publish_event(EventType.AGENT_DELETED)
async def delete_agent(agent_id: int, db: Session = Depends(get_db)):
    """Delete an agent"""
    # Get the agent first so we can include it in the event
    db_agent = crud.get_agent(db, agent_id=agent_id)
    if db_agent is None:
        raise HTTPException(status_code=404, detail="Agent not found")

    success = crud.delete_agent(db, agent_id=agent_id)
    if not success:
        raise HTTPException(status_code=404, detail="Agent not found")

    # Return the deleted agent data for the event
    return {"id": agent_id, "name": db_agent.name}


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


# DEPRECATED: Will be removed after frontend migration
# Use thread-based execution instead (/api/threads/{thread_id}/run)
# @router.post("/{agent_id}/run", response_model=Agent)
# async def run_agent(agent_id: int, db: Session = Depends(get_db)):
#     """Trigger an agent to run"""
#     # First check if the agent exists
#     db_agent = crud.get_agent(db, agent_id=agent_id)
#     if db_agent is None:
#         raise HTTPException(status_code=404, detail="Agent not found")
#
#     # Update agent status to "processing"
#     db_agent = crud.update_agent(db, agent_id=agent_id, status="processing")
#
#     # Broadcast agent status change to "processing"
#     await broadcast_event(
#         EventType.AGENT_STATUS_CHANGED,
#         {"agent_id": db_agent.id, "name": db_agent.name, "status": "processing", "action": "run"},
#     )
#
#     try:
#         # Prepare messages for OpenAI
#         messages = []
#
#         # Add system instructions if available
#         if db_agent.system_instructions:
#             messages.append({"role": "system", "content": db_agent.system_instructions})
#
#         # Add task instructions as the user message
#         if db_agent.task_instructions:
#             messages.append({"role": "user", "content": db_agent.task_instructions})
#         else:
#             logger.error(f"Agent {agent_id} has no task_instructions")
#             raise HTTPException(status_code=400, detail="Agent has no task instructions")
#
#         # Make the OpenAI API call
#         response = client.chat.completions.create(
#             model=db_agent.model,
#             messages=messages,
#         )
#
#         # Save response to message history
#         if response.choices and response.choices[0].message:
#             content = response.choices[0].message.content
#             crud.create_agent_message(db, agent_id=agent_id, role="assistant", content=content)
#
#         # Update agent status back to "idle"
#         crud.update_agent(db, agent_id=agent_id, status="idle")
#
#         # Broadcast status change back to "idle"
#         await broadcast_event(
#             EventType.AGENT_STATUS_CHANGED,
#             {"agent_id": db_agent.id, "name": db_agent.name, "status": "idle", "action": "run_complete"},
#         )
#
#     except Exception as e:
#         logger.error(f"Error running agent {agent_id}: {str(e)}")
#
#         # Update agent status to "error"
#         crud.update_agent(db, agent_id=agent_id, status="error")
#
#         # Broadcast error status
#         await broadcast_event(
#             EventType.AGENT_STATUS_CHANGED,
#             {"agent_id": db_agent.id, "name": db_agent.name, "status": "error", "error": str(e),
#         )
#
#     # Return the agent with its current status
#     return db_agent


# DEPRECATED: Will be removed after frontend migration
# Use thread-based WebSocket instead (/api/threads/{thread_id}/ws)
# @router.websocket("/{agent_id}/ws")
# async def agent_websocket(websocket: WebSocket, agent_id: int, db: Session = Depends(get_db)):
#     """WebSocket endpoint for real-time agent communication"""
#     await websocket.accept()
#
#     # Get the agent
#     db_agent = crud.get_agent(db, agent_id=agent_id)
#     if db_agent is None:
#         await websocket.send_json({"type": "error", "error": "Agent not found"})
#         await websocket.close()
#         return
#
#     try:
#         # Update agent status to processing
#         crud.update_agent(db, agent_id=agent_id, status="processing")
#
#         # Broadcast status change
#         await broadcast_event(
#             EventType.AGENT_STATUS_CHANGED,
#             {"agent_id": agent_id, "name": db_agent.name, "status": "processing", "action": "websocket_connected"},
#         )
#
#         # Get user message
#         data = await websocket.receive_text()
#         json_data = json.loads(data)
#
#         # Create message in DB
#         _ = crud.create_agent_message(db=db, agent_id=agent_id, role="user", content=json_data.get("text", ""))
#
#         # Prepare messages for OpenAI
#         messages = []
#         if db_agent.system_instructions:
#             messages.append({"role": "system", "content": db_agent.system_instructions})
#
#         # Add the user message
#         messages.append({"role": "user", "content": json_data.get("text", "")})
#
#         # Stream response from OpenAI
#         response = client.chat.completions.create(
#             model=db_agent.model or "gpt-4o",
#             messages=messages,
#             stream=True,
#         )
#
#         # Return message_id if provided
#         if "message_id" in json_data:
#             await websocket.send_json({"type": "message_id", "message_id": json_data["message_id"]})
#
#         # Collection for full response
#         full_response = ""
#
#         # Stream the response
#         for chunk in response:
#             if chunk.choices and chunk.choices[0].delta.content:
#                 content = chunk.choices[0].delta.content
#                 full_response += content
#                 await websocket.send_json({"type": "chunk", "chunk": content})
#
#         # Create assistant message in DB
#         _ = crud.create_agent_message(db=db, agent_id=agent_id, role="assistant", content=full_response)
#
#         # Update agent status back to idle
#         crud.update_agent(db, agent_id=agent_id, status="idle")
#
#         # Broadcast status change back to idle
#         await broadcast_event(
#             EventType.AGENT_STATUS_CHANGED,
#             {"agent_id": agent_id, "name": db_agent.name, "status": "idle", "action": "processing_complete"},
#         )
#
#         # Signal end of response
#         await websocket.send_json({"type": "done"})
#
#     except Exception as e:
#         logger.error(f"Agent WebSocket error: {str(e)}")
#
#         # Update agent status on error
#         crud.update_agent(db, agent_id=agent_id, status="error")
#
#         # Broadcast error status
#         await broadcast_event(
#             EventType.AGENT_STATUS_CHANGED,
#             {
#                 "agent_id": agent_id,
#                 "name": db_agent.name if db_agent else "Unknown",
#                 "status": "error",
#                 "error": str(e),
#                 "action": "websocket_error",
#             },
#         )
#
#         await websocket.send_json({"type": "error", "error": str(e)})
#
#         # Try to close WebSocket gracefully
#         try:
#             await websocket.close()
#         except Exception:
#             pass
