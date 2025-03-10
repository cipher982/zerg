use wasm_bindgen::prelude::*;
use serde_json::{to_string, from_str};
use crate::state::AppState;
use crate::models::{Node, ApiAgent, ApiAgentCreate, ApiAgentUpdate};
use crate::network::ApiClient;
use std::collections::HashMap;
use wasm_bindgen_futures::spawn_local;
use wasm_bindgen::closure::Closure;
use crate::constants::{DEFAULT_SYSTEM_INSTRUCTIONS, DEFAULT_TASK_INSTRUCTIONS};

// Add API URL configuration
const API_BASE_URL: &str = "http://localhost:8001/api";

// Structure to store viewport data
#[derive(serde::Serialize, serde::Deserialize)]
struct ViewportData {
    x: f64,
    y: f64,
    zoom: f64,
}

// Store the active view (Dashboard or Canvas)
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ActiveView {
    Dashboard,
    Canvas,
}

/// Save current app state (previously using localStorage)
pub fn save_state(app_state: &AppState) -> Result<(), JsValue> {
    // No longer saving to localStorage
    web_sys::console::log_1(&"State saving to API only".into());
    
    // Save changes to API
    save_state_to_api(app_state);
    
    Ok(())
}

/// Load app state from API only
pub fn load_state(app_state: &mut AppState) -> Result<bool, JsValue> {
    // Load data from API only
    load_state_from_api(app_state);
    
    // Return true to indicate we started the loading process
    // Actual loading happens asynchronously
    Ok(true)
}

/// Clear all stored data
pub fn clear_storage() -> Result<(), JsValue> {
    // No localStorage to clear, but we might want to add
    // API endpoint calls to clear data in the future
    
    Ok(())
}

/// Save app state to API (transitional approach)
pub fn save_state_to_api(app_state: &AppState) {
    // Clone the necessary data before passing to the async block
    let nodes = app_state.nodes.clone();
    let selected_model = app_state.selected_model.clone();
    let is_dragging = app_state.is_dragging_agent;
    
    // Skip state saving if we're actively dragging to prevent spamming API
    if is_dragging {
        web_sys::console::log_1(&"Skipping API save during active dragging".into());
        return;
    }
    
    // Spawn an async task to save agents to API
    spawn_local(async move {
        // We'll need to convert each Node of type AgentIdentity to an ApiAgent
        for (node_id, node) in nodes.iter() {
            // ONLY process nodes that are already known to be agents (with agent- prefix)
            // Skip all other nodes to ensure we NEVER create agents outside the create button flow
            if let Some(agent_id_str) = node_id.strip_prefix("agent-") {
                if let Ok(agent_id) = agent_id_str.parse::<u32>() {
                    if let crate::models::NodeType::AgentIdentity = node.node_type {
                        // Only synchronize agent identity nodes that already have agent IDs
                        let agent_update = ApiAgentUpdate {
                            name: Some(node.text.clone()),
                            status: node.status.clone(),
                            system_instructions: Some(node.system_instructions.clone()
                                .unwrap_or_else(|| DEFAULT_SYSTEM_INSTRUCTIONS.to_string())),
                            task_instructions: Some(node.task_instructions.clone()
                                .unwrap_or_else(|| DEFAULT_TASK_INSTRUCTIONS.to_string())),
                            model: Some(selected_model.clone()),
                            schedule: None,
                            config: None, // Will need to add more node data as needed
                        };
                        
                        // Try to update existing agent
                        let agent_json = match to_string(&agent_update) {
                            Ok(json) => json,
                            Err(e) => {
                                web_sys::console::error_1(&format!("Error serializing agent update: {}", e).into());
                                continue;
                            }
                        };
                        
                        if let Err(e) = ApiClient::update_agent(agent_id, &agent_json).await {
                            web_sys::console::error_1(&format!("Error updating agent in API: {:?}", e).into());
                        } else {
                            web_sys::console::log_1(&format!("Updated agent {} in API", agent_id).into());
                        }
                    }
                }
            } else if node_id.starts_with("node_") {
                // This is a canvas node with no corresponding agent - do not create an agent for it
                // Simply log it to console
                if let crate::models::NodeType::AgentIdentity = node.node_type {
                    web_sys::console::log_1(&format!("Canvas node {} of type AgentIdentity exists but has no agent ID, skipping API sync", node_id).into());
                }
            }
            // Completely remove the 'else' branch that was creating new agents
        }
    });
}

/// Load app state from API
pub fn load_state_from_api(app_state: &mut AppState) {
    // Set loading state flags
    app_state.is_loading = true;
    app_state.data_loaded = false;
    app_state.api_load_attempted = true;
    
    // Spawn an async task to load agents from API
    spawn_local(async move {
        // Try to fetch all agents from the API
        match ApiClient::get_agents().await {
            Ok(agents_json) => {
                match from_str::<Vec<ApiAgent>>(&agents_json) {
                    Ok(agents) => {
                        web_sys::console::log_1(&format!("Loaded {} agents from API", agents.len()).into());
                        
                        // Convert API agents to Node objects
                        let mut loaded_nodes = HashMap::new();
                        
                        for agent in agents {
                            if let Some(id) = agent.id {
                                let node_id = format!("agent-{}", id);
                                
                                // Create a node for this agent
                                let node = Node {
                                    id: node_id.clone(),
                                    x: 100.0, // Default position, would need to be stored in config
                                    y: 100.0,
                                    text: agent.name,
                                    width: 300.0, // Default width
                                    height: 200.0, // Default height
                                    color: "#e0f7fa".to_string(), // Default color
                                    parent_id: None,
                                    node_type: crate::models::NodeType::AgentIdentity,
                                    system_instructions: agent.system_instructions.clone(),
                                    task_instructions: agent.task_instructions.clone(),
                                    history: None, // We'll load this separately
                                    status: agent.status,
                                };
                                
                                // Add the node to our temporary collection
                                loaded_nodes.insert(node_id, node);
                            }
                        }
                        
                        // Update loading state flags after the API call completes
                        crate::state::APP_STATE.with(|state_ref| {
                            let mut state = state_ref.borrow_mut();
                            state.is_loading = false;
                            state.data_loaded = true;
                        });
                        
                        // Use a callback function to update the app state
                        if let Err(e) = crate::state::update_app_state_from_api(loaded_nodes) {
                            web_sys::console::error_1(&format!("Error updating app state: {:?}", e).into());
                        }
                        
                        // Schedule UI refresh after the current function completes
                        let window = web_sys::window().expect("no global window exists");
                        let closure = Closure::once(|| {
                            // This will run after the current execution context is complete
                            if let Err(e) = crate::state::AppState::refresh_ui_after_state_change() {
                                web_sys::console::warn_1(&format!("Failed to refresh UI after state load: {:?}", e).into());
                            }
                        });
                        
                        window.set_timeout_with_callback_and_timeout_and_arguments_0(
                            closure.as_ref().unchecked_ref(),
                            0
                        ).expect("Failed to set timeout");
                        
                        // Ensure closure lives long enough
                        closure.forget();
                    },
                    Err(e) => {
                        web_sys::console::error_1(&format!("Error parsing agents from API: {:?}", e).into());
                        mark_loading_complete();
                    }
                }
            },
            Err(e) => {
                web_sys::console::error_1(&format!("Error fetching agents from API: {:?}", e).into());
                mark_loading_complete();
            }
        }
    });
}

// Helper function to mark loading as complete and refresh UI
fn mark_loading_complete() {
    crate::state::APP_STATE.with(|state_ref| {
        let mut state = state_ref.borrow_mut();
        state.is_loading = false;
        state.data_loaded = true;
    });
    
    // Schedule UI refresh
    let window = web_sys::window().expect("no global window exists");
    let closure = Closure::once(|| {
        if let Err(e) = crate::state::AppState::refresh_ui_after_state_change() {
            web_sys::console::warn_1(&format!("Failed to refresh UI after state load: {:?}", e).into());
        }
    });
    
    window.set_timeout_with_callback_and_timeout_and_arguments_0(
        closure.as_ref().unchecked_ref(),
        0
    ).expect("Failed to set timeout");
    
    closure.forget();
}

/// Save agent messages to the API
pub fn save_agent_messages_to_api(node_id: &str, messages: &[crate::models::Message]) {
    // Extract the agent ID from the node ID if it starts with "agent-"
    if let Some(agent_id_str) = node_id.strip_prefix("agent-") {
        if let Ok(agent_id) = agent_id_str.parse::<u32>() {
            // Clone the messages for the async block
            let messages_clone = messages.to_vec();
            
            // Spawn an async task to save messages
            spawn_local(async move {
                for message in messages_clone {
                    // Convert our internal Message type to ApiMessageCreate
                    let api_message = crate::models::ApiMessageCreate {
                        role: message.role,
                        content: message.content,
                    };
                    
                    // Convert to JSON
                    let message_json = match to_string(&api_message) {
                        Ok(json) => json,
                        Err(e) => {
                            web_sys::console::error_1(&format!("Error serializing message: {}", e).into());
                            continue;
                        }
                    };
                    
                    // Send to API
                    if let Err(e) = ApiClient::create_agent_message(agent_id, &message_json).await {
                        web_sys::console::error_1(&format!("Error saving message to API: {:?}", e).into());
                    } else {
                        web_sys::console::log_1(&format!("Saved message to agent {} in API", agent_id).into());
                    }
                }
            });
        }
    }
}

/// Load agent messages from the API
pub fn load_agent_messages_from_api(node_id: &String, _agent_id: u32) {
    // Extract the agent ID from the node ID
    if let Some(agent_id_str) = node_id.strip_prefix("agent-") {
        if let Ok(agent_id) = agent_id_str.parse::<u32>() {
            // Reference to the node ID for the closure
            let node_id = node_id.to_string();
            
            // Spawn an async task to load messages
            spawn_local(async move {
                match ApiClient::get_agent_messages(agent_id).await {
                    Ok(messages_json) => {
                        match from_str::<Vec<crate::models::ApiMessage>>(&messages_json) {
                            Ok(api_messages) => {
                                web_sys::console::log_1(&format!("Loaded {} messages for agent {}", api_messages.len(), agent_id).into());
                                
                                // Convert API messages to our internal Message type
                                let messages: Vec<crate::models::Message> = api_messages.into_iter()
                                    .map(|api_msg| crate::models::Message {
                                        role: api_msg.role,
                                        content: api_msg.content,
                                        timestamp: 0, // We'll use current timestamp since created_at is a string
                                    })
                                    .collect();
                                
                                // Update the node with the messages
                                if !messages.is_empty() {
                                    crate::state::APP_STATE.with(|app_state_ref| {
                                        let mut app_state = app_state_ref.borrow_mut();
                                        if let Some(node) = app_state.nodes.get_mut(&node_id) {
                                            node.history = Some(messages);
                                            app_state.state_modified = true;
                                        }
                                    });
                                }
                            },
                            Err(e) => {
                                web_sys::console::error_1(&format!("Error parsing messages: {}", e).into());
                            }
                        }
                    },
                    Err(e) => {
                        web_sys::console::error_1(&format!("Error loading messages: {:?}", e).into());
                    }
                }
            });
        }
    }
}

/// One-time migration utility to transfer data from localStorage to the API
/// This function is no longer needed but kept for reference
pub fn migrate_local_storage_to_api() -> Result<(), JsValue> {
    web_sys::console::log_1(&"localStorage migration no longer needed - using API only".into());
    Ok(())
}

/// Load app state with API as source of truth
pub fn load_state_prioritizing_api(app_state: &mut AppState) {
    web_sys::console::log_1(&"Loading data from API only".into());
    
    // Just call our standard load function
    load_state_from_api(app_state);
}

/// Helper function to save just the nodes to the API
fn save_nodes_to_api(nodes: &HashMap<String, Node>) -> Result<(), JsValue> {
    // Implementation to save nodes to API
    // This is a simplified version - you'll need to expand based on your API client
    web_sys::console::log_1(&format!("Saving {} nodes to API", nodes.len()).into());
    
    for (node_id, node) in nodes {
        if let crate::models::NodeType::AgentIdentity = node.node_type {
            // Create or update agent in API
            web_sys::console::log_1(&format!("Would save agent {}: {}", node_id, node.text).into());
            // Here you would call your API client to save the agent
        }
    }
    
    Ok(())
} 