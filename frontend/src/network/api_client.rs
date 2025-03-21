use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{RequestMode};
use super::ui_updates::flash_activity;
use crate::constants::{DEFAULT_NODE_WIDTH, DEFAULT_NODE_HEIGHT};

// REST API Client for Agent operations
pub struct ApiClient;

impl ApiClient {
    // Get the base URL for API calls
    fn api_base_url() -> String {
        // In a production environment, this might be read from configuration
        super::get_api_base_url().unwrap_or_else(|_| "http://localhost:8001".to_string())
    }

    // Get all agents
    pub async fn get_agents() -> Result<String, JsValue> {
        let url = format!("{}/api/agents", Self::api_base_url());
        Self::fetch_json(&url, "GET", None).await
    }

    // Get a specific agent by ID
    pub async fn get_agent(agent_id: u32) -> Result<String, JsValue> {
        let url = format!("{}/api/agents/{}", Self::api_base_url(), agent_id);
        Self::fetch_json(&url, "GET", None).await
    }

    // Create a new agent
    pub async fn create_agent(agent_data: &str) -> Result<String, JsValue> {
        let url = format!("{}/api/agents", Self::api_base_url());
        Self::fetch_json(&url, "POST", Some(agent_data)).await
    }

    // Update an existing agent
    pub async fn update_agent(agent_id: u32, agent_data: &str) -> Result<String, JsValue> {
        let url = format!("{}/api/agents/{}", Self::api_base_url(), agent_id);
        Self::fetch_json(&url, "PUT", Some(agent_data)).await
    }

    // Delete an agent
    pub async fn delete_agent(agent_id: u32) -> Result<(), JsValue> {
        let url = format!("{}/api/agents/{}", Self::api_base_url(), agent_id);
        let _ = Self::fetch_json(&url, "DELETE", None).await?;
        Ok(())
    }

    // Get messages for a specific agent
    pub async fn get_agent_messages(agent_id: u32) -> Result<String, JsValue> {
        let url = format!("{}/api/agents/{}/messages", Self::api_base_url(), agent_id);
        Self::fetch_json(&url, "GET", None).await
    }

    // Add a message to an agent
    pub async fn create_agent_message(agent_id: u32, message_data: &str) -> Result<String, JsValue> {
        let url = format!("{}/api/agents/{}/messages", Self::api_base_url(), agent_id);
        Self::fetch_json(&url, "POST", Some(message_data)).await
    }

    // Trigger an agent to run
    pub async fn run_agent(agent_id: u32) -> Result<String, JsValue> {
        let url = format!("{}/api/agents/{}/run", Self::api_base_url(), agent_id);
        Self::fetch_json(&url, "POST", None).await
    }

    // Reset the database (development only)
    pub async fn reset_database() -> Result<String, JsValue> {
        let url = format!("{}/api/reset-database", Self::api_base_url());
        Self::fetch_json(&url, "POST", None).await
    }

    // Thread management
    pub async fn get_threads(agent_id: Option<u32>) -> Result<String, JsValue> {
        let url = if let Some(id) = agent_id {
            format!("{}/api/threads?agent_id={}", Self::api_base_url(), id)
        } else {
            format!("{}/api/threads", Self::api_base_url())
        };
        Self::fetch_json(&url, "GET", None).await
    }

    pub async fn get_thread(thread_id: u32) -> Result<String, JsValue> {
        let url = format!("{}/api/threads/{}", Self::api_base_url(), thread_id);
        Self::fetch_json(&url, "GET", None).await
    }

    pub async fn create_thread(agent_id: u32, title: &str) -> Result<String, JsValue> {
        let url = format!("{}/api/threads", Self::api_base_url());
        let thread_data = format!("{{\"agent_id\": {}, \"title\": \"{}\", \"active\": true}}", agent_id, title);
        Self::fetch_json(&url, "POST", Some(&thread_data)).await
    }

    pub async fn update_thread(thread_id: u32, title: &str) -> Result<String, JsValue> {
        let url = format!("{}/api/threads/{}", Self::api_base_url(), thread_id);
        let thread_data = format!("{{\"title\": \"{}\"}}", title);
        Self::fetch_json(&url, "PUT", Some(&thread_data)).await
    }

    pub async fn delete_thread(thread_id: u32) -> Result<(), JsValue> {
        let url = format!("{}/api/threads/{}", Self::api_base_url(), thread_id);
        let _ = Self::fetch_json(&url, "DELETE", None).await?;
        Ok(())
    }

    // Thread messages
    pub async fn get_thread_messages(thread_id: u32, skip: u32, limit: u32) -> Result<String, JsValue> {
        let url = format!("{}/api/threads/{}/messages?skip={}&limit={}", 
                       Self::api_base_url(), thread_id, skip, limit);
        Self::fetch_json(&url, "GET", None).await
    }

    pub async fn create_thread_message(thread_id: u32, content: &str) -> Result<String, JsValue> {
        let url = format!("{}/api/threads/{}/messages", Self::api_base_url(), thread_id);
        let message_data = format!("{{\"role\": \"user\", \"content\": \"{}\"}}", content);
        Self::fetch_json(&url, "POST", Some(&message_data)).await
    }

    // Helper function to make fetch requests
    async fn fetch_json(url: &str, method: &str, body: Option<&str>) -> Result<String, JsValue> {
        use web_sys::{Request, RequestInit, RequestMode, Response};
        
        let opts = RequestInit::new();
        opts.set_method(method);
        opts.set_mode(RequestMode::Cors);
        
        // Add body if provided
        if let Some(data) = body {
            // Create the JsValue once and pass a reference to set_body
            let js_body = JsValue::from_str(data);
            opts.set_body(&js_body);
            
            // Create and set headers
            let headers = web_sys::Headers::new()?;
            headers.append("Content-Type", "application/json")?;
            opts.set_headers(&headers);
        }
        
        let request = Request::new_with_str_and_init(url, &opts)?;
        
        let window = web_sys::window().expect("no global window exists");
        let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
        let resp: Response = resp_value.dyn_into()?;
        
        // Check if successful
        if !resp.ok() {
            let status = resp.status();
            let status_text = resp.status_text();
            return Err(JsValue::from_str(&format!(
                "API request failed: {} {}", status, status_text
            )));
        }
        
        // Parse JSON
        let json = JsFuture::from(resp.text()?).await?;
        Ok(json.as_string().unwrap_or_default())
    }
}

// Load agents from API and update state.agents
pub fn load_agents() {
    flash_activity(); // Flash on API call
    
    let api_base_url = format!("{}/api/agents", ApiClient::api_base_url());
    
    // Call the API and update state
    wasm_bindgen_futures::spawn_local(async move {
        let window = web_sys::window().expect("no global window exists");
        let opts = web_sys::RequestInit::new();
        opts.set_method("GET");
        opts.set_mode(RequestMode::Cors);
        
        // Create new request
        match web_sys::Request::new_with_str_and_init(&api_base_url, &opts) {
            Ok(request) => {
                let promise = window.fetch_with_request(&request);
                
                match wasm_bindgen_futures::JsFuture::from(promise).await {
                    Ok(resp_value) => {
                        let response: web_sys::Response = resp_value.dyn_into().unwrap();
                        
                        if response.ok() {
                            match response.json() {
                                Ok(json_promise) => {
                                    match wasm_bindgen_futures::JsFuture::from(json_promise).await {
                                        Ok(json_value) => {
                                            let agents_data = json_value;
                                            match serde_wasm_bindgen::from_value::<Vec<crate::models::ApiAgent>>(agents_data) {
                                                Ok(agents) => {
                                                    web_sys::console::log_1(&format!("Loaded {} agents from API", agents.len()).into());
                                                    
                                                    // Update the agents HashMap in AppState
                                                    crate::state::APP_STATE.with(|state| {
                                                        let mut state = state.borrow_mut();
                                                        state.agents.clear();
                                                        
                                                        // Add each agent to the HashMap
                                                        for agent in agents {
                                                            if let Some(id) = agent.id {
                                                                state.agents.insert(id, agent);
                                                            }
                                                        }
                                                        
                                                        // Now that we have loaded agents, check if we need to create nodes for them
                                                        // Only create nodes for agents that don't already have one
                                                        create_nodes_for_agents(&mut state);
                                                        
                                                        state.data_loaded = true;
                                                        state.api_load_attempted = true;
                                                        state.is_loading = false;
                                                    });
                                                    
                                                    // Update the UI
                                                    if let Err(e) = crate::state::AppState::refresh_ui_after_state_change() {
                                                        web_sys::console::error_1(&format!("Error refreshing UI: {:?}", e).into());
                                                    }
                                                },
                                                Err(e) => {
                                                    web_sys::console::error_1(&format!("Failed to deserialize agents: {:?}", e).into());
                                                    mark_load_attempted();
                                                }
                                            }
                                        },
                                        Err(e) => {
                                            web_sys::console::error_1(&format!("Failed to parse response: {:?}", e).into());
                                            mark_load_attempted();
                                        }
                                    }
                                },
                                Err(e) => {
                                    web_sys::console::error_1(&format!("Failed to call json(): {:?}", e).into());
                                    mark_load_attempted();
                                }
                            }
                        } else {
                            web_sys::console::error_1(&format!("API request failed with status: {}", response.status()).into());
                            mark_load_attempted();
                        }
                    },
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to fetch agents: {:?}", e).into());
                        mark_load_attempted();
                    }
                }
            },
            Err(e) => {
                web_sys::console::error_1(&format!("Failed to create request: {:?}", e).into());
                mark_load_attempted();
            }
        }
    });
}

// Helper to mark API load as attempted but failed
fn mark_load_attempted() {
    crate::state::APP_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.api_load_attempted = true;
        state.is_loading = false;
    });
}

// Reload a specific agent by ID
pub fn reload_agent(agent_id: u32) {
    let api_url = ApiClient::api_base_url();
    let url = format!("{}/api/agents/{}", api_url, agent_id);
    
    wasm_bindgen_futures::spawn_local(async move {
        let window = web_sys::window().expect("no global window exists");
        let opts = web_sys::RequestInit::new();
        opts.set_method("GET");
        opts.set_mode(web_sys::RequestMode::Cors);
        
        match web_sys::Request::new_with_str_and_init(&url, &opts) {
            Ok(request) => {
                match JsFuture::from(window.fetch_with_request(&request)).await {
                    Ok(resp_value) => {
                        let response: web_sys::Response = resp_value.dyn_into().unwrap();
                        
                        if response.ok() {
                            match response.json() {
                                Ok(json_promise) => {
                                    match JsFuture::from(json_promise).await {
                                        Ok(json_value) => {
                                            let agent_data = json_value;
                                            match serde_wasm_bindgen::from_value::<crate::models::ApiAgent>(agent_data) {
                                                Ok(agent) => {
                                                    // Update the agent in the agents HashMap
                                                    crate::state::APP_STATE.with(|state| {
                                                        let mut state = state.borrow_mut();
                                                        
                                                        // Update agent in the HashMap
                                                        if let Some(id) = agent.id {
                                                            state.agents.insert(id, agent.clone());
                                                            
                                                            // Also update any nodes that reference this agent
                                                            for (_, node) in state.nodes.iter_mut() {
                                                                if node.agent_id == Some(id) {
                                                                    node.text = agent.name.clone();
                                                                }
                                                            }
                                                        }
                                                        
                                                        state.draw_nodes();
                                                    });
                                                    
                                                    // Update the UI
                                                    if let Err(e) = crate::state::AppState::refresh_ui_after_state_change() {
                                                        web_sys::console::error_1(&format!("Error refreshing UI: {:?}", e).into());
                                                    }
                                                },
                                                Err(e) => {
                                                    web_sys::console::error_1(&format!("Failed to deserialize agent: {:?}", e).into());
                                                }
                                            }
                                        },
                                        Err(e) => {
                                            web_sys::console::error_1(&format!("Failed to parse json: {:?}", e).into());
                                        }
                                    }
                                },
                                Err(e) => {
                                    web_sys::console::error_1(&format!("Failed to call json(): {:?}", e).into());
                                }
                            }
                        }
                    },
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to fetch: {:?}", e).into());
                    }
                }
            },
            Err(e) => {
                web_sys::console::error_1(&format!("Failed to create request: {:?}", e).into());
            }
        }
    });
}

// Create nodes for agents that don't already have one
fn create_nodes_for_agents(state: &mut crate::state::AppState) {
    // First, collect all the information we need without holding the borrow
    let agents_to_add: Vec<(u32, String)> = state.agents.iter()
        .filter(|(agent_id, _)| {
            // Check if there's already a node for this agent
            !state.nodes.iter().any(|(_, node)| {
                node.agent_id == Some(**agent_id)
            })
        })
        .map(|(agent_id, agent)| (*agent_id, agent.name.clone()))
        .collect();
    
    // Calculate grid layout
    let grid_size = (agents_to_add.len() as f64).sqrt().ceil() as usize;
    
    // Now add the nodes without conflicting borrows
    for (i, (agent_id, name)) in agents_to_add.into_iter().enumerate() {
        // Calculate a grid-like position for the new node
        let row = i / grid_size;
        let col = i % grid_size;
        
        let x = 100.0 + (col as f64 * (DEFAULT_NODE_WIDTH + 50.0));
        let y = 100.0 + (row as f64 * (DEFAULT_NODE_HEIGHT + 70.0));
        
        let node_id = state.add_node_with_agent(Some(agent_id), x, y, 
            crate::models::NodeType::AgentIdentity, name);
        
        web_sys::console::log_1(&format!("Created visual node with ID: {} for agent {}", node_id, agent_id).into());
    }
} 