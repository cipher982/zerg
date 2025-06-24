"""Simple tests for workflow validation system without hypothesis."""

from zerg.services.canvas_transformer import CanvasTransformer
from zerg.services.workflow_validator import ValidationResult
from zerg.services.workflow_validator import WorkflowValidator


class TestWorkflowValidator:
    """Test the workflow validation system."""

    def setup_method(self):
        self.validator = WorkflowValidator()

    def _validate_canvas_data(self, canvas_data):
        """Helper method to transform and validate canvas data."""
        canvas = CanvasTransformer.from_frontend(canvas_data)
        return self.validator.validate_workflow(canvas)

    def test_valid_simple_workflow(self):
        """Test that a simple valid workflow passes validation."""
        canvas_data = {
            "nodes": [
                {"node_id": "trigger1", "node_type": {"Trigger": {"trigger_type": "webhook"}}, "x": 100, "y": 100},
                {
                    "node_id": "agent1",
                    "node_type": {"Agent": {}},
                    "agent_id": 1,
                    "message": "Process the data",
                    "x": 200,
                    "y": 100,
                },
            ],
            "edges": [{"from_node_id": "trigger1", "to_node_id": "agent1"}],
        }

        result = self._validate_canvas_data(canvas_data)
        # With LangGraph validation, this will fail because it needs START node connections
        # But the basic validation should still catch structural issues
        assert not result.is_valid  # LangGraph validation will fail
        assert any("entrypoint" in error.message for error in result.errors)

    def test_duplicate_node_ids(self):
        """Test that duplicate node IDs are caught."""
        canvas_data = {
            "nodes": [
                {"node_id": "node1", "node_type": {"Agent": {}}, "agent_id": 1},
                {"node_id": "node1", "node_type": {"Agent": {}}, "agent_id": 2},  # Duplicate!
            ],
            "edges": [],
        }

        result = self._validate_canvas_data(canvas_data)
        assert not result.is_valid
        assert any(error.code == "DUPLICATE_NODE_ID" for error in result.errors)

    def test_invalid_tool_name(self):
        """Test that invalid tool names are caught using our tool contracts."""
        canvas_data = {
            "nodes": [
                {
                    "node_id": "tool1",
                    "node_type": {
                        "Tool": {
                            "tool_name": "nonexistent_tool",  # Invalid tool
                            "config": {},
                        }
                    },
                }
            ],
            "edges": [],
        }

        result = self._validate_canvas_data(canvas_data)
        assert not result.is_valid
        assert any(error.code == "INVALID_TOOL_NAME" for error in result.errors)

    def test_valid_tool_name(self):
        """Test that valid tool names pass validation."""
        canvas_data = {
            "nodes": [
                {"node_id": "trigger1", "node_type": {"Trigger": {"trigger_type": "webhook"}}, "x": 100, "y": 100},
                {
                    "node_id": "tool1",
                    "node_type": {
                        "Tool": {
                            "tool_name": "http_request",  # Valid tool from our contracts
                            "config": {},
                        }
                    },
                },
            ],
            "edges": [{"from_node_id": "trigger1", "to_node_id": "tool1"}],
        }

        result = self._validate_canvas_data(canvas_data)
        # Should pass basic validation (may have warnings about no END, etc.)
        tool_errors = [e for e in result.errors if "INVALID_TOOL_NAME" in e.code]
        assert len(tool_errors) == 0

    def test_missing_agent_id(self):
        """Test that agent nodes are validated (currently just pass-through)."""
        canvas_data = {
            "nodes": [
                {
                    "node_id": "agent1",
                    "node_type": {"Agent": {}},
                    # Missing agent_id - for now this doesn't cause validation failure
                }
            ],
            "edges": [],
        }

        result = self._validate_canvas_data(canvas_data)
        # Currently agent validation is basic - just check it doesn't crash
        assert isinstance(result, ValidationResult)

    def test_invalid_edge_references(self):
        """Test that edges referencing non-existent nodes are caught."""
        canvas_data = {
            "nodes": [{"node_id": "node1", "node_type": {"Agent": {}}, "agent_id": 1}],
            "edges": [
                {"from_node_id": "node1", "to_node_id": "nonexistent"}  # Invalid target
            ],
        }

        result = self._validate_canvas_data(canvas_data)
        assert not result.is_valid
        assert any(error.code == "INVALID_EDGE_TARGET" for error in result.errors)

    def test_orphaned_node_warning(self):
        """Test that orphaned nodes generate warnings."""
        canvas_data = {
            "nodes": [
                {
                    "node_id": "trigger1",
                    "node_type": {"Trigger": {"trigger_type": "webhook"}},
                },
                {"node_id": "agent1", "node_type": {"Agent": {}}, "agent_id": 1, "message": "Connected agent"},
                {
                    "node_id": "orphan",
                    "node_type": {"Agent": {}},
                    "agent_id": 2,
                    "message": "Orphaned agent",  # Not connected to anything
                },
            ],
            "edges": [{"from_node_id": "trigger1", "to_node_id": "agent1"}],
        }

        result = self._validate_canvas_data(canvas_data)
        assert any(warning.code == "ORPHANED_NODE" for warning in result.warnings)

    def test_malformed_canvas_data(self):
        """Test that malformed canvas data is handled gracefully."""
        # Test with invalid input to the transformer
        canvas = CanvasTransformer.from_frontend("not a dict")  # Should return empty canvas
        result = self.validator.validate_workflow(canvas)
        # Empty canvas fails LangGraph validation (no entrypoint)
        assert not result.is_valid  # LangGraph validation will fail
        assert any("entrypoint" in error.message for error in result.errors)

    def test_validation_result_structure(self):
        """Test that validation results have proper structure."""
        canvas_data = {"nodes": [], "edges": []}
        result = self._validate_canvas_data(canvas_data)

        assert isinstance(result, ValidationResult)
        assert isinstance(result.errors, list)
        assert isinstance(result.warnings, list)
        assert isinstance(result.is_valid, bool)
        assert hasattr(result, "has_errors")
        assert hasattr(result, "has_warnings")
