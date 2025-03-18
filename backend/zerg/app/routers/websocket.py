"""WebSocket routing module.

This module provides the FastAPI router for WebSocket connections.
"""

import json
import logging
import uuid

from fastapi import APIRouter
from fastapi import Depends
from fastapi import WebSocket
from fastapi import WebSocketDisconnect
from sqlalchemy.orm import Session

from zerg.app.database import get_db
from zerg.app.schemas.ws_messages import ErrorMessage
from zerg.app.websocket.handlers import dispatch_message
from zerg.app.websocket.manager import manager

router = APIRouter(prefix="/api")
logger = logging.getLogger(__name__)


@router.websocket("/ws")
async def websocket_endpoint(
    websocket: WebSocket, thread_id: int = None, agent_id: str = None, db: Session = Depends(get_db)
):
    """Main WebSocket endpoint for all real-time communication.

    This endpoint handles all WebSocket connections and routes messages
    to the appropriate handlers based on message type.

    Query Parameters:
        thread_id: Optional thread ID to automatically subscribe to
        agent_id: Optional agent ID for agent-specific connections
    """
    client_id = str(uuid.uuid4())

    try:
        # Accept the connection and register the client
        await manager.connect(client_id, websocket)
        logger.info(f"WebSocket connection established for client {client_id}")

        # If thread_id is provided, automatically subscribe the client to that thread
        if thread_id is not None:
            logger.info(f"Auto-subscribing client {client_id} to thread {thread_id}")
            # Create a subscribe message to handle through the normal dispatch flow
            subscribe_msg = {
                "type": "subscribe_thread",
                "thread_id": thread_id,
                "message_id": f"auto-subscribe-{uuid.uuid4()}",
            }
            await dispatch_message(client_id, subscribe_msg, db)

        # If agent_id is provided, log it for potential future use
        if agent_id is not None:
            logger.info(f"Client {client_id} connected for agent {agent_id}")
            # For now we just log this, but we could use it for agent-specific logic

        # Main message loop
        while True:
            try:
                # Receive JSON data from the client
                raw_data = await websocket.receive_text()
                data = json.loads(raw_data)

                # Dispatch the message to the appropriate handler
                await dispatch_message(client_id, data, db)

            except json.JSONDecodeError:
                logger.warning(f"Invalid JSON from client {client_id}")
                error_msg = ErrorMessage(error="Invalid JSON payload")
                await websocket.send_json(error_msg.model_dump())

    except WebSocketDisconnect:
        # Handle client disconnect
        logger.info(f"WebSocket connection closed for client {client_id}")
        await manager.disconnect(client_id)

    except Exception as e:
        # Handle other exceptions
        logger.error(f"WebSocket error: {str(e)}")
        try:
            error_msg = ErrorMessage(error="Internal server error")
            await websocket.send_json(error_msg.model_dump())
        except Exception as e:
            pass
        finally:
            await manager.disconnect(client_id)
