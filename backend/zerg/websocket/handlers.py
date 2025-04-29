"""WebSocket message handlers for topic-based subscriptions.

This module provides handlers for the new topic-based WebSocket system,
supporting subscription to agent and thread events.
"""

import logging
from typing import Any
from typing import Dict
from typing import List
from typing import Optional

from fastapi.encoders import jsonable_encoder
from pydantic import BaseModel
from sqlalchemy.orm import Session

from zerg.crud import crud
from zerg.schemas.ws_messages import AgentStateMessage
from zerg.schemas.ws_messages import ErrorMessage
from zerg.schemas.ws_messages import MessageType
from zerg.schemas.ws_messages import PingMessage
from zerg.schemas.ws_messages import PongMessage
from zerg.schemas.ws_messages import SendMessageRequest
from zerg.schemas.ws_messages import SubscribeThreadMessage
from zerg.schemas.ws_messages import ThreadMessageData
from zerg.websocket.manager import topic_manager

logger = logging.getLogger(__name__)


class SubscribeMessage(BaseModel):
    """Request to subscribe to one or more topics."""

    type: str = "subscribe"
    topics: List[str]
    message_id: str


class UnsubscribeMessage(BaseModel):
    """Request to unsubscribe from one or more topics."""

    type: str = "unsubscribe"
    topics: List[str]
    message_id: str


async def send_to_client(client_id: str, message: Dict[str, Any]) -> bool:
    """Send a message to a client if they are connected.

    Args:
        client_id: The client ID to send to
        message: The message to send

    Returns:
        bool: True if message was sent, False if client not found
    """
    if client_id not in topic_manager.active_connections:
        return False

    try:
        await topic_manager.active_connections[client_id].send_json(message)
        return True
    except Exception as e:
        logger.error(f"Error sending to client {client_id}: {str(e)}")
        return False


async def send_error(client_id: str, error_msg: str, message_id: Optional[str] = None) -> None:
    """Send an error message to a client.

    Args:
        client_id: The client ID to send to
        error_msg: The error message
        message_id: Optional message ID to correlate with request
    """
    error = ErrorMessage(
        type=MessageType.ERROR,
        error=error_msg,
        message_id=message_id,
    )
    await send_to_client(client_id, error.model_dump())


async def handle_ping(client_id: str, message: Dict[str, Any], _: Session) -> None:
    """Handle ping messages to keep connection alive.

    Args:
        client_id: The client ID that sent the ping
        message: The ping message
        _: Unused database session
    """
    try:
        ping_msg = PingMessage(**message)
        pong = PongMessage(
            type=MessageType.PONG,
            message_id=ping_msg.message_id,
            timestamp=ping_msg.timestamp,
        )
        await send_to_client(client_id, pong.model_dump())
    except Exception as e:
        logger.error(f"Error handling ping: {str(e)}")
        await send_error(client_id, "Failed to process ping")


# Topic subscription handlers for different topic types
async def _subscribe_thread(client_id: str, thread_id: int, message_id: str, db: Session) -> None:
    """Subscribe to a thread and send initial history.

    Args:
        client_id: The client ID subscribing
        thread_id: The thread ID to subscribe to
        message_id: Message ID for correlation
        db: Database session
    """
    try:
        # Validate thread exists
        thread = crud.get_thread(db, thread_id)
        if not thread:
            await send_error(client_id, f"Thread {thread_id} not found", message_id)
            return

        # Subscribe to thread topic
        topic = f"thread:{thread_id}"
        await topic_manager.subscribe_to_topic(client_id, topic)

        # No longer send thread history here. REST endpoint is responsible for initial message load.

    except Exception as e:
        logger.error(f"Error in _subscribe_thread: {str(e)}")
        await send_error(client_id, "Failed to subscribe to thread", message_id)


async def _subscribe_agent(client_id: str, agent_id: int, message_id: str, db: Session) -> None:
    """Subscribe to agent events and send initial state.

    Args:
        client_id: The client ID subscribing
        agent_id: The agent ID to subscribe to
        message_id: Message ID for correlation
        db: Database session
    """
    try:
        # Validate agent exists
        agent = crud.get_agent(db, agent_id)
        if not agent:
            await send_error(client_id, f"Agent {agent_id} not found", message_id)
            return

        # Subscribe to agent topic
        topic = f"agent:{agent_id}"
        await topic_manager.subscribe_to_topic(client_id, topic)

        # Send initial agent state back to the client
        agent_state_msg = AgentStateMessage(
            type=MessageType.AGENT_STATE,
            message_id=message_id,
            data=agent,
        )
        await send_to_client(client_id, jsonable_encoder(agent_state_msg))
        logger.info(f"Sent initial agent_state for agent {agent_id} to client {client_id}")

    except Exception as e:
        logger.error(f"Error in _subscribe_agent: {str(e)}")
        await send_error(client_id, "Failed to subscribe to agent", message_id)


async def handle_subscribe(client_id: str, message: Dict[str, Any], db: Session) -> None:
    """Handle topic subscription requests.

    Args:
        client_id: The client ID that sent the message
        message: The subscription message
        db: Database session
    """
    try:
        subscribe_msg = SubscribeMessage(**message)

        for topic in subscribe_msg.topics:
            try:
                # Parse topic to validate and get initial data if needed
                topic_type, topic_id = topic.split(":", 1)

                if topic_type == "thread":
                    await _subscribe_thread(client_id, int(topic_id), subscribe_msg.message_id, db)
                elif topic_type == "agent":
                    await _subscribe_agent(client_id, int(topic_id), subscribe_msg.message_id, db)
                else:
                    await send_error(
                        client_id,
                        f"Invalid topic format: Unsupported topic type '{topic_type}'",
                        subscribe_msg.message_id,
                    )
            except ValueError as e:
                await send_error(client_id, f"Invalid topic format: {topic}. Error: {str(e)}", subscribe_msg.message_id)

    except ValueError as e:
        await send_error(client_id, f"Invalid topic format: {str(e)}", message.get("message_id", ""))
    except Exception as e:
        logger.error(f"Error handling subscription: {str(e)}")
        await send_error(client_id, "Failed to process subscription", message.get("message_id", ""))


async def handle_unsubscribe(client_id: str, message: Dict[str, Any], _: Session) -> None:
    """Handle topic unsubscription requests.

    Args:
        client_id: The client ID that sent the message
        message: The unsubscription message
        _: Unused database session
    """
    try:
        message_id = message.get("message_id", "")
        for topic in message.get("topics", []):
            await topic_manager.unsubscribe_from_topic(client_id, topic)

        # Send confirmation message back to client
        await send_to_client(
            client_id, {"type": "unsubscribe_success", "message_id": message_id, "topics": message.get("topics", [])}
        )
    except Exception as e:
        logger.error(f"Error handling unsubscribe: {str(e)}")
        await send_error(client_id, "Failed to process unsubscribe", message.get("message_id", ""))


# Message handler dispatcher
MESSAGE_HANDLERS = {
    "ping": handle_ping,
    "subscribe": handle_subscribe,
    "unsubscribe": handle_unsubscribe,
    # Thread‑specific handlers used by higher‑level chat API
    "subscribe_thread": None,  # populated below
    "send_message": None,  # populated below
}


# ---------------------------------------------------------------------------
# Dedicated helpers for chat‑centric message types introduced in
# zerg.schemas.ws_messages.MessageType. They wrap the existing topic
# subscription mechanism so the rest of the system (topic_manager &
# event_bus) continues to operate unchanged.


async def handle_subscribe_thread(client_id: str, message: Dict[str, Any], db: Session) -> None:  # noqa: D401
    """Handle a *subscribe_thread* request.

    This wraps the generic *subscribe* flow for threads. Initial history
    is now provided via the REST endpoint; WebSocket only delivers updates.
    """
    try:
        sub_msg = SubscribeThreadMessage(**message)
        await _subscribe_thread(client_id, sub_msg.thread_id, sub_msg.message_id, db)
    except Exception as e:  # broad catch ensures client gets feedback
        logger.error(f"Error in handle_subscribe_thread: {str(e)}")
        await send_error(client_id, "Failed to subscribe to thread", message.get("message_id"))


async def handle_send_message(client_id: str, message: Dict[str, Any], db: Session) -> None:  # noqa: D401
    """Persist a new message to a thread and broadcast it."""

    # Ensure ThreadMessageData is in scope for outgoing messages
    try:
        send_req = SendMessageRequest(**message)

        # Validate thread exists
        thread = crud.get_thread(db, send_req.thread_id)
        if not thread:
            await send_error(client_id, f"Thread {send_req.thread_id} not found", send_req.message_id)
            return

        # Persist the message
        db_msg = crud.create_thread_message(
            db,
            thread_id=send_req.thread_id,
            role="user",
            content=send_req.content,
            processed=False,
        )

        msg_dict = {
            "id": db_msg.id,
            "thread_id": db_msg.thread_id,
            "role": db_msg.role,
            "content": db_msg.content,
            "timestamp": db_msg.timestamp.isoformat() if db_msg.timestamp else None,
            "processed": db_msg.processed,
        }

        outgoing = ThreadMessageData(thread_id=send_req.thread_id, message=msg_dict, message_id=send_req.message_id)

        topic = f"thread:{send_req.thread_id}"
        await topic_manager.broadcast_to_topic(topic, outgoing.model_dump())

        # We intentionally do **not** publish a secondary
        # ``THREAD_MESSAGE_CREATED`` event here. The freshly‑created
        # ``thread_message`` broadcast above already informs every subscriber
        # of the new content. Emitting an additional event would be redundant
        # and lead to duplicate WebSocket payloads (see chat flow tests).

    except Exception as e:
        logger.error(f"Error in handle_send_message: {str(e)}")
        await send_error(client_id, "Failed to send message", message.get("message_id"))


# Register the chat-specific handlers in the dispatcher
MESSAGE_HANDLERS["subscribe_thread"] = handle_subscribe_thread
MESSAGE_HANDLERS["send_message"] = handle_send_message


async def dispatch_message(client_id: str, message: Dict[str, Any], db: Session) -> None:
    """Dispatch a message to the appropriate handler.

    Args:
        client_id: The client ID that sent the message
        message: The message to dispatch
        db: Database session
    """
    try:
        message_type = message.get("type")
        if message_type in MESSAGE_HANDLERS:
            await MESSAGE_HANDLERS[message_type](client_id, message, db)
        else:
            await send_error(client_id, f"Unknown message type: {message_type}")
    except Exception as e:
        logger.error(f"Error dispatching message: {str(e)}")
        await send_error(client_id, "Failed to process message")


__all__ = ["dispatch_message"]
