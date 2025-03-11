use wasm_bindgen::prelude::*;
use web_sys::{WebSocket, MessageEvent};
use crate::state::APP_STATE;
use wasm_bindgen_futures::JsFuture;
use js_sys::Array;
use std::cell::RefCell;

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
        handle_websocket_message(event);
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

fn handle_websocket_message(event: MessageEvent) {
    // Get the message data as string
    let response_json = event.data().as_string().unwrap();
    
    // Parse the JSON response
    let response: JsValue = js_sys::JSON::parse(&response_json).unwrap_or_else(|_| {
        web_sys::console::error_1(&"Failed to parse response JSON".into());
        return JsValue::NULL;
    });

    // Extract message type and ID
    let msg_type = js_sys::Reflect::get(&response, &"type".into())
        .unwrap_or_else(|_| "".into())
        .as_string()
        .unwrap_or_else(|| "".to_string());
    
    let message_id = js_sys::Reflect::get(&response, &"message_id".into())
        .unwrap_or_else(|_| "".into())
        .as_string()
        .unwrap_or_else(|| "".to_string());

    // Indicate activity in the websocket
    flash_activity();
    
    // Handle different message types
    if msg_type.is_empty() {
        // Legacy message format without explicit type
        return;
    }
    
    APP_STATE.with(|state| {
        let mut state = state.borrow_mut();
        
        // Only try to find node associations for node-specific message types
        if msg_type == "chunk" || msg_type == "done" || msg_type == "message_id" {
            // These are messages that need to update specific nodes
            if !message_id.is_empty() {
                let response_node_id = state.get_node_id_for_message(&message_id);
                
                if let Some(response_node_id) = response_node_id {
                    // Handle messages that target a specific node
                    match msg_type.as_str() {
                        "chunk" => {
                            // Extract the content from the chunk
                            let content = js_sys::Reflect::get(&response, &"chunk".into())
                                .unwrap_or_else(|_| "".into())
                                .as_string()
                                .unwrap_or_else(|| "".to_string());
                            
                            // Determine if this is the first chunk or an update
                            let is_first_chunk = {
                                if let Some(node) = state.nodes.get(&response_node_id) {
                                    node.text == "..."
                                } else {
                                    false
                                }
                            };
                            
                            // Use dispatch instead of direct mutation
                            state.dispatch(crate::messages::Message::UpdateNodeText {
                                node_id: response_node_id.clone(),
                                text: content,
                                is_first_chunk,
                            });
                        },
                        "done" => {
                            // Get the response content
                            let content = if let Some(node) = state.nodes.get(&response_node_id) {
                                node.text.clone()
                            } else {
                                String::new()
                            };
                            
                            // Use dispatch for completion
                            state.dispatch(crate::messages::Message::CompleteNodeResponse {
                                node_id: response_node_id.clone(),
                                final_text: content,
                            });
                        },
                        // Add other message types as needed
                        _ => {}
                    }
                }
            }
        }
    });
    
    // Refresh UI after state changes if needed
    let _ = crate::state::AppState::refresh_ui_after_state_change();
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
                    agent.system_instructions.clone().unwrap_or_default()
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