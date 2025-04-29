"""Pure agent definition using LangGraph Functional API (ReAct style).

This module contains **no database logic** it is purely responsible for
defining *how the agent thinks*.  Persistence and streaming will be handled by
AgentRunner.
"""

import datetime as _dt
import logging
import os
from typing import List
from typing import Optional

from langchain_core.messages import AIMessage
from langchain_core.messages import BaseMessage
from langchain_core.messages import ToolMessage
from langchain_core.tools import tool
from langchain_openai import ChatOpenAI
from langgraph.checkpoint.memory import MemorySaver
from langgraph.func import entrypoint
from langgraph.func import task
from langgraph.graph.message import add_messages

logger = logging.getLogger(__name__)


# ---------------------------------------------------------------------------
# Tools
# ---------------------------------------------------------------------------


@tool
def get_current_time() -> str:  # noqa: D401 – imperative description fits tool
    """Return the current date/time as ISO-8601 string."""

    return _dt.datetime.now().isoformat()


# ---------------------------------------------------------------------------
# LLM Factory (remains similar, adjusted docstring/comment)
# ---------------------------------------------------------------------------


def _make_llm(agent_row, tools):
    """Factory to create a streaming-capable ChatOpenAI bound to *tools*."""

    # Non-streaming LLM for synchronous invocation (streaming handled separately by runner)
    llm = ChatOpenAI(
        model=agent_row.model,
        temperature=0,
        streaming=False,  # Streaming is handled by the runner typically
        api_key=os.environ.get("OPENAI_API_KEY"),
    )

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
    tools = [get_current_time]
    tools_by_name = {tool.name: tool for tool in tools}
    llm_with_tools = _make_llm(agent_row, tools)

    # Create a simple memory saver for persistence
    checkpointer = MemorySaver()

    # --- Define Tasks ---
    @task
    def call_model(messages: List[BaseMessage]):
        """Call model with a sequence of messages."""
        response = llm_with_tools.invoke(messages)
        return response

    @task
    def call_tool(tool_call: dict):
        """Execute a single tool call."""
        tool_name = tool_call["name"]
        tool_to_call = tools_by_name.get(tool_name)

        if not tool_to_call:
            observation = f"Error: Tool '{tool_name}' not found."
            logger.error(observation)
        else:
            try:
                observation = tool_to_call.invoke(tool_call["args"])
            except Exception as exc:
                observation = f"<tool-error> {exc}"
                logger.exception("Error executing tool %s", tool_name)

        return ToolMessage(content=str(observation), tool_call_id=tool_call["id"], name=tool_name)

    # --- Define main entrypoint ---
    @entrypoint(checkpointer=checkpointer)
    def agent_executor(
        messages: List[BaseMessage], *, previous: Optional[List[BaseMessage]] = None
    ) -> List[BaseMessage]:
        """
        Main entrypoint for the agent. This is a simple ReAct loop:
        1. Call the model to get a response
        2. If the model calls a tool, execute it and append the result
        3. Repeat until the model generates a final response
        """
        # Initialize message history from previous or use the input messages
        current_messages = previous or messages

        # Start by calling the model with the current context
        llm_response = call_model(current_messages).result()

        # Until the model stops calling tools, continue the loop
        while isinstance(llm_response, AIMessage) and llm_response.tool_calls:
            # Execute tools in parallel
            tool_futures = [call_tool(tc) for tc in llm_response.tool_calls]
            tool_results = [fut.result() for fut in tool_futures]

            # Update message history with the model response and tool results
            current_messages = add_messages(current_messages, [llm_response] + tool_results)

            # Call model again with updated messages
            llm_response = call_model(current_messages).result()

        # Add the final response to history
        final_messages = add_messages(current_messages, [llm_response])

        # Return the full conversation history
        return final_messages

    # Return the compiled entrypoint
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

    tool_messages: List[ToolMessage] = []
    for tc in ai_msg.tool_calls:
        name = tc.get("name")
        content = "<no-op>"
        try:
            # Try to resolve tool by name in current module globals
            tool_fn = globals().get(name)
            if tool_fn is not None and hasattr(tool_fn, "invoke"):
                content = tool_fn.invoke(tc.get("args", {}))
        except Exception as exc:  # noqa: BLE001
            content = f"<tool-error> {exc}"

        tool_messages.append(ToolMessage(content=str(content), tool_call_id=tc.get("id"), name=name))

    return tool_messages
