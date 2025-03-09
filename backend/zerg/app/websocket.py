"""WebSocket utilities for the Zerg application.

This module provides functionality for system-wide event broadcasting.
WebSockets are used only for real-time events, not for data operations.
"""

import logging
from enum import Enum
from typing import Any
from typing import Dict
from typing import List

from fastapi import WebSocket

# Set up logging
logger = logging.getLogger(__name__)

# Connected WebSocket clients for broadcasting
connected_clients: List[WebSocket] = []


class EventType(str, Enum):
    """Standardized event types for the WebSocket system."""

    AGENT_CREATED = "agent_created"
    AGENT_UPDATED = "agent_updated"
    AGENT_DELETED = "agent_deleted"
    AGENT_STATUS_CHANGED = "agent_status_changed"
    SYSTEM_STATUS = "system_status"
    ERROR = "error"


async def broadcast_event(event_type: EventType, data: Dict[str, Any]) -> None:
    """Send an event to all connected WebSocket clients.

    Args:
        event_type: Type of event from the EventType enum
        data: Dictionary of event data to broadcast
    """
    if not connected_clients:
        return  # No clients connected, nothing to do

    message = {"type": event_type, **data}

    # Track clients that failed to receive the message
    disconnected_clients = []

    for client in connected_clients:
        try:
            await client.send_json(message)
        except Exception as e:
            logger.warning(f"Failed to send to client: {str(e)}")
            disconnected_clients.append(client)

    # Clean up any disconnected clients
    for client in disconnected_clients:
        if client in connected_clients:
            connected_clients.remove(client)
            logger.info("Removed disconnected client from broadcast list")
