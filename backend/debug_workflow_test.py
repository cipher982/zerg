#!/usr/bin/env python
"""Debug script to test workflow execution directly."""

import asyncio
import logging
from unittest.mock import patch

from zerg.database import get_session_factory
from zerg.models.models import Agent
from zerg.models.models import NodeExecutionState
from zerg.models.models import User
from zerg.models.models import Workflow
from zerg.models.models import WorkflowExecution
from zerg.services.workflow_engine import workflow_engine

# Set up logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


async def test_workflow():
    """Test workflow execution directly."""

    session_factory = get_session_factory()

    with session_factory() as db:
        # Get or create test user
        user = db.query(User).filter_by(email="debug@test.com").first()
        if not user:
            user = User(email="debug@test.com", provider="google", provider_user_id="test_debug_user", is_active=True)
            db.add(user)
            db.commit()
        else:
            logger.info(f"Using existing user: {user.email}")

        # Create test agent
        agent = Agent(
            name="Test Agent",
            system_instructions="Test system instructions",
            task_instructions="Test task instructions",
            owner_id=user.id,
            model="gpt-4o-mini",
            config={"temperature": 0.7},
            allowed_tools=None,  # NULL means all tools allowed
        )
        db.add(agent)
        db.commit()

        # Create test workflow with trigger -> tool -> agent chain
        workflow_canvas = {
            "nodes": [
                {
                    "id": "trigger-1",
                    "type": "trigger",
                    "position": {"x": 50, "y": 100},
                    "config": {"trigger_type": "manual"},
                },
                {
                    "id": "tool-1",
                    "type": "tool",
                    "position": {"x": 200, "y": 100},
                    "config": {"tool_name": "test_tool", "static_params": {"input": "test_data"}},
                },
                {
                    "id": "agent-1",
                    "type": "agent",
                    "position": {"x": 350, "y": 100},
                    "config": {"agent_id": agent.id, "message": "Process: ${tool-1.value.result}"},
                },
            ],
            "edges": [
                {"from_node_id": "trigger-1", "to_node_id": "tool-1"},
                {"from_node_id": "tool-1", "to_node_id": "agent-1"},
            ],
        }

        workflow = Workflow(
            name="Debug Test Workflow",
            description="Test workflow for debugging",
            owner_id=user.id,
            canvas=workflow_canvas,
            is_active=True,
        )
        db.add(workflow)
        db.commit()

        workflow_id = workflow.id

    # Mock tool and agent
    with patch("zerg.services.node_executors.get_tool_resolver") as mock_resolver:
        mock_tool = type("MockTool", (), {})()
        mock_tool.run = lambda params: {"result": "test_output"}

        mock_resolver_instance = type("MockResolver", (), {})()
        mock_resolver_instance.get_tool = lambda name: mock_tool if name == "test_tool" else None
        mock_resolver.return_value = mock_resolver_instance

        # Mock AgentRunner
        async def mock_run_thread(db, thread):
            mock_msg = type("MockMessage", (), {})()
            mock_msg.id = 1
            mock_msg.role = "assistant"
            mock_msg.content = "Test response"
            mock_msg.timestamp = None
            mock_msg.thread_id = thread.id
            return [mock_msg]

        with patch("zerg.services.node_executors.AgentRunner") as mock_agent_runner:
            mock_runner_instance = type("MockRunner", (), {})()
            mock_runner_instance.run_thread = mock_run_thread
            mock_agent_runner.return_value = mock_runner_instance

            # Execute workflow
            logger.info(f"Starting workflow execution for workflow_id={workflow_id}")
            execution_id = await workflow_engine.execute_workflow(workflow_id)
            logger.info(f"Workflow execution started with execution_id={execution_id}")

            # Wait a bit and check status
            await asyncio.sleep(2)

            with session_factory() as db:
                execution = db.query(WorkflowExecution).filter_by(id=execution_id).first()
                logger.info(f"Execution phase: {execution.phase}, result: {execution.result}")

                # Check node states
                node_states = db.query(NodeExecutionState).filter_by(workflow_execution_id=execution_id).all()
                for state in node_states:
                    logger.info(f"Node {state.node_id}: phase={state.phase}, result={state.result}")
                    if state.output:
                        logger.info(f"  Output: {state.output}")

                # Final check
                if execution.phase == "finished" and execution.result == "success":
                    logger.info("✅ Workflow completed successfully!")
                else:
                    logger.error(f"❌ Workflow did not complete: phase={execution.phase}, result={execution.result}")


if __name__ == "__main__":
    asyncio.run(test_workflow())
