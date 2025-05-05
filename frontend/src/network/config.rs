/// API route configuration
pub struct ApiConfig {
    base_url: String,
}

impl Default for ApiConfig {
    /// Provides a *minimal* default configuration that points to the local
    /// development backend.  The values returned here are **only** meant for
    /// unit-tests (which run in a headless browser without the normal
    /// bootstrap sequence) or very early start-up phases before
    /// `init_api_config()` is executed.  Production code **must** still call
    /// `init_api_config()` so that the real URL – injected at build-time via
    /// the `API_BASE_URL` environment variable or at runtime via
    /// `init_api_config_js()` – is stored inside the global `API_CONFIG`.
    ///
    /// Keeping the default lightweight avoids having to litter the rest of
    /// the code base with `Option` checks while still preventing hard panics
    /// during tests.
    fn default() -> Self {
        Self {
            base_url: "http://localhost".to_string(),
        }
    }
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

    /// Create a new ApiConfig from a URL string
    pub fn from_url(url: &str) -> Self {
        Self { base_url: url.trim_end_matches('/').to_string() }
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
    #[allow(dead_code)]
    pub fn url(&self, path: &str) -> String {
        format!("{}/api{}", self.base_url, path)
    }
} 