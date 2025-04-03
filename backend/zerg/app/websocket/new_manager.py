"""Topic-based WebSocket connection manager.

This module provides a new connection manager that supports topic-based subscriptions
and integrates with the event bus for a unified real-time messaging system.
"""

import logging
from typing import Any
from typing import Dict
from typing import Set

from fastapi import WebSocket

from zerg.app.events import EventType
from zerg.app.events import event_bus

logger = logging.getLogger(__name__)


class TopicConnectionManager:
    """Manages WebSocket connections with topic-based subscriptions."""

    def __init__(self):
        """Initialize an empty topic-based connection manager."""
        # Map of client_id to WebSocket connection
        self.active_connections: Dict[str, WebSocket] = {}
        # Map of topic to set of subscribed client_ids
        self.topic_subscriptions: Dict[str, Set[str]] = {}
        # Map of client_id to set of subscribed topics
        self.client_topics: Dict[str, Set[str]] = {}
        # Register for relevant events
        self._setup_event_handlers()

    def _setup_event_handlers(self) -> None:
        """Set up handlers for events we want to broadcast."""
        # Agent events
        event_bus.subscribe(EventType.AGENT_CREATED, self._handle_agent_event)
        event_bus.subscribe(EventType.AGENT_UPDATED, self._handle_agent_event)
        event_bus.subscribe(EventType.AGENT_DELETED, self._handle_agent_event)
        # Thread events
        event_bus.subscribe(EventType.THREAD_CREATED, self._handle_thread_event)
        event_bus.subscribe(EventType.THREAD_UPDATED, self._handle_thread_event)
        event_bus.subscribe(EventType.THREAD_DELETED, self._handle_thread_event)
        event_bus.subscribe(EventType.THREAD_MESSAGE_CREATED, self._handle_thread_event)

    async def connect(self, client_id: str, websocket: WebSocket) -> None:
        """Register a new client connection.

        Args:
            client_id: Unique identifier for the client
            websocket: The client's WebSocket connection
        """
        await websocket.accept()
        self.active_connections[client_id] = websocket
        self.client_topics[client_id] = set()
        logger.info(f"Client {client_id} connected")

    async def disconnect(self, client_id: str) -> None:
        """Remove a client connection and clean up subscriptions.

        Args:
            client_id: The client ID to remove
        """
        if client_id in self.active_connections:
            # Remove from active connections
            del self.active_connections[client_id]

            # Remove from all topic subscriptions
            if client_id in self.client_topics:
                topics = self.client_topics[client_id]
                for topic in topics:
                    if topic in self.topic_subscriptions:
                        self.topic_subscriptions[topic].discard(client_id)
                        # Clean up empty topic subscriptions
                        if not self.topic_subscriptions[topic]:
                            del self.topic_subscriptions[topic]
                del self.client_topics[client_id]

            logger.info(f"Client {client_id} disconnected")

    async def subscribe_to_topic(self, client_id: str, topic: str) -> None:
        """Subscribe a client to a topic.

        Args:
            client_id: The client ID to subscribe
            topic: The topic to subscribe to (e.g., "agent:123", "thread:45")
        """
        if topic not in self.topic_subscriptions:
            self.topic_subscriptions[topic] = set()

        self.topic_subscriptions[topic].add(client_id)
        self.client_topics[client_id].add(topic)
        logger.info(f"Client {client_id} subscribed to topic {topic}")

    async def unsubscribe_from_topic(self, client_id: str, topic: str) -> None:
        """Unsubscribe a client from a topic.

        Args:
            client_id: The client ID to unsubscribe
            topic: The topic to unsubscribe from
        """
        if topic in self.topic_subscriptions:
            self.topic_subscriptions[topic].discard(client_id)
            if not self.topic_subscriptions[topic]:
                del self.topic_subscriptions[topic]

        if client_id in self.client_topics:
            self.client_topics[client_id].discard(topic)

        logger.info(f"Client {client_id} unsubscribed from topic {topic}")

    async def broadcast_to_topic(self, topic: str, message: Dict[str, Any]) -> None:
        """Send a message to all clients subscribed to a topic.

        Args:
            topic: The topic to broadcast to
            message: The message to send
        """
        if topic not in self.topic_subscriptions:
            return

        disconnected = []
        for client_id in self.topic_subscriptions[topic]:
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

    async def _handle_agent_event(self, data: Dict[str, Any]) -> None:
        """Handle agent-related events from the event bus."""
        if "id" not in data:
            return

        agent_id = data["id"]
        topic = f"agent:{agent_id}"
        await self.broadcast_to_topic(topic, {"type": data.get("event_type", "agent_event"), "data": data})

    async def _handle_thread_event(self, data: Dict[str, Any]) -> None:
        """Handle thread-related events from the event bus."""
        if "thread_id" not in data:
            return

        thread_id = data["thread_id"]
        topic = f"thread:{thread_id}"
        await self.broadcast_to_topic(topic, {"type": data.get("event_type", "thread_event"), "data": data})


# Create a global instance of the new connection manager
topic_manager = TopicConnectionManager()

__all__ = ["TopicConnectionManager", "topic_manager"]
