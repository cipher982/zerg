"""MCP (Model Context Protocol) adapter for the Zerg tool registry.

This module provides integration between MCP servers and our internal tool registry,
allowing agents to use both built-in tools and MCP-provided tools seamlessly.

WARNING: This is a PROOF OF CONCEPT implementation!
The hardcoded PRESET_MCP_SERVERS should be removed. Instead:
- Allow customers to add ANY MCP server URL dynamically
- Presets should be stored in a configuration file or database
- See docs/mcp_integration_requirements.md for the final design
"""

import asyncio
import logging
from dataclasses import dataclass
from typing import Any
from typing import Callable
from typing import Dict
from typing import List
from typing import Optional

import httpx

from zerg.tools.registry import register_tool

logger = logging.getLogger(__name__)


@dataclass
class MCPServerConfig:
    """Configuration for an MCP server connection."""

    name: str
    url: str
    auth_token: Optional[str] = None
    allowed_tools: Optional[List[str]] = None


class MCPClient:
    """Simple MCP client for HTTP/SSE transport."""

    def __init__(self, server_url: str, auth_token: Optional[str] = None):
        self.server_url = server_url
        self.auth_token = auth_token
        self.client = httpx.AsyncClient(timeout=30.0)

    async def list_tools(self) -> List[Dict[str, Any]]:
        """List available tools from the MCP server."""
        headers = {"Accept": "application/json"}
        if self.auth_token:
            headers["Authorization"] = f"Bearer {self.auth_token}"

        try:
            response = await self.client.get(f"{self.server_url}/tools/list", headers=headers)
            response.raise_for_status()
            return response.json().get("tools", [])
        except Exception as e:
            logger.error(f"Failed to list MCP tools: {e}")
            return []

    async def call_tool(self, tool_name: str, arguments: Dict[str, Any]) -> Any:
        """Call a tool on the MCP server."""
        headers = {"Content-Type": "application/json"}
        if self.auth_token:
            headers["Authorization"] = f"Bearer {self.auth_token}"

        try:
            response = await self.client.post(
                f"{self.server_url}/tools/call", headers=headers, json={"name": tool_name, "arguments": arguments}
            )
            response.raise_for_status()
            result = response.json()

            # Extract content from MCP response format
            if "content" in result and isinstance(result["content"], list):
                # Concatenate text content blocks
                text_parts = []
                for block in result["content"]:
                    if block.get("type") == "text":
                        text_parts.append(block.get("text", ""))
                return "\n".join(text_parts)

            return result

        except Exception as e:
            logger.error(f"Failed to call MCP tool {tool_name}: {e}")
            raise


class MCPToolAdapter:
    """Adapts MCP tools to work with our internal tool registry."""

    def __init__(self, server_config: MCPServerConfig):
        self.config = server_config
        self.client = MCPClient(server_config.url, server_config.auth_token)
        self.tool_prefix = f"mcp_{server_config.name}_"

    async def register_tools(self):
        """Discover and register all tools from the MCP server."""
        try:
            tools = await self.client.list_tools()

            for tool_spec in tools:
                tool_name = tool_spec.get("name", "")

                # Check if tool is in allowlist (if specified)
                if self.config.allowed_tools and tool_name not in self.config.allowed_tools:
                    continue

                # Create a wrapper function for this tool
                tool_func = self._create_tool_wrapper(tool_name, tool_spec)

                # Register with our internal registry
                register_tool(
                    name=f"{self.tool_prefix}{tool_name}",
                    description=tool_spec.get("description", f"MCP tool: {tool_name}"),
                    return_direct=False,
                )(tool_func)

                logger.info(f"Registered MCP tool: {self.tool_prefix}{tool_name}")

        except Exception as e:
            logger.error(f"Failed to register MCP tools from {self.config.name}: {e}")

    def _create_tool_wrapper(self, tool_name: str, tool_spec: Dict[str, Any]) -> Callable:
        """Create a wrapper function that calls the MCP tool."""

        # Extract parameter information from the tool spec
        input_schema = tool_spec.get("inputSchema", {})
        properties = input_schema.get("properties", {})

        async def tool_wrapper(**kwargs) -> str:
            """Wrapper that calls the MCP tool and returns the result."""
            try:
                result = await self.client.call_tool(tool_name, kwargs)
                return str(result)
            except Exception as e:
                return f"Error calling MCP tool {tool_name}: {str(e)}"

        # Add parameter annotations for better tool discovery
        tool_wrapper.__name__ = f"{self.tool_prefix}{tool_name}"
        tool_wrapper.__doc__ = tool_spec.get("description", "")

        # For simplicity, we're returning the async wrapper
        # In production, we'd want to handle sync/async properly
        return lambda **kwargs: asyncio.run(tool_wrapper(**kwargs))


# Preset MCP servers for quick wins
PRESET_MCP_SERVERS = {
    "github": MCPServerConfig(
        name="github",
        url="https://github.com/api/mcp/sse",
        allowed_tools=["search_issues", "create_issue", "get_repository"],
    ),
    "linear": MCPServerConfig(
        name="linear", url="https://mcp.linear.app/sse", allowed_tools=["create_issue", "update_issue", "search_issues"]
    ),
    "slack": MCPServerConfig(
        name="slack", url="https://slack.com/api/mcp/sse", allowed_tools=["send_message", "list_channels"]
    ),
}


async def load_mcp_tools(mcp_configs: List[Dict[str, Any]]) -> None:
    """Load tools from multiple MCP servers into the registry.

    Args:
        mcp_configs: List of MCP server configurations
    """
    for config_dict in mcp_configs:
        # Check if it's a preset
        if "preset" in config_dict:
            preset_name = config_dict["preset"]
            if preset_name in PRESET_MCP_SERVERS:
                server_config = PRESET_MCP_SERVERS[preset_name]
                # Override with any custom auth token
                if "auth_token" in config_dict:
                    server_config.auth_token = config_dict["auth_token"]
            else:
                logger.warning(f"Unknown MCP preset: {preset_name}")
                continue
        else:
            # Custom MCP server configuration
            server_config = MCPServerConfig(**config_dict)

        adapter = MCPToolAdapter(server_config)
        await adapter.register_tools()


# Example usage in agent configuration:
# agent.config = {
#     "mcp_servers": [
#         {"preset": "github", "auth_token": "ghp_xxxxx"},
#         {"preset": "linear", "auth_token": "lin_xxxxx"},
#         {
#             "name": "custom",
#             "url": "https://my-company.com/mcp/sse",
#             "auth_token": "custom_token",
#             "allowed_tools": ["custom_tool_1", "custom_tool_2"]
#         }
#     ]
# }
