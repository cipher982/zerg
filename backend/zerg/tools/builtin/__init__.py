"""Built-in tools for Zerg agents.

This module contains the standard tools that come with the platform.
All tools in this module are aggregated into a single list for registry construction.
"""

from zerg.tools.builtin.datetime_tools import TOOLS as DATETIME_TOOLS
from zerg.tools.builtin.http_tools import TOOLS as HTTP_TOOLS
from zerg.tools.builtin.math_tools import TOOLS as MATH_TOOLS
from zerg.tools.builtin.uuid_tools import TOOLS as UUID_TOOLS

BUILTIN_TOOLS = DATETIME_TOOLS + HTTP_TOOLS + MATH_TOOLS + UUID_TOOLS

__all__ = [
    "BUILTIN_TOOLS",
]

# ---------------------------------------------------------------------------
# Automatically register built-in tools with the *mutable* singleton registry
# so legacy tests (and any runtime code relying on ToolRegistry) see them.
# ---------------------------------------------------------------------------

from zerg.tools.registry import ToolRegistry  # late import to avoid cycles

_registry = ToolRegistry()
for _tool in BUILTIN_TOOLS:
    if _tool.name not in _registry.list_tool_names():  # idempotent
        _registry.register(_tool)
