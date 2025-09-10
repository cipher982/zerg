use serde_json::{Map, Value};

use super::generated_client::WorkflowDataContract;

fn map_node_type(frontend_type: &str) -> &str {
    match frontend_type {
        "AgentIdentity" => "agent",
        "Tool" => "tool",
        "Trigger" => "trigger",
        // All others map to conditional
        "UserInput" | "ResponseOutput" | "GenericNode" | _ => "conditional",
    }
}

fn lower_str(v: &Value) -> Option<String> {
    v.as_str().map(|s| s.to_ascii_lowercase())
}

/// Normalize and validate a canvas JSON structure into the exact API contract.
/// - Maps node `type` to backend literals (agent/tool/trigger/conditional)
/// - Flattens typed `config.trigger` → legacy keys (trigger_type/params/filters/enabled)
/// - Removes `config.trigger` to avoid duplication in persisted payloads
pub fn normalize_canvas_for_api(mut canvas: Value) -> Result<WorkflowDataContract, String> {
    // Ensure nodes exist
    if let Some(nodes) = canvas.get_mut("nodes").and_then(|n| n.as_array_mut()) {
        for node in nodes {
            if let Some(t) = node.get_mut("type") {
                if let Some(ts) = t.as_str() {
                    *t = Value::String(map_node_type(ts).to_string());
                }
            }

            // Normalize trigger config
            if let Some(Value::Object(cfg)) = node.get_mut("config") {
                // If typed trigger meta exists, mirror to legacy keys and remove it
                if let Some(Value::Object(trigger_obj)) = cfg.get("trigger") {
                    // Determine trigger_type string
                    let tt = trigger_obj
                        .get("type")
                        .and_then(lower_str)
                        .unwrap_or_else(|| "manual".to_string());

                    // Extract nested config object
                    let (enabled, params, filters) = if let Some(Value::Object(conf)) = trigger_obj.get("config") {
                        let enabled = conf
                            .get("enabled")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(true);
                        let params = conf
                            .get("params")
                            .cloned()
                            .unwrap_or_else(|| Value::Object(Map::new()));
                        let filters = conf
                            .get("filters")
                            .cloned()
                            .unwrap_or_else(|| Value::Array(vec![]));
                        (enabled, params, filters)
                    } else {
                        (true, Value::Object(Map::new()), Value::Array(vec![]))
                    };

                    cfg.insert("trigger_type".to_string(), Value::String(tt));
                    cfg.insert("enabled".to_string(), Value::Bool(enabled));
                    cfg.insert("params".to_string(), params);
                    cfg.insert("filters".to_string(), filters);

                    // Drop the typed field for persisted payloads
                    cfg.remove("trigger");
                }
            }
        }
    }

    // Validate to API contract type
    serde_json::from_value::<WorkflowDataContract>(canvas)
        .map_err(|e| format!("Canvas data doesn't match contract: {}", e))
}

