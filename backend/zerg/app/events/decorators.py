"""Decorators for event handling."""

import functools
from typing import Any
from typing import Callable

from .event_bus import EventType
from .event_bus import event_bus


def publish_event(event_type: EventType):
    """Decorator that publishes an event after a successful function call.

    The decorated function must return a dict or model that can be converted to dict.
    The entire return value will be included in the event data.

    Args:
        event_type: The type of event to publish
    """

    def decorator(func: Callable):
        @functools.wraps(func)
        async def wrapper(*args, **kwargs) -> Any:
            # Call the original function
            result = await func(*args, **kwargs)

            if result is not None:
                # Convert result to dict if it's a model
                if hasattr(result, "__table__"):
                    # For SQLAlchemy models
                    event_data = {column.name: getattr(result, column.name) for column in result.__table__.columns}
                elif hasattr(result, "dict"):
                    # For Pydantic models
                    event_data = result.dict()
                elif hasattr(result, "__dict__"):
                    # For regular objects
                    event_data = result.__dict__
                else:
                    # For dictionaries or other types
                    event_data = result

                # Remove SQLAlchemy internal state if present
                event_data.pop("_sa_instance_state", None)

                # Publish the event
                await event_bus.publish(event_type, event_data)

            return result

        return wrapper

    return decorator
