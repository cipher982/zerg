use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{window, Document};

mod models;
mod state;
mod canvas;
mod ui;
// Removed: mod ui_old; // Temporarily commented out to confirm everything is refactored
mod network;
pub mod storage;
mod components;
mod messages;  // New module for Message enum
mod update;    // New module for update function
mod views;     // New module for view functions
mod constants; // Module for constants and default values
pub mod thread_handlers; // Add the new module

// Main entry point for the WASM application
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    // Initialize better panic messages
    console_error_panic_hook::set_once();

    // Get the document
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    
    // Create base UI elements (header and status bar)
    ui::setup::create_base_ui(&document)?;
    
    // Set up the WebSocket connection
    network::setup_websocket()?;
    
    // Create the tab navigation
    create_tab_navigation(&document)?;
    
    // Set up the UI components and canvas but don't show them initially
    ui::main::setup_ui(&document)?;
    components::canvas_editor::setup_canvas(&document)?;
    
    // Hide canvas container initially as we'll show dashboard by default
    if let Some(canvas_container) = document.get_element_by_id("canvas-container") {
        canvas_container.set_attribute("style", "display: none;")?;
    }
    
    if let Some(input_panel) = document.get_element_by_id("input-panel") {
        input_panel.set_attribute("style", "display: none;")?;
    }
    
    // Show initial loading spinner
    if let Some(loading_spinner) = document.get_element_by_id("loading-spinner") {
        if let Some(spinner_element) = loading_spinner.dyn_ref::<web_sys::HtmlElement>() {
            spinner_element.set_class_name("active");
        }
    }
    
    // Initialize loading of both legacy data and new agent data
    spawn_local(async {
        // Load the state (this will also try to load workflows and agents)
        state::APP_STATE.with(|s| {
            let mut app_state = s.borrow_mut();
            if let Err(e) = storage::load_state(&mut app_state) {
                web_sys::console::error_1(&format!("Error loading state: {:?}", e).into());
            }
        }); // The borrow is dropped when this block ends
        
        // We don't need to call load_agents() here as it's already called by load_state_from_api
        // network::load_agents();
        
        // Create a default workflow if none exists
        state::APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            if state.workflows.is_empty() {
                state.create_workflow("Default Workflow".to_string());
            }
        });
        
        // Set up auto-save timer
        if let Err(e) = setup_auto_save_timer(30000) { // 30 seconds
            web_sys::console::error_1(&format!("Failed to setup auto-save: {:?}", e).into());
        }
    });
    
    Ok(())
}

// Create tab navigation for switching between dashboard and canvas
fn create_tab_navigation(document: &Document) -> Result<(), JsValue> {
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
            // Use the new dispatch method with ToggleView message
            let (need_refresh, _) = state::APP_STATE.with(|state| {
                let mut state = state.borrow_mut();
                state.dispatch(messages::Message::ToggleView(storage::ActiveView::Dashboard))
            });
            
            // After borrowing mutably, we can refresh UI if needed in a separate borrow
            if need_refresh {
                if let Err(e) = state::AppState::refresh_ui_after_state_change() {
                    web_sys::console::warn_1(&format!("Failed to refresh UI: {:?}", e).into());
                }
            }
        }) as Box<dyn FnMut(_)>);
        
        dashboard_tab.add_event_listener_with_callback("click", dashboard_click.as_ref().unchecked_ref())?;
        dashboard_click.forget();
    }
    
    // Set up canvas tab click handler
    {
        let canvas_click = Closure::wrap(Box::new(move |_: web_sys::MouseEvent| {
            // Use the new dispatch method with ToggleView message
            let (need_refresh, _) = state::APP_STATE.with(|state| {
                let mut state = state.borrow_mut();
                state.dispatch(messages::Message::ToggleView(storage::ActiveView::Canvas))
            });
            
            // After borrowing mutably, we can refresh UI if needed in a separate borrow
            if need_refresh {
                if let Err(e) = state::AppState::refresh_ui_after_state_change() {
                    web_sys::console::warn_1(&format!("Failed to refresh UI: {:?}", e).into());
                }
            }
        }) as Box<dyn FnMut(_)>);
        
        canvas_tab.add_event_listener_with_callback("click", canvas_click.as_ref().unchecked_ref())?;
        canvas_click.forget();
    }
    
    // Add the tabs container to the document
    let header = document.get_element_by_id("header")
        .ok_or_else(|| JsValue::from_str("Header not found"))?;
    header.append_child(&tabs_container)?;
    
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
            } else {
                // Optional: log successful saves for debugging
                // web_sys::console::log_1(&"Auto-saved state".into());
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