use wasm_bindgen::prelude::*;
use web_sys::{WebSocket, MessageEvent};
use crate::state::APP_STATE;
use wasm_bindgen_futures::JsFuture;
use js_sys::Array;
use std::cell::RefCell;
use super::ui_updates::{update_connection_status, flash_activity};

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

pub fn schedule_reconnect() {
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
                        super::api_client::load_agents();
                        return Ok(());
                    },
                    Some("agent_updated") => {
                        web_sys::console::log_1(&"Received agent_updated event".into());
                        if let Some(agent_id_value) = json.get("agent_id") {
                            if let Some(agent_id) = agent_id_value.as_u64() {
                                let agent_id = agent_id as u32;
                                // Reload this specific agent - details moved to api_client
                                super::api_client::reload_agent(agent_id);
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
                    
                    // Handle thread-related events
                    Some("thread_created") => {
                        web_sys::console::log_1(&"Received thread_created event".into());
                        
                        // Extract thread ID and agent ID from the message
                        let thread_id = json.get("thread_id").and_then(|t| t.as_u64()).map(|t| t as u32);
                        let agent_id = json.get("agent_id").and_then(|a| a.as_u64()).map(|a| a as u32);
                        let title = json.get("title").and_then(|t| t.as_str()).unwrap_or("New Thread").to_string();
                        
                        if let (Some(thread_id), Some(agent_id)) = (thread_id, agent_id) {
                            // Create a scheduled operation to avoid nested borrowing
                            let cb = Closure::once(Box::new(move || {
                                // Create a thread object
                                let thread = crate::models::ApiThread {
                                    id: Some(thread_id),
                                    agent_id,
                                    title,
                                    active: true,
                                    created_at: Some(format!("{}", chrono::Utc::now())),
                                    updated_at: Some(format!("{}", chrono::Utc::now())),
                                };
                                
                                // Serialize the thread to JSON
                                if let Ok(thread_json) = serde_json::to_string(&thread) {
                                    // Dispatch as if the thread was created through the API
                                    crate::state::dispatch_global_message(
                                        crate::messages::Message::ThreadCreated(thread_json)
                                    );
                                } else {
                                    web_sys::console::error_1(&"Failed to serialize thread in thread_created event".into());
                                }
                            }) as Box<dyn FnOnce()>);
                            
                            // Schedule this to run outside of this message handler
                            web_sys::window()
                                .expect("no global window exists")
                                .set_timeout_with_callback_and_timeout_and_arguments_0(
                                    cb.as_ref().unchecked_ref(),
                                    0
                                )?;
                            cb.forget();
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
                    
                    // Don't dispatch directly while holding the state borrow
                    let message_id_clone = message_id.to_string();
                    let text_clone = text.to_string();
                    
                    // Use a two-step approach: first check if we need to update a node
                    let node_id_opt = crate::state::APP_STATE.with(|state| {
                        let state = state.borrow();
                        state.get_node_id_for_message(message_id_clone.as_str()).map(String::from)
                    });
                    
                    // If we have a node to update, schedule the update separately
                    if let Some(node_id) = node_id_opt {
                        // This dispatch happens outside of any borrowing
                        crate::state::dispatch_global_message(crate::messages::Message::UpdateNodeText {
                            node_id,
                            text: text_clone,
                            is_first_chunk,
                        });
                    }
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

pub fn setup_thread_websocket(thread_id: u32) -> Result<(), JsValue> {
    flash_activity(); // Flash activity indicator
    
    // Get the API base URL
    let protocol = if crate::network::get_api_base_url()?.starts_with("https") {
        "wss"
    } else {
        "ws"
    };
    
    let base_url = crate::network::get_api_base_url()?
        .replace("http://", "")
        .replace("https://", "");
    
    let ws_url = format!("{}://{}/api/threads/{}/ws", protocol, base_url, thread_id);
    
    web_sys::console::log_1(&format!("Connecting to WebSocket at {}", ws_url).into());
    
    // Create a new WebSocket connection
    let ws = WebSocket::new(&ws_url)?;
    
    // Create onopen handler
    let onopen_callback = Closure::wrap(Box::new(move |_| {
        web_sys::console::log_1(&"Thread WebSocket connection opened".into());
        update_connection_status("connected", "green");
    }) as Box<dyn FnMut(JsValue)>);
    ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
    onopen_callback.forget();
    
    // Create onmessage handler
    let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
        // Process incoming message
        if let Ok(text) = e.data().dyn_into::<js_sys::JsString>() {
            let message_str = String::from(text);
            
            web_sys::console::log_1(&format!("Thread WS received: {}", message_str).into());
            
            // Use a setTimeout to ensure we're not dispatching while inside another dispatch
            let message_str_clone = message_str.clone();
            let cb = Closure::once(Box::new(move || {
                // This dispatch happens outside of any borrowing
                crate::state::dispatch_global_message(crate::messages::Message::ThreadMessageReceived(message_str_clone));
            }) as Box<dyn FnOnce()>);
            
            web_sys::window()
                .expect("no global window exists")
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    cb.as_ref().unchecked_ref(),
                    0
                )
                .expect("failed to set timeout");
                
            cb.forget();
        }
    }) as Box<dyn FnMut(MessageEvent)>);
    ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    onmessage_callback.forget();
    
    // Create onclose handler
    let onclose_callback = Closure::wrap(Box::new(move |_| {
        web_sys::console::log_1(&"Thread WebSocket connection closed".into());
        update_connection_status("disconnected", "red");
    }) as Box<dyn FnMut(JsValue)>);
    ws.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
    onclose_callback.forget();
    
    // Create onerror handler
    let onerror_callback = Closure::wrap(Box::new(move |e: web_sys::ErrorEvent| {
        web_sys::console::error_1(&format!("Thread WebSocket error: {:?}", e).into());
        update_connection_status("error", "red");
    }) as Box<dyn FnMut(web_sys::ErrorEvent)>);
    ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
    onerror_callback.forget();
    
    // Store WebSocket instance and thread ID in the app state
    crate::state::APP_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.websocket = Some(ws);
        state.current_thread_id = Some(thread_id);
    });
    
    Ok(())
} 