"""
Enhanced Variable Resolution System

Provides type-preserving variable resolution with alias support and backward compatibility.
Replaces the string-based variable resolution with robust object handling.
"""

import logging
import re
from typing import Any
from typing import Dict
from typing import List
from typing import Tuple

from zerg.schemas.node_output import is_envelope_format

logger = logging.getLogger(__name__)


class VariableResolutionError(Exception):
    """Base exception for variable resolution errors."""

    pass


class NodeNotFoundError(VariableResolutionError):
    """Raised when referenced node doesn't exist."""

    pass


class FieldNotFoundError(VariableResolutionError):
    """Raised when referenced field doesn't exist in node output."""

    pass


class InvalidPathError(VariableResolutionError):
    """Raised when variable path is malformed."""

    pass


class VariableResolver:
    """
    Enhanced variable resolver with type preservation and aliases.

    Features:
    - Type preservation: Maintains original Python types (int, bool, dict, etc.)
    - Deep nesting: Supports arbitrary depth (node.result.field.subfield)
    - Alias support: ${node.result} → ${node.value} mapping
    - Backward compatibility: Handles legacy output formats
    - Error handling: Clear, helpful error messages
    """

    def __init__(self):
        """Initialize resolver with alias configuration."""
        self.aliases = {
            "result": "value",  # ${node.result} → ${node.value}
        }

    def resolve_variables(self, data: Any, node_outputs: Dict[str, Any]) -> Any:
        """
        Resolve variables in data structures while preserving types.

        Process:
        1. Recursively traverse data structure
        2. Find strings containing ${...} patterns
        3. Resolve each variable to actual value
        4. Preserve types for pure variables, stringify for interpolation

        Args:
            data: Data structure containing variable references
            node_outputs: Dictionary of node_id -> output mappings

        Returns:
            Data structure with variables resolved
        """
        if isinstance(data, str):
            return self._resolve_string_variables(data, node_outputs)
        elif isinstance(data, dict):
            return {key: self.resolve_variables(value, node_outputs) for key, value in data.items()}
        elif isinstance(data, list):
            return [self.resolve_variables(item, node_outputs) for item in data]
        else:
            # Return non-container types as-is
            return data

    def _resolve_string_variables(self, text: str, node_outputs: Dict[str, Any]) -> Any:
        """
        Resolve variables in string context with type preservation.

        Handles two cases:
        1. Pure variable: "${node.field}" -> return actual object
        2. String interpolation: "Score: ${node.field}" -> return formatted string

        Args:
            text: String potentially containing variables
            node_outputs: Node output mappings

        Returns:
            Resolved value (preserving type for pure variables)
        """
        # Quick check: does string contain variables?
        if "${" not in text:
            return text

        # Check if entire string is a single variable (pure variable case)
        single_var_pattern = r"^\$\{([^}]+)\}$"
        single_match = re.match(single_var_pattern, text)

        if single_match:
            # Pure variable case - return actual resolved value with type preservation
            var_path = single_match.group(1)
            try:
                return self.resolve_variable_path(var_path, node_outputs)
            except VariableResolutionError:
                logger.warning(f"Failed to resolve variable ${{{var_path}}}, returning original string")
                return text
        else:
            # String interpolation case - replace all variables with string representations
            return self._resolve_string_interpolation(text, node_outputs)

    def _resolve_string_interpolation(self, text: str, node_outputs: Dict[str, Any]) -> str:
        """
        Resolve variables within string templates.

        Example: "Score: ${tool-1.result.score} (${tool-1.meta.status})"
                 -> "Score: 85 (completed)"

        Args:
            text: String template with variables
            node_outputs: Node output mappings

        Returns:
            String with variables replaced by their string representations
        """
        pattern = r"\$\{([^}]+)\}"

        def replace_var(match):
            var_path = match.group(1)
            try:
                resolved_value = self.resolve_variable_path(var_path, node_outputs)
                # Convert to string for interpolation
                return str(resolved_value)
            except VariableResolutionError as e:
                logger.warning(f"Variable resolution failed for ${{{var_path}}}: {e}")
                # Keep original if resolution fails
                return match.group(0)

        return re.sub(pattern, replace_var, text)

    def resolve_variable_path(self, path: str, node_outputs: Dict[str, Any]) -> Any:
        """
        Resolve a single variable path to its value.

        Examples:
        - "tool-1" → node_outputs["tool-1"]["value"]
        - "tool-1.result" → node_outputs["tool-1"]["value"] (alias)
        - "tool-1.value" → node_outputs["tool-1"]["value"]
        - "tool-1.meta.status" → node_outputs["tool-1"]["meta"]["status"]
        - "tool-1.result.score" → node_outputs["tool-1"]["value"]["score"]

        Args:
            path: Variable path (e.g., "node.field.subfield")
            node_outputs: Node output mappings

        Returns:
            Resolved value with original type preserved

        Raises:
            NodeNotFoundError: If node doesn't exist
            FieldNotFoundError: If field path is invalid
            InvalidPathError: If path is malformed
        """
        try:
            node_id, field_path = self._parse_variable_path(path)

            # Check if node exists
            if node_id not in node_outputs:
                available_nodes = list(node_outputs.keys())
                raise NodeNotFoundError(f"Node '{node_id}' not found. Available nodes: {available_nodes}")

            node_output = node_outputs[node_id]

            # Detect output format and resolve accordingly
            if is_envelope_format(node_output):
                return self._resolve_envelope_format(node_output, field_path)
            else:
                return self._resolve_legacy_format(node_output, field_path)

        except Exception as e:
            if isinstance(e, VariableResolutionError):
                raise
            else:
                raise VariableResolutionError(f"Failed to resolve ${{{path}}}: {e}")

    def _parse_variable_path(self, path: str) -> Tuple[str, List[str]]:
        """
        Parse variable path into node_id and field path.

        Examples:
        - "tool-1" → ("tool-1", [])
        - "tool-1.result" → ("tool-1", ["result"])
        - "tool-1.meta.status" → ("tool-1", ["meta", "status"])

        Args:
            path: Variable path string

        Returns:
            Tuple of (node_id, field_path_list)

        Raises:
            InvalidPathError: If path is malformed
        """
        if not path or not isinstance(path, str):
            raise InvalidPathError("Variable path must be a non-empty string")

        parts = path.split(".")
        if not parts[0]:
            raise InvalidPathError("Variable path cannot start with '.'")

        node_id = parts[0]
        field_path = parts[1:] if len(parts) > 1 else []

        return node_id, field_path

    def _resolve_alias(self, field_path: List[str]) -> List[str]:
        """
        Resolve field path aliases.

        Examples:
        - ["result"] → ["value"]
        - ["result", "score"] → ["value", "score"]
        - ["meta", "status"] → ["meta", "status"] (no change)

        Args:
            field_path: List of field names

        Returns:
            Field path with aliases resolved
        """
        if not field_path:
            return field_path

        first_field = field_path[0]
        if first_field in self.aliases:
            resolved_path = [self.aliases[first_field]] + field_path[1:]
            return resolved_path

        return field_path

    def _resolve_envelope_format(self, node_output: Dict[str, Any], field_path: List[str]) -> Any:
        """
        Resolve field path in new envelope format.

        Envelope format: {"value": ..., "meta": {...}}

        Args:
            node_output: Node output in envelope format
            field_path: List of field names to traverse

        Returns:
            Resolved value
        """
        if not field_path:
            # ${node} → return the value
            return node_output["value"]

        # Resolve aliases
        resolved_path = self._resolve_alias(field_path)

        # Navigate to the appropriate section
        if resolved_path[0] == "value":
            # ${node.value} or ${node.value.field}
            current = node_output["value"]
            remaining_path = resolved_path[1:]
        elif resolved_path[0] == "meta":
            # ${node.meta.field}
            if len(resolved_path) < 2:
                raise FieldNotFoundError("Meta access requires field specification: ${node.meta.field}")
            current = node_output["meta"]
            remaining_path = resolved_path[1:]
        else:
            # Direct field access on value: ${node.field}
            current = node_output["value"]
            remaining_path = resolved_path

        # Traverse remaining path
        return self._traverse_path(current, remaining_path)

    def _resolve_legacy_format(self, node_output: Any, field_path: List[str]) -> Any:
        """
        Resolve field path in legacy format with compatibility mapping.

        Legacy formats vary by node type but generally have fields at top level.

        Args:
            node_output: Node output in legacy format
            field_path: List of field names to traverse

        Returns:
            Resolved value
        """
        if not field_path:
            # ${node} → return "result" field if available, otherwise entire output
            if isinstance(node_output, dict) and "result" in node_output:
                return node_output["result"]
            else:
                return node_output

        # Resolve aliases
        resolved_path = self._resolve_alias(field_path)
        first_field = resolved_path[0]

        if first_field in ["result", "value"]:
            # ${node.result} or ${node.value} → legacy result field
            if isinstance(node_output, dict) and "result" in node_output:
                result_value = node_output["result"]
                if len(resolved_path) > 1:
                    # ${node.result.field} → nested access in result
                    remaining_path = resolved_path[1:]
                    return self._traverse_path(result_value, remaining_path)
                else:
                    return result_value
            else:
                # No result field, return entire output
                return node_output

        elif first_field == "meta":
            # ${node.meta.field} → map to legacy top-level fields
            if len(resolved_path) < 2:
                raise FieldNotFoundError("Meta access requires field specification: ${node.meta.field}")

            meta_field = resolved_path[1]
            if isinstance(node_output, dict) and meta_field in node_output:
                meta_value = node_output[meta_field]
                if len(resolved_path) > 2:
                    # Further nesting
                    remaining_path = resolved_path[2:]
                    return self._traverse_path(meta_value, remaining_path)
                else:
                    return meta_value
            else:
                raise FieldNotFoundError(
                    f"Legacy format doesn't have meta field '{meta_field}'. "
                    f"Available fields: {list(node_output.keys()) if isinstance(node_output, dict) else 'none'}"
                )

        else:
            # Direct field access: ${node.field}
            if isinstance(node_output, dict) and first_field in node_output:
                field_value = node_output[first_field]
                if len(resolved_path) > 1:
                    # ${node.field.subfield}
                    remaining_path = resolved_path[1:]
                    return self._traverse_path(field_value, remaining_path)
                else:
                    return field_value
            else:
                raise FieldNotFoundError(
                    f"Field '{first_field}' not found in legacy output. "
                    f"Available fields: {list(node_output.keys()) if isinstance(node_output, dict) else 'none'}"
                )

    def _traverse_path(self, current: Any, path: List[str]) -> Any:
        """
        Traverse a field path through nested data structures.

        Args:
            current: Current object to traverse
            path: Remaining field path

        Returns:
            Value at the end of the path

        Raises:
            FieldNotFoundError: If any field in path doesn't exist
        """
        for field in path:
            if isinstance(current, dict) and field in current:
                current = current[field]
            elif isinstance(current, list):
                # Handle list indexing if field is numeric
                try:
                    index = int(field)
                    if 0 <= index < len(current):
                        current = current[index]
                    else:
                        raise FieldNotFoundError(f"List index {index} out of range (length: {len(current)})")
                except ValueError:
                    raise FieldNotFoundError(f"Invalid list index '{field}' (must be numeric)")
            else:
                # Field not found or current is not a container
                if isinstance(current, dict):
                    available_fields = list(current.keys())
                    raise FieldNotFoundError(f"Field '{field}' not found. Available fields: {available_fields}")
                else:
                    raise FieldNotFoundError(f"Cannot access field '{field}' on {type(current).__name__} object")

        return current

    def get_variable_names(self, data: Any) -> set:
        """
        Extract all variable names from a data structure.

        Args:
            data: Data structure to analyze

        Returns:
            Set of variable paths found (e.g., {"tool-1.result", "agent-1.status"})
        """
        variables = set()

        if isinstance(data, str):
            # Find all ${...} patterns
            pattern = r"\$\{([^}]+)\}"
            matches = re.findall(pattern, data)
            variables.update(matches)
        elif isinstance(data, dict):
            for value in data.values():
                variables.update(self.get_variable_names(value))
        elif isinstance(data, list):
            for item in data:
                variables.update(self.get_variable_names(item))

        return variables


# Global instance for convenience
variable_resolver = VariableResolver()


def resolve_variables(data: Any, node_outputs: Dict[str, Any]) -> Any:
    """
    Convenience function for variable resolution.

    Args:
        data: Data structure containing variable references
        node_outputs: Dictionary of node_id -> output mappings

    Returns:
        Data structure with variables resolved

    Raises:
        VariableResolutionError: For resolution failures
    """
    return variable_resolver.resolve_variables(data, node_outputs)


def resolve_variable_path(path: str, node_outputs: Dict[str, Any]) -> Any:
    """
    Convenience function for single variable path resolution.

    Args:
        path: Variable path (e.g., "node.field.subfield")
        node_outputs: Node output mappings

    Returns:
        Resolved value with original type preserved

    Raises:
        VariableResolutionError: For resolution failures
    """
    return variable_resolver.resolve_variable_path(path, node_outputs)
