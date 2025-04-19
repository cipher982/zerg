// Re-export network modules
pub mod api_client;
pub mod ws_client_v2;
pub mod event_types;
pub mod messages;
pub mod topic_manager;
pub mod ui_updates;
pub mod config;

// Re-export commonly used items
pub use api_client::ApiClient;
pub use ws_client_v2::WsClientV2;
pub use topic_manager::TopicManager;

use lazy_static::lazy_static;
use std::sync::RwLock;
use config::ApiConfig;

lazy_static! {
    static ref API_CONFIG: RwLock<Option<ApiConfig>> = RwLock::new(None);
}

/// Initialize the API configuration. Must be called before any network operations.
pub fn init_api_config() -> Result<(), &'static str> {
    let config = ApiConfig::new()?;
    *API_CONFIG.write().unwrap() = Some(config);
    Ok(())
}

/// Get the API configuration
fn get_api_config() -> Result<std::sync::RwLockReadGuard<'static, Option<ApiConfig>>, &'static str> {
    Ok(API_CONFIG.read().unwrap())
}

/// Get the WebSocket URL
pub(crate) fn get_ws_url() -> Result<String, &'static str> {
    get_api_config()
        .map(|config| config.as_ref().expect("API config not initialized").ws_url())
}

/// Get the base URL for API calls
pub(crate) fn get_api_base_url() -> Result<String, &'static str> {
    get_api_config()
        .map(|config| config.as_ref().expect("API config not initialized").base_url().to_string())
} 