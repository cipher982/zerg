#!/usr/bin/env python3
"""Validate tool contracts without modifying files (for pre-commit)."""

import sys
import yaml
import tempfile
import os
from pathlib import Path

# Add backend to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent / "backend"))

def validate_contracts_readonly():
    """Validate contracts and check generated files are up to date."""
    
    # Load schema
    schema_file = Path(__file__).parent.parent / "asyncapi" / "tools.yml"
    with open(schema_file) as f:
        schema = yaml.safe_load(f)
    
    expected_tools = set(schema['components']['schemas']['ToolName']['enum'])
    
    # Import and get actual tools
    from zerg.tools.builtin import BUILTIN_TOOLS
    actual_tools = set(tool.name for tool in BUILTIN_TOOLS)
    
    # Check all expected tools exist
    missing_tools = expected_tools - actual_tools
    if missing_tools:
        print(f"❌ Missing tools in registry: {missing_tools}")
        return False
    
    # Generate expected files to temp location
    sys.path.insert(0, str(Path(__file__).parent))
    from generate_tool_types import generate_rust_enums, generate_python_types
    
    # Generate what the files should contain
    expected_rust = generate_rust_enums(schema)
    expected_python = generate_python_types(schema)
    
    # Read actual files
    rust_file = Path("frontend/src/generated/tool_definitions.rs")
    python_file = Path("backend/zerg/tools/generated/tool_definitions.py")
    
    if not rust_file.exists():
        print(f"❌ Generated Rust file missing: {rust_file}")
        return False
        
    if not python_file.exists():
        print(f"❌ Generated Python file missing: {python_file}")
        return False
    
    with open(rust_file) as f:
        actual_rust = f.read()
        
    with open(python_file) as f:
        actual_python = f.read()
    
    # Compare content (normalize whitespace)
    if actual_rust.strip() != expected_rust.strip():
        print("❌ Generated Rust types are out of sync")
        print("   Run: make tool-code-gen")
        return False
        
    if actual_python.strip() != expected_python.strip():
        print("❌ Generated Python types are out of sync") 
        print("   Run: make tool-code-gen")
        return False
    
    print("✅ Tool contract validation passed")
    print(f"  - {len(expected_tools)} tools validated")
    print("  - Generated types are up to date")
    return True

def main():
    """Main entry point."""
    try:
        if validate_contracts_readonly():
            sys.exit(0)
        else:
            sys.exit(1)
    except Exception as e:
        print(f"❌ Validation failed with error: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()