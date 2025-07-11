#!/usr/bin/env python3
"""
WebSocket Protocol Code Generator

Generates Rust and Python types from the unified ws-protocol.yml schema.
This eliminates manual type definitions and transformation layers.
"""

import yaml
import json
import os
import sys
from pathlib import Path
from typing import Dict, Any, List, Optional
from datetime import datetime


class ProtocolCodeGenerator:
    def __init__(self, schema_path: str):
        """Initialize with the schema file path."""
        self.schema_path = Path(schema_path)
        self.schema = self._load_schema()
        self.output_dir = self.schema_path.parent
        
    def _load_schema(self) -> Dict[str, Any]:
        """Load and validate the YAML schema."""
        try:
            with open(self.schema_path, 'r') as f:
                return yaml.safe_load(f)
        except Exception as e:
            print(f"Error loading schema: {e}")
            sys.exit(1)
            
    def generate_all(self):
        """Generate all code artifacts."""
        print(f"Generating code from {self.schema_path}")
        
        # Generate backend Python types
        self.generate_python_types()
        
        # Generate frontend Rust types  
        self.generate_rust_types()
        
        # Generate contract validation JSON
        self.generate_contract_json()
        
        print("Code generation complete!")
        
    def generate_python_types(self):
        """Generate Pydantic models for the backend."""
        output_path = self.output_dir / "backend" / "zerg" / "generated" / "ws_messages.py"
        output_path.parent.mkdir(parents=True, exist_ok=True)
        
        code = self._generate_python_header()
        code += self._generate_python_envelope()
        code += self._generate_python_payloads()
        code += self._generate_python_message_types()
        code += self._generate_python_unions()
        code += self._generate_python_helpers()
        
        with open(output_path, 'w') as f:
            f.write(code)
            
        print(f"Generated Python types: {output_path}")
        
    def generate_rust_types(self):
        """Generate Rust structs and enums for the frontend."""
        output_path = self.output_dir / "frontend" / "src" / "generated" / "ws_messages.rs"
        output_path.parent.mkdir(parents=True, exist_ok=True)
        
        code = self._generate_rust_header()
        code += self._generate_rust_envelope()
        code += self._generate_rust_payloads()
        code += self._generate_rust_message_enum()
        code += self._generate_rust_helpers()
        
        with open(output_path, 'w') as f:
            f.write(code)
            
        print(f"Generated Rust types: {output_path}")
        
    def generate_contract_json(self):
        """Generate contract validation JSON."""
        output_path = self.output_dir / "contracts" / "ws-protocol-v1.json"
        output_path.parent.mkdir(parents=True, exist_ok=True)
        
        contract = {
            "version": self.schema["version"]["current"],
            "generated_at": datetime.utcnow().isoformat() + "Z",
            "envelope_schema": self.schema["definitions"]["Envelope"],
            "message_types": {},
            "topic_patterns": self.schema["topics"]
        }
        
        # Add message type schemas
        for msg_type, config in self.schema["messages"].items():
            contract["message_types"][msg_type] = {
                "payload_schema": config["payload"],
                "topic_pattern": config["topic_pattern"],
                "direction": config["direction"],
                "aliases": config.get("aliases", [])
            }
            
        with open(output_path, 'w') as f:
            json.dump(contract, f, indent=2)
            
        print(f"Generated contract JSON: {output_path}")

    def _generate_python_header(self) -> str:
        """Generate Python file header."""
        return f'''# AUTO-GENERATED FILE - DO NOT EDIT
# Generated from {self.schema_path.name} at {datetime.utcnow().isoformat()}Z
# 
# This file contains all WebSocket message types and schemas.
# To update, modify the schema file and run: python scripts/generate-ws-types.py

import time
from enum import Enum
from typing import Any, Dict, List, Optional, Union, Literal
from pydantic import BaseModel, Field


'''

    def _generate_python_envelope(self) -> str:
        """Generate Python Envelope class."""
        return '''class Envelope(BaseModel):
    """Unified envelope for all WebSocket messages."""
    
    v: int = Field(default=1, description="Protocol version")
    type: str = Field(description="Message type identifier") 
    topic: str = Field(description="Topic routing string")
    req_id: Optional[str] = Field(default=None, description="Request correlation ID")
    ts: int = Field(description="Timestamp in milliseconds since epoch")
    data: Dict[str, Any] = Field(description="Message payload")
    
    @classmethod
    def create(
        cls,
        message_type: str,
        topic: str, 
        data: Dict[str, Any],
        req_id: Optional[str] = None,
    ) -> "Envelope":
        """Create a new envelope with current timestamp."""
        return cls(
            type=message_type.lower(),
            topic=topic,
            data=data,
            req_id=req_id,
            ts=int(time.time() * 1000),
        )

'''

    def _generate_python_payloads(self) -> str:
        """Generate Python payload classes."""
        code = "# Message payload schemas\n\n"
        
        for name, schema in self.schema["definitions"].items():
            if name == "Envelope":
                continue
                
            code += self._python_schema_to_class(name, schema) + "\n\n"
            
        return code
        
    def _python_schema_to_class(self, name: str, schema: Dict[str, Any]) -> str:
        """Convert JSON schema to Python Pydantic class."""
        if schema.get("type") != "object":
            return f"# Skipping non-object schema: {name}"
            
        lines = [f"class {name}(BaseModel):"]
        lines.append(f'    """{schema.get("description", f"Payload for {name} messages")}"""')
        lines.append("")
        
        properties = schema.get("properties", {})
        required = schema.get("required", [])
        
        if not properties:
            lines.append("    pass")
            return "\n".join(lines)
            
        for prop_name, prop_schema in properties.items():
            python_type = self._json_type_to_python(prop_schema)
            is_required = prop_name in required
            
            if is_required:
                lines.append(f"    {prop_name}: {python_type}")
            else:
                lines.append(f"    {prop_name}: Optional[{python_type}] = None")
                
        return "\n".join(lines)
        
    def _json_type_to_python(self, schema: Dict[str, Any]) -> str:
        """Convert JSON schema type to Python type hint."""
        if "$ref" in schema:
            ref_name = schema["$ref"].split("/")[-1]
            return ref_name
            
        json_type = schema.get("type", "any")
        
        if json_type == "string":
            if "enum" in schema:
                enum_values = "', '".join(schema["enum"])
                return f"Literal['{enum_values}']"
            return "str"
        elif json_type == "integer":
            return "int"
        elif json_type == "number":
            return "float"
        elif json_type == "boolean":
            return "bool"
        elif json_type == "array":
            item_schema = schema.get("items", {"type": "any"})
            item_type = self._json_type_to_python(item_schema)
            return f"List[{item_type}]"
        elif json_type == "object":
            return "Dict[str, Any]"
        else:
            return "Any"
            
    def _generate_python_message_types(self) -> str:
        """Generate Python MessageType enum."""
        code = "class MessageType(str, Enum):\n"
        code += '    """Enumeration of all WebSocket message types."""\n\n'
        
        for msg_type in self.schema["messages"].keys():
            enum_name = msg_type.upper()
            code += f'    {enum_name} = "{msg_type}"\n'
            
        return code + "\n\n"
        
    def _generate_python_unions(self) -> str:
        """Generate Python union types for incoming/outgoing messages."""
        # TODO: Analyze direction field to separate incoming vs outgoing
        return '''# Union types for message handling (placeholder for future use)
# IncomingMessage = Union[...]
# OutgoingMessage = Union[...]

'''

    def _generate_python_helpers(self) -> str:
        """Generate Python helper functions."""
        return '''# Helper functions

def validate_envelope(data: Dict[str, Any]) -> Envelope:
    """Validate and parse envelope from raw data."""
    return Envelope.model_validate(data)
    
def create_message(
    message_type: str,
    topic: str,
    data: Dict[str, Any],
    req_id: Optional[str] = None
) -> Envelope:
    """Create a properly formatted message envelope."""
    return Envelope.create(message_type, topic, data, req_id)

'''

    def _generate_rust_header(self) -> str:
        """Generate Rust file header."""
        return f"""// AUTO-GENERATED FILE - DO NOT EDIT
// Generated from {self.schema_path.name} at {datetime.utcnow().isoformat()}Z
//
// This file contains all WebSocket message types and schemas.
// To update, modify the schema file and run: python scripts/generate-ws-types.py

use serde::{{Deserialize, Serialize}};
use serde_json::Value;

"""

    def _generate_rust_envelope(self) -> str:
        """Generate Rust Envelope struct."""
        return """#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Envelope {
    pub v: u8,
    #[serde(rename = "type")]
    pub message_type: String,
    pub topic: String,
    pub req_id: Option<String>,
    pub ts: u64,
    pub data: Value,
}

impl Envelope {
    pub fn new(
        message_type: String,
        topic: String,
        data: Value,
        req_id: Option<String>,
    ) -> Self {
        Self {
            v: 1,
            message_type,
            topic,
            req_id,
            ts: js_sys::Date::now() as u64,
            data,
        }
    }
}

"""

    def _generate_rust_payloads(self) -> str:
        """Generate Rust payload structs."""
        code = "// Message payload structs\n\n"
        
        for name, schema in self.schema["definitions"].items():
            if name == "Envelope":
                continue
                
            code += self._rust_schema_to_struct(name, schema) + "\n\n"
            
        return code
        
    def _rust_schema_to_struct(self, name: str, schema: Dict[str, Any]) -> str:
        """Convert JSON schema to Rust struct."""
        if schema.get("type") != "object":
            return f"// Skipping non-object schema: {name}"
            
        lines = ["#[derive(Debug, Deserialize, Serialize, Clone)]"]
        lines.append(f"pub struct {name} {{")
        
        properties = schema.get("properties", {})
        required = schema.get("required", [])
        
        if not properties:
            lines.append("    // Empty struct")
            lines.append("}")
            return "\n".join(lines)
            
        for prop_name, prop_schema in properties.items():
            rust_type = self._json_type_to_rust(prop_schema)
            is_required = prop_name in required
            
            if not is_required:
                rust_type = f"Option<{rust_type}>"
                
            lines.append(f"    pub {prop_name}: {rust_type},")
            
        lines.append("}")
        return "\n".join(lines)
        
    def _json_type_to_rust(self, schema: Dict[str, Any]) -> str:
        """Convert JSON schema type to Rust type."""
        if "$ref" in schema:
            ref_name = schema["$ref"].split("/")[-1]
            return ref_name
            
        json_type = schema.get("type", "any")
        
        if json_type == "string":
            return "String"
        elif json_type == "integer":
            return "u32"  # Adjust based on schema constraints
        elif json_type == "number":
            return "f64"
        elif json_type == "boolean":
            return "bool"
        elif json_type == "array":
            item_schema = schema.get("items", {"type": "any"})
            item_type = self._json_type_to_rust(item_schema)
            return f"Vec<{item_type}>"
        elif json_type == "object":
            return "Value"
        else:
            return "Value"
            
    def _generate_rust_message_enum(self) -> str:
        """Generate Rust message type enum."""
        code = "#[derive(Debug, Deserialize, Serialize, Clone)]\n"
        code += "#[serde(tag = \"type\")]\n"
        code += "pub enum WsMessage {\n"
        
        for msg_type, config in self.schema["messages"].items():
            # Extract payload type from schema reference
            payload_ref = config["payload"].get("$ref", "")
            if payload_ref:
                payload_type = payload_ref.split("/")[-1]
                
                # Add main message type
                variant_name = self._to_pascal_case(msg_type)
                code += f'    #[serde(rename = "{msg_type}"'
                
                # Add aliases if any
                aliases = config.get("aliases", [])
                if aliases:
                    alias_attrs = ", ".join([f'alias = "{alias}"' for alias in aliases])
                    code += f", {alias_attrs})]\n"
                else:
                    code += ")]\n"
                    
                code += f"    {variant_name} {{ data: {payload_type} }},\n\n"
                
        code += "    #[serde(other)]\n"
        code += "    Unknown,\n"
        code += "}\n\n"
        
        return code
        
    def _to_pascal_case(self, snake_str: str) -> str:
        """Convert snake_case to PascalCase."""
        return "".join(word.capitalize() for word in snake_str.split("_"))
        
    def _generate_rust_helpers(self) -> str:
        """Generate Rust helper implementations."""
        return """impl WsMessage {
    /// Extract the topic string for this message type.
    pub fn topic(&self) -> Option<String> {
        match self {
            // TODO: Generate topic extraction based on schema topic_pattern
            WsMessage::Unknown => None,
            _ => None, // Implement based on schema
        }
    }
}

/// Validate envelope format without deserializing payload
pub fn validate_envelope(data: &Value) -> Result<(), String> {
    // TODO: Implement JSON schema validation
    Ok(())
}

"""


def main():
    if len(sys.argv) != 2:
        print("Usage: python generate-ws-types.py <schema-file>")
        sys.exit(1)
        
    schema_file = sys.argv[1]
    generator = ProtocolCodeGenerator(schema_file)
    generator.generate_all()


if __name__ == "__main__":
    main()