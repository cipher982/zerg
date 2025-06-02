"""Shared *Enum* definitions for SQLAlchemy & Pydantic models.

The Enums inherit from ``str`` so that:

* JSON serialisation remains unchanged (values render as plain strings).
* Equality checks against raw literals (``role == "ADMIN"``) keep working –
  important for backwards compatibility with existing test-suite asserts.
"""

from __future__ import annotations

from enum import Enum


class UserRole(str, Enum):
    USER = "USER"
    ADMIN = "ADMIN"


class AgentStatus(str, Enum):
    IDLE = "idle"
    RUNNING = "running"
    ERROR = "error"
    PROCESSING = "processing"


class RunStatus(str, Enum):
    QUEUED = "queued"
    RUNNING = "running"
    SUCCESS = "success"
    FAILED = "failed"


class RunTrigger(str, Enum):
    MANUAL = "manual"
    SCHEDULE = "schedule"
    API = "api"


class ThreadType(str, Enum):
    CHAT = "chat"
    SCHEDULED = "scheduled"
    MANUAL = "manual"


__all__ = [
    "UserRole",
    "AgentStatus",
    "RunStatus",
    "RunTrigger",
    "ThreadType",
]
