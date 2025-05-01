"""Event bus implementation for decoupled event handling."""

import logging
from enum import Enum
from typing import Any
from typing import Awaitable
from typing import Callable
from typing import Dict
from typing import Set

logger = logging.getLogger(__name__)


class EventType(str, Enum):
    """Standardized event types for the system."""

    # Agent events
    AGENT_CREATED = "agent_created"
    AGENT_UPDATED = "agent_updated"
    AGENT_DELETED = "agent_deleted"

    # Thread events
    THREAD_CREATED = "thread_created"
    THREAD_UPDATED = "thread_updated"
    THREAD_DELETED = "thread_deleted"
    THREAD_MESSAGE_CREATED = "thread_message_created"

    # Run events (new run history feature)
    RUN_CREATED = "run_created"
    RUN_UPDATED = "run_updated"

    # Trigger events (external webhook or other sources)
    TRIGGER_FIRED = "trigger_fired"

    # System events
    SYSTEM_STATUS = "system_status"
    ERROR = "error"


class EventBus:
    """Central event bus for publishing and subscribing to events."""

    def __init__(self):
        """Initialize an empty event bus."""
        self._subscribers: Dict[EventType, Set[Callable[[Dict[str, Any]], Awaitable[None]]]] = {}

    async def publish(self, event_type: EventType, data: Dict[str, Any]) -> None:
        """Publish an event to all subscribers.

        Args:
            event_type: The type of event being published
            data: Event payload data
        """
        if event_type not in self._subscribers:
            return

        logger.debug(f"Publishing event {event_type} with data: {data}")

        for callback in self._subscribers[event_type]:
            try:
                await callback(data)
            except Exception as e:
                logger.error(f"Error in event handler for {event_type}: {str(e)}")

    def subscribe(self, event_type: EventType, callback: Callable[[Dict[str, Any]], Awaitable[None]]) -> None:
        """Subscribe to an event type.

        Args:
            event_type: The event type to subscribe to
            callback: Async callback function to handle the event
        """
        if event_type not in self._subscribers:
            self._subscribers[event_type] = set()

        self._subscribers[event_type].add(callback)
        logger.debug(f"Added subscriber for event {event_type}")

    def unsubscribe(self, event_type: EventType, callback: Callable[[Dict[str, Any]], Awaitable[None]]) -> None:
        """Unsubscribe from an event type.

        Args:
            event_type: The event type to unsubscribe from
            callback: The callback function to remove
        """
        if event_type in self._subscribers:
            self._subscribers[event_type].discard(callback)
            logger.debug(f"Removed subscriber for event {event_type}")

            # Clean up empty subscriber sets
            if not self._subscribers[event_type]:
                del self._subscribers[event_type]


# Global event bus instance
event_bus = EventBus()
