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
import threading
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from typing import Any
from typing import Callable
from typing import Dict
from typing import List
from typing import Optional
from typing import Tuple

import httpx
import jsonschema

from zerg.tools.mcp_config_schema import normalize_config
from zerg.tools.mcp_exceptions import MCPAuthenticationError
from zerg.tools.mcp_exceptions import MCPConfigurationError
from zerg.tools.mcp_exceptions import MCPConnectionError
from zerg.tools.mcp_exceptions import MCPToolExecutionError
from zerg.tools.mcp_exceptions import MCPValidationError
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
    timeout: float = 30.0
    max_retries: int = 3


class MCPClient:
    """Enhanced MCP client with connection pooling and retry logic."""

    def __init__(self, config: MCPServerConfig):
        self.config = config
        self.client = httpx.AsyncClient(
            timeout=config.timeout,
            limits=httpx.Limits(max_keepalive_connections=5, max_connections=10),
            http2=True,  # Enable HTTP/2 for better multiplexing
        )
        self._health_check_passed = False

    async def __aenter__(self):
        """Context manager entry."""
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit - ensure client is closed."""
        await self.client.aclose()

    async def health_check(self) -> bool:
        """Check if the MCP server is reachable and responsive."""
        if self._health_check_passed:
            return True

        headers = self._get_headers()
        try:
            response = await self.client.get(f"{self.config.url}/health", headers=headers, timeout=5.0)
            response.raise_for_status()
            self._health_check_passed = True
            logger.info(f"Health check passed for MCP server '{self.config.name}'")
            return True
        except httpx.HTTPStatusError as e:
            if e.response.status_code == 401:
                raise MCPAuthenticationError(self.config.name, "Invalid authentication token")
            logger.warning(f"Health check failed for MCP server '{self.config.name}': HTTP {e.response.status_code}")
            return False
        except Exception as e:
            logger.warning(f"Health check failed for MCP server '{self.config.name}': {e}")
            return False

    def _get_headers(self) -> Dict[str, str]:
        """Get common headers including auth."""
        headers = {"Accept": "application/json", "Content-Type": "application/json"}
        if self.config.auth_token:
            headers["Authorization"] = f"Bearer {self.config.auth_token}"
        return headers

    async def _request_with_retry(self, method: str, path: str, **kwargs) -> httpx.Response:
        """Make HTTP request with retry logic."""
        url = f"{self.config.url}{path}"
        headers = kwargs.pop("headers", {})
        headers.update(self._get_headers())

        last_exception = None
        for attempt in range(self.config.max_retries):
            try:
                if method == "GET":
                    response = await self.client.get(url, headers=headers, **kwargs)
                elif method == "POST":
                    response = await self.client.post(url, headers=headers, **kwargs)
                else:
                    raise ValueError(f"Unsupported HTTP method: {method}")

                response.raise_for_status()
                return response

            except httpx.HTTPStatusError as e:
                if e.response.status_code == 401:
                    raise MCPAuthenticationError(self.config.name, "Invalid authentication token")
                elif e.response.status_code < 500:
                    # Don't retry client errors
                    raise
                last_exception = e

            except (httpx.ConnectError, httpx.TimeoutException) as e:
                last_exception = e

            if attempt < self.config.max_retries - 1:
                wait_time = 2**attempt  # Exponential backoff
                logger.debug(
                    f"Retrying request to {url} in {wait_time}s (attempt {attempt + 1}/{self.config.max_retries})"
                )
                await asyncio.sleep(wait_time)

        raise MCPConnectionError(self.config.name, self.config.url, last_exception)

    async def list_tools(self) -> List[Dict[str, Any]]:
        """List available tools from the MCP server."""
        try:
            response = await self._request_with_retry("GET", "/tools/list")
            data = response.json()
            return data.get("tools", [])
        except MCPConnectionError:
            raise
        except Exception as e:
            logger.error(f"Failed to list MCP tools from '{self.config.name}': {e}")
            raise MCPConnectionError(self.config.name, self.config.url, e)

    async def call_tool(self, tool_name: str, arguments: Dict[str, Any]) -> Any:
        """Call a tool on the MCP server."""
        try:
            response = await self._request_with_retry(
                "POST", "/tools/call", json={"name": tool_name, "arguments": arguments}
            )
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

        except MCPConnectionError:
            raise
        except Exception as e:
            raise MCPToolExecutionError(tool_name, self.config.name, e)


class MCPToolAdapter:
    """Adapts MCP tools to work with our internal tool registry."""

    def __init__(self, server_config: MCPServerConfig):
        self.config = server_config
        self.client = MCPClient(server_config)
        self.tool_prefix = f"mcp_{server_config.name}_"
        self._tool_schemas: Dict[str, Dict[str, Any]] = {}

    async def register_tools(self):
        """Discover and register all tools from the MCP server."""
        # Perform health check first
        async with self.client as client:
            if not await client.health_check():
                logger.warning(f"Skipping MCP server '{self.config.name}' - health check failed")
                return

            try:
                tools = await client.list_tools()

                for tool_spec in tools:
                    tool_name = tool_spec.get("name", "")

                    # Check if tool is in allowlist (if specified)
                    if self.config.allowed_tools and tool_name not in self.config.allowed_tools:
                        continue

                    # Store the schema for validation
                    if "inputSchema" in tool_spec:
                        self._tool_schemas[tool_name] = tool_spec["inputSchema"]

                    # Create a wrapper function for this tool
                    tool_func = self._create_tool_wrapper(tool_name, tool_spec)

                    # Register with our internal registry
                    register_tool(
                        name=f"{self.tool_prefix}{tool_name}",
                        description=tool_spec.get("description", f"MCP tool: {tool_name}"),
                        return_direct=False,
                    )(tool_func)

                    logger.info(f"Registered MCP tool: {self.tool_prefix}{tool_name}")

            except MCPConnectionError as e:
                logger.error(f"Failed to register MCP tools: {e}")
                raise

    def _validate_inputs(self, tool_name: str, arguments: Dict[str, Any]) -> None:
        """Validate tool inputs against the schema."""
        schema = self._tool_schemas.get(tool_name)
        if not schema:
            return  # No schema to validate against

        try:
            jsonschema.validate(instance=arguments, schema=schema)
        except jsonschema.ValidationError as e:
            errors = {
                "message": str(e.message),
                "path": list(e.path),
                "schema_path": list(e.schema_path),
            }
            raise MCPValidationError(tool_name, errors)

    def _create_tool_wrapper(self, tool_name: str, tool_spec: Dict[str, Any]) -> Callable:
        """Create a wrapper function that calls the MCP tool."""

        async def _async_tool_wrapper(**kwargs):  # noqa: D401 – internal helper
            # Validate inputs
            try:
                self._validate_inputs(tool_name, kwargs)
            except MCPValidationError as e:
                logger.error(f"Input validation failed for tool '{tool_name}': {e}")
                return f"Error: {e}"

            # Execute the tool
            try:
                async with MCPClient(self.config) as client:
                    result = await client.call_tool(tool_name, kwargs)
                    return str(result)
            except MCPToolExecutionError as e:
                logger.error(f"Tool execution failed: {e}")
                return f"Error: {e}"
            except Exception as e:  # noqa: BLE001
                logger.exception("Unexpected error calling MCP tool %s on %s", tool_name, self.config.name)
                return f"Error calling MCP tool {tool_name}: {e}"

        # Synchronous wrapper that uses the shared event loop
        def _sync_tool_wrapper(**kwargs):  # noqa: D401 – wrapper
            return MCPManager().run_in_loop(_async_tool_wrapper(**kwargs))

        _sync_tool_wrapper.__name__ = f"{self.tool_prefix}{tool_name}"
        _sync_tool_wrapper.__doc__ = tool_spec.get("description", "")

        # Add parameter information for better introspection
        if "inputSchema" in tool_spec:
            _sync_tool_wrapper.__annotations__ = self._extract_annotations(tool_spec["inputSchema"])

        return _sync_tool_wrapper

    def _extract_annotations(self, schema: Dict[str, Any]) -> Dict[str, type]:
        """Extract type annotations from JSON schema."""
        annotations = {}
        properties = schema.get("properties", {})

        type_mapping = {
            "string": str,
            "number": float,
            "integer": int,
            "boolean": bool,
            "array": list,
            "object": dict,
        }

        for prop_name, prop_schema in properties.items():
            json_type = prop_schema.get("type", "string")
            annotations[prop_name] = type_mapping.get(json_type, Any)

        return annotations


# ---------------------------------------------------------------------------
#  Manager – ensures tools are only registered once per MCP server
# ---------------------------------------------------------------------------


class MCPManager:
    """Singleton that tracks *one* ``MCPToolAdapter`` per unique server."""

    _instance: Optional["MCPManager"] = None
    _lock = threading.Lock()

    def __new__(cls):
        with cls._lock:
            if cls._instance is None:
                cls._instance = super().__new__(cls)
                cls._instance._adapters: Dict[Tuple[str, str], MCPToolAdapter] = {}
                cls._instance._loop: Optional[asyncio.AbstractEventLoop] = None
                cls._instance._thread: Optional[threading.Thread] = None
                cls._instance._executor = ThreadPoolExecutor(max_workers=1)
        return cls._instance

    def _ensure_event_loop(self):
        """Ensure we have a running event loop for MCP operations."""
        if self._loop is None or not self._loop.is_running():
            self._loop = asyncio.new_event_loop()

            def run_loop():
                asyncio.set_event_loop(self._loop)
                self._loop.run_forever()

            self._thread = threading.Thread(target=run_loop, daemon=True)
            self._thread.start()

    def run_in_loop(self, coro):
        """Run a coroutine in the MCP event loop."""
        self._ensure_event_loop()
        future = asyncio.run_coroutine_threadsafe(coro, self._loop)
        return future.result()

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
        # Normalize and validate configuration
        try:
            normalized_config = normalize_config(cfg_dict)
        except ValueError as e:
            raise MCPConfigurationError(str(e))

        # Handle preset configuration
        if normalized_config["type"] == "preset":
            preset_name = normalized_config["preset"]
            presets = self._get_presets()

            if preset_name not in presets:
                raise MCPConfigurationError(f"Unknown preset: {preset_name}")

            base_cfg: MCPServerConfig = presets[preset_name]
            cfg = MCPServerConfig(
                name=base_cfg.name,
                url=base_cfg.url,
                auth_token=normalized_config.get("auth_token", base_cfg.auth_token),
                allowed_tools=normalized_config.get("allowed_tools", base_cfg.allowed_tools),
                timeout=normalized_config.get("timeout", base_cfg.timeout),
                max_retries=normalized_config.get("max_retries", base_cfg.max_retries),
            )
        else:  # type == "custom"
            try:
                cfg = MCPServerConfig(
                    name=normalized_config["name"],
                    url=normalized_config["url"],
                    auth_token=normalized_config.get("auth_token"),
                    allowed_tools=normalized_config.get("allowed_tools"),
                    timeout=normalized_config.get("timeout", 30.0),
                    max_retries=normalized_config.get("max_retries", 3),
                )
            except (KeyError, TypeError) as exc:
                raise MCPConfigurationError(f"Invalid configuration: {exc}")

        await self._init_adapter(cfg)

    def add_server(self, cfg_dict: Dict[str, Any]):
        """Synchronous wrapper around :pyfunc:`add_server_async`."""
        self.run_in_loop(self.add_server_async(cfg_dict))

    # ------------------------------------------------------------------
    # Internal helper – lazy preset loader to avoid circular import
    # ------------------------------------------------------------------

    @staticmethod
    def _get_presets() -> Dict[str, "MCPServerConfig"]:  # noqa: D401 – util
        """Return the preset mapping, importing the module on first use."""
        try:
            from zerg.tools.mcp_presets import PRESET_MCP_SERVERS  # type: ignore

            return PRESET_MCP_SERVERS
        except ImportError:  # pragma: no cover – missing optional file
            return {}

    def __del__(self):
        """Cleanup when the manager is destroyed."""
        if hasattr(self, "_loop") and self._loop and self._loop.is_running():
            self._loop.call_soon_threadsafe(self._loop.stop)
        if hasattr(self, "_executor"):
            self._executor.shutdown(wait=False)


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
    await asyncio.gather(
        *(manager.add_server_async(cfg) for cfg in mcp_configs),
        return_exceptions=True,  # Don't fail all if one fails
    )


def load_mcp_tools_sync(mcp_configs: List[Dict[str, Any]]) -> None:  # noqa: D401
    """Sync convenience wrapper for *blocking* contexts."""
    if not mcp_configs:
        return

    manager = MCPManager()
    manager.run_in_loop(load_mcp_tools(mcp_configs))
