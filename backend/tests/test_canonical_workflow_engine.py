"""
TDD Tests for Canonical Workflow Engine

These tests verify the new engine works with direct field access
and zero runtime parsing overhead.
"""

from unittest.mock import AsyncMock
from unittest.mock import Mock
from unittest.mock import patch

import pytest

from zerg.models.models import Agent
from zerg.models.models import Workflow
from zerg.schemas.canonical_serialization import serialize_workflow_for_database
from zerg.schemas.canonical_types import create_agent_node
from zerg.schemas.canonical_types import create_tool_node
from zerg.schemas.canonical_types import create_trigger_node
from zerg.services.workflow_engine import WorkflowEngine


@pytest.fixture
def sample_canonical_workflow_data():
    """Create sample workflow data in canonical format."""
    # Create canonical nodes
    trigger_node = create_trigger_node("trigger-1", 0, 0, "manual")
    agent_node = create_agent_node("agent-1", 100, 200, 42, "Execute task")
    tool_node = create_tool_node("tool-1", 300, 400, "http_request", {"url": "https://example.com"})

    from zerg.schemas.canonical_types import CanonicalEdge
    from zerg.schemas.canonical_types import CanonicalWorkflow
    from zerg.schemas.canonical_types import NodeId

    edge1 = CanonicalEdge(from_node_id=NodeId("trigger-1"), to_node_id=NodeId("agent-1"))
    edge2 = CanonicalEdge(from_node_id=NodeId("agent-1"), to_node_id=NodeId("tool-1"))

    canonical_workflow = CanonicalWorkflow(
        id=1, name="Test Canonical Workflow", nodes=[trigger_node, agent_node, tool_node], edges=[edge1, edge2]
    )

    # Convert to database format for storage
    return serialize_workflow_for_database(canonical_workflow)


@pytest.fixture
def mock_workflow_model(sample_canonical_workflow_data):
    """Mock workflow model from database."""
    workflow = Mock(spec=Workflow)
    workflow.id = 1
    workflow.name = "Test Canonical Workflow"
    workflow.canvas_data = sample_canonical_workflow_data
    workflow.is_active = True
    return workflow


@pytest.fixture
def mock_agent():
    """Mock agent model."""
    agent = Mock(spec=Agent)
    agent.id = 42
    agent.name = "Test Agent"
    agent.allowed_tools = ["http_request"]
    return agent


class TestCanonicalWorkflowEngine:
    """Test the canonical workflow engine with direct field access."""

    @pytest.mark.asyncio
    async def test_execute_workflow_basic_flow(self, db_session, mock_workflow_model, mock_agent):
        """Test basic workflow execution with canonical types."""
        engine = WorkflowEngine()

        # Mock database queries
        with patch.object(db_session, "query") as mock_query:
            # Setup workflow query mock
            workflow_query = Mock()
            workflow_query.filter_by.return_value.first.return_value = mock_workflow_model

            # Setup agent query mock
            agent_query = Mock()
            agent_query.filter_by.return_value.first.return_value = mock_agent

            # Configure query method to return appropriate mock based on model type
            def query_side_effect(model_class):
                if model_class == Workflow:
                    return workflow_query
                elif model_class == Agent:
                    return agent_query
                else:
                    # For WorkflowExecution and other models
                    generic_query = Mock()
                    generic_query.filter_by.return_value.first.return_value = None
                    return generic_query

            mock_query.side_effect = query_side_effect

            # Mock database operations
            with patch.object(db_session, "add"), patch.object(db_session, "commit"), patch.object(
                db_session, "refresh"
            ):
                # Mock crud operations
                with patch("zerg.crud.crud.create_thread") as mock_create_thread:
                    mock_thread = Mock()
                    mock_thread.id = 1
                    mock_create_thread.return_value = mock_thread

                    # Mock AgentRunner
                    with patch("zerg.managers.agent_runner.AgentRunner") as mock_runner_class:
                        mock_runner = Mock()
                        mock_runner.run_thread = AsyncMock()

                        # Mock assistant message response
                        mock_message = Mock()
                        mock_message.role = "assistant"
                        mock_message.content = "Task completed"
                        mock_runner.run_thread.return_value = [mock_message]

                        mock_runner_class.return_value = mock_runner

                        # Mock tool registry
                        with patch("zerg.tools.registry.get_registry") as mock_get_registry:
                            mock_registry = Mock()
                            mock_tool = Mock()
                            mock_tool.ainvoke = AsyncMock(return_value="HTTP response")
                            mock_registry.all_tools.return_value = [Mock(name="http_request", spec=mock_tool)]
                            mock_get_registry.return_value = mock_registry

                            # Mock event publishing
                            with patch("zerg.events.event_bus.publish", new_callable=AsyncMock):
                                # Mock session factory
                                with patch("zerg.database.get_session_factory") as mock_session_factory:
                                    mock_session_factory.return_value.return_value.__enter__.return_value = db_session
                                    mock_session_factory.return_value.return_value.__exit__.return_value = None

                                    # Execute workflow
                                    execution_id = await engine.execute_workflow(workflow_id=1)

                                    # Should return an execution ID
                                    assert execution_id is not None

                                    # Verify that canonical workflow was loaded and executed
                                    # The key insight: no parsing errors should occur because
                                    # canonical types provide direct field access

                                    # Verify agent was queried with direct field access
                                    agent_query.filter_by.assert_called_with(id=42)  # Direct from canonical type

    @pytest.mark.asyncio
    async def test_canonical_agent_node_direct_access(self, db_session, mock_agent):
        """Test that agent nodes use direct field access, not parsing."""
        engine = WorkflowEngine()

        # Create canonical agent node
        agent_node = create_agent_node("agent-1", 100, 200, 42, "Test message")

        # Create the node function
        node_func = engine._create_canonical_agent_node(agent_node)

        # Mock state
        mock_state = {"execution_id": 1, "node_outputs": {}, "completed_nodes": []}

        # Mock database and dependencies
        with patch("zerg.database.get_session_factory") as mock_session_factory:
            mock_session_factory.return_value.return_value.__enter__.return_value = db_session
            mock_session_factory.return_value.return_value.__exit__.return_value = None

            with patch.object(db_session, "query") as mock_query, patch.object(db_session, "add"), patch.object(
                db_session, "commit"
            ):
                # Setup agent query
                agent_query = Mock()
                agent_query.filter_by.return_value.first.return_value = mock_agent
                mock_query.return_value = agent_query

                # Mock crud operations
                with patch("zerg.crud.crud.create_thread") as mock_create_thread, patch(
                    "zerg.crud.crud.create_thread_message"
                ):
                    mock_thread = Mock()
                    mock_thread.id = 1
                    mock_create_thread.return_value = mock_thread

                    # Mock AgentRunner
                    with patch("zerg.managers.agent_runner.AgentRunner") as mock_runner_class:
                        mock_runner = Mock()
                        mock_runner.run_thread = AsyncMock()

                        mock_message = Mock()
                        mock_message.role = "assistant"
                        mock_message.content = "Response"
                        mock_runner.run_thread.return_value = [mock_message]

                        mock_runner_class.return_value = mock_runner

                        # Mock event publishing
                        with patch.object(engine, "_publish_node_event", new_callable=AsyncMock):
                            # Execute node function
                            result = await node_func(mock_state)

                            # Verify result structure
                            assert "node_outputs" in result
                            assert "completed_nodes" in result
                            assert "agent-1" in result["node_outputs"]
                            assert "agent-1" in result["completed_nodes"]

                            # Key test: Verify agent was queried with direct field access
                            # agent_node.agent_id.value should be used directly, no parsing
                            agent_query.filter_by.assert_called_with(id=42)

                            # Verify output contains direct field access values
                            output = result["node_outputs"]["agent-1"]
                            assert output["agent_id"] == 42  # Direct from agent_node.agent_id.value
                            assert output["message"] == "Test message"  # Direct from agent_node.agent_data.message

    @pytest.mark.asyncio
    async def test_canonical_tool_node_direct_access(self):
        """Test that tool nodes use direct field access, not parsing."""
        engine = WorkflowEngine()

        # Create canonical tool node
        tool_node = create_tool_node("tool-1", 300, 400, "http_request", {"method": "GET"})

        # Create the node function
        node_func = engine._create_canonical_tool_node(tool_node)

        # Mock state
        mock_state = {"execution_id": 1, "node_outputs": {}, "completed_nodes": []}

        # Mock database and dependencies
        with patch("zerg.database.get_session_factory") as mock_session_factory:
            mock_db_session = Mock()
            mock_session_factory.return_value.return_value.__enter__.return_value = mock_db_session
            mock_session_factory.return_value.return_value.__exit__.return_value = None

            with patch.object(mock_db_session, "add"), patch.object(mock_db_session, "commit"):
                # Mock tool registry
                with patch("zerg.tools.registry.get_registry") as mock_get_registry:
                    mock_registry = Mock()
                    mock_tool = Mock()
                    mock_tool.name = "http_request"
                    mock_tool.ainvoke = AsyncMock(return_value="HTTP response data")
                    mock_registry.all_tools.return_value = [mock_tool]
                    mock_get_registry.return_value = mock_registry

                    # Mock event publishing
                    with patch.object(engine, "_publish_node_event", new_callable=AsyncMock):
                        # Execute node function
                        result = await node_func(mock_state)

                        # Verify result structure
                        assert "node_outputs" in result
                        assert "completed_nodes" in result
                        assert "tool-1" in result["node_outputs"]

                        # Key test: Verify tool was executed with direct field access
                        output = result["node_outputs"]["tool-1"]
                        assert output["tool_name"] == "http_request"  # Direct from tool_node.tool_name
                        assert output["parameters"]["method"] == "GET"  # Direct from tool_node.tool_data.parameters
                        assert output["result"] == "HTTP response data"

                        # Verify tool.ainvoke was called with direct parameters
                        mock_tool.ainvoke.assert_called_once_with({"method": "GET"})

    def test_direct_field_access_no_parsing(self):
        """Test that canonical types provide direct field access without parsing."""
        # Create canonical nodes
        agent_node = create_agent_node("agent-1", 100, 200, 42, "Test message")
        tool_node = create_tool_node("tool-1", 300, 400, "http_request", {"url": "https://example.com"})
        trigger_node = create_trigger_node("trigger-1", 0, 0, "manual", {"timeout": 30})

        # Test direct field access - no parsing methods called
        assert agent_node.id.value == "agent-1"  # Direct access
        assert agent_node.agent_id.value == 42  # Direct access - no extract_agent_id() needed!
        assert agent_node.agent_data.message == "Test message"  # Direct access

        assert tool_node.id.value == "tool-1"  # Direct access
        assert tool_node.tool_name == "http_request"  # Direct access - no extract_tool_name() needed!
        assert tool_node.tool_data.parameters["url"] == "https://example.com"  # Direct access

        assert trigger_node.id.value == "trigger-1"  # Direct access
        assert trigger_node.trigger_data.trigger_type == "manual"  # Direct access
        assert trigger_node.trigger_data.config["timeout"] == 30  # Direct access

        # Test type checking - no isinstance() needed
        assert agent_node.is_agent is True
        assert agent_node.is_tool is False
        assert tool_node.is_tool is True
        assert tool_node.is_agent is False

    @pytest.mark.asyncio
    async def test_invalid_workflow_data_fails_fast(self, db_session):
        """Test that invalid workflow data fails at load time, not execution time."""
        engine = WorkflowEngine()

        # Create workflow with invalid canvas data
        invalid_workflow = Mock(spec=Workflow)
        invalid_workflow.id = 1
        invalid_workflow.name = "Invalid Workflow"
        invalid_workflow.canvas_data = {
            "nodes": [
                {
                    "id": "agent-1",
                    "type": "agent",
                    # Missing required agent_id - should fail at validation
                    "position": {"x": 100, "y": 200},
                }
            ],
            "edges": [],
        }
        invalid_workflow.is_active = True

        # Mock database to return invalid workflow
        with patch.object(db_session, "query") as mock_query:
            workflow_query = Mock()
            workflow_query.filter_by.return_value.first.return_value = invalid_workflow
            mock_query.return_value = workflow_query

            with patch.object(db_session, "add"), patch.object(db_session, "commit"):
                with patch("zerg.database.get_session_factory") as mock_session_factory:
                    mock_session_factory.return_value.return_value.__enter__.return_value = db_session
                    mock_session_factory.return_value.return_value.__exit__.return_value = None

                    # Should fail fast during workflow loading, not during execution
                    with pytest.raises(ValueError) as exc_info:
                        await engine.execute_workflow(workflow_id=1)

                    # Should fail with clear validation error
                    assert "Invalid workflow data" in str(exc_info.value)

                    # Key insight: Error happens at load time due to canonical validation,
                    # not during execution due to runtime parsing failures

    def test_performance_direct_access_vs_parsing(self):
        """Test that direct field access is significantly faster than parsing."""
        import time

        # Create canonical node
        agent_node = create_agent_node("agent-1", 100, 200, 42, "Test message")

        # Test direct field access performance
        start = time.perf_counter()
        for _ in range(10000):
            _ = agent_node.agent_id.value  # Direct access
            _ = agent_node.agent_data.message  # Direct access
            _ = agent_node.is_agent  # Direct property
        end = time.perf_counter()

        direct_access_ms = (end - start) * 1000

        # Should be incredibly fast - under 10ms for 10k accesses
        assert direct_access_ms < 10, f"Direct access too slow: {direct_access_ms:.2f}ms"

        # Compare with mock parsing approach
        def mock_parse_agent_id(node_dict):
            """Simulate old parsing approach."""
            if "agent_id" in node_dict:
                return node_dict["agent_id"]
            elif "config" in node_dict and "agent_id" in node_dict["config"]:
                return node_dict["config"]["agent_id"]
            elif "type" in node_dict and isinstance(node_dict["type"], dict):
                for key, value in node_dict["type"].items():
                    if key.lower() == "agentidentity" and "agent_id" in value:
                        return value["agent_id"]
            return None

        # Mock old format data
        old_format = {
            "id": "agent-1",
            "type": {"AgentIdentity": {"agent_id": 42}},
            "config": {"message": "Test message"},
        }

        start = time.perf_counter()
        for _ in range(10000):
            _ = mock_parse_agent_id(old_format)  # Parsing approach
        end = time.perf_counter()

        parsing_ms = (end - start) * 1000

        # Direct access should be faster (even modest improvement is good)
        speedup = parsing_ms / direct_access_ms
        assert (
            speedup > 1.1
        ), f"Direct access not faster. Speedup: {speedup:.1f}x (parsing: {parsing_ms:.2f}ms, direct: {direct_access_ms:.2f}ms)"
