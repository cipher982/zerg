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

# DB/CRUD helpers
from zerg.crud import crud
from zerg.database import get_db
from zerg.managers.agent_runner import AgentRunner
from zerg.schemas.schemas import Thread
from zerg.schemas.schemas import ThreadCreate
from zerg.schemas.schemas import ThreadMessageCreate
from zerg.schemas.schemas import ThreadMessageResponse
from zerg.schemas.schemas import ThreadUpdate
from zerg.schemas.ws_messages import StreamChunkMessage
from zerg.schemas.ws_messages import StreamEndMessage
from zerg.schemas.ws_messages import StreamStartMessage

# New higher-level ThreadService façade
from zerg.services.thread_service import ThreadService
from zerg.websocket.manager import topic_manager

# Set up logging
logger = logging.getLogger(__name__)

router = APIRouter(
    tags=["threads"],
)


@router.get("/", response_model=List[Thread])
@router.get("", response_model=List[Thread])
def read_threads(
    agent_id: Optional[int] = None,
    skip: int = 0,
    limit: int = 100,
    db: Session = Depends(get_db),
):
    """Get all threads, optionally filtered by agent_id"""
    threads = crud.get_threads(db, agent_id=agent_id, skip=skip, limit=limit)
    if not threads:
        return []
    return threads


@router.post("/", response_model=Thread, status_code=status.HTTP_201_CREATED)
@router.post("", response_model=Thread, status_code=status.HTTP_201_CREATED)
def create_thread(thread: ThreadCreate, db: Session = Depends(get_db)):
    """Create a new thread"""
    # Ensure agent exists and fetch row
    agent_row = crud.get_agent(db, agent_id=thread.agent_id)
    if agent_row is None:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Agent not found")

    # Delegate creation to ThreadService so the mandatory system message is
    # inserted atomically.
    created_thread = ThreadService.create_thread_with_system_message(
        db,
        agent_row,
        title=thread.title,
        thread_type=thread.thread_type or "chat",
        active=thread.active,
    )

    # If the request supplied agent_state or memory_strategy we update the
    # thread accordingly (ThreadService currently doesn't take those extras
    # to keep the helper minimal).
    if thread.agent_state or thread.memory_strategy != "buffer":
        _ = crud.update_thread(
            db,
            thread_id=created_thread.id,
            agent_state=thread.agent_state,
            memory_strategy=thread.memory_strategy,
        )

    return created_thread


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

    # Fetch ORM messages and map to response schema including tool metadata
    orm_msgs = crud.get_thread_messages(db, thread_id=thread_id, skip=skip, limit=limit)
    if not orm_msgs:
        return []
    result: List[ThreadMessageResponse] = []
    for m in orm_msgs:
        # Determine message_type based on role
        if m.role == "tool":
            message_type = "tool_output"
            tool_name = m.name
        elif m.role == "assistant":
            message_type = "assistant_message"
            tool_name = None
        elif m.role == "user":
            message_type = "user_message"
            tool_name = None
        else:
            # Fallback to raw role for unknown types
            message_type = f"{m.role}_message"
            tool_name = None
        result.append(
            ThreadMessageResponse(
                id=m.id,
                thread_id=m.thread_id,
                role=m.role,
                content=m.content,
                tool_calls=m.tool_calls,
                tool_call_id=m.tool_call_id,
                name=m.name,
                timestamp=m.timestamp,
                processed=m.processed,
                parent_id=m.parent_id,
                message_type=message_type,
                tool_name=tool_name,
            )
        )
    return result


@router.post(
    "/{thread_id}/messages",
    response_model=ThreadMessageResponse,
    status_code=status.HTTP_201_CREATED,
)
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

    runner = AgentRunner(agent)

    topic = f"thread:{thread_id}"

    # Notify start of (non token) stream
    await topic_manager.broadcast_to_topic(topic, StreamStartMessage(thread_id=thread_id).model_dump())

    # Execute the agent turn – *await* the async runner to get the assistant reply
    created_rows = await runner.run_thread(db, thread)

    # We maintain a single *stream* sequence for the entire agent turn so the
    # frontend can group chunks under one progress indicator.

    for row in created_rows:
        if row.role == "assistant":
            # Skip duplicate full-message chunk when token streaming was active
            if not runner.enable_token_stream:
                await topic_manager.broadcast_to_topic(
                    topic,
                    StreamChunkMessage(
                        thread_id=thread_id,
                        message_id=str(row.id),
                        content=row.content,
                        chunk_type="assistant_message",
                        tool_name=None,
                        tool_call_id=None,
                    ).model_dump(),
                )

        elif row.role == "tool":
            await topic_manager.broadcast_to_topic(
                topic,
                StreamChunkMessage(
                    thread_id=thread_id,
                    message_id=str(row.id) if row.id else None,
                    content=row.content,
                    chunk_type="tool_output",
                    tool_name=getattr(row, "name", None),
                    tool_call_id=getattr(row, "tool_call_id", None),
                ).model_dump(),
            )

    # Close the stream sequence at the end
    await topic_manager.broadcast_to_topic(topic, StreamEndMessage(thread_id=thread_id).model_dump())

    return {"status": "ok"}
