"""WebSocket routing module.

This module provides a FastAPI router for WebSocket connections
using a topic-based subscription system.
"""

import json
import logging
import uuid
from typing import Optional

from fastapi import APIRouter
from fastapi import WebSocket
from fastapi import WebSocketDisconnect
from sqlalchemy.orm import Session
from sqlalchemy.orm import sessionmaker

from zerg.app.database import default_session_factory
from zerg.app.schemas.ws_messages import ErrorMessage
from zerg.app.websocket.handlers import dispatch_message
from zerg.app.websocket.manager import topic_manager

router = APIRouter()
logger = logging.getLogger(__name__)


def get_websocket_session(session_factory: sessionmaker = None) -> Session:
    """Create a new database session for WebSocket handlers.

    Args:
        session_factory: Optional custom session factory to use

    Returns:
        A SQLAlchemy Session object that must be closed by the caller
    """
    factory = session_factory or default_session_factory
    return factory()


@router.websocket("/ws")
async def websocket_endpoint(websocket: WebSocket, initial_topics: Optional[str] = None):
    """WebSocket endpoint supporting topic-based subscriptions.

    Args:
        websocket: The WebSocket connection
        initial_topics: Optional comma-separated list of topics to subscribe to
            immediately upon connection (e.g., "agent:123,thread:45")
    """
    client_id = str(uuid.uuid4())
    logger.info(f"New WebSocket connection attempt from client {client_id}")

    try:
        await websocket.accept()
        await topic_manager.connect(client_id, websocket)
        logger.info(f"WebSocket connection established for client {client_id}")

        # Handle initial topic subscriptions if provided
        if initial_topics:
            # Use our explicit session creation function
            db = get_websocket_session()
            try:
                topics = [t.strip() for t in initial_topics.split(",")]
                subscribe_msg = {
                    "type": "subscribe",
                    "topics": topics,
                    "message_id": f"auto-subscribe-{uuid.uuid4()}",
                }
                await dispatch_message(client_id, subscribe_msg, db)
            finally:
                db.close()

        # Main message loop
        while True:
            try:
                # Get a fresh DB session for each message using our function
                db = get_websocket_session()
                try:
                    raw_data = await websocket.receive_text()
                    data = json.loads(raw_data)
                    await dispatch_message(client_id, data, db)
                finally:
                    db.close()

            except json.JSONDecodeError as e:
                logger.warning(f"Invalid JSON from client {client_id}: {e}")
                await websocket.send_json(ErrorMessage(error="Invalid JSON payload").model_dump())

    except WebSocketDisconnect:
        logger.info(f"WebSocket connection closed for client {client_id}")
    except Exception as e:
        logger.error(f"WebSocket error for client {client_id}: {str(e)}")
        try:
            await websocket.send_json(ErrorMessage(error="Internal server error").model_dump())
        except Exception:
            pass
    finally:
        await topic_manager.disconnect(client_id)
        logger.info(f"Cleaned up connection for client {client_id}")
