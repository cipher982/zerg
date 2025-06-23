"""Test comprehensive workflow validation system."""

from zerg.services.workflow_validator import WorkflowValidator


class TestWorkflowValidator:
    """Test the workflow validation system."""

    def setup_method(self):
        self.validator = WorkflowValidator()

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

        result = self.validator.validate_workflow(canvas_data)
        assert result.is_valid
        assert len(result.errors) == 0

    def test_duplicate_node_ids(self):
        """Test that duplicate node IDs are caught."""
        canvas_data = {
            "nodes": [
                {"node_id": "node1", "node_type": {"Agent": {}}, "agent_id": 1},
                {"node_id": "node1", "node_type": {"Agent": {}}, "agent_id": 2},  # Duplicate!
            ],
            "edges": [],
        }

        result = self.validator.validate_workflow(canvas_data)
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

        result = self.validator.validate_workflow(canvas_data)
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

        result = self.validator.validate_workflow(canvas_data)
        # Should pass basic validation (may have warnings about no END, etc.)
        tool_errors = [e for e in result.errors if "INVALID_TOOL_NAME" in e.code]
        assert len(tool_errors) == 0

    def test_missing_agent_id(self):
        """Test that agent nodes without agent_id are caught."""
        canvas_data = {
            "nodes": [
                {
                    "node_id": "agent1",
                    "node_type": {"Agent": {}},
                    # Missing agent_id
                }
            ],
            "edges": [],
        }

        result = self.validator.validate_workflow(canvas_data)
        assert not result.is_valid
        assert any(error.code == "MISSING_AGENT_ID" for error in result.errors)

    def test_invalid_edge_references(self):
        """Test that edges referencing non-existent nodes are caught."""
        canvas_data = {
            "nodes": [{"node_id": "node1", "node_type": {"Agent": {}}, "agent_id": 1}],
            "edges": [
                {"from_node_id": "node1", "to_node_id": "nonexistent"}  # Invalid target
            ],
        }

        result = self.validator.validate_workflow(canvas_data)
        assert not result.is_valid
        assert any(error.code == "INVALID_EDGE_TARGET" for error in result.errors)

    def test_too_many_nodes(self):
        """Test that workflows exceeding node limits are caught."""
        # Create workflow with too many nodes
        nodes = []
        for i in range(self.validator.max_nodes + 1):
            nodes.append({"node_id": f"node{i}", "node_type": {"Agent": {}}, "agent_id": 1})

        canvas_data = {"nodes": nodes, "edges": []}

        result = self.validator.validate_workflow(canvas_data)
        assert not result.is_valid
        assert any(error.code == "TOO_MANY_NODES" for error in result.errors)

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

        result = self.validator.validate_workflow(canvas_data)
        assert any(warning.code == "ORPHANED_NODE" for warning in result.warnings)

    def test_cycle_detection_warning(self):
        """Test that cycles generate warnings."""
        canvas_data = {
            "nodes": [
                {"node_id": "trigger1", "node_type": {"Trigger": {}}},
                {"node_id": "agent1", "node_type": {"Agent": {}}, "agent_id": 1},
                {"node_id": "agent2", "node_type": {"Agent": {}}, "agent_id": 2},
            ],
            "edges": [
                {"from_node_id": "trigger1", "to_node_id": "agent1"},
                {"from_node_id": "agent1", "to_node_id": "agent2"},
                {"from_node_id": "agent2", "to_node_id": "agent1"},  # Creates cycle
            ],
        }

        result = self.validator.validate_workflow(canvas_data)
        assert any(warning.code == "POTENTIAL_CYCLE" for warning in result.warnings)

    def test_malformed_canvas_data(self):
        """Test that malformed canvas data is handled gracefully."""
        result = self.validator.validate_workflow("not a dict")
        assert not result.is_valid
        assert any(error.code == "INVALID_CANVAS_DATA" for error in result.errors)


# Property-based testing would go here but requires hypothesis
# Commenting out for now to avoid dependency issues
# TODO: Add hypothesis dependency and enable property-based tests


class TestIntegrationWithLangGraph:
    """Test integration with LangGraph validation."""

    def setup_method(self):
        self.validator = WorkflowValidator()

    def test_langgraph_entrypoint_validation(self):
        """Test that LangGraph's entrypoint validation is caught."""
        # Create workflow without trigger (no entrypoint)
        canvas_data = {"nodes": [{"node_id": "agent1", "agent_id": 1, "message": "test"}], "edges": []}

        result = self.validator.validate_workflow(canvas_data)
        # Should have both our warning and LangGraph's error
        assert any(warning.code == "NO_TRIGGER_NODE" for warning in result.warnings)
        # LangGraph validation might also catch this as missing entrypoint

    def test_langgraph_unknown_node_validation(self):
        """Test that LangGraph catches unknown node references."""
        # This should be caught by our edge validation first,
        # but LangGraph provides a backup
        canvas_data = {
            "nodes": [{"node_id": "trigger1", "node_type": {"Trigger": {}}}],
            "edges": [{"from_node_id": "trigger1", "to_node_id": "unknown"}],
        }

        result = self.validator.validate_workflow(canvas_data)
        assert not result.is_valid
        # Should be caught by our validation or LangGraph's
        assert any("INVALID_EDGE_TARGET" in error.code or "UNKNOWN_NODE" in error.code for error in result.errors)
