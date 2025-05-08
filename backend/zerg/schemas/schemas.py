from datetime import datetime
from enum import Enum
from typing import Any
from typing import Dict
from typing import List
from typing import Optional

# ---------------------------------------------------------------------------
# New *Run History* schemas
# ---------------------------------------------------------------------------
from pydantic import BaseModel


# Agent schemas
class AgentBase(BaseModel):
    name: str
    system_instructions: str
    task_instructions: str
    model: str
    schedule: Optional[str] = None
    config: Optional[Dict[str, Any]] = None
    last_error: Optional[str] = None


class AgentCreate(AgentBase):
    pass


class AgentUpdate(BaseModel):
    name: Optional[str] = None
    system_instructions: Optional[str] = None
    task_instructions: Optional[str] = None
    model: Optional[str] = None
    status: Optional[str] = None
    schedule: Optional[str] = None
    config: Optional[Dict[str, Any]] = None
    last_error: Optional[str] = None


class AgentMessage(BaseModel):
    id: int
    agent_id: int
    role: str
    content: str
    timestamp: datetime

    class Config:
        from_attributes = True


# ------------------------------------------------------------
# Authentication schemas (Stage 1)
# ------------------------------------------------------------


class UserOut(BaseModel):
    id: int
    email: str
    is_active: bool
    created_at: datetime
    display_name: Optional[str] = None
    avatar_url: Optional[str] = None
    prefs: Optional[Dict[str, Any]] = None
    last_login: Optional[datetime] = None

    class Config:
        from_attributes = True


# User profile update schema (partial)
class UserUpdate(BaseModel):
    display_name: Optional[str] = None
    avatar_url: Optional[str] = None
    prefs: Optional[Dict[str, Any]] = None


class TokenOut(BaseModel):
    access_token: str
    token_type: str = "bearer"
    expires_in: int  # seconds until expiry


# Thread Message schemas
class ThreadMessageBase(BaseModel):
    role: str
    content: str
    tool_calls: Optional[List[Dict[str, Any]]] = None
    tool_call_id: Optional[str] = None
    name: Optional[str] = None


class ThreadMessageCreate(ThreadMessageBase):
    pass


class ThreadMessageResponse(ThreadMessageBase):
    id: int
    thread_id: int
    timestamp: datetime
    processed: bool = False
    parent_id: Optional[int] = None
    # Fields for message type and tool display
    message_type: Optional[str] = None
    tool_name: Optional[str] = None

    class Config:
        from_attributes = True


# Thread schemas
class ThreadBase(BaseModel):
    title: str
    agent_state: Optional[Dict[str, Any]] = None
    memory_strategy: Optional[str] = "buffer"
    active: Optional[bool] = True
    thread_type: Optional[str] = "chat"  # Types: chat, scheduled, manual


class ThreadCreate(ThreadBase):
    agent_id: int


class ThreadUpdate(BaseModel):
    title: Optional[str] = None
    agent_state: Optional[Dict[str, Any]] = None
    memory_strategy: Optional[str] = None
    active: Optional[bool] = None
    thread_type: Optional[str] = None


class Thread(ThreadBase):
    id: int
    agent_id: int
    created_at: datetime
    updated_at: datetime
    messages: List[ThreadMessageResponse] = []

    class Config:
        from_attributes = True


class Agent(AgentBase):
    id: int
    status: str
    created_at: datetime
    updated_at: datetime
    messages: List[AgentMessage] = []
    # Deprecated field removed – scheduling is implied by `schedule`.
    next_run_at: Optional[datetime] = None
    last_run_at: Optional[datetime] = None
    last_error: Optional[str] = None

    class Config:
        from_attributes = True


# ---------------------------------------------------------------------------
# Agent Details schema (wrapper used by /agents/{id}/details endpoint)
# ---------------------------------------------------------------------------


class AgentDetails(BaseModel):
    """Envelope object returned by the Agent *details* REST endpoint.

    In Phase 1 we only populate the mandatory ``agent`` field.  The optional
    ``threads``, ``runs`` and ``stats`` keys are included so that the response
    shape is forwards-compatible with the richer payloads planned for future
    phases (see *agent_debug_modal_design.md*).
    """

    agent: Agent
    # The following fields will be filled in future phases when the client
    # requests additional includes via the `include` query param.
    threads: Optional[List[Thread]] = None  # noqa: F821 – Thread is declared later in this file
    runs: Optional[List[Any]] = None  # Placeholder for run log entries
    stats: Optional[Dict[str, Any]] = None


# Message schemas
class MessageCreate(BaseModel):
    role: str
    content: str


class MessageResponse(BaseModel):
    id: int
    agent_id: int
    role: str
    content: str
    timestamp: datetime

    class Config:
        from_attributes = True


# ------------------------------------------------------------
# Trigger schemas
# ------------------------------------------------------------


class TriggerBase(BaseModel):
    agent_id: int
    type: str = "webhook"
    # Arbitrary configuration for non-webhook triggers (e.g. email server
    # settings).  For webhook triggers this is usually ``null``.
    config: Optional[Dict[str, Any]] = None


class TriggerCreate(TriggerBase):
    # Secret is generated by the server; client cannot supply it.
    pass


class Trigger(TriggerBase):
    id: int
    secret: str
    created_at: datetime

    class Config:
        from_attributes = True


# ------------------------------------------------------------
# AgentRun output schema (read-only, hence *Out* suffix)
# ------------------------------------------------------------


class RunStatus(str, Enum):
    """Enum-like convenience class for runtime validation.

    Using a plain ``str`` subclass keeps the dependency footprint minimal
    (avoids importing ``enum.Enum`` repeatedly in pydantic JSON serialisation
    hot-paths) while still providing a canonical list of allowed values.
    """

    queued = "queued"
    running = "running"
    success = "success"
    failed = "failed"


class RunTrigger(str, Enum):
    manual = "manual"
    schedule = "schedule"
    api = "api"


class AgentRunOut(BaseModel):
    id: int
    agent_id: int
    thread_id: int
    status: RunStatus
    trigger: RunTrigger
    started_at: Optional[datetime] = None
    finished_at: Optional[datetime] = None
    duration_ms: Optional[int] = None
    total_tokens: Optional[int] = None
    total_cost_usd: Optional[float] = None
    error: Optional[str] = None

    class Config:
        from_attributes = True
