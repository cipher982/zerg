use wasm_bindgen::prelude::*;
use web_sys::{WebSocket, MessageEvent};
use crate::state::APP_STATE;
use wasm_bindgen_futures::JsFuture;
use js_sys::Array;

pub fn setup_websocket() -> Result<(), JsValue> {
    // Create a new WebSocket connection
    let ws = WebSocket::new("ws://localhost:8001/ws")?;
    
    // Set up message event handler
    let onmessage_callback = Closure::wrap(Box::new(move |event: MessageEvent| {
        // Get the message data as string
        let response_json = event.data().as_string().unwrap();
        
        // Parse the JSON response
        let response: JsValue = js_sys::JSON::parse(&response_json).unwrap_or_else(|_| {
            // If parsing fails, create a default object
            web_sys::console::log_1(&"Failed to parse response JSON".into());
            return JsValue::NULL;
        });
        
        // Create a response node
        APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            
            // Extract the response text and message ID
            let response_text = js_sys::Reflect::get(&response, &"response".into())
                .unwrap_or_else(|_| "No response text".into())
                .as_string()
                .unwrap_or_else(|| "No response text".to_string());
            
            let message_id = js_sys::Reflect::get(&response, &"message_id".into())
                .unwrap_or_else(|_| "".into())
                .as_string()
                .unwrap_or_else(|| "".to_string());
            
            // Find the corresponding node ID using the message ID
            if !message_id.is_empty() {
                if let Some(node_id) = state.get_node_id_for_message(&message_id) {
                    // Clone the node_id to avoid borrowing state immutably while using it mutably
                    let node_id_copy = node_id.clone();
                    state.add_response_node(&node_id_copy, response_text);
                    state.draw_nodes();
                    return;
                }
            }
            
            // Fallback to using the latest user input if we can't find a matching message ID
            if let Some(latest_input_id) = &state.latest_user_input_id {
                // Clone the ID to avoid borrowing state immutably while using it mutably
                let input_id_copy = latest_input_id.clone();
                state.add_response_node(&input_id_copy, response_text);
                state.draw_nodes();
            } else {
                web_sys::console::log_1(&"No user input node found to attach response".into());
            }
        });
    }) as Box<dyn FnMut(_)>);
    
    ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    onmessage_callback.forget();
    
    // Set up error handler
    let onerror_callback = Closure::wrap(Box::new(move |e: web_sys::Event| {
        // Log more details about the error
        web_sys::console::log_1(&"WebSocket error occurred:".into());
        web_sys::console::log_1(&e);
        
        // Optionally could implement reconnection logic here
        // For now just log the error
    }) as Box<dyn FnMut(_)>);
    
    ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
    onerror_callback.forget();
    
    // Set up close handler
    let onclose_callback = Closure::wrap(Box::new(move |e: web_sys::Event| {
        web_sys::console::log_1(&"WebSocket connection closed:".into());
        web_sys::console::log_1(&e);
    }) as Box<dyn FnMut(_)>);
    
    ws.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
    onclose_callback.forget();
    
    APP_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.websocket = Some(ws);
    });
    
    Ok(())
}

pub fn send_text_to_backend(text: &str) {
    // Generate a message ID
    let message_id = APP_STATE.with(|state| {
        let state = state.borrow();
        state.generate_message_id()
    });
    
    // Get the selected model
    let selected_model = APP_STATE.with(|state| {
        let state = state.borrow();
        state.selected_model.clone()
    });
    
    // Store the message ID mapping to the latest user input node
    APP_STATE.with(|state| {
        let mut state = state.borrow_mut();
        if let Some(latest_input_id) = &state.latest_user_input_id {
            // Clone the ID to avoid borrowing state immutably while using it mutably
            let latest_id_copy = latest_input_id.clone();
            state.track_message(message_id.clone(), latest_id_copy);
        }
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
    
    // We're not handling the response here since we'll get it via WebSocket
}

pub async fn fetch_available_models() -> Result<(), JsValue> {
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