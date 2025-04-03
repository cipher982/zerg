// Re-export network modules
pub mod api_client;
pub mod event_types;
pub mod messages;
pub mod ui_updates;
pub mod ws_client;
pub mod ws_client_v2;

// Re-export commonly used items
pub use api_client::ApiClient;
pub use ws_client::setup_websocket;
pub use ws_client_v2::{WsClientV2, IWsClient};
pub use event_types::{EventType, MessageType};
pub use messages::{WsMessage, builders as message_builders};

// Add TopicManager
pub mod topic_manager;
pub use topic_manager::TopicManager;

use wasm_bindgen::JsValue;

// Helper function to get API base URL
pub(crate) fn get_api_base_url() -> Result<String, &'static str> {
    #[cfg(debug_assertions)]
    {
        Ok("http://localhost:8001".to_string())
    }
    #[cfg(not(debug_assertions))]
    {
        if let Some(window) = web_sys::window() {
            if let Ok(location) = window.location().href() {
                let url = url::Url::parse(&location)
                    .map_err(|_| "Failed to parse window location")?;
                return Ok(format!("{}://{}", url.scheme(), url.host_str().unwrap_or("localhost")));
            }
        }
        Err("Could not determine API base URL")
    }
} 