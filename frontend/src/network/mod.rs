// Re-export network modules
pub mod api_client;
pub mod ws_client_v2;
pub mod event_types;
pub mod messages;
pub mod topic_manager;
pub mod ui_updates;

// Re-export commonly used items
pub use api_client::ApiClient;
pub use ws_client_v2::WsClientV2;
pub use topic_manager::TopicManager;


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