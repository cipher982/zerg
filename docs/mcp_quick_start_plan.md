# MCP Integration Quick Start Plan

## Strategic Decision: Hybrid Approach

After analyzing MCP and our current implementation, the answer is clear:
- **Don't discard our system** - It provides valuable custom tool capabilities
- **Do integrate MCP** - It gives instant access to dozens of integrations
- **Wrap MCP in our system** - Best of both worlds approach

## Week 1: Proof of Value (3 days)

### Day 1-2: Basic MCP Integration
Update the agent factory to support MCP tools:

```python
# backend/zerg/agents_def/zerg_react_agent.py
def get_runnable(agent_row):
    # ... existing code ...
    
    # Check if agent has MCP servers configured
    if hasattr(agent_row, 'config') and agent_row.config:
        mcp_servers = agent_row.config.get('mcp_servers', [])
        if mcp_servers:
            # Load MCP tools asynchronously
            import asyncio
            from zerg.tools.mcp_adapter import load_mcp_tools
            asyncio.run(load_mcp_tools(mcp_servers))
```

### Day 3: Test with High-Value Integrations
Configure a demo agent with:
```json
{
  "mcp_servers": [
    {
      "preset": "github",
      "auth_token": "ghp_demo_token"
    },
    {
      "preset": "linear", 
      "auth_token": "lin_demo_token"
    }
  ]
}
```

### Demo Script for Customers:
"Create a GitHub issue for the bug we discussed, then create a Linear task to track the fix"

## Week 2: Production-Ready Features

### OAuth Token Management
```python
# backend/zerg/models/models.py
class MCPToken(Base):
    __tablename__ = "mcp_tokens"
    
    id = Column(Integer, primary_key=True)
    user_id = Column(Integer, ForeignKey("users.id"))
    server_name = Column(String, nullable=False)
    access_token = Column(String, nullable=False)  # Encrypted
    refresh_token = Column(String, nullable=True)
    expires_at = Column(DateTime, nullable=True)
    created_at = Column(DateTime, server_default=func.now())
```

### API Endpoint for MCP Configuration
```python
# backend/zerg/routers/mcp.py
@router.post("/agents/{agent_id}/mcp-servers")
async def configure_mcp_servers(
    agent_id: int,
    servers: List[MCPServerConfig],
    db: Session = Depends(get_db),
    current_user: User = Depends(get_current_user)
):
    agent = crud.get_agent(db, agent_id)
    if not agent or agent.owner_id != current_user.id:
        raise HTTPException(status_code=404)
    
    # Update agent config
    config = agent.config or {}
    config["mcp_servers"] = [s.dict() for s in servers]
    agent.config = config
    db.commit()
    
    return {"status": "configured"}
```

## Week 3: UI Integration

### Frontend Components
```rust
// frontend/src/components/agent_config/mcp_servers.rs
pub fn render_mcp_server_config(agent: &Agent) -> Node<Msg> {
    div![
        h3!["MCP Integrations"],
        
        // Quick presets
        div![
            C!["mcp-presets"],
            button![
                "Connect GitHub",
                ev(Ev::Click, |_| Msg::ConnectMCPPreset("github"))
            ],
            button![
                "Connect Linear",
                ev(Ev::Click, |_| Msg::ConnectMCPPreset("linear"))
            ],
            button![
                "Connect Slack",
                ev(Ev::Click, |_| Msg::ConnectMCPPreset("slack"))
            ],
        ],
        
        // Custom server form
        form![
            input![attrs! {At::Placeholder => "MCP Server URL"}],
            input![attrs! {At::Placeholder => "Auth Token"}],
            button!["Add Custom MCP Server"]
        ]
    ]
}
```

## Customer Value Timeline

### Immediate (Week 1)
- **GitHub Integration**: Create issues, search code, manage PRs
- **Linear Integration**: Create/update tasks, search projects
- **Demonstration Value**: Show working integrations in demos

### Short-term (Week 2-3)
- **Slack**: Send messages, monitor channels
- **Asana**: Task management
- **Atlassian**: Jira/Confluence integration

### Medium-term (Month 2)
- **PayPal/Square**: Payment processing
- **Cloudflare**: Infrastructure management
- **Custom MCP servers**: Customer-specific integrations

## Marketing Message

"Your AI agents can now connect to 50+ enterprise tools out of the box:
- GitHub, GitLab, Bitbucket for code
- Linear, Asana, Jira for project management  
- Slack, Discord, Teams for communication
- Stripe, PayPal, Square for payments
- And many more..."

## Implementation Checklist

### Week 1
- [ ] Implement basic MCP adapter
- [ ] Test with 2-3 preset servers
- [ ] Create demo video showing GitHub + Linear integration
- [ ] Update agent config schema

### Week 2  
- [ ] Add OAuth token management
- [ ] Implement MCP server configuration API
- [ ] Add error handling and retry logic
- [ ] Security review (token encryption)

### Week 3
- [ ] Frontend UI for MCP configuration
- [ ] OAuth flow implementation
- [ ] Documentation and tutorials
- [ ] Customer webinar preparation

## Risk Mitigation

1. **Performance**: MCP calls add latency
   - Solution: Implement caching for tool lists
   - Solution: Parallel tool execution

2. **Security**: OAuth tokens need protection
   - Solution: Encrypt at rest
   - Solution: Implement token refresh

3. **Reliability**: External services may fail
   - Solution: Timeout and retry logic
   - Solution: Graceful degradation

## Success Metrics

- Week 1: 3 working MCP integrations
- Week 2: 10 beta customers testing
- Week 3: 50% of new demos include MCP tools
- Month 2: 100+ agents using MCP tools in production

## Conclusion

By wrapping MCP in our existing tool system, we can:
1. **Preserve our investment** in custom tools
2. **Instantly add value** with 50+ integrations
3. **Maintain flexibility** for custom development
4. **Position for growth** as MCP ecosystem expands

This hybrid approach gives customers the best of both worlds and positions us as a leader in AI agent tooling.
