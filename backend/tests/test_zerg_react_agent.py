"""Unit-tests for the *pure* Functional-API agent definition."""

from unittest.mock import MagicMock
from unittest.mock import patch

import pytest


@pytest.fixture()
def agent_row(db_session):
    """Insert a minimal Agent row in the test DB and return it."""

    from zerg.models.models import Agent as AgentModel

    row = AgentModel(
        name="unit-bot",
        system_instructions="sys",
        task_instructions="task",
        model="gpt-4o-mini",
    )
    db_session.add(row)
    db_session.commit()
    db_session.refresh(row)
    return row


def test_get_runnable_returns_graph(agent_row):
    from zerg.agents_def import zerg_react_agent as mod

    runnable = mod.get_runnable(agent_row)

    # The compiled graph exposes .invoke() – that is enough for the runner.
    assert hasattr(runnable, "invoke")


def test_invoke_calls_llm(agent_row):
    from zerg.agents_def import zerg_react_agent as mod

    # Patch ChatOpenAI so we don't hit real LLM
    with (
        patch("zerg.agents_def.zerg_react_agent.ChatOpenAI") as mock_llm_cls,
        patch("zerg.agents_def.zerg_react_agent.StateGraph") as mock_state_graph_cls,
    ):
        mock_instance = MagicMock()
        # When llm_with_tools.invoke is called, return AIMessage("hi")
        from langchain_core.messages import AIMessage

        mock_instance.bind_tools.return_value.invoke.return_value = AIMessage(content="hi")
        mock_llm_cls.return_value = mock_instance

        # Stub the compiled graph so .invoke() returns our expected structure.
        from types import SimpleNamespace

        from langchain_core.messages import AIMessage

        fake_graph = SimpleNamespace(invoke=lambda _state: {"messages": [AIMessage(content="hi")]})

        mock_state_graph_cls.return_value.compile.return_value = fake_graph

        runnable = mod.get_runnable(agent_row)

        res = runnable.invoke({"messages": []})

        assert res["messages"][-1].content == "hi"
