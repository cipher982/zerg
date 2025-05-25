# MCP Integration Requirements - Single Source of Truth

*Status: Final - Ready for development team*

---

## Executive Summary

We will integrate MCP (Model Context Protocol) into our platform using a **hybrid approach** that preserves our built-in tool system while enabling customers to connect to ANY MCP server they choose. This gives customers immediate access to 50+ enterprise integrations while maintaining our architectural advantages.

---

## Core Principles

1. **Customer Freedom**: Customers can add ANY MCP server URL they find or build
2. **Convenience**: Provide one-click presets for popular services
3. **Reliability**: Keep critical tools as built-in for performance and availability
4. **Transparency**: Make it clear which tools are external dependencies

---

## Technical Architecture

### 1. Unified Tool Registry

All tools (built-in and MCP) appear in the same registry:

```python
# Tools are registered with prefixes to indicate source
- get_current_time           # Built-in tool
- math_eval                  # Built-in tool
- mcp_github_create_issue    # From GitHub MCP server
- mcp_linear_search_tasks    # From Linear MCP server
- mcp_custom_internal_tool   # From customer's MCP server
```

### 2. MCP Client Adapter

```python
class MCPManager:
    """Manages connections to multiple MCP servers."""
    
    def add_server(self, url: str, name: str, auth_token: str = None) -> None:
        """Add any MCP server - customer's primary use case."""
        client = MCPClient(url, auth_token)
        tools = client.discover_tools()
        for tool in tools:
            self.registry.register_mcp_tool(f"mcp_{name}_{tool.name}", tool)
    
    def add_preset(self, preset_name: str, auth_token: str) -> None:
        """Convenience method for popular services."""
        config = PRESET_CONFIGS[preset_name]
        self.add_server(config.url, preset_name, auth_token)
```

### 3. Agent Configuration Schema

```python
# backend/zerg/models/models.py
class Agent(Base):
    # ... existing fields ...
    
    # Option 1: Store MCP config in existing JSON config field
    config = Column(JSON, nullable=True)
    # config = {
    #     "mcp_servers": [
    #         {"url": "https://example.com/mcp", "name": "custom", "auth_token": "xxx"},
    #         {"preset": "github", "auth_token": "ghp_xxx"}
    #     ]
    # }
```

### 4. Error Handling & Resilience

```python
async def execute_mcp_tool(server_url: str, tool_name: str, args: dict):
    try:
        async with timeout(30):  # 30 second timeout
            return await mcp_client.call_tool(tool_name, args)
    except TimeoutError:
        logger.error(f"MCP tool {tool_name} timed out")
        return {"error": "Tool execution timed out", "status": "timeout"}
    except MCPServerError as e:
        logger.error(f"MCP server {server_url} error: {e}")
        return {"error": "MCP server unavailable", "status": "server_error"}
```

---

## UI/UX Requirements

### Agent Configuration Interface

```
┌─────────────────────────────────────────────────┐
│ 🛠️  Tool Configuration                          │
├─────────────────────────────────────────────────┤
│ Built-in Tools ✓                                │
│ • Date/Time (get_current_time)                  │
│ • Math (math_eval)                              │
│ • HTTP Requests (http_get)                      │
│                                                 │
│ ─────────────────────────────────────           │
│                                                 │
│ 🚀 Quick Connect (Popular Services)             │
│                                                 │
│ [🔗 Connect GitHub]  [📋 Connect Linear]        │
│ [💬 Connect Slack]   [📝 Connect Notion]        │
│                                                 │
│ ─────────────────────────────────────           │
│                                                 │
│ ➕ Add Custom MCP Server                        │
│                                                 │
│ Server URL: [________________________]         │
│ Name: [_____________]                           │
│ Auth Token: [_____________________] 🔒          │
│                                                 │
│ [Test Connection]  [Add Server]                 │
│                                                 │
│ ─────────────────────────────────────           │
│                                                 │
│ 📊 Connected MCP Servers                        │
│                                                 │
│ • github (3 tools)          ✅ Online  [Remove] │
│ • internal-crm (5 tools)    ✅ Online  [Remove] │
│ • weather-api (2 tools)     ⚠️ Slow    [Remove] │
└─────────────────────────────────────────────────┘
```

### Tool Selection Interface

When selecting allowed tools for an agent:

```
Available Tools
├── 🛠️ Built-in (Always Available)
│   ├── ☑️ get_current_time
│   ├── ☑️ math_eval
│   └── ☑️ http_get
└── 🌐 From MCP Servers
    ├── github
    │   ├── ☐ create_issue
    │   ├── ☐ search_repositories
    │   └── ☐ get_pull_request
    └── internal-crm
        ├── ☐ lookup_customer
        └── ☐ create_ticket
```

---

## Implementation Plan

### Week 1: Core MCP Support

1. **Update MCP adapter** (`backend/zerg/tools/mcp_adapter.py`)
   - Remove hardcoded servers
   - Implement dynamic server addition
   - Add proper error handling

2. **Database updates**
   - Store MCP server configs in agent.config JSON field
   - Add API endpoint for MCP server management

3. **Integration with agent factory**
   ```python
   # In get_runnable()
   if agent.config and 'mcp_servers' in agent.config:
       await load_mcp_tools(agent.config['mcp_servers'])
   ```

### Week 2: Presets & OAuth

1. **Popular service presets**
   - GitHub, Linear, Slack, Notion, Asana
   - Pre-configured URLs and tool allowlists

2. **OAuth token management**
   - Secure token storage (encrypted)
   - Token refresh logic
   - OAuth flow helpers for presets

### Week 3: UI Implementation

1. **Agent configuration UI**
   - Two-tab approach (Quick Connect / Custom)
   - Connection testing
   - Health status indicators

2. **Tool selection UI**
   - Clear separation of built-in vs MCP tools
   - Show which MCP server provides each tool

---

## Security Considerations

1. **Token Storage**: Encrypt all auth tokens at rest
2. **URL Validation**: Only allow HTTPS MCP server URLs
3. **Timeout Protection**: 30-second timeout on all MCP calls
4. **Rate Limiting**: Implement per-server rate limits
5. **Audit Logging**: Log all MCP tool executions

---

## Success Metrics

- **Week 1**: Successfully connect to any MCP server URL
- **Week 2**: 5 preset integrations working with OAuth
- **Week 3**: UI complete, 10 beta customers testing
- **Month 2**: 100+ agents using MCP tools in production

---

## API Examples

### Add MCP Server
```http
POST /api/agents/{agent_id}/mcp-servers
{
  "url": "https://example.com/mcp/sse",
  "name": "example",
  "auth_token": "bearer_xxx"
}
```

### Add Preset
```http
POST /api/agents/{agent_id}/mcp-servers
{
  "preset": "github",
  "auth_token": "ghp_xxx"
}
```

### List Available Tools
```http
GET /api/agents/{agent_id}/available-tools
{
  "builtin": ["get_current_time", "math_eval", "http_get"],
  "mcp": {
    "github": ["create_issue", "search_repositories"],
    "custom": ["internal_tool_1", "internal_tool_2"]
  }
}
```

---

## Key Decisions

1. **Both presets AND custom URLs**: Not either/or
2. **Unified tool registry**: All tools appear together
3. **Customer owns risk**: They choose which MCP servers to trust
4. **Clear UI separation**: Built-in vs external tools
5. **Graceful degradation**: Handle MCP failures without breaking agents

---

## Next Steps

1. Review and approve this requirements document
2. Create implementation tickets for Week 1 tasks
3. Begin with MCP adapter updates
4. Set up testing environment with real MCP servers

---

*This document supersedes all previous MCP-related documentation and represents our final architectural decision.*
