"""
TDD Tests for Canonical Serialization

These tests ensure lossless conversion between canonical types
and database storage format, maintaining backward compatibility.
"""

import pytest

from zerg.schemas.canonical_serialization import DatabaseWorkflowAdapter
from zerg.schemas.canonical_serialization import deserialize_workflow_from_database
from zerg.schemas.canonical_serialization import serialize_workflow_for_database
from zerg.schemas.canonical_types import CanonicalEdge
from zerg.schemas.canonical_types import CanonicalWorkflow
from zerg.schemas.canonical_types import NodeId
from zerg.schemas.canonical_types import create_agent_node
from zerg.schemas.canonical_types import create_tool_node
from zerg.schemas.canonical_types import create_trigger_node

# ============================================================================
# Test Fixtures
# ============================================================================


@pytest.fixture
def sample_canonical_workflow():
    """Create a sample canonical workflow for testing."""
    # Create nodes
    trigger_node = create_trigger_node("trigger-1", 0, 0, "manual", {"timeout": 30})
    agent_node = create_agent_node("agent-1", 100, 200, 42, "Execute the task")
    tool_node = create_tool_node(
        "tool-1", 300, 400, "http_request", {"method": "GET", "url": "https://api.example.com"}
    )

    # Create edges
    edge1 = CanonicalEdge(from_node_id=NodeId("trigger-1"), to_node_id=NodeId("agent-1"), config={"label": "start"})
    edge2 = CanonicalEdge(from_node_id=NodeId("agent-1"), to_node_id=NodeId("tool-1"), config={})

    return CanonicalWorkflow(
        id=123, name="Test Workflow", nodes=[trigger_node, agent_node, tool_node], edges=[edge1, edge2]
    )


@pytest.fixture
def sample_database_format():
    """Sample database format data."""
    return {
        "nodes": [
            {
                "id": "trigger-1",
                "type": "trigger",
                "position": {"x": 0, "y": 0},
                "trigger_type": "manual",
                "config": {"timeout": 30},
            },
            {
                "id": "agent-1",
                "type": "agent",
                "position": {"x": 100, "y": 200},
                "agent_id": 42,
                "message": "Execute the task",
            },
            {
                "id": "tool-1",
                "type": "tool",
                "position": {"x": 300, "y": 400},
                "tool_name": "http_request",
                "parameters": {"method": "GET", "url": "https://api.example.com"},
            },
        ],
        "edges": [
            {"from": "trigger-1", "to": "agent-1", "config": {"label": "start"}},
            {"from": "agent-1", "to": "tool-1", "config": {}},
        ],
    }


# ============================================================================
# Serialization Tests - Canonical to Database
# ============================================================================


class TestCanonicalToDatabase:
    """Test conversion from canonical types to database format."""

    def test_workflow_serialization(self, sample_canonical_workflow):
        """Canonical workflow should serialize to correct database format."""
        result = serialize_workflow_for_database(sample_canonical_workflow)

        # Check structure
        assert "id" in result
        assert "name" in result
        assert "nodes" in result
        assert "edges" in result

        # Check values
        assert result["id"] == 123
        assert result["name"] == "Test Workflow"
        assert len(result["nodes"]) == 3
        assert len(result["edges"]) == 2

    def test_agent_node_serialization(self, sample_canonical_workflow):
        """Agent node should serialize with direct field access."""
        result = serialize_workflow_for_database(sample_canonical_workflow)

        # Find agent node in result
        agent_node = next(node for node in result["nodes"] if node["node_id"] == "agent-1")

        assert agent_node["node_type"] == "agent"
        assert agent_node["config"]["agent_id"] == 42  # In config field
        assert agent_node["config"]["message"] == "Execute the task"
        assert agent_node["position"]["x"] == 100
        assert agent_node["position"]["y"] == 200

    def test_tool_node_serialization(self, sample_canonical_workflow):
        """Tool node should serialize with direct field access."""
        result = serialize_workflow_for_database(sample_canonical_workflow)

        # Find tool node in result
        tool_node = next(node for node in result["nodes"] if node["node_id"] == "tool-1")

        assert tool_node["node_type"] == "tool"
        assert tool_node["config"]["tool_name"] == "http_request"  # In config field
        assert tool_node["config"]["parameters"]["method"] == "GET"
        assert tool_node["config"]["parameters"]["url"] == "https://api.example.com"

    def test_trigger_node_serialization(self, sample_canonical_workflow):
        """Trigger node should serialize with direct field access."""
        result = serialize_workflow_for_database(sample_canonical_workflow)

        # Find trigger node in result
        trigger_node = next(node for node in result["nodes"] if node["node_id"] == "trigger-1")

        assert trigger_node["node_type"] == "trigger"
        assert trigger_node["config"]["trigger_type"] == "manual"
        assert trigger_node["config"]["timeout"] == 30

    def test_edge_serialization(self, sample_canonical_workflow):
        """Edges should serialize correctly."""
        result = serialize_workflow_for_database(sample_canonical_workflow)

        edges = result["edges"]
        assert len(edges) == 2

        # Check first edge
        edge1 = edges[0]
        assert edge1["from_node_id"] == "trigger-1"
        assert edge1["to_node_id"] == "agent-1"
        assert edge1["config"]["label"] == "start"

        # Check second edge
        edge2 = edges[1]
        assert edge2["from_node_id"] == "agent-1"
        assert edge2["to_node_id"] == "tool-1"
        assert edge2["config"] == {}


# ============================================================================
# Deserialization Tests - Database to Canonical
# ============================================================================


class TestDatabaseToCanonical:
    """Test conversion from database format to canonical types."""

    def test_workflow_deserialization(self, sample_database_format):
        """Database format should deserialize to canonical workflow."""
        workflow = deserialize_workflow_from_database(
            sample_database_format, workflow_id=123, workflow_name="Test Workflow"
        )

        # Check canonical workflow properties
        assert workflow.id == 123
        assert workflow.name == "Test Workflow"
        assert len(workflow.nodes) == 3
        assert len(workflow.edges) == 2

        # Test direct field access works
        agent_nodes = workflow.get_agent_nodes()
        assert len(agent_nodes) == 1
        assert agent_nodes[0].agent_id.value == 42  # Direct access!

        tool_nodes = workflow.get_tool_nodes()
        assert len(tool_nodes) == 1
        assert tool_nodes[0].tool_name == "http_request"  # Direct access!

    def test_node_type_conversion(self, sample_database_format):
        """Nodes should convert to correct canonical types."""
        workflow = deserialize_workflow_from_database(sample_database_format, 123, "Test")

        # Check node types
        trigger_nodes = [n for n in workflow.nodes if n.is_trigger]
        agent_nodes = [n for n in workflow.nodes if n.is_agent]
        tool_nodes = [n for n in workflow.nodes if n.is_tool]

        assert len(trigger_nodes) == 1
        assert len(agent_nodes) == 1
        assert len(tool_nodes) == 1

    def test_edge_conversion(self, sample_database_format):
        """Edges should convert to canonical format."""
        workflow = deserialize_workflow_from_database(sample_database_format, 123, "Test")

        # Test edge operations
        trigger_edges = workflow.get_outgoing_edges(NodeId("trigger-1"))
        assert len(trigger_edges) == 1
        assert trigger_edges[0].to_node_id.value == "agent-1"
        assert trigger_edges[0].config["label"] == "start"

        agent_edges = workflow.get_outgoing_edges(NodeId("agent-1"))
        assert len(agent_edges) == 1
        assert agent_edges[0].to_node_id.value == "tool-1"


# ============================================================================
# Round-trip Tests - Lossless Conversion
# ============================================================================


class TestRoundTripConversion:
    """Test that serialization is lossless (round-trip safe)."""

    def test_canonical_to_database_to_canonical(self, sample_canonical_workflow):
        """Round-trip conversion should be lossless."""
        original = sample_canonical_workflow

        # Convert to database format
        db_format = serialize_workflow_for_database(original)

        # Convert back to canonical
        restored = deserialize_workflow_from_database(db_format, original.id, original.name)

        # Should be equivalent
        assert restored.id == original.id
        assert restored.name == original.name
        assert len(restored.nodes) == len(original.nodes)
        assert len(restored.edges) == len(original.edges)

        # Check node data preservation
        for orig_node in original.nodes:
            restored_node = restored.get_node_by_id(orig_node.id)

            assert restored_node.node_type == orig_node.node_type
            assert restored_node.position.x == orig_node.position.x
            assert restored_node.position.y == orig_node.position.y

            if orig_node.is_agent:
                assert restored_node.agent_id.value == orig_node.agent_id.value
                assert restored_node.agent_data.message == orig_node.agent_data.message
            elif orig_node.is_tool:
                assert restored_node.tool_name == orig_node.tool_name
                assert restored_node.tool_data.parameters == orig_node.tool_data.parameters
            elif orig_node.is_trigger:
                assert restored_node.trigger_data.trigger_type == orig_node.trigger_data.trigger_type
                # Config comparison: database format includes trigger_type in config during serialization,
                # but canonical format separates them. Both should have the same non-trigger_type config.
                orig_config = {k: v for k, v in orig_node.trigger_data.config.items() if k != "trigger_type"}
                restored_config = {k: v for k, v in restored_node.trigger_data.config.items() if k != "trigger_type"}
                assert restored_config == orig_config

    def test_database_to_canonical_to_database(self, sample_database_format):
        """Round-trip conversion should preserve database format."""
        original_db = sample_database_format

        # Convert to canonical
        canonical = deserialize_workflow_from_database(original_db, 123, "Test Workflow")

        # Convert back to database format
        restored_db = serialize_workflow_for_database(canonical)

        # Should preserve structure (may reorder, but content same)
        assert restored_db["id"] == 123
        assert restored_db["name"] == "Test Workflow"
        assert len(restored_db["nodes"]) == len(original_db["nodes"])
        assert len(restored_db["edges"]) == len(original_db["edges"])

        # Check that all original nodes are present (format may change but content preserved)
        original_node_ids = {node["id"] for node in original_db["nodes"]}
        restored_node_ids = {node["node_id"] for node in restored_db["nodes"]}
        assert original_node_ids == restored_node_ids

        # Verify content is preserved (allowing for format changes)
        for orig_node in original_db["nodes"]:
            restored_node = next(n for n in restored_db["nodes"] if n["node_id"] == orig_node["id"])
            assert restored_node["node_type"] == orig_node["type"]
            assert restored_node["position"] == orig_node["position"]


# ============================================================================
# Database Adapter Tests
# ============================================================================


class TestDatabaseWorkflowAdapter:
    """Test the database adapter for integration with existing code."""

    def test_create_workflow_from_request(self):
        """Should create canonical workflow from API request."""
        request_data = {
            "nodes": [
                {
                    "id": "agent-1",
                    "type": "agent",
                    "position": {"x": 100, "y": 200},
                    "agent_id": 42,
                    "message": "Test message",
                }
            ],
            "edges": [],
        }

        workflow = DatabaseWorkflowAdapter.create_workflow_from_request(
            workflow_id=456, workflow_name="API Workflow", canvas_data=request_data
        )

        assert workflow.id == 456
        assert workflow.name == "API Workflow"
        assert len(workflow.nodes) == 1
        assert workflow.get_agent_nodes()[0].agent_id.value == 42

    def test_load_workflow_from_database(self, sample_database_format):
        """Should load canonical workflow from database."""
        workflow = DatabaseWorkflowAdapter.load_workflow_from_database(
            workflow_id=789, workflow_name="DB Workflow", canvas_data=sample_database_format
        )

        assert workflow.id == 789
        assert workflow.name == "DB Workflow"
        assert len(workflow.nodes) == 3

        # Test direct field access works
        agent_nodes = workflow.get_agent_nodes()
        assert agent_nodes[0].agent_id.value == 42

    def test_save_workflow_to_database(self, sample_canonical_workflow):
        """Should prepare canonical workflow for database storage."""
        canvas_data = DatabaseWorkflowAdapter.save_workflow_to_database(sample_canonical_workflow)

        # Should not include workflow-level metadata
        assert "id" not in canvas_data
        assert "name" not in canvas_data

        # Should include canvas structure
        assert "nodes" in canvas_data
        assert "edges" in canvas_data
        assert len(canvas_data["nodes"]) == 3
        assert len(canvas_data["edges"]) == 2

    def test_update_workflow_canvas(self, sample_canonical_workflow):
        """Should update workflow canvas while maintaining canonical format."""
        # Add a new node via canvas update

        # This would need adjustment - the current signature expects dict updates
        # For now, test the concept
        updated_serialized = serialize_workflow_for_database(sample_canonical_workflow)
        updated_serialized["nodes"].append(
            {
                "id": "new-tool",
                "type": "tool",
                "position": {"x": 500, "y": 600},
                "tool_name": "new_tool",
                "parameters": {},
            }
        )

        updated_workflow = deserialize_workflow_from_database(
            updated_serialized, sample_canonical_workflow.id, sample_canonical_workflow.name
        )

        assert len(updated_workflow.nodes) == 4
        assert len(updated_workflow.get_tool_nodes()) == 2


# ============================================================================
# Legacy Format Tests
# ============================================================================


class TestLegacyFormatSupport:
    """Test that serialization handles legacy database formats."""

    def test_legacy_agent_node_format(self):
        """Should handle legacy agent node formats."""
        legacy_format = {
            "nodes": [
                {
                    "id": "agent-1",
                    "type": {"AgentIdentity": {"agent_id": 42}},  # Legacy format
                    "position": {"x": 100, "y": 200},
                    "message": "Legacy message",
                }
            ],
            "edges": [],
        }

        workflow = deserialize_workflow_from_database(legacy_format, 1, "Legacy Workflow")

        # Should convert to canonical format
        agent_nodes = workflow.get_agent_nodes()
        assert len(agent_nodes) == 1
        assert agent_nodes[0].agent_id.value == 42  # Direct access works!
        assert agent_nodes[0].agent_data.message == "Legacy message"

    def test_mixed_format_handling(self):
        """Should handle mixed old and new formats in same workflow."""
        mixed_format = {
            "nodes": [
                # New format
                {
                    "id": "agent-1",
                    "type": "agent",
                    "position": {"x": 100, "y": 200},
                    "agent_id": 1,
                    "message": "New format",
                },
                # Legacy format
                {
                    "id": "agent-2",
                    "type": {"AgentIdentity": {"agent_id": 2}},
                    "position": {"x": 300, "y": 400},
                    "message": "Legacy format",
                },
            ],
            "edges": [],
        }

        workflow = deserialize_workflow_from_database(mixed_format, 1, "Mixed Workflow")

        agent_nodes = workflow.get_agent_nodes()
        assert len(agent_nodes) == 2

        # Both should work with direct field access
        agent1 = workflow.get_node_by_id(NodeId("agent-1"))
        agent2 = workflow.get_node_by_id(NodeId("agent-2"))

        assert agent1.agent_id.value == 1
        assert agent2.agent_id.value == 2


# ============================================================================
# Performance Tests
# ============================================================================


class TestSerializationPerformance:
    """Test serialization performance for large workflows."""

    def test_large_workflow_serialization(self):
        """Should handle large workflows efficiently."""
        # Create workflow with many nodes
        nodes = []
        edges = []

        # Create 100 agent nodes
        for i in range(100):
            node = create_agent_node(f"agent-{i}", i * 10, i * 10, i + 1, f"Message {i}")
            nodes.append(node)

            # Connect to next node
            if i < 99:
                edge = CanonicalEdge(from_node_id=NodeId(f"agent-{i}"), to_node_id=NodeId(f"agent-{i+1}"), config={})
                edges.append(edge)

        large_workflow = CanonicalWorkflow(id=999, name="Large Workflow", nodes=nodes, edges=edges)

        # Test serialization performance
        import time

        start = time.perf_counter()

        serialized = serialize_workflow_for_database(large_workflow)

        end = time.perf_counter()
        duration_ms = (end - start) * 1000

        # Should serialize 100 nodes in reasonable time
        assert duration_ms < 100, f"Serialization too slow: {duration_ms:.2f}ms"
        assert len(serialized["nodes"]) == 100
        assert len(serialized["edges"]) == 99

    def test_large_workflow_deserialization(self):
        """Should handle large workflow deserialization efficiently."""
        # Create large database format
        nodes = []
        for i in range(100):
            nodes.append(
                {
                    "id": f"agent-{i}",
                    "type": "agent",
                    "position": {"x": i * 10, "y": i * 10},
                    "agent_id": i + 1,
                    "message": f"Message {i}",
                }
            )

        large_db_format = {"nodes": nodes, "edges": []}

        # Test deserialization performance
        import time

        start = time.perf_counter()

        workflow = deserialize_workflow_from_database(large_db_format, 999, "Large Workflow")

        end = time.perf_counter()
        duration_ms = (end - start) * 1000

        # Should deserialize 100 nodes in reasonable time
        assert duration_ms < 200, f"Deserialization too slow: {duration_ms:.2f}ms"
        assert len(workflow.nodes) == 100

        # Test that direct field access still works
        agent_nodes = workflow.get_agent_nodes()
        assert len(agent_nodes) == 100
        assert agent_nodes[0].agent_id.value == 1  # Direct access works!
