"""AgentRunner – asynchronous one-turn execution helper.

This class bridges:

• Agent ORM row (system instructions, model name, …)
• ThreadService for DB persistence
• LangGraph **runnable** compiled from the functional ReAct definition.

Design goals
------------
1. Fully *async* – uses ``await runnable.ainvoke`` so no ``Future`` objects
   ever propagate.
2. Keep DB interactions synchronous for now (SQLAlchemy sync API).  These DB
   calls run inside FastAPI's request thread so they remain thread-safe.
3. Provide a thin synchronous wrapper ``run_thread_sync`` so legacy tests that
   call the method directly don't break.  This wrapper simply delegates to
   the async implementation via ``asyncio.run`` and will be removed once all
   call-sites are async.
"""

from __future__ import annotations

import logging
from typing import Any
from typing import Dict
from typing import Sequence
from typing import Tuple

from sqlalchemy.orm import Session

from zerg.agents_def import zerg_react_agent

# Token streaming context helper
from zerg.callbacks.token_stream import set_current_thread_id
from zerg.crud import crud
from zerg.models.models import Agent as AgentModel
from zerg.models.models import Thread as ThreadModel
from zerg.services.thread_service import ThreadService

logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Local in-memory cache for compiled LangGraph runnables.
# Keyed by (agent_id, agent_updated_at, stream_flag) so that any edit to the
# agent definition automatically busts the cache.  The cache is deliberately
# **process-local** – workers in a multi-process Gunicorn deployment will each
# compile their own runnable once on first use which is acceptable given the
# small cost (~100 ms).
# ---------------------------------------------------------------------------

_RUNNABLE_CACHE: Dict[Tuple[int, str, bool], Any] = {}


class AgentRunner:  # noqa: D401 – naming follows project conventions
    """Run one agent turn (async)."""

    def __init__(self, agent_row: AgentModel, *, thread_service: ThreadService | None = None):
        self.agent = agent_row
        self.thread_service = thread_service or ThreadService
        # ------------------------------------------------------------------
        # Lazily compile (or fetch from cache) the LangGraph runnable.  Using
        # a small in-process cache avoids the expensive graph compilation on
        # every single run (~100 ms) while still picking up changes whenever
        # the Agent row is modified (updated_at changes).
        # ------------------------------------------------------------------

        updated_at_str = agent_row.updated_at.isoformat() if getattr(agent_row, "updated_at", None) else "0"
        cache_key = (agent_row.id, updated_at_str, self.enable_token_stream)

        if cache_key in _RUNNABLE_CACHE:
            self._runnable = _RUNNABLE_CACHE[cache_key]
            logger.debug("AgentRunner: using cached runnable for agent %s", agent_row.id)
        else:
            self._runnable = zerg_react_agent.get_runnable(agent_row)
            _RUNNABLE_CACHE[cache_key] = self._runnable
            logger.debug("AgentRunner: compiled & cached runnable for agent %s", agent_row.id)

        # Whether this runner/LLM emits per-token chunks – treat env value
        # case-insensitively; anything truthy like "1", "true", "yes" enables
        # the feature.
        from zerg.constants import LLM_TOKEN_STREAM  # late import avoids cycles

        self.enable_token_stream = LLM_TOKEN_STREAM

    # ------------------------------------------------------------------
    # Public API – asynchronous
    # ------------------------------------------------------------------

    async def run_thread(self, db: Session, thread: ThreadModel) -> Sequence[AgentModel]:
        """Process unprocessed messages and return created assistant message rows."""

        original_msgs = self.thread_service.get_thread_messages_as_langchain(db, thread.id)
        unprocessed_rows = crud.get_unprocessed_messages(db, thread.id)

        if not unprocessed_rows:
            logger.info("No unprocessed messages for thread %s", thread.id)
            return []  # Return empty list if no work

        # Configuration for thread persistence
        config = {
            "configurable": {
                "thread_id": str(thread.id),
            }
        }

        # ------------------------------------------------------------------
        # Token-streaming context handling: set the *current* thread so the
        # ``WsTokenCallback`` can resolve the correct topic when forwarding
        # tokens.  We make sure to *always* reset afterwards to avoid leaking
        # state across concurrent agent turns.
        # ------------------------------------------------------------------

        # Set the context var and keep the **token** so we can restore safely
        _ctx_token = set_current_thread_id(thread.id)

        try:
            # Use **async** invoke with the entrypoint
            # Pass the messages list directly to the function
            # For Functional API, we use .ainvoke method with the config
            # The entrypoint function will return the full message history
            updated_messages = await self._runnable.ainvoke(original_msgs, config)
        finally:
            # Reset context so unrelated calls aren't attributed to this thread
            set_current_thread_id(None)

        # Extract only the new messages since our last context
        # The zerg_react_agent returns ALL messages including the history
        # so we need to extract just the new ones (those after our original list)
        if len(updated_messages) <= len(original_msgs):
            logger.warning("No new messages generated by agent for thread %s", thread.id)
            return []

        new_messages = updated_messages[len(original_msgs) :]

        # Persist the assistant & tool messages
        created_rows = self.thread_service.save_new_messages(
            db,
            thread_id=thread.id,
            messages=new_messages,
            processed=True,
        )

        # Mark user messages processed
        self.thread_service.mark_messages_processed(db, (row.id for row in unprocessed_rows))

        # Touch timestamp
        self.thread_service.touch_thread_timestamp(db, thread.id)

        # Return *all* created rows so callers can decide how to emit them
        # over WebSocket (assistant **and** tool messages).  The caller can
        # easily derive subsets by inspecting the ``role`` field.

        return created_rows

    # No synchronous wrapper – all call-sites should be async going forward.
