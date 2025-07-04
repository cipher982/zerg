"""WebSocket message handlers for topic-based subscriptions.

This module provides handlers for the new topic-based WebSocket system,
supporting subscription to agent and thread events.
"""

import logging
from typing import Any
from typing import Dict
from typing import List
from typing import Optional

from fastapi.encoders import jsonable_encoder
from pydantic import BaseModel
from pydantic import ValidationError
from sqlalchemy.orm import Session

from zerg.crud import crud
from zerg.schemas.schemas import UserOut
from zerg.schemas.ws_messages import AgentStateMessage

# ---------------------------------------------------------------------------
# Message & envelope helpers
# ---------------------------------------------------------------------------
from zerg.schemas.ws_messages import Envelope
from zerg.schemas.ws_messages import ErrorMessage
from zerg.schemas.ws_messages import MessageEnvelopeHelper
from zerg.schemas.ws_messages import MessageType
from zerg.schemas.ws_messages import PingMessage
from zerg.schemas.ws_messages import PongMessage
from zerg.schemas.ws_messages import SendMessageRequest
from zerg.schemas.ws_messages import SubscribeThreadMessage
from zerg.schemas.ws_messages import ThreadMessageData
from zerg.websocket.manager import topic_manager

logger = logging.getLogger(__name__)


class SubscribeMessage(BaseModel):
    """Request to subscribe to one or more topics."""

    type: str = "subscribe"
    topics: List[str]
    message_id: str


class UnsubscribeMessage(BaseModel):
    """Request to unsubscribe from one or more topics."""

    type: str = "unsubscribe"
    topics: List[str]
    message_id: str


async def send_to_client(
    client_id: str,
    message: Dict[str, Any],
    *,
    topic: Optional[str] = None,
) -> bool:
    """Send a message to a client if they are connected.

    Args:
        client_id: The client ID to send to
        message: The message to send

    Returns:
        bool: True if message was sent, False if client not found
    """
    """Low-level helper to push a JSON-serialisable payload to a single client.

    Every outgoing frame **must** follow the unified *Envelope* contract.
    For compatibility, messages that don't already have envelope structure
    are automatically wrapped in envelopes.

    Args:
        client_id: Recipient connection id (uuid4 string)
        message:  Arbitrary JSON-serialisable mapping.  If the mapping lacks
            the mandatory envelope keys (``v``, ``topic``, ``ts``) it will be
            embedded into a new envelope automatically.
        topic:    Optional topic string.  Required when *message* itself does
            not include a topic.  Helpers such as ``_subscribe_agent`` 
            therefore forward their known topic so the wrapper logic can 
            construct a valid envelope.

    Returns:
        True when the frame was queued for sending, False if the *client_id*
        was unknown or the underlying ``send_json`` raised an exception.
    """

    if client_id not in topic_manager.active_connections:
        return False

    # ------------------------------------------------------------------
    # Envelope wrapping
    # ------------------------------------------------------------------

    # Envelope structure is mandatory – wrap any payload that does not yet
    # include the required keys.

    # Use MessageEnvelopeHelper instead of isinstance checks
    try:
        # Extract req_id if available from the original message
        req_id = None
        if isinstance(message, dict):
            req_id = message.get("message_id")

        envelope = MessageEnvelopeHelper.ensure_envelope(message=message, topic=topic, req_id=req_id)

        payload = envelope.model_dump()

        await topic_manager.active_connections[client_id].send_json(payload)  # type: ignore[arg-type]
        return True
    except Exception as e:  # noqa: BLE001 – log & swallow
        logger.error("Error sending to client %s: %s", client_id, e)
        return False


async def send_error(
    client_id: str,
    error_msg: str,
    message_id: Optional[str] = None,
    *,
    close_code: Optional[int] = None,
) -> None:
    """Send an error message to a client.

    Args:
        client_id: The client ID to send to
        error_msg: The error message
        message_id: Optional message ID to correlate with request
    """
    error = ErrorMessage(
        type=MessageType.ERROR,
        error=error_msg,
        message_id=message_id,
    )
    await send_to_client(client_id, error.model_dump(), topic="system")

    # Optionally close the socket with a specific WebSocket close code.  We
    # perform the close *after* sending the error frame so the client learns
    # the reason before the connection drops.
    if close_code is not None and client_id in topic_manager.active_connections:
        try:
            await topic_manager.active_connections[client_id].close(code=close_code)
        except Exception:  # noqa: BLE001 – ignore errors during close
            pass


async def handle_ping(client_id: str, message: Dict[str, Any], _: Session) -> None:
    """Handle ping messages to keep connection alive.

    Args:
        client_id: The client ID that sent the ping
        message: The ping message
        _: Unused database session
    """
    try:
        ping_msg = PingMessage(**message)

        # Build the *raw* pong payload (same shape as before the envelope
        # upgrade) so existing consumers that have not opted in to V2 still
        # work unchanged.
        pong_payload = PongMessage(
            type=MessageType.PONG,
            message_id=ping_msg.message_id,
            timestamp=ping_msg.timestamp,
        ).model_dump()

        # Recompute envelope flag on **every** ping so tests that mutate the
        # environment mid-process pick up the change without clearing
        # ``functools.lru_cache`` used by :pyfunc:`get_settings`.
        # Always wrap in the unified Envelope format.

        # Wrap in protocol-level envelope so the frontend can rely on the
        # unified structure.  We treat all ping/pong traffic as a
        # *system* message which is consistent with the heartbeat logic.
        envelope = Envelope.create(
            message_type="PONG",
            topic="system",
            data=pong_payload,
            req_id=ping_msg.message_id,
        )

        await send_to_client(client_id, envelope.model_dump())

    except Exception as e:
        logger.error("Error handling ping: %s", e)
        await send_error(client_id, "Failed to process ping")


# ---------------------------------------------------------------------------
# Heart-beat response (client → pong)
# ---------------------------------------------------------------------------


async def handle_pong(client_id: str, message: Dict[str, Any], _: Session) -> None:  # noqa: D401
    """Handle *pong* frames sent by clients.

    Simply updates the TopicConnectionManager watchdog so the connection is
    considered alive. No response is sent back to the client.
    """

    try:
        # Validate schema (already done centrally) then record pong.
        topic_manager.record_pong(client_id)
    except Exception as exc:
        logger.debug("Failed to record pong from %s: %s", client_id, exc)


# Topic subscription handlers for different topic types
async def _subscribe_thread(client_id: str, thread_id: int, message_id: str, db: Session) -> None:
    """Subscribe to a thread and send initial history.

    Args:
        client_id: The client ID subscribing
        thread_id: The thread ID to subscribe to
        message_id: Message ID for correlation
        db: Database session
    """
    try:
        # Validate thread exists
        thread = crud.get_thread(db, thread_id)
        if not thread:
            await send_error(client_id, f"Thread {thread_id} not found", message_id)
            return

        # Subscribe to thread topic
        topic = f"thread:{thread_id}"
        await topic_manager.subscribe_to_topic(client_id, topic)

        # No longer send thread history here. REST endpoint is responsible for initial message load.

    except Exception as e:
        logger.error(f"Error in _subscribe_thread: {str(e)}")
        await send_error(client_id, "Failed to subscribe to thread", message_id)


async def _subscribe_agent(client_id: str, agent_id: int, message_id: str, db: Session) -> None:
    """Subscribe to agent events and send initial state.

    Args:
        client_id: The client ID subscribing
        agent_id: The agent ID to subscribe to
        message_id: Message ID for correlation
        db: Database session
    """
    try:
        # Validate agent exists
        agent = crud.get_agent(db, agent_id)
        if not agent:
            await send_error(client_id, f"Agent {agent_id} not found", message_id)
            return

        # Subscribe to agent topic
        topic = f"agent:{agent_id}"
        await topic_manager.subscribe_to_topic(client_id, topic)

        # Send initial agent state back to the client
        agent_state_msg = AgentStateMessage(
            type=MessageType.AGENT_STATE,
            message_id=message_id,
            data=agent,
        )
        await send_to_client(
            client_id,
            jsonable_encoder(agent_state_msg),
            topic=topic,
        )
        logger.info(f"Sent initial agent_state for agent {agent_id} to client {client_id}")

    except Exception as e:
        logger.error(f"Error in _subscribe_agent: {str(e)}")
        await send_error(client_id, "Failed to subscribe to agent", message_id)


# ---------------------------------------------------------------------------
# User subscriptions
# ---------------------------------------------------------------------------


async def _subscribe_user(client_id: str, user_id: int, message_id: str, db: Session) -> None:
    """Subscribe the *client* to updates for the given user.

    The backend currently broadcasts profile changes via the
    ``user:{id}`` topic (see :pyfunc:`zerg.websocket.manager.TopicConnectionManager`).
    However the generic *subscribe* handler previously rejected the "user"
    topic type, causing the frontend to receive *error* frames on page load
    when it attempted to listen for updates of the current profile.

    This helper mirrors ``_subscribe_agent`` and sends the **initial** profile
    snapshot back to the browser so all open tabs start with the same data.
    """

    try:
        # Fast-path: allow "user:0" placeholder subscription which the
        # frontend may emit *before* it knows the real user id.  We silently
        # accept but skip any initial payload so the browser does not spam
        # the console with error frames during startup.
        if user_id <= 0:
            topic = f"user:{user_id}"
            await topic_manager.subscribe_to_topic(client_id, topic)
            return

        # Validate user exists – any non-zero id must be present in the DB.
        user = crud.get_user(db, user_id)
        if user is None:
            await send_error(client_id, f"User {user_id} not found", message_id)
            return

        # Subscribe to dedicated user topic
        topic = f"user:{user_id}"
        await topic_manager.subscribe_to_topic(client_id, topic)

        # Send initial user state – we re-use the *user_update* payload shape
        # already handled by the frontend rather than introducing a new
        # message type.
        user_payload = jsonable_encoder(UserOut.model_validate(user))

        await send_to_client(
            client_id,
            {
                "type": "user_update",
                "message_id": message_id,
                "data": user_payload,
            },
            topic=topic,
        )

        logger.info("Sent initial user_update for user %s to client %s", user_id, client_id)

    except Exception as e:
        logger.error(f"Error in _subscribe_user: {str(e)}")
        await send_error(client_id, "Failed to subscribe to user", message_id)


async def _subscribe_workflow_execution(client_id: str, execution_id: int, message_id: str, db: Session) -> None:
    """Subscribe to workflow execution events.

    Args:
        client_id: The client ID subscribing
        execution_id: The workflow execution ID to subscribe to (can be future execution)
        message_id: Message ID for correlation
        db: Database session
    """
    try:
        # Subscribe to workflow execution topic (execution may not exist yet)
        # This allows subscribing before execution starts
        topic = f"workflow_execution:{execution_id}"
        await topic_manager.subscribe_to_topic(client_id, topic)

        logger.info("Client %s subscribed to workflow execution topic %s", client_id, topic)

        # ------------------------------------------------------------------
        # Send *current* execution status snapshot so late subscribers still
        # receive the final state even when the run already finished.
        # ------------------------------------------------------------------

        try:
            execution = crud.get_workflow_execution(db, execution_id)
        except Exception:
            execution = None

        # Log execution check for debugging
        logger.info(
            "Checking execution %s - found: %s, status: %s",
            execution_id,
            execution is not None,
            execution.status if execution else "None",
        )

        if execution is not None and execution.status in {"success", "failed", "cancelled"}:
            # Prepare payload mirroring the live EXECUTION_FINISHED event so
            # the frontend can reuse the same handler logic.
            duration_ms: Optional[int]
            if execution.started_at and execution.finished_at:
                delta = execution.finished_at - execution.started_at
                duration_ms = int(delta.total_seconds() * 1000)
            else:
                duration_ms = None

            payload = {
                "execution_id": execution.id,
                "status": execution.status,
                "error": execution.error,
                "duration_ms": duration_ms,
            }

            # Wrap into envelope so it complies with the WebSocket protocol
            envelope = Envelope.create(
                message_type="execution_finished",
                topic=topic,
                data=payload,
                req_id=message_id,
            )

            logger.info("Sending snapshot execution_finished to client %s for execution %s", client_id, execution_id)

            await send_to_client(
                client_id,
                jsonable_encoder(envelope),
                topic=topic,
            )

            logger.info("Snapshot sent successfully")

    except Exception as e:
        logger.error(f"Error in _subscribe_workflow_execution: {str(e)}")
        await send_error(client_id, "Failed to subscribe to workflow execution", message_id)


async def handle_subscribe(client_id: str, message: Dict[str, Any], db: Session) -> None:
    """Handle topic subscription requests.

    Args:
        client_id: The client ID that sent the message
        message: The subscription message
        db: Database session
    """
    try:
        subscribe_msg = SubscribeMessage(**message)

        for topic in subscribe_msg.topics:
            try:
                # Parse topic to validate and get initial data if needed
                topic_type, topic_id = topic.split(":", 1)

                if topic_type == "thread":
                    await _subscribe_thread(client_id, int(topic_id), subscribe_msg.message_id, db)
                elif topic_type == "agent":
                    await _subscribe_agent(client_id, int(topic_id), subscribe_msg.message_id, db)
                elif topic_type == "user":
                    await _subscribe_user(client_id, int(topic_id), subscribe_msg.message_id, db)
                elif topic_type == "workflow_execution":
                    await _subscribe_workflow_execution(client_id, int(topic_id), subscribe_msg.message_id, db)
                else:
                    await send_error(
                        client_id,
                        f"Invalid topic format: Unsupported topic type '{topic_type}'",
                        subscribe_msg.message_id,
                    )
            except ValueError as e:
                await send_error(
                    client_id,
                    f"Invalid topic format: {topic}. Error: {str(e)}",
                    subscribe_msg.message_id,
                )

    except ValueError as e:
        await send_error(client_id, f"Invalid topic format: {str(e)}", message.get("message_id", ""))
    except Exception as e:
        logger.error(f"Error handling subscription: {str(e)}")
        await send_error(client_id, "Failed to process subscription", message.get("message_id", ""))


async def handle_unsubscribe(client_id: str, message: Dict[str, Any], _: Session) -> None:
    """Handle topic unsubscription requests.

    Args:
        client_id: The client ID that sent the message
        message: The unsubscription message
        _: Unused database session
    """
    try:
        message_id = message.get("message_id", "")
        for topic in message.get("topics", []):
            await topic_manager.unsubscribe_from_topic(client_id, topic)

        # Send confirmation message back to client
        await send_to_client(
            client_id,
            {
                "type": "unsubscribe_success",
                "message_id": message_id,
                "topics": message.get("topics", []),
            },
        )
    except Exception as e:
        logger.error(f"Error handling unsubscribe: {str(e)}")
        await send_error(client_id, "Failed to process unsubscribe", message.get("message_id", ""))


# Message handler dispatcher
MESSAGE_HANDLERS = {
    "ping": handle_ping,
    "pong": handle_pong,
    "subscribe": handle_subscribe,
    "unsubscribe": handle_unsubscribe,
    # Thread‑specific handlers used by higher‑level chat API
    "subscribe_thread": None,  # populated below
    "send_message": None,  # populated below
}

# ---------------------------------------------------------------------------
# Runtime inbound payload validation (Phase 2 groundwork)
# ---------------------------------------------------------------------------

# Mapping of *client → server* message types to their strict Pydantic models.
# We validate the incoming JSON before it reaches individual handlers so that
# malformed payloads are rejected consistently in one place.

_INBOUND_SCHEMA_MAP: Dict[str, type[BaseModel]] = {
    "ping": PingMessage,
    "pong": PongMessage,
    "subscribe": SubscribeMessage,
    "unsubscribe": UnsubscribeMessage,
    "subscribe_thread": SubscribeThreadMessage,
    "send_message": SendMessageRequest,
}


# ---------------------------------------------------------------------------
# Dedicated helpers for chat‑centric message types introduced in
# zerg.schemas.ws_messages.MessageType. They wrap the existing topic
# subscription mechanism so the rest of the system (topic_manager &
# event_bus) continues to operate unchanged.


async def handle_subscribe_thread(client_id: str, message: Dict[str, Any], db: Session) -> None:  # noqa: D401
    """Handle a *subscribe_thread* request.

    This wraps the generic *subscribe* flow for threads. Initial history
    is now provided via the REST endpoint; WebSocket only delivers updates.
    """
    try:
        sub_msg = SubscribeThreadMessage(**message)
        await _subscribe_thread(client_id, sub_msg.thread_id, sub_msg.message_id, db)
    except Exception as e:  # broad catch ensures client gets feedback
        logger.error(f"Error in handle_subscribe_thread: {str(e)}")
        await send_error(client_id, "Failed to subscribe to thread", message.get("message_id"))


async def handle_send_message(client_id: str, message: Dict[str, Any], db: Session) -> None:  # noqa: D401
    """Persist a new message to a thread and broadcast it."""

    # Ensure ThreadMessageData is in scope for outgoing messages
    try:
        send_req = SendMessageRequest(**message)

        # Validate thread exists
        thread = crud.get_thread(db, send_req.thread_id)
        if not thread:
            await send_error(client_id, f"Thread {send_req.thread_id} not found", send_req.message_id)
            return

        # Persist the message
        db_msg = crud.create_thread_message(
            db,
            thread_id=send_req.thread_id,
            role="user",
            content=send_req.content,
            processed=False,
        )

        msg_dict = {
            "id": db_msg.id,
            "thread_id": db_msg.thread_id,
            "role": db_msg.role,
            "content": db_msg.content,
            "timestamp": db_msg.timestamp.isoformat() if db_msg.timestamp else None,
            "processed": db_msg.processed,
        }

        outgoing = ThreadMessageData(
            thread_id=send_req.thread_id,
            message=msg_dict,
            message_id=send_req.message_id,
        )

        topic = f"thread:{send_req.thread_id}"
        await topic_manager.broadcast_to_topic(topic, outgoing.model_dump())

        # We intentionally do **not** publish a secondary
        # ``THREAD_MESSAGE_CREATED`` event here. The freshly‑created
        # ``thread_message`` broadcast above already informs every subscriber
        # of the new content. Emitting an additional event would be redundant
        # and lead to duplicate WebSocket payloads (see chat flow tests).

    except Exception as e:
        logger.error(f"Error in handle_send_message: {str(e)}")
        await send_error(client_id, "Failed to send message", message.get("message_id"))


# Register the chat-specific handlers in the dispatcher
MESSAGE_HANDLERS["subscribe_thread"] = handle_subscribe_thread
MESSAGE_HANDLERS["send_message"] = handle_send_message


async def dispatch_message(client_id: str, message: Dict[str, Any], db: Session) -> None:
    """Dispatch a message to the appropriate handler.

    Args:
        client_id: The client ID that sent the message
        message: The message to dispatch
        db: Database session
    """
    try:
        message_type = message.get("type")
        # ------------------------------------------------------------------
        # 1) Fast-fail on completely unknown "type" field.
        # ------------------------------------------------------------------
        if message_type not in MESSAGE_HANDLERS:
            await send_error(client_id, f"Unknown message type: {message_type}")
            return

        # ------------------------------------------------------------------
        # 2) Schema validation using Pydantic models defined in
        #    ``_INBOUND_SCHEMA_MAP``.  We do **not** trust handlers to repeat
        #    validation – doing it centrally prevents duplicate effort and
        #    guarantees identical error semantics across all message types.
        # ------------------------------------------------------------------

        model_cls = _INBOUND_SCHEMA_MAP.get(message_type)
        if model_cls is not None:
            try:
                # Pydantic v2 – ``model_validate`` is the zero-copy validator.
                model_cls.model_validate(message)
            except ValidationError as exc:
                logger.debug("Schema validation failed for %s: %s", message_type, exc)
                await send_error(
                    client_id,
                    "INVALID_PAYLOAD",
                    message.get("message_id"),
                    close_code=1002,
                )
                # Draft spec says we should close with 1002 on protocol error;
                return

        # If validation passed (or no schema yet), forward to handler.
        await MESSAGE_HANDLERS[message_type](client_id, message, db)
    except Exception as e:
        logger.error(f"Error dispatching message: {str(e)}")
        await send_error(client_id, "Failed to process message")


__all__ = ["dispatch_message"]
