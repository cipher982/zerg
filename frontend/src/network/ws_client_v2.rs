use wasm_bindgen::prelude::*;
use web_sys::{WebSocket, MessageEvent};
use std::cell::RefCell;
use std::rc::Rc;
use js_sys::Array;
use serde_json::Value;
use std::any::Any;

use super::messages::builders;

/// Trait defining the WebSocket client interface
#[allow(dead_code)]
pub trait IWsClient: Any {
    fn connect(&mut self) -> Result<(), JsValue>;
    fn send_serialized_message(&self, message_json: &str) -> Result<(), JsValue>;
    fn connection_state(&self) -> ConnectionState;
    fn close(&mut self) -> Result<(), JsValue>;
    fn set_on_connect(&mut self, callback: Box<dyn FnMut() + 'static>);
    fn set_on_message(&mut self, callback: Box<dyn FnMut(Value) + 'static>);
    fn set_on_disconnect(&mut self, callback: Box<dyn FnMut() + 'static>);
    fn as_any(&self) -> &dyn Any;
}

/// Represents the current state of the WebSocket connection
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    #[allow(dead_code)]
    Error(String),
}

impl std::fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionState::Disconnected => write!(f, "Disconnected"),
            ConnectionState::Connecting => write!(f, "Connecting"),
            ConnectionState::Connected => write!(f, "Connected"),
            ConnectionState::Error(msg) => write!(f, "Error: {}", msg),
        }
    }
}

/// Configuration for the WebSocket client
#[derive(Debug, Clone)]
pub struct WsConfig {
    /// Base URL for the WebSocket connection
    pub url: String,
    /// Maximum number of reconnection attempts (None for infinite)
    pub max_reconnect_attempts: Option<u32>,
    /// Initial backoff delay in milliseconds
    pub initial_backoff_ms: u32,
    /// Maximum backoff delay in milliseconds
    pub max_backoff_ms: u32,
    /// Ping interval in milliseconds (None to disable)
    pub ping_interval_ms: Option<u32>,
}

impl Default for WsConfig {
    fn default() -> Self {
        let url = super::get_ws_url().unwrap_or_else(|_| {
            // Provide a sane fallback during unit tests where API config may
            // not be initialised.  At runtime `init_api_config()` should have
            // been called long before we reach this code.
            "ws://localhost/placeholder".to_string()
        });

        Self {
            url,
            max_reconnect_attempts: None,
            initial_backoff_ms: 1000,
            max_backoff_ms: 30000,
            ping_interval_ms: Some(30000), // 30-second ping interval
        }
    }
}

/// Type for the on_connect callback
type OnConnectCallback = Rc<RefCell<dyn FnMut()>>;
/// Type for the on_message callback (receives parsed JSON value)
type OnMessageCallback = Rc<RefCell<dyn FnMut(Value)>>;
/// Type for the on_disconnect callback
type OnDisconnectCallback = Rc<RefCell<dyn FnMut()>>;

/// Core WebSocket client implementation
pub struct WsClientV2 {
    config: WsConfig,
    websocket: Option<WebSocket>,
    state: Rc<RefCell<ConnectionState>>,
    reconnect_attempt: Rc<RefCell<u32>>,
    ping_interval: Option<i32>, // Store interval ID for cleanup

    // --- NEW Callbacks ---
    on_connect_callback: Option<OnConnectCallback>,
    on_message_callback: Option<OnMessageCallback>,
    on_disconnect_callback: Option<OnDisconnectCallback>,
}

impl WsClientV2 {
    /// Create a new WebSocket client with the given configuration
    pub fn new(config: WsConfig) -> Self {
        Self {
            config,
            websocket: None,
            state: Rc::new(RefCell::new(ConnectionState::Disconnected)),
            reconnect_attempt: Rc::new(RefCell::new(0)),
            ping_interval: None,
            // Initialize callbacks as None
            on_connect_callback: None,
            on_message_callback: None,
            on_disconnect_callback: None,
        }
    }

    /// Create a new WebSocket client with default configuration
    pub fn new_default() -> Self {
        Self::new(WsConfig::default())
    }

    /// Set a handler to be called upon successful connection/reconnection.
    pub fn set_on_connect<F>(&mut self, callback: F)
    where
        F: FnMut() + 'static,
    {
        self.on_connect_callback = Some(Rc::new(RefCell::new(Box::new(callback))));
    }

    /// Set a handler for incoming messages.
    /// The handler receives the parsed `serde_json::Value`.
    pub fn set_on_message<F>(&mut self, callback: F)
    where
        F: FnMut(Value) + 'static,
    {
        self.on_message_callback = Some(Rc::new(RefCell::new(Box::new(callback))));
    }

    /// Set a handler to be called when the connection is closed or fails to reconnect.
    pub fn set_on_disconnect<F>(&mut self, callback: F)
    where
        F: FnMut() + 'static,
    {
        self.on_disconnect_callback = Some(Rc::new(RefCell::new(Box::new(callback))));
    }

    /// Get the current connection state
    pub fn connection_state(&self) -> ConnectionState {
        self.state.borrow().clone()
    }

    /// Calculate the backoff delay for reconnection attempts
    fn get_backoff_ms(&self) -> u32 {
        let attempt = *self.reconnect_attempt.borrow();
        let base_delay = self.config.initial_backoff_ms;
        let max_delay = self.config.max_backoff_ms;
        let delay = base_delay * (2_u32.pow(attempt.min(10))); // Prevent overflow with min(10)
        delay.min(max_delay)
    }

    /// Set up ping interval
    fn setup_ping_interval(&mut self) {
        if let Some(interval_ms) = self.config.ping_interval_ms {
            let window = web_sys::window().expect("no global window exists");
            let ws_clone = self.websocket.clone();
            
            let ping_callback = Closure::wrap(Box::new(move || {
                if let Some(ws) = &ws_clone {
                    let ping = builders::create_ping();
                    if let Ok(json) = serde_json::to_string(&ping) {
                        if let Err(e) = ws.send_with_str(&json) {
                            web_sys::console::error_1(&format!("Failed to send ping: {:?}", e).into());
                        }
                    }
                }
            }) as Box<dyn FnMut()>);

            let interval_id = window
                .set_interval_with_callback_and_timeout_and_arguments(
                    ping_callback.as_ref().unchecked_ref(),
                    interval_ms as i32,
                    &Array::new(),
                )
                .expect("Failed to set ping interval");

            ping_callback.forget();
            self.ping_interval = Some(interval_id);
        }
    }

    /// Clear ping interval
    fn clear_ping_interval(&mut self) {
        if let Some(interval_id) = self.ping_interval.take() {
            if let Some(window) = web_sys::window() {
                window.clear_interval_with_handle(interval_id);
            }
        }
    }

    /// Creates the actual WebSocket connection and attaches handlers.
    /// This is used by both `connect` and `schedule_reconnect`.
    fn establish_connection(&mut self) -> Result<WebSocket, JsValue> {
        // Create WebSocket connection
        let ws = WebSocket::new(&self.config.url)?;

        // Clone necessary parts for closures
        let state_clone = self.state.clone();
        let reconnect_attempt_clone = self.reconnect_attempt.clone();
        let on_connect_cb_clone = self.on_connect_callback.clone();
        let on_message_cb_clone = self.on_message_callback.clone();
        let on_disconnect_cb_clone = self.on_disconnect_callback.clone();
        let client_clone_for_reconnect = self.clone(); // Clone for schedule_reconnect call
        let config_clone = self.config.clone();

        // Set up open handler
        let onopen_closure = Closure::wrap(Box::new(move |_: web_sys::Event| {
            web_sys::console::log_1(&"WebSocket connected".into());
            *state_clone.borrow_mut() = ConnectionState::Connected;
            *reconnect_attempt_clone.borrow_mut() = 0; // Reset counter

            // --- Call on_connect callback --- 
            if let Some(callback_rc) = &on_connect_cb_clone {
                if let Ok(mut callback) = callback_rc.try_borrow_mut() {
                    (*callback)();
                } else {
                    web_sys::console::error_1(&"Failed to borrow on_connect callback".into());
                }
            }
        }) as Box<dyn FnMut(web_sys::Event)>);
        ws.set_onopen(Some(onopen_closure.as_ref().unchecked_ref()));
        onopen_closure.forget();

        // Set up error handler (simple logging for now)
        let onerror_closure = Closure::wrap(Box::new(move |e: web_sys::Event| {
            web_sys::console::error_1(&format!("WebSocket error: {:?}", e).into());
            // Error often leads to close, state change handled in onclose
        }) as Box<dyn FnMut(web_sys::Event)>);
        ws.set_onerror(Some(onerror_closure.as_ref().unchecked_ref()));
        onerror_closure.forget();

        // Set up close handler (handles disconnect and triggers reconnect)
        let state_clone = self.state.clone();
        let reconnect_attempt_clone = self.reconnect_attempt.clone();
        let on_disconnect_cb_clone_for_close = on_disconnect_cb_clone.clone(); // Clone again for this closure
        let onclose_closure = Closure::wrap(Box::new(move |evt: web_sys::Event| {
            // Attempt to downcast to CloseEvent so we can inspect the code.
            if let Ok(close_evt) = evt.dyn_into::<web_sys::CloseEvent>() {
                let code = close_evt.code();
                if code == 4401 || code == 4003 {
                    // 4401: unauthenticated, 4003: forbidden – logout.
                    let _ = crate::utils::logout();
                }
            }

            web_sys::console::log_1(&"WebSocket closed".into());
            *state_clone.borrow_mut() = ConnectionState::Disconnected;

            // --- Call on_disconnect callback --- 
            if let Some(callback_rc) = &on_disconnect_cb_clone_for_close {
                 if let Ok(mut callback) = callback_rc.try_borrow_mut() {
                    (*callback)();
                } else {
                    web_sys::console::error_1(&"Failed to borrow on_disconnect callback".into());
                }
            }

            // Check if reconnection should be attempted
            let current_attempt = *reconnect_attempt_clone.borrow();
            if config_clone.max_reconnect_attempts.map_or(true, |max| current_attempt < max) {
                *reconnect_attempt_clone.borrow_mut() = current_attempt + 1;
                // Use the client clone to call schedule_reconnect
                client_clone_for_reconnect.schedule_reconnect();
            } else {
                web_sys::console::log_1(&"Max reconnection attempts reached".into());
                // Potentially update state to a permanent error state if needed
            }
        }) as Box<dyn FnMut(web_sys::Event)>);
        ws.set_onclose(Some(onclose_closure.as_ref().unchecked_ref()));
        onclose_closure.forget();

        // Set up message handler: parse JSON once, handle control frames, then forward to topic manager
        let onmessage_closure = Closure::wrap(Box::new(move |event: MessageEvent| {
            // Only handle text messages
            if let Ok(text) = event.data().dyn_into::<js_sys::JsString>() {
                if let Some(msg_str) = text.as_string() {
                    match serde_json::from_str::<Value>(&msg_str) {
                        Ok(parsed_value) => {
                            // Inspect the message type field
                            let msg_type = parsed_value
                                .get("type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");

                            match msg_type {
                                "pong" => {
                                    // Keep-alive frame, ignore
                                    return;
                                }
                                "error" => {
                                    web_sys::console::error_1(
                                        &format!("WebSocket error: {:?}", parsed_value).into()
                                    );
                                    return;
                                }
                                _ => {
                                    // Forward all other messages to topic manager via callback
                                    if let Some(callback_rc) = &on_message_cb_clone {
                                        if let Ok(mut callback) = callback_rc.try_borrow_mut() {
                                            (*callback)(parsed_value);
                                        } else {
                                            web_sys::console::error_1(
                                                &"Failed to borrow on_message callback".into()
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            web_sys::console::error_1(
                                &format!(
                                    "Failed to parse incoming WebSocket message as JSON: {}, Error: {:?}",
                                    msg_str, e
                                )
                                .into()
                            );
                        }
                    }
                }
            } else {
                web_sys::console::warn_1(&"Received non-text WebSocket message".into());
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        ws.set_onmessage(Some(onmessage_closure.as_ref().unchecked_ref()));
        onmessage_closure.forget();

        Ok(ws)
    }

    /// Schedule a reconnection attempt with exponential backoff
    fn schedule_reconnect(&self) {
        let window = web_sys::window().expect("no global window exists");
        let state_clone = self.state.clone();
        let delay = self.get_backoff_ms();
        let mut client_clone = self.clone(); // Clone self to move into the closure

        // Create reconnection callback
        let reconnect_callback = Closure::once(Box::new(move || {
            // Only attempt reconnection if we're still disconnected
            if *state_clone.borrow() == ConnectionState::Disconnected {
                web_sys::console::log_1(&format!("Attempting reconnection (attempt {})", *client_clone.reconnect_attempt.borrow()).into());
                
                // Update state to connecting before attempting
                *state_clone.borrow_mut() = ConnectionState::Connecting;

                // Attempt to establish connection
                 match client_clone.establish_connection() {
                    Ok(ws) => {
                        // Connection initiated, store the WebSocket instance
                        client_clone.websocket = Some(ws);
                        // Setup ping interval for the new connection
                        client_clone.setup_ping_interval();
                    }
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to create WebSocket during reconnect: {:?}", e).into());
                        // Connection failed, state might be reset by onclose, or manually trigger schedule_reconnect again if necessary
                         *state_clone.borrow_mut() = ConnectionState::Disconnected; // Reset state
                         // Schedule again immediately if creation failed (could lead to tight loop, consider delay)
                         client_clone.schedule_reconnect(); 
                    }
                }
            }
        }) as Box<dyn FnOnce()>);

        // Schedule the reconnection attempt
        window
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                reconnect_callback.as_ref().unchecked_ref(),
                delay as i32,
            )
            .expect("Failed to schedule reconnection");

        // Closure::once requires forget
        reconnect_callback.forget(); 
    }

    /// Connect to the WebSocket server for the first time.
    pub fn connect(&mut self) -> Result<(), JsValue> {
        web_sys::console::log_1(&"Initiating WebSocket connection...".into());
        // Reset reconnection counter when manually connecting
        *self.reconnect_attempt.borrow_mut() = 0;
        
        // Clear any previous interval
        self.clear_ping_interval();

        // Update state to connecting
        *self.state.borrow_mut() = ConnectionState::Connecting;

        // Establish the connection and set up handlers
        let ws = self.establish_connection()?;

        // Store WebSocket instance
        self.websocket = Some(ws);

        // Set up ping interval for the new connection
        self.setup_ping_interval();

        Ok(())
    }

    /// Send a pre-serialized message to the WebSocket server
    pub fn send_serialized_message(&self, message_json: &str) -> Result<(), JsValue> {
        if let Some(ws) = &self.websocket {
            if *self.state.borrow() == ConnectionState::Connected {
                ws.send_with_str(message_json)?;
                Ok(())
            } else {
                web_sys::console::warn_1(&"Attempted to send message while WebSocket is not connected".into());
                Err(JsValue::from_str("WebSocket is not connected"))
            }
        } else {
            Err(JsValue::from_str("WebSocket is not initialized"))
        }
    }

    /// Close the WebSocket connection gracefully.
    pub fn close(&mut self) -> Result<(), JsValue> {
        web_sys::console::log_1(&"Closing WebSocket connection...".into());
        // Clear ping interval
        self.clear_ping_interval();

        // Set state to disconnected *before* closing
        *self.state.borrow_mut() = ConnectionState::Disconnected;

        if let Some(ws) = self.websocket.take() { // Use take to remove it
             match ws.close() {
                Ok(_) => web_sys::console::log_1(&"WebSocket close command sent.".into()),
                Err(e) => web_sys::console::error_1(&format!("Error sending close command: {:?}", e).into()),
            }
        }
        // Note: The onclose handler will likely fire after this, potentially calling on_disconnect again.
        Ok(())
    }
}

// Implement IWsClient trait for WsClientV2
impl IWsClient for WsClientV2 {
    fn connect(&mut self) -> Result<(), JsValue> {
        self.connect()
    }

    fn send_serialized_message(&self, message_json: &str) -> Result<(), JsValue> {
        self.send_serialized_message(message_json)
    }

    fn connection_state(&self) -> ConnectionState {
        self.connection_state()
    }

    fn close(&mut self) -> Result<(), JsValue> {
        self.close()
    }

    fn set_on_connect(&mut self, callback: Box<dyn FnMut() + 'static>) {
        self.set_on_connect(callback);
    }

    fn set_on_message(&mut self, callback: Box<dyn FnMut(Value) + 'static>) {
        self.set_on_message(callback);
    }

    fn set_on_disconnect(&mut self, callback: Box<dyn FnMut() + 'static>) {
        self.set_on_disconnect(callback);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// --- Clone Implementation (Needs update) ---

impl Clone for WsClientV2 {
    fn clone(&self) -> Self {
        // Cloning is tricky with callbacks and WebSocket instance.
        // Typically used for passing into closures for reconnect logic.
        // We clone the Rc's for state/config but NOT the WebSocket instance or interval ID.
        // Callbacks are also NOT cloned here; they are captured by the closures where needed.
        Self {
            config: self.config.clone(),
            websocket: None, // Never clone the actual WebSocket
            state: self.state.clone(),
            reconnect_attempt: self.reconnect_attempt.clone(),
            ping_interval: None, // Don't clone interval ID
            // Callbacks are specifically NOT cloned here. They are managed by Rc.
            on_connect_callback: self.on_connect_callback.clone(),
            on_message_callback: self.on_message_callback.clone(), 
            on_disconnect_callback: self.on_disconnect_callback.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_client_creation() {
        let client = WsClientV2::new_default();
        assert_eq!(client.connection_state(), ConnectionState::Disconnected);
    }

    #[wasm_bindgen_test]
    fn test_backoff_calculation() {
        let client = WsClientV2::new_default();
        
        // First attempt should be initial_backoff_ms
        assert_eq!(*client.reconnect_attempt.borrow(), 0);
        assert_eq!(client.get_backoff_ms(), 1000);

        // Second attempt should be 2 * initial_backoff_ms
        *client.reconnect_attempt.borrow_mut() = 1;
        assert_eq!(client.get_backoff_ms(), 2000);

        // Should not exceed max_backoff_ms
        *client.reconnect_attempt.borrow_mut() = 10;
        assert_eq!(client.get_backoff_ms(), 30000);
    }

    #[wasm_bindgen_test]
    fn test_message_handler() {
        let mut client = WsClientV2::new_default();
        let received_messages = Rc::new(RefCell::new(Vec::new()));
        let received_clone = received_messages.clone();
        
        client.set_on_message(move |msg| {
            // Store the message type from the JSON if it exists
            if let Some(msg_type) = msg.get("type").and_then(|t| t.as_str()) {
                received_clone.borrow_mut().push(msg_type.to_string());
            }
        });
        
        // TODO: Add more message handler tests when we implement the full
        // message processing system
    }
}

// Helper function to send a message to a thread via WebSocket
#[allow(dead_code)]
pub fn send_thread_message(text: &str, message_id: String) {
    web_sys::console::log_1(&format!("Network: Sending thread message: '{}', message_id: {}", text, message_id).into());

    super::ui_updates::flash_activity(); // Flash on send
    
    // Check if we have a selected agent and a websocket connection
    let has_agent_and_websocket = crate::state::APP_STATE.with(|state| {
        let state = state.borrow();
        state.selected_node_id.is_some() && state.websocket.is_some()
    });
    
    if has_agent_and_websocket {
        // Use WebSocket for agent communication
        crate::state::APP_STATE.with(|state| {
            let state = state.borrow();
            if let Some(ws) = &state.websocket {
                if ws.ready_state() == 1 { // OPEN
                    // Get current thread ID
                    let thread_id = state.current_thread_id.unwrap_or(1);
                    
                    // Create message body with correct format
                    let body_obj = js_sys::Object::new();
                    js_sys::Reflect::set(&body_obj, &"type".into(), &"send_message".into()).unwrap();
                    js_sys::Reflect::set(&body_obj, &"thread_id".into(), &thread_id.into()).unwrap();
                    js_sys::Reflect::set(&body_obj, &"content".into(), &text.into()).unwrap();
                    js_sys::Reflect::set(&body_obj, &"message_id".into(), &message_id.into()).unwrap();
                    
                    // Get selected model if any
                    if !state.selected_model.is_empty() {
                        js_sys::Reflect::set(&body_obj, &"model".into(), &state.selected_model.clone().into()).unwrap();
                    }
                    
                    let body_string = js_sys::JSON::stringify(&body_obj).unwrap();
                    
                    // Convert JsString to String before sending
                    if let Some(string_data) = body_string.as_string() {
                        // Send through WebSocket
                        web_sys::console::log_1(&format!("Sending message: {}", string_data).into());
                        let _ = ws.send_with_str(&string_data);
                    } else {
                        web_sys::console::error_1(&"Failed to convert JSON to string".into());
                    }
                } else {
                    // WebSocket not connected, try to reconnect
                    web_sys::console::warn_1(&"WebSocket not connected, reconnecting...".into());
                    // Create a new client and connect
                    let mut client = WsClientV2::new_default();
                    let _ = client.connect();
                }
            }
        });
    } else {
        web_sys::console::error_1(&"Cannot send message: No websocket connection or no agent selected".into());
    }
} 