"""Topic-based WebSocket connection manager.

Manages WebSocket connections with topic-based subscriptions and relays
EventBus events to connected clients.  Requires Python ≥ 3.10 so modern Union
syntax (``int | None``) is available – no backwards-compat hacks.
"""

import logging
from typing import Any
from typing import Dict
from typing import Set

from fastapi import WebSocket
from fastapi.encoders import jsonable_encoder

from zerg.events import EventType
from zerg.events import event_bus

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
        # Map client_id -> authenticated user_id (optional)
        self.client_users: Dict[str, int | None] = {}

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

        # Run events
        event_bus.subscribe(EventType.RUN_CREATED, self._handle_run_event)
        event_bus.subscribe(EventType.RUN_UPDATED, self._handle_run_event)

        # User events (e.g., profile updated) – broadcast to dedicated topic
        event_bus.subscribe(EventType.USER_UPDATED, self._handle_user_event)

    async def connect(self, client_id: str, websocket: WebSocket, user_id: int | None = None) -> None:
        """Register a new client connection.

        Args:
            client_id: Unique identifier for the client
            websocket: The client's WebSocket connection
        """
        self.active_connections[client_id] = websocket
        self.client_topics[client_id] = set()
        self.client_users[client_id] = user_id

        # Auto-subscribe the socket to its personal topic so profile updates
        # propagate across tabs/devices without requiring an explicit
        # subscribe message from the client.
        if user_id is not None:
            personal_topic = f"user:{user_id}"
            await self.subscribe_to_topic(client_id, personal_topic)

        logger.info("Client %s connected (user=%s)", client_id, user_id)

    async def disconnect(self, client_id: str) -> None:
        """Remove a client connection and clean up subscriptions.

        Args:
            client_id: The client ID to remove
        """
        if client_id in self.active_connections:
            # Clean user mapping
            self.client_users.pop(client_id, None)
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
        """Broadcast a message to all clients subscribed to a topic.

        Args:
            topic: The topic to broadcast to
            message: The message to broadcast
        """
        # If there are no active subscribers we silently skip to avoid log
        # spam – this situation is perfectly normal when scheduled agents or
        # background jobs emit updates while no browser is connected.
        if topic not in self.topic_subscriptions:
            logger.debug("broadcast_to_topic: no subscribers for topic %s", topic)
            return

        # Track disconnected clients so we can clean up afterwards
        disconnected_clients = []

        for client_id in self.topic_subscriptions[topic]:
            # Remove any subscription pointing to a now‑missing websocket
            if client_id not in self.active_connections:
                disconnected_clients.append(client_id)
                continue

            try:
                await self.active_connections[client_id].send_json(message)
            except Exception:
                # If sending fails, mark the connection as lost and forget the socket
                disconnected_clients.append(client_id)

        # Unsubscribe clients that are no longer connected
        for client_id in disconnected_clients:
            # Remove websocket reference if still present
            self.active_connections.pop(client_id, None)
            self.client_users.pop(client_id, None)
            await self.unsubscribe_from_topic(client_id, topic)

            # Drop mapping entirely if no topics remain
            if client_id in self.client_topics and not self.client_topics[client_id]:
                del self.client_topics[client_id]

    async def _handle_agent_event(self, data: Dict[str, Any]) -> None:
        """Handle agent-related events from the event bus."""
        if "id" not in data:
            return

        agent_id = data["id"]
        topic = f"agent:{agent_id}"
        serialized_data = jsonable_encoder(data)
        await self.broadcast_to_topic(topic, {"type": data.get("event_type", "agent_event"), "data": serialized_data})

    async def _handle_thread_event(self, data: Dict[str, Any]) -> None:
        """Handle thread-related events from the event bus."""
        if "thread_id" not in data:
            return

        thread_id = data["thread_id"]
        topic = f"thread:{thread_id}"
        serialized_data = jsonable_encoder(data)
        await self.broadcast_to_topic(topic, {"type": data.get("event_type", "thread_event"), "data": serialized_data})

    async def _handle_run_event(self, data: Dict[str, Any]) -> None:
        """Forward run events to the *agent:* topic so dashboards update."""
        if "agent_id" not in data:
            return

        agent_id = data["agent_id"]
        topic = f"agent:{agent_id}"
        serialized_data = jsonable_encoder(data)
        await self.broadcast_to_topic(topic, {"type": "run_update", "data": serialized_data})

    async def _handle_user_event(self, data: Dict[str, Any]) -> None:
        """Forward user events to `user:{id}` topic so other tabs update."""
        user_id = data.get("id")
        if user_id is None:
            return

        topic = f"user:{user_id}"
        serialized_data = jsonable_encoder(data)
        await self.broadcast_to_topic(topic, {"type": "user_update", "data": serialized_data})


# Create a global instance of the new connection manager
topic_manager = TopicConnectionManager()

__all__ = ["TopicConnectionManager", "topic_manager"]
