use wasm_bindgen::prelude::*;
use web_sys::{WebSocket, MessageEvent};
use crate::state::APP_STATE;
use wasm_bindgen_futures::JsFuture;
use js_sys::Array;
use std::cell::RefCell;
use crate::constants::{
    DEFAULT_NODE_WIDTH,
    DEFAULT_NODE_HEIGHT,
    DEFAULT_AGENT_NODE_COLOR
};

// Track reconnection attempts
thread_local! {
    static RECONNECT_ATTEMPT: RefCell<u32> = RefCell::new(0);
}

// Track packet counter for activity indicator
thread_local! {
    static PACKET_COUNTER: RefCell<u32> = RefCell::new(0);
}

fn get_backoff_ms(attempt: u32) -> u32 {
    let base_delay = 1000; // Start with 1 second
    let max_delay = 30000; // Cap at 30 seconds
    let delay = base_delay * (2_u32.pow(attempt)); // Exponential backoff
    delay.min(max_delay)
}

fn schedule_reconnect() {
    let window = web_sys::window().expect("no global window exists");
    
    RECONNECT_ATTEMPT.with(|attempt| {
        let current = *attempt.borrow();
        let delay = get_backoff_ms(current);
        *attempt.borrow_mut() = current + 1;
        
        let reconnect_callback = Closure::wrap(Box::new(move || {
            if let Err(e) = setup_websocket() {
                web_sys::console::error_1(&format!("Reconnection failed: {:?}", e).into());
                schedule_reconnect(); // Try again if failed
            }
        }) as Box<dyn FnMut()>);

        window.set_timeout_with_callback_and_timeout_and_arguments(
            reconnect_callback.as_ref().unchecked_ref(),
            delay as i32,
            &Array::new(),
        ).expect("Failed to set timeout");
        
        reconnect_callback.forget();
    });
}

fn update_connection_status(status: &str, color: &str) {
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Some(status_element) = document.get_element_by_id("status") {
                status_element.set_class_name(color);
                status_element.set_inner_html(&format!("Status: {}", status));
            }
        }
    }
}

fn flash_activity() {
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Some(status_element) = document.get_element_by_id("api-status") {
                // Update packet counter
                PACKET_COUNTER.with(|counter| {
                    let count = *counter.borrow();
                    *counter.borrow_mut() = count.wrapping_add(1);
                    status_element.set_inner_html(&format!("PKT: {:08X}", count));
                });

                // Flash the LED
                status_element.set_class_name("flash");
                
                // Remove flash after 50ms
                let status_clone = status_element.clone();
                let clear_callback = Closure::wrap(Box::new(move || {
                    status_clone.set_class_name("");
                }) as Box<dyn FnMut()>);
                
                window.set_timeout_with_callback_and_timeout_and_arguments(
                    clear_callback.as_ref().unchecked_ref(),
                    50, // Very quick flash
                    &Array::new(),
                ).expect("Failed to set timeout");
                
                clear_callback.forget();
            }
        }
    }
}

pub fn setup_websocket() -> Result<(), JsValue> {
    // Get currently selected agent ID if available
    let agent_id = APP_STATE.with(|state| {
        let state = state.borrow();
        state.selected_node_id.clone()
    });
    
    // Create URL based on agent ID
    let ws_url = if let Some(id) = agent_id {
        format!("ws://localhost:8001/api/agents/{}/ws", id)
    } else {
        // Fall back to global endpoint if no agent is selected
        "ws://localhost:8001/ws".to_string()
    };
    
    // Create a new WebSocket connection
    let ws = WebSocket::new(&ws_url)?;
    
    // Set initial status
    update_connection_status("Connecting", "yellow");
    
    // Set up open handler
    let onopen_callback = Closure::wrap(Box::new(move |_: web_sys::Event| {
        update_connection_status("Connected", "green");
        flash_activity(); // Flash on connect
        // Reset reconnection counter on successful connection
        RECONNECT_ATTEMPT.with(|attempt| {
            *attempt.borrow_mut() = 0;
        });
    }) as Box<dyn FnMut(web_sys::Event)>);
    
    ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
    onopen_callback.forget();
    
    // Set up message event handler
    let onmessage_callback = Closure::wrap(Box::new(move |event: MessageEvent| {
        flash_activity(); // Flash on message
        let _ = handle_websocket_message(event);
    }) as Box<dyn FnMut(MessageEvent)>);
    
    ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    onmessage_callback.forget();
    
    // Set up error handler
    let onerror_callback = Closure::wrap(Box::new(move |e: web_sys::Event| {
        update_connection_status("Error", "red");
        web_sys::console::error_1(&format!("WebSocket error: {:?}", e).into());
    }) as Box<dyn FnMut(web_sys::Event)>);
    
    ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
    onerror_callback.forget();
    
    // Set up close handler
    let onclose_callback = Closure::wrap(Box::new(move |_: web_sys::Event| {
        update_connection_status("Disconnected", "red");
        schedule_reconnect();
    }) as Box<dyn FnMut(web_sys::Event)>);
    
    ws.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
    onclose_callback.forget();
    
    // Store WebSocket instance
    APP_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.websocket = Some(ws);
    });
    
    Ok(())
}

fn handle_websocket_message(event: web_sys::MessageEvent) -> Result<(), JsValue> {
    let data = event.data();
    let text = js_sys::JsString::from(data).as_string().unwrap_or_default();
    
    // Parse the JSON message
    match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(json) => {
            if let Some(message_type) = json.get("type") {
                match message_type.as_str() {
                    // Agent events
                    Some("agent_created") => {
                        web_sys::console::log_1(&"Received agent_created event".into());
                        // Refresh the agent list from the API
                        load_agents();
                        return Ok(());
                    },
                    Some("agent_updated") => {
                        web_sys::console::log_1(&"Received agent_updated event".into());
                        if let Some(agent_id_value) = json.get("agent_id") {
                            if let Some(agent_id) = agent_id_value.as_u64() {
                                let agent_id = agent_id as u32;
                                
                                // Reload this specific agent
                                if let Ok(api_url) = get_api_base_url() {
                                    let url = format!("{}/api/agents/{}", api_url, agent_id);
                                    
                                    let _request = web_sys::Request::new_with_str(&url)
                                        .expect("Failed to create request");
                                    
                                    // Set credentials mode to include cookies
                                    let opts = web_sys::RequestInit::new();
                                    opts.set_method("GET");
                                    opts.set_mode(web_sys::RequestMode::Cors);
                                    
                                    wasm_bindgen_futures::spawn_local(async move {
                                        let window = web_sys::window().expect("no global window exists");
                                        let opts = web_sys::RequestInit::new();
                                        opts.set_method("GET");
                                        opts.set_mode(web_sys::RequestMode::Cors);
                                        
                                        match web_sys::Request::new_with_str(&url) {
                                            Ok(request) => {
                                                let promise = window.fetch_with_request_and_init(&request, &opts);
                                                
                                                match wasm_bindgen_futures::JsFuture::from(promise).await {
                                                    Ok(resp_value) => {
                                                        let response: web_sys::Response = resp_value.dyn_into().unwrap();
                                                        
                                                        if response.ok() {
                                                            match response.json() {
                                                                Ok(json_promise) => {
                                                                    match wasm_bindgen_futures::JsFuture::from(json_promise).await {
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
                            }
                        }
                        return Ok(());
                    },
                    Some("agent_deleted") => {
                        web_sys::console::log_1(&"Received agent_deleted event".into());
                        if let Some(agent_id_value) = json.get("agent_id") {
                            if let Some(agent_id) = agent_id_value.as_u64() {
                                let agent_id = agent_id as u32;
                                
                                crate::state::APP_STATE.with(|state| {
                                    let mut state = state.borrow_mut();
                                    
                                    // Remove the agent from the HashMap
                                    state.agents.remove(&agent_id);
                                    
                                    // Remove any nodes that reference this agent
                                    let nodes_to_remove: Vec<String> = state.nodes.iter()
                                        .filter_map(|(node_id, node)| {
                                            if node.agent_id == Some(agent_id) {
                                                Some(node_id.clone())
                                            } else {
                                                None
                                            }
                                        })
                                        .collect();
                                    
                                    for node_id in nodes_to_remove {
                                        state.nodes.remove(&node_id);
                                    }
                                    
                                    // Also remove from any workflows
                                    for (_, workflow) in state.workflows.iter_mut() {
                                        workflow.nodes.retain(|node| node.agent_id != Some(agent_id));
                                    }
                                    
                                    state.draw_nodes();
                                });
                                
                                // Update the UI
                                if let Err(e) = crate::state::AppState::refresh_ui_after_state_change() {
                                    web_sys::console::error_1(&format!("Error refreshing UI: {:?}", e).into());
                                }
                            }
                        }
                        return Ok(());
                    },
                    Some("agent_status_changed") => {
                        if let Some(agent_id_value) = json.get("agent_id") {
                            if let Some(agent_id) = agent_id_value.as_u64() {
                                let agent_id = agent_id as u32;
                                
                                if let Some(status) = json.get("status").and_then(|s| s.as_str()) {
                                    web_sys::console::log_1(&format!("Agent {} status changed to {}", agent_id, status).into());
                                    
                                    crate::state::APP_STATE.with(|state| {
                                        let mut state = state.borrow_mut();
                                        
                                        // Update agent status in the HashMap
                                        if let Some(agent) = state.agents.get_mut(&agent_id) {
                                            agent.status = Some(status.to_string());
                                        }
                                        
                                        state.draw_nodes();
                                    });
                                    
                                    // Update the UI
                                    if let Err(e) = crate::state::AppState::refresh_ui_after_state_change() {
                                        web_sys::console::error_1(&format!("Error refreshing UI: {:?}", e).into());
                                    }
                                }
                            }
                        }
                        return Ok(());
                    },
                    Some("system_status") => {
                        web_sys::console::log_1(&"Received system_status event".into());
                        // Update connection status if message contains relevant info
                        if let Some(event) = json.get("event").and_then(|e| e.as_str()) {
                            if event == "connected" {
                                update_connection_status("Connected", "green");
                            }
                        }
                        return Ok(());
                    },
                    
                    // Handle other message types
                    Some(msg_type) => {
                        web_sys::console::log_1(&format!("Unhandled WebSocket message type: {}", msg_type).into());
                    },
                    None => {
                        web_sys::console::log_1(&"WebSocket message has no type".into());
                    }
                }
            }
            
            // Legacy handling for response streaming
            // This can be updated later to work with the new structure
            if let Some(message_id) = json.get("message_id").and_then(|m| m.as_str()) {
                if let Some(text) = json.get("text").and_then(|t| t.as_str()) {
                    let is_first_chunk = json.get("first_chunk").and_then(|f| f.as_bool()).unwrap_or(false);
                    
                    crate::state::APP_STATE.with(|state| {
                        let state = state.borrow();
                        if let Some(node_id) = state.get_node_id_for_message(message_id) {
                            // Dispatch a message to update the node text
                            crate::state::dispatch_global_message(crate::messages::Message::UpdateNodeText {
                                node_id,
                                text: text.to_string(),
                                is_first_chunk,
                            });
                        }
                    });
                }
            }
        },
        Err(e) => {
            web_sys::console::error_1(&format!("Failed to parse WebSocket message: {:?}", e).into());
        }
    }
    
    Ok(())
}

pub fn send_text_to_backend(text: &str, message_id: String) {
    flash_activity(); // Flash on send
    
    // Check if we have a selected agent and a websocket connection
    let has_agent_and_websocket = APP_STATE.with(|state| {
        let state = state.borrow();
        state.selected_node_id.is_some() && state.websocket.is_some()
    });
    
    if has_agent_and_websocket {
        // Use WebSocket for agent communication
        APP_STATE.with(|state| {
            let state = state.borrow();
            if let Some(ws) = &state.websocket {
                if ws.ready_state() == 1 { // OPEN
                    // Create message body
                    let body_obj = js_sys::Object::new();
                    js_sys::Reflect::set(&body_obj, &"text".into(), &text.into()).unwrap();
                    js_sys::Reflect::set(&body_obj, &"message_id".into(), &message_id.into()).unwrap();
                    
                    // Get selected model if any
                    if !state.selected_model.is_empty() {
                        js_sys::Reflect::set(&body_obj, &"model".into(), &state.selected_model.clone().into()).unwrap();
                    }
                    
                    let body_string = js_sys::JSON::stringify(&body_obj).unwrap();
                    
                    // Convert JsString to String before sending
                    if let Some(string_data) = body_string.as_string() {
                        // Send through WebSocket
                        let _ = ws.send_with_str(&string_data);
                    } else {
                        web_sys::console::error_1(&"Failed to convert JSON to string".into());
                    }
                } else {
                    // WebSocket not connected, try to reconnect
                    web_sys::console::warn_1(&"WebSocket not connected, reconnecting...".into());
                    let _ = setup_websocket();
                }
            }
        });
    } else {
        // Use the Fetch API for non-agent communication or as fallback
        let window = web_sys::window().expect("no global window exists");
        
        // Get the selected model and system instructions
        let (selected_model, system_instructions) = APP_STATE.with(|state| {
            let state = state.borrow();
            let model = state.selected_model.clone();
            
            // Get system instructions if any
            let instructions = if let Some(agent_id) = &state.selected_node_id {
                if let Some(agent) = state.nodes.get(agent_id) {
                    agent.system_instructions().clone().unwrap_or_default()
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
            
            (model, instructions)
        });
        
        // Create headers
        let headers = web_sys::Headers::new().unwrap();
        headers.append("Content-Type", "application/json").unwrap();
        
        // Create request body
        let body_obj = js_sys::Object::new();
        js_sys::Reflect::set(&body_obj, &"text".into(), &text.into()).unwrap();
        js_sys::Reflect::set(&body_obj, &"message_id".into(), &message_id.into()).unwrap();
        js_sys::Reflect::set(&body_obj, &"model".into(), &selected_model.into()).unwrap();
        js_sys::Reflect::set(&body_obj, &"system".into(), &system_instructions.into()).unwrap();
        let body_string = js_sys::JSON::stringify(&body_obj).unwrap();
        
        // Create request init object
        let opts = web_sys::RequestInit::new();
        opts.set_method("POST");
        opts.set_headers(&headers.into());
        opts.set_body(&body_string.into());
        
        // Create request
        let request = web_sys::Request::new_with_str_and_init(
            "http://localhost:8001/api/process-text", 
            &opts
        ).unwrap();
        
        // Send the fetch request
        let _ = window.fetch_with_request(&request);
    }
}

pub async fn fetch_available_models() -> Result<(), JsValue> {
    flash_activity(); // Flash when fetching models
    
    let window = web_sys::window().expect("no global window exists");
    
    // Create request
    let request = web_sys::Request::new_with_str(
        "http://localhost:8001/api/models"
    )?;
    
    // Fetch models
    let response_promise = window.fetch_with_request(&request);
    let response = JsFuture::from(response_promise).await?;
    
    // Convert to Response object
    let response: web_sys::Response = response.dyn_into()?;
    
    // Get JSON
    let json_promise = response.json()?;
    let json = JsFuture::from(json_promise).await?;
    
    // Parse models from response
    if let Some(models_value) = js_sys::Reflect::get(&json, &"models".into()).ok() {
        if let Some(models_array) = models_value.dyn_ref::<Array>() {
            // Clear existing models
            APP_STATE.with(|state| {
                let mut state = state.borrow_mut();
                state.available_models.clear();
                
                // Add models from the response
                for i in 0..models_array.length() {
                    if let Some(model) = models_array.get(i).dyn_ref::<js_sys::Object>() {
                        let id = js_sys::Reflect::get(&model, &"id".into())
                            .ok()
                            .and_then(|v| v.as_string())
                            .unwrap_or_default();
                            
                        let name = js_sys::Reflect::get(&model, &"name".into())
                            .ok()
                            .and_then(|v| v.as_string())
                            .unwrap_or_default();
                            
                        if !id.is_empty() && !name.is_empty() {
                            state.available_models.push((id, name));
                        }
                    }
                }
            });
        }
    }
    
    Ok(())
}

// REST API Client for Agent operations
pub struct ApiClient;

impl ApiClient {
    // Base URL for API calls
    fn api_base_url() -> String {
        // In a production environment, this might be read from configuration
        // For development, we'll use the standard FastAPI port
        "http://localhost:8001".to_string()
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
    let _window = web_sys::window().expect("no global window exists");
    let api_base_url = format!("{}/api/agents", get_api_base_url().unwrap_or_else(|_| "http://localhost:8001".to_string()));
    
    // Call the API and update state
    wasm_bindgen_futures::spawn_local(async move {
        let window = web_sys::window().expect("no global window exists");
        let opts = web_sys::RequestInit::new();
        
        // Create new request
        let _request = web_sys::Request::new_with_str(&api_base_url)
            .expect("Failed to create request");
        
        let promise = window.fetch_with_request_and_init(&_request, &opts);
        
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
                                            
                                            // Mark as attempted but failed
                                            crate::state::APP_STATE.with(|state| {
                                                let mut state = state.borrow_mut();
                                                state.api_load_attempted = true;
                                                state.is_loading = false;
                                            });
                                        }
                                    }
                                },
                                Err(e) => {
                                    web_sys::console::error_1(&format!("Failed to parse response: {:?}", e).into());
                                    
                                    // Mark as attempted but failed
                                    crate::state::APP_STATE.with(|state| {
                                        let mut state = state.borrow_mut();
                                        state.api_load_attempted = true;
                                        state.is_loading = false;
                                    });
                                }
                            }
                        },
                        Err(e) => {
                            web_sys::console::error_1(&format!("Failed to call json(): {:?}", e).into());
                            
                            // Mark as attempted but failed
                            crate::state::APP_STATE.with(|state| {
                                let mut state = state.borrow_mut();
                                state.api_load_attempted = true;
                                state.is_loading = false;
                            });
                        }
                    }
                } else {
                    web_sys::console::error_1(&format!("API request failed with status: {}", response.status()).into());
                    
                    // Mark as attempted but failed
                    crate::state::APP_STATE.with(|state| {
                        let mut state = state.borrow_mut();
                        state.api_load_attempted = true;
                        state.is_loading = false;
                    });
                }
            },
            Err(e) => {
                web_sys::console::error_1(&format!("Failed to fetch agents: {:?}", e).into());
                
                // Mark as attempted but failed
                crate::state::APP_STATE.with(|state| {
                    let mut state = state.borrow_mut();
                    state.api_load_attempted = true;
                    state.is_loading = false;
                });
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

// Fix the get_api_base_url function
pub fn get_api_base_url() -> Result<String, JsValue> {
    let window = web_sys::window().expect("no global window exists");
    let location = window.location();
    
    // If we're in local development, use a fixed port of 8001
    let hostname = location.hostname()?;
    if hostname == "localhost" || hostname == "127.0.0.1" {
        Ok("http://localhost:8001".to_string())
    } else {
        // For production, use same hostname with :8001
        let protocol = location.protocol()?;
        Ok(format!("{}//{}:8001", protocol, hostname))
    }
} 