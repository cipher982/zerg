"""Immutable tool registry for clean dependency injection."""

from dataclasses import dataclass
from types import MappingProxyType
from typing import Dict
from typing import FrozenSet
from typing import List
from typing import Optional

from langchain_core.tools import StructuredTool


@dataclass(frozen=True)
class ImmutableToolRegistry:
    """
    Thread-safe, immutable tool registry.

    Built once at startup, passed to agents via dependency injection.
    No global state, no mutations, no surprises.
    """

    _tools: MappingProxyType
    _names: FrozenSet[str]

    @classmethod
    def build(cls, tool_sources: List[List[StructuredTool]]) -> "ImmutableToolRegistry":
        """
        Build registry from multiple tool sources.

        Args:
            tool_sources: List of tool lists (builtin, MCP, custom, etc.)

        Raises:
            ValueError: If duplicate tool names are found
        """
        tools: Dict[str, StructuredTool] = {}
        for source in tool_sources:
            for tool in source:
                if tool.name in tools:
                    raise ValueError(
                        f"Duplicate tool name '{tool.name}' found. "
                        f"Existing: {tools[tool.name].description}, "
                        f"New: {tool.description}"
                    )
                tools[tool.name] = tool

        return cls(_tools=MappingProxyType(tools), _names=frozenset(tools.keys()))

    def get(self, name: str) -> Optional[StructuredTool]:
        return self._tools.get(name)

    def filter_by_allowlist(self, allowed: Optional[List[str]]) -> List[StructuredTool]:
        if not allowed:
            return list(self._tools.values())

        result = []
        for pattern in allowed:
            if pattern.endswith("*"):
                prefix = pattern[:-1]
                result.extend(t for n, t in self._tools.items() if n.startswith(prefix))
            elif pattern in self._tools:
                result.append(self._tools[pattern])
        return result

    def list_names(self) -> List[str]:
        return list(self._names)

    def all_tools(self) -> List[StructuredTool]:
        return list(self._tools.values())
