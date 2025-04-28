"""
LangGraph Agent Manager for the Zerg Agent Platform (legacy implementation).

This module contains the AgentManager class that handles all LangGraph-based
agent interactions. It is retained for backward compatibility and will be
deprecated in a future release.
"""

import datetime
import logging
import os
from typing import Annotated
from typing import Any
from typing import Dict
from typing import Optional
from typing import Tuple

from langchain_core.messages import AIMessage
from langchain_core.messages import ToolMessage
from langchain_core.tools import tool
from langchain_openai import ChatOpenAI
from langgraph.graph import END
from langgraph.graph import START
from langgraph.graph import StateGraph
from langgraph.graph.message import add_messages
from typing_extensions import TypedDict

from zerg.models.models import Agent as AgentModel
from zerg.models.models import Thread as ThreadModel

logger = logging.getLogger(__name__)


@tool
def get_current_time():
    """Returns the current date and time as ISO-8601 string."""
    return datetime.datetime.now().isoformat()


class AgentManagerState(TypedDict):
    """State definition for the LangGraph agent (legacy)."""

    messages: Annotated[list, add_messages]
    metadata: Dict[str, Any]


class AgentManager:
    """
    Manages LangGraph agent interactions and state persistence (legacy).
    """

    def __init__(self, agent_model: AgentModel):
        self.agent_model = agent_model
        self.tools = [get_current_time]
        self.llm = ChatOpenAI(
            model=agent_model.model,
            temperature=0,
            streaming=True,
            api_key=os.environ.get("OPENAI_API_KEY"),
        )
        self.llm_with_tools = self.llm.bind_tools(self.tools)
        self.tool_map = {tool.name: tool for tool in self.tools}

    def _build_graph(self) -> StateGraph:
        graph_builder = StateGraph(AgentManagerState)
        graph_builder.add_node("chatbot", self._chatbot_node)
        graph_builder.add_node("call_tool", self._call_tool_node)
        graph_builder.add_edge(START, "chatbot")
        graph_builder.add_conditional_edges(
            "chatbot",
            self._decide_next_step,
            {"call_tool": "call_tool", END: END},
        )
        graph_builder.add_edge("call_tool", "chatbot")
        return graph_builder.compile()

    def _chatbot_node(self, state: AgentManagerState) -> Dict:
        response = self.llm_with_tools.invoke(state["messages"])
        return {"messages": [response]}

    def _decide_next_step(self, state: AgentManagerState) -> str:
        last = state["messages"][-1]
        if isinstance(last, AIMessage) and getattr(last, "tool_calls", None):
            return "call_tool"
        return END

    def _call_tool_node(self, state: AgentManagerState) -> Dict:
        last = state["messages"][-1]
        if not isinstance(last, AIMessage) or not getattr(last, "tool_calls", None):
            return {}
        tool_messages = []
        for tc in last.tool_calls:
            tool_name = tc.get("name")
            tool_func = self.tool_map.get(tool_name)
            if tool_func:
                try:
                    observation = tool_func.invoke(tc.get("args", []))
                except Exception as e:
                    observation = f"<tool-error> {e}"
                tool_messages.append(
                    ToolMessage(
                        content=str(observation),
                        tool_call_id=tc.get("id"),
                        name=tool_name,
                    )
                )
        return {"messages": tool_messages}

    def get_or_create_thread(
        self, db, thread_id: Optional[int] = None, title: str = "New Thread", thread_type: str = "chat"
    ) -> Tuple[ThreadModel, bool]:
        from zerg.crud import crud

        created = False
        if thread_id:
            thread = crud.get_thread(db, thread_id)
            if not thread or thread.agent_id != self.agent_model.id:
                thread = crud.create_thread(
                    db=db,
                    agent_id=self.agent_model.id,
                    title=title,
                    active=True,
                    agent_state={},
                    memory_strategy="buffer",
                    thread_type=thread_type,
                )
                created = True
        else:
            thread = crud.create_thread(
                db=db,
                agent_id=self.agent_model.id,
                title=title,
                active=True,
                agent_state={},
                memory_strategy="buffer",
                thread_type=thread_type,
            )
            created = True
        return thread, created
