"""
E2E test for the most basic workflow scenario:
- Add one agent to workflow
- Press run
- Workflow should execute without errors

This test was added because this basic interaction was failing in production
despite having extensive test coverage.
"""

from unittest.mock import AsyncMock
from unittest.mock import patch

import pytest

from zerg.models.models import Workflow
from zerg.schemas.workflow import Position
from zerg.schemas.workflow import WorkflowData
from zerg.schemas.workflow import WorkflowEdge
from zerg.schemas.workflow import WorkflowNode
from zerg.services.workflow_engine import workflow_engine


def create_basic_agent_workflow(agent_id: int) -> WorkflowData:
    """Create the simplest possible workflow: trigger -> agent."""
    return WorkflowData(
        nodes=[
            # Trigger node (start)
            WorkflowNode(
                id="trigger-start",
                type="trigger",
                position=Position(x=100, y=100),
                config={"trigger_type": "manual"},
            ),
            # Agent node
            WorkflowNode(
                id="agent-1",
                type="agent",
                position=Position(x=300, y=100),
                config={"agent_id": agent_id, "message": "Hello, this is a test workflow execution."},
            ),
        ],
        edges=[WorkflowEdge(from_node_id="trigger-start", to_node_id="agent-1", config={})],
    )


@pytest.mark.asyncio
async def test_basic_agent_workflow_execution_e2e(db, test_user, sample_agent):
    """
    E2E test for basic agent workflow execution.

    This test ensures the most fundamental user interaction works:
    1. Create a workflow with a trigger and agent node
    2. Execute the workflow
    3. Verify it completes successfully without errors

    This test does NOT mock AgentRunner to catch real integration issues.
    """

    # Create a real workflow
    workflow_data = create_basic_agent_workflow(sample_agent.id)
    workflow = Workflow(
        owner_id=test_user.id,
        name="Basic E2E Test Workflow",
        description="Test the most basic agent workflow execution",
        canvas=workflow_data.model_dump(),
        is_active=True,
    )
    db.add(workflow)
    db.commit()

    # Mock only the LLM call to avoid external dependencies
    with patch("zerg.agents_def.zerg_react_agent.get_runnable") as mock_get_runnable:
        # Create a mock runnable that simulates successful agent execution
        mock_runnable = AsyncMock()

        # Mock the ainvoke method to return what a real agent would return
        async def mock_ainvoke(messages, config):
            # Return the input messages plus a new assistant message
            # This simulates what the real LangGraph agent does
            from langchain_core.messages import AIMessage

            return messages + [AIMessage(content="Hello! I successfully processed your request.")]

        mock_runnable.ainvoke = mock_ainvoke
        mock_get_runnable.return_value = mock_runnable

        # Execute the workflow - this should work end-to-end
        execution_id = await workflow_engine.execute_workflow(workflow.id)

        # Verify execution completed successfully
        assert execution_id is not None

        # Check execution record
        from zerg.models.models import WorkflowExecution

        execution = db.query(WorkflowExecution).filter_by(id=execution_id).first()
        assert execution is not None
        assert execution.phase == "finished"
        assert execution.result == "success"
        assert execution.started_at is not None
        assert execution.finished_at is not None
        assert execution.error_message is None

        # Check that duration calculation works (this was the failing part)
        assert execution.finished_at >= execution.started_at

        # Check node execution states
        from zerg.models.models import NodeExecutionState

        node_states = db.query(NodeExecutionState).filter_by(workflow_execution_id=execution_id).all()

        # Should have executed both nodes
        executed_nodes = {state.node_id for state in node_states}
        assert "trigger-start" in executed_nodes
        assert "agent-1" in executed_nodes

        # All nodes should have succeeded
        for state in node_states:
            assert state.phase == "finished"
            assert state.result == "success"
            assert state.error_message is None

        # Agent node should have output with messages
        agent_state = next(s for s in node_states if s.node_id == "agent-1")
        assert agent_state.output is not None
        assert "messages" in agent_state.output["value"]
        assert len(agent_state.output["value"]["messages"]) > 0

        # Verify the message serialization includes proper timestamp field
        # (This was the original bug - accessing created_at instead of timestamp)
        message = agent_state.output["value"]["messages"][0]
        assert "created_at" in message  # Should be serialized with this key
        assert message["created_at"] is not None  # Should have timestamp
        assert message["role"] == "assistant"
        assert message["content"] is not None

        print(f"✅ E2E test passed - execution {execution_id} completed successfully")


@pytest.mark.asyncio
async def test_basic_workflow_datetime_handling(db, test_user, sample_agent):
    """
    Specific test for datetime handling in workflow execution.

    This test ensures that started_at and finished_at can be subtracted
    without timezone awareness issues.
    """

    # Create a minimal workflow
    workflow_data = create_basic_agent_workflow(sample_agent.id)
    workflow = Workflow(
        owner_id=test_user.id,
        name="DateTime Test Workflow",
        description="Test datetime handling",
        canvas=workflow_data.model_dump(),
        is_active=True,
    )
    db.add(workflow)
    db.commit()

    # Mock the agent to avoid LLM calls
    with patch("zerg.agents_def.zerg_react_agent.get_runnable") as mock_get_runnable:
        mock_runnable = AsyncMock()

        async def mock_ainvoke(messages, config):
            from langchain_core.messages import AIMessage

            return messages + [AIMessage(content="Test response")]

        mock_runnable.ainvoke = mock_ainvoke
        mock_get_runnable.return_value = mock_runnable

        # Execute workflow
        execution_id = await workflow_engine.execute_workflow(workflow.id)

        # Get execution record
        from zerg.models.models import WorkflowExecution

        execution = db.query(WorkflowExecution).filter_by(id=execution_id).first()

        # Test the specific datetime operations that were failing
        assert execution.started_at is not None
        assert execution.finished_at is not None

        # This subtraction should not raise "can't subtract offset-naive and offset-aware datetimes"
        duration_delta = execution.finished_at - execution.started_at
        duration_ms = int(duration_delta.total_seconds() * 1000)

        assert duration_ms >= 0
        assert isinstance(duration_ms, int)

        print(f"✅ DateTime test passed - duration: {duration_ms}ms")
