"""Tools package for Zerg agent platform.

This package provides a centralized registry for agent tools and includes
built-in tools that agents can use during their ReAct loops.
"""

from zerg.tools.registry import ToolRegistry
from zerg.tools.registry import register_tool

__all__ = ["ToolRegistry", "register_tool"]
