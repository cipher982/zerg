"""
Database Serialization for Canonical Types

Handles conversion between canonical types and database storage format.
This maintains the single source of truth principle while providing
backward compatibility with existing database schema.
"""

from __future__ import annotations

import logging
from typing import Any
from typing import Dict

from .canonical_types import CanonicalEdge
from .canonical_types import CanonicalNode
from .canonical_types import CanonicalWorkflow
from .canonical_validators import validate_workflow_json

logger = logging.getLogger(__name__)


class CanonicalSerializer:
    """
    Converts between canonical types and database storage format.

    Key principles:
    1. Database format is JSON-compatible for existing schema
    2. Canonical types are the source of truth
    3. Serialization is lossless (round-trip safe)
    4. Forward and backward compatible
    """

    @staticmethod
    def workflow_to_database(workflow: CanonicalWorkflow) -> Dict[str, Any]:
        """
        Convert CanonicalWorkflow to database storage format.

        Returns JSON-compatible dict that can be stored in database
        canvas_data field.
        """
        return {
            "id": workflow.id,
            "name": workflow.name,
            "nodes": [CanonicalSerializer._node_to_dict(node) for node in workflow.nodes],
            "edges": [CanonicalSerializer._edge_to_dict(edge) for edge in workflow.edges],
        }

    @staticmethod
    def workflow_from_database(
        db_data: Dict[str, Any], workflow_id: int = None, workflow_name: str = None
    ) -> CanonicalWorkflow:
        """
        Convert database storage format to CanonicalWorkflow.

        Uses the validation system to ensure clean conversion.
        Handles legacy formats automatically.
        """
        # Ensure required fields are present
        if workflow_id is not None:
            db_data = {**db_data, "id": workflow_id}
        if workflow_name is not None:
            db_data = {**db_data, "name": workflow_name}

        # Use validation system for robust conversion
        return validate_workflow_json(db_data)

    @staticmethod
    def _node_to_dict(node: CanonicalNode) -> Dict[str, Any]:
        """Convert CanonicalNode to dict format."""
        base_dict = {"node_id": node.id.value, "position": {"x": node.position.x, "y": node.position.y}}

        # Add type-specific data
        if node.is_agent:
            base_dict.update(
                {
                    "node_type": "agent",
                    "config": {"agent_id": node.agent_data.agent_id.value, "message": node.agent_data.message},
                }
            )
        elif node.is_tool:
            base_dict.update(
                {
                    "node_type": "tool",
                    "config": {"tool_name": node.tool_data.tool_name, "parameters": node.tool_data.parameters},
                }
            )
        elif node.is_trigger:
            # Create a copy of config and add trigger_type to it for database storage
            # This maintains compatibility with the validator that expects trigger_type in config
            config_copy = dict(node.trigger_data.config)
            config_copy["trigger_type"] = node.trigger_data.trigger_type
            base_dict.update({"node_type": "trigger", "config": config_copy})

        return base_dict

    @staticmethod
    def _edge_to_dict(edge: CanonicalEdge) -> Dict[str, Any]:
        """Convert CanonicalEdge to dict format."""
        return {"from_node_id": edge.from_node_id.value, "to_node_id": edge.to_node_id.value, "config": edge.config}


# ============================================================================
# Convenience Functions for Database Operations
# ============================================================================


def serialize_workflow_for_database(workflow: CanonicalWorkflow) -> Dict[str, Any]:
    """
    Main function for converting canonical workflow to database format.

    Use this in database save operations.
    """
    return CanonicalSerializer.workflow_to_database(workflow)


def deserialize_workflow_from_database(
    canvas_data: Dict[str, Any], workflow_id: int, workflow_name: str
) -> CanonicalWorkflow:
    """
    Main function for converting database format to canonical workflow.

    Use this in database load operations.
    """
    return CanonicalSerializer.workflow_from_database(canvas_data, workflow_id, workflow_name)


def serialize_node_for_database(node: CanonicalNode) -> Dict[str, Any]:
    """Convert single node to database format."""
    return CanonicalSerializer._node_to_dict(node)


def serialize_edge_for_database(edge: CanonicalEdge) -> Dict[str, Any]:
    """Convert single edge to database format."""
    return CanonicalSerializer._edge_to_dict(edge)


# ============================================================================
# Database Integration Helpers
# ============================================================================


class DatabaseWorkflowAdapter:
    """
    Adapter for integrating canonical types with existing database models.

    This bridges the gap between the new canonical system and existing
    database/ORM code without requiring immediate migration.
    """

    @staticmethod
    def create_workflow_from_request(
        workflow_id: int, workflow_name: str, canvas_data: Dict[str, Any]
    ) -> CanonicalWorkflow:
        """
        Create canonical workflow from API request data.

        This is the entry point for new workflow creation.
        """
        # Add workflow metadata to canvas data
        full_data = {"id": workflow_id, "name": workflow_name, **canvas_data}

        # Validate and convert to canonical format
        return validate_workflow_json(full_data)

    @staticmethod
    def load_workflow_from_database(
        workflow_id: int, workflow_name: str, canvas_data: Dict[str, Any]
    ) -> CanonicalWorkflow:
        """
        Load canonical workflow from database.

        This is the entry point for workflow retrieval.
        """
        return deserialize_workflow_from_database(canvas_data, workflow_id, workflow_name)

    @staticmethod
    def save_workflow_to_database(workflow: CanonicalWorkflow) -> Dict[str, Any]:
        """
        Prepare canonical workflow for database storage.

        Returns the canvas_data dict to be stored in database.
        """
        serialized = serialize_workflow_for_database(workflow)

        # Remove workflow-level metadata (stored in separate DB fields)
        canvas_data = {"nodes": serialized["nodes"], "edges": serialized["edges"]}

        return canvas_data

    @staticmethod
    def update_workflow_canvas(
        existing_workflow: CanonicalWorkflow, canvas_updates: Dict[str, Any]
    ) -> CanonicalWorkflow:
        """
        Update workflow canvas data while maintaining canonical format.

        This handles partial updates to workflow structure.
        """
        # Serialize existing workflow
        current_data = serialize_workflow_for_database(existing_workflow)

        # Apply updates
        updated_data = {**current_data, **canvas_updates}

        # Validate and convert back to canonical format
        return validate_workflow_json(updated_data)
