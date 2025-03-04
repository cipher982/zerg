use wasm_bindgen::prelude::*;
use web_sys::{WebSocket, MessageEvent};
use crate::state::APP_STATE;

pub fn setup_websocket() -> Result<(), JsValue> {
    // Create a new WebSocket connection
    let ws = WebSocket::new("ws://localhost:8001/ws")?;
    
    // Set up message event handler
    let onmessage_callback = Closure::wrap(Box::new(move |event: MessageEvent| {
        // Get the message data as string
        let response = event.data().as_string().unwrap();
        
        // Create a response node
        APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            
            // Get the latest node that was added (the user's input)
            if let Some(first_node_id) = state.nodes.keys().next().cloned() {
                state.add_response_node(&first_node_id, response);
                state.draw_nodes();
            }
        });
    }) as Box<dyn FnMut(_)>);
    
    ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    onmessage_callback.forget();
    
    // Set up error handler
    let onerror_callback = Closure::wrap(Box::new(move |_e: web_sys::Event| {
        web_sys::console::log_1(&"WebSocket error occurred".into());
    }) as Box<dyn FnMut(_)>);
    
    ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
    onerror_callback.forget();
    
    APP_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.websocket = Some(ws);
    });
    
    Ok(())
}

pub fn send_text_to_backend(text: &str) {
    // Use the Fetch API to send data to the backend
    let window = web_sys::window().expect("no global window exists");
    
    // Create headers
    let headers = web_sys::Headers::new().unwrap();
    headers.append("Content-Type", "application/json").unwrap();
    
    // Create request body
    let body_obj = js_sys::Object::new();
    js_sys::Reflect::set(&body_obj, &"text".into(), &text.into()).unwrap();
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