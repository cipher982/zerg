"""Pure agent definition using LangGraph Functional API (ReAct style).

This module contains **no database logic** it is purely responsible for
defining *how the agent thinks*.  Persistence and streaming will be handled by
AgentRunner.
"""

import logging
from typing import List
from typing import Optional

from langchain_core.messages import AIMessage
from langchain_core.messages import BaseMessage
from langchain_core.messages import ToolMessage

# External dependencies
from langchain_openai import ChatOpenAI
from langgraph.checkpoint.memory import MemorySaver
from langgraph.func import entrypoint
from langgraph.func import task
from langgraph.graph.message import add_messages

# Local imports (late to avoid circulars)
from zerg.callbacks.token_stream import WsTokenCallback
from zerg.config import get_settings

# Centralised flags
from zerg.tools import get_registry

logger = logging.getLogger(__name__)


# ---------------------------------------------------------------------------
# LLM Factory (remains similar, adjusted docstring/comment)
# ---------------------------------------------------------------------------


def _make_llm(agent_row, tools):
    """Factory that returns a *tool-bound* ``ChatOpenAI`` instance.

    If the :pydataattr:`zerg.config.LLM_TOKEN_STREAM` flag is enabled the LLM
    will be configured for *streaming* and the ``WsTokenCallback`` will be
    attached so each new token is forwarded to the WebSocket layer.
    """

    # Feature flag – evaluate the environment variable *lazily* so test cases
    # that tweak ``LLM_TOKEN_STREAM`` via ``monkeypatch.setenv`` after the
    # module import still take effect.

    enable_token_stream = get_settings().llm_token_stream

    # Attach the token stream callback only when the feature flag is enabled.
    kwargs: dict = {
        "model": agent_row.model,
        "temperature": 0.6,
        "streaming": enable_token_stream,
        "api_key": get_settings().openai_api_key,
    }

    if enable_token_stream:
        kwargs["callbacks"] = [WsTokenCallback()]

    llm = ChatOpenAI(**kwargs)

    return llm.bind_tools(tools)


# ---------------------------------------------------------------------------
# Main Agent Implementation
# ---------------------------------------------------------------------------


def get_runnable(agent_row):  # noqa: D401 – matches public API naming
    """
    Return a compiled LangGraph runnable using the Functional API
    for the given Agent ORM row.
    """
    # --- Define tools and model within scope ---
    # ------------------------------------------------------------------
    # MCP INTEGRATION – Dynamically load tools provided by *all* MCP servers
    # referenced in the agent configuration.  We run the **synchronous**
    # helper which internally spins up an event-loop and fetches the tool
    # manifests.  Duplicate servers across multiple agents are cached by the
    # ``MCPManager`` so each server is contacted at most once per process.
    # ------------------------------------------------------------------

    cfg = getattr(agent_row, "config", {}) or {}
    if "mcp_servers" in cfg:
        # Deferred import to avoid cost when MCP is unused
        from zerg.tools.mcp_adapter import load_mcp_tools_sync  # noqa: WPS433 (late import)

        load_mcp_tools_sync(cfg["mcp_servers"])  # blocking – runs quickly (metadata only)

    # ------------------------------------------------------------------
    # Local registry setup
    # ------------------------------------------------------------------
    registry = get_registry()
    allowed_tools = getattr(agent_row, "allowed_tools", None)
    tools = registry.filter_by_allowlist(allowed_tools)

    if not tools:
        logger.warning(f"No tools available for agent {agent_row.id}")

    tools_by_name = {tool.name: tool for tool in tools}
    llm_with_tools = _make_llm(agent_row, tools)

    # Create a simple memory saver for persistence
    checkpointer = MemorySaver()

    # --- Define Tasks ---
    @task
    def _call_model_sync(messages: List[BaseMessage]):
        """**Synchronous** model invocation helper.

        Wrapped via :pyfunc:`asyncio.to_thread` in :pyfunc:`_call_model_async`
        so that the heavy OpenAI HTTP request never blocks the event-loop.
        """

        return llm_with_tools.invoke(messages)

    async def _call_model_async(messages: List[BaseMessage]):
        """Run the blocking LLM call in a worker thread and await the result."""

        import asyncio

        return await asyncio.to_thread(lambda: _call_model_sync(messages).result())

    #
    # NOTE ON CONCURRENCY
    # -------------------
    # The previous implementation claimed to run tool calls in *parallel* but
    # still resolved each future via ``future.result()`` **one-by-one** which
    # effectively serialised the loop once the first blocking call was hit.
    #
    # We now expose *both* a classic **sync** task wrapper (kept for
    # backwards-compatibility with the LangGraph Functional API runner) **and**
    # a thin **async** helper that executes the synchronous wrapper via
    # ``asyncio.to_thread`` so that callers can await *all* tool calls using
    # ``asyncio.gather``.
    #

    @task
    def _call_tool_sync(tool_call: dict):  # noqa: D401 – internal helper
        """Execute a single tool call (synchronous version)."""

        tool_name = tool_call["name"]
        tool_to_call = tools_by_name.get(tool_name)

        if not tool_to_call:
            observation = f"Error: Tool '{tool_name}' not found."
            logger.error(observation)
        else:
            try:
                observation = tool_to_call.invoke(tool_call["args"])
            except Exception as exc:  # noqa: BLE001
                observation = f"<tool-error> {exc}"
                logger.exception("Error executing tool %s", tool_name)

        return ToolMessage(content=str(observation), tool_call_id=tool_call["id"], name=tool_name)

    async def _call_tool_async(tool_call: dict):  # noqa: D401 – coroutine helper
        """Async wrapper around :pyfunc:`_call_tool_sync`.

        The function is executed in a default thread-pool so that long-running
        or blocking tool work (I/O, CPU) does **not** block the event-loop.
        """

        # ``_call_tool_sync`` returns a Future-like langgraph task object; the
        # actual synchronous work kicks off when we call ``.result()``.  We
        # therefore *wrap* that blocking call in ``asyncio.to_thread``.

        import asyncio

        return await asyncio.to_thread(lambda: _call_tool_sync(tool_call).result())

    # --- Define main entrypoint ---
    async def _agent_executor_async(
        messages: List[BaseMessage], *, previous: Optional[List[BaseMessage]] = None
    ) -> List[BaseMessage]:
        """
        Main entrypoint for the agent. This is a simple ReAct loop:
        1. Call the model to get a response
        2. If the model calls a tool, execute it and append the result
        3. Repeat until the model generates a final response
        """
        # Initialise message history.
        #
        # ``previous`` is populated by LangGraph's *checkpointing* mechanism
        # and therefore contains the **conversation as it existed at the end
        # of the *last* agent turn*.  When the user sends a *new* message the
        # frontend creates the row in the database which is forwarded as the
        # *messages* argument while *previous* still lacks that entry.
        #
        # If we were to prefer the *previous* list we would effectively drop
        # the most recent user input – the LLM would see an outdated context
        # and produce no new assistant response.  This manifested in the UI
        # as the agent replying only to the very first user message but
        # staying silent afterwards.
        #
        # We therefore always start from the *messages* list (which is the
        # *source of truth* pulled from the database right before the
        # runnable is invoked) and *only* fall back to *previous* when the
        # caller provides an *empty* messages array (which currently never
        # happens in normal operation but keeps the function robust for
        # direct unit-tests).
        current_messages = messages or previous or []

        # Start by calling the model with the current context
        llm_response = await _call_model_async(current_messages)

        # Until the model stops calling tools, continue the loop
        import asyncio

        while isinstance(llm_response, AIMessage) and llm_response.tool_calls:
            # --------------------------------------------------------------
            # True *parallel* tool execution
            # --------------------------------------------------------------
            # Convert every tool call into an **awaitable** coroutine and run
            # them concurrently via ``asyncio.gather``.  Errors inside an
            # individual tool no longer block the whole batch – the
            # *observation* string will contain the exception text which the
            # LLM can reason about in the next turn.
            # --------------------------------------------------------------

            coro_list = [_call_tool_async(tc) for tc in llm_response.tool_calls]
            tool_results = await asyncio.gather(*coro_list, return_exceptions=False)

            # Update message history with the model response and tool results
            current_messages = add_messages(current_messages, [llm_response] + list(tool_results))

            # Call model again with updated messages
            llm_response = await _call_model_async(current_messages)

        # Add the final response to history
        final_messages = add_messages(current_messages, [llm_response])

        # Return the full conversation history
        return final_messages

    # ------------------------------------------------------------------
    # Synchronous wrapper for libraries/tests that call ``.invoke``
    # ------------------------------------------------------------------

    def _agent_executor_sync(messages: List[BaseMessage], *, previous: Optional[List[BaseMessage]] = None):
        """Blocking wrapper that delegates to the async implementation."""

        import asyncio

        return asyncio.run(_agent_executor_async(messages, previous=previous))

    # ------------------------------------------------------------------
    # Expose BOTH sync & async entrypoints to LangGraph
    # ------------------------------------------------------------------

    @entrypoint(checkpointer=checkpointer)
    def agent_executor(messages: List[BaseMessage], *, previous: Optional[List[BaseMessage]] = None):
        return _agent_executor_sync(messages, previous=previous)

    # Attach the *async* implementation manually – LangGraph picks this up so
    # callers can use ``.ainvoke`` while tests and legacy code continue to use
    # the blocking ``.invoke`` API.

    agent_executor.afunc = _agent_executor_async  # type: ignore[attr-defined]

    return agent_executor


# ---------------------------------------------------------------------------
# Helper – preserve for unit-testing & potential reuse
# ---------------------------------------------------------------------------


def get_tool_messages(ai_msg: AIMessage):  # noqa: D401 – util function
    """Return a list of ToolMessage instances for each tool call in *ai_msg*.

    This helper is mainly used in unit-tests but can also aid debugging in a
    REPL. It was removed during an earlier refactor and has been reinstated to
    keep backwards-compatibility with the test-suite.
    """

    if not getattr(ai_msg, "tool_calls", None):
        return []

    # Import builtin tools to ensure they're registered

    # Get the tool registry
    registry = get_registry()

    tool_messages: List[ToolMessage] = []
    for tc in ai_msg.tool_calls:
        name = tc.get("name")
        content = "<no-op>"
        try:
            # Resolve the tool – tests may monkeypatch the **module-level**
            # reference (``zerg.agents_def.zerg_react_agent.get_current_time``)
            # so we first look it up dynamically on the module and fall back
            # to the registry entry.

            import sys

            module_tool = getattr(sys.modules[__name__], name, None)
            tool = module_tool or registry.get(name)

            if tool is not None:
                content = tool.invoke(tc.get("args", {}))
            else:
                content = f"<tool-error> Tool '{name}' not found in registry"
        except Exception as exc:  # noqa: BLE001
            content = f"<tool-error> {exc}"

        tool_messages.append(ToolMessage(content=str(content), tool_call_id=tc.get("id"), name=name))

    return tool_messages


# ---------------------------------------------------------------------------
# Backward compatibility - expose get_current_time at module level
# ---------------------------------------------------------------------------
# Import builtin tools to ensure registration
import zerg.tools.builtin  # noqa: F401, E402

# Get the tool from registry and expose it at module level for tests
_registry = get_registry()
get_current_time = _registry.get("get_current_time")
