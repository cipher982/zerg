"""Central registry for agent tools.

This module provides a singleton ToolRegistry that manages all available tools
in the system. Tools can be registered using the @register_tool decorator.
"""

import logging
from typing import Callable
from typing import List
from typing import Optional
from typing import Set

from langchain_core.tools import StructuredTool

logger = logging.getLogger(__name__)


class ToolRegistry:
    """Singleton registry for managing agent tools.

    The registry maintains a collection of tools that can be discovered
    and used by agents. It supports:
    - Tool registration via decorator
    - Tool discovery by name or pattern
    - Filtering tools based on allow-lists
    """

    _instance: Optional["ToolRegistry"] = None

    def __new__(cls) -> "ToolRegistry":
        """Ensure singleton pattern."""
        if cls._instance is None:
            cls._instance = super().__new__(cls)
            cls._instance._tools = {}
            cls._instance._initialized = False
        return cls._instance

    def __init__(self):
        """Initialize the registry (only runs once due to singleton)."""
        if not getattr(self, "_initialized", False):
            if not hasattr(self, "_tools"):
                self._tools = {}
            self._initialized = True
            logger.info("ToolRegistry initialized")

    def register(self, tool: StructuredTool) -> None:
        """Register a tool in the registry.

        Args:
            tool: The StructuredTool instance to register

        Raises:
            ValueError: If a tool with the same name already exists
        """
        if tool.name in self._tools:
            # Log warning but don't raise - this allows re-importing modules
            logger.warning(f"Tool '{tool.name}' is already registered, skipping")
            return

        self._tools[tool.name] = tool
        logger.info(f"Registered tool: {tool.name}")

    def get_tool(self, name: str) -> Optional[StructuredTool]:
        """Get a tool by name.

        Args:
            name: The name of the tool to retrieve

        Returns:
            The tool if found, None otherwise
        """
        return self._tools.get(name)

    def get_all_tools(self) -> List[StructuredTool]:
        """Get all registered tools.

        Returns:
            List of all registered tools
        """
        return list(self._tools.values())

    def get_tools_by_names(self, names: List[str]) -> List[StructuredTool]:
        """Get multiple tools by their names.

        Args:
            names: List of tool names to retrieve

        Returns:
            List of found tools (skips any not found)
        """
        tools = []
        for name in names:
            tool = self.get_tool(name)
            if tool:
                tools.append(tool)
            else:
                logger.warning(f"Tool '{name}' not found in registry")
        return tools

    def filter_tools_by_allowlist(self, allowed_tools: Optional[List[str]] = None) -> List[StructuredTool]:
        """Filter tools based on an allow-list.

        Args:
            allowed_tools: List of allowed tool names. If None or empty, all tools are allowed.

        Returns:
            List of allowed tools
        """
        if not allowed_tools:
            # Empty list means all tools are allowed
            return self.get_all_tools()

        # Support wildcards (e.g., "http_*" for all HTTP tools)
        allowed_set: Set[str] = set()
        for pattern in allowed_tools:
            if pattern.endswith("*"):
                prefix = pattern[:-1]
                # Add all tools that start with the prefix
                for tool_name in self._tools:
                    if tool_name.startswith(prefix):
                        allowed_set.add(tool_name)
            else:
                allowed_set.add(pattern)

        return self.get_tools_by_names(list(allowed_set))

    def list_tool_names(self) -> List[str]:
        """Get a list of all registered tool names.

        Returns:
            List of tool names
        """
        return list(self._tools.keys())

    def clear(self) -> None:
        """Clear all registered tools (mainly for testing)."""
        self._tools.clear()
        logger.info("ToolRegistry cleared")


# Global registry instance
_registry = ToolRegistry()


def register_tool(
    name: Optional[str] = None,
    description: Optional[str] = None,
    return_direct: bool = False,
) -> Callable:
    """Decorator to register a function as a tool.

    This decorator converts a regular function into a StructuredTool and
    registers it with the global ToolRegistry.

    Args:
        name: Optional custom name for the tool (defaults to function name)
        description: Optional description (defaults to function docstring)
        return_direct: Whether to return tool output directly to user

    Returns:
        Decorator function

    Example:
        @register_tool(name="get_time", description="Get current time")
        def get_current_time() -> str:
            return datetime.now().isoformat()
    """

    def decorator(func: Callable) -> Callable:
        # Create StructuredTool from the function
        tool_name = name or func.__name__
        tool_description = description or func.__doc__ or f"Tool: {tool_name}"

        # Create the tool using StructuredTool.from_function
        tool = StructuredTool.from_function(
            func=func,
            name=tool_name,
            description=tool_description,
            return_direct=return_direct,
        )

        # Register with the global registry
        _registry.register(tool)

        # Return the original function (not the tool) so it can still be called directly
        return func

    return decorator


def get_registry() -> ToolRegistry:
    """Get the global ToolRegistry instance.

    Returns:
        The singleton ToolRegistry instance
    """
    return _registry
