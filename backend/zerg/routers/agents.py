"""Agent routes module."""

import logging
import os
from typing import Any
from typing import List

from dotenv import load_dotenv
from fastapi import APIRouter
from fastapi import Depends
from fastapi import HTTPException
from fastapi import Response
from fastapi import status
from openai import OpenAI
from sqlalchemy.orm import Session

from zerg.crud import crud
from zerg.database import get_db
from zerg.events import EventType
from zerg.events.decorators import publish_event
from zerg.events.event_bus import event_bus  # Local import to prevent circular deps
from zerg.schemas.schemas import Agent
from zerg.schemas.schemas import AgentCreate
from zerg.schemas.schemas import AgentDetails
from zerg.schemas.schemas import AgentUpdate
from zerg.schemas.schemas import MessageCreate
from zerg.schemas.schemas import MessageResponse

load_dotenv()

# Set up logging
logger = logging.getLogger(__name__)

# ------------------------------------------------------------
# Helper validation
# ------------------------------------------------------------


def _validate_model_or_400(model_id: str) -> None:
    """Ensure the supplied model_id exists in the backend model registry.

    Raises HTTPException(400) if the model is unknown.
    """
    from zerg.models_config import MODELS_BY_ID

    if not model_id or model_id.strip() == "":
        raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail="'model' must be a non‑empty string")

    if model_id not in MODELS_BY_ID:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail=f"Unsupported model '{model_id}'. Call /api/models for the list of valid model IDs.",
        )


# Authentication dependency -------------------------------------------------

# NOTE: import placed after validation helper to avoid circular import issues
from zerg.dependencies.auth import get_current_user  # noqa: E402

router = APIRouter(
    tags=["agents"],
    dependencies=[Depends(get_current_user)],
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
    # Validate model against backend registry
    _validate_model_or_400(agent.model)

    return crud.create_agent(
        db=db,
        name=agent.name,
        system_instructions=agent.system_instructions,
        task_instructions=agent.task_instructions,
        model=agent.model,
        schedule=agent.schedule,
        config=agent.config,
    )


@router.get("/{agent_id}", response_model=Agent)
def read_agent(agent_id: int, db: Session = Depends(get_db)):
    """Get a specific agent by ID"""
    db_agent = crud.get_agent(db, agent_id=agent_id)
    if db_agent is None:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Agent not found")
    return db_agent


# ---------------------------------------------------------------------------
# Agent Details (debug) endpoint
# ---------------------------------------------------------------------------


# Use `response_model_exclude_none=True` so that optional fields which were
# *not* requested via the `include` query parameter are omitted entirely from
# the serialised JSON.  This keeps the payload small and ensures we respect
# the contract verified by the test-suite (see tests/test_agent_details.py).
@router.get(
    "/{agent_id}/details",
    response_model=AgentDetails,
    response_model_exclude_none=True,
)
def read_agent_details(
    agent_id: int,
    include: str | None = None,
    db: Session = Depends(get_db),
):
    """Return an *extended* payload for debugging / observability use-cases.

    Query-string ``include`` allows the caller to request heavier sub-resources
    (currently ignored – Phase 1 delivers only the top-level *agent* object).
    """

    # Fetch the agent or 404 early
    db_agent = crud.get_agent(db, agent_id=agent_id)
    if db_agent is None:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Agent not found")

    # Parse include param into a set for later phases (threads, runs, stats)
    include_set: set[str] = set()
    if include:
        include_set = {part.strip().lower() for part in include.split(",") if part.strip()}

    # Build response – only the mandatory `agent` field for now
    details_payload: dict[str, Any] = {"agent": db_agent}

    # Placeholder for forwards-compatibility – fill with empty list/dict so
    # that client JSON keys are stable even before we implement them.
    if "threads" in include_set:
        # Placeholder: actual thread details will be implemented in phase 2
        details_payload["threads"] = []  # type: ignore[assignment]
    if "runs" in include_set:
        # Include latest runs for this agent
        details_payload["runs"] = crud.list_runs(db, agent_id)  # type: ignore[assignment]
    if "stats" in include_set:
        # Placeholder for future aggregated stats
        details_payload["stats"] = {}  # type: ignore[assignment]

    return details_payload


@router.put("/{agent_id}", response_model=Agent)
@publish_event(EventType.AGENT_UPDATED)
async def update_agent(agent_id: int, agent: AgentUpdate, db: Session = Depends(get_db)):
    """Update an agent"""
    # Validate provided model if present
    if agent.model is not None:
        _validate_model_or_400(agent.model)

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
    )
    if db_agent is None:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Agent not found")
    return db_agent


@router.delete("/{agent_id}", status_code=status.HTTP_204_NO_CONTENT)
async def delete_agent(agent_id: int, db: Session = Depends(get_db)):
    """Delete an agent and broadcast an *agent_deleted* event.

    A 204 No Content response is returned to the HTTP caller as per REST
    conventions, while the full agent row (converted to a plain dict) is
    published on the internal EventBus so that the SchedulerService and any
    WebSocket clients can react to the deletion.
    """

    # ------------------------------------------------------------------
    # 1. Fetch the agent row **before** deleting it so we still have its
    #    attributes available for the event payload.
    # ------------------------------------------------------------------
    agent_row = crud.get_agent(db, agent_id=agent_id)
    if agent_row is None:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Agent not found")

    # ------------------------------------------------------------------
    # 2. Perform the actual deletion.  ``crud.delete_agent`` returns a boolean
    #    to indicate success so we can bail out with 404 if the row vanished
    #    between the initial SELECT and the DELETE (highly unlikely but keeps
    #    the semantics consistent with the other endpoints).
    # ------------------------------------------------------------------
    if not crud.delete_agent(db, agent_id=agent_id):
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Agent not found")

    # ------------------------------------------------------------------
    # 3. Publish the *agent_deleted* event – the payload is a lightweight
    #    plain dict containing only the column values.  This avoids the
    #    recursion issues we observed when FastAPI tried to serialise the raw
    #    SQLAlchemy model object (which can embed cyclic relationships).
    # ------------------------------------------------------------------

    event_payload = {column.name: getattr(agent_row, column.name) for column in agent_row.__table__.columns}
    # Remove SQLAlchemy internal state just to be safe (present when "expire_on_commit=False")
    event_payload.pop("_sa_instance_state", None)

    # Fire-and-forget publish – we ``await`` so tests can deterministically
    # observe the event right after the HTTP call returns.
    await event_bus.publish(EventType.AGENT_DELETED, event_payload)

    # ------------------------------------------------------------------
    # 4. Return an *empty* response body with status 204.  Starlette will make
    #    sure no content is sent to the client.
    # ------------------------------------------------------------------
    return Response(status_code=status.HTTP_204_NO_CONTENT)


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
    """Run the agent's main task (task_instructions) in a new thread."""
    # ------------------------------------------------------------------
    # Delegate to the new TaskRunner helper.
    # ------------------------------------------------------------------
    agent = crud.get_agent(db, agent_id)
    if agent is None:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Agent not found")

    # Import locally to avoid circular dependencies at module import time.
    from zerg.services.task_runner import execute_agent_task

    try:
        thread = await execute_agent_task(db, agent, thread_type="manual")
    except ValueError as ve:
        # Validation error translated to 400 for HTTP callers.
        raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail=str(ve)) from ve

    return {"thread_id": thread.id}
