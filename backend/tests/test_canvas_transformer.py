"""Tests for canvas data transformation service."""

from zerg.schemas.workflow_schema import WorkflowCanvas
from zerg.schemas.workflow_schema import WorkflowEdge
from zerg.schemas.workflow_schema import WorkflowNode
from zerg.services.canvas_transformer import CanvasTransformer


class TestCanvasTransformer:
    """Test canvas data transformation between formats."""

    def test_from_frontend_standard_format(self):
        """Test transformation from standard frontend format."""
        frontend_data = {
            "nodes": [
                {"id": "node1", "type": "agent", "position": {"x": 100, "y": 200}, "custom_field": "value1"},
                {"id": "node2", "type": {"Tool": {"tool_name": "http_request"}}, "position": {"x": 300, "y": 400}},
            ],
            "edges": [{"source": "node1", "target": "node2", "label": "connection1"}],
            "metadata": {"version": "1.0"},
        }

        canvas = CanvasTransformer.from_frontend(frontend_data)

        assert len(canvas.nodes) == 2
        assert len(canvas.edges) == 1

        # Check node transformation
        node1 = canvas.nodes[0]
        assert node1.node_id == "node1"
        assert node1.node_type == "agent"
        assert node1.position == {"x": 100, "y": 200}
        assert node1.config["custom_field"] == "value1"

        node2 = canvas.nodes[1]
        assert node2.node_id == "node2"
        assert node2.node_type == {"Tool": {"tool_name": "http_request"}}

        # Check edge transformation
        edge = canvas.edges[0]
        assert edge.from_node_id == "node1"
        assert edge.to_node_id == "node2"
        assert edge.config["label"] == "connection1"

        # Check metadata
        assert canvas.metadata["version"] == "1.0"

    def test_from_frontend_legacy_string_nodes(self):
        """Test handling of legacy string node format."""
        frontend_data = {"nodes": ["node1", "node2"], "edges": []}

        canvas = CanvasTransformer.from_frontend(frontend_data)

        assert len(canvas.nodes) == 2
        assert canvas.nodes[0].node_id == "node1"
        assert canvas.nodes[0].node_type == "unknown"
        assert canvas.nodes[1].node_id == "node2"
        assert canvas.nodes[1].node_type == "unknown"

    def test_from_frontend_mixed_formats(self):
        """Test handling of mixed node formats."""
        frontend_data = {
            "nodes": [
                "legacy_node",
                {"id": "modern_node", "type": "agent"},
                {"node_id": "canonical_node", "node_type": "tool"},  # Already canonical
            ],
            "edges": [],
        }

        canvas = CanvasTransformer.from_frontend(frontend_data)

        assert len(canvas.nodes) == 3
        assert canvas.nodes[0].node_id == "legacy_node"
        assert canvas.nodes[0].node_type == "unknown"
        assert canvas.nodes[1].node_id == "modern_node"
        assert canvas.nodes[1].node_type == "agent"
        assert canvas.nodes[2].node_id == "canonical_node"
        assert canvas.nodes[2].node_type == "tool"

    def test_from_frontend_edge_format_variations(self):
        """Test handling of different edge field name variations."""
        frontend_data = {
            "nodes": [{"id": "node1", "type": "agent"}, {"id": "node2", "type": "tool"}],
            "edges": [
                {"source": "node1", "target": "node2"},  # Frontend format
                {"from_node_id": "node1", "to_node_id": "node2"},  # Canonical format
                {"from": "node1", "to": "node2"},  # Alternative format
            ],
        }

        canvas = CanvasTransformer.from_frontend(frontend_data)

        assert len(canvas.edges) == 3
        for edge in canvas.edges:
            assert edge.from_node_id == "node1"
            assert edge.to_node_id == "node2"

    def test_from_frontend_invalid_data(self):
        """Test handling of invalid input data."""
        # Invalid root format
        canvas = CanvasTransformer.from_frontend("invalid")
        assert len(canvas.nodes) == 0
        assert len(canvas.edges) == 0

        # Invalid nodes
        frontend_data = {
            "nodes": [
                {"id": "valid_node", "type": "agent"},
                123,  # Invalid node
                None,  # Invalid node
                {"type": "agent"},  # Missing ID
            ],
            "edges": [],
        }

        canvas = CanvasTransformer.from_frontend(frontend_data)
        assert len(canvas.nodes) == 2  # valid_node + auto-generated ID for missing ID

        # Invalid edges
        frontend_data = {
            "nodes": [{"id": "node1", "type": "agent"}],
            "edges": [
                {"source": "node1", "target": "node2"},  # Valid
                {"source": "node1"},  # Missing target
                123,  # Invalid edge
                None,  # Invalid edge
            ],
        }

        canvas = CanvasTransformer.from_frontend(frontend_data)
        assert len(canvas.edges) == 1  # Only the valid edge

    def test_to_frontend(self):
        """Test conversion from canonical format to frontend format."""
        canvas = WorkflowCanvas(
            nodes=[
                WorkflowNode(
                    node_id="node1", node_type="agent", position={"x": 100, "y": 200}, config={"custom_field": "value1"}
                ),
                WorkflowNode(
                    node_id="node2", node_type={"Tool": {"tool_name": "http_request"}}, position={"x": 300, "y": 400}
                ),
            ],
            edges=[WorkflowEdge(from_node_id="node1", to_node_id="node2", config={"label": "connection1"})],
            metadata={"version": "1.0"},
        )

        frontend_data = CanvasTransformer.to_frontend(canvas)

        assert len(frontend_data["nodes"]) == 2
        assert len(frontend_data["edges"]) == 1

        # Check node conversion
        node1 = frontend_data["nodes"][0]
        assert node1["id"] == "node1"
        assert node1["type"] == "agent"
        assert node1["position"] == {"x": 100, "y": 200}
        assert node1["custom_field"] == "value1"

        # Check edge conversion
        edge = frontend_data["edges"][0]
        assert edge["source"] == "node1"
        assert edge["target"] == "node2"
        assert edge["label"] == "connection1"

        # Check metadata
        assert frontend_data["metadata"]["version"] == "1.0"

    def test_from_database_canonical_format(self):
        """Test loading already canonical data from database."""
        db_data = {
            "nodes": [{"node_id": "node1", "node_type": "agent", "position": {"x": 100, "y": 200}, "config": {}}],
            "edges": [{"from_node_id": "node1", "to_node_id": "node2", "config": {}}],
            "metadata": {},
        }

        canvas = CanvasTransformer.from_database(db_data)

        assert len(canvas.nodes) == 1
        assert canvas.nodes[0].node_id == "node1"
        assert canvas.nodes[0].node_type == "agent"

    def test_from_database_legacy_format(self):
        """Test loading legacy frontend format from database."""
        db_data = {"nodes": [{"id": "node1", "type": "agent"}], "edges": [{"source": "node1", "target": "node2"}]}

        canvas = CanvasTransformer.from_database(db_data)

        assert len(canvas.nodes) == 1
        assert canvas.nodes[0].node_id == "node1"
        assert canvas.nodes[0].node_type == "agent"
        assert len(canvas.edges) == 1
        assert canvas.edges[0].from_node_id == "node1"
        assert canvas.edges[0].to_node_id == "node2"

    def test_to_database(self):
        """Test conversion to database storage format."""
        canvas = WorkflowCanvas(
            nodes=[WorkflowNode(node_id="node1", node_type="agent")],
            edges=[WorkflowEdge(from_node_id="node1", to_node_id="node2")],
            metadata={"version": "1.0"},
        )

        db_data = CanvasTransformer.to_database(canvas)

        # Should be canonical format
        assert "nodes" in db_data
        assert "edges" in db_data
        assert "metadata" in db_data
        assert db_data["nodes"][0]["node_id"] == "node1"
        assert db_data["edges"][0]["from_node_id"] == "node1"

    def test_roundtrip_frontend_to_canonical_to_frontend(self):
        """Test roundtrip transformation preserves data."""
        original_frontend = {
            "nodes": [{"id": "node1", "type": "agent", "position": {"x": 100, "y": 200}, "custom_field": "value1"}],
            "edges": [{"source": "node1", "target": "node2", "label": "connection1"}],
            "metadata": {"version": "1.0"},
        }

        # Transform to canonical and back
        canvas = CanvasTransformer.from_frontend(original_frontend)
        result_frontend = CanvasTransformer.to_frontend(canvas)

        # Should preserve essential data
        assert result_frontend["nodes"][0]["id"] == "node1"
        assert result_frontend["nodes"][0]["type"] == "agent"
        assert result_frontend["nodes"][0]["position"] == {"x": 100, "y": 200}
        assert result_frontend["nodes"][0]["custom_field"] == "value1"

        assert result_frontend["edges"][0]["source"] == "node1"
        assert result_frontend["edges"][0]["target"] == "node2"
        assert result_frontend["edges"][0]["label"] == "connection1"

        assert result_frontend["metadata"]["version"] == "1.0"
