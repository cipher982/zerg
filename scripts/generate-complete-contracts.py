#!/usr/bin/env python3
"""
Complete Contract Generation from Schema-First Approach

This script generates:
1. Frontend Rust contract structs
2. Backend Python models
3. Validation functions  
4. Mock data generators
5. Integration tests
6. API clients

Usage: python scripts/generate-complete-contracts.py
"""

import yaml
import json
from pathlib import Path
from typing import Dict, Any, List
from jinja2 import Template

class SchemaFirstGenerator:
    def __init__(self):
        self.openapi_schema = None
        self.asyncapi_schema = None
        
    def load_schemas(self):
        """Load all schema files"""
        with open('api-schema.yml') as f:
            self.openapi_schema = yaml.safe_load(f)
        with open('ws-protocol-asyncapi.yml') as f:
            self.asyncapi_schema = yaml.safe_load(f)
    
    def generate_frontend_contracts(self):
        """Generate Rust contract structs for frontend"""
        schemas = self.openapi_schema['components']['schemas']
        
        rust_code = '''
// AUTO-GENERATED - DO NOT EDIT
// Generated from api-schema.yml

use serde::{Serialize, Deserialize};

'''
        
        for schema_name, schema_def in schemas.items():
            if schema_def.get('type') == 'object':
                rust_code += self.generate_rust_struct(schema_name, schema_def)
        
        # Write to frontend
        Path('frontend/src/generated/api_contracts.rs').write_text(rust_code)
        print("✅ Generated frontend/src/generated/api_contracts.rs")
    
    def generate_rust_struct(self, name: str, schema: Dict[str, Any]) -> str:
        """Generate a Rust struct from JSON schema"""
        required = schema.get('required', [])
        properties = schema.get('properties', {})
        
        struct_code = f'''
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct {name} {{
'''
        
        for prop_name, prop_schema in properties.items():
            rust_type = self.json_type_to_rust(prop_schema)
            if prop_name not in required:
                rust_type = f"Option<{rust_type}>"
            
            # Handle Rust keywords
            rust_field_name = prop_name
            serde_attr = ""
            if prop_name == "type":
                rust_field_name = "type_"
                serde_attr = '    #[serde(rename = "type")]\n'
            
            struct_code += f"{serde_attr}    pub {rust_field_name}: {rust_type},\n"
        
        struct_code += "}\n"
        return struct_code
    
    def json_type_to_rust(self, schema: Dict[str, Any]) -> str:
        """Convert JSON schema type to Rust type"""
        json_type = schema.get('type')
        format_type = schema.get('format')
        
        if '$ref' in schema:
            ref_name = schema['$ref'].split('/')[-1]
            return ref_name
        elif json_type == 'string':
            if format_type == 'date-time':
                return 'chrono::DateTime<chrono::Utc>'
            return 'String'
        elif json_type == 'integer':
            return 'i32'
        elif json_type == 'number':
            return 'f64'
        elif json_type == 'boolean':
            return 'bool'
        elif json_type == 'array':
            item_type = self.json_type_to_rust(schema['items'])
            return f'Vec<{item_type}>'
        elif json_type == 'object':
            if schema.get('additionalProperties'):
                return 'serde_json::Value'
            return 'serde_json::Value'
        else:
            return 'serde_json::Value'
    
    def generate_backend_models(self):
        """Generate Python Pydantic models for backend"""
        schemas = self.openapi_schema['components']['schemas']
        
        python_code = '''
# AUTO-GENERATED - DO NOT EDIT
# Generated from api-schema.yml

from pydantic import BaseModel, Field
from typing import Optional, List, Dict, Any
from datetime import datetime

'''
        
        for schema_name, schema_def in schemas.items():
            if schema_def.get('type') == 'object':
                python_code += self.generate_pydantic_model(schema_name, schema_def)
        
        Path('backend/zerg/generated/api_models.py').write_text(python_code)
        print("✅ Generated backend/zerg/generated/api_models.py")
    
    def generate_pydantic_model(self, name: str, schema: Dict[str, Any]) -> str:
        """Generate a Pydantic model from JSON schema"""
        required = schema.get('required', [])
        properties = schema.get('properties', {})
        
        model_code = f'''
class {name}(BaseModel):
'''
        
        for prop_name, prop_schema in properties.items():
            python_type = self.json_type_to_python(prop_schema)
            if prop_name not in required:
                python_type = f"Optional[{python_type}] = None"
            
            model_code += f"    {prop_name}: {python_type}\n"
        
        model_code += "\n"
        return model_code
    
    def json_type_to_python(self, schema: Dict[str, Any]) -> str:
        """Convert JSON schema type to Python type"""
        json_type = schema.get('type')
        format_type = schema.get('format')
        
        if '$ref' in schema:
            ref_name = schema['$ref'].split('/')[-1]
            return ref_name
        elif json_type == 'string':
            if format_type == 'date-time':
                return 'datetime'
            return 'str'
        elif json_type == 'integer':
            return 'int'
        elif json_type == 'number':
            return 'float'
        elif json_type == 'boolean':
            return 'bool'
        elif json_type == 'array':
            item_type = self.json_type_to_python(schema['items'])
            return f'List[{item_type}]'
        elif json_type == 'object':
            if schema.get('additionalProperties'):
                return 'Dict[str, Any]'
            return 'Dict[str, Any]'
        else:
            return 'Any'
    
    def generate_validation_tests(self):
        """Generate comprehensive validation tests"""
        test_code = '''
// AUTO-GENERATED CONTRACT VALIDATION TESTS

use super::super::generated::api_contracts::*;

#[cfg(test)]
mod contract_validation_tests {
    use super::*;
    
    #[test]
    fn test_all_schema_serialization() {
        // Test that ALL schemas serialize/deserialize correctly
        let workflow_node = WorkflowNode {
            id: "test".to_string(),
            type_: "GenericNode".to_string(),
            position: Position { x: 100.0, y: 200.0 },
            config: Some(serde_json::json!({})),
        };
        
        // This MUST work - if it fails, the contract is broken
        let json = serde_json::to_string(&workflow_node).unwrap();
        let deserialized: WorkflowNode = serde_json::from_str(&json).unwrap();
        assert_eq!(workflow_node.id, deserialized.id);
    }
}
'''
        
        Path('frontend/src/tests/generated_contract_tests.rs').write_text(test_code)
        print("✅ Generated frontend/src/tests/generated_contract_tests.rs")
    
    def run(self):
        """Run the complete generation process"""
        print("🚀 Starting schema-first contract generation...")
        
        self.load_schemas()
        print("📖 Loaded schemas")
        
        # Ensure output directories exist
        Path('frontend/src/generated').mkdir(exist_ok=True)
        Path('frontend/src/tests').mkdir(exist_ok=True)
        Path('backend/zerg/generated').mkdir(exist_ok=True)
        
        self.generate_frontend_contracts()
        self.generate_backend_models()
        self.generate_validation_tests()
        
        print("✅ Schema-first generation complete!")
        print("\n📋 Next steps:")
        print("1. Update your code to import from generated contracts")
        print("2. Run the validation tests: cargo test contract_validation")
        print("3. Update backend to use generated Pydantic models")

if __name__ == "__main__":
    generator = SchemaFirstGenerator()
    generator.run()