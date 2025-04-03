"""WebSocket message handlers for topic-based subscriptions.

This module provides handlers for the new topic-based WebSocket system,
supporting subscription to agent and thread events.
"""

import logging
from typing import Any
from typing import Dict
from typing import List

from pydantic import BaseModel
from sqlalchemy.orm import Session

from zerg.app.crud import crud
from zerg.app.schemas.ws_messages import ErrorMessage
from zerg.app.schemas.ws_messages import MessageType
from zerg.app.schemas.ws_messages import PingMessage
from zerg.app.schemas.ws_messages import PongMessage
from zerg.app.websocket.new_manager import topic_manager

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


async def handle_ping(client_id: str, message: Dict[str, Any], db: Session = None) -> None:
    """Handle ping messages to keep connections alive.

    Args:
        client_id: The client ID that sent the message
        message: The ping message
        db: Database session (not used for ping)
    """
    try:
        ping_msg = PingMessage(**message)
        pong_response = PongMessage(timestamp=ping_msg.timestamp)

        if client_id in topic_manager.active_connections:
            await topic_manager.active_connections[client_id].send_json(pong_response.model_dump())
    except Exception as e:
        logger.error(f"Error handling ping: {str(e)}")


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
            # Parse topic to validate and get initial data if needed
            topic_type, topic_id = topic.split(":", 1)

            if topic_type == "thread":
                # Validate thread exists and send history
                thread_id = int(topic_id)
                thread = crud.get_thread(db, thread_id)
                if not thread:
                    error_msg = ErrorMessage(error=f"Thread {thread_id} not found", message_id=subscribe_msg.message_id)
                    if client_id in topic_manager.active_connections:
                        await topic_manager.active_connections[client_id].send_json(error_msg.model_dump())
                    continue

                # Subscribe to topic
                await topic_manager.subscribe_to_topic(client_id, topic)

                # Send thread history
                messages = crud.get_thread_messages(db, thread_id)
                history_msg = {
                    "type": MessageType.THREAD_HISTORY,
                    "thread_id": thread_id,
                    "messages": [
                        {
                            "id": msg.id,
                            "thread_id": msg.thread_id,
                            "role": msg.role,
                            "content": msg.content,
                            "timestamp": msg.timestamp.isoformat() if msg.timestamp else None,
                            "processed": msg.processed,
                        }
                        for msg in messages
                    ],
                    "message_id": subscribe_msg.message_id,
                }
                if client_id in topic_manager.active_connections:
                    await topic_manager.active_connections[client_id].send_json(history_msg)

            elif topic_type == "agent":
                # Validate agent exists
                agent_id = int(topic_id)
                agent = crud.get_agent(db, agent_id)
                if not agent:
                    error_msg = ErrorMessage(error=f"Agent {agent_id} not found", message_id=subscribe_msg.message_id)
                    if client_id in topic_manager.active_connections:
                        await topic_manager.active_connections[client_id].send_json(error_msg.model_dump())
                    continue

                # Subscribe to topic
                await topic_manager.subscribe_to_topic(client_id, topic)

                # Send current agent state
                agent_msg = {
                    "type": "agent_state",
                    "data": {
                        "id": agent.id,
                        "name": agent.name,
                        "status": agent.status,
                        # Add other relevant agent fields
                    },
                    "message_id": subscribe_msg.message_id,
                }
                if client_id in topic_manager.active_connections:
                    await topic_manager.active_connections[client_id].send_json(agent_msg)

            else:
                error_msg = ErrorMessage(error=f"Invalid topic format: {topic}", message_id=subscribe_msg.message_id)
                if client_id in topic_manager.active_connections:
                    await topic_manager.active_connections[client_id].send_json(error_msg.model_dump())

    except ValueError as e:
        error_msg = ErrorMessage(error=f"Invalid topic format: {str(e)}", message_id=message.get("message_id", ""))
        if client_id in topic_manager.active_connections:
            await topic_manager.active_connections[client_id].send_json(error_msg.model_dump())
    except Exception as e:
        logger.error(f"Error handling subscription: {str(e)}")
        error_msg = ErrorMessage(error="Failed to process subscription", message_id=message.get("message_id", ""))
        if client_id in topic_manager.active_connections:
            await topic_manager.active_connections[client_id].send_json(error_msg.model_dump())


async def handle_unsubscribe(client_id: str, message: Dict[str, Any], db: Session = None) -> None:
    """Handle topic unsubscription requests.

    Args:
        client_id: The client ID that sent the message
        message: The unsubscription message
        db: Database session (not used for unsubscribe)
    """
    try:
        unsubscribe_msg = UnsubscribeMessage(**message)

        for topic in unsubscribe_msg.topics:
            await topic_manager.unsubscribe_from_topic(client_id, topic)

        # Send success confirmation
        success_msg = {"type": "unsubscribe_success", "message_id": unsubscribe_msg.message_id}
        if client_id in topic_manager.active_connections:
            await topic_manager.active_connections[client_id].send_json(success_msg)

    except Exception as e:
        logger.error(f"Error handling unsubscription: {str(e)}")
        error_msg = ErrorMessage(error="Failed to process unsubscription", message_id=message.get("message_id", ""))
        if client_id in topic_manager.active_connections:
            await topic_manager.active_connections[client_id].send_json(error_msg.model_dump())


# Message handler dispatcher
MESSAGE_HANDLERS = {
    "ping": handle_ping,
    "subscribe": handle_subscribe,
    "unsubscribe": handle_unsubscribe,
}


async def dispatch_message_v2(client_id: str, message: Dict[str, Any], db: Session) -> None:
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
            error_msg = ErrorMessage(error=f"Unknown message type: {message_type}")
            if client_id in topic_manager.active_connections:
                await topic_manager.active_connections[client_id].send_json(error_msg.model_dump())
    except Exception as e:
        logger.error(f"Error dispatching message: {str(e)}")
        error_msg = ErrorMessage(error="Failed to process message")
        if client_id in topic_manager.active_connections:
            await topic_manager.active_connections[client_id].send_json(error_msg.model_dump())


__all__ = ["dispatch_message_v2"]
