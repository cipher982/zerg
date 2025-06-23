"""
Canvas data transformation service.

Handles conversion between external formats (frontend, API) and internal
canonical schema. This is the single point where format differences are resolved.
"""

from __future__ import annotations

import logging
from typing import Any
from typing import Dict
from typing import Union

from pydantic import ValidationError

from zerg.schemas.workflow_schema import FrontendEdge
from zerg.schemas.workflow_schema import FrontendNode
from zerg.schemas.workflow_schema import InputCanvas
from zerg.schemas.workflow_schema import LegacyStringNode
from zerg.schemas.workflow_schema import WorkflowCanvas
from zerg.schemas.workflow_schema import WorkflowEdge
from zerg.schemas.workflow_schema import WorkflowNode

logger = logging.getLogger(__name__)


class CanvasTransformer:
    """Transform external formats to internal canonical schema."""

    @staticmethod
    def from_frontend(frontend_data: Union[Dict[str, Any], Any]) -> WorkflowCanvas:
        """
        Convert frontend canvas format to internal canonical schema using clean Pydantic validation.

        This method handles multiple frontend formats:
        - Standard frontend: {"nodes": [{"id": "x", "type": "y"}], "edges": [{"source": "a", "target": "b"}]}
        - Legacy string nodes: {"nodes": ["node1", "node2"]}
        - Mixed formats in the same payload

        All defensive programming is replaced with schema validation.
        """
        try:
            # First, validate as input canvas to handle basic structure
            if not isinstance(frontend_data, dict):
                logger.warning(f"Invalid frontend data format: {type(frontend_data)}, returning empty canvas")
                return WorkflowCanvas()

            # Create a cleaned version that filters out invalid items upfront
            cleaned_data = {
                "nodes": [
                    node for node in frontend_data.get("nodes", []) if isinstance(node, (dict, str))
                ],  # Keep dict and string nodes only
                "edges": [
                    edge for edge in frontend_data.get("edges", []) if isinstance(edge, dict)
                ],  # Keep dict edges only
                "metadata": frontend_data.get("metadata", {}),
            }

            input_canvas = InputCanvas(**cleaned_data)

            # Transform nodes using schema validation
            canonical_nodes = []
            for node_data in input_canvas.nodes:
                try:
                    if isinstance(node_data, str):
                        # Handle legacy string format - the model validator will convert string to dict
                        legacy_node = LegacyStringNode.model_validate(node_data)
                        canonical_nodes.append(
                            WorkflowNode(
                                node_id=legacy_node.node_id,
                                node_type=legacy_node.node_type,
                                position=legacy_node.position,
                                config={},
                            )
                        )
                        logger.debug(f"Converted legacy string node: {node_data}")
                    else:
                        # Handle frontend dictionary format
                        frontend_node = FrontendNode(**node_data)

                        # Extract config (everything except known fields)
                        config = {
                            k: v
                            for k, v in node_data.items()
                            if k not in ["id", "node_id", "type", "node_type", "position"]
                        }

                        # Flatten 'data' field into config if it exists
                        if "data" in config and isinstance(config["data"], dict):
                            data_content = config.pop("data")
                            config.update(data_content)

                        canonical_nodes.append(
                            WorkflowNode(
                                node_id=frontend_node.node_id,
                                node_type=frontend_node.node_type,
                                position=frontend_node.position,
                                config=config,
                            )
                        )

                except ValidationError as e:
                    logger.warning(f"Skipping invalid node data: {node_data}, error: {e}")
                    continue

            # Transform edges using schema validation
            canonical_edges = []
            for edge_data in input_canvas.edges:
                try:
                    frontend_edge = FrontendEdge(**edge_data)

                    # Extract config (everything except known fields)
                    config = {
                        k: v
                        for k, v in edge_data.items()
                        if k not in ["source", "target", "from_node_id", "to_node_id", "from", "to"]
                    }

                    canonical_edges.append(
                        WorkflowEdge(
                            from_node_id=frontend_edge.from_node_id, to_node_id=frontend_edge.to_node_id, config=config
                        )
                    )

                except ValidationError as e:
                    logger.warning(f"Skipping invalid edge data: {edge_data}, error: {e}")
                    continue

            canvas = WorkflowCanvas(nodes=canonical_nodes, edges=canonical_edges, metadata=input_canvas.metadata)

            logger.info(f"Transformed frontend data: {len(canonical_nodes)} nodes, {len(canonical_edges)} edges")
            return canvas

        except ValidationError as e:
            logger.error(f"Failed to parse input canvas: {e}")
            return WorkflowCanvas()
        except Exception as e:
            logger.error(f"Unexpected error in frontend transformation: {e}")
            return WorkflowCanvas()

    @staticmethod
    def to_frontend(canvas: WorkflowCanvas) -> Dict[str, Any]:
        """
        Convert internal canonical schema to frontend format.

        This is used when sending data back to the frontend.
        """
        return {
            "nodes": [
                {
                    "id": node.node_id,
                    "type": node.node_type,
                    "position": node.position,
                    **node.config,  # Spread additional config fields
                }
                for node in canvas.nodes
            ],
            "edges": [
                {
                    "source": edge.from_node_id,
                    "target": edge.to_node_id,
                    **edge.config,  # Spread additional config fields
                }
                for edge in canvas.edges
            ],
            "metadata": canvas.metadata,
        }

    @staticmethod
    def from_database(db_data: Union[Dict[str, Any], Any]) -> WorkflowCanvas:
        """
        Convert database stored format to internal canonical schema.

        Database might store either format depending on when it was saved.
        Uses schema validation instead of defensive programming.
        """
        try:
            # First try to parse as canonical format (new database entries)
            canvas = WorkflowCanvas(**db_data)
            logger.debug("Database data is already in canonical format")
            return canvas

        except ValidationError:
            # If that fails, try to parse as frontend format (legacy database entries)
            logger.debug("Database data appears to be legacy format, transforming...")
            return CanvasTransformer.from_frontend(db_data)

        except Exception as e:
            logger.error(f"Unexpected error parsing database data: {e}")
            return WorkflowCanvas()

    @staticmethod
    def to_database(canvas: WorkflowCanvas) -> Dict[str, Any]:
        """
        Convert internal canonical schema to database storage format.

        We store the canonical format in the database going forward.
        """
        return canvas.model_dump()
