"""
Router for thread-related endpoints.

This module provides API endpoints for creating, retrieving, updating,
and interacting with threads.
"""

import logging
from typing import List
from typing import Optional

from fastapi import APIRouter
from fastapi import Depends
from fastapi import HTTPException
from fastapi import status
from sqlalchemy.orm import Session

from zerg.app.agents import AgentManager
from zerg.app.crud import crud
from zerg.app.database import get_db
from zerg.app.schemas.schemas import Thread
from zerg.app.schemas.schemas import ThreadCreate
from zerg.app.schemas.schemas import ThreadMessageCreate
from zerg.app.schemas.schemas import ThreadMessageResponse
from zerg.app.schemas.schemas import ThreadUpdate

# Import the new topic manager
from zerg.app.websocket.new_manager import topic_manager

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


@router.post("/{thread_id}/messages", response_model=ThreadMessageResponse, status_code=status.HTTP_201_CREATED)
async def create_thread_message(thread_id: int, message: ThreadMessageCreate, db: Session = Depends(get_db)):
    """Add a new message to a thread without processing it"""
    # Check if the thread exists
    db_thread = crud.get_thread(db, thread_id=thread_id)
    if db_thread is None:
        raise HTTPException(status_code=404, detail="Thread not found")

    # Add the message to the database (unprocessed)
    db_message = crud.create_thread_message(
        db=db,
        thread_id=thread_id,
        role=message.role,
        content=message.content,
        tool_calls=message.tool_calls,
        tool_call_id=message.tool_call_id,
        name=message.name,
        processed=False,  # Mark as unprocessed
    )

    # Update the thread timestamp
    crud.update_thread(db, thread_id)

    return db_message


@router.post("/{thread_id}/run")
async def run_thread(thread_id: int, db: Session = Depends(get_db)):
    """Process unprocessed messages in a thread and broadcast the response via WebSocket."""
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

    # Check if there are unprocessed messages
    unprocessed_messages = crud.get_unprocessed_messages(db, thread_id)
    if not unprocessed_messages:
        return {"detail": "No unprocessed messages to run"}

    # Define the WebSocket topic for this thread
    topic = f"thread:{thread_id}"

    # Send stream_start event
    await topic_manager.broadcast_to_topic(topic, {"type": "stream_start", "thread_id": thread_id})

    full_response = ""
    try:
        # Process the message stream and broadcast chunks
        logger.info(f"Starting to process stream for thread {thread_id}...")
        chunk_count = 0
        for chunk in agent_manager.process_message(db=db, thread=db_thread, content=None, stream=True):
            chunk_count += 1
            logger.debug(f"Thread {thread_id} received chunk {chunk_count}: Type={type(chunk)}, Value='{chunk}'")
            if isinstance(chunk, str):  # Assuming chunks are strings
                full_response += chunk
                logger.info(f"Broadcasting chunk {chunk_count} for thread {thread_id}...")
                await topic_manager.broadcast_to_topic(
                    topic, {"type": "stream_chunk", "thread_id": thread_id, "content": chunk}
                )
                logger.info(f"Finished broadcasting chunk {chunk_count} for thread {thread_id}")
            else:
                # Handle potential non-string chunks if necessary (e.g., tool calls)
                logger.warning(
                    f"Thread {thread_id} received non-string chunk {chunk_count}: Type={type(chunk)}, Value={chunk}"
                )

        logger.info(f"Finished processing stream for thread {thread_id}. Total chunks: {chunk_count}")

        # Update the last assistant message with the full content (optional, depending on logic)
        # crud.update_last_assistant_message(db, thread_id, full_response)

        # Send stream_end event
        logger.info(f"Broadcasting stream_end for thread {thread_id}...")
        await topic_manager.broadcast_to_topic(topic, {"type": "stream_end", "thread_id": thread_id})
        logger.info(f"Finished broadcasting stream_end for thread {thread_id}")

        logger.info(f"Successfully processed and streamed response for thread {thread_id}")
        return {"detail": f"Successfully triggered run for thread {thread_id}"}

    except Exception as e:
        logger.error(f"Error processing message for thread {thread_id}: {str(e)}")
        # Send error over WebSocket if possible
        await topic_manager.broadcast_to_topic(topic, {"type": "stream_error", "thread_id": thread_id, "error": str(e)})
        # Also raise HTTP exception for the initial request
        raise HTTPException(status_code=500, detail=f"Error processing message: {str(e)}")
