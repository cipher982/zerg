use wasm_bindgen::prelude::*;
use serde_json::{to_string, from_str};
use crate::state::AppState;
use crate::models::{Node, ApiAgent, ApiAgentUpdate, Workflow};
use crate::network::ApiClient;
use std::collections::HashMap;
use wasm_bindgen_futures::spawn_local;
use wasm_bindgen::closure::Closure;
// (no constants needed now)
// Legacy extension trait for CanvasNode helpers

// Add API URL configuration
#[allow(dead_code)]
const API_BASE_URL: &str = "http://localhost:8001/api";

// Structure to store viewport data
#[derive(serde::Serialize, serde::Deserialize)]
struct ViewportData {
    x: f64,
    y: f64,
    zoom: f64,
}

// Store the active view (Dashboard, Canvas, or Chat)
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ActiveView {
    Dashboard,
    Canvas,
    ChatView,
}

/// Save current app state to API
pub fn save_state(app_state: &AppState) -> Result<(), JsValue> {
    // Save changes to API
    save_state_to_api(app_state);
    
    // Save the original node data
    let window = web_sys::window().expect("no global window exists");
    let local_storage = window.local_storage()?.expect("no local storage exists");
    
    // Convert nodes to a JSON string
    let nodes_str = to_string(&app_state.nodes).map_err(|e| JsValue::from_str(&e.to_string()))?;
    
    // Save to localStorage
    local_storage.set_item("nodes", &nodes_str)?;
    
    // Save viewport position and zoom
    let viewport_data = ViewportData {
        x: app_state.viewport_x,
        y: app_state.viewport_y,
        zoom: app_state.zoom_level,
    };
    
    let viewport_str = to_string(&viewport_data).map_err(|e| JsValue::from_str(&e.to_string()))?;
    local_storage.set_item("viewport", &viewport_str)?;
    
    // Save active view
    let active_view_str = to_string(&app_state.active_view).map_err(|e| JsValue::from_str(&e.to_string()))?;
    local_storage.set_item("active_view", &active_view_str)?;
    
    // Save workflows and CanvasNodes
    save_workflows(app_state)?;
    
    Ok(())
}

/// Load app state from API
pub fn load_state(app_state: &mut AppState) -> Result<bool, JsValue> {
    // Load data from API
    load_state_from_api(app_state);
    
    // Also load workflows
    load_workflows(app_state)?;
    
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
    let _selected_model = app_state.selected_model.clone();
    let is_dragging = app_state.is_dragging_agent;
    let agents = app_state.agents.clone();
    
    // Skip state saving if we're actively dragging to prevent spamming API
    if is_dragging {
        web_sys::console::log_1(&"Skipping API save during active dragging".into());
        return;
    }
    
    // Spawn an async task to save agents to API
    spawn_local(async move {
        // We'll need to convert each Node of type AgentIdentity to an ApiAgent
        for (_node_id, node) in nodes.iter() {
            if let crate::models::NodeType::AgentIdentity = node.node_type {
                if let Some(agent_id) = node.agent_id {
                        // Create the agent update object
                        let agent_update = ApiAgentUpdate {
                            name: Some(node.text.clone()),
                            status: None,
                            // Only update mutable fields we track on the node (name for now).
                            // System & task instructions are edited through the agent-centric
                            // modal and therefore persisted separately.
                            system_instructions: None,
                            task_instructions: None,
                            model: None,
                            schedule: None,
                            config: None, // Will need to add more node data as needed
                            last_error: None,
                        };
                        
                        // Check if the agent data has actually changed by comparing with backend data
                        let should_update = if let Some(backend_agent) = agents.get(&agent_id) {
                            // Log the values for debugging
                            web_sys::console::log_1(&format!("Comparing agent {}: Backend vs Frontend", agent_id).into());
                            web_sys::console::log_1(&format!("Name: '{}' vs '{}'", 
                                backend_agent.name, 
                                agent_update.name.as_ref().unwrap_or(&String::new())).into());
                            web_sys::console::log_1(&format!("System Instructions: '{:?}' vs '{:?}'", 
                                backend_agent.system_instructions, 
                                agent_update.system_instructions).into());
                            web_sys::console::log_1(&format!("Model: '{:?}' vs '{:?}'", 
                                backend_agent.model, 
                                agent_update.model).into());
                            
                            // Compare name (String vs Option<String>)
                            let name_changed = backend_agent.name != *agent_update.name.as_ref().unwrap_or(&String::new());
                            // Compare system_instructions (Option<String> vs Option<String>)
                            // We are not updating system/task instructions via this path anymore
                            let sys_instr_changed = false;
                            // Compare model (Option<String> vs Option<String>) – still relevant once we wire model editing
                            let model_changed = backend_agent.model != agent_update.model;
                            
                            if name_changed {
                                web_sys::console::log_1(&"Name differs".into());
                            }
                            if sys_instr_changed {
                                web_sys::console::log_1(&"System instructions differ".into());
                            }
                            if model_changed {
                                web_sys::console::log_1(&"Model differs".into());
                            }
                            
                            name_changed || sys_instr_changed || model_changed
                        } else {
                            // If we don't have backend data, assume we need to update
                            web_sys::console::log_1(&format!("No backend data for agent {}, will update", agent_id).into());
                            true
                        };
                        
                        // Only update if there are actual changes
                        if should_update {
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
                                web_sys::console::log_1(&format!("Updated agent {} in API (with changes)", agent_id).into());
                            }
                        } else {
                            // No changes detected, log it differently
                            web_sys::console::log_1(&format!("Verified agent {} in API (no changes)", agent_id).into());
                        }
                    }
                }
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
                        
                        // Update the agents in the global APP_STATE
                        crate::state::APP_STATE.with(|state_ref| {
                            let mut state = state_ref.borrow_mut();
                            state.agents.clear();
                            
                            // Add each agent to the HashMap
                            for agent in &agents {
                                if let Some(id) = agent.id {
                                    state.agents.insert(id, agent.clone());
                                }
                            }
                            
                            // IMPORTANT: We no longer automatically create nodes for agents
                            // This is a key part of separating agent domain logic from node UI logic
                            
                            // Update loading state flags after the API call completes
                            state.is_loading = false;
                            state.data_loaded = true;
                            
                            // If there are no nodes but we have agents, show a message to user
                            if state.nodes.is_empty() && !state.agents.is_empty() {
                                // Show message that agents are loaded but not displayed
                                web_sys::console::log_1(&"Agents loaded but no nodes exist. Use 'Generate Canvas' to visualize agents.".into());
                                
                                // In a real app, you might want to display a UI message or button
                                // that lets users generate nodes for their agents
                            }
                        });
                    },
                    Err(e) => {
                        web_sys::console::error_1(&format!("Error parsing agents from API: {}", e).into());
                        // Update loading state flags even in case of error
                        crate::state::APP_STATE.with(|state_ref| {
                            let mut state = state_ref.borrow_mut();
                            state.is_loading = false;
                        });
                    }
                }
            },
            Err(e) => {
                web_sys::console::error_1(&format!("Error fetching agents from API: {:?}", e).into());
                // Update loading state flags even in case of error
                crate::state::APP_STATE.with(|state_ref| {
                    let mut state = state_ref.borrow_mut();
                    state.is_loading = false;
                });
            }
        }
        
        // Schedule a UI refresh
        crate::state::AppState::refresh_ui_after_state_change().expect("Failed to refresh UI");
        
        // Call a function to signal that loading is complete
        mark_loading_complete();
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
    // Resolve the `agent_id` using the global mapping.  This avoids any
    // string-based assumptions about the node_id.
    use crate::state::APP_STATE;

    if let Some(agent_id) = APP_STATE.with(|s| {
        let st = s.borrow();
        st.nodes.get(node_id).and_then(|n| n.agent_id)
    }) {
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

/// Load agent messages from the API
#[allow(dead_code)]
pub fn load_agent_messages_from_api(node_id: &String, _agent_id: u32) {
    use crate::state::APP_STATE;

    if let Some(agent_id) = APP_STATE.with(|s| {
        let st = s.borrow();
        st.nodes.get(node_id).and_then(|n| n.agent_id)
    }) {
            // Reference to the node ID for the closure
            let _node_id = node_id.to_string();
            
            // Spawn an async task to load messages
            spawn_local(async move {
                match ApiClient::get_agent_messages(agent_id).await {
                    Ok(messages_json) => {
                        match from_str::<Vec<crate::models::ApiMessage>>(&messages_json) {
                            Ok(api_messages) => {
                                web_sys::console::log_1(&format!("Loaded {} messages for agent {}", api_messages.len(), agent_id).into());
                                
                                // Convert API messages to our internal Message type
                                let _messages: Vec<crate::models::Message> = api_messages.into_iter()
                                    .map(|api_msg| crate::models::Message {
                                        role: api_msg.role,
                                        content: api_msg.content,
                                        timestamp: 0, // We'll use current timestamp since created_at is a string
                                    })
                                    .collect();
                                
                                // TODO: store historical messages with thread model once
                                // the frontend chat rewrite lands.  Until then we ignore.
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

/// Helper function to save just the nodes to the API
#[allow(dead_code)]
fn save_nodes_to_api(nodes: &HashMap<String, Node>) -> Result<(), JsValue> {
    // Implementation to save nodes to API
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

// Save workflows to localStorage
pub fn save_workflows(app_state: &AppState) -> Result<(), JsValue> {
    let window = web_sys::window().expect("no global window exists");
    let local_storage = window.local_storage()?.expect("no local storage exists");
    
    // Convert the workflows to a JSON string
    let workflows_str = to_string(&app_state.workflows).map_err(|e| JsValue::from_str(&e.to_string()))?;
    
    // Save to localStorage
    local_storage.set_item("workflows", &workflows_str)?;
    
    // Also save the current workflow ID if it exists
    if let Some(workflow_id) = app_state.current_workflow_id {
        local_storage.set_item("current_workflow_id", &workflow_id.to_string())?;
    } else {
        local_storage.remove_item("current_workflow_id")?;
    }
    
    Ok(())
}

// Load workflows from localStorage
pub fn load_workflows(app_state: &mut AppState) -> Result<(), JsValue> {
    let window = web_sys::window().expect("no global window exists");
    let local_storage = window.local_storage()?.expect("no local storage exists");
    
    // Load workflows
    if let Some(workflows_str) = local_storage.get_item("workflows")? {
        match from_str::<HashMap<u32, Workflow>>(&workflows_str) {
            Ok(workflows) => {
                app_state.workflows = workflows;
            },
            Err(e) => {
                web_sys::console::error_1(&format!("Failed to parse workflows: {}", e).into());
                // Initialize with empty workflows
                app_state.workflows = HashMap::new();
            }
        }
    }
    
    // Load current workflow ID
    if let Some(workflow_id_str) = local_storage.get_item("current_workflow_id")? {
        if let Ok(workflow_id) = workflow_id_str.parse::<u32>() {
            app_state.current_workflow_id = Some(workflow_id);
            
            // Load the nodes from the current workflow into nodes
            if let Some(workflow) = app_state.workflows.get(&workflow_id) {
                app_state.nodes.clear();
                for node in &workflow.nodes {
                    app_state.nodes.insert(node.node_id.clone(), node.clone());
                }
            }
        }
    }
    
    Ok(())
} 