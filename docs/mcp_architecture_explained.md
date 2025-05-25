# MCP Architecture Explained

## How MCP Servers Work

### The "Web Server" Concept

MCP servers are indeed web services that expose tools through standardized endpoints. There are two main transport types:

1. **HTTP/SSE (Server-Sent Events)**
   ```
   https://example.com/mcp/sse
   ├── /tools/list     (GET - discover available tools)
   ├── /tools/call     (POST - execute a tool)
   ├── /resources/list (GET - discover resources)
   └── /prompts/list   (GET - discover prompts)
   ```

2. **STDIO (Standard Input/Output)**
   - Local processes that communicate via JSON-RPC
   - Like the weather server example in MCP docs
   - Not accessible over network

### What This Means in Practice

When you connect to an MCP server like Linear:
```
https://mcp.linear.app/sse
```

1. Your agent discovers tools: `GET /tools/list`
   ```json
   {
     "tools": [
       {
         "name": "create_issue",
         "description": "Create a new Linear issue",
         "inputSchema": { ... }
       },
       {
         "name": "search_issues",
         "description": "Search Linear issues",
         "inputSchema": { ... }
       }
     ]
   }
   ```

2. Your agent calls a tool: `POST /tools/call`
   ```json
   {
     "name": "create_issue",
     "arguments": {
       "title": "Fix login bug",
       "description": "Users can't login with SSO"
     }
   }
   ```

## Customer Self-Service vs Our Integration

### Option 1: Full Customer Self-Service
Allow customers to register ANY MCP server they find:

**Pros:**
- Maximum flexibility
- No maintenance burden on us
- Customers can use niche/custom MCPs
- True "marketplace" approach

**Cons:**
- No quality control
- Security risks (malicious servers)
- No standardized error handling
- Confusing UX (which tools from which server?)

**Implementation:**
```python
# Customer provides:
{
  "mcp_servers": [
    {
      "url": "https://random-mcp-i-found.com/sse",
      "auth_token": "my_token"
    }
  ]
}
```

### Option 2: Curated + Self-Service (Recommended)
Provide tested presets AND allow custom servers:

**Implementation:**
```python
# backend/zerg/schemas/schemas.py
class AgentUpdate(BaseModel):
    # ... existing fields ...
    mcp_servers: Optional[List[MCPServerConfig]] = None

class MCPServerConfig(BaseModel):
    # For presets
    preset: Optional[str] = None  # "github", "linear", etc.
    
    # For custom servers
    custom_url: Optional[str] = None
    custom_name: Optional[str] = None
    
    # Common fields
    auth_token: Optional[str] = None
    allowed_tools: Optional[List[str]] = None
```

**Customer Experience:**
```yaml
# Easy mode - tested integrations
- preset: "github"
  auth_token: "ghp_xxx"

# Advanced mode - any MCP server
- custom_url: "https://my-company.com/mcp/sse"
  custom_name: "internal-tools"
  auth_token: "xxx"
```

## External API Stability Concerns

### The Reality of MCP Dependencies

Yes, MCP servers are external dependencies with risks:

1. **Availability**: Server could go down
2. **Latency**: Network calls add delay
3. **Rate Limits**: External services have limits
4. **Breaking Changes**: APIs can change

### Mitigation Strategies

1. **Hybrid Approach Benefits**
   - Critical tools: Keep as built-in (fast, reliable)
   - Nice-to-have tools: Use MCP (broad coverage)

2. **Resilience Patterns**
   ```python
   class MCPClient:
       async def call_tool(self, tool_name: str, arguments: Dict):
           try:
               # Timeout protection
               async with timeout(30):
                   result = await self._call_mcp_tool(...)
               
               # Cache successful schema lookups
               self._cache_tool_schema(tool_name, result)
               
               return result
               
           except TimeoutError:
               # Fallback to cached response if available
               if cached := self._get_cached_fallback(tool_name):
                   return cached
               raise
           
           except MCPServerError as e:
               # Log for monitoring
               logger.error(f"MCP server {self.url} failed: {e}")
               # Graceful degradation
               return {"error": "Service temporarily unavailable"}
   ```

3. **Quality Indicators**
   Show server health in UI:
   ```
   GitHub Tools     ✅ Connected (15ms)
   Linear Tools     ⚠️  Slow (2500ms)
   Custom MCP       ❌ Unreachable
   ```

## Recommended Architecture

### 1. Enable Both Approaches
```python
# Built-in tools (reliable, fast)
- get_current_time
- math_eval
- http_get

# Curated MCP presets (tested, documented)
- GitHub (99.9% uptime)
- Linear (99.5% uptime)
- Slack (99.99% uptime)

# Customer custom MCPs (their responsibility)
- https://customer-specific-mcp.com
```

### 2. Clear UX Separation
```
┌─────────────────────────────────────┐
│ 🛠️  Built-in Tools (Always Available) │
├─────────────────────────────────────┤
│ ✓ Date/Time Tools                   │
│ ✓ Math Calculator                   │
│ ✓ HTTP Requests                     │
└─────────────────────────────────────┘

┌─────────────────────────────────────┐
│ 🔌 Connected Services               │
├─────────────────────────────────────┤
│ ✓ GitHub (via MCP)    [Disconnect] │
│ ✓ Linear (via MCP)    [Disconnect] │
│ + Add Service...                    │
└─────────────────────────────────────┘

┌─────────────────────────────────────┐
│ 🔧 Custom MCP Servers               │
├─────────────────────────────────────┤
│ + Add Custom MCP Server URL...      │
└─────────────────────────────────────┘
```

### 3. Progressive Disclosure
- **Beginners**: Use presets only
- **Power Users**: Add custom MCP servers
- **Enterprises**: Self-host MCP servers

## Conclusion

Your instinct is correct - we should absolutely allow customers to register any MCP server they find. The key is to:

1. **Provide curated presets** for common services (GitHub, Linear, etc.)
2. **Allow custom MCP servers** for flexibility
3. **Keep critical tools built-in** for reliability
4. **Make the tradeoffs clear** in the UI

This gives customers the autonomy they want while maintaining quality where it matters.
