"""WebSocket message schemas.

This module defines the Pydantic models for WebSocket messages
in the unified WebSocket system.
"""

from enum import Enum
from typing import Any
from typing import Dict
from typing import List
from typing import Optional
from typing import Union

from pydantic import BaseModel


class MessageType(str, Enum):
    """Standardized message types for the WebSocket system."""

    # Connection messages
    CONNECT = "connect"
    DISCONNECT = "disconnect"
    ERROR = "error"
    PING = "ping"
    PONG = "pong"

    # Thread messages
    SUBSCRIBE_THREAD = "subscribe_thread"
    THREAD_HISTORY = "thread_history"
    THREAD_MESSAGE = "thread_message"
    SEND_MESSAGE = "send_message"

    # Streaming messages
    STREAM_START = "stream_start"
    STREAM_CHUNK = "stream_chunk"
    STREAM_END = "stream_end"

    # System events
    SYSTEM_STATUS = "system_status"


class BaseMessage(BaseModel):
    """Base message with common fields for all WebSocket messages."""

    type: MessageType
    message_id: Optional[str] = None


class ErrorMessage(BaseMessage):
    """Error message sent when something goes wrong."""

    type: MessageType = MessageType.ERROR
    error: str
    details: Optional[Dict[str, Any]] = None


class PingMessage(BaseMessage):
    """Client ping to keep connection alive."""

    type: MessageType = MessageType.PING
    timestamp: Optional[int] = None


class PongMessage(BaseMessage):
    """Server response to ping."""

    type: MessageType = MessageType.PONG
    timestamp: Optional[int] = None


class SubscribeThreadMessage(BaseMessage):
    """Request to subscribe to a thread."""

    type: MessageType = MessageType.SUBSCRIBE_THREAD
    thread_id: int


class ThreadHistoryMessage(BaseMessage):
    """Thread history sent in response to subscription."""

    type: MessageType = MessageType.THREAD_HISTORY
    thread_id: int
    messages: List[Dict[str, Any]]


class ThreadMessageData(BaseMessage):
    """Message within a thread."""

    type: MessageType = MessageType.THREAD_MESSAGE
    thread_id: int
    message: Dict[str, Any]


class SendMessageRequest(BaseMessage):
    """Request to send a new message to a thread."""

    type: MessageType = MessageType.SEND_MESSAGE
    thread_id: int
    content: str
    metadata: Optional[Dict[str, Any]] = None


class StreamStartMessage(BaseMessage):
    """Indicates the start of a streamed response."""

    type: MessageType = MessageType.STREAM_START
    thread_id: int


class StreamChunkMessage(BaseMessage):
    """Chunk of a streamed response."""

    type: MessageType = MessageType.STREAM_CHUNK
    thread_id: int
    content: str


class StreamEndMessage(BaseMessage):
    """Indicates the end of a streamed response."""

    type: MessageType = MessageType.STREAM_END
    thread_id: int


# Union type for all possible incoming messages
IncomingMessage = Union[
    PingMessage,
    SubscribeThreadMessage,
    SendMessageRequest,
]

# Union type for all possible outgoing messages
OutgoingMessage = Union[
    ErrorMessage,
    PongMessage,
    ThreadHistoryMessage,
    ThreadMessageData,
    StreamStartMessage,
    StreamChunkMessage,
    StreamEndMessage,
]
