"""Convenience MCP server presets.

Keeping the presets in a *dedicated* module allows product teams to update the
default list (or downstream forks to customise it) **without** touching the
core adapter logic.  All entries use the :pyclass:`zerg.tools.mcp_adapter.MCPServerConfig` dataclass.
"""

from __future__ import annotations

from typing import Dict


from zerg.tools.mcp_adapter import MCPServerConfig


# ---------------------------------------------------------------------------
# Public mapping – **modify** as needed
# ---------------------------------------------------------------------------

# Naming convention: the *dict key* doubles as the prefix inserted between the
# ``mcp_`` namespace and the tool name (see ``MCPToolAdapter.tool_prefix``).

PRESET_MCP_SERVERS: Dict[str, MCPServerConfig] = {
    "github": MCPServerConfig(
        name="github",
        url="https://api.github.com/mcp",  # hypothetical MCP endpoint
        allowed_tools=[
            "search_issues",
            "create_issue",
            "get_repository",
        ],
    ),
    "linear": MCPServerConfig(
        name="linear",
        url="https://mcp.linear.app",  # hypothetical MCP endpoint
        allowed_tools=[
            "create_issue",
            "update_issue",
            "search_issues",
        ],
    ),
    "slack": MCPServerConfig(
        name="slack",
        url="https://slack.com/api/mcp",  # hypothetical MCP endpoint
        allowed_tools=[
            "send_message",
            "list_channels",
        ],
    ),
}
