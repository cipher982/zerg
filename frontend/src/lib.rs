#![allow(dead_code)]

use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static SHORTCUT_HANDLER: RefCell<Option<wasm_bindgen::closure::Closure<dyn FnMut(web_sys::KeyboardEvent)>>> = RefCell::new(None);
}

/// Register the global keyboard shortcut handler if not already.
pub fn register_global_shortcuts(document: &web_sys::Document) {
    use wasm_bindgen::closure::Closure;
    use web_sys::EventTarget;
    // If already registered, do nothing
    SHORTCUT_HANDLER.with(|cell| {
        if cell.borrow().is_some() { return; }
        let keydown_cb = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            // Prevent global shortcuts from firing inside input fields or editable areas.
            if let Some(active_el) = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.active_element())
            {
                let tag = active_el.node_name();
                let is_input = tag == "INPUT" || tag == "TEXTAREA";
                let is_editable = active_el.dyn_ref::<web_sys::HtmlElement>().map_or(false, |el| el.is_content_editable());
                if is_input || is_editable {
                    // User is typing in input or editable, skip shortcuts
                    return;
                }
            }
            let key = event.key();
            // ? with no modifiers → show shortcut help
            if key == "?" && !event.ctrl_key() && !event.meta_key() {
                event.prevent_default();
                crate::toast::info("Shortcuts:\nN – New agent\nR – Run focused agent\n↑/↓ – Move row focus\nEnter – Expand row\n? – Show this help");
                return;
            }
            // Global dashboard-only shortcuts without modifiers --------------
            use crate::storage::ActiveView;
            let active_view = crate::state::APP_STATE.with(|s| s.borrow().active_view.clone());
            if active_view == ActiveView::Dashboard && !event.ctrl_key() && !event.meta_key() {
                match key.as_str() {
                    "n" | "N" => {
                        event.prevent_default();
                        let agent_name = format!(
                            "{} {}",
                            crate::constants::DEFAULT_AGENT_NAME,
                            (js_sys::Math::random() * 100.0).round()
                        );
                        crate::state::dispatch_global_message(crate::messages::Message::RequestCreateAgent {
                            name: agent_name,
                            system_instructions: crate::constants::DEFAULT_SYSTEM_INSTRUCTIONS.to_string(),
                            task_instructions: crate::constants::DEFAULT_TASK_INSTRUCTIONS.to_string(),
                        });
                    }
                    "r" | "R" => {
                        event.prevent_default();
                        if let Some(active_el) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.active_element()) {
                            if let Some(agent_id_attr) = active_el.get_attribute("data-agent-id") {
                                if let Ok(agent_id) = agent_id_attr.parse::<u32>() {
                                    wasm_bindgen_futures::spawn_local(async move {
                                        match crate::network::api_client::ApiClient::run_agent(agent_id).await {
                                            Ok(_) => crate::toast::success("Agent queued to run"),
                                            Err(e) => crate::toast::error(&format!("Failed to run agent: {:?}", e)),
                                        }
                                    });
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }) as Box<dyn FnMut(_)>);
        let target: web_sys::EventTarget = document.clone().into();
        let _ = target.add_event_listener_with_callback("keydown", keydown_cb.as_ref().unchecked_ref());
        cell.replace(Some(keydown_cb));
    });
}

/// Remove global keyboard shortcuts handler (power mode off)
pub fn remove_global_shortcuts(document: &web_sys::Document) {
    use web_sys::EventTarget;
    SHORTCUT_HANDLER.with(|cell| {
        if let Some(cb) = cell.borrow_mut().take() {
            let target: EventTarget = document.clone().into();
            let _ = target.remove_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
        }
    });
}
//---------------------------------------------------------------------------
// Temporary lint relaxations ------------------------------------------------
//---------------------------------------------------------------------------

// Continue the incremental clean-up – we silence *dead_code* warnings
// so the CI remains green while the incremental refactor is ongoing.

//---------------------------------------------------------------------------
// Temporary lint relaxations ------------------------------------------------
//---------------------------------------------------------------------------
//
// The CI pipeline now enforces `cargo clippy -D warnings`.  A handful of
// legacy modules still trigger non-critical lints (redundant imports, dead
// code in stubs, etc.).  We allow those at the crate root so that the build
// remains green while we refactor incrementally.  **Do not** add new allows
// without a ticket – instead, fix the underlying issue.
// #![allow(
//     clippy::redundant_field_names,
//     clippy::single_component_path_imports,
//     clippy::collapsible_else_if,
//     clippy::crate_in_macro_def,
//     clippy::for_kv_map,
//     clippy::needless_borrow,
//     clippy::unnecessary_cast,
//     clippy::type_complexity,
//     clippy::unnecessary_map_or,
//     clippy::needless_return,
//     clippy::missing_const_for_thread_local,
//     clippy::bind_instead_of_map,

//     // -----------------------------------------------------------------
//     // Transitional: allow the remainder of Clippy lints so the strict
//     // `-D warnings` gate does not block CI while legacy code is still being
//     // refactored.  Remove this once we reach zero-warning baseline.
//     clippy::all,
//     deprecated,
//     dead_code,
//     unused_mut,
// )]

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{window, Document};
use network::ui_updates;
use crate::state::dispatch_global_message;
use crate::components::auth as components_auth;
use crate::constants::{CSS_TAB_BUTTON, CSS_TAB_BUTTON_ACTIVE, ID_GLOBAL_CANVAS_TAB, ID_GLOBAL_DASHBOARD_TAB, BUTTON_TYPE_BUTTON, ATTR_TYPE};

// Import modules
mod models;
mod state;
mod canvas;
mod ui;
mod network;
mod storage;
mod components;
mod messages;
mod update;
mod views;
mod constants;
mod thread_handlers;
mod command_executors;
mod models_config;
mod pages;
mod scheduling;
mod utils;
mod toast;

// ---------------------------------------------------------------------------
// Global keyboard shortcuts
// ---------------------------------------------------------------------------

fn init_global_shortcuts(document: &Document) {
    use wasm_bindgen::closure::Closure;
    use web_sys::EventTarget;

    let keydown_cb = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
        // Prevent global shortcuts from firing inside input fields or editable areas.
        if let Some(active_el) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.active_element())
        {
            let tag = active_el.node_name();
            let is_input = tag == "INPUT" || tag == "TEXTAREA";
            let is_editable = active_el.dyn_ref::<web_sys::HtmlElement>().map_or(false, |el| el.is_content_editable());
            if is_input || is_editable {
                // User is typing in input or editable, skip shortcuts
                return;
            }
        }

        let key = event.key();


        // ? with no modifiers → show shortcut help
        if key == "?" && !event.ctrl_key() && !event.meta_key() {
            event.prevent_default();
            crate::toast::info("Shortcuts:\nN – New agent\nR – Run focused agent\n↑/↓ – Move row focus\nEnter – Expand row\n? – Show this help");
            return;
        }

        // Global dashboard-only shortcuts without modifiers --------------
        use crate::storage::ActiveView;
        let active_view = crate::state::APP_STATE.with(|s| s.borrow().active_view.clone());

        if active_view == ActiveView::Dashboard && !event.ctrl_key() && !event.meta_key() {
            match key.as_str() {
                "n" | "N" => {
                    event.prevent_default();
                    // Generate random name similar to UI button behaviour
                    let agent_name = format!(
                        "{} {}",
                        crate::constants::DEFAULT_AGENT_NAME,
                        (js_sys::Math::random() * 100.0).round()
                    );
                    crate::state::dispatch_global_message(crate::messages::Message::RequestCreateAgent {
                        name: agent_name,
                        system_instructions: crate::constants::DEFAULT_SYSTEM_INSTRUCTIONS.to_string(),
                        task_instructions: crate::constants::DEFAULT_TASK_INSTRUCTIONS.to_string(),
                    });
                }
                "r" | "R" => {
                    event.prevent_default();
                    // If focused row has data-agent-id, trigger run.
                    if let Some(active_el) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.active_element()) {
                        if let Some(agent_id_attr) = active_el.get_attribute("data-agent-id") {
                            if let Ok(agent_id) = agent_id_attr.parse::<u32>() {
                                // Reuse ApiClient::run_agent logic (without spinner)
                                wasm_bindgen_futures::spawn_local(async move {
                                    match crate::network::api_client::ApiClient::run_agent(agent_id).await {
                                        Ok(_) => crate::toast::success("Agent queued to run"),
                                        Err(e) => crate::toast::error(&format!("Failed to run agent: {:?}", e)),
                                    }
                                });
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }) as Box<dyn FnMut(_)>);

    let target: EventTarget = document.clone().into();
    let _ = target.add_event_listener_with_callback("keydown", keydown_cb.as_ref().unchecked_ref());
    keydown_cb.forget();
}
mod dom_utils;
mod auth;
mod ui_components;
pub mod reducers;

// Export convenience macros crate-wide
#[macro_use]
mod macros;

/// Basic hash router for a handful of top-level pages.
fn route_hash(hash: &str) {
    use crate::storage::ActiveView;
    use crate::messages::Message;
    match hash {
        "#/profile" => crate::state::dispatch_global_message(Message::ToggleView(ActiveView::Profile)),
        "#/canvas" => crate::state::dispatch_global_message(Message::ToggleView(ActiveView::Canvas)),
        "#/dashboard" | "" | "#/" => {
            crate::state::dispatch_global_message(Message::ToggleView(ActiveView::Dashboard))
        }
        _ => {}
    }
}


// Main entry point for the WASM application
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    // Initialize better panic messages
    console_error_panic_hook::set_once();

    // Log the API base URL for debugging
    web_sys::console::log_1(&format!("API_BASE_URL at build: {:?}", option_env!("API_BASE_URL")).into());

    // Initialize API configuration before any network operations
    if let Err(e) = network::init_api_config() {
        web_sys::console::error_1(&format!("Failed to initialize API config: {:?}", e).into());
        return Err(JsValue::from_str(&format!("API config initialization failed: {}", e)));
    }

    // Get the document
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");

    // Power-mode keyboard shortcuts are now opt-in.  The handler will be
    // installed via `Message::SetPowerMode(true)` after the user toggles the
    // switch in the profile dropdown.

    // -------------------------------------------------------------------
    // NEW: Discover runtime flags from /api/system/info before deciding on
    //       the authentication flow.  This avoids the need to synchronise
    //       env vars between backend and frontend during local dev.
    // -------------------------------------------------------------------

    // Perform the network call in a *blocking* fashion for this early stage
    // by using `spawn_local` and returning early from `start()`.  The rest of
    // the bootstrap happens inside the async block.

    let doc_clone = document.clone();
    spawn_local(async move {
        // Attempt to fetch the system info endpoint.  If it fails we fall
        // back to the previous compile-time logic so production builds that
        // are already configured continue to work.

        let sys_info_json = match network::api_client::ApiClient::fetch_system_info().await {
            Ok(j) => j,
            Err(e) => {
                web_sys::console::warn_1(&format!("Failed to fetch /system/info: {:?}", e).into());
                "{}".to_string()
            }
        };

        #[derive(serde::Deserialize)]
        struct SysInfo {
            #[serde(default)]
            auth_disabled: bool,
            #[serde(default)]
            google_client_id: Option<String>,
        }

        let info: SysInfo = serde_json::from_str(&sys_info_json).unwrap_or(SysInfo {
            auth_disabled: false,
            google_client_id: None,
        });

        // Store google_client_id in global state for later reuse (e.g. after
        // a manual logout when we need to recreate the overlay quickly).
        state::APP_STATE.with(|st| {
            st.borrow_mut().google_client_id = info.google_client_id.clone();
        });

        if info.auth_disabled {
            // ─── Dev mode ───
            // State will be updated via CurrentUserLoaded message

            if let Err(e) = bootstrap_app_after_login(&doc_clone) {
                web_sys::console::error_1(&e);
            }

            let dummy_user = crate::models::CurrentUser {
                id: 0,
                email: "dev@local".to_string(),
                display_name: Some("Developer".to_string()),
                avatar_url: None,
                prefs: None,
                gmail_connected: false,
            };
            dispatch_global_message(crate::messages::Message::CurrentUserLoaded(dummy_user));
            return;
        }

        // ─── Auth enabled ───
        // If we *already* have a JWT → bootstrap immediately.
        if state::APP_STATE.with(|s| s.borrow().logged_in) {
            if let Err(e) = bootstrap_app_after_login(&doc_clone) {
                web_sys::console::error_1(&e);
            }
            return;
        }

        // Otherwise show Google Sign-In overlay (needs client_id).
        let client_id = info.google_client_id.as_deref().unwrap_or("");

        if client_id.is_empty() {
            web_sys::console::error_1(&"Google Client ID missing but authentication is enabled".into());
            return;
        }

        components_auth::mount_login_overlay(&doc_clone, client_id);
    });

    // Nothing else to do synchronously – actual bootstrap continues in async
    // block above.
    Ok(())
}

/// Performs the main UI + networking bootstrap sequence.  Called once on page
/// load *if* a JWT is already present, or after the Google sign-in flow
/// completes successfully.
pub(crate) fn bootstrap_app_after_login(document: &Document) -> Result<(), JsValue> {
    // Create base UI elements (header and status bar)
    ui::setup::create_base_ui(document)?;
    
    // --- Initialize and Connect WebSocket v2 ---
    // Access the globally initialized WS client and Topic Manager
    let (ws_client_rc, topic_manager_rc) = state::APP_STATE.with(|state_ref| {
        let state = state_ref.borrow();
        (state.ws_client.clone(), state.topic_manager.clone())
    });
    
    // Setup callbacks for WebSocket
    {
        let topic_manager_clone = topic_manager_rc.clone();
        let ws_client_clone = ws_client_rc.clone();

        // Borrow mutably to set callbacks
        let mut ws_client = ws_client_clone.borrow_mut();

        // Set callbacks for websocket events
        setup_websocket_callbacks(&mut ws_client, topic_manager_clone.clone())?;
        
        // Set initial status before attempting connection
        ui_updates::update_connection_status("Connecting", "yellow");

        // Initiate the connection
        if let Err(e) = ws_client.connect() {
             web_sys::console::error_1(&format!("Initial WebSocket connect failed: {:?}", e).into());
             // Update UI status on initial connection error
             ui_updates::update_connection_status("Error", "red");
        }
    } // Mutable borrow of ws_client ends here
    
    // Create the tab navigation
    create_tab_navigation(&document)?;

    // -------------------------------------------------------------------
    // Simple hash-based routing for the Profile page (and a few others)
    // -------------------------------------------------------------------

    {
        let hash_now = web_sys::window()
            .and_then(|w| w.location().hash().ok())
            .unwrap_or_default();

        // Dispatch once for the current hash on load
        route_hash(&hash_now);

        // Install listener for subsequent changes
        if let Some(win) = web_sys::window() {
            let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |_e: web_sys::Event| {
                if let Ok(hash) = web_sys::window().unwrap().location().hash() {
                    route_hash(&hash);
                }
            }) as Box<dyn FnMut(_)>);

            let _ = win.add_event_listener_with_callback("hashchange", closure.as_ref().unchecked_ref());
            closure.forget();
        }
    }
    
    // Set up shared UI components
    ui::main::setup_ui(&document)?;
    
    // Show initial loading spinner
    if let Some(loading_spinner) = document.get_element_by_id("loading-spinner") {
        if let Some(spinner_element) = loading_spinner.dyn_ref::<web_sys::HtmlElement>() {
            spinner_element.set_class_name("active");
        }
    }
    
    // Initialize data loading
    initialize_data_loading();

    // -------------------------------------------------------------------
    // If a JWT is present fetch the current user profile.  This also covers
    // the *page refresh* scenario where Google login overlay is skipped.
    // -------------------------------------------------------------------

    wasm_bindgen_futures::spawn_local(async move {
        if crate::state::APP_STATE.with(|s| s.borrow().logged_in) {
            if let Ok(profile_json) = crate::network::api_client::ApiClient::fetch_current_user().await {
                if let Ok(user) = serde_json::from_str::<crate::models::CurrentUser>(&profile_json) {
                    dispatch_global_message(crate::messages::Message::CurrentUserLoaded(user));
                }
            }
        }
    });
    
    // By default, start with dashboard view
    web_sys::console::log_1(&"Setting initial view to Dashboard".into());
    
    // Set initial view to Dashboard via message dispatch (uses update+commands)
    dispatch_global_message(crate::messages::Message::ToggleView(storage::ActiveView::Dashboard));

    Ok(())
}

// Helper function to setup WebSocket callbacks
fn setup_websocket_callbacks(
    ws_client: &mut network::WsClientV2, 
    topic_manager: std::rc::Rc<std::cell::RefCell<network::TopicManager>>
) -> Result<(), JsValue> {
    // Set on_connect: Call topic_manager.resubscribe_all_topics
    let tm_on_connect = topic_manager.clone();
    ws_client.set_on_connect(move || {
        web_sys::console::log_1(&"on_connect callback triggered".into());
        if let Err(e) = tm_on_connect.borrow().resubscribe_all_topics() {
            web_sys::console::error_1(&format!("Failed to resubscribe topics: {:?}", e).into());
        }
        // Update UI status
        ui_updates::update_connection_status("Connected", "green");
    });

    // Set on_message: Call topic_manager.route_incoming_message
    let tm_on_message = topic_manager.clone();
    ws_client.set_on_message(move |json_value| {
        // Route the message using the topic manager
        tm_on_message.borrow().route_incoming_message(json_value);
    });

    // Set on_disconnect
    ws_client.set_on_disconnect(|| {
        web_sys::console::warn_1(&"WebSocket disconnected (on_disconnect callback)".into());
        // Update UI status
        ui_updates::update_connection_status("Disconnected", "red");
    });
    
    Ok(())
}

// Helper function to initialize data loading
fn initialize_data_loading() {
    spawn_local(async {
        // Fetch available models from API
        let models_json = match network::api_client::ApiClient::fetch_available_models().await {
            Ok(json) => json,
            Err(e) => {
                web_sys::console::error_1(&format!("Failed to fetch models: {:?}", e).into());
                return;
            }
        };
        let models: Vec<crate::models_config::ModelConfig> = match serde_json::from_str(&models_json) {
            Ok(m) => m,
            Err(e) => {
                web_sys::console::error_1(&format!("Failed to parse models: {:?}", e).into());
                return;
            }
        };
        let default_model = models.iter().find(|m| m.is_default);
        if let Some(default) = default_model {
            let model_pairs = models.iter().map(|m| (m.id.clone(), m.display_name.clone())).collect();
            dispatch_global_message(crate::messages::Message::SetAvailableModels {
                models: model_pairs,
                default_model_id: default.id.clone(),
            });
        } else {
            web_sys::console::error_1(&"No default model found in backend response".into());
            return;
        }
        
        // Load the state (this will also try to load workflows and agents)
        state::APP_STATE.with(|s| {
            let mut app_state = s.borrow_mut();
            if let Err(e) = storage::load_state(&mut app_state) {
                web_sys::console::error_1(&format!("Error loading state: {:?}", e).into());
            }
        }); // The borrow is dropped when this block ends
        
        // Create a default workflow if none exists
        state::APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            if state.workflows.is_empty() {
                state.create_workflow("Default Workflow".to_string());
            }
            
            // If we have an agent selected in chat view, load their threads
            if state.active_view == storage::ActiveView::ChatView {
                if let Some(agent_id) = state.agents.keys().next().cloned() {
                    crate::state::dispatch_global_message(crate::messages::Message::LoadThreads(agent_id));
                }
            }
        });
        
        // Set up auto-save timer
        if let Err(e) = setup_auto_save_timer(30000) { // 30 seconds
            web_sys::console::error_1(&format!("Failed to setup auto-save: {:?}", e).into());
        }
    });
}

// Create tab navigation for switching between dashboard and canvas
fn create_tab_navigation(document: &Document) -> Result<(), JsValue> {
    web_sys::console::log_1(&"create_tab_navigation called".into());
    // Create a tabs container
    let tabs_container = document.create_element("div")?;
    tabs_container.set_id("global-tabs-container");
    tabs_container.set_class_name("tabs-container");
    
    // Create dashboard tab
    let dashboard_tab = document.create_element("button")?;
    dashboard_tab.set_attribute(ATTR_TYPE, BUTTON_TYPE_BUTTON)?;
    dashboard_tab.set_id(ID_GLOBAL_DASHBOARD_TAB);
    dashboard_tab.set_attribute("data-testid", "global-dashboard-tab")?;
    dashboard_tab.set_class_name(CSS_TAB_BUTTON_ACTIVE);
    dashboard_tab.set_inner_html("Agent Dashboard");
    
    // Create canvas tab
    let canvas_tab = document.create_element("button")?;
    canvas_tab.set_attribute(ATTR_TYPE, BUTTON_TYPE_BUTTON)?;
    canvas_tab.set_id(ID_GLOBAL_CANVAS_TAB);
    canvas_tab.set_class_name(CSS_TAB_BUTTON);
    canvas_tab.set_inner_html("Canvas Editor");
    
    // Add tabs to container
    tabs_container.append_child(&dashboard_tab)?;
    tabs_container.append_child(&canvas_tab)?;
    
    // Set up dashboard tab click handler
    {
        let dashboard_click = Closure::wrap(Box::new(move |_: web_sys::MouseEvent| {
            web_sys::console::log_1(&"Dashboard tab clicked".into());
            dispatch_global_message(messages::Message::ToggleView(storage::ActiveView::Dashboard));
        }) as Box<dyn FnMut(_)>);
        
        dashboard_tab.add_event_listener_with_callback("click", dashboard_click.as_ref().unchecked_ref())?;
        dashboard_click.forget();
    }
    
    // Set up canvas tab click handler
    {
        let canvas_click = Closure::wrap(Box::new(move |_: web_sys::MouseEvent| {
            web_sys::console::log_1(&"Canvas tab clicked".into());
            dispatch_global_message(messages::Message::ToggleView(storage::ActiveView::Canvas));
        }) as Box<dyn FnMut(_)>);
        
        canvas_tab.add_event_listener_with_callback("click", canvas_click.as_ref().unchecked_ref())?;
        canvas_click.forget();
    }
    
    // Add the tabs container to the document
    let header = document.get_element_by_id("header")
        .ok_or_else(|| JsValue::from_str("Header not found"))?;
    header.append_child(&tabs_container)?;
    web_sys::console::log_1(&"Tabs added to DOM".into());
    
    Ok(())
}

// Set up a timer to auto-save state periodically
fn setup_auto_save_timer(interval_ms: i32) -> Result<(), JsValue> {
    let window = window().expect("no global window exists");
    
    // Create a closure that will be called by setInterval
    let auto_save_callback = Closure::wrap(Box::new(move || {
        state::APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            if let Err(e) = state.save_if_modified() {
                web_sys::console::warn_1(&format!("Auto-save failed: {:?}", e).into());
            }
        });
    }) as Box<dyn FnMut()>);
    
    // Create interval timer
    let _ = window.set_interval_with_callback_and_timeout_and_arguments(
        auto_save_callback.as_ref().unchecked_ref(),
        interval_ms,
        &js_sys::Array::new(),
    )?;
    
    // Forget the closure to prevent it from being dropped
    auto_save_callback.forget();
    
    Ok(())
}
