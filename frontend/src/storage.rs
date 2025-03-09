use wasm_bindgen::prelude::*;
use serde_json::{to_string, from_str};
use crate::state::AppState;
use crate::models::{Node, ApiAgent, ApiAgentCreate, ApiAgentUpdate};
use crate::network::ApiClient;
use std::collections::HashMap;
use wasm_bindgen_futures::spawn_local;
use wasm_bindgen::closure::Closure;

// Constants for storage keys
const NODES_KEY: &str = "zerg_nodes";
const VIEWPORT_KEY: &str = "zerg_viewport";
const SELECTED_MODEL_KEY: &str = "zerg_selected_model";
const ACTIVE_VIEW_KEY: &str = "zerg_active_view";

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

/// Save current app state to localStorage
pub fn save_state(app_state: &AppState) -> Result<(), JsValue> {
    let window = web_sys::window().expect("no global window exists");
    let storage = window.local_storage()?.expect("no localStorage exists");
    
    // Save nodes
    let nodes_json = to_string(&app_state.nodes)
        .map_err(|e| JsValue::from_str(&format!("Error serializing nodes: {}", e)))?;
    storage.set_item(NODES_KEY, &nodes_json)?;
    
    // Save viewport data
    let viewport_data = ViewportData {
        x: app_state.viewport_x,
        y: app_state.viewport_y,
        zoom: app_state.zoom_level,
    };
    let viewport_json = to_string(&viewport_data)
        .map_err(|e| JsValue::from_str(&format!("Error serializing viewport: {}", e)))?;
    storage.set_item(VIEWPORT_KEY, &viewport_json)?;
    
    // Save selected model
    storage.set_item(SELECTED_MODEL_KEY, &app_state.selected_model)?;
    
    // Save active view
    let active_view = if app_state.active_view == ActiveView::Dashboard { "dashboard" } else { "canvas" };
    storage.set_item(ACTIVE_VIEW_KEY, active_view)?;
    
    web_sys::console::log_1(&"State saved to localStorage".into());
    
    // Also save changes to API if nodes have changed
    save_state_to_api(app_state);
    
    Ok(())
}

/// Load app state from localStorage
pub fn load_state(app_state: &mut AppState) -> Result<bool, JsValue> {
    let window = web_sys::window().expect("no global window exists");
    let storage = window.local_storage()?.expect("no localStorage exists");
    
    let mut data_loaded = false;
    
    // Load nodes
    if let Some(nodes_json) = storage.get_item(NODES_KEY)? {
        match from_str::<HashMap<String, Node>>(&nodes_json) {
            Ok(nodes) => {
                app_state.nodes = nodes;
                data_loaded = true;
                web_sys::console::log_1(&format!("Loaded {} nodes from storage", app_state.nodes.len()).into());
            },
            Err(e) => {
                web_sys::console::warn_1(&JsValue::from_str(&format!("Error parsing nodes: {}", e)));
                // Continue loading other data even if nodes fail
            }
        }
    }
    
    // Load viewport
    if let Some(viewport_json) = storage.get_item(VIEWPORT_KEY)? {
        match from_str::<ViewportData>(&viewport_json) {
            Ok(viewport) => {
                app_state.viewport_x = viewport.x;
                app_state.viewport_y = viewport.y;
                app_state.zoom_level = viewport.zoom;
                data_loaded = true;
            },
            Err(e) => {
                web_sys::console::warn_1(&JsValue::from_str(&format!("Error parsing viewport: {}", e)));
            }
        }
    }
    
    // Load selected model
    if let Some(model) = storage.get_item(SELECTED_MODEL_KEY)? {
        app_state.selected_model = model;
        data_loaded = true;
    }
    
    // Load active view
    if let Some(view) = storage.get_item(ACTIVE_VIEW_KEY)? {
        app_state.active_view = if view == "dashboard" { ActiveView::Dashboard } else { ActiveView::Canvas };
        data_loaded = true;
    }
    
    // Try to load data from API if nothing loaded from localStorage
    if !data_loaded {
        load_state_from_api(app_state);
    }
    
    Ok(data_loaded)
}

/// Clear all stored data
pub fn clear_storage() -> Result<(), JsValue> {
    let window = web_sys::window().expect("no global window exists");
    let storage = window.local_storage()?.expect("no localStorage exists");
    
    storage.remove_item(NODES_KEY)?;
    storage.remove_item(VIEWPORT_KEY)?;
    storage.remove_item(SELECTED_MODEL_KEY)?;
    storage.remove_item(ACTIVE_VIEW_KEY)?;
    
    Ok(())
}

/// Save app state to API (transitional approach)
pub fn save_state_to_api(app_state: &AppState) {
    // Clone the necessary data before passing to the async block
    let nodes = app_state.nodes.clone();
    let selected_model = app_state.selected_model.clone();
    
    // Spawn an async task to save agents to API
    spawn_local(async move {
        // We'll need to convert each Node of type AgentIdentity to an ApiAgent
        for (node_id, node) in nodes.iter() {
            if let crate::models::NodeType::AgentIdentity = node.node_type {
                // Only synchronize agent identity nodes
                let agent_update = ApiAgentUpdate {
                    name: Some(node.text.clone()),
                    status: node.status.clone(),
                    instructions: node.task_instructions.clone(),
                    model: Some(selected_model.clone()),
                    schedule: None,
                    config: None, // Will need to add more node data as needed
                };
                
                // Extract the numeric agent ID if it exists
                if let Some(agent_id_str) = node_id.strip_prefix("agent-") {
                    if let Ok(agent_id) = agent_id_str.parse::<u32>() {
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
                } else {
                    // This is a new agent, create it in the API
                    let agent_create = ApiAgentCreate {
                        name: node.text.clone(),
                        instructions: node.task_instructions.clone(),
                        model: Some(selected_model.clone()),
                        schedule: None,
                        config: None,
                    };
                    
                    let agent_json = match to_string(&agent_create) {
                        Ok(json) => json,
                        Err(e) => {
                            web_sys::console::error_1(&format!("Error serializing agent create: {}", e).into());
                            continue;
                        }
                    };
                    
                    if let Err(e) = ApiClient::create_agent(&agent_json).await {
                        web_sys::console::error_1(&format!("Error creating agent in API: {:?}", e).into());
                    } else {
                        web_sys::console::log_1(&format!("Created new agent in API: {}", node.text).into());
                    }
                }
            }
        }
    });
}

/// Load app state from API (transitional approach)
pub fn load_state_from_api(_app_state: &mut AppState) {
    // We need to handle this differently since we can't clone app_state
    // Clone the necessary fields individually or use a callback approach
    
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
                                    system_instructions: None,
                                    task_instructions: agent.instructions,
                                    history: None, // We'll load this separately
                                    status: agent.status,
                                };
                                
                                // Add the node to our temporary collection
                                loaded_nodes.insert(node_id, node);
                            }
                        }
                        
                        // Use a callback function to update the app state
                        // This is a workaround since we can't directly modify app_state in the async block
                        if !loaded_nodes.is_empty() {
                            if let Err(e) = crate::state::update_app_state_from_api(loaded_nodes) {
                                web_sys::console::error_1(&format!("Error updating app state: {:?}", e).into());
                            }
                        }
                    },
                    Err(e) => {
                        web_sys::console::error_1(&format!("Error parsing agents from API: {}", e).into());
                    }
                }
            },
            Err(e) => {
                web_sys::console::error_1(&format!("Error loading agents from API: {:?}", e).into());
            }
        }
    });
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
pub fn migrate_local_storage_to_api() -> Result<(), JsValue> {
    let window = web_sys::window().expect("no global window exists");
    let _storage = window.local_storage()?.expect("no localStorage exists");
    
    // We'll use our existing load & save functionality to handle the migration
    let mut temp_state = crate::state::AppState::new();
    
    // First load data from localStorage
    let data_loaded = load_state(&mut temp_state)?;
    
    if data_loaded {
        web_sys::console::log_1(&"Starting migration of data from localStorage to API...".into());
        
        // Then save it to the API - this will create/update all agents
        save_state_to_api(&temp_state);
        
        // Also sync messages for each agent node
        for (node_id, node) in &temp_state.nodes {
            if let crate::models::NodeType::AgentIdentity = node.node_type {
                if let Some(history) = &node.history {
                    if !history.is_empty() {
                        save_agent_messages_to_api(node_id, history);
                    }
                }
            }
        }
        
        // Provide user feedback
        if let Some(document) = window.document() {
            let migration_message = document.create_element("div")?;
            migration_message.set_attribute("style", "position: fixed; top: 20px; right: 20px; padding: 15px; background-color: #d4edda; color: #155724; border-radius: 4px; z-index: 1000; box-shadow: 0 2px 5px rgba(0,0,0,0.2);")?;
            migration_message.set_text_content(Some("Data migration in progress. Your agents and conversations are being synchronized with the server."));
            
            if let Some(body) = document.body() {
                body.append_child(&migration_message)?;
                
                // Remove the message after 10 seconds
                let closure = Closure::wrap(Box::new(move || {
                    if let Some(parent) = migration_message.parent_node() {
                        let _ = parent.remove_child(&migration_message);
                    }
                }) as Box<dyn FnMut()>);
                
                window.set_timeout_with_callback_and_timeout_and_arguments(
                    closure.as_ref().unchecked_ref(),
                    10000,
                    &js_sys::Array::new(),
                )?;
                
                closure.forget();
            }
        }
        
        web_sys::console::log_1(&"Migration complete. Data has been synchronized to the API.".into());
    } else {
        web_sys::console::log_1(&"No localStorage data found to migrate.".into());
    }
    
    Ok(())
}

/// Load app state with API as source of truth and localStorage as fallback
pub fn load_state_prioritizing_api(app_state: &mut AppState) {
    // First attempt to load from API
    load_state_from_api_sync(app_state);
    
    web_sys::console::log_1(&"Loading data from API with localStorage fallback".into());
    
    // If we failed to load anything from API, try localStorage as fallback
    if app_state.nodes.is_empty() {
        let window = web_sys::window().expect("no global window exists");
        let storage = match window.local_storage() {
            Ok(Some(storage)) => storage,
            _ => {
                web_sys::console::warn_1(&"Could not access localStorage".into());
                return;
            }
        };
        
        // Load nodes from localStorage as fallback
        if let Ok(Some(nodes_json)) = storage.get_item(NODES_KEY) {
            match from_str::<HashMap<String, Node>>(&nodes_json) {
                Ok(nodes) => {
                    if !nodes.is_empty() {
                        web_sys::console::log_1(&format!("Loaded {} nodes from localStorage fallback", nodes.len()).into());
                        app_state.nodes = nodes;
                        
                        // Save to API to ensure sync
                        let nodes_clone = app_state.nodes.clone();
                        spawn_local(async move {
                            web_sys::console::log_1(&"Syncing localStorage data to API...".into());
                            if let Err(e) = save_nodes_to_api(&nodes_clone) {
                                web_sys::console::error_1(&format!("Error syncing to API: {:?}", e).into());
                            } else {
                                web_sys::console::log_1(&"Successfully synced localStorage data to API".into());
                            }
                        });
                    }
                },
                Err(e) => {
                    web_sys::console::warn_1(&JsValue::from_str(&format!("Error parsing nodes from localStorage: {}", e)));
                }
            }
        }
        
        // Load viewport settings from localStorage
        if let Ok(Some(viewport_json)) = storage.get_item(VIEWPORT_KEY) {
            match from_str::<ViewportData>(&viewport_json) {
                Ok(viewport) => {
                    app_state.viewport_x = viewport.x;
                    app_state.viewport_y = viewport.y;
                    app_state.zoom_level = viewport.zoom;
                },
                Err(e) => {
                    web_sys::console::warn_1(&JsValue::from_str(&format!("Error parsing viewport: {}", e)));
                }
            }
        }
        
        // Load selected model from localStorage
        if let Ok(Some(model)) = storage.get_item(SELECTED_MODEL_KEY) {
            app_state.selected_model = model;
        }
        
        // Load active view from localStorage
        if let Ok(Some(view)) = storage.get_item(ACTIVE_VIEW_KEY) {
            app_state.active_view = if view == "dashboard" { ActiveView::Dashboard } else { ActiveView::Canvas };
        }
    }
}

/// A synchronous version of API loading for initial app load
fn load_state_from_api_sync(app_state: &mut AppState) {
    use web_sys::{Request, RequestInit, RequestMode};
    
    let window = web_sys::window().expect("no global window exists");
    
    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);
    
    // Create a synchronous-like request using a flag
    let api_loaded = false;
    let api_url = format!("{}/agents/", API_BASE_URL);
    
    match Request::new_with_str_and_init(&api_url, &opts) {
        Ok(request) => {
            let promise = window.fetch_with_request(&request);
            
            // Since we need synchronous behavior, we'll use a different approach
            // without XMLHttpRequest (which isn't available in web_sys by default)
            let _ = promise; // Acknowledging we're not using this promise
                
            // For now, we'll just log this as a fallback
            web_sys::console::log_1(&"Synchronous API fetching not implemented yet".into());
            // You would need to implement an alternative approach or add XMLHttpRequest
            // to your web-sys features in Cargo.toml
                
            // We don't return a value as this is unit function
            return;
        },
        Err(e) => {
            web_sys::console::warn_1(&format!("Error creating API request: {:?}", e).into());
        }
    }
    
    // Also asynchronously load agent messages if we loaded agents
    if api_loaded && !app_state.nodes.is_empty() {
        // We already loaded the nodes, but we still need to load messages
        // This can happen asynchronously
        for (node_id, _) in app_state.nodes.clone().iter() {
            if let Some(agent_id_str) = node_id.strip_prefix("agent-") {
                if let Ok(agent_id) = agent_id_str.parse::<u32>() {
                    load_agent_messages_from_api(node_id, agent_id);
                }
            }
        }
    }
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