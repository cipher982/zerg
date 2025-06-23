"""Thread-level persistence helpers.

This module isolates all database interactions related to *conversation
threads* and their messages.  It is an extraction from the former
AgentManager implementation so that higher-level orchestration code can rely
on a clean, well-typed interface rather than calling the low-level *crud*
functions directly.

The goal is **separation of concerns** – AgentDefinition (LLM logic) and
AgentRunner (execution orchestration) should not be aware of SQLAlchemy or
our schema details.
"""

import logging
from typing import Any
from typing import Callable
from typing import Dict
from typing import Iterable
from typing import List
from typing import Optional
from typing import Type

from langchain_core.messages import AIMessage
from langchain_core.messages import BaseMessage
from langchain_core.messages import HumanMessage
from langchain_core.messages import SystemMessage
from langchain_core.messages import ToolMessage
from sqlalchemy.orm import Session

from zerg.crud import crud
from zerg.models.models import Agent as AgentModel
from zerg.models.models import Thread as ThreadModel
from zerg.models.models import ThreadMessage as ThreadMessageModel

logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Message conversion helpers
# ---------------------------------------------------------------------------


def _db_to_langchain(msg_row: ThreadMessageModel) -> BaseMessage:  # pragma: no cover
    """Convert a *ThreadMessage* ORM row into a LangChain *BaseMessage*.

    The mapping is fairly direct except for *assistant* rows that contain
    pending tool calls (``tool_calls`` JSON).  Those become *AIMessage*
    instances that include the *tool_calls* attribute.
    """

    role = msg_row.role

    if role == "system":
        return SystemMessage(content=msg_row.content)
    if role == "user":
        return HumanMessage(content=msg_row.content)
    if role == "assistant":
        # Assistant messages may or may not include tool calls.
        if msg_row.tool_calls:
            return AIMessage(content=msg_row.content, tool_calls=msg_row.tool_calls)  # type: ignore[arg-type]
        return AIMessage(content=msg_row.content)
    if role == "tool":
        return ToolMessage(
            content=msg_row.content,
            tool_call_id=msg_row.tool_call_id or "",  # tool_call_id required by pydantic schema
            name=msg_row.name or "tool",
        )

    # Fallback – treat as generic AI message to avoid crashing; log for devs
    logger.warning("Unknown message role '%s' encountered – defaulting to AIMessage", role)
    return AIMessage(content=msg_row.content)


# ==============================================================================
# MESSAGE CONVERTER - Eliminate isinstance() chain code smell
# ==============================================================================


class LangChainMessageConverter:
    """Clean message converter that eliminates isinstance chain patterns."""

    # Registry of message type converters
    _converters: Dict[Type[BaseMessage], Callable[[BaseMessage], Dict[str, Any]]] = {}

    @classmethod
    def register_converter(cls, message_type: Type[BaseMessage]) -> Callable:
        """Decorator to register a message converter function."""

        def decorator(converter_func: Callable[[BaseMessage], Dict[str, Any]]) -> Callable:
            cls._converters[message_type] = converter_func
            return converter_func

        return decorator

    @classmethod
    def to_create_kwargs(cls, msg: BaseMessage) -> Dict[str, Any]:
        """Convert a LangChain message to crud.create_thread_message kwargs."""
        message_type = type(msg)

        # Look up converter in registry
        converter = cls._converters.get(message_type)
        if converter:
            return converter(msg)

        # Fallback: check if it's a subclass of any registered type
        for registered_type, converter_func in cls._converters.items():
            if isinstance(msg, registered_type):
                return converter_func(msg)

        raise TypeError(f"Unsupported message type: {message_type}")


# Register converters for each LangChain message type
@LangChainMessageConverter.register_converter(SystemMessage)
def _convert_system_message(msg: SystemMessage) -> Dict[str, Any]:
    return {"role": "system", "content": msg.content}


@LangChainMessageConverter.register_converter(HumanMessage)
def _convert_human_message(msg: HumanMessage) -> Dict[str, Any]:
    return {"role": "user", "content": msg.content}


@LangChainMessageConverter.register_converter(AIMessage)
def _convert_ai_message(msg: AIMessage) -> Dict[str, Any]:
    return {
        "role": "assistant",
        "content": msg.content or "",
        "tool_calls": getattr(msg, "tool_calls", None),
    }


@LangChainMessageConverter.register_converter(ToolMessage)
def _convert_tool_message(msg: ToolMessage) -> Dict[str, Any]:
    return {
        "role": "tool",
        "content": msg.content,
        "tool_call_id": msg.tool_call_id,
        "name": msg.name,
    }


def _langchain_to_create_kwargs(msg: BaseMessage):  # pragma: no cover
    """Convert a LangChain message into kwargs for *crud.create_thread_message*.

    CLEAN VERSION: Uses registry pattern instead of isinstance chain.
    """
    return LangChainMessageConverter.to_create_kwargs(msg)


# ---------------------------------------------------------------------------
# Public service façade
# ---------------------------------------------------------------------------


class ThreadService:
    """High-level API for thread/message persistence."""

    # NOTE: All methods are *static* because the service is currently
    # stateless.  However, an instance could later hold config or cache.

    # ---------------------------- Thread helpers -------------------------

    @staticmethod
    def create_thread_with_system_message(
        db: Session,
        agent: AgentModel,
        *,
        title: str,
        thread_type: str = "chat",
        active: Optional[bool] = None,
    ) -> ThreadModel:
        """Create a thread and insert the agent's system message atomically."""

        if active is None:
            # Chat threads should be active by default; others not.
            active = thread_type == "chat"

        thread = crud.create_thread(
            db=db,
            agent_id=agent.id,
            title=title,
            active=active,
            agent_state={},
            memory_strategy="buffer",
            thread_type=thread_type,
        )

        # First message is always the agent's system prompt.
        crud.create_thread_message(
            db=db,
            thread_id=thread.id,
            role="system",
            content=agent.system_instructions,
            processed=True,  # System messages are implicitly processed
        )

        return thread

    # ------------------------------------------------------------------
    # Retrieval helpers
    # ------------------------------------------------------------------

    @staticmethod
    def get_valid_thread_for_agent(db: Session, *, thread_id: int, agent_id: int) -> ThreadModel:
        """Return thread if it belongs to *agent_id* else raise ValueError."""

        thread = crud.get_thread(db, thread_id)
        if thread is None:
            raise ValueError("Thread not found")

        if thread.agent_id != agent_id:
            raise ValueError("Thread does not belong to agent")

        return thread

    @staticmethod
    def get_thread_messages_as_langchain(db: Session, thread_id: int) -> List[BaseMessage]:
        """Return thread history converted to LangChain message objects."""

        rows = crud.get_thread_messages(db, thread_id=thread_id)
        return [_db_to_langchain(row) for row in rows]

    # ------------------------------------------------------------------
    # Mutation helpers
    # ------------------------------------------------------------------

    @staticmethod
    def save_new_messages(
        db: Session,
        *,
        thread_id: int,
        messages: Iterable[BaseMessage],
        processed: bool = True,
        parent_id: Optional[int] = None,
    ) -> List[ThreadMessageModel]:
        """Persist *messages* to the DB and return the created rows."""

        created_rows: List[ThreadMessageModel] = []

        # Track the *last* assistant row so we can back-fill parent_id for
        # subsequent tool messages.  This lets the UI reliably group tool
        # outputs under the correct assistant bubble even after a full page
        # reload (history endpoint).

        current_parent_id: int | None = parent_id

        for msg in messages:
            kwargs = _langchain_to_create_kwargs(msg)
            kwargs.setdefault("processed", processed)

            # Associate tool rows with the latest assistant message.
            if kwargs.get("role") == "tool":
                kwargs["parent_id"] = current_parent_id

            row = crud.create_thread_message(db=db, thread_id=thread_id, commit=False, **kwargs)
            created_rows.append(row)

            # Update parent tracker whenever we hit a new assistant row.
            if row.role == "assistant":
                current_parent_id = row.id

        # Flush once so SQL assigns primary keys for the inserted rows.
        db.flush()
        db.commit()

        return created_rows

    @staticmethod
    def mark_messages_processed(db: Session, message_ids: Iterable[int]):
        """Set processed=True for given DB rows."""

        mids = list(message_ids)
        if not mids:
            return

        crud.mark_messages_processed_bulk(db, mids)

    @staticmethod
    def touch_thread_timestamp(db: Session, thread_id: int):
        """Bump the thread.updated_at field to *now()*."""

        crud.update_thread(db, thread_id)
