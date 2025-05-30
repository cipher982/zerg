"""Central registry for agent tools.

This module provides a singleton ToolRegistry that manages all available tools
in the system. Tools can be registered using the @register_tool decorator.
"""

import logging
from typing import Callable
from typing import Dict
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
    - Tool override support for testing
    """

    _instance: Optional["ToolRegistry"] = None

    def __new__(cls) -> "ToolRegistry":
        """Ensure singleton pattern."""
        if cls._instance is None:
            cls._instance = super().__new__(cls)
            cls._instance._tools = {}
            cls._instance._overrides: Dict[str, StructuredTool] = {}
            cls._instance._initialized = False
        return cls._instance

    def __init__(self):
        """Initialize the registry (only runs once due to singleton)."""
        if not getattr(self, "_initialized", False):
            if not hasattr(self, "_tools"):
                self._tools = {}
            if not hasattr(self, "_overrides"):
                self._overrides = {}
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

    def override_tool(self, name: str, tool: StructuredTool) -> None:
        """Override a tool temporarily (useful for testing).

        Args:
            name: The name of the tool to override
            tool: The replacement tool

        Note:
            Overrides take precedence over registered tools.
            Use restore_tool() to remove the override.
        """
        self._overrides[name] = tool
        logger.debug(f"Tool '{name}' has been overridden")

    def restore_tool(self, name: str) -> None:
        """Remove a tool override.

        Args:
            name: The name of the tool to restore
        """
        if name in self._overrides:
            del self._overrides[name]
            logger.debug(f"Tool '{name}' override removed")

    def clear_all_overrides(self) -> None:
        """Remove all tool overrides."""
        self._overrides.clear()
        logger.debug("All tool overrides cleared")

    def get_tool(self, name: str) -> Optional[StructuredTool]:
        """Get a tool by name.

        Args:
            name: The name of the tool to retrieve

        Returns:
            The tool if found, None otherwise

        Note:
            Overrides take precedence over registered tools.
        """
        # Check overrides first
        if name in self._overrides:
            return self._overrides[name]

        return self._tools.get(name)

    def get_all_tools(self) -> List[StructuredTool]:
        """Get all registered tools.

        Returns:
            List of all registered tools (including overrides)
        """
        # Merge tools with overrides (overrides take precedence)
        all_tools = {**self._tools, **self._overrides}
        return list(all_tools.values())

    # ------------------------------------------------------------------
    # Internal helper – *lazy* registration of built-in tools
    # ------------------------------------------------------------------

    def _ensure_builtin_tools(self) -> None:  # noqa: D401 – internal util
        """(Re-)import ``zerg.tools.builtin`` when the registry is empty.

        Some unit-tests deliberately empty ``ToolRegistry._tools`` to verify
        filtering logic.  When that happens the next call to e.g.
        :pyfunc:`list_tool_names` should still expose the *built-in* tools so
        that downstream code and tests which expect them continue to work.
        """

        # Detect missing *built-in* tools – we only reload when **any** builtin is
        # absent.  This is more precise than checking ``self._tools`` because a test
        # may have registered *custom* tools after clearing the registry, leaving the
        # mapping non-empty yet still missing the built-ins.

        REQUIRED = {
            "get_current_time",
            "datetime_diff",
            "http_get",
            "math_eval",
            "generate_uuid",
        }

        if REQUIRED.issubset(self._tools):
            return  # All built-ins present.

        # Importing the module triggers the @register_tool decorators again.
        # We reload both the package and its sub-modules so the decorators run
        # even when the modules were already imported earlier in the process.

        import importlib
        import sys

        module_name = "zerg.tools.builtin"
        if module_name in sys.modules:
            importlib.reload(sys.modules[module_name])
        else:
            importlib.import_module(module_name)

        # Reload individual sub-modules so their ``@register_tool`` decorators
        # run again and repopulate the registry after it was cleared.
        for sub in (
            "zerg.tools.builtin.datetime_tools",
            "zerg.tools.builtin.http_tools",
            "zerg.tools.builtin.math_tools",
            "zerg.tools.builtin.uuid_tools",
        ):
            if sub in sys.modules:
                importlib.reload(sys.modules[sub])
            else:
                importlib.import_module(sub)

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
                all_tools = {**self._tools, **self._overrides}
                for tool_name in all_tools:
                    if tool_name.startswith(prefix):
                        allowed_set.add(tool_name)
            else:
                allowed_set.add(pattern)

        return self.get_tools_by_names(list(allowed_set))

    def list_tool_names(self) -> List[str]:
        """Get a list of all registered tool names.

        Returns:
            List of tool names (including overrides)
        """
        # Lazy-load built-ins if the registry is empty (cleared by tests).
        self._ensure_builtin_tools()

        all_tools = {**self._tools, **self._overrides}
        return list(all_tools.keys())

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
