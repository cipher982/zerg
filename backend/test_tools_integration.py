#!/usr/bin/env python
"""Quick test to verify tool registry integration."""

import sys

# Test 1: Check tool registry
print("=== Testing Tool Registry ===")
try:
    from zerg.tools.registry import get_registry

    registry = get_registry()
    tools = registry.list_tool_names()
    print(f"✓ Registered tools: {tools}")
    assert len(tools) == 5, f"Expected 5 tools, got {len(tools)}"
    print("✓ All 5 built-in tools registered")
except Exception as e:
    print(f"✗ Tool registry test failed: {e}")
    sys.exit(1)

# Test 2: Check backward compatibility
print("\n=== Testing Backward Compatibility ===")
try:
    from zerg.agents_def.zerg_react_agent import get_current_time

    print(f"✓ get_current_time imported: {get_current_time}")
    if get_current_time:
        print(f"✓ Tool name: {get_current_time.name}")
        result = get_current_time.invoke({})
        print(f"✓ Tool works: {result}")
    else:
        print("✗ get_current_time is None")
        sys.exit(1)
except Exception as e:
    print(f"✗ Backward compatibility test failed: {e}")
    import traceback

    traceback.print_exc()
    sys.exit(1)

# Test 3: Check agent can use tools
print("\n=== Testing Agent Tool Usage ===")
try:
    from types import SimpleNamespace

    from zerg.agents_def.zerg_react_agent import get_runnable

    # Create a dummy agent
    agent = SimpleNamespace(
        id=1,
        model="gpt-3.5-turbo",
        allowed_tools=None,  # Allow all tools
    )

    # This should not raise an error
    runnable = get_runnable(agent)
    print("✓ Agent runnable created successfully")

    # Test with restricted tools
    agent_restricted = SimpleNamespace(id=2, model="gpt-3.5-turbo", allowed_tools=["get_current_time", "math_eval"])
    runnable_restricted = get_runnable(agent_restricted)
    print("✓ Agent with restricted tools created successfully")

except Exception as e:
    print(f"✗ Agent tool usage test failed: {e}")
    import traceback

    traceback.print_exc()
    sys.exit(1)

print("\n✅ All tests passed!")
