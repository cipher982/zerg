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

# Set up logging
logger = logging.getLogger(__name__)

router = APIRouter(
    tags=["threads"],
)


@router.get("/", response_model=List[Thread])
@router.get("", response_model=List[Thread])
def read_threads(agent_id: Optional[int] = None, skip: int = 0, limit: int = 100, db: Session = Depends(get_db)):
    """Get all threads, optionally filtered by agent_id"""
    threads = crud.get_threads(db, agent_id=agent_id, skip=skip, limit=limit)
    if not threads:
        return []
    return threads


@router.post("/", response_model=Thread, status_code=status.HTTP_201_CREATED)
@router.post("", response_model=Thread, status_code=status.HTTP_201_CREATED)
def create_thread(thread: ThreadCreate, db: Session = Depends(get_db)):
    """Create a new thread"""
    # First check if agent exists
    if not crud.get_agent(db, agent_id=thread.agent_id):
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Agent not found")

    return crud.create_thread(
        db=db,
        agent_id=thread.agent_id,
        title=thread.title,
        active=thread.active,
        agent_state=thread.agent_state,
        memory_strategy=thread.memory_strategy,
    )


@router.get("/{thread_id}", response_model=Thread)
def read_thread(thread_id: int, db: Session = Depends(get_db)):
    """Get a specific thread by ID"""
    db_thread = crud.get_thread(db, thread_id=thread_id)
    if db_thread is None:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Thread not found")
    return db_thread


@router.put("/{thread_id}", response_model=Thread)
def update_thread(thread_id: int, thread: ThreadUpdate, db: Session = Depends(get_db)):
    """Update a thread"""
    db_thread = crud.update_thread(
        db,
        thread_id=thread_id,
        title=thread.title,
        active=thread.active,
        agent_state=thread.agent_state,
        memory_strategy=thread.memory_strategy,
    )
    if db_thread is None:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Thread not found")
    return db_thread


@router.delete("/{thread_id}", status_code=status.HTTP_204_NO_CONTENT)
def delete_thread(thread_id: int, db: Session = Depends(get_db)):
    """Delete a thread"""
    if not crud.delete_thread(db, thread_id=thread_id):
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Thread not found")
    return None


@router.get("/{thread_id}/messages", response_model=List[ThreadMessageResponse])
def read_thread_messages(thread_id: int, skip: int = 0, limit: int = 100, db: Session = Depends(get_db)):
    """Get all messages for a thread"""
    # First check if thread exists
    if not crud.get_thread(db, thread_id=thread_id):
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Thread not found")

    messages = crud.get_thread_messages(db, thread_id=thread_id, skip=skip, limit=limit)
    if not messages:
        return []
    return messages


@router.post("/{thread_id}/messages", response_model=ThreadMessageResponse, status_code=status.HTTP_201_CREATED)
def create_thread_message(thread_id: int, message: ThreadMessageCreate, db: Session = Depends(get_db)):
    """Create a new message in a thread"""
    # First check if thread exists
    if not crud.get_thread(db, thread_id=thread_id):
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Thread not found")

    return crud.create_thread_message(db=db, thread_id=thread_id, role=message.role, content=message.content)


@router.post("/{thread_id}/run", status_code=status.HTTP_202_ACCEPTED)
def run_thread(thread_id: int, db: Session = Depends(get_db)):
    """Run a thread to process its messages"""
    # First check if thread exists
    thread = crud.get_thread(db, thread_id=thread_id)
    if thread is None:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Thread not found")

    # Get unprocessed messages
    messages = crud.get_unprocessed_messages(db, thread_id=thread_id)
    if not messages:
        return {"status": "No unprocessed messages"}

    # Process messages through OpenAI
    try:
        # Get agent configuration
        agent = crud.get_agent(db, agent_id=thread.agent_id)
        if agent is None:
            raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Agent not found")

        # Initialize the AgentManager
        _ = AgentManager(agent)

        # Create response message
        message = ThreadMessageCreate(role="assistant", content="Processing...")
        response_msg = crud.create_thread_message(
            db=db, thread_id=thread_id, role=message.role, content=message.content
        )

        # Mark messages as processed
        for msg in messages:
            crud.mark_message_processed(db, msg.id)

        return response_msg

    except Exception as e:
        logger.error(f"Error running thread {thread_id}: {str(e)}")
        raise HTTPException(status_code=500, detail=str(e))
