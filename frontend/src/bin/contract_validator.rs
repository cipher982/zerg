//! Fast contract validation binary that runs during `make start`
//! 
//! This validates that frontend data structures serialize to exactly
//! what the backend API contracts expect, catching runtime errors at build time.

use std::process;

use agent_platform_frontend::generated::api_contracts::*;

fn main() {
    println!("🔍 Running fast contract validation checks...");

    // Test 1: WorkflowNode contract validation
    if !test_workflow_node_contracts() {
        println!("❌ WorkflowNode contract validation failed");
        process::exit(1);
    }

    // Test 2: Canvas serialization
    if !test_canvas_contracts() {
        println!("❌ Canvas contract validation failed");
        process::exit(1);
    }

    println!("✅ All contract validation checks passed");
}

fn test_workflow_node_contracts() -> bool {
    println!("  📋 Testing WorkflowNode contracts...");

    // Create a node using generated contract structs
    let position = Position {
        x: 100.0,
        y: 200.0
    };

    let node = WorkflowNode {
        id: "test-node".to_string(),
        type_: "trigger".to_string(),
        position,
        config: None,
    };

    // Test serialization matches backend expectations
    let serialized = match serde_json::to_value(&node) {
        Ok(val) => val,
        Err(e) => {
            println!("    ❌ Failed to serialize WorkflowNode: {}", e);
            return false;
        }
    };

    // Verify required fields exist and have correct types
    match serialized.get("id") {
        Some(serde_json::Value::String(_)) => {},
        _ => {
            println!("    ❌ Missing or invalid 'id' field");
            return false;
        }
    }

    match serialized.get("type") {
        Some(serde_json::Value::String(_)) => {},
        _ => {
            println!("    ❌ Missing or invalid 'type' field");
            return false;
        }
    }

    match serialized.get("position") {
        Some(serde_json::Value::Object(_)) => {},
        _ => {
            println!("    ❌ Missing or invalid 'position' field");
            return false;
        }
    }

    // Test round-trip serialization
    let _deserialized: WorkflowNode = match serde_json::from_value(serialized) {
        Ok(node) => node,
        Err(e) => {
            println!("    ❌ Failed to deserialize WorkflowNode: {}", e);
            return false;
        }
    };

    println!("    ✅ WorkflowNode contract validation passed");
    true
}

fn test_canvas_contracts() -> bool {
    println!("  🎨 Testing Canvas contracts...");

    let node = WorkflowNode {
        id: "canvas-test-node".to_string(),
        type_: "trigger".to_string(),
        position: Position { x: 100.0, y: 200.0 },
        config: None,
    };

    let edge = WorkflowEdge {
        from_node_id: "canvas-test-node".to_string(),
        to_node_id: "canvas-test-node-2".to_string(),
        config: None,
    };

    let canvas = WorkflowCanvas {
        nodes: vec![node],
        edges: vec![edge],
    };

    // Test canvas serialization
    let serialized = match serde_json::to_value(&canvas) {
        Ok(val) => val,
        Err(e) => {
            println!("    ❌ Failed to serialize Canvas: {}", e);
            return false;
        }
    };

    // Verify canvas structure
    match serialized.get("nodes") {
        Some(serde_json::Value::Array(_)) => {},
        _ => {
            println!("    ❌ Missing or invalid 'nodes' field");
            return false;
        }
    }

    match serialized.get("edges") {
        Some(serde_json::Value::Array(_)) => {},
        _ => {
            println!("    ❌ Missing or invalid 'edges' field");
            return false;
        }
    }

    // Test round-trip serialization
    let _deserialized: WorkflowCanvas = match serde_json::from_value(serialized) {
        Ok(canvas) => canvas,
        Err(e) => {
            println!("    ❌ Failed to deserialize Canvas: {}", e);
            return false;
        }
    };

    println!("    ✅ Canvas contract validation passed");
    true
}