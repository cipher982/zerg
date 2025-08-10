#!/usr/bin/env python3
"""
Generate type-safe API client from OpenAPI schema.

This prevents API endpoint mismatches by generating the client 
from the single source of truth (api-schema.yml).
"""

import yaml
import json
from pathlib import Path

def generate_rust_api_client(openapi_spec):
    """Generate Rust API client from OpenAPI spec."""
    
    # Extract paths and generate Rust functions
    rust_code = """
// GENERATED CODE - DO NOT EDIT
// Generated from api-schema.yml

use wasm_bindgen::JsValue;
use crate::network::api_client::ApiClient;

impl ApiClient {
"""
    
    for path, methods in openapi_spec.get('paths', {}).items():
        for method, spec in methods.items():
            if method.upper() not in ['GET', 'POST', 'PUT', 'DELETE', 'PATCH']:
                continue
                
            # Convert OpenAPI path to function name
            func_name = generate_function_name(path, method, spec)
            
            # Extract path parameters
            path_params = []
            for param in spec.get('parameters', []):
                if param.get('in') == 'path':
                    path_params.append(param['name'])
            
            # Generate function signature
            params = ', '.join([f'{p}: u32' for p in path_params])
            if params:
                params = ', ' + params
                
            rust_code += f"""
    /// {spec.get('summary', 'API call')}
    pub async fn {func_name}({params}) -> Result<String, JsValue> {{
        let url = format!(
            "{{}}{}",
            Self::api_base_url(){','.join([f', {p}' for p in path_params])}
        );
        Self::fetch_json(&url, "{method.upper()}", None).await
    }}
"""
    
    rust_code += "}\n"
    return rust_code

def generate_function_name(path, method, spec):
    """Convert OpenAPI path to Rust function name."""
    # Remove /api/ prefix and convert to snake_case
    clean_path = path.replace('/api/', '').replace('/', '_').replace('{', '').replace('}', '')
    summary = spec.get('operationId', spec.get('summary', ''))
    
    if summary:
        return summary.lower().replace(' ', '_').replace('-', '_')
    
    return f"{method.lower()}_{clean_path}"

def main():
    """Generate API client from OpenAPI schema."""
    
    schema_path = Path(__file__).parent.parent / 'api-schema.yml'
    output_path = Path(__file__).parent.parent / 'frontend' / 'src' / 'generated' / 'api_client.rs'
    
    if not schema_path.exists():
        print(f"❌ OpenAPI schema not found: {schema_path}")
        return 1
    
    # Load OpenAPI schema
    with open(schema_path, 'r') as f:
        openapi_spec = yaml.safe_load(f)
    
    # Generate Rust client
    rust_code = generate_rust_api_client(openapi_spec)
    
    # Write to file
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, 'w') as f:
        f.write(rust_code)
    
    print(f"✅ Generated API client: {output_path}")
    return 0

if __name__ == '__main__':
    exit(main())