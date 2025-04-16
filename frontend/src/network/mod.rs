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
    if let Some(url) = option_env!("API_BASE_URL") {
        Ok(url.to_string())
    } else {
        Err("API_BASE_URL environment variable is not set")
    }
} 