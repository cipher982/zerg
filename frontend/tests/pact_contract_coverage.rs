//! Integration test to ensure all WebSocket message types have corresponding Pact contracts.
//!
//! This test automatically detects when new message types are added to WsMessage
//! without corresponding contracts, or when contracts exist for message types
//! that are no longer in the enum.
//!
//! Run with: cargo test --test pact_contract_coverage

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

/// Extract all message type names from the WsMessage enum.
/// This is done by parsing the source code to get the exact enum variants
/// and their serde rename attributes.
fn get_ws_message_types() -> HashSet<String> {
    let mut message_types = HashSet::new();
    
    // Read the ws_schema.rs file
    let ws_schema_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/network/ws_schema.rs");
    
    let content = fs::read_to_string(&ws_schema_path)
        .expect("Failed to read ws_schema.rs");
    
    // Parse the file to find WsMessage enum variants
    let mut in_ws_message_enum = false;
    let mut in_serde_attribute = false;
    let mut attribute_buffer = String::new();
    
    for line in content.lines() {
        let trimmed = line.trim();
        
        // Start of WsMessage enum
        if trimmed.starts_with("pub enum WsMessage") {
            in_ws_message_enum = true;
            continue;
        }
        
        // End of enum
        if in_ws_message_enum && trimmed == "}" {
            break;
        }
        
        if !in_ws_message_enum {
            continue;
        }
        
        // Skip the Unknown variant as it's a catch-all
        if trimmed.contains("Unknown") {
            continue;
        }
        
        // Handle multi-line serde attributes
        if trimmed.starts_with("#[serde(") {
            in_serde_attribute = true;
            attribute_buffer.clear();
            attribute_buffer.push_str(trimmed);
            
            // If the attribute ends on the same line
            if trimmed.ends_with(")]") {
                in_serde_attribute = false;
                parse_serde_attribute(&attribute_buffer, &mut message_types);
            }
        } else if in_serde_attribute {
            // Continue collecting multi-line attribute
            attribute_buffer.push(' ');
            attribute_buffer.push_str(trimmed);
            
            if trimmed.ends_with(")]") {
                in_serde_attribute = false;
                parse_serde_attribute(&attribute_buffer, &mut message_types);
            }
        }
    }
    
    message_types
}

/// Parse a serde attribute string and extract rename and alias values
fn parse_serde_attribute(attr: &str, message_types: &mut HashSet<String>) {
    // Extract rename attribute
    if let Some(rename_start) = attr.find("rename = \"") {
        let start = rename_start + 10;
        let remaining = &attr[start..];
        if let Some(end) = remaining.find('"') {
            let rename_value = &remaining[..end];
            message_types.insert(rename_value.to_string());
        }
    }
    
    // Extract alias attributes
    let mut remaining = attr;
    while let Some(alias_start) = remaining.find("alias = \"") {
        let start = alias_start + 9;
        remaining = &remaining[start..];
        if let Some(end) = remaining.find('"') {
            let alias_value = &remaining[..end];
            message_types.insert(alias_value.to_string());
            remaining = &remaining[end + 1..];
        }
    }
}

/// Extract all message types that have Pact contracts defined.
fn get_pact_contract_types() -> HashSet<String> {
    let mut contract_types = HashSet::new();
    
    // Read the contract_capture.rs file
    let contract_capture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/bin/contract_capture.rs");
    
    let content = fs::read_to_string(&contract_capture_path)
        .expect("Failed to read contract_capture.rs");
    
    // Parse the JSON structure to find all message types in interactions
    // Look for patterns like "type": "message_type" in both request and response bodies
    for line in content.lines() {
        let trimmed = line.trim();
        
        // Look for type fields in the JSON
        if trimmed.contains("\"type\":") && trimmed.contains("\"") {
            // Extract the value after "type":
            if let Some(type_start) = trimmed.find("\"type\":") {
                let after_type = &trimmed[type_start + 7..].trim();
                if after_type.starts_with('"') {
                    let value_start = 1;
                    if let Some(value_end) = after_type[value_start..].find('"') {
                        let type_value = &after_type[value_start..value_start + value_end];
                        
                        // Skip generic types like "websocket", "text", and "subscribe"
                        if type_value != "websocket" && type_value != "text" && type_value != "subscribe" {
                            contract_types.insert(type_value.to_string());
                        }
                    }
                }
            }
        }
    }
    
    contract_types
}

#[test]
fn test_all_ws_message_types_have_contracts() {
    let ws_message_types = get_ws_message_types();
    let contract_types = get_pact_contract_types();
    
    // Special cases: Some message types might not need contracts
    // (e.g., client-only messages or special internal types)
    let exceptions = HashSet::from([
        // Add any message types here that should be excluded from contract testing
        // Example: "internal_debug".to_string(),
    ]);
    
    // Filter out exceptions
    let ws_message_types_filtered: HashSet<String> = ws_message_types
        .difference(&exceptions)
        .cloned()
        .collect();
    
    // Find message types without contracts
    let mut missing_contracts: Vec<String> = ws_message_types_filtered
        .difference(&contract_types)
        .cloned()
        .collect();
    missing_contracts.sort();
    
    // Find contracts without corresponding message types (orphaned contracts)
    let mut orphaned_contracts: Vec<String> = contract_types
        .difference(&ws_message_types)
        .cloned()
        .collect();
    orphaned_contracts.sort();
    
    // Generate helpful error messages
    let mut errors = Vec::new();
    
    if !missing_contracts.is_empty() {
        errors.push(format!(
            "The following WebSocket message types are missing Pact contracts:\n  - {}",
            missing_contracts.join("\n  - ")
        ));
    }
    
    if !orphaned_contracts.is_empty() {
        errors.push(format!(
            "The following Pact contracts have no corresponding WebSocket message type:\n  - {}",
            orphaned_contracts.join("\n  - ")
        ));
    }
    
    // Print diagnostics for debugging
    println!("\n=== Pact Contract Coverage Report ===");
    println!("WebSocket message types found: {} total", ws_message_types.len());
    println!("  Types: {:?}", {
        let mut sorted: Vec<_> = ws_message_types.iter().cloned().collect();
        sorted.sort();
        sorted
    });
    
    println!("\nPact contract types found: {} total", contract_types.len());
    println!("  Types: {:?}", {
        let mut sorted: Vec<_> = contract_types.iter().cloned().collect();
        sorted.sort();
        sorted
    });
    
    println!("\nExceptions (types excluded from contract requirement): {:?}", exceptions);
    
    // Assert both lists are in sync
    assert!(
        errors.is_empty(),
        "\n\nPact contract coverage issues found:\n\n{}\n\n\
        To fix this:\n\
        1. For missing contracts: Add interactions to contract_capture.rs for each missing message type\n\
        2. For orphaned contracts: Either remove the contract or add the message type to WsMessage enum\n\
        3. If a message type should be excluded from contract testing, add it to the 'exceptions' set in this test\n",
        errors.join("\n\n")
    );
    
    println!("\n✅ All WebSocket message types have corresponding Pact contracts!");
}

#[test]
fn generate_pact_coverage_report() {
    let ws_message_types = get_ws_message_types();
    let contract_types = get_pact_contract_types();
    
    let mut ws_types_sorted: Vec<_> = ws_message_types.iter().cloned().collect();
    ws_types_sorted.sort();
    
    let mut contract_types_sorted: Vec<_> = contract_types.iter().cloned().collect();
    contract_types_sorted.sort();
    
    println!("\n=== Pact Contract Coverage Report ===");
    println!("\nTotal WebSocket message types: {}", ws_message_types.len());
    println!("Total Pact contracts: {}", contract_types.len());
    
    let coverage_percentage = if ws_message_types.is_empty() {
        0.0
    } else {
        (contract_types.intersection(&ws_message_types).count() as f64 / ws_message_types.len() as f64) * 100.0
    };
    
    println!("Coverage: {:.1}%", coverage_percentage);
    
    println!("\n📊 Coverage Status:");
    println!("─────────────────────────────────────────────────");
    println!("{:<30} │ {:<15}", "Message Type", "Contract Status");
    println!("─────────────────────────────────────────────────");
    
    for msg_type in &ws_types_sorted {
        let status = if contract_types.contains(msg_type) {
            "✅ Covered"
        } else {
            "❌ Missing"
        };
        println!("{:<30} │ {:<15}", msg_type, status);
    }
    
    println!("─────────────────────────────────────────────────");
    
    // Show orphaned contracts
    let orphaned: Vec<_> = contract_types.difference(&ws_message_types).cloned().collect();
    if !orphaned.is_empty() {
        println!("\n⚠️  Orphaned Contracts (no corresponding message type):");
        for contract in orphaned {
            println!("  - {}", contract);
        }
    }
    
    // Generate recommendations
    println!("\n📝 Recommendations:");
    
    let missing: Vec<_> = ws_message_types.difference(&contract_types).cloned().collect();
    if !missing.is_empty() {
        println!("\n1. Add the following interactions to contract_capture.rs:");
        for msg_type in missing.iter().take(3) {
            println!("\n   Example for '{}':", msg_type);
            println!("   {{");
            println!("       \"description\": \"TODO: describe {} interaction\",", msg_type);
            println!("       \"request\": {{");
            println!("           \"type\": \"websocket\",");
            println!("           \"subtype\": \"text\",");
            println!("           \"body\": {{");
            println!("               // TODO: Add request body for {}", msg_type);
            println!("           }}");
            println!("       }},");
            println!("       \"response\": {{");
            println!("           \"body\": {{");
            println!("               \"type\": \"{}\",", msg_type);
            println!("               // TODO: Add response fields");
            println!("           }}");
            println!("       }}");
            println!("   }}");
        }
        
        if missing.len() > 3 {
            println!("\n   ... and {} more message types", missing.len() - 3);
        }
    }
    
    println!("\n=====================================");
}