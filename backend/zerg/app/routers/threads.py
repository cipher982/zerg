"""
Router for thread-related endpoints.

This module provides API endpoints for creating, retrieving, updating,
and interacting with threads.
"""

import json
import logging
from typing import List
from typing import Optional

from fastapi import APIRouter
from fastapi import Depends
from fastapi import HTTPException
from fastapi import WebSocket
from fastapi import WebSocketDisconnect
from fastapi import status
from fastapi.responses import StreamingResponse
from sqlalchemy.orm import Session

from zerg.app.agents import AgentManager
from zerg.app.crud import crud
from zerg.app.database import get_db
from zerg.app.schemas.schemas import Thread
from zerg.app.schemas.schemas import ThreadCreate
from zerg.app.schemas.schemas import ThreadMessageCreate
from zerg.app.schemas.schemas import ThreadMessageResponse
from zerg.app.schemas.schemas import ThreadUpdate
from zerg.app.websocket import EventType
from zerg.app.websocket import broadcast_event

# Set up logging
logger = logging.getLogger(__name__)

router = APIRouter(
    prefix="/api/threads",
    tags=["threads"],
)


@router.get("/", response_model=List[Thread])
@router.get("", response_model=List[Thread])
def read_threads(agent_id: Optional[int] = None, skip: int = 0, limit: int = 100, db: Session = Depends(get_db)):
    """Get all threads, optionally filtered by agent_id"""
    threads = crud.get_threads(db, agent_id=agent_id, skip=skip, limit=limit)
    return threads


@router.post("/", response_model=Thread, status_code=status.HTTP_201_CREATED)
@router.post("", response_model=Thread, status_code=status.HTTP_201_CREATED)
async def create_thread(thread: ThreadCreate, db: Session = Depends(get_db)):
    """Create a new thread"""
    # Check if the agent exists
    db_agent = crud.get_agent(db, agent_id=thread.agent_id)
    if db_agent is None:
        raise HTTPException(status_code=404, detail="Agent not found")

    # If setting this as active, deactivate other threads for the agent
    if thread.active:
        active_thread = crud.get_active_thread(db, thread.agent_id)
        if active_thread:
            crud.update_thread(db, active_thread.id, active=False)

    # Create the thread
    new_thread = crud.create_thread(
        db=db,
        agent_id=thread.agent_id,
        title=thread.title,
        active=thread.active,
        agent_state=thread.agent_state,
        memory_strategy=thread.memory_strategy,
    )

    # Add system message if the agent has system instructions
    agent_manager = AgentManager(db_agent)
    agent_manager.add_system_message(db, new_thread)

    # Broadcast event
    try:
        await broadcast_event(
            EventType.CONVERSATION_CREATED,
            {
                "thread_id": new_thread.id,
                "agent_id": new_thread.agent_id,
                "title": new_thread.title,
            },
        )
    except Exception as e:
        logger.error(f"Error broadcasting thread creation: {str(e)}")

    return new_thread


@router.get("/{thread_id}", response_model=Thread)
def read_thread(thread_id: int, db: Session = Depends(get_db)):
    """Get a specific thread by ID"""
    db_thread = crud.get_thread(db, thread_id=thread_id)
    if db_thread is None:
        raise HTTPException(status_code=404, detail="Thread not found")
    return db_thread


@router.put("/{thread_id}", response_model=Thread)
async def update_thread(thread_id: int, thread: ThreadUpdate, db: Session = Depends(get_db)):
    """Update a thread"""
    # Check if the thread exists
    db_thread = crud.get_thread(db, thread_id=thread_id)
    if db_thread is None:
        raise HTTPException(status_code=404, detail="Thread not found")

    # Update the thread
    updated_thread = crud.update_thread(
        db=db,
        thread_id=thread_id,
        title=thread.title,
        active=thread.active,
        agent_state=thread.agent_state,
        memory_strategy=thread.memory_strategy,
    )

    # Broadcast event
    try:
        await broadcast_event(
            EventType.CONVERSATION_UPDATED,
            {
                "thread_id": updated_thread.id,
                "agent_id": updated_thread.agent_id,
                "title": updated_thread.title,
            },
        )
    except Exception as e:
        logger.error(f"Error broadcasting thread update: {str(e)}")

    return updated_thread


@router.delete("/{thread_id}", status_code=status.HTTP_204_NO_CONTENT)
async def delete_thread(thread_id: int, db: Session = Depends(get_db)):
    """Delete a thread"""
    # Check if the thread exists
    db_thread = crud.get_thread(db, thread_id=thread_id)
    if db_thread is None:
        raise HTTPException(status_code=404, detail="Thread not found")

    # Delete the thread
    result = crud.delete_thread(db, thread_id=thread_id)

    # Broadcast event
    try:
        await broadcast_event(
            EventType.CONVERSATION_DELETED,
            {
                "thread_id": thread_id,
                "agent_id": db_thread.agent_id,
            },
        )
    except Exception as e:
        logger.error(f"Error broadcasting thread deletion: {str(e)}")

    if not result:
        raise HTTPException(status_code=500, detail="Failed to delete thread")


@router.get("/{thread_id}/messages", response_model=List[ThreadMessageResponse])
def read_thread_messages(thread_id: int, skip: int = 0, limit: int = 100, db: Session = Depends(get_db)):
    """Get all messages for a specific thread"""
    # Check if the thread exists
    db_thread = crud.get_thread(db, thread_id=thread_id)
    if db_thread is None:
        raise HTTPException(status_code=404, detail="Thread not found")

    messages = crud.get_thread_messages(db, thread_id=thread_id, skip=skip, limit=limit)
    return messages


@router.post("/{thread_id}/messages", status_code=status.HTTP_201_CREATED)
async def create_thread_message(thread_id: int, message: ThreadMessageCreate, db: Session = Depends(get_db)):
    """Process a new message in a thread and get a streaming response"""
    # Check if the thread exists
    db_thread = crud.get_thread(db, thread_id=thread_id)
    if db_thread is None:
        raise HTTPException(status_code=404, detail="Thread not found")

    # Get the agent
    db_agent = crud.get_agent(db, agent_id=db_thread.agent_id)
    if db_agent is None:
        raise HTTPException(status_code=404, detail="Agent not found")

    # Set up the agent manager
    agent_manager = AgentManager(db_agent)

    # Process the message and stream the response
    async def stream_response():
        try:
            for chunk in agent_manager.process_message(db=db, thread=db_thread, content=message.content, stream=True):
                yield chunk
        except Exception as e:
            logger.error(f"Error processing message: {str(e)}")
            yield f"Error: {str(e)}"

    return StreamingResponse(stream_response(), media_type="text/plain")


@router.websocket("/{thread_id}/ws")
async def thread_websocket(websocket: WebSocket, thread_id: int, db: Session = Depends(get_db)):
    """WebSocket endpoint for real-time thread updates"""
    await websocket.accept()
    logger.info(f"WebSocket connection established for thread {thread_id}")

    try:
        # Check if the thread exists
        db_thread = crud.get_thread(db, thread_id=thread_id)
        if db_thread is None:
            await websocket.send_json({"error": "Thread not found"})
            await websocket.close()
            return

        # Get the agent
        db_agent = crud.get_agent(db, agent_id=db_thread.agent_id)
        if db_agent is None:
            await websocket.send_json({"error": "Agent not found"})
            await websocket.close()
            return

        # Set up the agent manager
        agent_manager = AgentManager(db_agent)

        # Send the current messages
        messages = crud.get_thread_messages(db, thread_id=thread_id)
        await websocket.send_json(
            {
                "type": "thread_history",
                "thread_id": thread_id,
                "messages": [
                    {
                        "id": msg.id,
                        "role": msg.role,
                        "content": msg.content,
                        "timestamp": msg.timestamp.isoformat(),
                    }
                    for msg in messages
                ],
            }
        )

        # Listen for messages from the client
        while True:
            data = await websocket.receive_text()
            message_data = json.loads(data)

            if message_data.get("type") == "message":
                user_message = message_data.get("content", "")
                if not user_message:
                    await websocket.send_json({"type": "error", "error": "Message content is required"})
                    continue

                # Process the message and stream the response
                try:
                    # First, add the user message to the database
                    user_db_message = crud.create_thread_message(
                        db=db, thread_id=thread_id, role="user", content=user_message
                    )

                    # Send acknowledgment of the user message
                    await websocket.send_json(
                        {
                            "type": "message_received",
                            "message_id": user_db_message.id,
                            "thread_id": thread_id,
                        }
                    )

                    # Start streaming the response
                    await websocket.send_json({"type": "stream_start"})

                    response_content = ""
                    for chunk in agent_manager.process_message(
                        db=db, thread=db_thread, content=user_message, stream=True
                    ):
                        response_content += chunk
                        await websocket.send_json({"type": "stream_chunk", "content": chunk})

                    # Signal the end of the stream
                    await websocket.send_json({"type": "stream_end"})

                except Exception as e:
                    logger.error(f"Error processing message: {str(e)}")
                    await websocket.send_json({"type": "error", "error": str(e)})

            elif message_data.get("type") == "ping":
                await websocket.send_json({"type": "pong", "timestamp": message_data.get("timestamp", 0)})

    except WebSocketDisconnect:
        logger.info(f"WebSocket connection closed for thread {thread_id}")
    except json.JSONDecodeError:
        logger.warning("Received invalid JSON payload")
        await websocket.send_json({"type": "error", "error": "Invalid JSON payload"})
    except Exception as e:
        logger.error(f"WebSocket error: {str(e)}")
        try:
            await websocket.send_json({"type": "error", "error": "Internal server error"})
        except Exception:
            pass
