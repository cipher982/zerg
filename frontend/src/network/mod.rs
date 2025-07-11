// Re-export network modules
pub mod api_client;
pub mod config;
pub mod event_types;
pub mod messages;
pub mod topic_manager;
pub mod ui_updates;
pub mod ws_client_v2;

// Re-export commonly used items
pub use api_client::ApiClient;
pub use topic_manager::TopicManager;
pub use ws_client_v2::WsClientV2;

use config::ApiConfig;
use lazy_static::lazy_static;
use std::sync::RwLock;
use wasm_bindgen::prelude::*;

lazy_static! {
    static ref API_CONFIG: RwLock<Option<ApiConfig>> = RwLock::new(None);
}

/// Initialize the API configuration. Must be called before any network operations.
pub fn init_api_config() -> Result<(), &'static str> {
    let config = ApiConfig::new()?;
    *API_CONFIG.write().unwrap() = Some(config);
    Ok(())
}

/// Initialize the API configuration from a JS-provided URL.
/// This allows runtime configuration of the API endpoints.
#[wasm_bindgen]
pub fn init_api_config_js(api_base_url: &str) -> Result<(), JsValue> {
    let config = ApiConfig::from_url(api_base_url);
    *API_CONFIG.write().unwrap() = Some(config);
    Ok(())
}

/// Get the API configuration
#[allow(dead_code)]
fn get_api_config() -> Result<std::sync::RwLockReadGuard<'static, Option<ApiConfig>>, &'static str>
{
    Ok(API_CONFIG.read().unwrap())
}

/// Get the WebSocket URL
pub(crate) fn get_ws_url() -> Result<String, &'static str> {
    let maybe_url = {
        let guard = API_CONFIG.read().unwrap();
        guard.as_ref().map(|cfg| cfg.ws_url())
    };

    // Use the configured URL or fall back to a sensible default so unit and
    // wasm-bindgen tests can run without the full app bootstrap.
    let base_url = maybe_url.unwrap_or_else(|| ApiConfig::default().ws_url());

    // If a JWT is present in localStorage append it as query parameter so the
    // backend can authenticate the WebSocket upgrade request.
    let token_opt = crate::utils::current_jwt();

    if let Some(tok) = token_opt {
        Ok(format!("{}?token={}", base_url, tok))
    } else {
        Ok(base_url)
    }
}

/// Get the base URL for API calls
pub(crate) fn get_api_base_url() -> Result<String, &'static str> {
    let maybe_base = {
        let guard = API_CONFIG.read().unwrap();
        guard.as_ref().map(|cfg| cfg.base_url().to_string())
    };

    Ok(maybe_base.unwrap_or_else(|| ApiConfig::default().base_url().to_string()))
}
