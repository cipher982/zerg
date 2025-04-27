"""Pure agent definition using LangGraph Functional API (ReAct style).

This module contains **no database logic** it is purely responsible for
defining *how the agent thinks*.  Persistence and streaming will be handled by
AgentRunner.
"""

import datetime as _dt
import logging
import os
from typing import Annotated
from typing import List

from langchain_core.messages import AIMessage
from langchain_core.messages import BaseMessage
from langchain_core.messages import ToolMessage
from langchain_core.tools import tool
from langchain_openai import ChatOpenAI
from langgraph.func import task
from langgraph.graph import END
from langgraph.graph import START
from langgraph.graph import StateGraph
from langgraph.graph.message import add_messages
from typing_extensions import TypedDict

logger = logging.getLogger(__name__)


# ---------------------------------------------------------------------------
# Tools
# ---------------------------------------------------------------------------


@tool
def get_current_time() -> str:  # noqa: D401 – imperative description fits tool
    """Return the current date/time as ISO-8601 string."""

    return _dt.datetime.now().isoformat()


# ---------------------------------------------------------------------------
# State definition for the functional API
# ---------------------------------------------------------------------------


class AgentState(TypedDict):
    messages: Annotated[List[BaseMessage], add_messages]


# ---------------------------------------------------------------------------
# Graph tasks (Functional API)
# ---------------------------------------------------------------------------


def _make_llm(agent_row, tools):
    """Factory to create a streaming-capable ChatOpenAI bound to *tools*."""

    llm = ChatOpenAI(
        model=agent_row.model,
        temperature=0,
        streaming=True,
        api_key=os.environ.get("OPENAI_API_KEY"),
    )

    return llm.bind_tools(tools)


def build_react_graph(agent_row):
    """Return a compiled LangGraph runnable representing the ReAct loop."""

    tools = [get_current_time]
    llm_with_tools = _make_llm(agent_row, tools)

    # ----------------------------- tasks ----------------------------------

    @task
    def call_llm(state: AgentState):  # noqa: D401 – descriptive naming is fine
        """Run the chat model; may emit tool calls."""

        response = llm_with_tools.invoke(state["messages"])
        return {"messages": [response]}

    @task
    def call_tool(state: AgentState):
        """Execute tool requested in the last AIMessage (if any)."""

        last_msg = state["messages"][-1]
        if not (isinstance(last_msg, AIMessage) and last_msg.tool_calls):
            # Defensive – nothing to do, edge logic should prevent this
            return {}

        tool_msgs: List[ToolMessage] = []

        for tc in last_msg.tool_calls:
            tool_name = tc["name"]
            try:
                outcome = {tool.name: tool for tool in tools}[tool_name].invoke(tc["args"])
            except Exception as exc:  # pragma: no cover – unlikely but log
                logger.exception("Error executing tool %s", tool_name)
                outcome = f"<tool-error> {exc}"

            tool_msgs.append(ToolMessage(content=str(outcome), tool_call_id=tc["id"], name=tool_name))

        return {"messages": tool_msgs}

    # ----------------------------- graph ----------------------------------

    gb = StateGraph(AgentState)
    gb.add_node("llm", call_llm)
    gb.add_node("tool", call_tool)

    gb.add_edge(START, "llm")

    def _router(state: AgentState):
        last = state["messages"][-1]
        if isinstance(last, AIMessage) and last.tool_calls:
            return "tool"
        return END

    gb.add_conditional_edges("llm", _router, {"tool": "tool", END: END})
    gb.add_edge("tool", "llm")

    runnable = gb.compile()
    return runnable


# ---------------------------------------------------------------------------
# Public helper
# ---------------------------------------------------------------------------


def get_runnable(agent_row):  # noqa: D401 – matches public API naming
    """Return a compiled LangGraph runnable for the given Agent ORM row."""

    return build_react_graph(agent_row)
