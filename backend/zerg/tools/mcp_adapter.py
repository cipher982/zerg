"""MCP (Model Context Protocol) adapter for the Zerg tool registry.

This module provides integration between MCP servers and our internal tool registry,
allowing agents to use both built-in tools and MCP-provided tools seamlessly.

This module started life as a **proof-of-concept** with a few hard-coded
presets.  It has now been promoted to a *production-ready* component that
supports **dynamic** MCP server registration while still exposing the same
convenience presets (now moved to `zerg.tools.mcp_presets`).

Key changes compared to the PoC version:

1.  ❌  No more hard-coded presets in the adapter itself.  Presets live in
    `mcp_presets.py` and can be modified without touching any logic.
2.  ✅  New `MCPManager` singleton caches one adapter per (url, auth_token)
    so we never double-register tools when multiple agents share the same MCP
    server.
3.  ✅  Public helpers `load_mcp_tools()` (async) and
    `load_mcp_tools_sync()` (sync) make it trivial to load tools from within
    both asynchronous and synchronous code paths.

See `docs/mcp_integration_requirements.md` for the end-to-end design.
"""

import asyncio
import logging
from dataclasses import dataclass
from typing import Any, Callable, Dict, List, Optional, Tuple

import httpx

from zerg.tools.registry import register_tool

# ---------------------------------------------------------------------------
# Logging helper
# ---------------------------------------------------------------------------

logger = logging.getLogger(__name__)

# We intentionally *do not* import the preset mapping here to avoid a circular
# dependency (`mcp_presets` imports :pyclass:`MCPServerConfig` from this very
# module).  The mapping is loaded **lazily** in :pyfunc:`MCPManager._get_presets`.


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

        # Extract parameter information from the tool spec (kept for future
        # schema-driven validation but currently unused).
        _ = tool_spec.get("inputSchema", {})

        async def _async_tool_wrapper(**kwargs):  # noqa: D401 – internal helper
            try:
                result = await self.client.call_tool(tool_name, kwargs)
                return str(result)
            except Exception as exc:  # noqa: BLE001
                logger.exception("Error calling MCP tool %s on %s", tool_name, self.config.name)
                return f"Error calling MCP tool {tool_name}: {exc}"

        # ------------------------------------------------------------------
        # Synchronous façade expected by LangChain.  It simply spins up a
        # private event-loop *inside* the worker thread and delegates to the
        # async implementation.
        # ------------------------------------------------------------------

        def _sync_tool_wrapper(**kwargs):  # noqa: D401 – wrapper
            return asyncio.run(_async_tool_wrapper(**kwargs))

        _sync_tool_wrapper.__name__ = f"{self.tool_prefix}{tool_name}"
        _sync_tool_wrapper.__doc__ = tool_spec.get("description", "")

        return _sync_tool_wrapper


# ---------------------------------------------------------------------------
#  Manager – ensures tools are only registered once per MCP server
# ---------------------------------------------------------------------------


class MCPManager:
    """Singleton that tracks *one* ``MCPToolAdapter`` per unique server."""

    _instance: Optional["MCPManager"] = None

    def __new__(cls):
        if cls._instance is None:
            cls._instance = super().__new__(cls)
            cls._instance._adapters: Dict[Tuple[str, str], MCPToolAdapter] = {}
        return cls._instance

    # ------------------------------------------------------------------
    # Helpers
    # ------------------------------------------------------------------

    async def _init_adapter(self, cfg: MCPServerConfig):
        key = (cfg.url, cfg.auth_token or "")
        if key in self._adapters:
            return  # Already initialised

        adapter = MCPToolAdapter(cfg)
        await adapter.register_tools()
        self._adapters[key] = adapter

    async def add_server_async(self, cfg_dict: Dict[str, Any]):
        """Add a server configuration (async version)."""

        # Expand presets (if any)
        if "preset" in cfg_dict:
            preset_name = cfg_dict["preset"]
            presets = self._get_presets()

            if preset_name not in presets:
                logger.warning("Unknown MCP preset: %s", preset_name)
                return

            base_cfg: MCPServerConfig = presets[preset_name]
            cfg = MCPServerConfig(
                name=base_cfg.name,
                url=base_cfg.url,
                auth_token=cfg_dict.get("auth_token", base_cfg.auth_token),
                allowed_tools=cfg_dict.get("allowed_tools", base_cfg.allowed_tools),
            )
        else:
            try:
                cfg = MCPServerConfig(**cfg_dict)
            except TypeError as exc:  # noqa: BLE001
                logger.error("Invalid MCP server config: %s", exc)
                return

        await self._init_adapter(cfg)

    def add_server(self, cfg_dict: Dict[str, Any]):
        """Synchronous wrapper around :pyfunc:`add_server_async`."""

        asyncio.run(self.add_server_async(cfg_dict))

    # ------------------------------------------------------------------
    # Internal helper – lazy preset loader to avoid circular import
    # ------------------------------------------------------------------

    @staticmethod
    def _get_presets() -> Dict[str, "MCPServerConfig"]:  # noqa: D401 – util
        """Return the preset mapping, importing the module on first use."""

        try:
            from zerg.tools.mcp_presets import PRESET_MCP_SERVERS  # type: ignore

            return PRESET_MCP_SERVERS
        except ModuleNotFoundError:  # pragma: no cover – missing optional file
            return {}


# ---------------------------------------------------------------------------
#  Public helpers – bulk-loaders
# ---------------------------------------------------------------------------


async def load_mcp_tools(mcp_configs: List[Dict[str, Any]]) -> None:  # noqa: D401
    """Async bulk loader.

    *mcp_configs* follows the schema documented in
    `docs/mcp_integration_requirements.md` (list of dicts where each dict is
    either a complete server config or contains a "preset" key).
    """

    if not mcp_configs:
        return

    manager = MCPManager()
    await asyncio.gather(*(manager.add_server_async(cfg) for cfg in mcp_configs))


def load_mcp_tools_sync(mcp_configs: List[Dict[str, Any]]) -> None:  # noqa: D401
    """Sync convenience wrapper for *blocking* contexts."""

    if not mcp_configs:
        return

    asyncio.run(load_mcp_tools(mcp_configs))


# Legacy alias – keep old import paths working
load_mcp_tools_sync.__doc__  # silence linters unused
