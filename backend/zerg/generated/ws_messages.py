# AUTO-GENERATED FILE - DO NOT EDIT
# Generated from ws-protocol-asyncapi.yml at 2025-08-02T09:31:50.550335Z
# Using AsyncAPI 3.0 + Modern Python Code Generation
#
# This file contains strongly-typed WebSocket message definitions.
# To update, modify the schema file and run: python scripts/generate-ws-types-modern.py

import time
from enum import Enum
from typing import Any
from typing import Dict
from typing import List
from typing import Literal
from typing import Optional
from typing import Protocol

import jsonschema
from pydantic import BaseModel
from pydantic import Field


class Envelope(BaseModel):
    """Unified envelope for all WebSocket messages with validation."""

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
        """Create and validate a new envelope."""
        envelope = cls(
            type=message_type.lower(),
            topic=topic,
            data=data,
            req_id=req_id,
            ts=int(time.time() * 1000),
        )
        # Validate on creation for fail-fast behavior
        validate_envelope_fast(envelope.model_dump())
        return envelope

    def model_dump_validated(self) -> Dict[str, Any]:
        """Dump model with runtime validation."""
        data = self.model_dump()
        validate_envelope_fast(data)
        return data


# Message payload schemas


class AgentRef(BaseModel):
    """Payload for AgentRef messages"""

    id: int = Field(ge=1, description="")


class ThreadRef(BaseModel):
    """Payload for ThreadRef messages"""

    thread_id: int = Field(ge=1, description="")


class UserRef(BaseModel):
    """Payload for UserRef messages"""

    id: int = Field(ge=1, description="")


class ExecutionRef(BaseModel):
    """Payload for ExecutionRef messages"""

    execution_id: int = Field(ge=1, description="")


class PingData(BaseModel):
    """Payload for PingData messages"""

    timestamp: Optional[int] = Field(default=None, ge=0, description="")


class PongData(BaseModel):
    """Payload for PongData messages"""

    timestamp: Optional[int] = Field(default=None, ge=0, description="")


class ErrorData(BaseModel):
    """Payload for ErrorData messages"""

    error: str = Field(min_length=1, description="")
    details: Optional[Dict[str, Any]] = None


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

    thread_id: int = Field(ge=1, description="")
    content: str = Field(min_length=1, description="")
    metadata: Optional[Dict[str, Any]] = None


class ThreadMessageData(BaseModel):
    """Payload for ThreadMessageData messages"""

    thread_id: int = Field(ge=1, description="")
    message: Dict[str, Any]


class ThreadEventData(BaseModel):
    """Payload for ThreadEventData messages"""

    thread_id: int = Field(ge=1, description="")
    agent_id: Optional[int] = Field(default=None, ge=1, description="")
    title: Optional[str] = None
    created_at: Optional[str] = None
    updated_at: Optional[str] = None


class StreamStartData(BaseModel):
    """Payload for StreamStartData messages"""

    thread_id: int = Field(ge=1, description="")


class StreamChunkData(BaseModel):
    """Payload for StreamChunkData messages"""

    thread_id: int = Field(ge=1, description="")
    chunk_type: Literal["assistant_token", "assistant_message", "tool_output"]
    content: Optional[str] = None
    tool_name: Optional[str] = None
    tool_call_id: Optional[str] = None
    message_id: Optional[int] = Field(default=None, ge=1, description="")


class StreamEndData(BaseModel):
    """Payload for StreamEndData messages"""

    thread_id: int = Field(ge=1, description="")


class AssistantIdData(BaseModel):
    """Payload for AssistantIdData messages"""

    thread_id: int = Field(ge=1, description="")
    message_id: int = Field(ge=1, description="")


class AgentEventData(BaseModel):
    """Payload for AgentEventData messages"""

    id: int = Field(ge=1, description="")
    status: Optional[str] = None
    last_run_at: Optional[str] = None
    next_run_at: Optional[str] = None
    last_error: Optional[str] = None
    name: Optional[str] = None
    description: Optional[str] = None


class RunUpdateData(BaseModel):
    """Payload for RunUpdateData messages"""

    id: int = Field(ge=1, description="")
    agent_id: int = Field(ge=1, description="")
    thread_id: Optional[int] = Field(default=None, ge=1, description="")
    status: Literal["queued", "running", "success", "failed"]
    trigger: Optional[Literal["manual", "schedule", "api"]] = None
    started_at: Optional[str] = None
    finished_at: Optional[str] = None
    duration_ms: Optional[int] = Field(default=None, ge=0, description="")
    error: Optional[str] = None


class UserUpdateData(BaseModel):
    """Payload for UserUpdateData messages"""

    id: int = Field(ge=1, description="")
    email: Optional[str] = None
    display_name: Optional[str] = None
    avatar_url: Optional[str] = None


class NodeStateData(BaseModel):
    """Payload for NodeStateData messages"""

    execution_id: int = Field(ge=1, description="")
    node_id: str = Field(min_length=1, description="")
    status: Literal["running", "success", "failed"]
    output: Optional[Dict[str, Any]] = None
    error: Optional[str] = None


class ExecutionFinishedData(BaseModel):
    """Payload for ExecutionFinishedData messages"""

    execution_id: int = Field(ge=1, description="")
    status: Literal["success", "failed"]
    error: Optional[str] = None
    duration_ms: Optional[int] = Field(default=None, ge=0, description="")


class NodeLogData(BaseModel):
    """Payload for NodeLogData messages"""

    execution_id: int = Field(ge=1, description="")
    node_id: str = Field(min_length=1, description="")
    stream: Literal["stdout", "stderr"]
    text: str


class MessageType(str, Enum):
    """Enumeration of all WebSocket message types."""

    PING = "ping"
    PONG = "pong"
    ERROR = "error"
    SUBSCRIBE = "subscribe"
    UNSUBSCRIBE = "unsubscribe"
    SEND_MESSAGE = "send_message"
    THREAD_MESSAGE = "thread_message"
    THREAD_EVENT = "thread_event"
    STREAM_START = "stream_start"
    STREAM_CHUNK = "stream_chunk"
    STREAM_END = "stream_end"
    ASSISTANT_ID = "assistant_id"
    AGENT_EVENT = "agent_event"
    RUN_UPDATE = "run_update"
    USER_UPDATE = "user_update"
    NODE_STATE = "node_state"
    EXECUTION_FINISHED = "execution_finished"
    NODE_LOG = "node_log"


# Typed emitter for contract enforcement


class TypedEmitter(Protocol):
    """Protocol for typed WebSocket message emission."""

    async def send_typed(self, topic: str, message_type: MessageType, payload: BaseModel) -> None:
        """Send a typed message with validation."""
        ...


class TypedEmitterImpl:
    """Implementation of typed emitter with runtime validation."""

    def __init__(self, raw_emitter):
        """Initialize with raw broadcast function."""
        self.raw_emitter = raw_emitter

    async def send_typed(self, topic: str, message_type: MessageType, payload: BaseModel) -> None:
        """Send a typed message with full validation."""
        # Validate payload matches expected type for message
        validate_payload_for_message_type(message_type, payload)

        # Create envelope with validation
        envelope = Envelope.create(message_type=message_type.value, topic=topic, data=payload.model_dump())

        # Send via raw emitter
        await self.raw_emitter(topic, envelope.model_dump_validated())


def create_typed_emitter(raw_emitter) -> TypedEmitter:
    """Factory for typed emitter."""
    return TypedEmitterImpl(raw_emitter)


# Fast validation functions


def validate_envelope_fast(data: Dict[str, Any]) -> None:
    """Envelope validation using jsonschema."""
    try:
        jsonschema.validate(data, ENVELOPE_SCHEMA)
    except jsonschema.ValidationError as e:
        from pydantic import ValidationError as PydanticValidationError

        raise PydanticValidationError(f"Envelope validation failed: {e}")


def validate_payload_for_message_type(message_type: MessageType, payload: BaseModel) -> None:
    """Validate payload matches expected type for message."""
    # This will be populated by specific payload type checks
    # TODO: Generate type-specific validation from schema
    pass


# Schema constants for validation
ENVELOPE_SCHEMA = {
    "type": "object",
    "required": ["v", "type", "topic", "ts", "data"],
    "additionalProperties": False,
    "properties": {
        "v": {"type": "integer", "const": 1},
        "type": {"type": "string"},
        "topic": {"type": "string"},
        "req_id": {"type": ["string", "null"]},
        "ts": {"type": "integer"},
        "data": {"type": "object"},
    },
}
