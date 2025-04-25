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
from zerg.app.schemas.ws_messages import StreamChunkMessage
from zerg.app.schemas.ws_messages import StreamEndMessage
from zerg.app.schemas.ws_messages import StreamStartMessage
from zerg.app.websocket.manager import topic_manager

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
    logger.info(f"Creating message in thread {thread_id}: role={message.role}, content={message.content}")

    # First check if thread exists
    if not crud.get_thread(db, thread_id=thread_id):
        logger.warning(f"Thread {thread_id} not found when creating message")
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Thread not found")

    # Create the message (note: by default, processed=False for user messages)
    new_message = crud.create_thread_message(db=db, thread_id=thread_id, role=message.role, content=message.content)
    logger.info(f"Created message with ID {new_message.id} in thread {thread_id}, processed={new_message.processed}")

    return new_message


@router.post("/{thread_id}/run", status_code=status.HTTP_202_ACCEPTED)
async def run_thread(thread_id: int, db: Session = Depends(get_db)):
    """Process any unprocessed messages in the thread and stream back the result."""

    # Validate thread & agent
    thread = crud.get_thread(db, thread_id=thread_id)
    if thread is None:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Thread not found")

    messages = crud.get_unprocessed_messages(db, thread_id=thread_id)
    if not messages:
        return {"status": "No unprocessed messages"}

    agent = crud.get_agent(db, agent_id=thread.agent_id)
    if agent is None:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Agent not found")

    agent_manager = AgentManager(agent)

    topic = f"thread:{thread_id}"

    # Notify clients that a streamed response is starting
    await topic_manager.broadcast_to_topic(topic, StreamStartMessage(thread_id=thread_id).model_dump())

    # Stream the assistant response chunk‑by‑chunk.  We already have the
    # latest user message stored in the DB (`latest_message`).  Passing
    # `content=None` avoids creating a duplicate copy of the same user
    # message inside `AgentManager.process_message`.

    for chunk in agent_manager.process_message(
        db,
        thread,
        content=None,
        stream=True,
        token_stream=True,
    ):
        await topic_manager.broadcast_to_topic(
            topic,
            StreamChunkMessage(thread_id=thread_id, content=chunk).model_dump(),
        )

    # Signal the end of the stream
    await topic_manager.broadcast_to_topic(topic, StreamEndMessage(thread_id=thread_id).model_dump())

    # Mark all consumed messages as processed
    for msg in messages:
        crud.mark_message_processed(db, msg.id)

    return {"status": "ok"}
