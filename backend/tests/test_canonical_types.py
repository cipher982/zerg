"""
TDD Test Suite for Carmack-Style Canonical Types

These tests ensure:
1. No invalid data can be created
2. Direct field access works without parsing
3. Validation happens at creation time, not runtime
4. Zero transformation overhead for valid data
"""

import pytest

from zerg.schemas.canonical_types import AgentId
from zerg.schemas.canonical_types import AgentNodeData
from zerg.schemas.canonical_types import CanonicalEdge
from zerg.schemas.canonical_types import CanonicalNode
from zerg.schemas.canonical_types import CanonicalWorkflow
from zerg.schemas.canonical_types import NodeId
from zerg.schemas.canonical_types import NodeType
from zerg.schemas.canonical_types import Position
from zerg.schemas.canonical_types import ToolNodeData
from zerg.schemas.canonical_types import TriggerNodeData
from zerg.schemas.canonical_types import create_agent_node
from zerg.schemas.canonical_types import create_tool_node
from zerg.schemas.canonical_types import create_trigger_node

# ============================================================================
# Value Object Tests - Fail fast validation
# ============================================================================


class TestValueObjects:
    """Test that value objects validate at creation and are immutable."""

    def test_node_id_valid(self):
        """Valid NodeId should create successfully."""
        node_id = NodeId("test-node-123")
        assert node_id.value == "test-node-123"

    def test_node_id_empty_fails(self):
        """Empty NodeId should fail validation."""
        with pytest.raises(ValueError, match="NodeId must be non-empty string"):
            NodeId("")

    def test_node_id_none_fails(self):
        """None NodeId should fail validation."""
        with pytest.raises(ValueError, match="NodeId must be non-empty string"):
            NodeId(None)

    def test_node_id_too_long_fails(self):
        """Excessively long NodeId should fail validation."""
        long_id = "x" * 101  # Over 100 char limit
        with pytest.raises(ValueError, match="NodeId too long"):
            NodeId(long_id)

    def test_agent_id_valid(self):
        """Valid AgentId should create successfully."""
        agent_id = AgentId(42)
        assert agent_id.value == 42

    def test_agent_id_negative_fails(self):
        """Negative AgentId should fail validation."""
        with pytest.raises(ValueError, match="AgentId must be positive integer"):
            AgentId(-1)

    def test_agent_id_zero_fails(self):
        """Zero AgentId should fail validation."""
        with pytest.raises(ValueError, match="AgentId must be positive integer"):
            AgentId(0)

    def test_agent_id_string_fails(self):
        """String AgentId should fail validation."""
        with pytest.raises(ValueError, match="AgentId must be positive integer"):
            AgentId("42")

    def test_position_valid(self):
        """Valid Position should create successfully."""
        pos = Position(10.5, 20.0)
        assert pos.x == 10.5
        assert pos.y == 20.0

    def test_position_integers_valid(self):
        """Integer coordinates should be valid."""
        pos = Position(10, 20)
        assert pos.x == 10
        assert pos.y == 20

    def test_position_string_fails(self):
        """String coordinates should fail validation."""
        with pytest.raises(ValueError, match="Position coordinates must be numeric"):
            Position("10", 20)


# ============================================================================
# Node Data Tests - Type-specific validation
# ============================================================================


class TestNodeData:
    """Test node data payloads validate correctly."""

    def test_agent_data_valid(self):
        """Valid AgentNodeData should create successfully."""
        data = AgentNodeData(agent_id=AgentId(42), message="Test message")
        assert data.agent_id.value == 42
        assert data.message == "Test message"

    def test_agent_data_empty_message_valid(self):
        """Empty message should be valid (defaults to empty string)."""
        data = AgentNodeData(agent_id=AgentId(42))
        assert data.message == ""

    def test_agent_data_non_string_message_fails(self):
        """Non-string message should fail validation."""
        with pytest.raises(ValueError, match="Agent message must be string"):
            AgentNodeData(agent_id=AgentId(42), message=123)

    def test_tool_data_valid(self):
        """Valid ToolNodeData should create successfully."""
        data = ToolNodeData(tool_name="http_request", parameters={"url": "https://example.com"})
        assert data.tool_name == "http_request"
        assert data.parameters["url"] == "https://example.com"

    def test_tool_data_empty_name_fails(self):
        """Empty tool name should fail validation."""
        with pytest.raises(ValueError, match="Tool name must be non-empty string"):
            ToolNodeData(tool_name="", parameters={})

    def test_tool_data_non_dict_parameters_fails(self):
        """Non-dict parameters should fail validation."""
        with pytest.raises(ValueError, match="Tool parameters must be dict"):
            ToolNodeData(tool_name="test", parameters="not a dict")

    def test_trigger_data_valid(self):
        """Valid TriggerNodeData should create successfully."""
        data = TriggerNodeData(trigger_type="webhook", config={"url": "/webhook"})
        assert data.trigger_type == "webhook"
        assert data.config["url"] == "/webhook"


# ============================================================================
# Canonical Node Tests - Composition and direct access
# ============================================================================


class TestCanonicalNode:
    """Test canonical node creation and direct field access."""

    def test_agent_node_creation(self):
        """Agent node should create with correct data."""
        node = CanonicalNode(
            id=NodeId("agent-1"),
            position=Position(100, 200),
            node_type=NodeType.AGENT,
            agent_data=AgentNodeData(agent_id=AgentId(42), message="Test agent"),
        )

        # Direct field access - no parsing
        assert node.id.value == "agent-1"
        assert node.position.x == 100
        assert node.position.y == 200
        assert node.agent_id.value == 42  # Direct access!
        assert node.is_agent is True
        assert node.is_tool is False
        assert node.is_trigger is False

    def test_tool_node_creation(self):
        """Tool node should create with correct data."""
        node = CanonicalNode(
            id=NodeId("tool-1"),
            position=Position(300, 400),
            node_type=NodeType.TOOL,
            tool_data=ToolNodeData(tool_name="http_request", parameters={"method": "GET"}),
        )

        # Direct field access - no parsing
        assert node.tool_name == "http_request"  # Direct access!
        assert node.is_tool is True
        assert node.is_agent is False

    def test_multiple_data_fields_fails(self):
        """Node with multiple data fields should fail validation."""
        with pytest.raises(ValueError, match="Exactly one data field must be set"):
            CanonicalNode(
                id=NodeId("bad-node"),
                position=Position(0, 0),
                node_type=NodeType.AGENT,
                agent_data=AgentNodeData(agent_id=AgentId(1)),
                tool_data=ToolNodeData(tool_name="test", parameters={}),
            )

    def test_no_data_fields_fails(self):
        """Node with no data fields should fail validation."""
        with pytest.raises(ValueError, match="Exactly one data field must be set"):
            CanonicalNode(id=NodeId("bad-node"), position=Position(0, 0), node_type=NodeType.AGENT)

    def test_mismatched_type_and_data_fails(self):
        """Node with mismatched type and data should fail."""
        with pytest.raises(ValueError, match="Agent node must have agent_data"):
            CanonicalNode(
                id=NodeId("bad-node"),
                position=Position(0, 0),
                node_type=NodeType.AGENT,
                tool_data=ToolNodeData(tool_name="test", parameters={}),
            )

    def test_wrong_field_access_fails(self):
        """Accessing wrong field should fail with clear error."""
        agent_node = CanonicalNode(
            id=NodeId("agent-1"),
            position=Position(0, 0),
            node_type=NodeType.AGENT,
            agent_data=AgentNodeData(agent_id=AgentId(42)),
        )

        with pytest.raises(ValueError, match="Node agent-1 is not a tool node"):
            _ = agent_node.tool_name


# ============================================================================
# Factory Function Tests - Clean creation interface
# ============================================================================


class TestFactoryFunctions:
    """Test factory functions provide clean creation interface."""

    def test_create_agent_node(self):
        """Agent node factory should create valid node."""
        node = create_agent_node(
            node_id="agent-1", position_x=100.0, position_y=200.0, agent_id=42, message="Test message"
        )

        assert node.id.value == "agent-1"
        assert node.position.x == 100.0
        assert node.position.y == 200.0
        assert node.agent_id.value == 42
        assert node.agent_data.message == "Test message"
        assert node.is_agent is True

    def test_create_tool_node(self):
        """Tool node factory should create valid node."""
        node = create_tool_node(
            node_id="tool-1",
            position_x=300.0,
            position_y=400.0,
            tool_name="http_request",
            parameters={"url": "https://example.com"},
        )

        assert node.id.value == "tool-1"
        assert node.tool_name == "http_request"
        assert node.tool_data.parameters["url"] == "https://example.com"
        assert node.is_tool is True

    def test_create_trigger_node(self):
        """Trigger node factory should create valid node."""
        node = create_trigger_node(node_id="trigger-1", position_x=0.0, position_y=0.0, trigger_type="webhook")

        assert node.id.value == "trigger-1"
        assert node.trigger_data.trigger_type == "webhook"
        assert node.is_trigger is True


# ============================================================================
# Workflow Tests - Complete workflow validation
# ============================================================================


class TestCanonicalWorkflow:
    """Test complete workflow creation and operations."""

    @pytest.fixture
    def sample_workflow(self):
        """Create a sample workflow for testing."""
        agent_node = create_agent_node("agent-1", 100, 200, 42, "Do task")
        tool_node = create_tool_node("tool-1", 300, 400, "http_request")
        trigger_node = create_trigger_node("trigger-1", 0, 0, "manual")

        edge1 = CanonicalEdge(from_node_id=NodeId("trigger-1"), to_node_id=NodeId("agent-1"))
        edge2 = CanonicalEdge(from_node_id=NodeId("agent-1"), to_node_id=NodeId("tool-1"))

        return CanonicalWorkflow(
            id=1, name="Test Workflow", nodes=[trigger_node, agent_node, tool_node], edges=[edge1, edge2]
        )

    def test_workflow_creation(self, sample_workflow):
        """Workflow should create with valid structure."""
        workflow = sample_workflow

        assert workflow.id == 1
        assert workflow.name == "Test Workflow"
        assert len(workflow.nodes) == 3
        assert len(workflow.edges) == 2

    def test_get_node_by_id(self, sample_workflow):
        """Should find node by ID directly."""
        workflow = sample_workflow

        node = workflow.get_node_by_id(NodeId("agent-1"))
        assert node.is_agent is True
        assert node.agent_id.value == 42

    def test_get_node_by_id_not_found(self, sample_workflow):
        """Should fail when node not found."""
        workflow = sample_workflow

        with pytest.raises(ValueError, match="Node not found: nonexistent"):
            workflow.get_node_by_id(NodeId("nonexistent"))

    def test_get_agent_nodes(self, sample_workflow):
        """Should filter agent nodes directly."""
        workflow = sample_workflow

        agent_nodes = workflow.get_agent_nodes()
        assert len(agent_nodes) == 1
        assert agent_nodes[0].agent_id.value == 42

    def test_get_tool_nodes(self, sample_workflow):
        """Should filter tool nodes directly."""
        workflow = sample_workflow

        tool_nodes = workflow.get_tool_nodes()
        assert len(tool_nodes) == 1
        assert tool_nodes[0].tool_name == "http_request"

    def test_get_outgoing_edges(self, sample_workflow):
        """Should find outgoing edges directly."""
        workflow = sample_workflow

        edges = workflow.get_outgoing_edges(NodeId("agent-1"))
        assert len(edges) == 1
        assert edges[0].to_node_id.value == "tool-1"

    def test_get_incoming_edges(self, sample_workflow):
        """Should find incoming edges directly."""
        workflow = sample_workflow

        edges = workflow.get_incoming_edges(NodeId("agent-1"))
        assert len(edges) == 1
        assert edges[0].from_node_id.value == "trigger-1"


# ============================================================================
# Performance Tests - Validate zero overhead
# ============================================================================


class TestPerformance:
    """Test that canonical types have zero parsing overhead."""

    def test_direct_field_access_performance(self):
        """Direct field access should be immediate - no parsing."""
        node = create_agent_node("test", 0, 0, 42, "message")

        # These should be direct field access, no method calls
        import time

        start = time.perf_counter()
        for _ in range(10000):
            _ = node.agent_id.value  # Direct access
            _ = node.is_agent  # Direct property
            _ = node.position.x  # Direct access
        end = time.perf_counter()

        # Should be incredibly fast - under 1ms for 10k accesses
        duration_ms = (end - start) * 1000
        assert duration_ms < 10, f"Direct access too slow: {duration_ms:.2f}ms"

    def test_node_creation_performance(self):
        """Node creation should be fast with validation."""
        import time

        start = time.perf_counter()
        for i in range(1000):
            create_agent_node(f"node-{i}", i, i, i + 1, f"message-{i}")
        end = time.perf_counter()

        # Should create 1000 nodes in under 100ms
        duration_ms = (end - start) * 1000
        assert duration_ms < 100, f"Node creation too slow: {duration_ms:.2f}ms"


# ============================================================================
# Edge Case Tests - Comprehensive validation coverage
# ============================================================================


class TestEdgeCases:
    """Test edge cases and boundary conditions."""

    def test_unicode_node_id(self):
        """Unicode characters in node ID should work."""
        node_id = NodeId("node-测试-🚀")
        assert node_id.value == "node-测试-🚀"

    def test_large_parameters_dict(self):
        """Large parameters dict should work."""
        large_params = {f"param_{i}": f"value_{i}" for i in range(1000)}
        node = create_tool_node("tool", 0, 0, "test", large_params)
        assert len(node.tool_data.parameters) == 1000

    def test_negative_position_coordinates(self):
        """Negative coordinates should be valid."""
        pos = Position(-100.5, -200.0)
        assert pos.x == -100.5
        assert pos.y == -200.0

    def test_workflow_with_no_nodes(self):
        """Workflow with empty nodes list should be valid."""
        workflow = CanonicalWorkflow(id=1, name="Empty Workflow", nodes=[], edges=[])
        assert len(workflow.nodes) == 0
        assert len(workflow.get_agent_nodes()) == 0
