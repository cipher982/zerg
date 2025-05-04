use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
// Individual web-sys types are imported where required – RequestMode is still
// used in several helper methods so we keep the top-level import for
// convenience.
use web_sys::RequestMode;
use super::ui_updates::flash_activity;
use crate::constants::{DEFAULT_NODE_WIDTH, DEFAULT_NODE_HEIGHT};
use std::rc::Rc;
use serde::Deserialize;

#[derive(Deserialize)]
struct TokenOut {
    access_token: String,
    expires_in: u32,
    token_type: String,
}

// ----------------------------------------------------------------------------
// Helper – read the persisted JWT from localStorage
// ----------------------------------------------------------------------------

use crate::utils as auth_utils;

// REST API Client for Agent operations
pub struct ApiClient;

impl ApiClient {
    // Get the base URL for API calls
    fn api_base_url() -> String {
        super::get_api_base_url().expect("API base URL must be set (no fallback allowed)")
    }

    // ---------------- Agent Runs ----------------

    /// Fetch most recent runs for an agent (limit parameter default 20)
    pub async fn get_agent_runs(agent_id: u32, limit: u32) -> Result<String, JsValue> {
        let url = format!(
            "{}/api/agents/{}/runs?limit={}",
            Self::api_base_url(),
            agent_id,
            limit
        );
        Self::fetch_json(&url, "GET", None).await
    }

    // Get available models
    pub async fn fetch_available_models() -> Result<String, JsValue> {
        let url = format!("{}/api/models", Self::api_base_url());
        Self::fetch_json(&url, "GET", None).await
    }

    // Get all agents
    pub async fn get_agents() -> Result<String, JsValue> {
        let url = format!("{}/api/agents", Self::api_base_url());
        Self::fetch_json(&url, "GET", None).await
    }

    // Get a specific agent by ID
    pub async fn get_agent(agent_id: u32) -> Result<String, JsValue> {
        let url = format!("{}/api/agents/{}", Self::api_base_url(), agent_id);
        Self::fetch_json(&url, "GET", None).await
    }

    // Create a new agent
    #[allow(dead_code)]
    pub async fn create_agent(agent_data: &str) -> Result<String, JsValue> {
        let url = format!("{}/api/agents", Self::api_base_url());
        Self::fetch_json(&url, "POST", Some(agent_data)).await
    }

    // Update an existing agent
    pub async fn update_agent(agent_id: u32, agent_data: &str) -> Result<String, JsValue> {
        let url = format!("{}/api/agents/{}", Self::api_base_url(), agent_id);
        Self::fetch_json(&url, "PUT", Some(agent_data)).await
    }

    // Delete an agent
    pub async fn delete_agent(agent_id: u32) -> Result<(), JsValue> {
        let url = format!("{}/api/agents/{}", Self::api_base_url(), agent_id);
        let _ = Self::fetch_json(&url, "DELETE", None).await?;
        Ok(())
    }

    // Get messages for a specific agent
    #[allow(dead_code)]
    pub async fn get_agent_messages(agent_id: u32) -> Result<String, JsValue> {
        let url = format!("{}/api/agents/{}/messages", Self::api_base_url(), agent_id);
        Self::fetch_json(&url, "GET", None).await
    }

    // Add a message to an agent
    pub async fn create_agent_message(agent_id: u32, message_data: &str) -> Result<String, JsValue> {
        let url = format!("{}/api/agents/{}/messages", Self::api_base_url(), agent_id);
        Self::fetch_json(&url, "POST", Some(message_data)).await
    }

    // Trigger an agent to run
    pub async fn run_agent(agent_id: u32) -> Result<String, JsValue> {
        let url = format!("{}/api/agents/{}/task", Self::api_base_url(), agent_id);
        Self::fetch_json(&url, "POST", None).await
    }

    // -------------------------------------------------------------------
    // Agent *details* endpoint (debug modal)
    // -------------------------------------------------------------------

    pub async fn get_agent_details(agent_id: u32) -> Result<String, JsValue> {
        let url = format!("{}/api/agents/{}/details", Self::api_base_url(), agent_id);
        Self::fetch_json(&url, "GET", None).await
    }

    // Reset the database (development only)
    pub async fn reset_database() -> Result<String, JsValue> {
        let url = format!("{}/api/admin/reset-database", Self::api_base_url());
        Self::fetch_json(&url, "POST", None).await
    }

    // Thread management
    pub async fn get_threads(agent_id: Option<u32>) -> Result<String, JsValue> {
        let url = if let Some(id) = agent_id {
            format!("{}/api/threads?agent_id={}", Self::api_base_url(), id)
        } else {
            format!("{}/api/threads", Self::api_base_url())
        };
        Self::fetch_json(&url, "GET", None).await
    }

    #[allow(dead_code)]
    pub async fn get_thread(thread_id: u32) -> Result<String, JsValue> {
        let url = format!("{}/api/threads/{}", Self::api_base_url(), thread_id);
        Self::fetch_json(&url, "GET", None).await
    }

    pub async fn create_thread(agent_id: u32, title: &str) -> Result<String, JsValue> {
        let url = format!("{}/api/threads", Self::api_base_url());
        let thread_data = format!("{{\"agent_id\": {}, \"title\": \"{}\", \"active\": true}}", agent_id, title);
        Self::fetch_json(&url, "POST", Some(&thread_data)).await
    }

    pub async fn update_thread(thread_id: u32, title: &str) -> Result<String, JsValue> {
        let url = format!("{}/api/threads/{}", Self::api_base_url(), thread_id);
        let thread_data = format!("{{\"title\": \"{}\"}}", title);
        Self::fetch_json(&url, "PUT", Some(&thread_data)).await
    }

    pub async fn delete_thread(thread_id: u32) -> Result<(), JsValue> {
        let url = format!("{}/api/threads/{}", Self::api_base_url(), thread_id);
        let _ = Self::fetch_json(&url, "DELETE", None).await?;
        Ok(())
    }

    // Thread messages
    pub async fn get_thread_messages(thread_id: u32, skip: u32, limit: u32) -> Result<String, JsValue> {
        let url = format!("{}/api/threads/{}/messages?skip={}&limit={}", 
                       Self::api_base_url(), thread_id, skip, limit);
        Self::fetch_json(&url, "GET", None).await
    }

    pub async fn create_thread_message(thread_id: u32, content: &str) -> Result<String, JsValue> {
        let url = format!("{}/api/threads/{}/messages", Self::api_base_url(), thread_id);
        let message_data = format!("{{\"role\": \"user\", \"content\": \"{}\"}}", content);
        Self::fetch_json(&url, "POST", Some(&message_data)).await
    }

    // Run a thread – the backend expects NO body, simply triggers processing
    pub async fn run_thread(thread_id: u32) -> Result<String, JsValue> {
        let url = format!("{}/api/threads/{}/run", Self::api_base_url(), thread_id);
        // We deliberately pass None for the body – backend ignores payload.
        Self::fetch_json(&url, "POST", None).await
    }

    // -------------------------------------------------------------------
    // Authentication & User profile
    // -------------------------------------------------------------------

    /// Fetch the authenticated user's profile (`/api/users/me`).  Caller is
    /// responsible for ensuring that a valid JWT is already present in
    /// localStorage – otherwise the request will fail with 401.
    pub async fn fetch_current_user() -> Result<String, JsValue> {
        let url = format!("{}/api/users/me", Self::api_base_url());
        Self::fetch_json(&url, "GET", None).await
    }

    /// Update (PATCH) the current user's profile via `/api/users/me`.
    /// Expects `patch_json` to be a JSON string matching the backend
    /// `UserUpdate` schema, e.g. `{ "display_name": "Alice" }`.
    pub async fn update_current_user(patch_json: &str) -> Result<String, JsValue> {
        let url = format!("{}/api/users/me", Self::api_base_url());
        Self::fetch_json(&url, "PUT", Some(patch_json)).await
    }

    // Helper function to make fetch requests
    pub async fn fetch_json(url: &str, method: &str, body: Option<&str>) -> Result<String, JsValue> {
        use web_sys::{Request, RequestInit, RequestMode, Response, Headers};

        let mut opts = RequestInit::new();
        opts.set_method(method);
        opts.set_mode(RequestMode::Cors);

        // ----------------------------------------------------------------
        // Headers
        // ----------------------------------------------------------------
        let headers = Headers::new()?;

        // Always attempt to attach Authorization header if token present.
        if let Some(jwt) = auth_utils::current_jwt() {
            headers.append("Authorization", &format!("Bearer {}", jwt))?;
        }

        // Add Content-Type & body if provided
        if let Some(data) = body {
            let js_body = JsValue::from_str(data);
            opts.set_body(&js_body);
            headers.append("Content-Type", "application/json")?;
        }

        opts.set_headers(&headers);

        let request = Request::new_with_str_and_init(url, &opts)?;

        let window = web_sys::window().expect("no global window exists");
        let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
        let resp: Response = resp_value.dyn_into()?;

        // Check HTTP status – handle authentication expiry gracefully.
        if !resp.ok() {
            let status = resp.status();

            // 401 → token expired or invalid → logout & show error.
            if status == 401 {
                // Attempt to log out; ignore errors (e.g. during unit tests)
                let _ = crate::utils::logout();
            }

            let status_text = resp.status_text();
            return Err(JsValue::from_str(&format!("API request failed: {} {}", status, status_text)));
        }

        // Parse body as text – caller can decode JSON.
        let text = JsFuture::from(resp.text()?).await?;
        Ok(text.as_string().unwrap_or_default())
    }

    // -------------------------------------------------------------------
    // Authentication – Google Sign-In
    // -------------------------------------------------------------------

    /// Exchange a Google ID token for a platform JWT and persist it.
    pub async fn google_auth_login(id_token: &str) -> Result<(), JsValue> {
        let url = format!("{}/api/auth/google", Self::api_base_url());
        let payload = format!("{{\"id_token\": \"{}\"}}", id_token);

        let resp_json = Self::fetch_json(&url, "POST", Some(&payload)).await?;

        let token_out: TokenOut = serde_json::from_str(&resp_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse token response: {:?}", e)))?;

        // Store JWT in localStorage so future fetches/websocket connections are authenticated
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item("zerg_jwt", &token_out.access_token);
            }
        }

        Ok(())
    }
}

// Load agents from API and update state.agents
pub fn load_agents() {
    flash_activity(); // Flash on API call
    
    let api_base_url = format!("{}/api/agents", ApiClient::api_base_url());
    
    // Call the API and update state
    wasm_bindgen_futures::spawn_local(async move {
        let window = web_sys::window().expect("no global window exists");
        let opts = web_sys::RequestInit::new();
        opts.set_method("GET");
        opts.set_mode(RequestMode::Cors);
        
        // Create new request
        match web_sys::Request::new_with_str_and_init(&api_base_url, &opts) {
            Ok(request) => {
                let promise = window.fetch_with_request(&request);
                
                match wasm_bindgen_futures::JsFuture::from(promise).await {
                    Ok(resp_value) => {
                        let response: web_sys::Response = resp_value.dyn_into().unwrap();
                        
                        if response.ok() {
                            match response.json() {
                                Ok(json_promise) => {
                                    match wasm_bindgen_futures::JsFuture::from(json_promise).await {
                                        Ok(json_value) => {
                                            let agents_data = json_value;
                                            match serde_wasm_bindgen::from_value::<Vec<crate::models::ApiAgent>>(agents_data) {
                                                Ok(agents) => {
                                                    web_sys::console::log_1(&format!("Loaded {} agents from API", agents.len()).into());
                                                    
                                                    // Update the agents HashMap in AppState
                                                    crate::state::APP_STATE.with(|state| {
                                                        let mut state = state.borrow_mut();
                                                        state.agents.clear();
                                                        
                                                        // Add each agent to the HashMap
                                                        for agent in agents {
                                                            if let Some(id) = agent.id {
                                                                state.agents.insert(id, agent);
                                                            }
                                                        }
                                                    // Subscribe dashboard WS manager (if already initialised)
                                                    let topic_manager_rc = state.topic_manager.clone();
                                                    let handler_opt = crate::components::dashboard::ws_manager::DASHBOARD_WS.with(|cell| {
                                                        cell.borrow().as_ref().and_then(|mgr| mgr.agent_subscription_handler.clone())
                                                    });

                                                    if let Some(handler) = handler_opt {
                                                        if let Ok(mut tm) = topic_manager_rc.try_borrow_mut() {
                                                            for id in state.agents.keys() {
                                                                let topic = format!("agent:{}", id);
                                                                let _ = tm.subscribe(topic.clone(), Rc::clone(&handler));
                                                            }
                                                        }
                                                    }
                                                        
                                                        // Now that we have loaded agents, check if we need to create nodes for them
                                                        // Only create nodes for agents that don't already have one
                                                        create_nodes_for_agents(&mut state);
                                                        
                                                        state.data_loaded = true;
                                                        state.api_load_attempted = true;
                                                        state.is_loading = false;
                                                    });
                                                    
                                                    // Update the UI
                                                    if let Err(e) = crate::state::AppState::refresh_ui_after_state_change() {
                                                        web_sys::console::error_1(&format!("Error refreshing UI: {:?}", e).into());
                                                    }

                                                    // Notify the application that the dashboard data changed
                                                    crate::state::dispatch_global_message(crate::messages::Message::RefreshDashboard);
                                                },
                                                Err(e) => {
                                                    web_sys::console::error_1(&format!("Failed to deserialize agents: {:?}", e).into());
                                                    mark_load_attempted();
                                                }
                                            }
                                        },
                                        Err(e) => {
                                            web_sys::console::error_1(&format!("Failed to parse response: {:?}", e).into());
                                            mark_load_attempted();
                                        }
                                    }
                                },
                                Err(e) => {
                                    web_sys::console::error_1(&format!("Failed to call json(): {:?}", e).into());
                                    mark_load_attempted();
                                }
                            }
                        } else {
                            web_sys::console::error_1(&format!("API request failed with status: {}", response.status()).into());
                            mark_load_attempted();
                        }
                    },
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to fetch agents: {:?}", e).into());
                        mark_load_attempted();
                    }
                }
            },
            Err(e) => {
                web_sys::console::error_1(&format!("Failed to create request: {:?}", e).into());
                mark_load_attempted();
            }
        }
    });
}

// Helper to mark API load as attempted but failed
fn mark_load_attempted() {
    crate::state::APP_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.api_load_attempted = true;
        state.is_loading = false;
    });
}

// Reload a specific agent by ID
pub fn reload_agent(agent_id: u32) {
    let api_url = ApiClient::api_base_url();
    let url = format!("{}/api/agents/{}", api_url, agent_id);
    
    wasm_bindgen_futures::spawn_local(async move {
        let window = web_sys::window().expect("no global window exists");
        let opts = web_sys::RequestInit::new();
        opts.set_method("GET");
        opts.set_mode(web_sys::RequestMode::Cors);
        
        match web_sys::Request::new_with_str_and_init(&url, &opts) {
            Ok(request) => {
                match JsFuture::from(window.fetch_with_request(&request)).await {
                    Ok(resp_value) => {
                        let response: web_sys::Response = resp_value.dyn_into().unwrap();
                        
                        if response.ok() {
                            match response.json() {
                                Ok(json_promise) => {
                                    match JsFuture::from(json_promise).await {
                                        Ok(json_value) => {
                                            let agent_data = json_value;
                                            match serde_wasm_bindgen::from_value::<crate::models::ApiAgent>(agent_data) {
                                                Ok(agent) => {
                                                    // Update the agent in the agents HashMap
                                                        // First mutate state data
                                                        crate::state::APP_STATE.with(|state| {
                                                            let mut state = state.borrow_mut();
                                                            if let Some(id) = agent.id {
                                                                state.agents.insert(id, agent.clone());
                                                                for (_, node) in state.nodes.iter_mut() {
                                                                    if node.agent_id == Some(id) {
                                                                        node.text = agent.name.clone();
                                                                    }
                                                                }
                                                            }
                                                        });

                                                        // After the previous borrow ends, mark canvas dirty via queued Message
                                                        crate::state::dispatch_global_message(crate::messages::Message::MarkCanvasDirty);
                                                    
                                                    // Update the UI
                                                    if let Err(e) = crate::state::AppState::refresh_ui_after_state_change() {
                                                        web_sys::console::error_1(&format!("Error refreshing UI: {:?}", e).into());
                                                    }
                                                },
                                                Err(e) => {
                                                    web_sys::console::error_1(&format!("Failed to deserialize agent: {:?}", e).into());
                                                }
                                            }
                                        },
                                        Err(e) => {
                                            web_sys::console::error_1(&format!("Failed to parse json: {:?}", e).into());
                                        }
                                    }
                                },
                                Err(e) => {
                                    web_sys::console::error_1(&format!("Failed to call json(): {:?}", e).into());
                                }
                            }
                        }
                    },
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to fetch: {:?}", e).into());
                    }
                }
            },
            Err(e) => {
                web_sys::console::error_1(&format!("Failed to create request: {:?}", e).into());
            }
        }
    });
}

// Create nodes for agents that don't already have one
fn create_nodes_for_agents(state: &mut crate::state::AppState) {
    // First, collect all the information we need without holding the borrow
    let agents_to_add: Vec<(u32, String)> = state.agents.iter()
        .filter(|(agent_id, _)| {
            // Check if there's already a node for this agent
            !state.nodes.iter().any(|(_, node)| {
                node.agent_id == Some(**agent_id)
            })
        })
        .map(|(agent_id, agent)| (*agent_id, agent.name.clone()))
        .collect();
    
    // Calculate grid layout
    let grid_size = (agents_to_add.len() as f64).sqrt().ceil() as usize;
    
    // Now add the nodes without conflicting borrows
    for (i, (agent_id, name)) in agents_to_add.into_iter().enumerate() {
        // Calculate a grid-like position for the new node
        let row = i / grid_size;
        let col = i % grid_size;
        
        let x = 100.0 + (col as f64 * (DEFAULT_NODE_WIDTH + 50.0));
        let y = 100.0 + (row as f64 * (DEFAULT_NODE_HEIGHT + 70.0));
        
        let node_id = state.add_node_with_agent(Some(agent_id), x, y, 
            crate::models::NodeType::AgentIdentity, name);
        
        web_sys::console::log_1(&format!("Created visual node with ID: {} for agent {}", node_id, agent_id).into());
    }
} 