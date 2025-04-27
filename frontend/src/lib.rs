use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{window, Document};
use network::ui_updates;
use crate::state::dispatch_global_message;

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
    
    // Create base UI elements (header and status bar)
    ui::setup::create_base_ui(&document)?;
    
    // --- NEW: Initialize and Connect WebSocket v2 ---
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
    
    // By default, start with dashboard view
    web_sys::console::log_1(&"Setting initial view to Dashboard".into());
    
    // Set active view in app state first
    state::APP_STATE.with(|state_ref| {
        let mut state = state_ref.borrow_mut();
        state.active_view = storage::ActiveView::Dashboard;
    });
    
    // Then render the dashboard view
    views::render_active_view_by_type(&storage::ActiveView::Dashboard, &document)?;
    
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
    tabs_container.set_id("tabs-container");
    tabs_container.set_class_name("tabs-container");
    
    // Create dashboard tab
    let dashboard_tab = document.create_element("button")?;
    dashboard_tab.set_id("dashboard-tab");
    dashboard_tab.set_class_name("tab-button active");
    dashboard_tab.set_inner_html("Agent Dashboard");
    
    // Create canvas tab
    let canvas_tab = document.create_element("button")?;
    canvas_tab.set_id("canvas-tab");
    canvas_tab.set_class_name("tab-button");
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