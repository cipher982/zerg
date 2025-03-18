"""WebSocket message handlers.

This module provides handlers for different types of WebSocket messages.
"""

import logging
from typing import Any
from typing import Dict

from sqlalchemy.orm import Session

from zerg.app.crud import crud
from zerg.app.schemas.ws_messages import ErrorMessage
from zerg.app.schemas.ws_messages import MessageType
from zerg.app.schemas.ws_messages import PingMessage
from zerg.app.schemas.ws_messages import PongMessage
from zerg.app.schemas.ws_messages import SendMessageRequest
from zerg.app.schemas.ws_messages import SubscribeThreadMessage
from zerg.app.schemas.ws_messages import ThreadHistoryMessage
from zerg.app.websocket.manager import manager

logger = logging.getLogger(__name__)


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

        if client_id in manager.active_connections:
            await manager.active_connections[client_id].send_json(pong_response.model_dump())
    except Exception as e:
        logger.error(f"Error handling ping: {str(e)}")


async def handle_subscribe_thread(client_id: str, message: Dict[str, Any], db: Session) -> None:
    """Handle thread subscription requests.

    Args:
        client_id: The client ID that sent the message
        message: The subscription message
        db: Database session
    """
    try:
        subscribe_msg = SubscribeThreadMessage(**message)
        thread_id = subscribe_msg.thread_id

        # Validate thread exists
        thread = crud.get_thread(db, thread_id)
        if not thread:
            error_msg = ErrorMessage(error=f"Thread {thread_id} not found", message_id=subscribe_msg.message_id)

            if client_id in manager.active_connections:
                await manager.active_connections[client_id].send_json(error_msg.model_dump())
            return

        # Subscribe client to thread
        await manager.subscribe_to_thread(client_id, thread_id)

        # Get thread history
        messages = crud.get_thread_messages(db, thread_id)

        # Send thread history to client
        history_msg = ThreadHistoryMessage(
            thread_id=thread_id,
            messages=[
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
            message_id=subscribe_msg.message_id,
        )

        if client_id in manager.active_connections:
            await manager.active_connections[client_id].send_json(history_msg.model_dump())

    except Exception as e:
        logger.error(f"Error handling thread subscription: {str(e)}")
        error_msg = ErrorMessage(error=f"Failed to subscribe to thread: {str(e)}")

        if client_id in manager.active_connections:
            await manager.active_connections[client_id].send_json(error_msg.model_dump())


async def handle_send_message(client_id: str, message: Dict[str, Any], db: Session) -> None:
    """Handle requests to send messages to a thread.

    Args:
        client_id: The client ID that sent the message
        message: The message to send
        db: Database session
    """
    try:
        send_msg = SendMessageRequest(**message)
        thread_id = send_msg.thread_id

        # Validate thread exists
        thread = crud.get_thread(db, thread_id)
        if not thread:
            error_msg = ErrorMessage(error=f"Thread {thread_id} not found", message_id=send_msg.message_id)

            if client_id in manager.active_connections:
                await manager.active_connections[client_id].send_json(error_msg.model_dump())
            return

        # Create the message in the database
        thread_message = crud.create_thread_message(
            db,
            thread_id=thread_id,
            role="user",
            content=send_msg.content,
            processed=False,
        )

        # Broadcast the new message to all subscribed clients
        await manager.broadcast_to_thread(
            thread_id,
            {
                "type": MessageType.THREAD_MESSAGE,
                "thread_id": thread_id,
                "message": {
                    "id": thread_message.id,
                    "thread_id": thread_message.thread_id,
                    "role": thread_message.role,
                    "content": thread_message.content,
                    "timestamp": thread_message.timestamp.isoformat() if thread_message.timestamp else None,
                    "processed": thread_message.processed,
                },
                "message_id": send_msg.message_id,
            },
        )

    except Exception as e:
        logger.error(f"Error handling send message: {str(e)}")
        error_msg = ErrorMessage(error=f"Failed to send message: {str(e)}")

        if client_id in manager.active_connections:
            await manager.active_connections[client_id].send_json(error_msg.model_dump())


# Message handler dispatcher
MESSAGE_HANDLERS = {
    MessageType.PING: handle_ping,
    MessageType.SUBSCRIBE_THREAD: handle_subscribe_thread,
    MessageType.SEND_MESSAGE: handle_send_message,
}


async def dispatch_message(client_id: str, message: Dict[str, Any], db: Session) -> None:
    """Dispatch a message to the appropriate handler.

    Args:
        client_id: The client ID that sent the message
        message: The message to dispatch
        db: Database session
    """
    try:
        message_type = message.get("type")

        if not message_type:
            error_msg = ErrorMessage(error="Missing message type")

            if client_id in manager.active_connections:
                await manager.active_connections[client_id].send_json(error_msg.model_dump())
            return

        handler = MESSAGE_HANDLERS.get(message_type)

        if not handler:
            error_msg = ErrorMessage(error=f"Unknown message type: {message_type}")

            if client_id in manager.active_connections:
                await manager.active_connections[client_id].send_json(error_msg.model_dump())
            return

        await handler(client_id, message, db)

    except Exception as e:
        logger.error(f"Error dispatching message: {str(e)}")
        error_msg = ErrorMessage(error=f"Failed to process message: {str(e)}")

        if client_id in manager.active_connections:
            await manager.active_connections[client_id].send_json(error_msg.model_dump())
