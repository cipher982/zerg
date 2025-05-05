from sqlalchemy import JSON
from sqlalchemy import Boolean
from sqlalchemy import Column
from sqlalchemy import DateTime
from sqlalchemy import Float
from sqlalchemy import ForeignKey
from sqlalchemy import Integer
from sqlalchemy import String
from sqlalchemy import Text
from sqlalchemy import UniqueConstraint
from sqlalchemy.orm import relationship
from sqlalchemy.sql import func

from zerg.database import Base

# ---------------------------------------------------------------------------
# Authentication – User table (Stage 1)
# ---------------------------------------------------------------------------


class User(Base):
    """Application user.

    For the MVP we only support Google sign-in, but we leave provider fields
    generic to allow future providers (e.g. GitHub, email, etc.).
    """

    __tablename__ = "users"

    id = Column(Integer, primary_key=True, index=True)

    # OAuth provider details -------------------------------------------------
    provider = Column(String, nullable=True, default="google")
    provider_user_id = Column(String, nullable=True, index=True)

    # Core identity ----------------------------------------------------------
    email = Column(String, unique=True, nullable=False, index=True)
    is_active = Column(Boolean, default=True, nullable=False)

    # -------------------------------------------------------------------
    # Personalisation fields (introduced in *User Personalisation* feature)
    # -------------------------------------------------------------------
    # Optional display name shown in the UI (fallback: e-mail)
    display_name = Column(String, nullable=True)
    # User-supplied avatar URL (fallback: generated initial)
    avatar_url = Column(String, nullable=True)
    # Store arbitrary UI preferences (theme, timezone, etc.)
    prefs = Column(JSON, nullable=True, default={})

    # Login tracking
    last_login = Column(DateTime, nullable=True)

    # Timestamps -------------------------------------------------------------
    created_at = Column(DateTime, server_default=func.now())
    updated_at = Column(DateTime, server_default=func.now(), onupdate=func.now())


class Agent(Base):
    __tablename__ = "agents"

    id = Column(Integer, primary_key=True, index=True)
    name = Column(String, nullable=False)
    status = Column(String, default="idle")
    system_instructions = Column(Text, nullable=False)
    task_instructions = Column(Text, nullable=False)
    schedule = Column(String, nullable=True)  # CRON expression or interval
    model = Column(String, nullable=False)  # Model to use (no default)
    config = Column(JSON, nullable=True)  # Additional configuration as JSON
    # Scheduling metadata
    # Next time this agent is currently expected to run.  Updated by the
    # SchedulerService whenever a cron job is (re)scheduled.
    next_run_at = Column(DateTime, nullable=True)
    # Last time a scheduled (or manual) run actually finished successfully.
    last_run_at = Column(DateTime, nullable=True)
    created_at = Column(DateTime, server_default=func.now())
    updated_at = Column(DateTime, server_default=func.now(), onupdate=func.now())
    # --------------------------------------------------
    # *run_on_schedule* has been removed – the presence of a non-NULL cron string
    # in the *schedule* column now **alone** determines whether the Scheduler
    # service will run the agent.  A NULL / empty schedule means "disabled".
    # --------------------------------------------------
    last_error = Column(Text, nullable=True)  # Store the last error message

    # Define relationship with AgentMessage
    messages = relationship("AgentMessage", back_populates="agent", cascade="all, delete-orphan")
    # Define relationship with Thread
    threads = relationship("Thread", back_populates="agent", cascade="all, delete-orphan")

    # Relationship to execution runs (added in the *Run History* feature).
    runs = relationship("AgentRun", back_populates="agent", cascade="all, delete-orphan")


# ---------------------------------------------------------------------------
# CanvasLayout – persist per-user canvas/UI state (Phase-B)
# ---------------------------------------------------------------------------


class CanvasLayout(Base):
    """Persisted *canvas layout* for a user.

    At the moment every user stores at most **one** layout (keyed by
    ``workspace`` = NULL).  The table is future-proofed for multi-tenant
    scenarios by including an optional *workspace* column.
    """

    __tablename__ = "canvas_layouts"

    # Enforce *one layout per (user, workspace)*.  Workspace is currently
    # always ``NULL`` but the uniqueness constraint makes future multi-tenant
    # work easier and allows us to rely on an atomic *upsert* in the CRUD
    # helper.
    __table_args__ = (
        # NOTE: SQLite supports the ON CONFLICT clause that references this
        # UNIQUE constraint, enabling an atomic UPSERT in
        # ``crud.upsert_canvas_layout``.
        UniqueConstraint("user_id", "workspace", name="uix_user_workspace_layout"),
    )

    id = Column(Integer, primary_key=True)

    # Foreign key to *users* – **NOT NULL**.  A NULL value would break the
    # UNIQUE(user_id, workspace) constraint in SQLite because every row that
    # contains a NULL is considered *distinct*.  That would allow unlimited
    # duplicate layouts for anonymous users which is *never* what we want.
    #
    # For the dev-mode bypass (`AUTH_DISABLED`) the helper in
    # `zerg.dependencies.auth` ensures a deterministic *dev@local* user row
    # is always present so a proper `user_id` exists.
    user_id = Column(Integer, ForeignKey("users.id"), nullable=False)

    # Reserved for a future multi-tenant feature where a user can switch
    # between different *workspaces*.
    workspace = Column(String, nullable=True)

    # Raw JSON blobs coming from the WASM frontend.
    nodes_json = Column(JSON, nullable=False)
    viewport = Column(JSON, nullable=True)

    # Track last update timestamp (creation time is implicit – equals first
    # value of *updated_at*).
    # Let the **database** rather than Python set and update the timestamp so
    # values are consistent across multiple application instances and not
    # subject to clock skew.
    updated_at = Column(
        DateTime,
        server_default=func.now(),
        onupdate=func.now(),
        nullable=False,
    )

    # ORM relationship back to the owning user – one-to-one convenience.
    user = relationship("User", backref="canvas_layout", uselist=False)


class AgentMessage(Base):
    __tablename__ = "agent_messages"

    id = Column(Integer, primary_key=True, index=True)
    agent_id = Column(Integer, ForeignKey("agents.id"))
    role = Column(String, nullable=False)  # "system", "user", "assistant"
    content = Column(Text, nullable=False)
    timestamp = Column(DateTime, server_default=func.now())

    # Define relationship with Agent
    agent = relationship("Agent", back_populates="messages")


class Thread(Base):
    __tablename__ = "agent_threads"

    id = Column(Integer, primary_key=True, index=True)
    agent_id = Column(Integer, ForeignKey("agents.id"))
    title = Column(String, nullable=False)
    active = Column(Boolean, default=True)
    # Store additional metadata like agent state
    agent_state = Column(JSON, nullable=True)
    memory_strategy = Column(String, default="buffer", nullable=True)
    thread_type = Column(String, default="chat", nullable=False)  # Types: chat, scheduled, manual
    created_at = Column(DateTime, server_default=func.now())
    updated_at = Column(DateTime, server_default=func.now(), onupdate=func.now())

    # Define relationship with Agent
    agent = relationship("Agent", back_populates="threads")
    # Define relationship with ThreadMessage
    messages = relationship("ThreadMessage", back_populates="thread", cascade="all, delete-orphan")


# ------------------------------------------------------------
# Triggers
# ------------------------------------------------------------


class Trigger(Base):
    """A trigger that can fire an agent (e.g. via webhook).

    Currently only the *webhook* type is implemented.  Each trigger owns a
    unique secret token that must be supplied when the webhook is invoked.
    """

    __tablename__ = "triggers"

    id = Column(Integer, primary_key=True, index=True)
    agent_id = Column(Integer, ForeignKey("agents.id"), nullable=False)

    # For now we only support webhook triggers but leave room for future
    # extension (e.g. kafka, email, slack, etc.).
    type = Column(String, default="webhook", nullable=False)

    # Shared secret that must accompany incoming webhook calls.  Very simple
    # scheme for now – a random hex string.
    secret = Column(String, nullable=False, unique=True, index=True)

    created_at = Column(DateTime, server_default=func.now())

    # ORM relationships
    agent = relationship("Agent", backref="triggers")


class ThreadMessage(Base):
    __tablename__ = "thread_messages"

    id = Column(Integer, primary_key=True, index=True)
    thread_id = Column(Integer, ForeignKey("agent_threads.id"))
    role = Column(String, nullable=False)  # "system", "user", "assistant", "tool"
    content = Column(Text, nullable=False)
    # Store tool calls and their results
    tool_calls = Column(JSON, nullable=True)
    tool_call_id = Column(String, nullable=True)  # For tool responses
    name = Column(String, nullable=True)  # For tool messages
    timestamp = Column(DateTime, server_default=func.now())
    processed = Column(Boolean, default=False, nullable=False)  # Track if message has been processed by agent
    message_metadata = Column(JSON, nullable=True)  # Store additional metadata
    parent_id = Column(Integer, ForeignKey("thread_messages.id"), nullable=True)

    # Define relationship with Thread
    thread = relationship("Thread", back_populates="messages")


# ---------------------------------------------------------------------------
# AgentRun – lightweight execution telemetry row
# ---------------------------------------------------------------------------


class AgentRun(Base):
    """Represents a single *execution* of an Agent.

    An AgentRun is created whenever an agent task is executed either manually,
    via the scheduler or through an external trigger.  It references the
    underlying *Thread* that captures the chat transcript but keeps
    additional execution-level metadata (status, timing, cost, etc.) that is
    cumbersome to derive from the chat model alone.
    """

    __tablename__ = "agent_runs"

    id = Column(Integer, primary_key=True, index=True)

    # Foreign keys -------------------------------------------------------
    agent_id = Column(Integer, ForeignKey("agents.id"), nullable=False)
    thread_id = Column(Integer, ForeignKey("agent_threads.id"), nullable=False)

    # Lifecycle ----------------------------------------------------------
    status = Column(String, default="queued", nullable=False)  # queued → running → success|failed
    trigger = Column(String, default="manual", nullable=False)  # manual / schedule / api

    # Timing -------------------------------------------------------------
    started_at = Column(DateTime, nullable=True)
    finished_at = Column(DateTime, nullable=True)
    duration_ms = Column(Integer, nullable=True)

    # Usage --------------------------------------------------------------
    total_tokens = Column(Integer, nullable=True)
    total_cost_usd = Column(Float, nullable=True)

    # Failure ------------------------------------------------------------
    error = Column(Text, nullable=True)

    # Relationships ------------------------------------------------------
    agent = relationship("Agent", back_populates="runs")
    thread = relationship("Thread", backref="runs")
