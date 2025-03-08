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
    // Create a new WebSocket connection
    let ws = WebSocket::new("ws://localhost:8001/ws")?;
    
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

    // Track if we need to refresh UI after handling the message
    let need_refresh = {
        // Handle the message based on type - isolate the borrow in its own scope
        APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            
            if let Some(response_node_id) = state.get_node_id_for_message(&message_id) {
                match msg_type.as_str() {
                    "chunk" => {
                        // Extract the content from the chunk
                        let content = js_sys::Reflect::get(&response, &"content".into())
                            .unwrap_or_else(|_| "".into())
                            .as_string()
                            .unwrap_or_else(|| "".to_string());
                        
                        // Update the response node text
                        if let Some(node) = state.nodes.get_mut(&response_node_id) {
                            // If this is the first chunk, clear the placeholder
                            if node.text == "..." {
                                node.text = content;
                            } else {
                                node.text.push_str(&content);
                            }
                            
                            // Update node size based on new content
                            state.resize_node_for_content(&response_node_id);
                            state.draw_nodes();
                            
                            // Mark state as modified
                            state.state_modified = true;
                        } else {
                            web_sys::console::error_1(&format!("Node not found: {}", response_node_id).into());
                        }
                        
                        // Chunks don't require a full UI refresh
                        false
                    },
                    "completion" => {
                        // First extract all needed data from the immutable borrow
                        let (parent_id_to_update, response_text) = if let Some(node) = state.nodes.get(&response_node_id) {
                            if let Some(parent_id) = &node.parent_id {
                                (Some(parent_id.clone()), node.text.clone())
                            } else {
                                (None, String::new())
                            }
                        } else {
                            (None, String::new())
                        };
                        
                        // Now use the mutable borrow with the data we extracted
                        if let Some(parent_id) = parent_id_to_update {
                            if let Some(agent_node) = state.nodes.get_mut(&parent_id) {
                                // Set status back to idle
                                agent_node.status = Some("idle".to_string());
                                
                                // Create a new message for the history if it doesn't already exist
                                let assistant_message = crate::models::Message {
                                    role: "assistant".to_string(),
                                    content: response_text.clone(),
                                    timestamp: js_sys::Date::now() as u64,
                                };
                                
                                // Add to history if it exists
                                if let Some(history) = &mut agent_node.history {
                                    // Only add if this exact message isn't already there
                                    if !history.iter().any(|msg| 
                                        msg.role == "assistant" && msg.content == response_text
                                    ) {
                                        history.push(assistant_message);
                                    }
                                }
                            }
                        };
                        
                        state.draw_nodes();
                        
                        // Save state on completion
                        let _ = state.save_if_modified();
                        
                        // Return true to indicate we need to refresh the UI after this borrow ends
                        true
                    },
                    _ => {
                        // Handle legacy format
                        if let Ok(response_text) = js_sys::Reflect::get(&response, &"response".into()) {
                            if let Some(text) = response_text.as_string() {
                                if let Some(node) = state.nodes.get_mut(&response_node_id) {
                                    node.text = text;
                                    state.resize_node_for_content(&response_node_id);
                                    state.draw_nodes();
                                    
                                    // Mark state as modified
                                    state.state_modified = true;
                                }
                            }
                        }
                        
                        // Legacy format doesn't require UI refresh
                        false
                    }
                }
            } else {
                web_sys::console::error_1(&format!("No node found for message_id: {}", message_id).into());
                false
            }
        })
    }; // The borrow is completely dropped here
    
    // If we need to refresh UI after handling a completion message, do it in a separate borrow
    if need_refresh {
        // This happens in its own borrow scope
        let _ = crate::state::AppState::refresh_ui_after_state_change();
    }
}

pub fn send_text_to_backend(text: &str, message_id: String) {
    flash_activity(); // Flash on send
    
    // Get the selected model and system instructions
    let (selected_model, system_instructions) = APP_STATE.with(|state| {
        let state = state.borrow();
        let model = state.selected_model.clone();
        
        // Get system instructions from the selected agent
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
    
    // Use the Fetch API to send data to the backend
    let window = web_sys::window().expect("no global window exists");
    
    // Create headers
    let headers = web_sys::Headers::new().unwrap();
    headers.append("Content-Type", "application/json").unwrap();
    
    // Create request body with message ID and model
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
    
    // Convert body_string to JsValue
    let body_value: JsValue = body_string.into();
    opts.set_body(&body_value);
    
    // Create request
    let request = web_sys::Request::new_with_str_and_init(
        "http://localhost:8001/api/process-text", 
        &opts
    ).unwrap();
    
    // Send the fetch request
    let _ = window.fetch_with_request(&request);
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