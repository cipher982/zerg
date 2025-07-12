# AUTO-GENERATED FILE - DO NOT EDIT
# Generated from ws-protocol.yml at 2025-07-11T13:28:20.712836Z
#
# This file contains all WebSocket message types and schemas.
# To update, modify the schema file and run: python scripts/generate-ws-types.py

import time
from enum import Enum
from typing import Any
from typing import Dict
from typing import List
from typing import Literal
from typing import Optional

from pydantic import BaseModel
from pydantic import Field


class Envelope(BaseModel):
    """Unified envelope for all WebSocket messages."""

    v: int = Field(default=1, description="Protocol version")
    type: str = Field(description="Message type identifier")
    topic: str = Field(description="Topic routing string")
    req_id: Optional[str] = Field(default=None, description="Request correlation ID")
    ts: int = Field(description="Timestamp in milliseconds since epoch")
    data: Dict[str, Any] = Field(description="Message payload")

    @classmethod
    def create(
        cls,
        message_type: str,
        topic: str,
        data: Dict[str, Any],
        req_id: Optional[str] = None,
    ) -> "Envelope":
        """Create a new envelope with current timestamp."""
        return cls(
            type=message_type.lower(),
            topic=topic,
            data=data,
            req_id=req_id,
            ts=int(time.time() * 1000),
        )


# Message payload schemas


class AgentRef(BaseModel):
    """Payload for AgentRef messages"""

    id: int


class ThreadRef(BaseModel):
    """Payload for ThreadRef messages"""

    thread_id: int


class UserRef(BaseModel):
    """Payload for UserRef messages"""

    id: int


class ExecutionRef(BaseModel):
    """Payload for ExecutionRef messages"""

    execution_id: int


class RunUpdateData(BaseModel):
    """Payload for RunUpdateData messages"""

    id: int
    agent_id: int
    thread_id: Optional[int] = None
    status: Literal["queued", "running", "success", "failed"]
    trigger: Optional[Literal["manual", "schedule", "api"]] = None
    started_at: Optional[str] = None
    finished_at: Optional[str] = None
    duration_ms: Optional[int] = None
    error: Optional[str] = None


class AgentEventData(BaseModel):
    """Payload for AgentEventData messages"""

    id: int
    status: Optional[str] = None
    last_run_at: Optional[str] = None
    next_run_at: Optional[str] = None
    last_error: Optional[str] = None
    name: Optional[str] = None
    description: Optional[str] = None


class ThreadEventData(BaseModel):
    """Payload for ThreadEventData messages"""

    thread_id: int
    agent_id: Optional[int] = None
    title: Optional[str] = None
    created_at: Optional[str] = None
    updated_at: Optional[str] = None


class ThreadMessageData(BaseModel):
    """Payload for ThreadMessageData messages"""

    thread_id: int
    message: Dict[str, Any]


class StreamStartData(BaseModel):
    """Payload for StreamStartData messages"""

    thread_id: int


class StreamChunkData(BaseModel):
    """Payload for StreamChunkData messages"""

    thread_id: int
    chunk_type: Literal["assistant_token", "assistant_message", "tool_output"]
    content: Optional[str] = None
    tool_name: Optional[str] = None
    tool_call_id: Optional[str] = None


class StreamEndData(BaseModel):
    """Payload for StreamEndData messages"""

    thread_id: int


class AssistantIdData(BaseModel):
    """Payload for AssistantIdData messages"""

    thread_id: int
    message_id: int


class UserUpdateData(BaseModel):
    """Payload for UserUpdateData messages"""

    id: int
    email: Optional[str] = None
    display_name: Optional[str] = None
    avatar_url: Optional[str] = None


class NodeStateData(BaseModel):
    """Payload for NodeStateData messages"""

    execution_id: int
    node_id: str
    status: Literal["running", "success", "failed"]
    output: Optional[Dict[str, Any]] = None
    error: Optional[str] = None


class ExecutionFinishedData(BaseModel):
    """Payload for ExecutionFinishedData messages"""

    execution_id: int
    status: Literal["success", "failed"]
    error: Optional[str] = None
    duration_ms: Optional[int] = None


class NodeLogData(BaseModel):
    """Payload for NodeLogData messages"""

    execution_id: int
    node_id: str
    stream: Literal["stdout", "stderr"]
    text: str


class SubscribeData(BaseModel):
    """Payload for SubscribeData messages"""

    topics: List[str]
    message_id: Optional[str] = None


class UnsubscribeData(BaseModel):
    """Payload for UnsubscribeData messages"""

    topics: List[str]
    message_id: Optional[str] = None


class SendMessageData(BaseModel):
    """Payload for SendMessageData messages"""

    thread_id: int
    content: str
    metadata: Optional[Dict[str, Any]] = None


class PingData(BaseModel):
    """Payload for PingData messages"""

    timestamp: Optional[int] = None


class PongData(BaseModel):
    """Payload for PongData messages"""

    timestamp: Optional[int] = None


class ErrorData(BaseModel):
    """Payload for ErrorData messages"""

    error: str
    details: Optional[Dict[str, Any]] = None


class MessageType(str, Enum):
    """Enumeration of all WebSocket message types."""

    PING = "ping"
    PONG = "pong"
    ERROR = "error"
    SUBSCRIBE = "subscribe"
    UNSUBSCRIBE = "unsubscribe"
    SEND_MESSAGE = "send_message"
    THREAD_MESSAGE = "thread_message"
    STREAM_START = "stream_start"
    STREAM_CHUNK = "stream_chunk"
    STREAM_END = "stream_end"
    ASSISTANT_ID = "assistant_id"
    AGENT_EVENT = "agent_event"
    THREAD_EVENT = "thread_event"
    RUN_UPDATE = "run_update"
    USER_UPDATE = "user_update"
    NODE_STATE = "node_state"
    EXECUTION_FINISHED = "execution_finished"
    NODE_LOG = "node_log"


# Union types for message handling (placeholder for future use)
# IncomingMessage = Union[...]
# OutgoingMessage = Union[...]

# Helper functions


def validate_envelope(data: Dict[str, Any]) -> Envelope:
    """Validate and parse envelope from raw data."""
    return Envelope.model_validate(data)


def create_message(message_type: str, topic: str, data: Dict[str, Any], req_id: Optional[str] = None) -> Envelope:
    """Create a properly formatted message envelope."""
    return Envelope.create(message_type, topic, data, req_id)
