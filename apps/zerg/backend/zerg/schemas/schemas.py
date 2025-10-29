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
from pydantic import ConfigDict

from zerg.models.enums import AgentStatus
from zerg.models.enums import RunStatus
from zerg.schemas.workflow import WorkflowData


# Agent schemas
class AgentBase(BaseModel):
    name: str
    system_instructions: str
    task_instructions: str
    model: str
    schedule: Optional[str] = None
    config: Optional[Dict[str, Any]] = None
    last_error: Optional[str] = None
    allowed_tools: Optional[List[str]] = None


class AgentCreate(AgentBase):
    pass


class AgentUpdate(BaseModel):
    name: Optional[str] = None
    system_instructions: Optional[str] = None
    task_instructions: Optional[str] = None
    model: Optional[str] = None
    status: Optional[AgentStatus] = None
    schedule: Optional[str] = None
    config: Optional[Dict[str, Any]] = None
    last_error: Optional[str] = None
    allowed_tools: Optional[List[str]] = None


# ---------------------------------------------------------------------------
# Pydantic 2.x quirk: on some Python versions ForwardRef resolution fails when
# schema is imported *before* all referenced types are defined.  Calling
# `.model_rebuild()` ensures the internal TypeAdapter tree is fully resolved.
# This is a no-op if everything is already valid.
for _m in [AgentCreate]:
    try:
        _m.model_rebuild()
    except Exception:  # pragma: no cover – defensive only
        pass


class AgentMessage(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    id: int
    agent_id: int
    role: str
    content: str
    timestamp: datetime


# ------------------------------------------------------------
# Authentication schemas (Stage 1)
# ------------------------------------------------------------


class UserOut(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    id: int
    email: str
    is_active: bool
    created_at: datetime
    display_name: Optional[str] = None
    avatar_url: Optional[str] = None
    prefs: Optional[Dict[str, Any]] = None
    last_login: Optional[datetime] = None
    role: str = "USER"

    # -------------------- Gmail integration (Phase-C) --------------------
    # Whether the authenticated user already connected their Gmail account.
    # Convenience flag derived from the presence of a refresh-token – exposed
    # so the WASM frontend can enable e-mail trigger creation without an
    # extra round-trip.
    gmail_connected: bool = False


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
    model_config = ConfigDict(from_attributes=True)

    id: int
    thread_id: int
    timestamp: datetime
    created_at: datetime  # For chronological message ordering on frontend
    processed: bool = False
    parent_id: Optional[int] = None
    # Fields for message type and tool display
    message_type: Optional[str] = None
    tool_name: Optional[str] = None


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
    model_config = ConfigDict(from_attributes=True)

    id: int
    agent_id: int
    created_at: datetime
    updated_at: datetime
    messages: List[ThreadMessageResponse] = []


class Agent(AgentBase):
    model_config = ConfigDict(from_attributes=True)

    id: int
    owner_id: int
    owner: Optional[UserOut] = None
    status: str
    created_at: datetime
    updated_at: datetime
    messages: List[AgentMessage] = []
    # Deprecated field removed – scheduling is implied by `schedule`.
    next_run_at: Optional[datetime] = None
    last_run_at: Optional[datetime] = None
    last_error: Optional[str] = None


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
    model_config = ConfigDict(from_attributes=True)

    id: int
    agent_id: int
    role: str
    content: str
    timestamp: datetime


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
    model_config = ConfigDict(from_attributes=True)

    id: int
    secret: str
    created_at: datetime


# ------------------------------------------------------------
# AgentRun output schema (read-only, hence *Out* suffix)
# ------------------------------------------------------------


# RunStatus moved to models.enums for single source of truth


class RunTrigger(str, Enum):
    manual = "manual"
    schedule = "schedule"
    api = "api"


class AgentRunOut(BaseModel):
    model_config = ConfigDict(from_attributes=True)

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


# ------------------------------------------------------------
# Workflow schemas
# ------------------------------------------------------------


class WorkflowBase(BaseModel):
    name: str
    description: Optional[str] = None


class WorkflowCreate(WorkflowBase):
    canvas: Optional[WorkflowData] = None
    template_id: Optional[int] = None  # Optional template to deploy
    template_name: Optional[str] = None  # Optional template name (e.g., "minimal")


class WorkflowUpdate(BaseModel):
    name: Optional[str] = None
    description: Optional[str] = None


class Workflow(WorkflowBase):
    model_config = ConfigDict(from_attributes=True)

    id: int
    owner_id: int
    is_active: bool
    created_at: datetime
    updated_at: datetime
    canvas: WorkflowData


# Template Gallery schemas
# ------------------------------------------------------------


class WorkflowTemplateBase(BaseModel):
    name: str
    description: Optional[str] = None
    category: str
    canvas: WorkflowData
    tags: Optional[List[str]] = []
    preview_image_url: Optional[str] = None


class WorkflowTemplateCreate(WorkflowTemplateBase):
    pass


class WorkflowTemplate(WorkflowTemplateBase):
    model_config = ConfigDict(from_attributes=True)

    id: int
    created_by: int
    is_public: bool
    created_at: datetime
    updated_at: datetime


class TemplateDeployRequest(BaseModel):
    template_id: int
    name: Optional[str] = None  # Override template name if desired
    description: Optional[str] = None  # Override template description if desired
