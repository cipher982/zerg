"""Generate Hypothesis strategies directly from Pydantic JSON Schema.

This helper exposes a single function ``strategy_for(message_type: str)``
that returns a *hypothesis-jsonschema* strategy producing dictionaries that
conform to the Pydantic model we use for inbound WebSocket payload
validation.  The mapping stays **single-source-of-truth** because we pull the
schema via ``model.model_json_schema()`` rather than duplicating field
definitions here.

Usage
-----
```python
from jsonschema_strategies import strategy_for
st_messages = strategy_for("ping")
```
"""

from __future__ import annotations

from typing import Dict

from hypothesis_jsonschema import from_schema  # type: ignore

# Import *after* hypothesis_jsonschema so optional import errors surface early
from zerg.schemas.ws_messages import PingMessage
from zerg.schemas.ws_messages import SendMessageRequest
from zerg.schemas.ws_messages import SubscribeThreadMessage

# Map of runtime ``type`` → Pydantic model
_MODEL_MAP: Dict[str, type] = {
    "ping": PingMessage,
    "subscribe_thread": SubscribeThreadMessage,
    "send_message": SendMessageRequest,
}


def strategy_for(message_type: str):  # noqa: D401 – helper function
    """Return a Hypothesis strategy for *message_type*.

    Args:
        message_type: The ``type`` field used on the wire.

    Raises:
        KeyError: If *message_type* is not known.
    """

    model = _MODEL_MAP[message_type]
    schema = model.model_json_schema(mode="validation")
    return from_schema(schema)


# Convenience – bulk generator ------------------------------------------------


def strategies_for(*message_types: str):  # noqa: D401 – helper
    """Return list of strategies for the given message types."""

    return [strategy_for(t) for t in message_types]
