"""
TDD Tests for Canonical Validators

These tests ensure the validation system correctly converts
messy input data into clean canonical types while providing
clear error messages for invalid input.
"""

import pytest

from zerg.schemas.canonical_validators import ValidationError
from zerg.schemas.canonical_validators import validate_edge_json
from zerg.schemas.canonical_validators import validate_node_json
from zerg.schemas.canonical_validators import validate_workflow_json

# ============================================================================
# Test Data Fixtures
# ============================================================================


@pytest.fixture
def valid_agent_node_json():
    """Clean agent node JSON."""
    return {
        "id": "agent-1",
        "type": "agent",
        "position": {"x": 100, "y": 200},
        "agent_id": 42,
        "message": "Do the task",
    }


@pytest.fixture
def valid_tool_node_json():
    """Clean tool node JSON."""
    return {
        "id": "tool-1",
        "type": "tool",
        "position": {"x": 200, "y": 300},
        "tool_name": "http_request",
        "parameters": {"url": "https://example.com"},
    }


@pytest.fixture
def valid_trigger_node_json():
    """Clean trigger node JSON."""
    return {"id": "trigger-1", "type": "trigger", "position": {"x": 0, "y": 0}, "trigger_type": "manual", "config": {}}


@pytest.fixture
def valid_workflow_json(valid_agent_node_json, valid_tool_node_json, valid_trigger_node_json):
    """Complete valid workflow JSON."""
    return {
        "id": 1,
        "name": "Test Workflow",
        "nodes": [valid_trigger_node_json, valid_agent_node_json, valid_tool_node_json],
        "edges": [{"from": "trigger-1", "to": "agent-1"}, {"from": "agent-1", "to": "tool-1"}],
    }


# ============================================================================
# Workflow Validation Tests
# ============================================================================


class TestWorkflowValidation:
    """Test complete workflow validation."""

    def test_valid_workflow_success(self, valid_workflow_json):
        """Valid workflow should convert successfully."""
        workflow = validate_workflow_json(valid_workflow_json)

        assert workflow.id == 1
        assert workflow.name == "Test Workflow"
        assert len(workflow.nodes) == 3
        assert len(workflow.edges) == 2

        # Check nodes converted correctly with direct access
        agent_nodes = workflow.get_agent_nodes()
        assert len(agent_nodes) == 1
        assert agent_nodes[0].agent_id.value == 42  # Direct field access!

        tool_nodes = workflow.get_tool_nodes()
        assert len(tool_nodes) == 1
        assert tool_nodes[0].tool_name == "http_request"  # Direct field access!

    def test_workflow_missing_id_fails(self, valid_workflow_json):
        """Workflow without ID should fail."""
        del valid_workflow_json["id"]

        with pytest.raises(ValidationError) as exc_info:
            validate_workflow_json(valid_workflow_json)

        assert "Missing required field 'id'" in str(exc_info.value)
        assert "workflow.id" in str(exc_info.value)

    def test_workflow_invalid_id_fails(self, valid_workflow_json):
        """Workflow with invalid ID should fail."""
        valid_workflow_json["id"] = "not-a-number"

        with pytest.raises(ValidationError) as exc_info:
            validate_workflow_json(valid_workflow_json)

        assert "must be positive integer" in str(exc_info.value)
        assert "workflow.id" in str(exc_info.value)

    def test_workflow_missing_name_fails(self, valid_workflow_json):
        """Workflow without name should fail."""
        del valid_workflow_json["name"]

        with pytest.raises(ValidationError) as exc_info:
            validate_workflow_json(valid_workflow_json)

        assert "Missing required field 'name'" in str(exc_info.value)

    def test_workflow_empty_name_fails(self, valid_workflow_json):
        """Workflow with empty name should fail."""
        valid_workflow_json["name"] = "   "

        with pytest.raises(ValidationError) as exc_info:
            validate_workflow_json(valid_workflow_json)

        assert "must be non-empty string" in str(exc_info.value)

    def test_workflow_invalid_nodes_type_fails(self, valid_workflow_json):
        """Workflow with non-array nodes should fail."""
        valid_workflow_json["nodes"] = "not-an-array"

        with pytest.raises(ValidationError) as exc_info:
            validate_workflow_json(valid_workflow_json)

        assert "Nodes must be array" in str(exc_info.value)
        assert "workflow.nodes" in str(exc_info.value)


# ============================================================================
# Node Validation Tests - Clean Format
# ============================================================================


class TestNodeValidationClean:
    """Test node validation with clean input formats."""

    def test_agent_node_clean_format(self, valid_agent_node_json):
        """Clean agent node should validate successfully."""
        node = validate_node_json(valid_agent_node_json)

        assert node.id.value == "agent-1"
        assert node.is_agent is True
        assert node.agent_id.value == 42  # Direct access!
        assert node.agent_data.message == "Do the task"
        assert node.position.x == 100
        assert node.position.y == 200

    def test_tool_node_clean_format(self, valid_tool_node_json):
        """Clean tool node should validate successfully."""
        node = validate_node_json(valid_tool_node_json)

        assert node.id.value == "tool-1"
        assert node.is_tool is True
        assert node.tool_name == "http_request"  # Direct access!
        assert node.tool_data.parameters["url"] == "https://example.com"

    def test_trigger_node_clean_format(self, valid_trigger_node_json):
        """Clean trigger node should validate successfully."""
        node = validate_node_json(valid_trigger_node_json)

        assert node.id.value == "trigger-1"
        assert node.is_trigger is True
        assert node.trigger_data.trigger_type == "manual"


# ============================================================================
# Node Validation Tests - Legacy Formats
# ============================================================================


class TestNodeValidationLegacy:
    """Test that validation handles messy legacy input formats."""

    def test_agent_node_config_format(self):
        """Agent node with agent_id in config should work."""
        messy_input = {
            "id": "agent-1",
            "type": "agentidentity",  # Different case
            "position": {"x": 100, "y": 200},
            "config": {
                "agent_id": 42,  # agent_id in nested config
                "other_field": "ignored",
            },
            "message": "Test message",
        }

        node = validate_node_json(messy_input)
        assert node.agent_id.value == 42  # Should extract correctly
        assert node.agent_data.message == "Test message"

    def test_agent_node_type_dict_format(self):
        """Agent node with agent_id in type dict should work."""
        messy_input = {
            "id": "agent-1",
            "type": {
                "AgentIdentity": {  # Legacy format
                    "agent_id": 42
                }
            },
            "position": {"x": 100, "y": 200},
            "message": "Test message",
        }

        node = validate_node_json(messy_input)
        assert node.agent_id.value == 42
        assert node.is_agent is True

    def test_tool_node_config_format(self):
        """Tool node with tool_name in config should work."""
        messy_input = {
            "id": "tool-1",
            "type": "Tool",  # Different case
            "position": {"x": 200, "y": 300},
            "config": {"tool_name": "http_request", "parameters": {"method": "POST"}},
        }

        node = validate_node_json(messy_input)
        assert node.tool_name == "http_request"
        assert node.tool_data.parameters["method"] == "POST"

    def test_node_id_field_variations(self):
        """Should handle both 'id' and 'node_id' fields."""
        # Test with 'node_id' instead of 'id'
        input_data = {
            "node_id": "test-node",  # Different field name
            "type": "agent",
            "position": {"x": 0, "y": 0},
            "agent_id": 1,
        }

        node = validate_node_json(input_data)
        assert node.id.value == "test-node"

    def test_position_defaults(self):
        """Missing position coordinates should default to 0."""
        input_data = {
            "id": "test-node",
            "type": "agent",
            "agent_id": 1,
            # No position field
        }

        node = validate_node_json(input_data)
        assert node.position.x == 0
        assert node.position.y == 0

    def test_position_partial(self):
        """Partial position should default missing coordinates."""
        input_data = {
            "id": "test-node",
            "type": "agent",
            "agent_id": 1,
            "position": {"x": 100},  # Missing y
        }

        node = validate_node_json(input_data)
        assert node.position.x == 100
        assert node.position.y == 0


# ============================================================================
# Node Validation Error Tests
# ============================================================================


class TestNodeValidationErrors:
    """Test that validation fails appropriately with clear errors."""

    def test_node_missing_id_fails(self):
        """Node without ID should fail."""
        input_data = {"type": "agent", "position": {"x": 0, "y": 0}, "agent_id": 1}

        with pytest.raises(ValidationError) as exc_info:
            validate_node_json(input_data)

        assert "Missing required field 'id'" in str(exc_info.value)

    def test_node_empty_id_fails(self):
        """Node with empty ID should fail."""
        input_data = {
            "id": "   ",  # Empty after strip
            "type": "agent",
            "agent_id": 1,
        }

        with pytest.raises(ValidationError) as exc_info:
            validate_node_json(input_data)

        assert "must be non-empty string" in str(exc_info.value)

    def test_node_missing_type_fails(self):
        """Node without type should fail."""
        input_data = {"id": "test-node", "position": {"x": 0, "y": 0}, "agent_id": 1}

        with pytest.raises(ValidationError) as exc_info:
            validate_node_json(input_data)

        assert "Missing required field 'type'" in str(exc_info.value)

    def test_node_unknown_type_fails(self):
        """Node with unknown type should fail."""
        input_data = {"id": "test-node", "type": "unknown_type", "position": {"x": 0, "y": 0}}

        with pytest.raises(ValidationError) as exc_info:
            validate_node_json(input_data)

        assert "Unknown node type: unknown_type" in str(exc_info.value)
        assert "Supported types: agent, tool, trigger" in str(exc_info.value)

    def test_agent_node_missing_agent_id_fails(self):
        """Agent node without agent_id should fail."""
        input_data = {
            "id": "agent-1",
            "type": "agent",
            "position": {"x": 0, "y": 0},
            "message": "Test",
            # Missing agent_id
        }

        with pytest.raises(ValidationError) as exc_info:
            validate_node_json(input_data)

        assert "Missing required field 'agent_id'" in str(exc_info.value)
        assert "Must be in root, config, or type object" in str(exc_info.value)

    def test_agent_node_invalid_agent_id_fails(self):
        """Agent node with invalid agent_id should fail."""
        input_data = {"id": "agent-1", "type": "agent", "position": {"x": 0, "y": 0}, "agent_id": "not-a-number"}

        with pytest.raises(ValidationError) as exc_info:
            validate_node_json(input_data)

        assert "Agent ID must be positive integer" in str(exc_info.value)

    def test_tool_node_missing_tool_name_fails(self):
        """Tool node without tool_name should fail."""
        input_data = {
            "id": "tool-1",
            "type": "tool",
            "position": {"x": 0, "y": 0},
            # Missing tool_name
        }

        with pytest.raises(ValidationError) as exc_info:
            validate_node_json(input_data)

        assert "Missing required field 'tool_name'" in str(exc_info.value)

    def test_invalid_position_coordinates_fail(self):
        """Invalid position coordinates should fail."""
        input_data = {"id": "test-node", "type": "agent", "position": {"x": "not-a-number", "y": 0}, "agent_id": 1}

        with pytest.raises(ValidationError) as exc_info:
            validate_node_json(input_data)

        assert "Invalid position coordinates" in str(exc_info.value)


# ============================================================================
# Edge Validation Tests
# ============================================================================


class TestEdgeValidation:
    """Test edge validation with different input formats."""

    def test_edge_clean_format(self):
        """Clean edge format should validate successfully."""
        edge_data = {"from": "node-1", "to": "node-2", "config": {"label": "test"}}

        edge = validate_edge_json(edge_data)
        assert edge.from_node_id.value == "node-1"
        assert edge.to_node_id.value == "node-2"
        assert edge.config["label"] == "test"

    def test_edge_field_variations(self):
        """Should handle different field name formats."""
        # Test source/target format
        edge_data = {"source": "node-1", "target": "node-2"}

        edge = validate_edge_json(edge_data)
        assert edge.from_node_id.value == "node-1"
        assert edge.to_node_id.value == "node-2"

        # Test from_node_id/to_node_id format
        edge_data = {"from_node_id": "node-1", "to_node_id": "node-2"}

        edge = validate_edge_json(edge_data)
        assert edge.from_node_id.value == "node-1"
        assert edge.to_node_id.value == "node-2"

    def test_edge_missing_config_defaults(self):
        """Missing config should default to empty dict."""
        edge_data = {
            "from": "node-1",
            "to": "node-2",
            # No config
        }

        edge = validate_edge_json(edge_data)
        assert edge.config == {}

    def test_edge_missing_from_fails(self):
        """Edge without source should fail."""
        edge_data = {
            "to": "node-2"
            # Missing from/source
        }

        with pytest.raises(ValidationError) as exc_info:
            validate_edge_json(edge_data)

        assert "Missing required field: 'from', 'from_node_id', or 'source'" in str(exc_info.value)

    def test_edge_missing_to_fails(self):
        """Edge without target should fail."""
        edge_data = {
            "from": "node-1"
            # Missing to/target
        }

        with pytest.raises(ValidationError) as exc_info:
            validate_edge_json(edge_data)

        assert "Missing required field: 'to', 'to_node_id', or 'target'" in str(exc_info.value)


# ============================================================================
# Error Context Tests
# ============================================================================


class TestErrorContext:
    """Test that validation errors provide good context."""

    def test_nested_error_context(self, valid_workflow_json):
        """Errors in nested structures should show full path."""
        # Break the second node's agent_id
        valid_workflow_json["nodes"][1]["agent_id"] = "invalid"

        with pytest.raises(ValidationError) as exc_info:
            validate_workflow_json(valid_workflow_json)

        error_str = str(exc_info.value)
        assert "workflow.nodes[1].agent_id" in error_str
        assert "must be positive integer" in error_str

    def test_edge_error_context(self, valid_workflow_json):
        """Errors in edges should show full path."""
        # Break the first edge
        valid_workflow_json["edges"][0]["from"] = ""

        with pytest.raises(ValidationError) as exc_info:
            validate_workflow_json(valid_workflow_json)

        error_str = str(exc_info.value)
        assert "workflow.edges[0].from" in error_str
        assert "Missing required field" in error_str  # Empty string treated as missing


# ============================================================================
# Integration Tests - Real World Scenarios
# ============================================================================


class TestRealWorldScenarios:
    """Test validation with real-world messy input data."""

    def test_frontend_workflow_format(self):
        """Test validation with actual frontend-generated data."""
        frontend_data = {
            "id": 1,
            "name": "Frontend Generated Workflow",
            "nodes": [
                {
                    "id": "trigger_123",
                    "type": {"Trigger": {"trigger_type": "Manual", "config": {}}},
                    "position": {"x": 50, "y": 100},
                },
                {
                    "node_id": "agent_456",  # Different ID field name
                    "type": "AgentIdentity",
                    "position": {"x": 250, "y": 100},
                    "agent_id": 789,
                    "message": "Process the request",
                },
                {
                    "id": "tool_789",
                    "type": "Tool",
                    "position": {"x": 450, "y": 100},
                    "config": {
                        "tool_name": "http_request",
                        "parameters": {"method": "GET", "url": "https://api.example.com"},
                    },
                },
            ],
            "edges": [
                {"source": "trigger_123", "target": "agent_456"},
                {"from_node_id": "agent_456", "to_node_id": "tool_789"},
            ],
        }

        # Should handle all these variations successfully
        workflow = validate_workflow_json(frontend_data)

        assert workflow.id == 1
        assert len(workflow.nodes) == 3
        assert len(workflow.edges) == 2

        # Verify direct field access works
        agent_nodes = workflow.get_agent_nodes()
        assert len(agent_nodes) == 1
        assert agent_nodes[0].agent_id.value == 789

        tool_nodes = workflow.get_tool_nodes()
        assert len(tool_nodes) == 1
        assert tool_nodes[0].tool_name == "http_request"

    def test_database_legacy_format(self):
        """Test validation with legacy database format."""
        legacy_data = {
            "id": 2,
            "name": "Legacy Workflow",
            "nodes": [
                {
                    "id": "old_agent",
                    "type": {"AgentIdentity": {"agent_id": 123}},  # Nested format
                    "position": {"x": 100, "y": 200},
                    "data": {  # Legacy data wrapper
                        "message": "Legacy message"
                    },
                }
            ],
            "edges": [],
        }

        workflow = validate_workflow_json(legacy_data)

        agent_nodes = workflow.get_agent_nodes()
        assert len(agent_nodes) == 1
        assert agent_nodes[0].agent_id.value == 123
        # Note: Legacy data wrapper handling would need additional logic
        # if we wanted to support it, but for now we expect clean input
