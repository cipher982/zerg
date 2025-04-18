/// API route configuration
pub struct ApiConfig {
    base_url: String,
}

impl ApiConfig {
    /// Create a new ApiConfig from the API_BASE_URL environment variable
    pub fn new() -> Result<Self, &'static str> {
        if let Some(url) = option_env!("API_BASE_URL") {
            Ok(Self { base_url: url.trim_end_matches('/').to_string() })
        } else {
            Err("API_BASE_URL environment variable is not set")
        }
    }

    /// Get the base URL for all API calls
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Get the WebSocket URL
    pub fn ws_url(&self) -> String {
        let ws_base = if self.base_url.starts_with("https://") {
            self.base_url.replace("https://", "wss://")
        } else {
            self.base_url.replace("http://", "ws://")
        };
        format!("{}/api/ws", ws_base)
    }

    /// Get a full API URL for a given path
    pub fn url(&self, path: &str) -> String {
        format!("{}/api{}", self.base_url, path)
    }
} 