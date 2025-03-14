// Re-export network modules
pub mod ws_client;
pub mod api_client;
pub mod ui_updates;

// Re-export commonly used functions for backwards compatibility
pub use ws_client::{setup_websocket, send_text_to_backend, fetch_available_models};
pub use api_client::ApiClient;
pub use ui_updates::{update_connection_status, flash_activity};

use wasm_bindgen::JsValue;

// Shared utility functions
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