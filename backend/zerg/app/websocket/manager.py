"""WebSocket connection manager.

This module provides a manager for WebSocket connections that handles:
- Client registration and disconnection
- Thread subscriptions
- Message broadcasting
"""

import logging
from typing import Any
from typing import Dict
from typing import Set

from fastapi import WebSocket

logger = logging.getLogger(__name__)


class ConnectionManager:
    """Manages WebSocket connections and message routing."""

    def __init__(self):
        """Initialize an empty connection manager."""
        self.active_connections: Dict[str, WebSocket] = {}
        self.thread_subscriptions: Dict[int, Set[str]] = {}

    async def connect(self, client_id: str, websocket: WebSocket) -> None:
        """Register a new client connection.

        Args:
            client_id: Unique identifier for the client
            websocket: The client's WebSocket connection
        """
        await websocket.accept()
        self.active_connections[client_id] = websocket
        logger.info(f"Client {client_id} connected")

    async def disconnect(self, client_id: str) -> None:
        """Remove a client connection.

        Args:
            client_id: The client ID to remove
        """
        if client_id in self.active_connections:
            del self.active_connections[client_id]

            # Remove client from all thread subscriptions
            for thread_id in list(self.thread_subscriptions.keys()):
                if client_id in self.thread_subscriptions[thread_id]:
                    self.thread_subscriptions[thread_id].remove(client_id)

                # Clean up empty thread subscriptions
                if not self.thread_subscriptions[thread_id]:
                    del self.thread_subscriptions[thread_id]

            logger.info(f"Client {client_id} disconnected")

    async def subscribe_to_thread(self, client_id: str, thread_id: int) -> None:
        """Subscribe a client to messages for a specific thread.

        Args:
            client_id: The client ID to subscribe
            thread_id: The thread ID to subscribe to
        """
        if thread_id not in self.thread_subscriptions:
            self.thread_subscriptions[thread_id] = set()

        self.thread_subscriptions[thread_id].add(client_id)
        logger.info(f"Client {client_id} subscribed to thread {thread_id}")

    async def broadcast_to_thread(self, thread_id: int, message: Dict[str, Any]) -> None:
        """Send a message to all clients subscribed to a thread.

        Args:
            thread_id: The thread ID to broadcast to
            message: The message to send
        """
        if thread_id not in self.thread_subscriptions:
            return

        disconnected = []
        for client_id in self.thread_subscriptions[thread_id]:
            if client_id in self.active_connections:
                try:
                    await self.active_connections[client_id].send_json(message)
                except Exception as e:
                    logger.error(f"Failed to send to client {client_id}: {str(e)}")
                    disconnected.append(client_id)
            else:
                disconnected.append(client_id)

        # Clean up any disconnected clients
        for client_id in disconnected:
            await self.disconnect(client_id)

    async def broadcast_global(self, message: Dict[str, Any]) -> None:
        """Send a message to all connected clients.

        Args:
            message: The message to send
        """
        disconnected = []
        for client_id, websocket in self.active_connections.items():
            try:
                await websocket.send_json(message)
            except Exception as e:
                logger.error(f"Failed to send to client {client_id}: {str(e)}")
                disconnected.append(client_id)

        # Clean up any disconnected clients
        for client_id in disconnected:
            await self.disconnect(client_id)


# Create a global instance of the connection manager
manager = ConnectionManager()
