"""New WebSocket routing module.

This module provides a new FastAPI router for WebSocket connections
using the topic-based subscription system.
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
from zerg.app.websocket.new_handlers import dispatch_message
from zerg.app.websocket.new_manager import topic_manager

router = APIRouter(prefix="/api")
logger = logging.getLogger(__name__)


@router.websocket("/ws/v2")
async def websocket_endpoint(websocket: WebSocket, initial_topics: str = None, db: Session = Depends(get_db)):
    """New WebSocket endpoint supporting topic-based subscriptions.

    This endpoint handles all WebSocket connections and routes messages
    to the appropriate handlers based on message type. It supports
    subscribing to multiple topics (agent events, thread events) over
    a single connection.

    Query Parameters:
        initial_topics: Optional comma-separated list of topics to subscribe to
                      immediately upon connection (e.g., "agent:123,thread:45")
    """
    client_id = str(uuid.uuid4())

    try:
        # Accept the connection and register the client
        await topic_manager.connect(client_id, websocket)
        logger.info(f"WebSocket connection established for client {client_id}")

        # Handle initial topic subscriptions if provided
        if initial_topics:
            topics = [t.strip() for t in initial_topics.split(",")]
            subscribe_msg = {
                "type": "subscribe",
                "topics": topics,
                "message_id": f"auto-subscribe-{uuid.uuid4()}",
            }
            await dispatch_message(client_id, subscribe_msg, db)

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
        await topic_manager.disconnect(client_id)

    except Exception as e:
        # Handle other exceptions
        logger.error(f"WebSocket error: {str(e)}")
        try:
            error_msg = ErrorMessage(error="Internal server error")
            await websocket.send_json(error_msg.model_dump())
        except Exception:
            pass
        finally:
            await topic_manager.disconnect(client_id)
